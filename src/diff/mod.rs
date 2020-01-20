#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, DirectoryContents, Label, Path};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug)]
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
    pub modified: Vec<ModifiedFile>,
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
pub struct ModifiedFile {
    pub path: Path,
    pub diff: FileDiff,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileDiff {
    // TODO
}

impl Diff {
    fn new() -> Self {
        Diff {
            created: Vec::new(),
            deleted: Vec::new(),
            moved: Vec::new(),
            modified: Vec::new(),
        }
    }

    // TODO: Direction of comparison is not obvious with this signature.
    // For now using conventional approach with the right being "newer".
    pub fn diff(left: Directory, right: Directory) -> Result<Diff, DiffError> {
        let mut diff = Diff::new();
        let path = Rc::new(RefCell::new(Path::from_labels(right.current(), &[])));
        Diff::collect_diff(&left, &right, &path, &mut diff)?;

        // TODO: Some of the deleted files may actually be moved (renamed) to one of the created files.
        // Finding out which of the deleted files were deleted and which were moved will probably require
        // performing some variant of the longest common substring algorithm for each pair in D x C.
        // Final decision can be based on heuristics, e.g. the file can be considered moved,
        // if len(LCS) > 0,25 * min(size(d), size(c)), and deleted otherwise.

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
                        }
                        Ordering::Less => {
                            diff.add_created_files(new_entry, parent_path)?;
                            new_entry_opt = new_iter.next();
                        }
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
                                    diff.add_modified_file(
                                        &new_file_name,
                                        &RefCell::borrow(parent_path),
                                    );
                                }
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            }
                            (
                                DirectoryContents::File {
                                    name: new_file_name,
                                    file: new_file,
                                },
                                DirectoryContents::Directory(old_dir),
                            ) => {
                                diff.add_created_file(
                                    &new_file_name,
                                    &RefCell::borrow(parent_path),
                                );
                                diff.add_deleted_files(old_entry, parent_path)?;
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            }
                            (
                                DirectoryContents::Directory(new_dir),
                                DirectoryContents::File {
                                    name: old_file_name,
                                    file: old_file,
                                },
                            ) => {
                                diff.add_created_files(new_entry, parent_path)?;
                                diff.add_deleted_file(
                                    &old_file_name,
                                    &RefCell::borrow(parent_path),
                                );
                                old_entry_opt = old_iter.next();
                                new_entry_opt = new_iter.next();
                            }
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
                            }
                        },
                    }
                }
                (Some(ref old_entry), None) => {
                    diff.add_deleted_files(old_entry, parent_path)?;
                    old_entry_opt = old_iter.next();
                }
                (None, Some(ref new_entry)) => {
                    diff.add_created_files(new_entry, parent_path)?;
                    new_entry_opt = new_iter.next();
                }
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
        F: Fn(&Label, &Path) -> T + Copy,
    {
        match entry {
            DirectoryContents::Directory(dir) => Diff::collect_files(dir, parent_path, mapper),
            DirectoryContents::File { name, .. } => {
                let mapped = mapper(name, &RefCell::borrow(parent_path));
                Ok(vec![mapped])
            }
        }
    }

    fn collect_files<F, T>(
        dir: &Directory,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
    ) -> Result<Vec<T>, String>
    where
        F: Fn(&Label, &Path) -> T + Copy,
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
        F: Fn(&Label, &Path) -> T + Copy,
    {
        parent_path.borrow_mut().push(dir.current().clone());
        for entry in dir.iter() {
            match entry {
                DirectoryContents::Directory(subdir) => {
                    Diff::collect_files_inner(&subdir, parent_path, mapper, files)?;
                }
                DirectoryContents::File { name, .. } => {
                    files.push(mapper(&name, &RefCell::borrow(parent_path)));
                }
            }
        }
        parent_path.borrow_mut().pop();
        Ok(())
    }

    fn convert_to_deleted(name: &Label, parent_path: &Path) -> DeleteFile {
        DeleteFile(Diff::build_path(&name, parent_path))
    }

    fn convert_to_created(name: &Label, parent_path: &Path) -> CreateFile {
        CreateFile(Diff::build_path(&name, parent_path))
    }

    fn add_modified_file(&mut self, name: &Label, parent_path: &Path) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        self.modified.push(ModifiedFile {
            path: Diff::build_path(&name, parent_path),
            diff: FileDiff {},
        });
    }

    fn add_created_file(&mut self, name: &Label, parent_path: &Path) {
        self.created
            .push(Diff::convert_to_created(name, parent_path));
    }

    fn add_created_files(
        &mut self,
        dc: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
    ) -> Result<(), String> {
        let mut new_files: Vec<CreateFile> =
            Diff::collect_files_from_entry(dc, &parent_path, Diff::convert_to_created)?;
        self.created.append(&mut new_files);
        Ok(())
    }

    fn add_deleted_file(&mut self, name: &Label, parent_path: &Path) {
        self.deleted
            .push(Diff::convert_to_deleted(name, parent_path));
    }

    fn add_deleted_files(
        &mut self,
        dc: &DirectoryContents,
        parent_path: &Rc<RefCell<Path>>,
    ) -> Result<(), String> {
        let mut new_files: Vec<DeleteFile> =
            Diff::collect_files_from_entry(dc, &parent_path, Diff::convert_to_deleted)?;
        self.deleted.append(&mut new_files);
        Ok(())
    }

    fn build_path(name: &Label, parent_path: &Path) -> Path {
        let mut result_path = parent_path.clone();
        result_path.push(name.clone());
        result_path
    }
}

#[cfg(test)]
mod tests {
    use crate::diff::*;
    use crate::file_system::unsound;
    use crate::file_system::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_create_file() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![CreateFile(Path::with_root(&[unsound::label::new(
                "banana.rs",
            )]))],
            deleted: vec![],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_file() {
        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let new_directory = Directory::root();

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile(Path::with_root(&[unsound::label::new(
                "banana.rs",
            )]))],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_moved_file() {
        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("mod.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        // TODO(fintan): Move is not detected yet
        // assert_eq!(diff, Diff::new())
        assert!(true)
    }

    #[test]
    fn test_modify_file() {
        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana"));

        let mut new_directory = Directory::root();
        new_directory.insert_file(&unsound::path::new("banana.rs"), File::new(b"use banana;"));

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[unsound::label::new("banana.rs")]),
                diff: FileDiff {},
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_create_directory() {
        let directory = Directory::root();

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            &unsound::path::new("src/banana.rs"),
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
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            &unsound::path::new("src/banana.rs"),
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
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_modify_file_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            &unsound::path::new("src/banana.rs"),
            File::new(b"use banana"),
        );

        let mut new_directory = Directory::root();
        new_directory.insert_file(
            &unsound::path::new("src/banana.rs"),
            File::new(b"use banana;"),
        );

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("banana.rs"),
                ]),
                diff: FileDiff {},
            }],
        };

        assert_eq!(diff, expected_diff)
    }

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

        // TODO(fintan): Tricky stuff
        // assert_eq!(diff, expected_diff)
        assert!(true)
    }
}
