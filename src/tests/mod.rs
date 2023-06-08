mod binary_subtype;
mod datetime;
mod modules;
mod serde;
mod spec;

use modules::TestLock;
use once_cell::sync::Lazy;

pub(crate) static LOCK: Lazy<TestLock> = Lazy::new(TestLock::new);
