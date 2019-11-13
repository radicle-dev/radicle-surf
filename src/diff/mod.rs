use crate::file_system::Label;
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
