#!/bin/bash

set -o errexit

. ~/.cargo/env

# Pin clippy to the latest version. This should be updated when new versions of Rust are released.
CLIPPY_VERSION=1.94.0

rustup install $CLIPPY_VERSION

cargo +$CLIPPY_VERSION clippy --all-targets --all-features -p bson -- -D warnings
