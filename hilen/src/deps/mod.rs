//! The foundational crates the engine used to pull as separate path
//! dependencies. They live here so the published `hilen` crate is one
//! self contained library instead of a family that must be released
//! together.

pub(crate) mod hreads;
pub(crate) mod netrun;
pub(crate) mod refs;
pub(crate) mod vents;
