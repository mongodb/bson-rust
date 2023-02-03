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

type Order = Msb0;

#[derive(Debug, Clone, PartialEq)]
enum Decimal128Kind {
    NaN { signalling: bool },
    Infinity,
    Finite {
        exponent: Exponent,
        significand: Significand,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Exponent([u8; 2]);

impl Exponent {
    const BIAS: i16 = 6176;

    fn raw(&self) -> u16 {
        self.0.view_bits::<Order>().load_be::<u16>()
    }

    fn value(&self) -> i16 {
        (self.raw() as i16) - Self::BIAS
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Significand([u8; 16]);

impl Significand {
    fn value(&self) -> u128 {
        self.0.view_bits::<Order>().load_be::<u128>()
    }
}

macro_rules! pdbg {
    ($expr: expr) => {
        {
            let val = $expr;
            println!("{} = {}", stringify!($expr), val);
            val
        }
    }
}

impl ParsedDecimal128 {
    fn new(source: &Decimal128) -> Self {
        let tmp: [u8; 16] = {
            let mut tmp = [0u8; 16];
            for i in 0..16 {
                tmp[i] = source.bytes[15-i];
            }
            tmp
        };
        let bits = tmp.view_bits::<Order>();
        pdbg!(&bits);

        let sign = bits[0];
        let kind = if bits[1..5].all() {
            // Special value
            if bits[5] {
                Decimal128Kind::NaN { signalling: bits[6] }
            } else {
                Decimal128Kind::Infinity
            }
        } else {
            // Finite value
            let mut exponent = [0u8; 2];
            let exponent_bits = exponent.view_bits_mut::<Order>();
            let mut significand = [0u8; 16];
            let significand_bits = &mut significand.view_bits_mut::<Order>()[14..];

            if bits[1..3].all() {
                exponent_bits[2..].copy_from_bitslice(&bits[3..17]);
                significand_bits[0..3].clone_from_bitslice(bits![1, 0, 0]);
                significand_bits[3..].copy_from_bitslice(&bits[17..]);
            } else {
                exponent_bits[2..].copy_from_bitslice(&bits[1..15]);
                significand_bits[1..].copy_from_bitslice(&bits[15..]);
            }

            pdbg!(&exponent_bits);
            pdbg!(&significand_bits);
            Decimal128Kind::Finite {
                exponent: Exponent(exponent),
                significand: Significand(significand),
            }
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
                let coeff_str = format!("{}", significand.value());
                let exp_val = exponent.value();
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
    use crate::Document;

    use super::*;

    fn dec_from_hex(s: &str) -> ParsedDecimal128 {
        let bytes = hex::decode(s).unwrap();
        let d = crate::from_slice::<Document>(&bytes).unwrap();
        ParsedDecimal128::new(&d.get_decimal128("d").unwrap())
    }

    #[test]
    fn nan() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000007C00");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::NaN { signalling: false },
        });
    }

    #[test]
    fn negative_nan() {
        let parsed = dec_from_hex("18000000136400000000000000000000000000000000FC00");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: true,
            kind: Decimal128Kind::NaN { signalling: false },
        });
    }

    #[test]
    fn snan() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000007E00");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::NaN { signalling: true },
        });
    }

    #[test]
    fn inf() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000007800");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::Infinity,
        });
    }

    #[test]
    fn finite_0() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000403000");
        let (exp, sig) = if let Decimal128Kind::Finite { exponent, significand } = parsed.kind {
            (exponent, significand)
        } else {
            panic!("expected finite, got {:?}", parsed);
        };
        assert_eq!(sig.value(), 0);
        assert_eq!(exp.value(), 0);
    }

    #[test]
    fn finite_0_1() {
        let parsed = dec_from_hex("1800000013640001000000000000000000000000003E3000");
        let (exp, sig) = if let Decimal128Kind::Finite { exponent, significand } = parsed.kind {
            (exponent, significand)
        } else {
            panic!("expected finite, got {:?}", parsed);
        };
        assert_eq!(sig.value(), 1);
        assert_eq!(exp.value(), -1);
    }
}