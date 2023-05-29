
[![Crates.io](https://img.shields.io/crates/v/versatiles?label=version)](https://crates.io/crates/versatiles)
[![Crates.io](https://img.shields.io/crates/d/versatiles?label=crates.io+downloads)](https://crates.io/crates/versatiles)
[![Code Coverage](https://codecov.io/gh/versatiles-org/versatiles-rs/branch/main/graph/badge.svg?token=IDHAI13M0K)](https://codecov.io/gh/versatiles-org/versatiles-rs)
[![Docker Pulls](https://img.shields.io/docker/pulls/versatiles/versatiles)](https://hub.docker.com/r/versatiles/versatiles/tags)
[![Docker Image Size](https://img.shields.io/docker/image-size/versatiles/versatiles/latest-scratch?label=docker+size)](https://hub.docker.com/r/versatiles/versatiles/tags)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Matrix Chat](https://img.shields.io/matrix/versatiles:matrix.org?label=matrix)](https://matrix.to/#/#versatiles:matrix.org)

# install

- Install [Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- Then run `cargo install versatiles`

# run

running `versatiles` will list you the available commands:
```
Usage: versatiles <COMMAND>

Commands:
   convert  Convert between different tile containers
   probe    Show information about a tile container
   serve    Serve tiles via http
```

# supported formats

<table>
   <thead>
      <tr><th>feature</th><th>versatiles</th><th>mbtiles</th><th>tar</th></tr>
   </thead>
   <tbody>
      <tr><th colspan="4" style="text-align:center">read container</th></tr>
      <tr><td>from file</td><td>✅</td><td>✅</td><td>✅</td></tr>
      <tr><td>from http</td><td>✅</td><td>🚫</td><td>🚫</td></tr>
      <tr><td>from gcs</td><td>🚧</td><td>🚫</td><td>🚫</td></tr>
      <tr><td>from S3</td><td>🚧</td><td>🚫</td><td>🚫</td></tr>
      <tr><th colspan="4" style="text-align:center">write container</th></tr>
      <tr><td>to file</td><td>✅</td><td>🚫</td><td>✅</td></tr>
      <tr><th colspan="4" style="text-align:center">compression</th></tr>
      <tr><td>uncompressed</td><td>✅</td><td>🚫</td><td>✅</td></tr>
      <tr><td>gzip</td><td>✅</td><td>✅</td><td>✅</td></tr>
      <tr><td>brotli</td><td>✅</td><td>🚫</td><td>✅</td></tr>
   </tbody>
</table>

More about the VersaTiles container format: [github.com/versatiles-org/**versatiles-spec**](https://github.com/versatiles-org/versatiles-spec)

# examples

```bash
versatiles convert --tile-format webp satellite_tiles.tar satellite_tiles.versatiles

versatiles serve satellite_tiles.versatiles
```
