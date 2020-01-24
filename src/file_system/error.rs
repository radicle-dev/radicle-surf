//! Errors that can occur within the file system logic.
//!
//! These errors occur due to [`Label`] and [`Path`] parsing when using their respective `TryFrom`
//! instances.

pub(crate) const EMPTY_PATH: Error = Error::Path(Path::Empty);

pub(crate) const INVALID_UTF8: Error = Error::Label(Label::InvalidUTF8);
pub(crate) const EMPTY_LABEL: Error = Error::Label(Label::Empty);
pub(crate) const CONTAINS_SLASH: Error = Error::Label(Label::ContainsSlash);

/// Error type for all file system errors that can occur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A `Label` specific error for parsing a [`Label`].
    Label(Label),
    /// A `Path` specific error for parsing a [`Path`].
    Path(Path),
}

impl From<Label> for Error {
    fn from(err: Label) -> Self {
        Error::Label(err)
    }
}

impl From<Path> for Error {
    fn from(err: Path) -> Self {
        Error::Path(err)
    }
}

/// Parse errors for when parsing a string to a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Path {
    /// An error signifying that a [`Path`] is empty.
    Empty,
}

/// Parse errors for when parsing a string to a [`Label`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// An error signifying that a [`Label`] is contains invalid UTF-8.
    InvalidUTF8,
    /// An error signifying that a [`Label`] contains a `/`.
    ContainsSlash,
    /// An error signifying that a [`Label`] is empty.
    Empty,
}
