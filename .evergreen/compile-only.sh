#!/bin/sh

set -o errexit

. ~/.cargo/env
rustup update $RUST_VERSION

rustup run $RUST_VERSION cargo build
