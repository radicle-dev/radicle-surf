//! Collection of errors and helper instances that can occur when performing operations from
//! [`crate::vcs::git`].

use crate::file_system::error as file_error;
use crate::vcs::git::object::{BranchName, TagName};
use std::str;

/// Enumeration of errors that can occur in operations from [`crate::vcs::git`].
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Tried to create a commit history that ended up being empty.
    /// This is quite an unlikely case since `revwalk`s are used,
    /// and the walk is initialised with first commit.
    EmptyCommitHistory,
    /// The user tried to fetch a branch, but the name provided does not
    /// exist as a branch. This could mean that the branch does not exist
    /// or that a tag or commit was provided by accident.
    NotBranch(BranchName),
    /// The user tried to fetch a tag, but the name provided does not
    /// exist as a tag. This could mean that the tag does not exist
    /// or that a branch or commit was provided by accident.
    NotTag(TagName),
    /// A `revspec` was provided that could not be parsed into a branch, tag, or commit object.
    RevParseFailure(String),
    /// A [`str::Utf8Error`] error, which usually occurs when a git object's name is not in
    /// UTF-8 form and parsing of it as such fails.
    Utf8Error(str::Utf8Error),
    /// An error that comes from performing a [`crate::file_system`] operation.
    FileSystem(file_error::Error),
    /// While attempting to calculate a diff for retrieving the
    /// [`crate::vcs::git::Browser.last_commit()`], the file path was returned as an `Option`.
    LastCommitException,
    /// A wrapper around the generic [`git2::Error`].
    Git(git2::Error),
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

impl From<file_error::Error> for Error {
    fn from(err: file_error::Error) -> Self {
        Error::FileSystem(err)
    }
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Error::Git(err)
    }
}

/// A private enum that captures a recoverable and
/// non-recoverable error when walking the git tree.
///
/// In the case of `NotBlob` we abort the the computation but do
/// a check for it and recover.
///
/// In the of `Git` we abort both computations.
#[derive(Debug)]
pub(crate) enum TreeWalkError {
    NotBlob,
    Commit,
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
