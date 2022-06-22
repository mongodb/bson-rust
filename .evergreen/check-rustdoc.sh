#!/bin/sh

set -o errexit

. ~/.cargo/env
. .evergreen/feature-sets.sh

for f in ${FEATURE_SETS[@]}; do
    echo Testing with features $f
    cargo +nightly rustdoc -p bson --features $f -- --cfg docsrs -D warnings
done
