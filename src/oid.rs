//! ObjectId
use std::{
    error,
    fmt,
    result,
    sync::atomic::{AtomicUsize, Ordering},
};

use byteorder::{BigEndian, ByteOrder};

use hex::{self, FromHexError};

use rand::{thread_rng, Rng};

use crate::tests::LOCK;
use chrono::Utc;

const TIMESTAMP_SIZE: usize = 4;
const PROCESS_ID_SIZE: usize = 5;
const COUNTER_SIZE: usize = 3;

const TIMESTAMP_OFFSET: usize = 0;
const PROCESS_ID_OFFSET: usize = TIMESTAMP_OFFSET + TIMESTAMP_SIZE;
const COUNTER_OFFSET: usize = PROCESS_ID_OFFSET + PROCESS_ID_SIZE;

const MAX_U24: usize = 0xFF_FFFF;

static OID_COUNTER: AtomicUsize = AtomicUsize::new(0);

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

/// A wrapper around raw 12-byte ObjectId representations.
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
        let seconds_since_epoch = BigEndian::read_u32(&self.id);
        let naive_datetime = chrono::NaiveDateTime::from_timestamp(seconds_since_epoch as i64, 0);
        let timestamp: chrono::DateTime<Utc> = chrono::DateTime::from_utc(naive_datetime, Utc);
        timestamp
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
        let timespec = time::get_time();
        let timestamp = timespec.sec as u32;

        let mut buf: [u8; 4] = [0; 4];
        BigEndian::write_u32(&mut buf, timestamp);
        buf
    }

    // Generate a random 5-byte array.
    fn gen_process_id() -> [u8; 5] {
        let rng = thread_rng().gen_range(0, MAX_U24) as u32;
        let mut buf: [u8; 5] = [0; 5];
        BigEndian::write_u32(&mut buf, rng);
        buf
    }

    // Gets an incremental 3-byte count.
    // Represented in Big Endian.
    fn gen_count() -> [u8; 3] {
        // Init oid counter
        if OID_COUNTER.load(Ordering::SeqCst) == 0 {
            let start = thread_rng().gen_range(0, MAX_U24 + 1);
            OID_COUNTER.store(start, Ordering::SeqCst);
        }

        let u_counter = OID_COUNTER.fetch_add(1, Ordering::SeqCst);

        // Mod result instead of OID_COUNTER to prevent threading issues.
        // Static mutexes are currently unstable; once they have been
        // stabilized, one should be used to access OID_COUNTER and
        // perform multiple operations atomically.
        let u = u_counter % MAX_U24;

        // Convert usize to writable u64, then extract the first three bytes.
        let u_int = u as u64;

        let mut buf: [u8; 8] = [0; 8];
        BigEndian::write_u64(&mut buf, u_int);
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

#[test]
fn count_generated_is_big_endian() {
    let start = 1_122_866;
    OID_COUNTER.store(start, Ordering::SeqCst);

    // Test count generates correct value 1122866
    let count_bytes = ObjectId::gen_count();

    let mut buf: [u8; 4] = [0; 4];
    buf[1..=COUNTER_SIZE].clone_from_slice(&count_bytes[..COUNTER_SIZE]);

    let count = BigEndian::read_u32(&buf);
    assert_eq!(start as u32, count);

    // Test OID formats count correctly as big endian
    let oid = ObjectId::new();

    assert_eq!(0x11u8, oid.bytes()[COUNTER_OFFSET]);
    assert_eq!(0x22u8, oid.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0x33u8, oid.bytes()[COUNTER_OFFSET + 2]);
}

#[test]
fn test_counter_overflow() {
    // let start = MAX_U24;
    // OID_COUNTER.store(start, Ordering::SeqCst);

    // let oid = ObjectId::new();

    // let counter = BigEndian::read_u24(oid.bytes()[COUNTER_OFFSET]);
    // assert_eq!(counter, 0);
}

#[cfg(test)]
mod test {
    // use byteorder::{ByteOrder, LittleEndian};
    use chrono::{offset::TimeZone, Utc};

    #[test]
    fn test_display() {
        let id = super::ObjectId::with_string("53e37d08776f724e42000000").unwrap();

        assert_eq!(format!("{}", id), "53e37d08776f724e42000000")
    }

    #[test]
    fn test_debug() {
        let id = super::ObjectId::with_string("53e37d08776f724e42000000").unwrap();

        assert_eq!(format!("{:?}", id), "ObjectId(53e37d08776f724e42000000)")
    }

    #[test]
    fn test_timestamp() {
        let id = super::ObjectId::with_string("000000000000000000000000").unwrap();
        // "Jan 1st, 1970 00:00:00 UTC"
        assert_eq!(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0), id.timestamp());

        let id = super::ObjectId::with_string("7FFFFFFF0000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:07 UTC"
        assert_eq!(Utc.ymd(2038, 1, 19).and_hms(3, 14, 7), id.timestamp());

        let id = super::ObjectId::with_string("800000000000000000000000").unwrap();
        // "Jan 19th, 2038 03:14:08 UTC"
        assert_eq!(Utc.ymd(2038, 1, 19).and_hms(3, 14, 8), id.timestamp());

        let id = super::ObjectId::with_string("FFFFFFFF0000000000000000").unwrap();
        // "Feb 7th, 2106 06:28:15 UTC"
        assert_eq!(Utc.ymd(2106, 2, 7).and_hms(6, 28, 15), id.timestamp());
    }
}
