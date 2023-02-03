//! [BSON Decimal128](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst) data type representation

use std::{convert::TryInto, fmt};

use bitvec::prelude::*;

/// Struct representing a BSON Decimal128 type.
///
/// Currently, this type can only be used to round-trip through BSON. See
/// [RUST-36](https://jira.mongodb.org/browse/RUST-36) to track the progress towards a complete implementation.
#[derive(Copy, Clone, PartialEq)]
pub struct Decimal128 {
    /// BSON bytes containing the decimal128. Stored for round tripping.
    pub(crate) bytes: [u8; 16],
}

impl Decimal128 {
    /// Constructs a new `Decimal128` from the provided raw byte representation.
    pub fn from_bytes(bytes: [u8; 128 / 8]) -> Self {
        Self { bytes }
    }

    /// Returns the raw byte representation of this `Decimal128`.
    pub fn bytes(&self) -> [u8; 128 / 8] {
        self.bytes
    }

    pub(crate) fn deserialize_from_slice<E: serde::de::Error>(
        bytes: &[u8],
    ) -> std::result::Result<Self, E> {
        let arr: [u8; 128 / 8] = bytes.try_into().map_err(E::custom)?;
        Ok(Decimal128 { bytes: arr })
    }
}

impl fmt::Debug for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal128(...)")
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedDecimal128 {
    sign: bool,
    kind: Decimal128Kind,
}

#[derive(Debug, Clone, PartialEq)]
enum Decimal128Kind {
    NaN { signalling: bool },
    Infinity,
    Finite {
        exponent: BitVec<u8, Msb0>,
        significand: BitVec<u8, Msb0>,
    }
}

impl ParsedDecimal128 {
    fn new(source: &Decimal128) -> Self {
        let bits = source.bytes.view_bits::<Msb0>();

        let sign = bits[0];
        let combination = &bits[1..6];
        let exp_continuation = &bits[6..18];
        let sig_continuation = &bits[18..];

        let kind = if combination[0..4].all() {
            // Special value
            if combination[4] {
                Decimal128Kind::NaN { signalling: exp_continuation[0] }
            } else {
                Decimal128Kind::Infinity
            }
        } else {
            // Finite
            let mut exponent = bitvec![u8, Msb0;];
            let mut significand = bitvec![u8, Msb0;];
            // Extract initial bits from combination
            if combination[0..1].all() {
                exponent.extend(&combination[2..4]);
                significand.extend(bits![u8, Msb0; 1, 0, 0]);
                significand.push(combination[4]);
            } else {
                exponent.extend(&combination[0..1]);
                significand.push(false);
                significand.extend(&combination[2..5]);
            }
            exponent.extend(exp_continuation);
            significand.extend(sig_continuation);
            Decimal128Kind::Finite { exponent, significand }
        };
        ParsedDecimal128 { sign, kind }
    }
}

impl fmt::Display for ParsedDecimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.sign {
            write!(f, "-")?;
        }
        match &self.kind {
            Decimal128Kind::NaN { signalling } => {
                if *signalling {
                    write!(f, "s")?;
                }
                write!(f, "NaN")?;
            }
            Decimal128Kind::Infinity => write!(f, "Infinity")?,
            Decimal128Kind::Finite { exponent, significand } => {
                let coeff_str = format!("{}", significand.load_be::<u128>());
                let exp_val = exponent.load_be::<i16>();
                let adj_exp = exp_val + (coeff_str.len() as i16) - 1;
                if exp_val <= 0 && adj_exp >= -6 {
                    // Plain notation
                } else {
                    // Exponential notation
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_infinity() {
        let val = Decimal128::from_bytes([
            0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        let parsed = ParsedDecimal128::new(&val);
        assert_eq!(parsed, ParsedDecimal128 {
            sign: true,
            kind: Decimal128Kind::Infinity,
        });
    }
}