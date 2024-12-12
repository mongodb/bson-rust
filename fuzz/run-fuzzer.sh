#!/bin/bash
set -e

# Directory setup
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS_DIR="$SCRIPT_DIR/corpus"
ARTIFACTS_DIR="$SCRIPT_DIR/artifacts"

# Ensure directories exist
mkdir -p "$CORPUS_DIR"
mkdir -p "$ARTIFACTS_DIR"

# Generate corpus if it doesn't exist or is empty
if [ ! -d "$CORPUS_DIR" ] || [ -z "$(ls -A $CORPUS_DIR)" ]; then
    echo "Generating initial corpus..."
    cargo run --bin generate_corpus
    # Move generated corpus files to the corpus directory
    mv generated_corpus/* "$CORPUS_DIR/" 2>/dev/null || true
fi

# List of fuzz targets
TARGETS=(
    "malformed_length"
    "type_markers"
    "string_handling"
    "serialization"
)

# Run each fuzz target with the corpus
for target in "${TARGETS[@]}"; do
    echo "Running fuzzer for target: $target"
    RUST_BACKTRACE=1 cargo fuzz run "$target" "$CORPUS_DIR" -j 1 --release --max-total-time=3600
done
