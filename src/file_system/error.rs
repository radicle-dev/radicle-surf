//! Errors that can occur within the file system logic.
//!
//! These errors occur due to [`Label`](super::path::Label) and [`Path`](super::path::Path) parsing
//! when using their respective `TryFrom` instances.

use std::ffi::OsStr;
use thiserror::Error;

pub(crate) const EMPTY_PATH: Error = Error::Path(PathError::Empty);
pub(crate) const EMPTY_LABEL: Error = Error::Label(LabelError::Empty);

/// Build an [`Error::Label(LabelError::InvalidUTF8)`] from an [`OsStr`](std::ffi::OsStr)
pub(crate) fn label_invalid_utf8(item: &OsStr) -> Error {
    Error::Label(LabelError::InvalidUTF8(item.to_string_lossy().into()))
}

/// Build an [`Error::Label(LabelError::ContainsSlash)`] from a [`str`]
pub(crate) fn label_has_slash(item: &str) -> Error {
    Error::Label(LabelError::ContainsSlash(item.into()))
}

/// Error type for all file system errors that can occur.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A `LabelError` specific error for parsing a [`Path`](super::path::Label).
    #[error("Label error: {0}")]
    Label(#[from] LabelError),
    /// A `PathError` specific error for parsing a [`Path`](super::path::Path).
    #[error("Path error: {0}")]
    Path(#[from] PathError),
}

/// Parse errors for when parsing a string to a [`Path`](super::path::Path).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathError {
    /// An error signifying that a [`Path`](super::path::Path) is empty.
    #[error("Path is empty")]
    Empty,
}

/// Parse errors for when parsing a string to a [`Label`](super::path::Label).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LabelError {
    /// An error signifying that a [`Label`](super::path::Label) is contains invalid UTF-8.
    #[error("Label contains invalid UTF-8: {0}")]
    InvalidUTF8(String),
    /// An error signifying that a [`Label`](super::path::Label) contains a `/`.
    #[error("Label contains a slash: {0}")]
    ContainsSlash(String),
    /// An error signifying that a [`Label`](super::path::Label) is empty.
    #[error("Label is empty")]
    Empty,
}
