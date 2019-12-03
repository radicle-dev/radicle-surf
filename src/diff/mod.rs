#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, Label};
use nonempty::NonEmpty;
use crate::file_system::{DirectoryContents, File};
use std::cmp::Ordering;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct DiffError {
    reason: String
}

impl From<String> for DiffError {
    fn from(reason: String) -> Self {
        DiffError { reason }
    }
}

pub struct Diff {
    pub created: Vec<CreateFile>,
    pub deleted: Vec<DeleteFile>,
    pub moved: Vec<MoveFile>,
    pub modified: Vec<ModifiedFile>,
}

pub struct CreateFile {
    pub path: NonEmpty<Label>,
}

pub struct DeleteFile {
    pub path: NonEmpty<Label>,
}

pub struct MoveFile {
    pub old_path: NonEmpty<Label>,
    pub new_path: NonEmpty<Label>,
}

pub struct ModifiedFile {
    pub path: NonEmpty<Label>,
    pub diff: FileDiff,
}

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
        let path = Rc::new(RefCell::new(Vec::new()));
        Diff::collect_diff(&left, &right, &path, &mut diff)?;

        // TODO: Some of the deleted files may actually be moved (renamed) to one of the created files.
        // Finding out which of the deleted files were deleted and which were moved will probably require
        // performing some variant of the longest common substring algorithm for each pair in D x C.
        // Final decision can be based on heuristics, e.g. the file can be considered moved,
        // if len(LCS) > 0,25 * min(size(d), size(c)), and deleted otherwise.

        Ok(diff)
    }

    fn collect_diff(old: &Directory, new: &Directory, parent_path: &Rc<RefCell<Vec<Label>>>, diff: &mut Diff)
        -> Result<(), String> {

        // TODO: Consider storing directory contents in sorted order
        let old = get_sorted_contents(old);
        let new = get_sorted_contents(new);

        let mut old_iter = old.iter();
        let mut new_iter = new.iter();
        let mut old_entry_opt = old_iter.next();
        let mut new_entry_opt = new_iter.next();

        while old_entry_opt.is_some() || new_entry_opt.is_some() {
            // need to skip Repos
            while let Some(DirectoryContents::Repo) = old_entry_opt {
                old_entry_opt = old_iter.next();
            }
            while let Some(DirectoryContents::Repo) = new_entry_opt {
                new_entry_opt = new_iter.next();
            }
            match (old_entry_opt, new_entry_opt) {
                (Some(old_entry), Some(new_entry)) => {
                    let cmp = new_entry.label().cmp(&old_entry.label());
                    match cmp {
                        Ordering::Greater => {
                            diff.add_deleted_files(old_entry, parent_path)?;
                            old_entry_opt = old_iter.next();
                        },
                        Ordering::Less => {
                            diff.add_created_files(new_entry, parent_path)?;
                            new_entry_opt = new_iter.next();
                        },
                        Ordering::Equal => {
                            use DirectoryContents::{File, SubDirectory};
                            match (new_entry, old_entry) {
                                (File(new_file), File(old_file)) => {
                                    if old_file.size != new_file.size || &old_file.checksum() != &new_file.checksum() {
                                        diff.add_modified_file(new_file, parent_path);
                                    }
                                },
                                (File(new_file), SubDirectory(old_dir)) => {
                                    diff.add_created_file(new_file, parent_path);
                                    diff.add_deleted_files(old_entry, parent_path)?;
                                },
                                (SubDirectory(new_dir), File(old_file)) => {
                                    diff.add_created_files(new_entry, parent_path)?;
                                    diff.add_deleted_file(old_file, parent_path);
                                },
                                (SubDirectory(new_dir), SubDirectory(old_dir)) => {
                                    parent_path.borrow_mut().push(new_dir.label.clone());
                                    Diff::collect_diff(&**old_dir, &**new_dir, parent_path, diff)?;
                                    parent_path.borrow_mut().pop();
                                },
                                _ => panic!("should not happen unless the algo is incorrect")
                            }
                            old_entry_opt = old_iter.next();
                            new_entry_opt = new_iter.next();
                        }
                    }
                },
                (Some(old_entry), None) => {
                    diff.add_deleted_files(old_entry, parent_path)?;
                    old_entry_opt = old_iter.next();
                },
                (None, Some(new_entry)) => {
                    diff.add_created_files(new_entry, parent_path)?;
                    new_entry_opt = new_iter.next();
                },
                (None, None) => break
            }
        }
        Ok(())
    }

    // if entry is a file, then return this file,
    // or a list of files in the directory tree otherwise
    fn collect_files_from_entry<F, T>(entry: &DirectoryContents, parent_path: &Rc<RefCell<Vec<Label>>>, mapper: F)
        -> Result<Vec<T>, String>
        where F: Fn(&File, Vec<Label>) -> Result<T, String> + Copy {

        match entry {
            DirectoryContents::SubDirectory(dir) => Diff::collect_files(dir, parent_path, mapper),
            DirectoryContents::File(file) => {
                parent_path.borrow_mut().push(file.filename.clone());
                let mapped = mapper(file, parent_path.borrow().to_vec())?;
                parent_path.borrow_mut().pop();
                Ok(vec![mapped])
            },
            _ => Err(String::from("Unexpected entry type"))
        }
    }

    fn collect_files<F, T>(dir: &Directory, parent_path: &Rc<RefCell<Vec<Label>>>, mapper: F)
        -> Result<Vec<T>, String>
        where F: Fn(&File, Vec<Label>) -> Result<T, String> + Copy {

        let mut files: Vec<T> = Vec::new();
        Diff::collect_files_inner(dir, parent_path, mapper, &mut files)?;
        Ok(files)
    }

    fn collect_files_inner<'a, F, T>(dir: &'a Directory, parent_path: &Rc<RefCell<Vec<Label>>>, mapper: F, files: &mut Vec<T>)
        -> Result<(), String>
        where F: Fn(&File, Vec<Label>) -> Result<T, String> + Copy {

        parent_path.borrow_mut().push(dir.label.clone());
        for entry in dir.entries.iter() {
            match entry {
                DirectoryContents::SubDirectory(subdir) => {
                    parent_path.borrow_mut().push(subdir.label.clone());
                    Diff::collect_files_inner(&**subdir, parent_path, mapper, files)?;
                    parent_path.borrow_mut().pop();
                },
                DirectoryContents::File(file) => {
                    let mut path = parent_path.borrow().clone();
                    path.push(file.filename.clone());
                    files.push(mapper(file, path)?);
                },
                DirectoryContents::Repo => { /* Skip repo directory */ }
            }
        }
        parent_path.borrow_mut().pop();
        Ok(())
    }

    fn convert_to_deleted(file: &File, path: Vec<Label>) -> Result<DeleteFile, String> {
        Ok(DeleteFile {
            path: NonEmpty::from_slice(&path).ok_or("Empty path")?
        })
    }

    fn convert_to_created(file: &File, path: Vec<Label>) -> Result<CreateFile, String> {
        Ok(CreateFile {
            path: NonEmpty::from_slice(&path).ok_or("Empty path")?
        })
    }

    fn add_modified_file(&mut self, file: &File, parent_path: &Rc<RefCell<Vec<Label>>>) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        self.modified.push(ModifiedFile {
            path: Diff::build_non_empty_path(file, parent_path),
            diff: FileDiff {}
        });
    }

    fn add_created_file(&mut self, file: &File, parent_path: &Rc<RefCell<Vec<Label>>>) {
        self.created.push(CreateFile {
            path: Diff::build_non_empty_path(file, parent_path)
        });
    }

    fn add_created_files(&mut self, dc: &DirectoryContents, parent_path: &Rc<RefCell<Vec<Label>>>)
        -> Result<(), String> {

        let mut new_files: Vec<CreateFile> = Diff::collect_files_from_entry(
            dc, &parent_path, Diff::convert_to_created)?;
        self.created.append(&mut new_files);
        Ok(())
    }

    fn add_deleted_file(&mut self, file: &File, parent_path: &Rc<RefCell<Vec<Label>>>) {
        self.deleted.push(DeleteFile {
            path: Diff::build_non_empty_path(file, parent_path)
        });
    }

    fn add_deleted_files(&mut self, dc: &DirectoryContents, parent_path: &Rc<RefCell<Vec<Label>>>)
        -> Result<(), String> {

        let mut new_files: Vec<DeleteFile> = Diff::collect_files_from_entry(
            dc, &parent_path, Diff::convert_to_deleted)?;
        self.deleted.append(&mut new_files);
        Ok(())
    }

    fn build_non_empty_path(file: &File, parent_path: &Rc<RefCell<Vec<Label>>>) -> NonEmpty<Label> {
        let mut path = parent_path.borrow().to_vec();
        path.push(file.filename.clone());
        // path is always non-empty, so we can use unwrap()
        NonEmpty::from_slice(&path).unwrap()
    }
}

// returns list of contents, sorted by label; Repos are prepended to the beginning
fn get_sorted_contents(dir: &Directory) -> Vec<&DirectoryContents> {
    let mut vec: Vec<&DirectoryContents> = dir.entries.iter().collect();
    vec.sort_by_key(|e| {
        match e {
            DirectoryContents::SubDirectory(subdir) => Some(subdir.label.clone()),
            DirectoryContents::File(file) => Some(file.filename.clone()),
            DirectoryContents::Repo => None,
        }
    });
    vec
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {

}
