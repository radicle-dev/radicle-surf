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

//! Collection of errors and helper instances that can occur when performing
//! operations from [`crate::vcs::git`].

use crate::{
    file_system::error as file_error,
    vcs::git::object::{BranchName, TagName},
};
use std::str;
use thiserror::Error;

/// Enumeration of errors that can occur in operations from [`crate::vcs::git`].
#[derive(Debug, PartialEq, Error)]
pub enum Error {
    /// The user tried to fetch a branch, but the name provided does not
    /// exist as a branch. This could mean that the branch does not exist
    /// or that a tag or commit was provided by accident.
    #[error("Provided branch name does not exist: {0}")]
    NotBranch(BranchName),
    /// The user tried to fetch a tag, but the name provided does not
    /// exist as a tag. This could mean that the tag does not exist
    /// or that a branch or commit was provided by accident.
    #[error("Provided tag name does not exist: {0}")]
    NotTag(TagName),
    /// A `revspec` was provided that could not be parsed into a branch, tag, or
    /// commit object.
    #[error("Provided revspec could not be parsed into a git object: {0}")]
    RevParseFailure(String),
    /// A [`str::Utf8Error`] error, which usually occurs when a git object's
    /// name is not in UTF-8 form and parsing of it as such fails.
    #[error("Git object name is invalid UTF-8: {0}")]
    Utf8Error(#[from] str::Utf8Error),
    /// An error that comes from performing a [`crate::file_system`] operation.
    #[error("File system error: {0}")]
    FileSystem(#[from] file_error::Error),
    /// While attempting to calculate a diff for retrieving the
    /// [`crate::vcs::git::Browser.last_commit()`], the file path was returned
    /// as an `Option::None`.
    #[error("Last commit has an invalid file path")]
    LastCommitException,
    /// A wrapper around the generic [`git2::Error`].
    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// A private enum that captures a recoverable and
/// non-recoverable error when walking the git tree.
///
/// In the case of `NotBlob` we abort the the computation but do
/// a check for it and recover.
///
/// In the of `Git` we abort both computations.
#[derive(Debug, Error)]
pub(crate) enum TreeWalkError {
    #[error("Entry is not a blob")]
    NotBlob,
    #[error("Git object is a commit")]
    Commit,
    #[error("Git error: {0}")]
    Git(Error),
}

impl From<git2::Error> for TreeWalkError {
    fn from(err: git2::Error) -> Self {
        TreeWalkError::Git(err.into())
    }
}

impl From<file_error::Error> for TreeWalkError {
    fn from(err: file_error::Error) -> Self {
        err.into()
    }
}

impl From<str::Utf8Error> for TreeWalkError {
    fn from(err: str::Utf8Error) -> Self {
        err.into()
    }
}

impl From<Error> for TreeWalkError {
    fn from(err: Error) -> Self {
        err.into()
    }
}
