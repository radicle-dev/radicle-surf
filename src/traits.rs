use std::collections::HashMap;
use std::hash::Hash;

pub trait RepoI {
    type Commit: CommitI;
    type CommitHistory: CommitHistoryI<Commit = Self::Commit>;

    fn new() -> Self;

    fn add_commit_history(&mut self, history: Self::CommitHistory);

    fn get_commit_histories(&self) -> Vec<Self::CommitHistory>;

    fn get_commit(&self, hash: String) -> Option<Self::Commit>;
}

pub trait CommitHistoryI {
    type Commit: CommitI;

    fn new() -> Self;

    fn add_commit(&mut self, commit: Self::Commit);

    fn get_commits(&self) -> Vec<Self::Commit>;

    fn get_up_to_commit(&self, commit: Self::Commit) -> Vec<Self::Commit>;
}

pub trait CommitI where Self: Sized {
    type Repo: RepoI<Commit = Self>;
    type Change: ChangeI<Commit = Self>;
    type Signature;

    fn author(&self) -> String;

    fn parents(&self) -> Vec<Self>;

    fn children(&self, repo: Self::Repo) -> Vec<Self>;

    fn match_hash(&self, hash: String) -> Option<Self>;

    fn get_changes(&self) -> Vec<Self::Change>;

    fn sign_commit(&mut self, key: Self::Signature);
}

pub trait ChangeI {
    type FileName;
    type Commit;

    fn get_filename(&self) -> Self::FileName;

    fn add_change(
        &self,
        commit: Self::Commit,
        change_map: &mut HashMap<Self::FileName, Vec<Self::Commit>>,
    );
}

pub trait FileI {
    type FileContents;
    type FileName;
    type Directory: DirectoryI;
    type Change: ChangeI<FileName = Self::FileName, Commit = Self::Commit>;
    type Commit: CommitI<Change = Self::Change>;
    type CommitHistory: CommitHistoryI<Commit = Self::Commit>;

    fn history(&self) -> Vec<Self::Commit>;

    fn directory(&self) -> Self::Directory;

    fn build_contents(
        filename: Self::FileName,
        commits: Vec<Self::Commit>,
    ) -> Self::FileContents;

    fn to_file(filename: Self::FileName, commits: Vec<Self::Commit>) -> Self;
}

pub trait DirectoryI {
    fn is_prefix_of(&self, directory: &Self) -> bool;
}

pub fn get_files<File>(commits: Vec<File::Commit>) -> Vec<File>
    where File: FileI,
          File::FileName: Hash + Eq,
          File::Commit: Clone,
{
    let mut commit_map = HashMap::new();
    for commit in commits {
        for change in commit.get_changes() {
            change.add_change(commit.clone(), &mut commit_map)
        }
    }

    let mut files = Vec::new();
    for (filename, commits) in commit_map {
        files.push(File::to_file(filename, commits));
    }
    files
}

pub fn directory_commits<File>(history: File::CommitHistory, directory: File::Directory)
    -> Vec<File::Commit>
    where File: FileI,
          File::FileName: Hash + Eq,
          File::Commit: Clone,
{
    get_directory_view(directory, get_files(history.get_commits()))
        .into_iter()
        .flat_map(|file: File| file.history().into_iter())
        .collect()
}

pub fn directory_history<File>(history: File::CommitHistory) -> Vec<File::Directory>
    where File: FileI,
          File::Directory: Clone + Hash + Eq,
          File::FileName: Hash + Eq,
          File::Commit: Clone,
{
    // Used to get unique directories
    use itertools::Itertools;

    get_files(history.get_commits())
        .into_iter()
        .map(|file: File| file.directory())
        .unique()
        .collect()
}

pub fn get_directory_view<File: FileI>(directory: File::Directory, files: Vec<File>)
    -> Vec<File>
    where File: FileI + Sized,
{
    files.into_iter().filter(|file| directory.is_prefix_of(&file.directory())).collect()
}

pub fn find_author_commits<Commit>(commits: Vec<Commit>, author: String) -> Vec<Commit>
    where Commit: CommitI,
{
    commits.into_iter().filter(|commit| commit.author() == author).collect()
}

pub(crate) mod properties {
    use super::*;

    pub(crate) fn prop_new_repo_has_empty_history<Repo>() -> bool
        where Repo: RepoI,
              Repo::CommitHistory: CommitHistoryI + PartialEq,
    {
        let repo: Repo = RepoI::new();
        let commit_history: Vec<Repo::CommitHistory> = repo.get_commit_histories();
        commit_history == Vec::new()
    }

    pub(crate) fn prop_is_prefix_identity<Directory>(directory: Directory) -> bool
        where Directory: DirectoryI,
    {
        directory.is_prefix_of(&directory)
    }

    pub(crate) fn prop_no_commits_no_files<File>() -> bool
        where File: FileI + PartialEq,
              File::FileName: Hash + Eq + Clone,
              File::Commit: Clone,
    {
        get_files::<File>(Vec::new()) == Vec::new()
    }
}
