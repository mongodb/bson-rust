#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test --no-default-features

cd serde-tests
RUST_BACKTRACE=1 cargo test --no-default-features
