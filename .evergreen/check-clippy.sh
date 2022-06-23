#!/bin/bash

set -o errexit

. ~/.cargo/env
. .evergreen/feature-sets.sh

for f in ${FEATURE_SETS[@]}; do
    echo Testing with features $f
    cargo clippy --all-targets --features $f -p bson -- -D warnings
done

cd serde-tests
cargo clippy --all-targets --all-features -p serde-tests -- -D warnings
