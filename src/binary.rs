#! Module containing functionality related to BSON binary values.
mod vector;

use std::{
    convert::TryFrom,
    fmt::{self, Display},
};

use crate::{
    base64,
    error::{Error, Result},
    spec::BinarySubtype,
    RawBinaryRef,
};

pub use vector::{PackedBitVector, Vector};

/// Represents a BSON binary value.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Binary {
    /// The subtype of the bytes.
    pub subtype: BinarySubtype,

    /// The binary bytes.
    pub bytes: Vec<u8>,
}

impl Display for Binary {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Binary({:#x}, {})",
            u8::from(self.subtype),
            base64::encode(&self.bytes)
        )
    }
}

impl Binary {
    /// Creates a [`Binary`] from a base64 string and optional [`BinarySubtype`]. If the
    /// `subtype` argument is [`None`], the [`Binary`] constructed will default to
    /// [`BinarySubtype::Generic`].
    ///
    /// ```rust
    /// # use bson::{Binary, error::Result};
    /// # fn example() -> Result<()> {
    /// let input = base64::encode("hello");
    /// let binary = Binary::from_base64(input, None)?;
    /// println!("{:?}", binary);
    /// // binary: Binary { subtype: Generic, bytes: [104, 101, 108, 108, 111] }
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_base64(
        input: impl AsRef<str>,
        subtype: impl Into<Option<BinarySubtype>>,
    ) -> Result<Self> {
        let bytes = base64::decode(input.as_ref()).map_err(Error::binary)?;
        let subtype = match subtype.into() {
            Some(s) => s,
            None => BinarySubtype::Generic,
        };
        Ok(Binary { subtype, bytes })
    }

    #[cfg(feature = "serde")]
    pub(crate) fn from_extended_doc(doc: &crate::Document) -> Option<Self> {
        let binary_doc = doc.get_document("$binary").ok()?;

        if let Ok(bytes) = binary_doc.get_str("base64") {
            let bytes = base64::decode(bytes).ok()?;
            let subtype = binary_doc.get_str("subType").ok()?;
            let subtype = hex::decode(subtype).ok()?;
            if subtype.len() == 1 {
                Some(Self {
                    bytes,
                    subtype: subtype[0].into(),
                })
            } else {
                None
            }
        } else {
            // in non-human-readable mode, RawBinary will serialize as
            // { "$binary": { "bytes": <bytes>, "subType": <i32> } };
            let binary = binary_doc.get_binary_generic("bytes").ok()?;
            let subtype = binary_doc.get_i32("subType").ok()?;

            Some(Self {
                bytes: binary.clone(),
                subtype: u8::try_from(subtype).ok()?.into(),
            })
        }
    }

    /// Borrow the contents as a [`RawBinaryRef`].
    pub fn as_raw_binary(&self) -> RawBinaryRef<'_> {
        RawBinaryRef {
            bytes: self.bytes.as_slice(),
            subtype: self.subtype,
        }
    }
}
