use super::{tile_bbox::TileBBox, tile_coords::TileCoord3};
use std::{ops::RangeInclusive, slice::Iter};

const MAX_ZOOM_LEVEL: usize = 32;
pub struct TileBBoxPyramide {
	level_bbox: Vec<TileBBox>,
}
impl TileBBoxPyramide {
	pub fn new_full() -> TileBBoxPyramide {
		return TileBBoxPyramide {
			level_bbox: (0..=MAX_ZOOM_LEVEL)
				.map(|z| TileBBox::new_full(z as u64))
				.collect(),
		};
	}
	pub fn new_empty() -> TileBBoxPyramide {
		return TileBBoxPyramide {
			level_bbox: (0..=MAX_ZOOM_LEVEL)
				.map(|z| TileBBox::new_empty(z as u64))
				.collect(),
		};
	}
	pub fn set_zoom_min(&mut self, zoom_level_min: u64) {
		for (index, bbox) in self.level_bbox.iter_mut().enumerate() {
			let level = index as u64;
			if level < zoom_level_min {
				bbox.set_empty(level);
			}
		}
	}
	pub fn set_zoom_max(&mut self, zoom_level_max: u64) {
		for (index, bbox) in self.level_bbox.iter_mut().enumerate() {
			let level = index as u64;
			if level > zoom_level_max {
				bbox.set_empty(level);
			}
		}
	}
	pub fn limit_by_geo_bbox(&mut self, geo_bbox: &[f32; 4]) {
		for (level, bbox) in self.level_bbox.iter_mut().enumerate() {
			bbox.intersect(&TileBBox::from_geo(level as u64, geo_bbox));
		}
	}
	pub fn intersect(&mut self, level_bbox: &TileBBoxPyramide) {
		for (level, bbox) in self.level_bbox.iter_mut().enumerate() {
			bbox.intersect(level_bbox.get_level_bbox(level as u64));
		}
	}
	pub fn get_level_bbox(&self, level: u64) -> &TileBBox {
		return &self.level_bbox[level as usize];
	}
	pub fn set_level_bbox(&mut self, level: u64, bbox: &TileBBox) {
		self.level_bbox[level as usize].set(bbox);
	}
	pub fn include_tile(&mut self, level: u64, col: u64, row: u64) {
		self.level_bbox[level as usize].include_tile(col, row);
	}
	pub fn iter(&self) -> Iter<TileBBox> {
		return self.level_bbox.iter();
	}
	pub fn iter_tile_indexes(&self) -> impl Iterator<Item = TileCoord3> + '_ {
		return self
			.level_bbox
			.as_slice()
			.iter()
			.enumerate()
			.map(move |(z, bbox)| {
				bbox.iter_tile_indexes().map(move |coord2| TileCoord3 {
					x: coord2.x,
					y: coord2.y,
					z: z as u64,
				})
			})
			.flatten();
	}
	pub fn get_zoom_range(&self) -> RangeInclusive<u64> {
		let levels: Vec<u64> = self
			.level_bbox
			.iter()
			.enumerate()
			.filter_map(|(level, bbox)| {
				if bbox.is_empty() {
					None
				} else {
					Some(level as u64)
				}
			})
			.collect();

		let start: u64;
		let end: u64;

		if levels.len() == 0 {
			start = 0;
			end = 0;
		} else {
			start = *levels.first().unwrap();
			end = *levels.last().unwrap();
		}

		return RangeInclusive::new(start, end);
	}
}