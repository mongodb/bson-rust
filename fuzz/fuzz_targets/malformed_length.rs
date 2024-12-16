//! BSON Document Length Field Fuzzer
//!
//! This fuzz test focuses on finding security vulnerabilities related to BSON document length
//! fields. It specifically targets:
//! - Integer overflow/underflow in length calculations
//! - Malformed length fields that could cause buffer overruns
//! - Mismatches between declared and actual document sizes
//! - Memory allocation issues with large or invalid lengths

#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::RawDocument;

fuzz_target!(|buf: &[u8]| {
    // Focus on document length field manipulation
    // This should return an error if the buf.len() < 4 rather than panic.
    let _ = RawDocument::from_bytes(buf);
});
