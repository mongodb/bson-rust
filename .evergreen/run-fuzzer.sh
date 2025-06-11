#!/bin/bash

set -o errexit

. ~/.cargo/env

cd fuzz

# Create directories for crashes and corpus
mkdir -p artifacts
mkdir -p corpus

# Generate initial corpus if directory is empty
if [ -z "$(ls -A corpus)" ]; then
    echo "Generating initial corpus..."
    cargo run --bin generate_corpus
fi

# Function to run fuzzer and collect crashes
run_fuzzer() {
    target=$1
    echo "Running fuzzer for $target"
    # Run fuzzer and redirect crashes to artifacts directory
    RUST_BACKTRACE=1 cargo +nightly fuzz run $target -- \
        -rss_limit_mb=4096 \
        -max_total_time=60 \
        -artifact_prefix=artifacts/ \
        -print_final_stats=1 \
        corpus/
}

# Run existing targets
run_fuzzer "decode"
run_fuzzer "raw_deserialize"
run_fuzzer "raw_deserialize_utf8_lossy"
run_fuzzer "iterate"

# Run new security-focused targets
run_fuzzer "type_markers"
run_fuzzer "string_handling"
run_fuzzer "encoding"
