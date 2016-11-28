# bson-rs

[![Build Status](https://img.shields.io/travis/zonyitoo/bson-rs.svg)](https://travis-ci.org/zonyitoo/bson-rs)
[![crates.io](https://img.shields.io/crates/v/bson.svg)](https://crates.io/crates/bson)
[![crates.io](https://img.shields.io/crates/l/bson.svg)](https://crates.io/crates/bson)

Encoding and decoding support for BSON in Rust

## Useful links
- [API Documentation](https://docs.rs/bson/)
- [Serde](https://serde.rs/)

## Installation
This crate works with Cargo and can be found on
[crates.io](https://crates.io/crates/bson) with a `Cargo.toml` like:

```toml
[dependencies]
bson = "*"
```
## Usage

Prepare your struct for Serde serialization:
```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    #[serde(rename = "_id")]  // Use MongoDB's special primary key field name when serializing 
    pub id: String,
    pub name: String,
    pub age: u32
}
```

Serialize the struct:
```rust
use bson;

let person = Person {
    id: "12345",
    name: "Emma",
    age: 3
};

let serialized_person = bson::to_bson(&person)?;  // Serialize
 
if let bson::Bson::Document(document) = serialized {
    mongoCollection.insert_one(document, None)?;  // Insert into a MongoDB collection
} else {
    println!("Error converting the BSON object into a MongoDB document")
}
```

Deserialize the struct:
```rust
// Read the document from a MongoDB collection
let person_document = mongoCollection.find_one(Some(doc! { "_id" => "12345" }), None)?
    .expect("Document not found")

// Deserialize the document into a Person instance
let person = bson::from_bson(bson::Bson::Document(person_document))?
```
