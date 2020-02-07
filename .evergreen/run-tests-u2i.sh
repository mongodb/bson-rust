#!/bin/sh

set -o errexit

. ~/.cargo/env
RUST_BACKTRACE=1 cargo test --features u2i
