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
    const UNUSED_BITS: usize = 2;

    fn from_bits(src_bits: &BitSlice<u8, Order>) -> Self {
        let mut bytes = [0u8; 2];
        bytes
            .view_bits_mut::<Order>()[Self::UNUSED_BITS..]
            .copy_from_bitslice(src_bits);
        Self(bytes)
    }

    fn bits(&self) -> &BitSlice<u8, Order> {
        &self.0.view_bits::<Order>()[Self::UNUSED_BITS..]
    }

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
    const UNUSED_BITS: usize = 14;

    fn from_bits(src_prefix: &BitSlice<u8, Order>, src_suffix: &BitSlice<u8, Order>) -> Self {
        let mut bytes = [0u8; 16];
        let bits = &mut bytes.view_bits_mut::<Order>()[Self::UNUSED_BITS..];
        let prefix_len = src_prefix.len();
        bits[0..prefix_len].copy_from_bitslice(src_prefix);
        bits[prefix_len..].copy_from_bitslice(src_suffix);
        Self(bytes)
    }

    fn bits(&self) -> &BitSlice<u8, Order> {
        &self.0.view_bits::<Order>()[Self::UNUSED_BITS..]
    }

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

const EXPONENT_WIDTH: usize = 14;

impl ParsedDecimal128 {
    fn new(source: &Decimal128) -> Self {
        // BSON byte order is the opposite of the decimal128 byte order, so flip 'em.  The rest of this method could be rewritten to not need this, but readability is helped by keeping the implementation congruent with the spec.
        let tmp: [u8; 16] = {
            let mut tmp = [0u8; 16];
            for i in 0..16 {
                tmp[i] = source.bytes[15-i];
            }
            tmp
        };
        let src_bits = tmp.view_bits::<Order>();
        pdbg!(&src_bits);

        let sign = src_bits[0];

        let kind = if src_bits[1..5].all() {
            // Special value
            if src_bits[5] {
                Decimal128Kind::NaN { signalling: src_bits[6] }
            } else {
                Decimal128Kind::Infinity
            }
        } else {
            // Finite value
            let exponent_offset;
            let significand_prefix;
            if src_bits[1..3].all() {
                exponent_offset = 3;
                significand_prefix = bits![static u8, Msb0; 1, 0, 0];
            } else {
                exponent_offset = 1;
                significand_prefix = bits![static u8, Msb0; 0];
            }

            let exponent = Exponent::from_bits(&src_bits[exponent_offset..exponent_offset+EXPONENT_WIDTH]);
            let significand = Significand::from_bits(significand_prefix, &src_bits[exponent_offset+EXPONENT_WIDTH..]);
            pdbg!(exponent.bits());
            pdbg!(significand.bits());
            Decimal128Kind::Finite {
                exponent,
                significand,
            }
        };
        ParsedDecimal128 { sign, kind }
    }

    fn pack(&self) -> Decimal128 {
        let mut tmp = [0u8; 16];
        let dest_bits = tmp.view_bits_mut::<Order>();

        dest_bits.set(0, self.sign);

        match &self.kind {
            Decimal128Kind::NaN { signalling } => {
                dest_bits[1..6].clone_from_bitslice(bits![1, 1, 1, 1, 1]);
                dest_bits.set(6, *signalling);
            }
            Decimal128Kind::Infinity => {
                dest_bits[1..6].clone_from_bitslice(bits![1, 1, 1, 1, 0]);
            }
            Decimal128Kind::Finite { exponent, significand } => {
                let sig_bits = significand.bits();
                let exponent_offset;
                if sig_bits[0] {
                    dest_bits.set(1, true);
                    dest_bits.set(2, true);
                    exponent_offset = 3;
                } else {
                    exponent_offset = 1;
                };
                dest_bits[exponent_offset..exponent_offset+EXPONENT_WIDTH]
                    .copy_from_bitslice(exponent.bits());
                dest_bits[exponent_offset+EXPONENT_WIDTH..]
                    .copy_from_bitslice(sig_bits);
            }
        }

        let mut bytes = [0u8; 16];
        for i in 0..16 {
            bytes[i] = tmp[15-i];
        }
        Decimal128 { bytes }
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
                    if exp_val == 0 {
                        write!(f, "{}", coeff_str)?;
                    } else {
                        let dec_charlen = exp_val.abs() as usize;
                        if dec_charlen >= coeff_str.len() {
                            write!(f, "0.")?;
                            write!(f, "{}", "0".repeat(dec_charlen - coeff_str.len()))?;
                            write!(f, "{}", coeff_str)?;
                        } else {
                            let (pre, post) = coeff_str.split_at(coeff_str.len() - dec_charlen);
                            write!(f, "{}", pre)?;
                            write!(f, ".")?;
                            write!(f, "{}", post)?;
                        }
                    }
                } else {
                    // Exponential notation
                    let (pre, post) = coeff_str.split_at(1);
                    write!(f, "{}", pre)?;
                    if !post.is_empty() {
                        write!(f, ".{}", post)?;
                    }
                    write!(f, "E")?;
                    if adj_exp > 0 {
                        write!(f, "+")?;
                    }
                    write!(f, "{}", adj_exp)?;
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

    fn hex_from_dec(src: &ParsedDecimal128) -> String {
        let bytes = crate::to_vec(&doc! { "d": src.pack() }).unwrap();
        hex::encode(bytes)
    }

    #[test]
    fn nan() {
        let hex = "180000001364000000000000000000000000000000007C00";
        let parsed = dec_from_hex(hex);
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::NaN { signalling: false },
        });
        assert_eq!(parsed.to_string(), "NaN");
        assert_eq!(hex_from_dec(&parsed).to_ascii_lowercase(), hex.to_ascii_lowercase());
    }

    #[test]
    fn negative_nan() {
        let parsed = dec_from_hex("18000000136400000000000000000000000000000000FC00");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: true,
            kind: Decimal128Kind::NaN { signalling: false },
        });
        assert_eq!(parsed.to_string(), "-NaN");
    }

    #[test]
    fn snan() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000007E00");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::NaN { signalling: true },
        });
        assert_eq!(parsed.to_string(), "sNaN");
    }

    #[test]
    fn inf() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000007800");
        assert_eq!(parsed, ParsedDecimal128 {
            sign: false,
            kind: Decimal128Kind::Infinity,
        });
        assert_eq!(parsed.to_string(), "Infinity");
    }

    fn finite_parts(parsed: ParsedDecimal128) -> (i16, u128) {
        if let Decimal128Kind::Finite { exponent, significand } = parsed.kind {
            (exponent.value(), significand.value())
        } else {
            panic!("expected finite, got {:?}", parsed);
        }
    }

    #[test]
    fn finite_0() {
        let parsed = dec_from_hex("180000001364000000000000000000000000000000403000");
        assert_eq!(parsed.to_string(), "0");
        assert!(!parsed.sign);
        assert_eq!(finite_parts(parsed), (0, 0));
    }

    #[test]
    fn finite_0_1() {
        let parsed = dec_from_hex("1800000013640001000000000000000000000000003E3000");
        assert_eq!(parsed.to_string(), "0.1");
        assert!(!parsed.sign);
        assert_eq!(finite_parts(parsed), (-1, 1));
    }

    #[test]
    fn finite_long_decimal() {
        let parsed = dec_from_hex("18000000136400F2AF967ED05C82DE3297FF6FDE3CFC2F00");
        assert_eq!(parsed.to_string(), "0.1234567890123456789012345678901234");
        assert!(!parsed.sign);
        assert_eq!(finite_parts(parsed), (-34, 1234567890123456789012345678901234));
    }

    #[test]
    fn finite_smallest() {
        let parsed = dec_from_hex("18000000136400D204000000000000000000000000343000");
        assert_eq!(parsed.to_string(), "0.001234");
        assert!(!parsed.sign);
        assert_eq!(finite_parts(parsed), (-6, 1234));
    }

    #[test]
    fn finite_fractional() {
        let parsed = dec_from_hex("1800000013640064000000000000000000000000002CB000");
        assert_eq!(parsed.to_string(), "-1.00E-8");
        assert!(parsed.sign);
        assert_eq!(finite_parts(parsed), (-10, 100));
    }
}