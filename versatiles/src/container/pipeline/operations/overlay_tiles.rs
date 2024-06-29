use crate::{
	container::{
		pipeline::{OperationTrait, PipelineFactory, ReadOperationFactoryTrait},
		TilesReaderParameters,
	},
	types::{Blob, TileBBox, TileCoord3, TileStream},
	utils::{
		recompress,
		vdl::{VDLNode, VDLPipeline},
	},
};
use anyhow::{ensure, Result};
use async_trait::async_trait;
use futures::future::{join_all, BoxFuture};
use versatiles_core::types::TileCompression;

#[derive(versatiles_derive::VDLDecode, Clone, Debug)]
/// Overlays multiple tile sources. The tile of the first source that returns a tile is used.
struct Args {
	/// All tile source must have the same tile format.
	children: Vec<VDLPipeline>,
}

#[derive(Debug)]
struct Operation {
	parameters: TilesReaderParameters,
	sources: Vec<Box<dyn OperationTrait>>,
}

impl<'a> Operation {
	fn new(
		vdl_node: VDLNode,
		factory: &'a PipelineFactory,
	) -> BoxFuture<'a, Result<Box<dyn OperationTrait>, anyhow::Error>>
	where
		Self: Sized + OperationTrait,
	{
		Box::pin(async move {
			let args = Args::from_vdl_node(&vdl_node)?;
			let sources = join_all(args.children.into_iter().map(|c| factory.build_pipeline(c)))
				.await
				.into_iter()
				.collect::<Result<Vec<_>>>()?;

			ensure!(!sources.is_empty(), "must have at least one child");

			let parameters = sources.first().unwrap().get_parameters();
			let mut pyramid = parameters.bbox_pyramid.clone();
			let tile_format = parameters.tile_format;
			let mut tile_compression = parameters.tile_compression;

			for source in sources.iter() {
				let parameters = source.get_parameters();
				pyramid.include_bbox_pyramid(&parameters.bbox_pyramid);
				ensure!(
					parameters.tile_format == tile_format,
					"all children must have the same tile format"
				);
				if parameters.tile_compression != tile_compression {
					tile_compression = TileCompression::Uncompressed;
				}
			}

			let parameters = TilesReaderParameters::new(tile_format, tile_compression, pyramid);
			Ok(Box::new(Self {
				parameters,
				sources,
			}) as Box<dyn OperationTrait>)
		})
	}
}

#[async_trait]
impl OperationTrait for Operation {
	fn get_parameters(&self) -> &TilesReaderParameters {
		&self.parameters
	}

	fn get_meta(&self) -> Option<Blob> {
		todo!();
		//self.reader.lock().await.get_meta()
	}

	async fn get_tile_data(&mut self, coord: &TileCoord3) -> Result<Option<Blob>> {
		for source in self.sources.iter_mut() {
			let result = source.get_tile_data(coord).await?;
			if let Some(mut blob) = result {
				blob = recompress(
					blob,
					&source.get_parameters().tile_compression,
					&self.parameters.tile_compression,
				)?;
				return Ok(Some(blob));
			}
		}
		return Ok(None);
	}

	async fn get_bbox_tile_stream(&self, bbox: TileBBox) -> TileStream {
		let output_compression = &self.parameters.tile_compression;
		let bboxes: Vec<TileBBox> = bbox.clone().iter_bbox_grid(32).collect();
		TileStream::from_stream_iter(bboxes.into_iter().map(move |bbox| async move {
			let mut tiles: Vec<(TileCoord3, Blob)> = Vec::new();
			for source in self.sources.iter() {
				source
					.get_bbox_tile_stream(bbox.clone())
					.await
					.for_each_sync(|(coord, mut blob)| {
						let index = bbox.get_tile_index3(&coord);
						if tiles.get(index).is_none() {
							blob = recompress(
								blob,
								&source.get_parameters().tile_compression,
								output_compression,
							)
							.unwrap();
							tiles.insert(index, (coord, blob));
						}
					})
					.await;
			}
			TileStream::from_vec(tiles)
		}))
		.await
	}
}

pub struct Factory {}

#[async_trait]
impl ReadOperationFactoryTrait for Factory {
	fn get_tag_name(&self) -> &str {
		"overlay_tiles"
	}
	async fn build<'a>(
		&self,
		vdl_node: VDLNode,
		factory: &'a PipelineFactory,
	) -> Result<Box<dyn OperationTrait>> {
		Operation::new(vdl_node, factory).await
	}
}