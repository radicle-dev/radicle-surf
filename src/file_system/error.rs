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
    Label(Label),
}

impl From<Label> for Path {
    fn from(err: Label) -> Self {
        Path::Label(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    ContainsSlash,
    Empty,
}
