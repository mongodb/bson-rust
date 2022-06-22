#!/bin/sh

set -o errexit

. ~/.cargo/env
. .evergreen/feature-sets.sh

RUST_BACKTRACE=1 cargo test
for f in ${FEATURE_SETS[@]}; do
    echo Testing with features $f
    RUST_BACKTRACE=1 cargo test --features $f
done

cd serde-tests
RUST_BACKTRACE=1 cargo test
