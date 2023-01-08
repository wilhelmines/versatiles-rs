use crate::opencloudtiles::{
	container::{TileReaderBox, TileReaderTrait},
	lib::*,
};
use std::{
	collections::HashMap, env::current_dir, fmt::Debug, fs::File, os::unix::prelude::FileExt,
	path::Path, str::from_utf8, io::Read,
};
use itertools::Itertools;
use tar::{Archive, EntryType};

#[derive(PartialEq, Eq, Hash)]
struct TileKey {
	z: u8,
	y: u64,
	x: u64,
}

struct TarByteRange {
	offset: u64,
	length: u64,
}

pub struct TileReader {
	meta: Blob,
	name: String,
	file: File,
	tile_map: HashMap<TileCoord3, TarByteRange>,
	parameters: TileReaderParameters,
}
impl TileReaderTrait for TileReader {
	fn new(path: &str) -> TileReaderBox
	where
		Self: Sized,
	{
		let mut filename = current_dir().unwrap();
		filename.push(Path::new(path));

		assert!(filename.exists(), "file {:?} does not exist", filename);
		assert!(
			filename.is_absolute(),
			"path {:?} must be absolute",
			filename
		);

		filename = filename.canonicalize().unwrap();

		let file = File::open(filename).unwrap();
		let mut archive = Archive::new(&file);

		let mut meta = Blob::empty();
		let mut tile_map = HashMap::new();
		let mut tile_form: Option<TileFormat> = None;
		let mut tile_comp: Option<Precompression> = None;
		let mut bbox_pyramide = TileBBoxPyramide::new_empty();

		for entry in archive.entries().unwrap() {
			let mut entry = entry.unwrap();
			let header = entry.header();
			if header.entry_type() != EntryType::Regular {
				continue;
			}

			let path = entry.path().unwrap().clone();
			let mut path_tmp: Vec<&str> = path.iter().map(|s| s.to_str().unwrap()).collect_vec();
			path_tmp.remove(0);
			let path_string = path_tmp.join("/");
			drop(path);
			let path_vec: Vec<&str> = path_string.split('/').collect();

			let mut add_tile = || {
				let z = path_vec[0].parse::<u64>().unwrap();
				let y = path_vec[1].parse::<u64>().unwrap();

				let mut filename: Vec<&str> = path_vec[2].split('.').collect();
				let x = filename[0].parse::<u64>().unwrap();

				let mut extension = filename.pop().unwrap();
				let this_comp = match extension {
					"gz" => {
						extension = filename.pop().unwrap();
						Precompression::Gzip
					}
					"br" => {
						extension = filename.pop().unwrap();
						Precompression::Brotli
					}
					_ => Precompression::Uncompressed,
				};

				let this_form = match extension {
					"png" => TileFormat::PNG,
					"jpg" => TileFormat::JPG,
					"jpeg" => TileFormat::JPG,
					"webp" => TileFormat::WEBP,
					"pbf" => TileFormat::PBF,
					_ => panic!("unknown extension for {:?}", path_vec),
				};

				if tile_form.is_none() {
					tile_form = Some(this_form);
				} else {
					assert_eq!(
						tile_form.as_ref().unwrap(),
						&this_form,
						"unknown filename {:?}",
						path_string
					);
				}

				if tile_comp.is_none() {
					tile_comp = Some(this_comp);
				} else {
					assert_eq!(
						tile_comp.as_ref().unwrap(),
						&this_comp,
						"unknown filename {:?}",
						path_string
					);
				}

				let offset = entry.raw_file_position();
				let length = entry.size();

				let coord3 = TileCoord3 { z, y, x };
				bbox_pyramide.include_coord(&coord3);
				tile_map.insert(coord3, TarByteRange { offset, length });
			};

			if path_vec.len() == 3 {
				add_tile();
				continue;
			}

			let mut add_meta = |precompression:Precompression| {
				let mut blob: Vec<u8> = Vec::new();
				entry.read_to_end(&mut blob).unwrap();

				meta = decompress(Blob::from_vec(blob), &precompression);
			};

			if path_vec.len() == 1 {
				match path_vec[0] {
					"meta.json" | "tiles.json" | "metadata.json" => add_meta(Precompression::Uncompressed),
					"meta.json.gz" | "tiles.json.gz" | "metadata.json.gz" => add_meta(Precompression::Gzip),
					"meta.json.br" | "tiles.json.br" | "metadata.json.br" => add_meta(Precompression::Brotli),
					&_ => continue
				};
			}

			// ignore
		}

		Box::new(TileReader {
			meta,
			name: path.to_string(),
			file,
			tile_map,
			parameters: TileReaderParameters::new(
				tile_form.unwrap(),
				tile_comp.unwrap(),
				bbox_pyramide,
			),
		})
	}
	fn get_parameters(&self) -> &TileReaderParameters {
		&self.parameters
	}
	fn get_meta(&self) -> Blob {
		self.meta.clone()
	}
	fn get_tile_data(&self, coord: &TileCoord3) -> Option<Blob> {
		let range = self.tile_map.get(coord);

		range?;

		let offset = range.unwrap().offset;
		let length = range.unwrap().length as usize;

		let mut buf: Vec<u8> = Vec::new();
		buf.resize(length, 0);

		self.file.read_exact_at(&mut buf, offset).unwrap();

		Some(Blob::from_vec(buf))
	}
	fn get_name(&self) -> &str {
		&self.name
	}
}

impl Debug for TileReader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TileReader:Tar")
			.field("meta", &from_utf8(self.get_meta().as_slice()).unwrap())
			.field("parameters", &self.get_parameters())
			.finish()
	}
}
