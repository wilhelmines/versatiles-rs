use super::{OperationTrait, PipelineFactory};
use crate::{
	container::{TilesReader, TilesReaderParameters},
	io::DataReader,
	types::{Blob, TileBBox, TileCompression, TileCoord3, TileStream},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::Path;

/// The `PipelineReader` struct is responsible for managing the tile reading process,
/// applying operations, and returning the composed tiles.
pub struct PipelineReader {
	pub name: String,
	pub operation: Box<dyn OperationTrait>,
	pub parameters: TilesReaderParameters,
}

#[allow(dead_code)]
impl PipelineReader {
	/// Opens a PipelineReader from a vdl file path.
	///
	/// # Arguments
	///
	/// * `path` - The path to the vdl file.
	///
	/// # Returns
	///
	/// * `Result<PipelineReader>` - The constructed PipelineReader or an error if the configuration is invalid.
	pub async fn open_path(path: &Path) -> Result<PipelineReader> {
		let vdl =
			std::fs::read_to_string(path).with_context(|| anyhow!("Failed to open {path:?}"))?;
		Self::from_str(&vdl, path.to_str().unwrap(), path.parent().unwrap())
			.await
			.with_context(|| format!("failed parsing {path:?} as VDL"))
	}

	/// Opens a PipelineReader from a DataReader.
	///
	/// # Arguments
	///
	/// * `reader` - The DataReader containing the vdl configuration.
	///
	/// # Returns
	///
	/// * `Result<PipelineReader>` - The constructed PipelineReader or an error if the configuration is invalid.
	pub async fn open_reader(mut reader: DataReader, dir: &Path) -> Result<PipelineReader> {
		let vdl = reader.read_all().await?.into_string();
		Self::from_str(&vdl, reader.get_name(), dir)
			.await
			.with_context(|| format!("failed parsing {} as VDL", reader.get_name()))
	}

	#[cfg(test)]
	pub async fn open_str(vdl: &str, dir: &Path) -> Result<PipelineReader> {
		Self::from_str(vdl, "from str", dir)
			.await
			.with_context(|| format!("failed parsing '{vdl}' as VDL"))
	}

	async fn from_str(vdl: &str, name: &str, dir: &Path) -> Result<PipelineReader> {
		let factory = PipelineFactory::default(dir);
		let operation: Box<dyn OperationTrait> = factory.operation_from_vdl(vdl).await?;
		let parameters = operation.get_parameters().clone();

		Ok(PipelineReader {
			name: name.to_string(),
			operation,
			parameters,
		})
	}
}

#[async_trait]
impl TilesReader for PipelineReader {
	/// Get the name of the reader source, e.g., the filename.
	fn get_name(&self) -> &str {
		&self.name
	}

	/// Get the container name, e.g., versatiles, mbtiles, etc.
	fn get_container_name(&self) -> &str {
		"pipeline"
	}

	/// Get the reader parameters.
	fn get_parameters(&self) -> &TilesReaderParameters {
		&self.parameters
	}

	/// Override the tile compression.
	fn override_compression(&mut self, _tile_compression: TileCompression) {
		panic!("you can't override the compression of pipeline")
	}

	/// Get the metadata, always uncompressed.
	fn get_meta(&self) -> Result<Option<Blob>> {
		Ok(self.operation.get_meta())
	}

	/// Get tile data for the given coordinate, always compressed and formatted.
	async fn get_tile_data(&mut self, coord: &TileCoord3) -> Result<Option<Blob>> {
		self.operation.get_tile_data(coord).await
	}

	/// Get a stream of tiles within the bounding box.
	async fn get_bbox_tile_stream(&mut self, bbox: TileBBox) -> TileStream {
		self.operation.get_bbox_tile_stream(bbox).await
	}
}

impl std::fmt::Debug for PipelineReader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PipelineReader")
			.field("name", &self.name)
			.field("parameters", &self.parameters)
			.field("output", &self.operation)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::container::MockTilesWriter;

	pub const VDL: &str = include_str!("../../../../testdata/berlin.vdl");

	#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
	async fn open_vdl_str() -> Result<()> {
		let mut reader = PipelineReader::open_str(VDL, Path::new("../testdata/")).await?;
		MockTilesWriter::write(&mut reader).await?;

		Ok(())
	}

	#[tokio::test]
	async fn test_tile_pipeline_reader_open_path() -> Result<()> {
		let path = Path::new("../testdata/pipeline.vdl");
		let result = PipelineReader::open_path(path).await;
		assert_eq!(
			result.unwrap_err().to_string(),
			"Failed to open \"../testdata/pipeline.vdl\""
		);

		Ok(())
	}

	#[tokio::test]
	async fn test_tile_pipeline_reader_get_tile_data() -> Result<()> {
		let mut reader = PipelineReader::open_str(VDL, Path::new("../testdata/")).await?;

		let result = reader.get_tile_data(&TileCoord3::new(0, 0, 14)?).await;
		assert_eq!(result?, None);

		let result = reader
			.get_tile_data(&TileCoord3::new(8800, 5377, 14)?)
			.await?
			.unwrap();

		assert_eq!(result.len(), 104532);

		Ok(())
	}

	#[tokio::test]
	async fn test_tile_pipeline_reader_get_bbox_tile_stream() -> Result<()> {
		let mut reader = PipelineReader::open_str(VDL, Path::new("../testdata/")).await?;
		let bbox = TileBBox::new(1, 0, 0, 1, 1)?;
		let result_stream = reader.get_bbox_tile_stream(bbox).await;
		let result = result_stream.collect().await;

		assert!(!result.is_empty());

		Ok(())
	}
}