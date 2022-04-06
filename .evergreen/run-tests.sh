#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=1 cargo test --all-features

cd serde-tests
RUST_BACKTRACE=1 cargo test
