// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::convert::TryFrom;

use thiserror::Error;

use crate::{diff::LineDiff, file_system::Path};

/// A Git diff error.
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
            (Some(l), Some(r)) => Ok(Self::context(line.content().to_owned(), l, r)),
            (None, None) => Err(Error::InvalidLineDiff),
        }
    }
}
