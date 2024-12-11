#!/bin/bash

set -o errexit

. ~/.cargo/env

cd fuzz

# Create artifacts directory for crash reports
mkdir -p artifacts

# Function to run fuzzer and collect crashes
run_fuzzer() {
    target=$1
    time=$2
    echo "Running fuzzer for $target"
    # Run fuzzer and redirect crashes to artifacts directory
    RUST_BACKTRACE=1 cargo +nightly fuzz run $target -- \
        -rss_limit_mb=4096 \
        -max_total_time=$time \
        -artifact_prefix=artifacts/ \
        -print_final_stats=1
}

# Run existing targets
run_fuzzer "deserialize" 60
run_fuzzer "raw_deserialize" 60
run_fuzzer "iterate" 60

# Run new security-focused targets
run_fuzzer "malformed_length" 60
run_fuzzer "type_markers" 120
run_fuzzer "string_handling" 120
run_fuzzer "serialization" 60

# If any crashes were found, save them as test artifacts
if [ "$(ls -A artifacts)" ]; then
    echo "Crashes found! Check artifacts directory."
    exit 1
else
    echo "No crashes found."
fi

