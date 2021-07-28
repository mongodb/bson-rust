#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test --features u2i

cd serde-tests
RUST_BACKTRACE=1 cargo test --features u2i
