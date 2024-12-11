#!/bin/bash

set -o errexit

. ~/.cargo/env

cd fuzz

# Create artifacts directory for crash reports
mkdir -p artifacts

# Function to run fuzzer and collect crashes
run_fuzzer() {
    target=$1
    echo "Running fuzzer for $target"
    # Run fuzzer and redirect crashes to artifacts directory
    RUST_BACKTRACE=1 cargo +nightly fuzz run $target -- \
        -rss_limit_mb=4096 \
        -max_total_time=3600 \
        -artifact_prefix=artifacts/ \
        -print_final_stats=1
}

# Run existing targets
run_fuzzer "deserialize"
run_fuzzer "raw_deserialize"
run_fuzzer "iterate"

# Run new security-focused targets
run_fuzzer "malformed_length"
run_fuzzer "type_markers"
run_fuzzer "string_handling"
run_fuzzer "serialization"

# If any crashes were found, save them as test artifacts
if [ "$(ls -A artifacts)" ]; then
    echo "Crashes found! Check artifacts directory."
    exit 1
else
    echo "No crashes found."
fi

