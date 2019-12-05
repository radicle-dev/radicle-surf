#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, Path};
use crate::file_system::{DirectoryContents, File};
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
        let path = Rc::new(RefCell::new(Path::from_labels(right.label.clone(), &[])));
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
        // TODO: Consider storing directory contents in sorted order
        let old = get_sorted_contents(old);
        let new = get_sorted_contents(new);

        let mut old_iter = old.iter();
        let mut new_iter = new.iter();
        let mut old_entry_opt = old_iter.next();
        let mut new_entry_opt = new_iter.next();

        while old_entry_opt.is_some() || new_entry_opt.is_some() {
            match (old_entry_opt, new_entry_opt) {
                (Some(old_entry), Some(new_entry)) => {
                    match new_entry.label().cmp(&old_entry.label()) {
                        Ordering::Greater => {
                            diff.add_deleted_files(old_entry, parent_path)?;
                            old_entry_opt = old_iter.next();
                        }
                        Ordering::Less => {
                            diff.add_created_files(new_entry, parent_path)?;
                            new_entry_opt = new_iter.next();
                        }
                        Ordering::Equal => {
                            use DirectoryContents::{File, Repo, SubDirectory};
                            match (new_entry, old_entry) {
                                (File(new_file), File(old_file)) => {
                                    if old_file.size != new_file.size
                                        || old_file.checksum() != new_file.checksum()
                                    {
                                        diff.add_modified_file(
                                            new_file,
                                            &RefCell::borrow(parent_path),
                                        );
                                    }
                                    old_entry_opt = old_iter.next();
                                    new_entry_opt = new_iter.next();
                                }
                                (File(new_file), SubDirectory(old_dir)) => {
                                    diff.add_created_file(new_file, &RefCell::borrow(parent_path));
                                    diff.add_deleted_files(old_entry, parent_path)?;
                                    old_entry_opt = old_iter.next();
                                    new_entry_opt = new_iter.next();
                                }
                                (SubDirectory(new_dir), File(old_file)) => {
                                    diff.add_created_files(new_entry, parent_path)?;
                                    diff.add_deleted_file(old_file, &RefCell::borrow(parent_path));
                                    old_entry_opt = old_iter.next();
                                    new_entry_opt = new_iter.next();
                                }
                                (SubDirectory(new_dir), SubDirectory(old_dir)) => {
                                    parent_path.borrow_mut().push(new_dir.label.clone());
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
                                // need to skip Repos
                                (Repo, Repo) => {
                                    old_entry_opt = old_iter.next();
                                    new_entry_opt = new_iter.next();
                                }
                                (Repo, _) => {
                                    old_entry_opt = old_iter.next();
                                }
                                (_, Repo) => {
                                    new_entry_opt = new_iter.next();
                                }
                            }
                        }
                    }
                }
                (Some(old_entry), None) => {
                    diff.add_deleted_files(old_entry, parent_path)?;
                    old_entry_opt = old_iter.next();
                }
                (None, Some(new_entry)) => {
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
        F: Fn(&File, &Path) -> T + Copy,
    {
        match entry {
            DirectoryContents::SubDirectory(dir) => Diff::collect_files(dir, parent_path, mapper),
            DirectoryContents::File(file) => {
                let mapped = mapper(file, &RefCell::borrow(parent_path));
                Ok(vec![mapped])
            }
            DirectoryContents::Repo => Ok(vec![]),
        }
    }

    fn collect_files<F, T>(
        dir: &Directory,
        parent_path: &Rc<RefCell<Path>>,
        mapper: F,
    ) -> Result<Vec<T>, String>
    where
        F: Fn(&File, &Path) -> T + Copy,
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
        F: Fn(&File, &Path) -> T + Copy,
    {
        parent_path.borrow_mut().push(dir.label.clone());
        for entry in dir.entries.iter() {
            match entry {
                DirectoryContents::SubDirectory(subdir) => {
                    Diff::collect_files_inner(subdir.deref(), parent_path, mapper, files)?;
                }
                DirectoryContents::File(file) => {
                    files.push(mapper(file, &RefCell::borrow(parent_path)));
                }
                DirectoryContents::Repo => { /* Skip repo directory */ }
            }
        }
        parent_path.borrow_mut().pop();
        Ok(())
    }

    fn convert_to_deleted(file: &File, parent_path: &Path) -> DeleteFile {
        DeleteFile(Diff::build_path(file, parent_path))
    }

    fn convert_to_created(file: &File, parent_path: &Path) -> CreateFile {
        CreateFile(Diff::build_path(file, parent_path))
    }

    fn add_modified_file(&mut self, file: &File, parent_path: &Path) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        self.modified.push(ModifiedFile {
            path: Diff::build_path(file, parent_path),
            diff: FileDiff {},
        });
    }

    fn add_created_file(&mut self, file: &File, parent_path: &Path) {
        self.created
            .push(Diff::convert_to_created(file, parent_path));
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

    fn add_deleted_file(&mut self, file: &File, parent_path: &Path) {
        self.deleted
            .push(Diff::convert_to_deleted(file, parent_path));
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

    fn build_path(file: &File, parent_path: &Path) -> Path {
        let mut result_path = parent_path.clone();
        result_path.push(file.filename.clone());
        result_path
    }
}

// returns list of contents, sorted by label; Repos are prepended to the beginning
fn get_sorted_contents(dir: &Directory) -> Vec<&DirectoryContents> {
    let mut vec: Vec<&DirectoryContents> = dir.entries.iter().collect();
    vec.sort_by_key(|e| match e {
        DirectoryContents::SubDirectory(subdir) => Some(subdir.label.clone()),
        DirectoryContents::File(file) => Some(file.filename.clone()),
        DirectoryContents::Repo => None,
    });
    vec
}

#[cfg(test)]
mod tests {
    use crate::diff::*;
    use crate::file_system::*;
    use nonempty::NonEmpty;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Clone)]
    struct TestRepo {}

    impl RepoBackend for TestRepo {
        fn repo_directory() -> Directory {
            Directory {
                label: ".test".into(),
                entries: NonEmpty::new(DirectoryContents::Repo),
            }
        }
    }

    #[test]
    fn test_create_file() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::Repo),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::file("banana.rs".into(), b"use banana")],
            )
                .into(),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![CreateFile(Path::with_root(&["banana.rs".into()]))],
            deleted: vec![],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_file() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::file("banana.rs".into(), b"use banana")],
            )
                .into(),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::Repo),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile(Path::with_root(&["banana.rs".into()]))],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_moved_file() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::file("mod.rs".into(), b"use banana")),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::file("banana.rs".into(), b"use banana")),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");

        // TODO(fintan): Move is not detected yet
        // assert_eq!(diff, Diff::new())
        assert!(true)
    }

    #[test]
    fn test_modify_file() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::file("banana.rs".into(), b"use banana")],
            )
                .into(),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::file("banana.rs".into(), b"use banana;")],
            )
                .into(),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&["banana.rs".into()]),
                diff: FileDiff {},
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_create_directory() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::Repo),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::sub_directory(Directory {
                    label: "src".into(),
                    entries: NonEmpty::new(DirectoryContents::file(
                        "banana.rs".into(),
                        b"use banana",
                    )),
                })],
            )
                .into(),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![CreateFile(Path::with_root(&[
                "src".into(),
                "banana.rs".into(),
            ]))],
            deleted: vec![],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_delete_directory() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::sub_directory(Directory {
                    label: "src".into(),
                    entries: NonEmpty::new(DirectoryContents::file(
                        "banana.rs".into(),
                        b"use banana",
                    )),
                })],
            )
                .into(),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::Repo),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![],
            deleted: vec![DeleteFile(Path::with_root(&[
                "src".into(),
                "banana.rs".into(),
            ]))],
            moved: vec![],
            modified: vec![],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_modify_file_directory() {
        let directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::sub_directory(Directory {
                    label: "src".into(),
                    entries: NonEmpty::new(DirectoryContents::file(
                        "banana.rs".into(),
                        b"use banana",
                    )),
                })],
            )
                .into(),
        };

        let new_directory: Directory = Directory {
            label: Label::root(),
            entries: (
                DirectoryContents::Repo,
                vec![DirectoryContents::sub_directory(Directory {
                    label: "src".into(),
                    entries: NonEmpty::new(DirectoryContents::file(
                        "banana.rs".into(),
                        b"use banana;",
                    )),
                })],
            )
                .into(),
        };

        let diff = Diff::diff(directory, new_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![],
            deleted: vec![],
            moved: vec![],
            modified: vec![ModifiedFile {
                path: Path::with_root(&["src".into(), "banana.rs".into()]),
                diff: FileDiff {},
            }],
        };

        assert_eq!(diff, expected_diff)
    }

    #[test]
    fn test_disjoint_directories() {
        let directory: Directory = Directory {
            label: "foo".into(),
            entries: NonEmpty::new(DirectoryContents::sub_directory(Directory {
                label: "src".into(),
                entries: NonEmpty::new(DirectoryContents::file("banana.rs".into(), b"use banana")),
            })),
        };

        let other_directory: Directory = Directory {
            label: "bar".into(),
            entries: NonEmpty::new(DirectoryContents::sub_directory(Directory {
                label: "src".into(),
                entries: NonEmpty::new(DirectoryContents::file(
                    "pineapple.rs".into(),
                    b"use pineapple",
                )),
            })),
        };

        let diff = Diff::diff(directory, other_directory).expect("diff failed");
        let expected_diff = Diff {
            created: vec![CreateFile(Path::from_labels(
                "bar".into(),
                &["src".into(), "pineapple.rs".into()],
            ))],
            deleted: vec![DeleteFile(Path::from_labels(
                "foo".into(),
                &["src".into(), "banana.rs".into()],
            ))],
            moved: vec![],
            modified: vec![],
        };

        // TODO(fintan): Tricky stuff
        // assert_eq!(diff, expected_diff)
        assert!(true)
    }
}
