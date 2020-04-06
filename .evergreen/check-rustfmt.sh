#!/bin/sh

set -o errexit

. ~/.cargo/env
cargo +nightly fmt -- --check

cd serde-tests && cargo +nightly fmt -- --check
