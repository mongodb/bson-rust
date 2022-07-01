#!/bin/bash

set -o errexit

. ~/.cargo/env

cargo clippy --all-targets --all-features -p bson -- -D warnings

cd serde-tests
cargo clippy --all-targets --all-features -p serde-tests -- -D warnings
