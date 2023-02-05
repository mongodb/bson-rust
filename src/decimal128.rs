//! [BSON Decimal128](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst) data type representation

use std::{convert::TryInto, fmt};

/// Struct representing a BSON Decimal128 type.
///
/// Currently, this type can only be used to round-trip through BSON. See
/// [RUST-36](https://jira.mongodb.org/browse/RUST-36) to track the progress towards a complete implementation.
#[derive(Copy, Clone, PartialEq)]
pub struct Decimal128 {
    /// BSON bytes containing the decimal128. Stored for round tripping.
    pub(crate) bytes: [u8; 128 / 8],
}

struct ArrayIntStruct {
    low: u64,
    high: u64,
}
struct DecimalStruct {
    sign: u8,
    digits: u128,
    exponent: i64,
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

    /// Returns the string representation of this `Decimal128`.
    pub fn to_string(&self) -> String {
        let _exponent_mask: u64 = 3 << 61;
        let _exponent_bias: u64 = 6176;
        let _exponent_max: u64 = 6144;
        let _exponent_min: i64 = -6143;
        let _max_digits: u64 = 34;

        let _inf: u64 = 0x7800000000000000;
        let _nan: u64 = 0x7C00000000000000;
        let _snan: u64 = 0x7E00000000000000;
        let _sign: u64 = 0x8000000000000000;

        let value = Self::_unpack_arr(self.bytes);

        let high = value.high;
        let low = value.low;

        let sign = if (high & _sign) > 0 { 1 } else { 0 };

        if (high & _snan) == _snan {
            String::from("Nan")
        } else if (high & _nan) == _nan {
            String::from("nan")
        } else if (high & _inf) == _inf {
            String::from("Infinity")
        } else {
            let res_info = if (high & _exponent_mask) == _exponent_mask {
                let exponent = ((high & 0x1FFFE00000000000) >> 47) as i64 - _exponent_bias as i64;
                DecimalStruct {
                    sign: sign,
                    digits: 0,
                    exponent: exponent,
                }
            } else {
                let exponent = ((high & 0x7FFF800000000000) >> 49) as i64 - _exponent_bias as i64;

                let mut arr: [u64; 15] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                let mut mask = 0x00000000000000FF;
                for i in (7..15).rev() {
                    arr[i] = (low & mask) >> ((14 - i) << 3);
                    mask = mask << 8;
                }

                let mut mask = 0x00000000000000FF;
                for i in (1..7).rev() {
                    arr[i] = (high & mask) >> ((6 - i) << 3);
                    mask = mask << 8;
                }

                let mask = 0x0001000000000000;
                arr[0] = (high & mask) >> 48;

                let mut big_int: u128 = 0;
                let mut flg: usize = 1;
                for i in arr {
                    big_int = big_int + u128::from(i);
                    if flg < 15 {
                        big_int = big_int << 8;
                        flg = flg + 1;
                    }
                }

                DecimalStruct {
                    sign: sign,
                    digits: big_int,
                    exponent: exponent,
                }
            };

            Self::decimal_struct_to_string(res_info)
        }
    }

    pub(crate) fn deserialize_from_slice<E: serde::de::Error>(
        bytes: &[u8],
    ) -> std::result::Result<Self, E> {
        let arr: [u8; 128 / 8] = bytes.try_into().map_err(E::custom)?;
        Ok(Decimal128 { bytes: arr })
    }

    fn _unpack_arr(arr: [u8; 16]) -> ArrayIntStruct {
        let mut low: u64 = 0;
        let mut high: u64 = 0;

        for i in (0..8).rev() {
            low = low + u64::from(arr[i]);
            if i > 0 {
                low = low << 8;
            }
        }

        for i in (8..16).rev() {
            high = high + u64::from(arr[i]);
            if i > 8 {
                high = high << 8;
            }
        }

        ArrayIntStruct { low, high }
    }

    fn decimal_struct_to_string(dec_struct: DecimalStruct) -> String {
        let sign = if dec_struct.sign == 0 {
            String::from("")
        } else {
            String::from("-")
        };

        if dec_struct.exponent == 0 {
            let digits = dec_struct.digits.to_string();
            let res = sign + &digits;
            return res;
        } else {
            let int_str = dec_struct.digits.to_string();
            let int_len = int_str.len();
            let int_split = int_str.as_bytes().to_vec();
            let mut index: usize = 0;

            // if there are too many significant digits, we should just be treating numbers
            // as + or - 0 and using the non-scientific exponent (this is for the "invalid
            // representation should be treated as 0/-0" spec cases in decimal128-1.json)
            let exponent = dec_struct.exponent;
            let scientific_exponent = exponent - 1 + (int_len as i64);
            if scientific_exponent >= 34 || scientific_exponent <= -15 || exponent > 0 {
                if int_len > 34 {
                    let return_string = if exponent > 0 {
                        sign + &String::from("0E+") + &(exponent.to_string())
                    } else {
                        sign + &String::from("0E") + &(exponent.to_string())
                    };
                    return return_string;
                } else {
                    let mut return_string = sign + &String::from(int_split[index] as char);
                    index += 1;
                    let significand_digits = int_len - 1;

                    if significand_digits != 0 {
                        return_string = return_string + &String::from(".");
                    }

                    let mut float_string = String::from("");
                    for _s in 0..significand_digits {
                        float_string = float_string + &String::from(int_split[index] as char);
                        index += 1;
                    }

                    return_string = return_string + &float_string;

                    return_string = return_string + &String::from("E");

                    if scientific_exponent > 0 {
                        return return_string
                            + &String::from("+")
                            + &(scientific_exponent.to_string());
                    } else {
                        return return_string + &(scientific_exponent.to_string());
                    }
                }
            } else {
                let mut return_string = sign + &String::from("");
                if exponent >= 0 {
                    for _i in 0..int_len {
                        return_string += &String::from(int_split[index] as char);
                        index += 1;
                    }

                    return return_string;
                } else {
                    let mut radix_position = int_len as i64 + exponent;

                    if radix_position > 0 {
                        for _i in 0..radix_position {
                            return_string += &String::from(int_split[index] as char);
                            index += 1;
                        }
                    } else {
                        return_string += &String::from("0");
                    }

                    return_string += &String::from(".");

                    while radix_position < 0 {
                        return_string += &String::from("0");
                        radix_position += 1;
                    }

                    let d = int_len - std::cmp::max(radix_position, 0) as usize;

                    for _i in 0..d {
                        return_string += &String::from(int_split[index] as char);
                        index += 1;
                    }

                    return return_string;
                }
            }
        }
    }

}

impl fmt::Debug for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal128({})", self.to_string())
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
