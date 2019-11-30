#![allow(dead_code, unused_variables)]

use crate::file_system::{Directory, Label};
use nonempty::NonEmpty;
use std::collections::BTreeSet;
use crate::file_system::{DirectoryContents, File};
use std::cmp::Ordering;

type Result = std::result::Result<Diff, DiffError>;

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
    fn diff<Repo>(left: Directory, right: Directory) -> Result {
        let files_left = Diff::collect_files(&left)?;
        let files_right = Diff::collect_files(&right)?;

        let mut diff = Diff::new();

        for file in files_right.symmetric_difference(&files_left) {
            if files_left.contains(file) {
                diff.deleted.push(DeleteFile { path: NonEmpty::new(file.0.filename.clone()) })
            } else {
                diff.created.push(CreateFile { path: NonEmpty::new(file.0.filename.clone()) })
            }
        }

        // TODO: Verify that actual values in the intersection are from self!
        // Until then, explicitly get file_old and file_new from both sets.
        for file in files_right.intersection(&files_left) {
            // TODO: Use pijul's transaction diff as an inspiration?
            // https://nest.pijul.com/pijul_org/pijul:master/1468b7281a6f3785e9#anesp4Qdq3V
            let file_old = files_left.get(file).unwrap(); // can't happen
            let file_new = files_right.get(file).unwrap(); // can't happen
            if file_new.0.size != file_old.0.size {
                diff.modified.push(ModifiedFile {
                    path: NonEmpty::new(file_new.0.filename.clone()),
                    diff: FileDiff {}
                });
            } else {
                // TODO
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
        Diff::collect_files_inner(dir, &mut files)?;
        Ok(files)
    }

    fn collect_files_inner<'a>(dir: &'a Directory, files: &mut BTreeSet<ComparableFile<'a>>)
        -> std::result::Result<(), String> {

        for entry in dir.entries.iter() {
            match entry {
                DirectoryContents::SubDirectory(subdir) => Diff::collect_files_inner(&**subdir, files)?,
                DirectoryContents::File(file) => {
                    if !files.insert(ComparableFile(&file)) {
                        return Err(format!("Duplicate filename: {:?}", file.filename))
                    }
                },
                DirectoryContents::Repo => return Err(String::from("Can't diff a repo!"))
            }
        }
        Ok(())
    }
}

#[derive(Eq)]
struct ComparableFile<'a>(&'a File);

impl Ord for ComparableFile<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.filename.cmp(&other.0.filename)
    }
}

impl PartialOrd for ComparableFile<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.filename.partial_cmp(&other.0.filename)
    }
}

impl PartialEq for ComparableFile<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.filename.eq(&other.0.filename)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_diff() {

    }
}
