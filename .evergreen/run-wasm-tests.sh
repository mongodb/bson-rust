#!/bin/bash

set -o errexit

. ~/.cargo/env

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

cd $(dirname $0)/../wasm-test
wasm-pack test --node