#!/bin/bash

set -o errexit

. ~/.cargo/env

cd fuzz

# each runs for a minute
cargo +nightly fuzz run deserialize -- -rss_limit_mb=4096 -max_total_time=60
cargo +nightly fuzz run raw_deserialize -- -rss_limit_mb=4096 -max_total_time=60
cargo +nightly fuzz run iterate -- -rss_limit_mb=4096 -max_total_time=60
cargo +nightly fuzz run malformed_length -- -rss_limit_mb=4096 -max_total_time=60
cargo +nightly fuzz run type_markers -- -rss_limit_mb=4096 -max_total_time=120
cargo +nightly fuzz run string_handling -- -rss_limit_mb=4096 -max_total_time=120
cargo +nightly fuzz run serialization -- -rss_limit_mb=4096 -max_total_time=60
