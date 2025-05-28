#!/bin/bash

set -o errexit

. ~/.cargo/env

export RUST_BACKTRACE=1

cargo test
cargo test --all-features
cargo test --doc

cd serde-tests
cargo test
