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
use std::sync::LazyLock;

pub(crate) static LOCK: LazyLock<TestLock> = LazyLock::new(TestLock::new);
