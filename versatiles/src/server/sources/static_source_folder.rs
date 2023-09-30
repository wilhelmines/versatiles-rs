use super::{response::SourceResponse, static_source::StaticSourceTrait};
use crate::server::guess_mime;
use anyhow::Result;
use async_trait::async_trait;
use std::{
	env::current_dir,
	fmt::Debug,
	fs::File,
	io::{BufReader, Read},
	path::{Path, PathBuf},
};
use versatiles_lib::shared::{Blob, Compression, TargetCompression};

// Folder struct definition
#[derive(Clone)]
pub struct Folder {
	folder: PathBuf,
	name: String,
}

impl Folder {
	// Constructor for the Folder struct
	pub fn from(path: &str) -> Result<Folder> {
		let mut folder = current_dir()?;
		folder.push(Path::new(path));

		// Check that the folder exists, is absolute and is a directory
		assert!(folder.exists(), "path {folder:?} does not exist");
		assert!(folder.is_absolute(), "path {folder:?} must be absolute");
		assert!(folder.is_dir(), "path {folder:?} must be a directory");

		folder = folder.canonicalize()?;

		// Create a new Folder struct with the given path and name
		Ok(Folder {
			folder,
			name: path.to_string(),
		})
	}
}

#[async_trait]
impl StaticSourceTrait for Folder {
	// Returns the name of the folder
	fn get_name(&self) -> Result<String> {
		Ok(self.name.clone())
	}

	// Gets the data at the given path and responds with a compressed or uncompressed version
	// based on the accept header
	async fn get_data(&self, path: &[&str], _accept: &TargetCompression) -> Option<SourceResponse> {
		let mut local_path = self.folder.clone();
		local_path.push(PathBuf::from(path.join("/")));

		// If the path is a directory, append 'index.html'
		if local_path.is_dir() {
			local_path.push("index.html");
		}

		// If the local path is not a subpath of the folder or it doesn't exist, return not found
		if !local_path.starts_with(&self.folder) || !local_path.exists() || !local_path.is_file() {
			return None;
		}

		let f = File::open(&local_path).unwrap();
		let mut buffer = Vec::new();
		BufReader::new(f).read_to_end(&mut buffer).unwrap();
		let blob = Blob::from(buffer);

		let mime = guess_mime(&local_path);

		return SourceResponse::new_some(blob, &Compression::None, &mime);
	}
}

impl Debug for Folder {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Folder")
			.field("folder", &self.folder)
			.field("name", &self.name)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use crate::server::sources::static_source::StaticSourceTrait;

	use super::Folder;
	use versatiles_lib::shared::TargetCompression;

	#[tokio::test]
	async fn test() {
		// Create a new Folder instance
		let folder = Folder::from("../testdata").unwrap();

		let debug: String = format!("{:?}", folder);
		assert!(debug.starts_with("Folder { folder: \""));
		assert!(debug.ends_with("testdata\", name: \"../testdata\" }"));

		// Test get_name function
		assert_eq!(folder.get_name().unwrap(), "../testdata");

		// Test get_data function with a non-existent file
		let result = folder
			.get_data(&["recipes", "Queijo.txt"], &TargetCompression::from_none())
			.await;
		assert!(result.is_none());

		// Test get_data function with an existing file
		let result = folder
			.get_data(&["berlin.mbtiles"], &TargetCompression::from_none())
			.await;
		assert!(result.is_some());

		let result = result.unwrap().blob;
		assert_eq!(result.len(), 26533888);
	}
}