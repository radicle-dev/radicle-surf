use crate::file_system::error as file_error;
use std::str;

#[derive(Debug, PartialEq)]
pub enum Error {
    EmptyCommitHistory,
    NotBranch,
    NotTag,
    Utf8Error(str::Utf8Error),
    FileSystem(file_error::Error),
    FileDiffException,
    Internal(git2::Error),
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
        Error::Internal(err)
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
