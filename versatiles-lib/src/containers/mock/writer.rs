use crate::{
	containers::{TilesReaderBox, TilesWriterBox, TilesWriterParameters, TilesWriterTrait},
	shared::{Compression, TileFormat},
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;

#[allow(dead_code)]
#[derive(Debug)]
pub enum MockTilesWriterProfile {
	PNG,
	PBF,
}

pub struct MockTilesWriter {
	parameters: TilesWriterParameters,
}

impl MockTilesWriter {
	pub fn new_mock_profile(profile: MockTilesWriterProfile) -> TilesWriterBox {
		Self::new_mock(match profile {
			MockTilesWriterProfile::PNG => TilesWriterParameters::new(TileFormat::PNG, Compression::None),
			MockTilesWriterProfile::PBF => TilesWriterParameters::new(TileFormat::PBF, Compression::Gzip),
		})
	}
	pub fn new_mock(parameters: TilesWriterParameters) -> TilesWriterBox {
		Box::new(MockTilesWriter { parameters })
	}
}

#[async_trait]
impl TilesWriterTrait for MockTilesWriter {
	async fn write_tiles(&mut self, reader: &mut TilesReaderBox) -> Result<()> {
		let _temp = reader.get_container_name();
		let _temp = reader.get_name();
		let _temp = reader.get_meta().await?;

		let bbox_pyramid = reader.get_parameters().bbox_pyramid.clone();

		for bbox in bbox_pyramid.iter_levels() {
			let mut stream = reader.get_bbox_tile_stream(bbox.clone()).await;
			while let Some((_coord, _blob)) = stream.next().await {}
		}

		Ok(())
	}
	fn get_parameters(&self) -> &TilesWriterParameters {
		&self.parameters
	}
}

#[cfg(test)]
mod tests {
	use super::{MockTilesWriter, MockTilesWriterProfile};
	use crate::containers::mock::{reader::MockTilesReaderProfile, MockTilesReader};

	#[tokio::test]
	async fn convert_png() {
		let mut writer = MockTilesWriter::new_mock_profile(MockTilesWriterProfile::PNG);
		let mut reader = MockTilesReader::new_mock_profile(MockTilesReaderProfile::PNG);
		writer.write_from_reader(&mut reader).await.unwrap();
	}

	#[tokio::test]
	async fn convert_pbf() {
		let mut writer = MockTilesWriter::new_mock_profile(MockTilesWriterProfile::PBF);
		let mut reader = MockTilesReader::new_mock_profile(MockTilesReaderProfile::PBF);
		writer.write_from_reader(&mut reader).await.unwrap();
	}
}
