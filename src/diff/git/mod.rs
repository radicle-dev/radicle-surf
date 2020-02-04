use git2::*;
use std::convert::TryFrom;
use std::path;

pub struct Content {
    line: u32,
    offset: i64,
    contents: Vec<u8>,
}

pub enum ContentChange {
    Addition(Content),
    Deletion(Content),
}

impl<'a> TryFrom<DiffLine<'a>> for ContentChange {
    type Error = ();

    fn try_from(diff_line: DiffLine) -> Result<Self, Self::Error> {
        match diff_line.origin() {
            '+' => {
                let content = Content {
                    line: diff_line.new_lineno().ok_or(())?,
                    offset: diff_line.content_offset(),
                    contents: diff_line.content().to_vec(),
                };
                Ok(ContentChange::Addition(content))
            }
            '-' => {
                let content = Content {
                    line: diff_line.old_lineno().ok_or(())?,
                    offset: diff_line.content_offset(),
                    contents: diff_line.content().to_vec(),
                };
                Ok(ContentChange::Deletion(content))
            }
            _ => Err(()),
        }
    }
}

pub enum FileChange {
    Addition {
        path: path::PathBuf,
    },
    Deletion {
        path: path::PathBuf,
    },
    Move {
        old_path: path::PathBuf,
        new_path: path::PathBuf,
    },
    Modification {
        path: path::PathBuf,
        changes: Vec<ContentChange>,
    },
}

impl TryFrom<Patch> for FileChange {
    type Error = ();

    fn try_from(patch: Patch) -> Result<Self, Self::Error> {
        let diff_delta = patch.delta();

        match diff_delta.status() {
            Delta::Added => {
                let path = diff_delta.new_file().path().ok_or(())?.to_path_buf();
                Ok(FileChange::Addition { path })
            }
            Delta::Deleted => {
                let path = diff_delta.old_file().path().ok_or(())?.to_path_buf();
                Ok(FileChange::Deletion { path })
            }
            Delta::Modified => {
                let path = diff_delta.new_file().path().ok_or(())?.to_path_buf();
                let mut changes = vec![];
                let n_hunks = patch.num_hunks();

                for i in 0..n_hunks {
                    let n_lines = patch.num_lines_in_hunk(i).map_err(|_| ())?;

                    for l in 0..n_lines {
                        let diff_line = patch.line_in_hunk(i, l).map_err(|_| ())?;
                        let change = ContentChange::try_from(diff_line)?;
                        changes.push(change);
                    }
                }

                Ok(FileChange::Modification { path, changes })
            }
            Delta::Renamed => {
                let old_path = diff_delta.old_file().path().ok_or(())?.to_path_buf();
                let new_path = diff_delta.new_file().path().ok_or(())?.to_path_buf();
                Ok(FileChange::Move { old_path, new_path })
            }
            _ => Err(()),
        }
    }
}
