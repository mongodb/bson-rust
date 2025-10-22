#!/bin/bash

set -o errexit

. ~/.cargo/env

# Test with default features and excluding doctests (some of which require the 'serde' feature)
RUST_BACKTRACE=1 cargo test --all-targets
# Test with all features and including doctests
RUST_BACKTRACE=1 cargo test --all-features
