# Migrating from 2.x to 3.0
3.0 updates several APIs in backwards-incompatible ways; in most cases these changes should require only minor updates in application code.

## Unified error hierarchy
In 2.x, many crate submodules had their own `Error` types, with inconsistent conversions between those types.  In 3.0, the crate defines a single `bson::error::Error` type, with fields for values common across errors like message or associated key, and a `kind` enum that provides granular root cause information.

## `&CStr`
The [bson spec](https://bsonspec.org/spec.html) describes a "cstring" type as UTF-8, with the exception that it cannot contain byte `0` (which is otherwise a valid UTF8 byte value).  This type is used in bson for the keys of documents and for the pattern and options of regular expressions; all other string values include a length header and allow full UTF8 values, including `0` bytes.

In 2.x, attempting to use `rawdoc!` or `RawDocumentBuf::append` with a key or regular expression containing a `0` byte will panic.

3.0 introduces the `&CStr`/`CString` types to represent the "cstring" type as described in the spec; these types are analogous to `&str` and `String` but validate on construction that no `0` byte is contained.  The `cstr!` macro will construct a literal `&``static CStr` from a `&``static str` with compile-time validation, and `TryFrom` impls are provided for run-time validation.

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

In 3.x:
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

## `append` and `append_ref`

In 2.x, `RawDocumentBuf` provided both `append` and `append_ref` for appending owned or borrowed values respectively.  In 3.x, `append` can accept both and `append_ref` has been removed.

## Clarifying encoding vs serialization

In 2.x, the API documentation and naming frequently conflated _encoding_ (directly converting Rust BSON values into BSON bytes) with _serialization_ (converting arbitrary Rust structs, including BSON values, to arbitrary formats, including BSON bytes, via the `serde` crate), and likewise for decoding vs deserialization.  This was a persistent footgun for crate users, who could easily end up using `serde` functionality when encoding or decoding would have been simpler and more efficient.

In 3.x, use of `serde` is now an optional feature, disabled by default; additionally, the functions for serialization and deserialization now have `serialize_to_` or `deserialize_from_` prefixes to make the distinction obvious at point of use.