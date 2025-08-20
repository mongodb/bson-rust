//! UUID support for BSON.
//!
//! ## The [`crate::Uuid`] type
//!
//! The BSON format supports UUIDs via the "binary" type with the UUID subtype (4).
//! To facilitate working with these UUID-subtyped binary values, this crate provides a
//! [`crate::Uuid`] type, whose `serde` implementation automatically serializes to and deserializes
//! from binary values with subtype 4.
//!
//! The popular [`uuid`](https://docs.rs/uuid) crate also provides a
//! [UUID type](https://docs.rs/uuid/latest/uuid/struct.Uuid.html),
//! though its `serde` implementation does not produce or parse subtype 4
//! binary values. Instead, when serialized with `bson::to_bson`, it produces as a string, and when
//! serialized with `bson::to_vec`, it produces a binary value with subtype _0_ rather than 4.
//! Because of this, it is highly recommended to use the [`crate::Uuid`] type when working with BSON
//! instead of the `uuid` crate's [`Uuid`], since [`crate::Uuid`] correctly produces subtype 4
//! binary values via either serialization function.
//!
//! e.g.
//!
//! ``` rust
//! # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
//! # {
//! # use uuid as uuid;
//! use bson::doc;
//! use serde::{Serialize, Deserialize};
//! use serde_with::serde_as;
//!
//!
//! #[serde_as]
//! #[derive(Serialize, Deserialize)]
//! struct Foo {
//!     /// serializes as a String or subtype 0 BSON binary, depending
//!     /// on whether `bson::to_bson` or `bson::to_vec` is used.
//!     uuid: uuid::Uuid,
//!
//!     /// serializes as a BSON binary with subtype 4 when either is used.
//!     bson_uuid: bson::Uuid,
//!
//!     /// serializes as a BSON binary with subtype 4 when either is used.
//!     /// this requires the "uuid-1" and "serde_with-3" feature flags
//!     #[serde_as(as = "bson::serde_helpers::uuid_1::AsBinary")]
//!     uuid_as_bson: uuid::Uuid,
//! }
//! # };
//! ```
//!
//! ## The `uuid-1` feature flag
//!
//! To facilitate the conversion between [`crate::Uuid`] values and the `uuid` crate's [`Uuid`]
//! values, the `uuid-1` feature flag can be enabled. This flag exposes a number of convenient
//! conversions, including the `crate::Uuid::to_uuid_1` method and the `From<uuid::Uuid>`
//! implementation for [`Bson`], which allows the `uuid` crate's [`Uuid`] values to be used in the
//! `doc!` and `bson!` macros.
//!
//! ```
//! # #[cfg(feature = "uuid-1")]
//! # {
//! # use uuid as uuid;
//! use bson::doc;
//!
//! // this automatic conversion does not require any feature flags
//! let query = doc! {
//!     "uuid": bson::Uuid::new(),
//! };
//!
//! // but this automatic conversion requires the "uuid-1" feature flag
//! let query = doc! {
//!     "uuid": uuid::Uuid::new_v4(),
//! };
//!
//! // this also requires the "uuid-1" feature flag.
//! let uuid = bson::Uuid::new().to_uuid_1();
//! # };
//! ```
//!
//! ## Serde conversion
//!
//! Fields using the [`uuid::Uuid`] type can be (de)serialized as BSON using the converters provided
//! in [`serde_helpers`](crate::serde_helpers):
//!
//! ```
//! # #[cfg(feature = "uuid-1")]
//! # {
//! use uuid;
//! use serde::{Deserialize, Serialize};
//! use bson::{doc, serde_helpers::uuid_1};
//! #[derive(Deserialize, Serialize, PartialEq, Debug)]
//! struct Foo {
//!   /// Serializes as a BSON binary rather than using [`uuid::Uuid`]'s serialization
//!   #[serde(with = "uuid_1::AsBinary")]
//!   as_bson: uuid::Uuid,
//! }
//!
//! let foo = Foo {
//!   as_bson: uuid::Uuid::new_v4(),
//! };
//!
//! let expected = doc! {
//!   "as_bson": bson::Uuid::from(foo.as_bson),
//! };
//!
//! assert_eq!(bson::serialize_to_document(&foo)?, expected);
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## The `serde_with-3` feature flag
//!
//! The `serde_with-3` feature can be enabled to support more ergonomic serde attributes for
//! conversions. The main benefit of this compared to the regular `serde_helpers` is that
//! `serde_with-3` can handle nested [`uuid::Uuid`] values (e.g. in [`Option`]), whereas the former
//! only works on fields that are exactly [`uuid::Uuid`].
//! ```
//! # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
//! # {
//! # use uuid;
//! use serde::{Deserialize, Serialize};
//! use bson::{doc, serde_helpers::uuid_1};
//!
//! #[serde_with::serde_as]
//! #[derive(Deserialize, Serialize, PartialEq, Debug)]
//! struct Foo {
//!   /// Serializes as a BSON binary rather than using [`uuid::Uuid`]'s serialization
//!   #[serde_as(as = "Option<uuid_1::AsBinary>")]
//!   as_bson: Option<uuid::Uuid>,
//! }
//!
//! let foo = Foo {
//!   as_bson: Some(uuid::Uuid::new_v4()),
//! };
//!
//! let expected = doc! {
//!   "as_bson": bson::Uuid::from(foo.as_bson.unwrap()),
//! };
//!
//! assert_eq!(bson::serialize_to_document(&foo)?, expected);
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Using [`crate::Uuid`] with non-BSON formats
//!
//! [`crate::Uuid`]'s `serde` implementation is the same as [`uuid::Uuid`]'s
//! for non-BSON formats such as JSON:
//! ``` rust
//! # #[cfg(feature = "uuid-1")]
//! # {
//! # use uuid as uuid;
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize, Deserialize)]
//! # struct Foo {
//! #   uuid: uuid::Uuid,
//! #   bson_uuid: bson::Uuid,
//! # }
//! use serde_json::json;
//!
//! let uuid = uuid::Uuid::new_v4();
//! let bson_uuid: bson::Uuid = uuid.into();
//! let foo = Foo { uuid, bson_uuid, };
//!
//! let json = serde_json::to_value(&foo)?;
//! assert_eq!(json, json!({ "uuid": uuid.to_string(), "bson_uuid": uuid.to_string() }));
//! # }
//! # Ok::<(), Box::<dyn std::error::Error>>(())
//! ```
#[cfg(test)]
mod test;

use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::{
    error::{Error, Result},
    spec::BinarySubtype,
    Binary,
    Bson,
};

/// Special type name used in the [`Uuid`] serialization implementation to indicate a BSON
/// UUID is being serialized or deserialized. The BSON serializers/deserializers will handle this
/// name specially, but other serializers/deserializers will just ignore it and use [`uuid::Uuid`]'s
/// serde integration.
#[cfg(feature = "serde")]
pub(crate) const UUID_NEWTYPE_NAME: &str = "$__bson_private_uuid";

/// A struct modeling a BSON UUID value (i.e. a Binary value with subtype 4).
///
/// This type should be used instead of [`uuid::Uuid`](https://docs.rs/uuid/latest/uuid/struct.Uuid.html)
/// when serializing to or deserializing from BSON, since
/// [`uuid::Uuid`](https://docs.rs/uuid/latest/uuid/struct.Uuid.html)'s `serde` implementation doesn't
/// produce or parse BSON UUIDs.
///
/// To enable interop with the [`Uuid`] type from the `uuid` crate, enable the `uuid-0_8` feature
/// flag.
///
/// For more information on the usage of this type, see the [`uuid`] module-level documentation.
///
/// Note: due to an issue in serde (see [here](https://github.com/serde-rs/serde/issues/2106)), this type
/// will also allow deserialization from 16 byte + subtype 0 Binary values in BSON if part of a
/// `#[serde(flatten)]` chain. This behavior shouldn't be relied upon as it may be fixed at some
/// point in the future.
#[derive(Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord)]
pub struct Uuid {
    uuid: uuid::Uuid,
}

impl Uuid {
    /// Creates a random UUID.
    ///
    /// This uses the operating system's RNG as the source of random numbers. If you'd like to use a
    /// custom generator, generate random bytes and pass them to [`Uuid::from_bytes`] instead.
    pub fn new() -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
        }
    }

    /// Creates a [`Uuid`] using the supplied big-endian bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::from_external_uuid(uuid::Uuid::from_bytes(bytes))
    }

    /// Creates a [`Uuid`] from the provided hex string.
    pub fn parse_str(input: impl AsRef<str>) -> Result<Self> {
        let uuid = uuid::Uuid::parse_str(input.as_ref()).map_err(Error::invalid_uuid_string)?;
        Ok(Self::from_external_uuid(uuid))
    }

    pub(crate) const fn from_external_uuid(uuid: uuid::Uuid) -> Self {
        Self { uuid }
    }

    /// Returns an array of 16 bytes containing the [`Uuid`]'s data.
    pub const fn bytes(self) -> [u8; 16] {
        *self.uuid.as_bytes()
    }
}

impl Default for Uuid {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Uuid {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

#[cfg(feature = "uuid-1")]
impl Uuid {
    /// Create a [`Uuid`] from a [`uuid::Uuid`](https://docs.rs/uuid/0.8/uuid/struct.Uuid.html) from
    /// the [`uuid`](https://docs.rs/uuid/0.8) crate.
    pub fn from_uuid_1(uuid: uuid::Uuid) -> Self {
        Self::from_external_uuid(uuid)
    }

    /// Convert this [`Uuid`] to a [`uuid::Uuid`](https://docs.rs/uuid/0.8/uuid/struct.Uuid.html) from
    /// the [`uuid`](https://docs.rs/uuid/0.8) crate.
    pub fn to_uuid_1(self) -> uuid::Uuid {
        self.uuid
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(UUID_NEWTYPE_NAME, &self.uuid)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer.deserialize_newtype_struct(UUID_NEWTYPE_NAME, crate::de::BsonVisitor)? {
            // Need to support deserializing from generic subtypes for non-BSON formats.
            // When using the BSON deserializer, the newtype name will ensure the subtype is only
            // ever BinarySubtype::Uuid.
            Bson::Binary(b)
                if matches!(b.subtype, BinarySubtype::Uuid | BinarySubtype::Generic) =>
            {
                let uuid =
                    uuid::Uuid::from_slice(b.bytes.as_slice()).map_err(serde::de::Error::custom)?;
                Ok(Self::from_external_uuid(uuid))
            }
            Bson::Binary(b) if b.subtype == BinarySubtype::UuidOld => {
                Err(serde::de::Error::custom(
                    "received legacy UUID (subtype 3) but expected regular UUID (subtype 4)",
                ))
            }
            Bson::String(s) => {
                use std::str::FromStr as _;
                let uuid = uuid::Uuid::from_str(s.as_str()).map_err(serde::de::Error::custom)?;
                Ok(Self::from_external_uuid(uuid))
            }
            b => Err(serde::de::Error::invalid_type(b.as_unexpected(), &"a UUID")),
        }
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.uuid.fmt(f)
    }
}

impl std::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&self.uuid, f)
    }
}

impl From<Uuid> for Binary {
    fn from(uuid: Uuid) -> Self {
        Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.bytes().to_vec(),
        }
    }
}

impl From<Uuid> for Bson {
    fn from(u: Uuid) -> Self {
        Bson::Binary(u.into())
    }
}

#[cfg(feature = "uuid-1")]
impl From<uuid::Uuid> for Uuid {
    fn from(u: uuid::Uuid) -> Self {
        Self::from_uuid_1(u)
    }
}

#[cfg(feature = "uuid-1")]
impl From<Uuid> for uuid::Uuid {
    fn from(s: Uuid) -> Self {
        s.to_uuid_1()
    }
}

/// Enum of the possible representations to use when converting between [`Uuid`] and [`Binary`].
/// This enum is necessary because the different drivers used to have different ways of encoding
/// UUIDs, with the BSON subtype: 0x03 (UUID old).
/// If a UUID has been serialized with a particular representation, it MUST
/// be deserialized with the same representation.
///
/// Example:
/// ```
/// use bson::{Binary, uuid::{Uuid, UuidRepresentation}};
///
/// let uuid = Uuid::parse_str("00112233445566778899AABBCCDDEEFF")?;
/// let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::PythonLegacy);
///
/// // This conversion fails, since the binary holds a PythonLegacy UUID, so we're required to specify
/// // that.
/// assert!(bin.to_uuid().is_err());
///
/// // This conversion succeeds, since we specified the correct representation.
/// let new_uuid = bin.to_uuid_with_representation(UuidRepresentation::PythonLegacy)?;
/// assert_eq!(new_uuid, uuid);
///
/// # Ok::<(), Box::<dyn std::error::Error>>(())
/// ```
#[non_exhaustive]
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum UuidRepresentation {
    /// The canonical representation of UUIDs in BSON (binary with subtype 0x04)
    Standard,
    /// The legacy representation of UUIDs in BSON used by the C# driver (binary subtype 0x03)
    CSharpLegacy,
    /// The legacy representation of UUIDs in BSON used by the Java driver (binary subtype 0x03)
    JavaLegacy,
    /// The legacy representation of UUIDs in BSON used by the Python driver, which is the same
    /// format as STANDARD, but has binary subtype 0x03
    PythonLegacy,
}

impl Binary {
    /// Serializes a [`Uuid`] into BSON [`Binary`] type
    pub fn from_uuid(uuid: Uuid) -> Self {
        Binary::from(uuid)
    }

    /// Serializes a [`Uuid`] into BSON binary type and takes the desired representation as a
    /// parameter. `Binary::from_uuid_with_representation(uuid, UuidRepresentation::Standard)` is
    /// equivalent to `Binary::from_uuid(uuid)`.
    ///
    /// See the documentation for [`UuidRepresentation`] for more information on the possible
    /// representations.
    pub fn from_uuid_with_representation(uuid: Uuid, rep: UuidRepresentation) -> Self {
        match rep {
            UuidRepresentation::Standard => Binary::from_uuid(uuid),
            UuidRepresentation::CSharpLegacy => {
                let mut bytes = uuid.bytes().to_vec();
                bytes[0..4].reverse();
                bytes[4..6].reverse();
                bytes[6..8].reverse();
                Binary {
                    subtype: BinarySubtype::UuidOld,
                    bytes,
                }
            }
            UuidRepresentation::PythonLegacy => Binary {
                subtype: BinarySubtype::UuidOld,
                bytes: uuid.bytes().to_vec(),
            },
            UuidRepresentation::JavaLegacy => {
                let mut bytes = uuid.bytes().to_vec();
                bytes[0..8].reverse();
                bytes[8..16].reverse();
                Binary {
                    subtype: BinarySubtype::UuidOld,
                    bytes,
                }
            }
        }
    }

    /// Deserializes a BSON [`Binary`] type into a [`Uuid`] according to the provided
    /// representation. If the representation does not match the [`Binary`], an error will be
    /// returned.
    ///
    /// See the documentation for [`UuidRepresentation`] for more information on the possible
    /// representations.
    pub fn to_uuid_with_representation(&self, rep: UuidRepresentation) -> Result<Uuid> {
        // If representation is non-standard, then its subtype must be UuidOld
        if rep != UuidRepresentation::Standard && self.subtype != BinarySubtype::UuidOld {
            return Err(Error::uuid_representation_mismatch(
                rep,
                self.subtype,
                BinarySubtype::UuidOld,
            ));
        }
        // If representation is standard, then its subtype must be Uuid
        if rep == UuidRepresentation::Standard && self.subtype != BinarySubtype::Uuid {
            return Err(Error::uuid_representation_mismatch(
                rep,
                self.subtype,
                BinarySubtype::UuidOld,
            ));
        }
        // Must be 16 bytes long
        if self.bytes.len() != 16 {
            return Err(Error::invalid_uuid_length(self.bytes.len()));
        }
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&self.bytes);
        Ok(match rep {
            UuidRepresentation::Standard => Uuid::from_bytes(buf),
            UuidRepresentation::CSharpLegacy => {
                buf[0..4].reverse();
                buf[4..6].reverse();
                buf[6..8].reverse();
                Uuid::from_bytes(buf)
            }
            UuidRepresentation::PythonLegacy => Uuid::from_bytes(buf),
            UuidRepresentation::JavaLegacy => {
                buf[0..8].reverse();
                buf[8..16].reverse();
                Uuid::from_bytes(buf)
            }
        })
    }

    /// Deserializes a BSON [`Binary`] type into a [`Uuid`] using the standard
    /// representation.
    pub fn to_uuid(&self) -> Result<Uuid> {
        self.to_uuid_with_representation(UuidRepresentation::Standard)
    }
}

#[cfg(feature = "uuid-1")]
impl From<uuid::Uuid> for Binary {
    fn from(uuid: uuid::Uuid) -> Self {
        Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        }
    }
}
