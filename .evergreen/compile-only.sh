#!/bin/bash

set -o errexit

. ~/.cargo/env
rustup update $RUST_VERSION

if [ ! -z "$TARGET" ]; then
    rustup target add $TARGET --toolchain $RUST_VERSION
    TARGET="--target=$TARGET"
fi

# pin necessary dependencies when checking msrv compilation
if [ "$MSRV" = "true" ]; then
    patch Cargo.toml .evergreen/MSRV-Cargo.toml.diff
fi

rustup run $RUST_VERSION cargo build $TARGET
