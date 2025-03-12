#!/bin/bash

set -o errexit

. ~/.cargo/env

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# install nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
. ~/.nvm/nvm.sh
# install node
nvm install --no-progress node
node --version

cd $(dirname $0)/../wasm-test
wasm-pack test --node