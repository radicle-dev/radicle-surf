#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, Label};
use nonempty::NonEmpty;
use std::collections::BTreeSet;
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
        let files_left = Diff::collect_files(&left)?;
        let files_right = Diff::collect_files(&right)?;

        let mut diff = Diff::new();

        for file in files_right.symmetric_difference(&files_left) {
            if files_left.contains(file) {
                diff.deleted.push(DeleteFile { path: into_non_empty(&file.path)? })
            } else {
                diff.created.push(CreateFile { path: into_non_empty(&file.path)? })
            }
        }

        // TODO: Verify that actual values in the intersection are from self!
        // Until then, explicitly get file_old and file_new from both sets.
        for file in files_right.intersection(&files_left) {
            // TODO: Use pijul's transaction diff as an inspiration?
            // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
            let file_old = files_left.get(file).unwrap(); // can't happen
            let file_new = files_right.get(file).unwrap(); // can't happen
            if (file_new.file.size != file_old.file.size) || (Diff::checksum(file_new) != Diff::checksum(file_old)) {
                diff.modified.push(ModifiedFile {
                    path: into_non_empty(&file_new.path)?,
                    diff: FileDiff {}
                });
            }
        }

        // TODO: Some of the deleted files may actually be moved (renamed) to one of the created files.
        // Finding out which of the deleted files were deleted and which were moved will probably require
        // performing some variant of the longest common substring algorithm for each pair in D x C.
        // Final decision can be based on heuristics, e.g. the file can be considered moved,
        // if len(LCS) > 0,25 * min(size(d), size(c)), and deleted otherwise.

        Ok(diff)
    }

    fn collect_files<'a>(dir: &'a Directory) -> std::result::Result<BTreeSet<ComparableFile<'a>>, String> {
        let mut files = BTreeSet::new();
        let parent_path = Rc::new(RefCell::new(Vec::new()));
        Diff::collect_files_inner(dir, &parent_path, &mut files)?;
        Ok(files)
    }

    fn collect_files_inner<'a>(dir: &'a Directory, parent_path: &Rc<RefCell<Vec<Label>>>, files: &mut BTreeSet<ComparableFile<'a>>)
        -> std::result::Result<(), String> {

        parent_path.borrow_mut().push(dir.label.clone());
        for entry in dir.entries.iter() {
            match entry {
                DirectoryContents::SubDirectory(subdir) => {
                    parent_path.borrow_mut().push(subdir.label.clone());
                    Diff::collect_files_inner(&**subdir, parent_path, files)?;
                    parent_path.borrow_mut().pop();
                },
                DirectoryContents::File(file) => {
                    let mut path = parent_path.borrow().clone();
                    path.push(file.filename.clone());
                    if !files.insert(ComparableFile { file: &file, path }) {
                        return Err(format!("Duplicate filename: {:?}", file.filename))
                    }
                },
                DirectoryContents::Repo => { /* Skip repo directory */ }
            }
        }
        parent_path.borrow_mut().pop();
        Ok(())
    }

    fn checksum(file: &ComparableFile) -> u64 {
        let mut hasher = DefaultHasher::new();
        file.file.contents.hash(&mut hasher);
        hasher.finish()
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
    use crate::diff::Diff;

    // run `cargo test -- --nocapture` to see output
    #[test]
    fn test_diff() {
        let repo = GitRepository::new(".").unwrap();
        let mut browser = GitBrowser::new(&repo).unwrap();
        browser.head().unwrap();
        let head_directory = browser.get_directory().unwrap();
        let old_commit_id = "e88d1d8e34212f2dfa9d34d2d2005932fd84cb06"; // one of the old commits
        let old_commit = browser.get_history()
            .find_in_history(&Oid::from_str(old_commit_id).unwrap(), |artifact| artifact.id()).unwrap();
        browser.set_history(History(NonEmpty::new(old_commit)));
        let old_directory = browser.get_directory().unwrap();

        let diff = Diff::diff(old_directory, head_directory).unwrap();
        print_diff_summary(&diff);
    }

    fn print_diff_summary(diff: &Diff) {
        diff.created.iter().for_each(|created| { println!("+++ {:?}", created.path); });
        diff.deleted.iter().for_each(|deleted| { println!("--- {:?}", deleted.path); });
        diff.modified.iter().for_each(|modified| { println!("mod {:?}", modified.path); });
    }
}
