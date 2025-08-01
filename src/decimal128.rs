//! [BSON Decimal128](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst) data type representation

use std::{convert::TryInto, fmt, num::ParseIntError};

use bitvec::prelude::*;

use crate::error::{Decimal128ErrorKind, Error, Result};

/// Struct representing a BSON Decimal128 type.
///
/// This type supports conversion to and from human-readable strings via the [std::fmt::Display] and
/// [std::str::FromStr] traits:
///
/// ```rust
/// # use std::str::FromStr;
/// # use bson::Decimal128;
/// # fn example() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// let value: Decimal128 = "3.14159".parse()?;
/// assert_eq!("3.14159", format!("{}", value));
/// let scientific = Decimal128::from_str("1.05E+3")?;
/// assert_eq!("1.05E+3", scientific.to_string());
/// # Ok(())
/// # }
/// # example().unwrap()
/// ```
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
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

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize_from_slice<E: serde::de::Error>(
        bytes: &[u8],
    ) -> std::result::Result<Self, E> {
        let arr: [u8; 128 / 8] = bytes.try_into().map_err(E::custom)?;
        Ok(Decimal128 { bytes: arr })
    }
}

impl fmt::Debug for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal128({})", hex::encode(self.bytes))
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ParsedDecimal128::new(self))
    }
}

impl std::str::FromStr for Decimal128 {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(s.parse::<ParsedDecimal128>()?.pack())
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedDecimal128 {
    sign: bool,
    kind: Decimal128Kind,
}

#[derive(Debug, Clone, PartialEq)]
enum Decimal128Kind {
    NaN {
        signalling: bool,
    },
    Infinity,
    Finite {
        exponent: Exponent,
        coefficient: Coefficient,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Exponent([u8; 2]);

impl Exponent {
    /// The exponent is stored as an unsigned value; `BIAS` is subtracted to get the actual value.
    const BIAS: i16 = 6176;
    /// The minimum representable exponent.  This is distinct from the specifications "min" value,
    /// which marks the point at which exponents are considered subnormal.
    const TINY: i16 = -6176;
    /// The maximum representable exponent.
    const MAX: i16 = 6111;

    /// The number of unused bits in the parsed representation.
    const UNUSED_BITS: usize = 2;
    /// The total number of bits in the packed representation.
    const PACKED_WIDTH: usize = 14;

    fn from_bits(src_bits: &BitSlice<u8, Msb0>) -> Self {
        let mut bytes = [0u8; 2];
        bytes.view_bits_mut::<Msb0>()[Self::UNUSED_BITS..].copy_from_bitslice(src_bits);
        Self(bytes)
    }

    fn from_native(value: i16) -> Self {
        let mut bytes = [0u8; 2];
        bytes.view_bits_mut::<Msb0>().store_be(value + Self::BIAS);
        Self(bytes)
    }

    fn bits(&self) -> &BitSlice<u8, Msb0> {
        &self.0.view_bits::<Msb0>()[Self::UNUSED_BITS..]
    }

    fn raw(&self) -> u16 {
        self.0.view_bits::<Msb0>().load_be::<u16>()
    }

    fn value(&self) -> i16 {
        (self.raw() as i16) - Self::BIAS
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Coefficient([u8; 16]);

impl Coefficient {
    /// The number of unused bits in the parsed representation.
    const UNUSED_BITS: usize = 14;
    /// The maximum number of digits allowed in a base-10 string representation of the coefficient.
    const MAX_DIGITS: usize = 34;
    /// The maximum allowable value of a coefficient.
    const MAX_VALUE: u128 = 9_999_999_999_999_999_999_999_999_999_999_999;

    fn from_bits(src_prefix: &BitSlice<u8, Msb0>, src_suffix: &BitSlice<u8, Msb0>) -> Result<Self> {
        let mut bytes = [0u8; 16];
        let bits = &mut bytes.view_bits_mut::<Msb0>()[Self::UNUSED_BITS..];
        let prefix_len = src_prefix.len();
        bits[0..prefix_len].copy_from_bitslice(src_prefix);
        bits[prefix_len..].copy_from_bitslice(src_suffix);
        let out = Self(bytes);
        if out.value() > Self::MAX_VALUE {
            Err(Error::decimal128(Decimal128ErrorKind::Overflow {}))
        } else {
            Ok(out)
        }
    }

    fn from_native(value: u128) -> Self {
        let mut bytes = [0u8; 16];
        bytes.view_bits_mut::<Msb0>().store_be(value);
        Self(bytes)
    }

    fn bits(&self) -> &BitSlice<u8, Msb0> {
        &self.0.view_bits::<Msb0>()[Self::UNUSED_BITS..]
    }

    fn value(&self) -> u128 {
        self.0.view_bits::<Msb0>().load_be::<u128>()
    }
}

impl ParsedDecimal128 {
    fn new(source: &Decimal128) -> Self {
        // BSON byte order is the opposite of the decimal128 spec byte order, so flip 'em.  The rest
        // of this method could be rewritten to not need this, but readability is helped by
        // keeping the implementation congruent with the spec.
        let tmp: [u8; 16] = {
            let mut tmp = [0u8; 16];
            tmp.view_bits_mut::<Msb0>()
                .store_be(source.bytes.view_bits::<Msb0>().load_le::<u128>());
            tmp
        };
        let src_bits = tmp.view_bits::<Msb0>();

        let sign = src_bits[0];
        let kind = if src_bits[1..5].all() {
            // Special value
            if src_bits[5] {
                Decimal128Kind::NaN {
                    signalling: src_bits[6],
                }
            } else {
                Decimal128Kind::Infinity
            }
        } else {
            // Finite value
            let exponent_offset;
            let coeff_prefix;
            if src_bits[1..3].all() {
                exponent_offset = 3;
                coeff_prefix = bits![static u8, Msb0; 1, 0, 0];
            } else {
                exponent_offset = 1;
                coeff_prefix = bits![static u8, Msb0; 0];
            }
            let coeff_offset = exponent_offset + Exponent::PACKED_WIDTH;

            let exponent = Exponent::from_bits(&src_bits[exponent_offset..coeff_offset]);
            let coefficient = match Coefficient::from_bits(coeff_prefix, &src_bits[coeff_offset..])
            {
                Ok(c) => c,
                // Invalid coefficients get silently replaced with zero.
                Err(_) => Coefficient([0u8; 16]),
            };
            Decimal128Kind::Finite {
                exponent,
                coefficient,
            }
        };
        ParsedDecimal128 { sign, kind }
    }

    fn pack(&self) -> Decimal128 {
        let mut tmp = [0u8; 16];
        let dest_bits = tmp.view_bits_mut::<Msb0>();

        dest_bits.set(0, self.sign);

        match &self.kind {
            Decimal128Kind::NaN { signalling } => {
                dest_bits[1..6].copy_from_bitslice(bits![u8, Msb0; 1, 1, 1, 1, 1]);
                dest_bits.set(6, *signalling);
            }
            Decimal128Kind::Infinity => {
                dest_bits[1..6].copy_from_bitslice(bits![u8, Msb0; 1, 1, 1, 1, 0]);
            }
            Decimal128Kind::Finite {
                exponent,
                coefficient,
            } => {
                let mut coeff_bits = coefficient.bits();
                let exponent_offset;
                if coeff_bits[0] {
                    dest_bits.set(1, true);
                    dest_bits.set(2, true);
                    coeff_bits = &coeff_bits[3..];
                    exponent_offset = 3;
                } else {
                    coeff_bits = &coeff_bits[1..];
                    exponent_offset = 1;
                };
                let coeff_offset = exponent_offset + Exponent::PACKED_WIDTH;
                dest_bits[exponent_offset..coeff_offset].copy_from_bitslice(exponent.bits());
                dest_bits[coeff_offset..].copy_from_bitslice(coeff_bits);
            }
        }

        let mut bytes = [0u8; 16];
        bytes
            .view_bits_mut::<Msb0>()
            .store_le(tmp.view_bits::<Msb0>().load_be::<u128>());
        Decimal128 { bytes }
    }
}

impl fmt::Display for ParsedDecimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // MongoDB diverges from the IEEE spec and requires no sign for NaN
        if self.sign && !matches!(&self.kind, Decimal128Kind::NaN { .. }) {
            write!(f, "-")?;
        }
        match &self.kind {
            Decimal128Kind::NaN {
                signalling: _signalling,
            } => {
                /* Likewise, MongoDB requires no 's' prefix for signalling.
                if *signalling {
                    write!(f, "s")?;
                }
                */
                write!(f, "NaN")?;
            }
            Decimal128Kind::Infinity => write!(f, "Infinity")?,
            Decimal128Kind::Finite {
                exponent,
                coefficient,
            } => {
                let coeff_str = format!("{}", coefficient.value());
                let exp_val = exponent.value();
                let adj_exp = exp_val + (coeff_str.len() as i16) - 1;
                if exp_val <= 0 && adj_exp >= -6 {
                    // Plain notation
                    if exp_val == 0 {
                        write!(f, "{}", coeff_str)?;
                    } else {
                        let dec_charlen = exp_val.unsigned_abs() as usize;
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

impl std::str::FromStr for ParsedDecimal128 {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self> {
        let sign;
        if let Some(rest) = s.strip_prefix(&['-', '+'][..]) {
            sign = s.starts_with('-');
            s = rest;
        } else {
            sign = false;
        }
        let kind = match s.to_ascii_lowercase().as_str() {
            "nan" => Decimal128Kind::NaN { signalling: false },
            "snan" => Decimal128Kind::NaN { signalling: true },
            "infinity" | "inf" => Decimal128Kind::Infinity,
            finite_str => {
                // Split into parts
                let mut decimal_str;
                let exp_str;
                match finite_str.split_once('e') {
                    None => {
                        decimal_str = finite_str;
                        exp_str = "0";
                    }
                    Some((_, "")) => {
                        return Err(Error::decimal128(Decimal128ErrorKind::EmptyExponent {}))
                    }
                    Some((pre, post)) => {
                        decimal_str = pre;
                        exp_str = post;
                    }
                }
                // Initially parse the exponent as an i128 to handle corner cases with zero
                // coefficients and very large exponents.
                let mut wide_exp = exp_str.parse::<i128>().map_err(|e| {
                    Error::decimal128(Decimal128ErrorKind::InvalidExponent {}).with_message(e)
                })?;

                // Remove decimal point and adjust exponent
                let joined_str;
                if let Some((pre, post)) = decimal_str.split_once('.') {
                    let exp_adj = post
                        .len()
                        .try_into()
                        .map_err(|_| Error::decimal128(Decimal128ErrorKind::Underflow {}))?;
                    wide_exp = wide_exp
                        .checked_sub(exp_adj)
                        .ok_or_else(|| Error::decimal128(Decimal128ErrorKind::Underflow {}))?;
                    joined_str = format!("{}{}", pre, post);
                    decimal_str = &joined_str;
                }

                // Strip leading zeros
                let rest = decimal_str.trim_start_matches('0');
                decimal_str = if rest.is_empty() { "0" } else { rest };

                // Check decimal precision
                {
                    let len = decimal_str.len();
                    if len > Coefficient::MAX_DIGITS {
                        decimal_str = round_decimal_str(decimal_str, Coefficient::MAX_DIGITS)?;
                        let exp_adj = (len - decimal_str.len())
                            .try_into()
                            .map_err(|_| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                        wide_exp = wide_exp
                            .checked_add(exp_adj)
                            .ok_or_else(|| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                    }
                }

                // Check exponent limits
                const TINY: i128 = Exponent::TINY as i128;
                if wide_exp < TINY {
                    if decimal_str != "0" {
                        let delta = (TINY - wide_exp)
                            .try_into()
                            .map_err(|_| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                        let new_precision = decimal_str
                            .len()
                            .checked_sub(delta)
                            .ok_or_else(|| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                        decimal_str = round_decimal_str(decimal_str, new_precision)?;
                    }
                    wide_exp = Exponent::TINY.into();
                }
                let padded_str;
                const MAX: i128 = Exponent::MAX as i128;
                if wide_exp > MAX {
                    if decimal_str != "0" {
                        let delta = (wide_exp - MAX)
                            .try_into()
                            .map_err(|_| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                        if decimal_str
                            .len()
                            .checked_add(delta)
                            .ok_or_else(|| Error::decimal128(Decimal128ErrorKind::Overflow {}))?
                            > Coefficient::MAX_DIGITS
                        {
                            return Err(Error::decimal128(Decimal128ErrorKind::Overflow {}));
                        }
                        padded_str = format!("{}{}", decimal_str, "0".repeat(delta));
                        decimal_str = &padded_str;
                    }
                    wide_exp = Exponent::MAX.into();
                }

                // Assemble the final value
                let exp: i16 = wide_exp
                    .try_into()
                    .map_err(|_| Error::decimal128(Decimal128ErrorKind::Overflow {}))?;
                let exponent = Exponent::from_native(exp);
                let coeff: u128 = decimal_str.parse().map_err(|e: ParseIntError| {
                    Error::decimal128(Decimal128ErrorKind::InvalidCoefficient {}).with_message(e)
                })?;
                let coefficient = Coefficient::from_native(coeff);
                Decimal128Kind::Finite {
                    exponent,
                    coefficient,
                }
            }
        };

        Ok(Self { sign, kind })
    }
}

fn round_decimal_str(s: &str, precision: usize) -> Result<&str> {
    let (pre, post) = s
        .split_at_checked(precision)
        .ok_or_else(|| Error::decimal128(Decimal128ErrorKind::Unparseable {}))?;
    // Any nonzero trimmed digits mean it would be an imprecise round.
    if post.chars().any(|c| c != '0') {
        return Err(Error::decimal128(Decimal128ErrorKind::InexactRounding {}));
    }
    Ok(pre)
}
