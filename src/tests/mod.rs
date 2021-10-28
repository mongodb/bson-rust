mod binary_subtype;
mod datetime;
mod modules;
mod serde;
mod spec;

use lazy_static::lazy_static;
use modules::TestLock;

lazy_static! {
    pub(crate) static ref LOCK: TestLock = TestLock::new();
}
