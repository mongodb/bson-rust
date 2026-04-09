#!/usr/bin/env bash

# make sure we're running in the repo root
cd $(dirname $0)/..

RUSTFLAGS="--cfg mongodb_internal_bench" cargo +nightly bench tests::bench --all-features