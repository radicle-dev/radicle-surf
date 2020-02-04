use git2;
use git2::{Delta, DiffLine, Patch};
use std::convert::TryFrom;
use std::fmt;
use std::path;
use std::str;

#[derive(Debug)]
pub enum Error {
    Git(git2::Error),
    MissingNewLine,
    MissingOldLine,
    UnhandledChange(char),
    MissingNewPath,
    MissingOldPath,
    UnhandledDelta(Delta),
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Error::Git(err)
    }
}

#[derive(Debug)]
pub struct Content {
    pub line: u32,
    pub offset: i64,
    pub contents: Vec<u8>,
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = str::from_utf8(&self.contents).expect("Utf8-Error");
        write!(
            f,
            "line no: {}\nline offset: {}\ncontents: {}",
            self.line, self.offset, contents
        )
    }
}

#[derive(Debug)]
pub enum ContentChange {
    Addition(Content),
    Deletion(Content),
    Context(Content),
}

impl fmt::Display for ContentChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentChange::Addition(content) => write!(f, "++++\n{}", content),
            ContentChange::Deletion(content) => write!(f, "----\n{}", content),
            ContentChange::Context(content) => write!(f, "{}", content),
        }
    }
}

impl<'a> TryFrom<DiffLine<'a>> for ContentChange {
    type Error = Error;

    fn try_from(diff_line: DiffLine) -> Result<Self, Self::Error> {
        match diff_line.origin() {
            '+' => {
                let content = Content {
                    line: diff_line.new_lineno().ok_or(Error::MissingNewLine)?,
                    offset: diff_line.content_offset(),
                    contents: diff_line.content().to_vec(),
                };
                Ok(ContentChange::Addition(content))
            },
            '-' => {
                let content = Content {
                    line: diff_line.old_lineno().ok_or(Error::MissingOldLine)?,
                    offset: diff_line.content_offset(),
                    contents: diff_line.content().to_vec(),
                };
                Ok(ContentChange::Deletion(content))
            },
            ' ' => {
                let content = Content {
                    line: diff_line.old_lineno().ok_or(Error::MissingOldLine)?,
                    offset: diff_line.content_offset(),
                    contents: diff_line.content().to_vec(),
                };
                Ok(ContentChange::Context(content))
            },
            c => Err(Error::UnhandledChange(c)),
        }
    }
}

#[derive(Debug)]
pub enum FileChange {
    Addition(path::PathBuf),
    Deletion(path::PathBuf),
    Move {
        old_path: path::PathBuf,
        new_path: path::PathBuf,
    },
    Modification {
        path: path::PathBuf,
        changes: Vec<ContentChange>,
    },
}

impl fmt::Display for FileChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileChange::Addition(path) => write!(f, "+ {:?}", path),
            FileChange::Deletion(path) => write!(f, "- n{:?}", path),
            FileChange::Move { old_path, new_path } => {
                write!(f, "{:?} -> {:?}", old_path, new_path)
            },
            FileChange::Modification { path, changes } => {
                write!(f, "{:?}", path)?;
                for change in changes {
                    write!(f, "\n{}", change)?
                }
                Ok(())
            },
        }
    }
}

impl TryFrom<Patch> for FileChange {
    type Error = Error;

    fn try_from(patch: Patch) -> Result<Self, Self::Error> {
        let diff_delta = patch.delta();

        match diff_delta.status() {
            Delta::Added => {
                let path = diff_delta
                    .new_file()
                    .path()
                    .ok_or(Error::MissingNewPath)?
                    .to_path_buf();
                Ok(FileChange::Addition(path))
            },
            Delta::Deleted => {
                let path = diff_delta
                    .old_file()
                    .path()
                    .ok_or(Error::MissingOldPath)?
                    .to_path_buf();
                Ok(FileChange::Deletion(path))
            },
            Delta::Modified => {
                let path = diff_delta
                    .new_file()
                    .path()
                    .ok_or(Error::MissingNewPath)?
                    .to_path_buf();
                let mut changes = vec![];
                let n_hunks = patch.num_hunks();

                for i in 0..n_hunks {
                    let n_lines = patch.num_lines_in_hunk(i)?;

                    for l in 0..n_lines {
                        let diff_line = patch.line_in_hunk(i, l)?;
                        let change = ContentChange::try_from(diff_line)?;
                        changes.push(change);
                    }
                }

                Ok(FileChange::Modification { path, changes })
            },
            Delta::Renamed => {
                let old_path = diff_delta
                    .old_file()
                    .path()
                    .ok_or(Error::MissingOldPath)?
                    .to_path_buf();
                let new_path = diff_delta
                    .new_file()
                    .path()
                    .ok_or(Error::MissingNewPath)?
                    .to_path_buf();
                Ok(FileChange::Move { old_path, new_path })
            },
            d => Err(Error::UnhandledDelta(d)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcs::git::*;
    use git2;

    #[test]
    fn test_diff() -> Result<(), ()> {
        let repo = Repository::new("./data/git-platinum").expect("Repo Uhoh");
        let head = repo
            .0
            .find_commit(
                Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95").expect("Commit Uhoh"),
            )
            .expect("Commit Uhoh");
        let head1 = repo
            .0
            .find_commit(
                Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5").expect("Commit Uhoh"),
            )
            .expect("Commit Uhoh");

        let diff = repo.diff_commits(&head, Some(&head1)).expect("Diff Uhoh");

        let patch = git2::Patch::from_diff(&diff, 0)
            .expect("Uhoh Patch")
            .expect("Uhoh No Patch");

        println!("{}", FileChange::try_from(patch).expect("Error"));

        Err(())
    }
}
