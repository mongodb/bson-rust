use std::{
    convert::{TryFrom, TryInto},
    mem::size_of,
};

use serde::{Deserialize, Serialize};

use super::{Binary, Error, Result};
use crate::{spec::BinarySubtype, Bson, RawBson};

const INT8: u8 = 0x03;
const FLOAT32: u8 = 0x27;
const PACKED_BIT: u8 = 0x10;

/// A vector of numeric values. This type can be converted into a [`Binary`] of subtype
/// [`BinarySubtype::Vector`].
///
/// ```rust
/// # use bson::binary::{Binary, Vector};
/// let vector = Vector::Int8(vec![0, 1, 2]);
/// let binary = Binary::from(vector);
/// ```
///
/// `Vector` serializes to and deserializes from a `Binary`.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::{binary::{Result, Vector}, spec::ElementType};
/// #[derive(Serialize, Deserialize)]
/// struct Data {
///     vector: Vector,
/// }
///
/// let data = Data { vector: Vector::Int8(vec![0, 1, 2]) };
/// let document = bson::to_document(&data).unwrap();
/// assert_eq!(document.get("vector").unwrap().element_type(), ElementType::Binary);
///
/// let data: Data = bson::from_document(document).unwrap();
/// assert_eq!(data.vector, Vector::Int8(vec![0, 1, 2]));
/// ```
///
/// See the
/// [specification](https://github.com/mongodb/specifications/blob/master/source/bson-binary-vector/bson-binary-vector.md)
/// for more details.
#[derive(Clone, Debug, PartialEq)]
pub enum Vector {
    /// A vector of `i8` values.
    Int8(Vec<i8>),

    /// A vector of `f32` values.
    Float32(Vec<f32>),

    /// A vector of packed bits. See [`PackedBitVector::new`] for more details.
    PackedBit(PackedBitVector),
}

/// A vector of packed bits. This type can be constructed by calling [`PackedBitVector::new`].
#[derive(Clone, Debug, PartialEq)]
pub struct PackedBitVector {
    vector: Vec<u8>,
    padding: u8,
}

impl PackedBitVector {
    /// Construct a new `PackedBitVector`. Each `u8` value in the provided `vector` represents 8
    /// single-bit elements in little-endian format. For example, the following vector:
    ///
    /// ```rust
    /// # use bson::binary::{Result, PackedBitVector};
    /// # fn main() -> Result<()> {
    /// let packed_bits = vec![238, 224];
    /// let vector = PackedBitVector::new(packed_bits, 0)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// represents a 16-bit vector containing the following values:
    ///
    /// ```text
    /// [1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 0]
    /// ```
    ///
    /// Padding can optionally be specified to ignore a number of least-significant bits in the
    /// final byte. For example, the vector in the previous example with a padding of 4 would
    /// represent a 12-bit vector containing the following values:
    ///
    /// ```text
    /// [1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0]
    /// ```
    ///
    /// Padding must be within 0-7 inclusive. Padding must be 0 or unspecified if the provided
    /// vector is empty.
    pub fn new(vector: Vec<u8>, padding: impl Into<Option<u8>>) -> Result<Self> {
        let padding = padding.into().unwrap_or(0);
        if !(0..8).contains(&padding) {
            return Err(Error::Vector {
                message: format!("padding must be within 0-7 inclusive, got {}", padding),
            });
        }
        if padding != 0 && vector.is_empty() {
            return Err(Error::Vector {
                message: format!(
                    "cannot specify non-zero padding if the provided vector is empty, got {}",
                    padding
                ),
            });
        }
        Ok(Self { vector, padding })
    }
}

impl Vector {
    /// Construct a [`Vector`] from the given bytes. See the
    /// [specification](https://github.com/mongodb/specifications/blob/master/source/bson-binary-vector/bson-binary-vector.md#specification)
    /// for details on the expected byte format.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let bytes = bytes.as_ref();

        if bytes.len() < 2 {
            return Err(Error::Vector {
                message: format!(
                    "the provided bytes must have a length of at least 2, got {}",
                    bytes.len()
                ),
            });
        }

        let d_type = bytes[0];
        let padding = bytes[1];
        if d_type != PACKED_BIT && padding != 0 {
            return Err(Error::Vector {
                message: format!(
                    "padding can only be specified for a packed bit vector (data type {}), got \
                     type {}",
                    PACKED_BIT, d_type
                ),
            });
        }
        let number_bytes = &bytes[2..];

        match d_type {
            INT8 => {
                let vector = number_bytes
                    .iter()
                    .map(|n| i8::from_le_bytes([*n]))
                    .collect();
                Ok(Self::Int8(vector))
            }
            FLOAT32 => {
                const F32_BYTES: usize = size_of::<f32>();

                let mut vector = Vec::new();
                for chunk in number_bytes.chunks(F32_BYTES) {
                    let bytes: [u8; F32_BYTES] = chunk.try_into().map_err(|_| Error::Vector {
                        message: format!(
                            "f32 vector values must be {} bytes, got {:?}",
                            F32_BYTES, chunk,
                        ),
                    })?;
                    vector.push(f32::from_le_bytes(bytes));
                }
                Ok(Self::Float32(vector))
            }
            PACKED_BIT => {
                let packed_bit_vector = PackedBitVector::new(number_bytes.to_vec(), padding)?;
                Ok(Self::PackedBit(packed_bit_vector))
            }
            other => Err(Error::Vector {
                message: format!("unsupported vector data type: {}", other),
            }),
        }
    }

    fn d_type(&self) -> u8 {
        match self {
            Self::Int8(_) => INT8,
            Self::Float32(_) => FLOAT32,
            Self::PackedBit(_) => PACKED_BIT,
        }
    }

    fn padding(&self) -> u8 {
        match self {
            Self::Int8(_) => 0,
            Self::Float32(_) => 0,
            Self::PackedBit(PackedBitVector { padding, .. }) => *padding,
        }
    }
}

impl From<&Vector> for Binary {
    fn from(vector: &Vector) -> Self {
        let d_type = vector.d_type();
        let padding = vector.padding();
        let mut bytes = vec![d_type, padding];

        match vector {
            Vector::Int8(vector) => {
                for n in vector {
                    bytes.extend_from_slice(&n.to_le_bytes());
                }
            }
            Vector::Float32(vector) => {
                for n in vector {
                    bytes.extend_from_slice(&n.to_le_bytes());
                }
            }
            Vector::PackedBit(PackedBitVector { vector, .. }) => {
                for n in vector {
                    bytes.extend_from_slice(&n.to_le_bytes());
                }
            }
        }

        Self {
            subtype: BinarySubtype::Vector,
            bytes,
        }
    }
}

impl From<Vector> for Binary {
    fn from(vector: Vector) -> Binary {
        Self::from(&vector)
    }
}

impl TryFrom<&Binary> for Vector {
    type Error = Error;

    fn try_from(binary: &Binary) -> Result<Self> {
        if binary.subtype != BinarySubtype::Vector {
            return Err(Error::Vector {
                message: format!("expected vector binary subtype, got {:?}", binary.subtype),
            });
        }
        Self::from_bytes(&binary.bytes)
    }
}

impl TryFrom<Binary> for Vector {
    type Error = Error;

    fn try_from(binary: Binary) -> std::result::Result<Self, Self::Error> {
        Self::try_from(&binary)
    }
}

// Convenience impl to allow passing a Vector directly into the doc! macro. From<&Vector> is already
// implemented by a blanket impl in src/bson.rs.
impl From<Vector> for Bson {
    fn from(vector: Vector) -> Self {
        Self::Binary(Binary::from(vector))
    }
}

// Convenience impls to allow passing a Vector directly into the rawdoc! macro
impl From<&Vector> for RawBson {
    fn from(vector: &Vector) -> Self {
        Self::Binary(Binary::from(vector))
    }
}

impl From<Vector> for RawBson {
    fn from(vector: Vector) -> Self {
        Self::from(&vector)
    }
}

impl Serialize for Vector {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let binary = Binary::from(self);
        binary.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Vector {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let binary = Binary::deserialize(deserializer)?;
        Self::try_from(binary).map_err(serde::de::Error::custom)
    }
}
