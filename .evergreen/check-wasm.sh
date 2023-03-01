#!/bin/bash

set -o errexit

. ~/.cargo/env

rustup target add wasm32-unknown-unknown
cd $(dirname $0)/../wasm-test
cargo build --target wasm32-unknown-unknown