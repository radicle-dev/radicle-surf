//! Errors that can occur within the file system logic.
//!
//! These errors occur due to [`Label`] and [`Path`] parsing when using their respective `TryFrom`
//! instances.

use thiserror::Error;

pub(crate) const EMPTY_PATH: Error = Error::Path(Path::Empty);

pub(crate) const INVALID_UTF8: Error = Error::Label(Label::InvalidUTF8);
pub(crate) const EMPTY_LABEL: Error = Error::Label(Label::Empty);
pub(crate) const CONTAINS_SLASH: Error = Error::Label(Label::ContainsSlash);

/// Error type for all file system errors that can occur.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A `Label` specific error for parsing a [`Label`].
    #[error("Label error: {0}")]
    Label(#[from] Label),
    /// A `Path` specific error for parsing a [`Path`].
    #[error("Path error: {0}")]
    Path(#[from] Path),
}

/// Parse errors for when parsing a string to a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Path {
    /// An error signifying that a [`Path`] is empty.
    #[error("Path is empty")]
    Empty,
}

/// Parse errors for when parsing a string to a [`Label`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Label {
    /// An error signifying that a [`Label`] is contains invalid UTF-8.
    #[error("Label contains invalid UTF-8")]
    InvalidUTF8,
    /// An error signifying that a [`Label`] contains a `/`.
    #[error("Label contains a slash")]
    ContainsSlash,
    /// An error signifying that a [`Label`] is empty.
    #[error("Label is empty")]
    Empty,
}
