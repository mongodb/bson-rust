mod binary_subtype;
mod datetime;
mod modules;
#[cfg(feature = "serde")]
mod serde;
#[cfg(feature = "serde")]
mod serde_helpers;
#[cfg(feature = "serde")]
mod spec;

use modules::TestLock;
use once_cell::sync::Lazy;

pub(crate) static LOCK: Lazy<TestLock> = Lazy::new(TestLock::new);
