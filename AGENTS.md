# AGENTS.md - Rust BSON

## Overview
Rust crate for working with the BSON format.

## Project Structure
- `src/` - the crate source
- `fuzz/` - fuzz testing with `cargo fuzz`
- `etc/` - useful scripts
- `wasm-test/` - auxiliary binary to test wasm compatibility

## Parsed vs Raw types
The crate provides a set of Rust types corresponding to the types in the BSON spec; these are available as parsed (`Bson`, `Document`, etc.) and raw (`RawBson`, `RawDocument`, etc.).  Parsed types are fully validated and easier to work with; raw types are thin wrappers around raw byte values and are higher performance.

## Serde and Facet
Optional support is present for convenient conversion with arbitrary user types via both `serde` and `facet`; facet support is still somewhat experimental.

## Features
The crate provides a lot of optional functionality, mostly integration with third-party data types.  When checking compilation or running tests, make sure the corresponding features are enabled; see `Cargo.toml` or the feature flags section of `README.md` for details.

## Commands
- Build: `cargo build`
- Run all tests: `cargo test`
- Run a single test: `cargo test path::to::test_fn`

## Testing
The fuzz tests require an initial corpus-generation step before they can be run; see `.evergreen/run-fuzzer.sh` for details.