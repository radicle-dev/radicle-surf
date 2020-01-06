pub(crate) const EMPTY_PATH: Error = Error::Path(Path::Empty);

pub(crate) const INVALID_UTF8: Error = Error::Label(Label::InvalidUTF8);
pub(crate) const EMPTY_LABEL: Error = Error::Label(Label::Empty);
pub(crate) const CONTAINS_SLASH: Error = Error::Label(Label::ContainsSlash);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Label(Label),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Path {
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    InvalidUTF8,
    ContainsSlash,
    Empty,
}
