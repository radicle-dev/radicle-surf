// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![allow(dead_code, unused_variables, missing_docs)]

use crate::file_system::{Directory, DirectoryContents, Path};
use std::{cell::RefCell, cmp::Ordering, ops::Deref, rc::Rc};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("A diff error occurred: {reason}")]
pub struct DiffError {
    reason: String,
}

impl From<String> for DiffError {
    fn from(reason: String) -> Self {
        DiffError { reason }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Diff {
    pub created: Vec<CreateFile>,
    pub deleted: Vec<DeleteFile>,
    pub moved: Vec<MoveFile>,
    pub copied: Vec<CopyFile>,
    pub modified: Vec<ModifiedFile>,
}

impl Default for Diff {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateFile(pub Path);

#[derive(Debug, PartialEq, Eq)]
pub struct DeleteFile(pub Path);

#[derive(Debug, PartialEq, Eq)]
pub struct MoveFile {
    pub old_path: Path,
    pub new_path: Path,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CopyFile {
    pub old_path: Path,
    pub new_path: Path,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModifiedFile {
    pub path: Path,
    pub diff: FileDiff,
}

/// A set of changes belonging to one file.
#[derive(Debug, PartialEq, Eq)]
pub enum FileDiff {
    Binary,
    Plain(Vec<Hunk>),
}

/// A set of line changes.
#[derive(Debug, PartialEq, Eq)]
pub struct Hunk {
    pub header: Line,
    pub lines: Vec<LineDiff>,
}

/// The content of a single line.
pub type Line = Vec<u8>;

/// Single line delta. Two of these are need to represented a modified line: one
/// addition and one deletion. Context is also represented with this type.
#[derive(Debug, PartialEq, Eq)]
pub struct LineDiff {
    /// Line number in old or new file.
    pub line_num: u32,
    /// Line content.
    pub line: Line,
    /// Line diff kind, eg. addition or deletion.
    pub kind: LineDiffKind,
}

/// The kind or "status" of a `LineDiff`.
#[derive(Debug, PartialEq, Eq)]
pub enum LineDiffKind {
    /// Line addition.
    Addition,
    /// Line deletion.
    Deletion,
    /// Line context.
    Context,
}

impl From<LineDiffKind> for char {
    fn from(kind: LineDiffKind) -> Self {
        match kind {
            LineDiffKind::Addition => '+',
            LineDiffKind::Deletion => '-',
            LineDiffKind::Context => ' ',
        }
    }
}

impl LineDiff {
    pub fn addition(line: Line, line_num: u32) -> Self {
        Self {
            line_num,
            line,
            kind: LineDiffKind::Addition,
        }
    }

    pub fn deletion(line: Line, line_num: u32) -> Self {
        Self {
            line_num,
            line,
            kind: LineDiffKind::Deletion,
        }
    }

    pub fn context(line: Line, line_num: u32) -> Self {
        Self {
            line_num,
            line,
            kind: LineDiffKind::Context,
        }
    }
}

impl Diff {
    pub fn new() -> Self {
        Diff {
            created: Vec::new(),
            deleted: Vec::new(),
            moved: Vec::new(),
            copied: Vec::new(),
            modified: Vec::new(),
        }
    }

    // TODO: Direction of comparison is not obvious with this signature.
    // For now using conventional approach with the right being "newer".
    pub fn diff(left: Directory, right: Directory) -> Result<Diff, DiffError> {
        let mut diff = Diff::new();
        let path = Rc::new(RefCell::new(Path::from_labels(right.current(), &[])));
        Diff::collect_diff(&left, &right, &path, &mut diff)?;

        // TODO: Some of the deleted files may actually be moved (renamed) to one of the
        // created files. Finding out which of the deleted files were deleted
        // and which were moved will probably require performing some variant of
        // the longest common substring algorithm for each pair in D x C. Final
        // decision can be based on heuristics, e.g. the file can be considered
        // moved, if len(LCS) > 0,25 * min(size(d), size(c)), and
        // deleted otherwise.

        Ok(diff)
    }

    fn collect_diff(
        old: &Directory,
        new: &Directory,
        parent_path: &Rc<RefCell<Path>>,
        diff: &mut Diff,
    ) -> Result<(), String> {
        let mut old_iter = old.iter();
        let mut new_iter = new.iter();
        let mut old_entry_opt = old_iter.next();
        let mut new_entry_opt = new_iter.next();

        while old_entry_opt.is_some() || new_entry_opt.is_some() {
            match (&old_entry_opt, &new_entry_opt) {
                (Some(ref old_entry), Some(ref new_entry)) => {
                    match new_entry.label().cmp(&old_entry.label()) {
                        Ordering::Greater => {
                            diff.add_deleted_files(old_entry, parent_path)?;
                            old_entry_opt = old_iter.next();
                        },
                        Ordering::Less => {
                            diff.add_created_files(new_entry, parent_path)?;
                            new_entry_opt = new_iter.next();
                        },
                        Ordering::Equal => match (new_entry, old_entry) {
                            (
                                DirectoryContents::File {
                                    name: new_file_name,
                                    file: new_file,
                                },
                                DirectoryContents::File {
                                    name: old_file_name,
                                    file: old_file,
                                },
                            ) => {
                                if old_file.size != new_file.size
                                    || old_file.checksum() != new_file.checksum()
                                {
                                    let mut path = parent_path.borrow().clone();
                                    path.push(new_file_name.clone());

                                    diff.add_modified_file(path, vec![]);
                                }
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::File {
                                    name: new_file_name,
                                    file: new_file,
                                },
                                DirectoryContents::Directory(old_dir),
                            ) => {
                                let mut path = parent_path.borrow().clone();
                                path.push(new_file_name.clone());

                                diff.add_created_file(path);
                                diff.add_deleted_files(old_entry, parent_path)?;

                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::Directory(new_dir),
                                DirectoryContents::File {
                                    name: old_file_name,
                                    file: old_file,
                                },
                            ) => {
                                let mut path = parent_path.borrow().clone();
                                path.push(old_file_name.clone());

                                diff.add_created_files(new_entry, parent_path)?;
                                diff.add_deleted_file(path);

                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                            (
                                DirectoryContents::Directory(new_dir),
                                DirectoryContents::Directory(old_dir),
                            ) => {
                                parent_path.borrow_mut().push(new_dir.current().clone());
                                Diff::collect_diff(
                                    old_dir.deref(),
                                    new_dir.deref(),
                                    parent_path,
                                    diff,
                                )?;
                                parent_path.borrow_mut().pop();
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            },
                        },
                    }
                },
                (Some(ref old_entry), None) => {
                    diff.add_deleted_files(old_entry, parent_path)?;
                    old_entry_opt = old_iter.next();
                },
                (None, Some(ref new_entry)) => {
                    diff.add_created_files(new_entry, parent_path)?;
                    new_entry_opt = new_iter.next();
                },
                (None, None) => break,
            }
        }
        Ok(())
    }

    // if entry is a file, then return this file,
    // or a list of files in the directory tree otherwise
    fn collect_files_from_entry<F, T>(
        entry: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
    ) -> Result<Vec<T>, String>
    where
        F: Fn(Path) -> T + Copy,
    {
        match entry {
            DirectoryContents::Directory(dir) => Diff::collect_files(dir, parent_path, mapper),
            DirectoryContents::File { name, .. } => {
                let mut path = parent_path.borrow().clone();
                path.push(name.clone());

                Ok(vec![mapper(path)])
            },
        }
    }

    fn collect_files<F, T>(
        dir: &Directory,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
    ) -> Result<Vec<T>, String>
    where
        F: Fn(Path) -> T + Copy,
    {
        let mut files: Vec<T> = Vec::new();
        Diff::collect_files_inner(dir, parent_path, mapper, &mut files)?;
        Ok(files)
    }

    fn collect_files_inner<'a, F, T>(
        dir: &'a Directory,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
        files: &mut Vec<T>,
    ) -> Result<(), String>
    where
        F: Fn(Path) -> T + Copy,
    {
        parent_path.borrow_mut().push(dir.current());
        for entry in dir.iter() {
            match entry {
                DirectoryContents::Directory(subdir) => {
                    Diff::collect_files_inner(&subdir, parent_path, mapper, files)?;
                },
                DirectoryContents::File { name, .. } => {
                    let mut path = parent_path.borrow().clone();
                    path.push(name);
                    files.push(mapper(path));
                },
            }
        }
        parent_path.borrow_mut().pop();
        Ok(())
    }

    pub(crate) fn add_modified_file(&mut self, path: Path, hunks: Vec<Hunk>) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        self.modified.push(ModifiedFile {
            path,
            diff: FileDiff::Plain(hunks),
        });
    }

    pub(crate) fn add_moved_file(&mut self, old_path: Path, new_path: Path) {
        self.moved.push(MoveFile { old_path, new_path });
    }

    pub(crate) fn add_copied_file(&mut self, old_path: Path, new_path: Path) {
        self.copied.push(CopyFile { old_path, new_path });
    }

    pub(crate) fn add_modified_binary_file(&mut self, path: Path) {
        self.modified.push(ModifiedFile {
            path,
            diff: FileDiff::Binary,
        });
    }

    pub(crate) fn add_created_file(&mut self, path: Path) {
        self.created.push(CreateFile(path));
    }

    fn add_created_files(
        &mut self,
        dc: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
    ) -> Result<(), String> {
        let mut new_files: Vec<CreateFile> =
            Diff::collect_files_from_entry(dc, &parent_path, CreateFile)?;
        self.created.append(&mut new_files);
        Ok(())
    }

    pub(crate) fn add_deleted_file(&mut self, path: Path) {
        self.deleted.push(DeleteFile(path));
    }

    fn add_deleted_files(
        &mut self,
        dc: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
    ) -> Result<(), String> {
        let mut new_files: Vec<DeleteFile> =
            Diff::collect_files_from_entry(dc, &parent_path, DeleteFile)?;
        self.deleted.append(&mut new_files);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        diff::*,
        file_system::{unsound, *},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_create_file() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![CreateFile(Path::with_root(&[unsound::label::new(
                "banana.rs",
            )]))],
            deleted: vec![],
            copied: vec![],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_file() {
        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let new_directory = Directory::root();

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile(Path::with_root(&[unsound::label::new(
                "banana.rs",
            )]))],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    /* TODO(fintan): Move is not detected yet
    #[test]
    fn test_moved_file() {
        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("mod.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        assert_eq!(diff, Diff::new())
    }
    */

    #[test]
    fn test_modify_file() {
        let mut directory = Directory::root();
        directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(unsound::path::new("banana.rs"), File::new(b"use banana;"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[unsound::label::new("banana.rs")]),
                diff: FileDiff::Plain(vec![]),
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_create_directory() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![CreateFile(Path::with_root(&[
                unsound::label::new("src"),
                unsound::label::new("banana.rs"),
            ]))],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let new_directory = Directory::root();

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile(Path::with_root(&[
                unsound::label::new("src"),
                unsound::label::new("banana.rs"),
            ]))],
            moved: vec![],
            copied: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_modify_file_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            unsound::path::new("src/banana.rs"),
            File::new(b"use banana;"),
        );

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            copied: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("banana.rs"),
                ]),
                diff: FileDiff::Plain(vec![]),
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    /* TODO(fintan): Tricky stuff
    #[test]
    fn test_disjoint_directories() {
        let mut directory = Directory::root();
        directory.insert_file(
            &unsound::path::new("foo/src/banana.rs"),
            File::new(b"use banana"),
        );

        let mut other_directory = Directory::root();
        other_directory.insert_file(
            &unsound::path::new("bar/src/pineapple.rs"),
            File::new(b"use pineapple"),
        );

        let diff = Diff::diff(directory, other_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![CreateFile(Path::from_labels(
                unsound::label::new("bar"),
                &[
                    unsound::label::new("src"),
                    unsound::label::new("pineapple.rs"),
                ],
            ))],
            deleted: vec![DeleteFile(Path::from_labels(
                unsound::label::new("foo"),
                &[unsound::label::new("src"), unsound::label::new("banana.rs")],
            ))],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }
    */
}
