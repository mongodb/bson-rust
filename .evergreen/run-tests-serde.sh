#!/bin/sh

set -o errexit

. ~/.cargo/env
cd serde-tests
RUST_BACKTRACE=1 cargo test
