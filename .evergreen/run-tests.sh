#!/bin/bash

set -o errexit

. ~/.cargo/env

RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=1 cargo test --all-features
RUST_BACKTRACE=1 cargo test --features serde nesting_limit