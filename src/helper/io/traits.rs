use crate::types::{Blob, ByteRange};
use anyhow::Result;
use async_trait::async_trait;
use std::{fmt::Debug, io::Read, path::Path};

pub type DataReaderBox = Box<dyn DataReaderTrait>;

#[async_trait]
pub trait DataReaderTrait: Debug + Read + Send + Sync {
	async fn read_range(&mut self, range: &ByteRange) -> Result<Blob>;
	fn get_name(&self) -> &str;
}

// pub type DataWriterBox = Box<dyn DataWriterTrait>;

#[async_trait]
pub trait DataWriterTrait: Send {
	fn new(path: &Path) -> Result<Box<Self>>
	where
		Self: Sized;
	fn append(&mut self, blob: &Blob) -> Result<ByteRange>;
	fn write_start(&mut self, blob: &Blob) -> Result<()>;
	fn get_position(&mut self) -> Result<u64>;
}
