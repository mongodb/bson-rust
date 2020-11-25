use std::{convert::TryFrom, result::Result};

use serde::{ser::Error, Serializer};

pub fn serialize_u32_as_i32<S: Serializer>(
    val: u32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i32::try_from(val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(Error::custom(format!("u32 {} does not fit into an i32", val))),
    }
}

pub fn serialize_u32_as_i64<S: Serializer>(
    val: u32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(val as i64)
}

pub fn serialize_u64_as_i32<S: Serializer>(
    val: u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i32::try_from(val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(Error::custom(format!("u64 {} does not fit into an i32", val))),
    }
}

pub fn serialize_u64_as_i64<S: Serializer>(
    val: u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i64::try_from(val) {
        Ok(val) => serializer.serialize_i64(val),
        Err(_) => Err(Error::custom(format!("u64 {} does not fit into an i64", val))),
    }
}

pub fn deserialize_date_time_from_ext_json<S: Serializer>(
    val: Document,
    serializer: S,
) -> Result<S::
