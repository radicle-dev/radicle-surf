#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, Label};
use nonempty::NonEmpty;
use crate::file_system::{DirectoryContents, File};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::cell::RefCell;
use std::rc::Rc;

type Result = std::result::Result<Diff, DiffError>;

#[derive(Debug)]
struct DiffError {
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
    path: NonEmpty<Label>,
}

pub struct DeleteFile {
    path: NonEmpty<Label>,
}

pub struct MoveFile {
    old_path: NonEmpty<Label>,
    new_path: NonEmpty<Label>,
}

pub struct ModifiedFile {
    path: NonEmpty<Label>,
    diff: FileDiff,
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
    fn diff(left: Directory, right: Directory) -> Result {
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
        -> std::result::Result<(), String> {

        // TODO: Consider storing directory contents in sorted order
        let old = Diff::get_sorted_contents(old);
        let new = Diff::get_sorted_contents(new);

        let mut old_iter = old.iter();
        let mut new_iter = new.iter();
        let mut old_entry = old_iter.next();
        let mut new_entry = new_iter.next();
        while old_entry.is_some() || new_entry.is_some() {
            // need to skip Repos
            while let Some(DirectoryContents::Repo) = old_entry {
                old_entry = old_iter.next();
            }
            while let Some(DirectoryContents::Repo) = new_entry {
                new_entry = new_iter.next();
            }
            if new_entry.is_none() && old_entry.is_none() {
                break;
            }
            let old_entry_label = old_entry.and_then(|dc| Diff::get_label(dc));
            let new_entry_label = new_entry.and_then(|dc| Diff::get_label(dc));
            let cmp = {
                if old_entry.is_none() || new_entry.is_none() {
                    Ordering::Equal
                } else {
                    new_entry_label.unwrap().cmp(&old_entry_label.unwrap())
                }
            };
            if new_entry.is_none() || cmp == Ordering::Greater {
                let mut old_files: Vec<DeleteFile> = Diff::collect_files_from_entry(
                    old_entry.unwrap(), &parent_path, Diff::convert_to_deleted)?;
                diff.deleted.append(&mut old_files);
                old_entry = old_iter.next();
            } else if old_entry.is_none() || cmp == Ordering::Less {
                let mut new_files: Vec<CreateFile> = Diff::collect_files_from_entry(
                    new_entry.unwrap(), &parent_path, Diff::convert_to_created)?;
                diff.created.append(&mut new_files);
                new_entry = new_iter.next();
            } else /*both entries are present and cmp == Ordering::Equal*/ {
                match (new_entry.unwrap(), old_entry.unwrap()) {
                    (DirectoryContents::File(new_file), DirectoryContents::File(old_file)) => {
                        if old_file.size != new_file.size || Diff::checksumf(&old_file) != Diff::checksumf(&new_file) {
                            Diff::add_modified_file(new_file, parent_path, diff);
                        }
                    },
                    (DirectoryContents::File(new_file), DirectoryContents::SubDirectory(old_dir)) => {
                        Diff::add_created_file(new_file, parent_path, diff);
                        let mut old_files: Vec<DeleteFile> = Diff::collect_files_from_entry(
                            old_entry.unwrap(), &parent_path, Diff::convert_to_deleted)?;
                        diff.deleted.append(&mut old_files);
                    },
                    (DirectoryContents::SubDirectory(new_dir), DirectoryContents::File(old_file)) => {
                        let mut new_files: Vec<CreateFile> = Diff::collect_files_from_entry(
                            new_entry.unwrap(), &parent_path, Diff::convert_to_created)?;
                        diff.created.append(&mut new_files);
                        Diff::add_deleted_file(old_file, parent_path, diff);
                    },
                    (DirectoryContents::SubDirectory(new_dir), DirectoryContents::SubDirectory(old_dir)) => {
                        parent_path.borrow_mut().push(new_dir.label.clone());
                        Diff::collect_diff(&**old_dir, &**new_dir, parent_path, diff)?;
                        parent_path.borrow_mut().pop();
                    },
                    _ => panic!("should not happen unless the algo is incorrect")
                }
                old_entry = old_iter.next();
                new_entry = new_iter.next();
            }
        }
        Ok(())
    }

    // returns list of contents, sorted by label; Repos are prepended to the beginning
    fn get_sorted_contents(dir: &Directory) -> Vec<&DirectoryContents> {
        let mut vec: Vec<&DirectoryContents> = dir.entries.iter().collect();
        vec.sort_by_key(|e| {
            match e {
                DirectoryContents::SubDirectory(subdir) => subdir.label.clone(),
                DirectoryContents::File(file) => file.filename.clone(),
                DirectoryContents::Repo => Label::from(""),
            }
        });
        vec
    }

    fn get_label(dc: &DirectoryContents) -> Option<&Label> {
        match dc {
            DirectoryContents::SubDirectory(dir) => Some(&dir.label),
            DirectoryContents::File(file) => Some(&file.filename),
            _ => None
        }
    }

    fn checksumf(file: &File) -> u64 {
        let mut hasher = DefaultHasher::new();
        file.contents.hash(&mut hasher);
        hasher.finish()
    }

    // if entry is a file, then return this file,
    // or a list of files in the directory tree otherwise
    fn collect_files_from_entry<F, T>(entry: &DirectoryContents, parent_path: &Rc<RefCell<Vec<Label>>>, mapper: F)
        -> std::result::Result<Vec<T>, String>
        where F: Fn(&File, Vec<Label>) -> std::result::Result<T, String> + Copy {

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
        -> std::result::Result<Vec<T>, String>
        where F: Fn(&File, Vec<Label>) -> std::result::Result<T, String> + Copy {

        let mut files: Vec<T> = Vec::new();
        Diff::collect_files_inner(dir, parent_path, mapper, &mut files)?;
        Ok(files)
    }

    fn collect_files_inner<'a, F, T>(dir: &'a Directory, parent_path: &Rc<RefCell<Vec<Label>>>, mapper: F, files: &mut Vec<T>)
        -> std::result::Result<(), String>
        where F: Fn(&File, Vec<Label>) -> std::result::Result<T, String> + Copy {

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

    fn convert_to_deleted(file: &File, path: Vec<Label>) -> std::result::Result<DeleteFile, String> {
        Ok(DeleteFile {
            path: into_non_empty(&path)?
        })
    }

    fn convert_to_created(file: &File, path: Vec<Label>) -> std::result::Result<CreateFile, String> {
        Ok(CreateFile {
            path: into_non_empty(&path)?
        })
    }

    fn add_modified_file(file: &File, parent_path: &Rc<RefCell<Vec<Label>>>, diff: &mut Diff) {
        // TODO: file diff can be calculated at this point
        // Use pijul's transaction diff as an inspiration?
        // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
        diff.modified.push(ModifiedFile {
            path: Diff::build_non_empty_path(file, parent_path),
            diff: FileDiff {}
        });
    }

    fn add_created_file(file: &File, parent_path: &Rc<RefCell<Vec<Label>>>, diff: &mut Diff) {
        diff.created.push(CreateFile {
            path: Diff::build_non_empty_path(file, parent_path)
        });
    }

    fn add_deleted_file(file: &File, parent_path: &Rc<RefCell<Vec<Label>>>, diff: &mut Diff) {
        diff.deleted.push(DeleteFile {
            path: Diff::build_non_empty_path(file, parent_path)
        });
    }

    fn build_non_empty_path(file: &File, parent_path: &Rc<RefCell<Vec<Label>>>) -> NonEmpty<Label> {
        let mut path = parent_path.borrow().to_vec();
        path.push(file.filename.clone());
        // path is always non-empty, so we can use unwrap()
        into_non_empty(&path).unwrap()
    }
}

// TODO: Move somewhere?
fn into_non_empty<T>(vec: &Vec<T>) -> std::result::Result<NonEmpty<T>, String> where T: Clone {
    if vec.is_empty() {
        return Err(String::from("Empty vec"))
    }
    Ok(NonEmpty::from((vec[0].clone(), vec[1..].to_vec())))
}

#[derive(Eq)]
struct ComparableFile<'a> {
    file: &'a File,
    path: Vec<Label>
}

impl Ord for ComparableFile<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for ComparableFile<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.path.partial_cmp(&other.path)
    }
}

impl PartialEq for ComparableFile<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use crate::vcs::git::{GitBrowser, GitRepository};
    use pretty_assertions::assert_eq;
    use git2::Oid;
    use nonempty::NonEmpty;
    use crate::vcs::History;
    use crate::diff::{Diff, DiffError};
    use crate::file_system::Directory;

    // run `cargo test -- --nocapture` to see output
    #[test]
    fn test_diff() -> std::result::Result<(), DiffError> {
        let repo = GitRepository::new(".").unwrap();
        let mut browser = GitBrowser::new(&repo).unwrap();
        browser.head().unwrap();
        let head_directory = browser.get_directory().unwrap();
        let old_commit_id = "e88d1d8e34212f2dfa9d34d2d2005932fd84cb06"; // one of the old commits
        let old_commit = browser.get_history()
            .find_in_history(&Oid::from_str(old_commit_id).unwrap(), |artifact| artifact.id()).unwrap();
        browser.set_history(History(NonEmpty::new(old_commit)));
        let old_directory = browser.get_directory().unwrap();

        let diff = Diff::diff(old_directory, head_directory)?;
        print_diff_summary(&diff);
        Ok(())
    }

    fn print_diff_summary(diff: &Diff) {
        diff.created.iter().for_each(|created| { println!("+++ {:?}", created.path); });
        diff.deleted.iter().for_each(|deleted| { println!("--- {:?}", deleted.path); });
        diff.modified.iter().for_each(|modified| { println!("mod {:?}", modified.path); });
    }
}
