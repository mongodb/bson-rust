# Migrating from 2.x to 3.0
3.0 updates several APIs in backwards-incompatible ways; in most cases these changes should require only minor updates in application code.

## Unified error hierarchy
In 2.x, many crate submodules had their own `Error` types, with inconsistent conversions between those types.  In 3.0, the crate defines a single `bson::error::Error` type, with fields for values common across errors like message or associated key, and a `kind` enum that provides granular root cause information.

## `&CStr`
The [bson spec](https://bsonspec.org/spec.html) describes a "cstring" type as UTF-8, with the exception that it cannot contain byte `0` (which is otherwise a valid UTF8 byte value).  This type is used in BSON for the keys of documents and for the pattern and options of regular expressions.

In 2.x, attempting to use `rawdoc!` or `RawDocumentBuf::append` with a key or regular expression containing a `0` byte would panic.

3.0 introduces the `&CStr`/`CString` types to represent the "cstring" type as described in the spec; these types are analogous to `&str` and `String` but validate on construction that no `0` byte is contained.  The `cstr!` macro will construct a literal ``&`static CStr`` from a ``&`static str`` with compile-time validation, and `TryFrom` impls are provided for run-time validation.

In 2.x:
```rust
let mut computed_key = "foo".to_owned();
computed_key.push_str("bar");
let mut doc_buf = rawdoc! {
    "hello": "world",
    computed_key: 42,
    "regex": Regex {
        pattern: "needle".to_owned(),
        options: "".to_owned(),
    },
};
doc_buf.append("a key", "a value");
```

In 3.0:
```rust
let mut computed_key = "foo".to_owned();
computed_key.push_str("bar");
// Non-static values need to be checked at runtime
let computed_key = CString::try_from(computed_key)?;
let doc_buf = rawdoc! {
    // String literal keys are implicitly checked at compile-time.
    "hello": "world",
    computed_key: 42,
    "regex": Regex {
        // `&CStr` implements many common traits like `ToOwned`
        pattern: cstr!("needle").to_owned(),
        options: cstr!("").to_owned(),
    }
};
```

## Conversions
In 2.x, conversions between raw types (`RawDocumentBuf`, `RawBson`, `RawArray`) and their associated reference types and equivalent core types (`Document`, `Bson`, `Array`) were via a mix of standard traits, ad-hoc functions, and in some cases not present at all.  In 3.0, all appropriate conversions are available, and all are via standard library traits.

## Clarifying encoding vs serialization
In 2.x, the API documentation, structure, and naming frequently conflated _encoding_ (directly converting Rust BSON values into BSON bytes) with _serialization_ (converting arbitrary Rust structs, including BSON values, to arbitrary formats, including BSON bytes, via the `serde` crate), and likewise for decoding vs deserialization.  This was a persistent footgun for crate users, who could easily end up using `serde` functionality when encoding or decoding would have been simpler and more efficient.

In 3.0, use of `serde` is now an optional feature, disabled by default; additionally, the functions for serialization and deserialization now have `serialize_to_` or `deserialize_from_` prefixes to make the distinction obvious at point of use.

## Documenting supported `serde` formats
The `serde` data model allows a high degree of flexibility in how data types represent themselves, and how data formats will parse and reconstruct that representation.  This flexibility comes with the downside that not all values will produce the same values when serialized and deserialized with a given format.  Because of that, for 3.0 we have clarified our compatibility policy:

The implementations of `Serialize` and `Deserialize` for BSON value types are tested with the `serde` \[de\]serializers provided by this crate and by the `serde_json` crate.  Compatibility with formats provided by other crates is not guaranteed and the data produced by serializing BSON values to other formats may change when this crate is updated.

## Lossy UTF8 text decoding
BSON text is required to be UTF8 encoded.  However, in various real-world circumstances, text strings may be truncated or contain invalid character sequences; in those circumstances, it's sometimes appropriate to use _lossy text decoding_, where invalid sequences are replaced with the Unicode replacement character.

In 2.x, this was only available for deserialization, not decoding, and had multiple overlapping APIs:
* the `Utf8LossyDeserialization` wrapper type that would cause the BSON binary deserializer to use lossy string decoding for the wrapped type,
* `from_slice_utf8_lossy` / `from_reader_utf8_lossy`, functions to deserialize arbitrary types from BSON with lossy string decoding
* `Document::from_reader_utf8_lossy`, deserializing a `Document` from a byte stream with lossy string decoding

In 3.0, this API has updated to be simpler and to cover both decoding and deserialization:
* the `Utf8Lossy` wraper type provides the same functionality as `Utf8LossyDeserialization` from 2.x
* a `RawDocument` can be decoded into a `Utf8Lossy<Document>` via `TryFrom`
* the `RawElement::value_utf8_lossy` allows low-level element-by-element lossy text decoding

## Serde helpers
The BSON crate provides a number of helper functions to allow \[de\]serializing common types like `ObjectId` or `DateTime` in other useful formats.  For 3.0, these have been updated to work with the `serde_as` annotation provided by the `serde_with` crate; this allows substantially more flexibility and composition of the annotated field types.

## Smaller changes
Finally, we made a few small changes for API consistency:
* `append` and `append_ref` have been merged; `append` now accepts both owned and borrowed values,
* `RawElement::len` has been renamed to `RawElement::size` to better reflect its purpose.
