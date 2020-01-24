//! A model of a non-empty directory data structure that can be searched, queried, and rendered.
//! The concept is to represent VCS directory, but is not necessarily tied to one.
//!
//! TODO: Add examples

pub mod directory;
pub mod error;
mod path;

pub use self::directory::*;
pub use self::path::*;
