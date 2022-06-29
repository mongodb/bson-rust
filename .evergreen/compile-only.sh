#!/bin/sh

set -o errexit

. ~/.cargo/env
rustup update $RUST_VERSION

# pin all dependencies when checking msrv compilation
if [  "$MSRV" = "true" ]; then
    cp .evergreen/Cargo.lock.msrv Cargo.lock
fi

rustup run $RUST_VERSION cargo build
