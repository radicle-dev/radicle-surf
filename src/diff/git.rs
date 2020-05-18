use std::convert::TryFrom;

use thiserror::Error;

use crate::{diff::LineDiff, file_system::Path};

#[derive(Debug, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A The path of a file isn't available.
    #[error("couldn't retrieve file path")]
    PathUnavailable,
    /// A patch is unavailable.
    #[error("couldn't retrieve patch for {0}")]
    PatchUnavailable(Path),
    /// A Git delta type isn't currently handled.
    #[error("git delta type is not handled")]
    DeltaUnhandled(git2::Delta),
    /// A Git `DiffLine` is invalid.
    #[error("invalid `git2::DiffLine`")]
    InvalidLineDiff,
}

impl<'a> TryFrom<git2::DiffLine<'a>> for LineDiff {
    type Error = Error;

    fn try_from(line: git2::DiffLine) -> Result<Self, Self::Error> {
        match (line.old_lineno(), line.new_lineno()) {
            (None, Some(n)) => Ok(Self::addition(line.content().to_owned(), n)),
            (Some(n), None) => Ok(Self::deletion(line.content().to_owned(), n)),
            (Some(_), Some(n)) => Ok(Self::context(line.content().to_owned(), n)),
            (None, None) => Err(Error::InvalidLineDiff),
        }
    }
}
