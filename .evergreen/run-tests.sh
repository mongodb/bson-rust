#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=1 cargo test --features chrono-0_4,uuid-0_8
