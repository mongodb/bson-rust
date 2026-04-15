#[cfg(mongodb_internal_bench)]
mod bench;
mod binary_subtype;
pub(crate) mod corpus;
mod datetime;
#[cfg(feature = "facet-unstable")]
mod facet;
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
