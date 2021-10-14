use std::{
    fmt::{self, Display},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{spec::BinarySubtype, Binary, Bson};

pub(crate) const UUID_NEWTYPE_NAME: &'static str = "BsonUuid";

/// A struct modeling a BSON UUID value (i.e. a Binary value with subtype 4).
///
/// This struct can be used to ensure UUID values get serialized to and from BSON correctly.
/// It should be used instead of [`uuid::Uuid`]() when serializing to BSON TODO
///
/// To enable interop with the `Uuid` type from the `uuid` crate, enable the `uuid-0_8` feature flag.
#[derive(Clone, Copy, PartialEq)]
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
        let uuid = uuid::Uuid::parse_str(input.as_ref()).map_err(|e| Error::InvalidUuidString {
            message: e.to_string(),
        })?;
        Ok(Self::from_external_uuid(uuid))
    }

    pub(crate) const fn from_external_uuid(uuid: uuid::Uuid) -> Self {
        Self { uuid }
    }

    /// Returns an array of 16 bytes containing the [`Uuid`]'s data.
    pub fn as_bytes(&self) -> [u8; 16] {
        *self.uuid.as_bytes()
    }
}

#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
impl Uuid {
    /// Convert this [`Uuid`] to a [`uuid::Uuid`] from the [`uuid`](https://docs.rs/uuid/latest) crate.
    pub fn from_uuid_0_8(uuid: uuid::Uuid) -> Self {
        Self::from_external_uuid(uuid)
    }

    /// Convert this [`Uuid`] to a [`uuid::Uuid`] from the [`uuid`](https://docs.rs/uuid/latest) crate.
    pub fn to_uuid_0_8(self) -> uuid::Uuid {
        self.uuid
    }
}

impl Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("BsonUuid", &self.uuid)
    }
}

impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            // need to support deserializing from generic subtypes for non-BSON formats.
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
                let uuid = uuid::Uuid::from_str(s.as_str()).map_err(serde::de::Error::custom)?;
                Ok(Self::from_external_uuid(uuid))
            }
            b => Err(serde::de::Error::invalid_type(b.as_unexpected(), &"a UUID")),
        }
    }
}

impl std::hash::Hash for Uuid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.uuid.fmt(f)
    }
}

impl std::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.uuid.fmt(f)
    }
}

impl From<Uuid> for Binary {
    fn from(uuid: Uuid) -> Self {
        Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        }
    }
}

impl From<Uuid> for Bson {
    fn from(u: Uuid) -> Self {
        Bson::Binary(u.into())
    }
}

#[cfg(feature = "uuid-0_8")]
impl From<uuid::Uuid> for Uuid {
    fn from(u: uuid::Uuid) -> Self {
        Self::from_external_uuid(u)
    }
}

#[cfg(feature = "uuid-0_8")]
impl From<Uuid> for uuid::Uuid {
    fn from(s: Uuid) -> Self {
        s.to_uuid_0_8()
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
/// use crate::{bson::UuidRepresentation, bson::Binary};
/// use uuid::Uuid;
/// let uuid = Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
/// let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::PythonLegacy);
/// let new_uuid = bin.to_uuid();
/// assert!(new_uuid.is_err());
/// let new_uuid = bin.to_uuid_with_representation(UuidRepresentation::PythonLegacy);
/// assert!(new_uuid.is_ok());
/// assert_eq!(new_uuid.unwrap(), uuid);
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
    /// Serializes a [Uuid](https://docs.rs/uuid/0.8.2/uuid/) into BSON [`Binary`] type
    pub fn from_uuid(uuid: Uuid) -> Self {
        Binary::from(uuid)
    }

    /// Serializes a UUID into BSON binary type and takes the desired representation as a parameter.
    /// `Binary::from_uuid_with_representation(uuid, UuidRepresentation::Standard)` is equivalent
    /// to `Binary::from_uuid(uuid)`.
    ///
    /// See the documentation for [`UuidRepresentation`] for more information on the possible
    /// representations.
    pub fn from_uuid_with_representation(uuid: Uuid, rep: UuidRepresentation) -> Self {
        match rep {
            UuidRepresentation::Standard => Binary::from_uuid(uuid),
            UuidRepresentation::CSharpLegacy => {
                let mut bytes = uuid.as_bytes().to_vec();
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
                bytes: uuid.as_bytes().to_vec(),
            },
            UuidRepresentation::JavaLegacy => {
                let mut bytes = uuid.as_bytes().to_vec();
                bytes[0..8].reverse();
                bytes[8..16].reverse();
                Binary {
                    subtype: BinarySubtype::UuidOld,
                    bytes,
                }
            }
        }
    }

    /// Deserializes a BSON [`Binary`] type into a [Uuid](https://docs.rs/uuid/0.8.2/uuid/), takes the representation
    /// with which the [`Binary`] was serialized.
    ///
    /// See the documentation for [`UuidRepresentation`] for more information on the possible
    /// representations.
    pub fn to_uuid_with_representation(&self, rep: UuidRepresentation) -> Result<Uuid> {
        // If representation is non-standard, then its subtype must be UuidOld
        if rep != UuidRepresentation::Standard && self.subtype != BinarySubtype::UuidOld {
            return Err(Error::RepresentationMismatch {
                requested_representation: rep,
                actual_binary_subtype: self.subtype,
                expected_binary_subtype: BinarySubtype::UuidOld,
            });
        }
        // If representation is standard, then its subtype must be Uuid
        if rep == UuidRepresentation::Standard && self.subtype != BinarySubtype::Uuid {
            return Err(Error::RepresentationMismatch {
                requested_representation: rep,
                actual_binary_subtype: self.subtype,
                expected_binary_subtype: BinarySubtype::Uuid,
            });
        }
        // Must be 16 bytes long
        if self.bytes.len() != 16 {
            // return Err(crate::de::Error::custom(format!(
            //     "expected UUID to contain 16 bytes, instead got {}",
            //     self.bytes.len()
            // )));
            return Err(Error::InvalidLength {
                length: self.bytes.len(),
            });
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

    /// Deserializes a BSON [`Binary`] type into a [Uuid](https://docs.rs/uuid/0.8.2/uuid/) using the standard
    /// representation.
    pub fn to_uuid(&self) -> Result<Uuid> {
        self.to_uuid_with_representation(UuidRepresentation::Standard)
    }
}

#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
impl From<uuid::Uuid> for Binary {
    fn from(uuid: uuid::Uuid) -> Self {
        Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        }
    }
}

/// Errors that can occur during [`Uuid`] construction and generation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    #[non_exhaustive]
    InvalidUuidString { message: String },

    #[non_exhaustive]
    RepresentationMismatch {
        expected_binary_subtype: BinarySubtype,
        actual_binary_subtype: BinarySubtype,
        requested_representation: UuidRepresentation,
    },

    #[non_exhaustive]
    InvalidLength { length: usize },
}

/// Alias for `Result<T, bson::uuid::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidUuidString { message } => {
                write!(fmt, "{}", message)
            }
            Error::RepresentationMismatch {
                expected_binary_subtype,
                actual_binary_subtype,
                requested_representation,
            } => {
                write!(
                    fmt,
                    "expected {:?} when converting to UUID with {:?}, isntead got {:?}",
                    expected_binary_subtype, requested_representation, actual_binary_subtype
                )
            }
            Error::InvalidLength { length } => {
                write!(
                    fmt,
                    "expected UUID to contain 16 bytes, instead got {}",
                    length
                )
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod test {
    use crate::{uuid::Uuid, Document};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[test]
    fn serialization() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct U {
            uuid: Uuid,
        }

        let u = U {
            uuid: Uuid(uuid::Uuid::new_v4()),
        };
        let bytes = crate::to_vec(&u).unwrap();

        let doc: Document = crate::from_slice(bytes.as_slice()).unwrap();
        assert_eq!(doc, doc! { "uuid": u.uuid });

        let u_roundtrip: U = crate::from_slice(bytes.as_slice()).unwrap();
        assert_eq!(u_roundtrip, u);

        let json = serde_json::to_value(&u).unwrap();
        assert_eq!(json, json!({ "uuid": u.uuid.0.to_string() }));

        let u_roundtrip_json: U = serde_json::from_value(json).unwrap();
        assert_eq!(u_roundtrip_json, u);
    }
}
