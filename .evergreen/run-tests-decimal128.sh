#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test --features decimal128

cd serde-tests
RUST_BACKTRACE=1 cargo test --features decimal128
