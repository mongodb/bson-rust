#!/bin/bash

set -o errexit

. ~/.cargo/env

RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=1 cargo test --all-features
