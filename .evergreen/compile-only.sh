#!/bin/bash

set -o errexit

. ~/.cargo/env
rustup update $RUST_VERSION

if [ ! -z "$TARGET" ]; then
    if [[ "$TARGET" = "wasm32-unknown-unknown" ]]; then
        export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
    fi
    rustup target add $TARGET --toolchain $RUST_VERSION
    TARGET="--target=$TARGET"
fi

# Generate a new lockfile with MSRV-compatible dependencies.
if [ "$MSRV" = "true" ]; then
    CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback cargo +nightly -Zmsrv-policy generate-lockfile
fi

rustup run $RUST_VERSION cargo build $TARGET
