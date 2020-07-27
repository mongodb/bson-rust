//! ObjectId

use std::{
    convert::TryInto,
    error,
    fmt,
    result,
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use chrono::Utc;
use hex::{self, FromHexError};
use rand::{thread_rng, Rng};

use lazy_static::lazy_static;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::io::Cursor;

const TIMESTAMP_SIZE: usize = 4;
const PROCESS_ID_SIZE: usize = 5;
const COUNTER_SIZE: usize = 3;

const TIMESTAMP_OFFSET: usize = 0;
const PROCESS_ID_OFFSET: usize = TIMESTAMP_OFFSET + TIMESTAMP_SIZE;
const COUNTER_OFFSET: usize = PROCESS_ID_OFFSET + PROCESS_ID_SIZE;

const MAX_U24: usize = 0xFF_FFFF;

lazy_static! {
    static ref OID_COUNTER: AtomicUsize = AtomicUsize::new(thread_rng().gen_range(0, MAX_U24 + 1));
}

/// Errors that can occur during OID construction and generation.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An invalid argument was passed in.
    ArgumentError { message: String },

    /// An error occured parsing a hex string.
    FromHexError(FromHexError),
}

impl From<FromHexError> for Error {
    fn from(err: FromHexError) -> Error {
        Error::FromHexError(err)
    }
}

/// Alias for Result<T, oid::Error>.
pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ArgumentError { ref message } => message.fmt(fmt),
            Error::FromHexError(ref inner) => inner.fmt(fmt),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::ArgumentError { .. } => None,
            Error::FromHexError(ref inner) => Some(inner),
        }
    }
}

/// This is a Rust representation of ObjectId
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ObjectIdRepr {
    pub timestamp: chrono::DateTime<Utc>,
    pub salt: u64,
    pub count: u32,
}

impl From<&ObjectId> for ObjectIdRepr {
    fn from(object_id: &ObjectId) -> Self {
        let mut id = Cursor::new(&object_id.id[..]);

        let seconds_since_epoch = id.read_u32::<BigEndian>().unwrap();
        let naive_datetime = chrono::NaiveDateTime::from_timestamp(seconds_since_epoch as i64, 0);
        let timestamp: chrono::DateTime<Utc> = chrono::DateTime::from_utc(naive_datetime, Utc);

        let salt = id.read_uint::<BigEndian>(5).unwrap();
        let count = id.read_u24::<BigEndian>().unwrap();

        Self {
            timestamp,
            salt,
            count,
        }
    }
}

impl From<ObjectId> for ObjectIdRepr {
    fn from(object_id: ObjectId) -> Self {
        (&object_id).into()
    }
}

impl From<&ObjectIdRepr> for ObjectId {
    fn from(repr: &ObjectIdRepr) -> Self {
        let mut id = [0; 12];
        let mut buf = Cursor::new(&mut id[..]);

        let seconds_since_epoch = repr.timestamp.timestamp().try_into().unwrap(); // will succeed until 2106 since timestamp is unsigned
        buf.write_u32::<BigEndian>(seconds_since_epoch).unwrap();
        buf.write_uint::<BigEndian>(repr.salt, 5).unwrap();
        buf.write_u24::<BigEndian>(repr.count).unwrap();

        Self { id }
    }
}

impl From<ObjectIdRepr> for ObjectId {
    fn from(repr: ObjectIdRepr) -> Self {
        (&repr).into()
    }
}

/// A wrapper around raw 12-byte ObjectId representations.
///
/// a 4-byte timestamp value, representing the ObjectId’s creation, measured in seconds since the
/// Unix epoch a 5-byte random value
/// a 3-byte incrementing counter, initialized to a random value
///
/// While the BSON format itself is little-endian, the timestamp and counter values are big-endian,
/// with the most significant bytes appearing first in the byte sequence.

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ObjectId {
    id: [u8; 12],
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectId {
    /// Generates a new ObjectID, represented in bytes.
    /// See the [docs](http://docs.mongodb.org/manual/reference/object-id/)
    /// for more information.
    pub fn new() -> ObjectId {
        let timestamp = ObjectId::gen_timestamp();
        let process_id = ObjectId::gen_process_id();
        let counter = ObjectId::gen_count();

        let mut buf: [u8; 12] = [0; 12];
        buf[TIMESTAMP_OFFSET..(TIMESTAMP_SIZE + TIMESTAMP_OFFSET)]
            .clone_from_slice(&timestamp[..TIMESTAMP_SIZE]);
        buf[PROCESS_ID_OFFSET..(PROCESS_ID_SIZE + PROCESS_ID_OFFSET)]
            .clone_from_slice(&process_id[..PROCESS_ID_SIZE]);
        buf[COUNTER_OFFSET..(COUNTER_SIZE + COUNTER_OFFSET)]
            .clone_from_slice(&counter[..COUNTER_SIZE]);

        ObjectId::with_bytes(buf)
    }

    /// Constructs a new ObjectId wrapper around the raw byte representation.
    pub fn with_bytes(bytes: [u8; 12]) -> ObjectId {
        ObjectId { id: bytes }
    }

    /// Creates an ObjectID using a 12-byte (24-char) hexadecimal string.
    pub fn with_string(s: &str) -> Result<ObjectId> {
        let bytes: Vec<u8> = hex::decode(s.as_bytes())?;
        if bytes.len() != 12 {
            Err(Error::ArgumentError {
                message: "Provided string must be a 12-byte hexadecimal string.".to_owned(),
            })
        } else {
            let mut byte_array: [u8; 12] = [0; 12];
            byte_array[..].copy_from_slice(&bytes[..]);
            Ok(ObjectId::with_bytes(byte_array))
        }
    }

    /// Retrieves the timestamp (chrono::DateTime) from an ObjectId.
    pub fn timestamp(&self) -> chrono::DateTime<Utc> {
        let mut buf = [0; 4];
        buf.copy_from_slice(&self.id[0..4]);
        let seconds_since_epoch = u32::from_be_bytes(buf);

        let naive_datetime = chrono::NaiveDateTime::from_timestamp(seconds_since_epoch as i64, 0);
        let timestamp: chrono::DateTime<Utc> = chrono::DateTime::from_utc(naive_datetime, Utc);
        timestamp
    }

    /// Returns an usable representation of ObjectId, use it if you need all information inside a
    /// ObjectId
    pub fn repr(&self) -> ObjectIdRepr {
        self.into()
    }

    /// Returns the raw byte representation of an ObjectId.
    pub fn bytes(&self) -> [u8; 12] {
        self.id
    }

    /// Convert the objectId to hex representation.
    pub fn to_hex(&self) -> String {
        hex::encode(self.id)
    }

    // Generates a new timestamp representing the current seconds since epoch.
    // Represented in Big Endian.
    fn gen_timestamp() -> [u8; 4] {
        let timestamp: u32 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock is before 1970")
            .as_secs()
            .try_into()
            .unwrap(); // will succeed until 2106 since timestamp is unsigned
        timestamp.to_be_bytes()
    }

    // Generate a random 5-byte array.
    fn gen_process_id() -> [u8; 5] {
        let rng = thread_rng().gen_range(0, MAX_U24) as u32;
        let mut buf: [u8; 5] = [0; 5];
        buf[0..4].copy_from_slice(&rng.to_be_bytes());
        buf
    }

    // Gets an incremental 3-byte count.
    // Represented in Big Endian.
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
        f.write_str(&format!("ObjectId({})", self.to_hex()))
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
    let start = usize::max_value();
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
    use super::{ObjectId, ObjectIdRepr};
    use chrono::{offset::TimeZone, Utc};

    #[test]
    fn test_display() {
        let id = ObjectId::with_string("53e37d08776f724e42000000").unwrap();

        assert_eq!(format!("{}", id), "53e37d08776f724e42000000")
    }

    #[test]
    fn test_debug() {
        let id = ObjectId::with_string("53e37d08776f724e42000000").unwrap();

        assert_eq!(format!("{:?}", id), "ObjectId(53e37d08776f724e42000000)")
    }

    #[test]
    fn test_timestamp() {
        let id = ObjectId::with_string("000000000000000000000000").unwrap();
        // "Jan 1st, 1970 00:00:00 UTC"
        assert_eq!(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0), id.timestamp());

        let id = ObjectId::with_string("7FFFFFFF0000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:07 UTC"
        assert_eq!(Utc.ymd(2038, 1, 19).and_hms(3, 14, 7), id.timestamp());

        let id = ObjectId::with_string("800000000000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:08 UTC"
        assert_eq!(Utc.ymd(2038, 1, 19).and_hms(3, 14, 8), id.timestamp());

        let id = ObjectId::with_string("FFFFFFFF0000000000000000").unwrap();
        // "Feb 7th, 2106 06:28:15 UTC"
        assert_eq!(Utc.ymd(2106, 2, 7).and_hms(6, 28, 15), id.timestamp());
    }

    #[test]
    fn test_object_id_repr() {
        let id = ObjectId::with_string("FFFFFFFF0011223344999999").unwrap();
        let repr = (&id).into();

        let result = ObjectIdRepr {
            // "Feb 7th, 2106 06:28:15 UTC"
            timestamp: Utc.ymd(2106, 2, 7).and_hms(6, 28, 15),
            salt: 0x0011223344,
            count: 0x999999,
        };
        assert_eq!(result, repr);

        let result: ObjectId = result.into();
        assert_eq!(result, id);
    }

    #[test]
    #[should_panic]
    fn test_invalid_timestamp() {
        let result = ObjectIdRepr {
            // "Feb 7th, 2106 06:28:16 UTC"
            timestamp: Utc.ymd(2106, 2, 7).and_hms(6, 28, 16),
            salt: 0x0011223344,
            count: 0x999999,
        };

        let _: ObjectId = result.into();
    }
}
