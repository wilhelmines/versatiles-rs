use core::time;
use criterion::{black_box, criterion_group, Criterion};
use futures::executor::block_on;
use rand::{seq::SliceRandom, thread_rng};
use reqwest::blocking::get;
use std::thread;
use versatiles::{
	containers::get_reader,
	server::{source::TileContainer, TileServer},
	shared::TileCoord3,
};

fn bench_server(c: &mut Criterion) {
	let mut group = c.benchmark_group("test_server");

	let reader = block_on(get_reader("benches/ressources/berlin.mbtiles")).unwrap();
	let coords: Vec<TileCoord3> = reader
		.get_parameters()
		.unwrap()
		.get_bbox_pyramide()
		.iter_tile_indexes()
		.collect();
	drop(reader);

	let mut server = TileServer::new("127.0.0.1", 8080);
	let reader = block_on(get_reader("src/bin/benches/ressources/berlin.mbtiles")).unwrap();
	server.add_tile_source(&format!("/tiles/berlin/"), TileContainer::from(reader).unwrap());

	thread::spawn(move || block_on(server.start()).unwrap());

	thread::sleep(time::Duration::from_secs(1));

	group.sample_size(50);
	group.bench_function("tile_request", |b| {
		b.iter(|| {
			let coord = coords.choose(&mut thread_rng()).unwrap();
			let url = format!("http://127.0.0.1:8080/tiles/berlin/{}/{}/{}", coord.z, coord.y, coord.x);

			let _resp = black_box(get(url).unwrap().text().unwrap());
		})
	});
}

criterion_group!(server, bench_server);