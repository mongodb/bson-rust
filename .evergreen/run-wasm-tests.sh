#!/bin/bash

set -o errexit

. ~/.cargo/env

rustup update 1.81
rustup default 1.81

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

cd $(dirname $0)/../wasm-test
wasm-pack test --node