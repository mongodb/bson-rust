use std::fmt;
use crate::spec::BinarySubtype;

impl fmt::LowerHex for BinarySubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value: u8 = (*self).into();
        fmt::LowerHex::fmt(&value, f)
    }
}
