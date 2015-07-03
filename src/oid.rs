use libc;

use crypto::digest::Digest;
use crypto::md5::Md5;

use byteorder::{ByteOrder, BigEndian, LittleEndian};
use rand::{Rng, OsRng};
use rustc_serialize::hex::{self, FromHex};
use time;

use std::{fmt, io, error, result};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

const TIMESTAMP_SIZE: usize = 4;
const MACHINE_ID_SIZE: usize = 3;
const PROCESS_ID_SIZE: usize = 2;
const COUNTER_SIZE: usize = 3;

const TIMESTAMP_OFFSET: usize = 0;
const MACHINE_ID_OFFSET: usize = TIMESTAMP_OFFSET + TIMESTAMP_SIZE;
const PROCESS_ID_OFFSET: usize = MACHINE_ID_OFFSET + MACHINE_ID_SIZE;
const COUNTER_OFFSET: usize = PROCESS_ID_OFFSET + PROCESS_ID_SIZE;

const MAX_U24: usize = 0xFFFFFF;

static OID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
static mut MACHINE_BYTES: [u8; 3] = [0; 3];

extern {
    fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

/// Errors that can occur during OID construction and generation.
#[derive(Debug)]
pub enum Error {
    ArgumentError(String),
    FromHexError(hex::FromHexError),
    IoError(io::Error),
    HostnameError,
}

impl From<hex::FromHexError> for Error {
    fn from(err: hex::FromHexError) -> Error {
        Error::FromHexError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

/// Alias for Result<T, oid::Error>.
pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::ArgumentError(ref inner) => inner.fmt(fmt),
            &Error::FromHexError(ref inner) => inner.fmt(fmt),
            &Error::IoError(ref inner) => inner.fmt(fmt),
            &Error::HostnameError => write!(fmt, "Failed to retrieve hostname for OID generation."),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::ArgumentError(ref inner) => &inner,
            &Error::FromHexError(ref inner) => inner.description(),
            &Error::IoError(ref inner) => inner.description(),
            &Error::HostnameError => "Failed to retrieve hostname for OID generation.",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {            
            &Error::ArgumentError(_) => None,
            &Error::FromHexError(ref inner) => Some(inner),
            &Error::IoError(ref inner) => Some(inner),
            &Error::HostnameError => None,
        }
    }
}

/// A wrapper around raw 12-byte ObjectId representations.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ObjectId {
    id: [u8; 12],
}

impl ObjectId {
    /// Generates a new ObjectID, represented in bytes.
    /// See the [docs](http://docs.mongodb.org/manual/reference/object-id/)
    /// for more information.
    pub fn new() -> Result<ObjectId> {
        let timestamp = ObjectId::gen_timestamp();
        let machine_id = try!(ObjectId::gen_machine_id());
        let process_id = ObjectId::gen_process_id();
        let counter = try!(ObjectId::gen_count());

        let mut buf: [u8; 12] = [0; 12];
        for i in 0..TIMESTAMP_SIZE { buf[TIMESTAMP_OFFSET + i] = timestamp[i]; }
        for i in 0..MACHINE_ID_SIZE { buf[MACHINE_ID_OFFSET + i] = machine_id[i]; }
        for i in 0..PROCESS_ID_SIZE { buf[PROCESS_ID_OFFSET + i] = process_id[i]; }
        for i in 0..COUNTER_SIZE { buf[COUNTER_OFFSET +i] = counter[i]; }

        Ok(ObjectId::with_bytes(buf))
    }

    /// Constructs a new ObjectId wrapper around the raw byte representation.
    pub fn with_bytes(bytes: [u8; 12]) -> ObjectId {
        ObjectId {
            id: bytes,
        }
    }

    /// Creates an ObjectID using a 12-byte (24-char) hexadecimal string.
    pub fn with_string(s: &str) -> Result<ObjectId> {
        let bytes = try!(s.from_hex());
        if bytes.len() != 12 {
            Err(Error::ArgumentError("Provided string must be a 12-byte hexadecimal string.".to_owned()))
        } else {
            let mut byte_array: [u8; 12] = [0; 12];
            for i in 0..12 {
                byte_array[i] = bytes[i];
            }
            Ok(ObjectId::with_bytes(byte_array))
        }
    }

    /// Creates a dummy ObjectId with a specific generation time.
    /// This method should only be used to do range queries on a field
    /// containing ObjectId instances.
    pub fn with_timestamp(time: u32) -> ObjectId {
        let mut buf: [u8; 12] = [0; 12];
        BigEndian::write_u32(&mut buf, time);
        ObjectId::with_bytes(buf)
    }

    /// Returns the raw byte representation of an ObjectId.
    pub fn bytes(&self) -> [u8; 12] {
        self.id
    }

    /// Retrieves the timestamp (seconds since epoch) from an ObjectId.
    pub fn timestamp(&self) -> u32 {
        BigEndian::read_u32(&self.id)
    }

    /// Retrieves the machine id associated with an ObjectId.
    pub fn machine_id(&self) -> u32 {
        let mut buf: [u8; 4] = [0; 4];
        for i in 0..MACHINE_ID_SIZE {
            buf[i] = self.id[MACHINE_ID_OFFSET+i];
        }
        LittleEndian::read_u32(&buf)
    }

    /// Retrieves the process id associated with an ObjectId.
    pub fn process_id(&self) -> u16 {
        LittleEndian::read_u16(&self.id[PROCESS_ID_OFFSET..])
    }

    /// Retrieves the increment counter from an ObjectId.
    pub fn counter(&self) -> u32 {
        let mut buf: [u8; 4] = [0; 4];
        for i in 0..COUNTER_SIZE {
            buf[i+1] = self.id[COUNTER_OFFSET+i];
        }
        BigEndian::read_u32(&buf)
    }

    // Generates a new timestamp representing the current seconds since epoch.
    // Represented in Big Endian.
    fn gen_timestamp() -> [u8; 4] {
        let timespec = time::get_time();
        let timestamp = timespec.sec as u32;

        let mut buf: [u8; 4] = [0; 4];
        BigEndian::write_u32(&mut buf,timestamp);
        buf
    }

    // Generates a new machine id represented as an MD5-hashed 3-byte-encoded hostname string.
    // Represented in Little Endian.
    fn gen_machine_id() -> Result<[u8; 3]> {
        // Short-circuit if machine id has already been calculated.
        // Since the generated machine id is not variable, arising race conditions
        // will have the same MACHINE_BYTES result.
        unsafe {
            if MACHINE_BYTES[0] != 0 || MACHINE_BYTES[1] != 0 || MACHINE_BYTES[2] != 0 {
                return Ok(MACHINE_BYTES);
            }
        }

        // Retrieve hostname through libc
        let len = 255;
        let mut buf = Vec::<u8>::with_capacity(len);
        let ptr = buf.as_mut_ptr();
        let err = unsafe { gethostname(ptr as *mut libc::c_char, len as libc::size_t) } as i32;

        if err != 0 {
            return Err(Error::HostnameError);
        }

        // Convert bytes into string
        let s = String::from_utf8_lossy(&buf);

        // Hash hostname string
        let mut md5 = Md5::new();
        md5.input_str(&s.into_owned()[..]);
        let hash = md5.result_str();

        // Re-convert string to bytes and grab first three
        let mut bytes = hash.bytes();
        let mut vec: [u8; 3] = [0; 3];
        for i in 0..MACHINE_ID_SIZE {
            match bytes.next() {
                Some(b) => vec[i] = b,
                None => break,
            }
        }

        unsafe { MACHINE_BYTES = vec };
        Ok(vec)
    }

    // Gets the process ID and returns it as a 2-byte array.
    // Represented in Little Endian.
    fn gen_process_id() -> [u8; 2] {
        let pid = unsafe { libc::getpid() as u16 };
        let mut buf: [u8; 2] = [0; 2];
        LittleEndian::write_u16(&mut buf, pid);
        buf
    }

    // Gets an incremental 3-byte count.
    // Represented in Big Endian.
    fn gen_count() -> Result<[u8; 3]> {
        // Init oid counter
        if OID_COUNTER.load(Ordering::SeqCst) == 0 {
            let mut rng = try!(OsRng::new());
            let start = rng.gen_range(0, MAX_U24 + 1);
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
        Ok(buf_u24)
    }
}

#[test]
fn pid_generation() {
    let pid = unsafe { libc::getpid() as u16 };
    let generated = ObjectId::gen_process_id();
    assert_eq!(pid, LittleEndian::read_u16(&generated));
}

#[test]
fn count_generation() {
    let start = 52222;
    OID_COUNTER.store(start, Ordering::SeqCst);
    let count_res = ObjectId::gen_count();
    assert!(count_res.is_ok());
    let count_bytes = count_res.unwrap();

    let mut buf: [u8; 4] = [0; 4];
    for i in 0..COUNTER_SIZE {
        buf[i+1] = count_bytes[i];
    }

    let count = BigEndian::read_u32(&buf);
    assert_eq!(start as u32, count);
}

#[test]
fn count_is_big_endian() {
    let start = 1122867;
    OID_COUNTER.store(start, Ordering::SeqCst);
    let oid_res = ObjectId::new();
    assert!(oid_res.is_ok());
    let oid = oid_res.unwrap();

    assert_eq!(0x11u8, oid.bytes()[COUNTER_OFFSET]);
    assert_eq!(0x22u8, oid.bytes()[COUNTER_OFFSET + 1]);
    assert_eq!(0x33u8, oid.bytes()[COUNTER_OFFSET + 2]);
}
