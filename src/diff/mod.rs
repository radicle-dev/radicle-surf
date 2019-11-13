use crate::file_system::{Directory, Label};
use nonempty::NonEmpty;

pub struct Location {}

pub enum Change {
    AddLine {
        path: NonEmpty<Label>,
        location: Location,
        contents: String,
    },
    RemoveLine {
        path: NonEmpty<Label>,
        location: Location,
    },
    MoveFile {
        old_path: NonEmpty<Label>,
        new_path: NonEmpty<Label>,
    },
    CreateFile {
        path: NonEmpty<Label>,
    },
    DeleteFile {
        path: NonEmpty<Label>,
    },
}

pub struct Diff(pub Vec<Change>);

impl Diff {
    // TODO(fintan): This is a bit more involved going to elide for now
    #[allow(dead_code)]
    fn diff<Repo>(_directory: Directory<Repo>, _directory_: Directory<Repo>) -> Self {
        panic!("TODO")
    }
}
