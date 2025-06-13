//! Module containing functionality related to BSON ObjectIds.
//! For more information, see the documentation for the [`ObjectId`] type.

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::{convert::TryInto, time::SystemTime};
use std::{
    fmt,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use once_cell::sync::Lazy;
use rand::{random, rng, Rng};

use crate::error::{Error, Result};

const TIMESTAMP_SIZE: usize = 4;
const PROCESS_ID_SIZE: usize = 5;
const COUNTER_SIZE: usize = 3;

const TIMESTAMP_OFFSET: usize = 0;
const PROCESS_ID_OFFSET: usize = TIMESTAMP_OFFSET + TIMESTAMP_SIZE;
const COUNTER_OFFSET: usize = PROCESS_ID_OFFSET + PROCESS_ID_SIZE;

const MAX_U24: usize = 0xFF_FFFF;

static OID_COUNTER: Lazy<AtomicUsize> =
    Lazy::new(|| AtomicUsize::new(rng().random_range(0..=MAX_U24)));

/// A wrapper around a raw 12-byte ObjectId.
///
/// ## `serde` integration
/// When serialized to BSON via `serde`, this type produces a BSON ObjectId. In non-BSON formats, it
/// will serialize to and deserialize from that format's equivalent of the [extended JSON representation](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/) of a BSON ObjectId.
///
/// [`ObjectId`]s can be deserialized from hex strings in all formats.
///
/// e.g.
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use bson::oid::ObjectId;
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     oid: ObjectId,
/// }
///
/// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// let f = Foo { oid: ObjectId::new() };
/// println!("bson: {}", bson::to_document(&f)?);
/// println!("json: {}", serde_json::to_string(&f)?);
/// # Ok(())
/// # }
/// ```
/// Produces the following output:
/// ```text
/// bson: { "oid": ObjectId("63ceed18f71dda7d8cf21e8e") }
/// json: {"oid":{"$oid":"63ceed18f71dda7d8cf21e8e"}}
/// ```
///
/// ### `serde_helpers`
/// The `bson` crate provides a number of useful helpers for serializing and deserializing
/// various types to and from different formats. For example, to serialize an
/// [`ObjectId`] as a hex string, you can use
/// [`crate::serde_helpers::serialize_object_id_as_hex_string`].
/// Check out the [`crate::serde_helpers`] module documentation for a list of all of the helpers
/// offered by the crate.
///
/// e.g.
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use bson::oid::ObjectId;
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     // Serializes as a BSON ObjectId or extJSON in non-BSON formats
///     oid: ObjectId,
///
///     // Serializes as a hex string in all formats
///     #[serde(serialize_with = "bson::serde_helpers::serialize_object_id_as_hex_string")]
///     oid_as_hex: ObjectId,
/// }
/// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// let f = Foo { oid: ObjectId::new(), oid_as_hex: ObjectId::new() };
/// println!("bson: {}", bson::to_document(&f)?);
/// println!("json: {}", serde_json::to_string(&f)?);
/// # Ok(())
/// # }
/// ```
/// Produces the following output:
/// ```text
/// bson: { "oid": ObjectId("63ceeffd37518221cdc6cda2"), "oid_as_hex": "63ceeffd37518221cdc6cda3" }
/// json: {"oid":{"$oid":"63ceeffd37518221cdc6cda2"},"oid_as_hex":"63ceeffd37518221cdc6cda3"}
/// ```
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ObjectId {
    id: [u8; 12],
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for ObjectId {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl From<[u8; 12]> for ObjectId {
    fn from(bytes: [u8; 12]) -> Self {
        Self { id: bytes }
    }
}

impl ObjectId {
    /// Generates a new [`ObjectId`], represented in bytes.
    /// See the [docs](http://www.mongodb.com/docs/manual/reference/object-id/)
    /// for more information.
    pub fn new() -> Self {
        let timestamp = Self::gen_timestamp();
        let process_id = Self::gen_process_id();
        let counter = Self::gen_count();

        Self::from_parts(timestamp, process_id, counter)
    }

    /// Constructs a new ObjectId wrapper around the raw byte representation.
    pub const fn from_bytes(bytes: [u8; 12]) -> ObjectId {
        ObjectId { id: bytes }
    }

    /// Construct an `ObjectId` from its parts.
    /// See the [docs](http://www.mongodb.com/docs/manual/reference/object-id/)
    /// for more information.
    pub fn from_parts(seconds_since_epoch: u32, process_id: [u8; 5], counter: [u8; 3]) -> Self {
        let mut bytes = [0; 12];

        bytes[TIMESTAMP_OFFSET..(TIMESTAMP_OFFSET + TIMESTAMP_SIZE)]
            .clone_from_slice(&u32::to_be_bytes(seconds_since_epoch));
        bytes[PROCESS_ID_OFFSET..(PROCESS_ID_OFFSET + PROCESS_ID_SIZE)]
            .clone_from_slice(&process_id);
        bytes[COUNTER_OFFSET..(COUNTER_OFFSET + COUNTER_SIZE)].clone_from_slice(&counter);

        Self::from_bytes(bytes)
    }

    /// Creates an ObjectID using a 12-byte (24-char) hexadecimal string.
    pub fn parse_str(s: impl AsRef<str>) -> Result<ObjectId> {
        let s = s.as_ref();

        let bytes: Vec<u8> =
            hex::decode(s.as_bytes()).map_err(|e| Error::from_hex_error(e, s.len()))?;
        if bytes.len() != 12 {
            Err(Error::oid_invalid_length(bytes.len()))
        } else {
            let mut byte_array: [u8; 12] = [0; 12];
            byte_array[..].copy_from_slice(&bytes[..]);
            Ok(ObjectId::from_bytes(byte_array))
        }
    }

    /// Retrieves the timestamp from an [`ObjectId`].
    pub fn timestamp(&self) -> crate::DateTime {
        let mut buf = [0; 4];
        buf.copy_from_slice(&self.id[0..4]);
        let seconds_since_epoch = u32::from_be_bytes(buf);

        // This doesn't overflow since u32::MAX * 1000 < i64::MAX
        crate::DateTime::from_millis(seconds_since_epoch as i64 * 1000)
    }

    /// Returns the raw byte representation of an ObjectId.
    pub const fn bytes(&self) -> [u8; 12] {
        self.id
    }

    /// Convert this [`ObjectId`] to its hex string representation.
    pub fn to_hex(self) -> String {
        hex::encode(self.id)
    }

    /// Generates a new timestamp representing the current seconds since epoch.
    fn gen_timestamp() -> u32 {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let timestamp: u32 = (js_sys::Date::now() / 1000.0) as u32;
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let timestamp: u32 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock is before 1970")
            .as_secs()
            .try_into()
            .unwrap(); // will succeed until 2106 since timestamp is unsigned

        timestamp
    }

    /// Generate a random 5-byte array.
    fn gen_process_id() -> [u8; 5] {
        static BUF: Lazy<[u8; 5]> = Lazy::new(random);

        *BUF
    }

    /// Gets an incremental 3-byte count.
    /// Represented in Big Endian.
    fn gen_count() -> [u8; 3] {
        let u_counter = OID_COUNTER.fetch_add(1, Ordering::SeqCst);

        // Mod result instead of OID_COUNTER to prevent threading issues.
        let u = u_counter % (MAX_U24 + 1);

        // Convert usize to writable u64, then extract the first three bytes.
        let u_int = u as u64;

        let buf = u_int.to_be_bytes();
        let buf_u24: [u8; 3] = [buf[5], buf[6], buf[7]];
        buf_u24
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ObjectId").field(&self.to_hex()).finish()
    }
}

#[cfg(test)]
use crate::tests::LOCK;

#[test]
fn count_generated_is_big_endian() {
    let _guard = LOCK.run_exclusively();
    let start = 1_122_866;
    OID_COUNTER.store(start, Ordering::SeqCst);

    // Test count generates correct value 1122866
    let count_bytes = ObjectId::gen_count();

    let mut buf: [u8; 4] = [0; 4];
    buf[1..=COUNTER_SIZE].clone_from_slice(&count_bytes[..COUNTER_SIZE]);

    let count = u32::from_be_bytes(buf);
    assert_eq!(start as u32, count);

    // Test OID formats count correctly as big endian
    let oid = ObjectId::new();

    assert_eq!(0x11u8, oid.bytes()[COUNTER_OFFSET]);
    assert_eq!(0x22u8, oid.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0x33u8, oid.bytes()[COUNTER_OFFSET + 2]);
}

#[test]
fn test_counter_overflow_u24_max() {
    let _guard = LOCK.run_exclusively();
    let start = MAX_U24;
    OID_COUNTER.store(start, Ordering::SeqCst);
    let oid = ObjectId::new();
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET]);
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET + 2]);
    // Test counter overflows to 0 when set to MAX_24 + 1
    let oid_new = ObjectId::new();
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET]);
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET + 2]);
}

#[test]
fn test_counter_overflow_usize_max() {
    let _guard = LOCK.run_exclusively();
    let start = usize::MAX;
    OID_COUNTER.store(start, Ordering::SeqCst);
    // Test counter overflows to u24_max when set to usize_max
    let oid = ObjectId::new();
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET]);
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0xFFu8, oid.bytes()[COUNTER_OFFSET + 2]);
    // Test counter overflows to 0 when set to usize_max + 1
    let oid_new = ObjectId::new();
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET]);
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0x00u8, oid_new.bytes()[COUNTER_OFFSET + 2]);
}

#[cfg(test)]
mod test {
    use time::macros::datetime;

    #[test]
    fn test_display() {
        let id = super::ObjectId::parse_str("53e37d08776f724e42000000").unwrap();

        assert_eq!(format!("{}", id), "53e37d08776f724e42000000")
    }

    #[test]
    fn test_debug() {
        let id = super::ObjectId::parse_str("53e37d08776f724e42000000").unwrap();

        assert_eq!(
            format!("{:?}", id),
            "ObjectId(\"53e37d08776f724e42000000\")"
        );
        assert_eq!(
            format!("{:#?}", id),
            "ObjectId(\n    \"53e37d08776f724e42000000\",\n)"
        );
    }

    #[test]
    fn test_timestamp() {
        let id = super::ObjectId::parse_str("000000000000000000000000").unwrap();
        // "Jan 1st, 1970 00:00:00 UTC"
        assert_eq!(datetime!(1970-01-01 0:00 UTC), id.timestamp().to_time_0_3());

        let id = super::ObjectId::parse_str("7FFFFFFF0000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:07 UTC"
        assert_eq!(
            datetime!(2038-01-19 3:14:07 UTC),
            id.timestamp().to_time_0_3()
        );

        let id = super::ObjectId::parse_str("800000000000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:08 UTC"
        assert_eq!(
            datetime!(2038-01-19 3:14:08 UTC),
            id.timestamp().to_time_0_3()
        );

        let id = super::ObjectId::parse_str("FFFFFFFF0000000000000000").unwrap();
        // "Feb 7th, 2106 06:28:15 UTC"
        assert_eq!(
            datetime!(2106-02-07 6:28:15 UTC),
            id.timestamp().to_time_0_3()
        );
    }
}
