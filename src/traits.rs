use std::collections::BTreeMap;
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

    fn get_up_to_commit(&self, commit: &Self::Commit) -> Vec<Self::Commit>;
}

pub trait CommitI where Self: Sized {
    type Repo: RepoI<Commit = Self>;
    type Change: ChangeI<Commit = Self>;
    type Signature;

    fn author(&self) -> String;

    fn parents(&self) -> Vec<Self>;

    fn children(&self, repo: &Self::Repo) -> Vec<Self>;

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
        change_map: &mut BTreeMap<Self::FileName, Vec<Self::Commit>>,
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
        filename: &Self::FileName,
        commits: &[Self::Commit],
    ) -> Self::FileContents;

    fn to_file(filename: Self::FileName, commits: &[Self::Commit]) -> Self;
}

pub trait DirectoryI {
    fn is_prefix_of(&self, directory: &Self) -> bool;
}

pub fn get_files<File>(commits: &[File::Commit]) -> Vec<File>
    where File: FileI,
          File::FileName: Ord + Eq,
          File::Commit: Clone,
{
    let mut commit_map = BTreeMap::new();
    for commit in commits {
        for change in commit.get_changes() {
            change.add_change(commit.clone(), &mut commit_map)
        }
    }

    let mut files = Vec::new();
    for (filename, commits) in commit_map {
        files.push(File::to_file(filename, &commits));
    }
    files
}

pub fn directory_commits<File>(history: &File::CommitHistory, directory: &File::Directory)
    -> Vec<File::Commit>
    where File: FileI + Clone,
          File::FileName: Ord + Eq,
          File::Commit: Clone,
{
    get_directory_view(directory, &get_files(&history.get_commits()))
        .into_iter()
        .flat_map(|file: File| file.history().into_iter())
        .collect()
}

pub fn directory_history<File>(history: &File::CommitHistory) -> Vec<File::Directory>
    where File: FileI,
          File::Directory: Clone + Hash + Eq,
          File::FileName: Ord + Eq,
          File::Commit: Clone,
{
    // Used to get unique directories
    use itertools::Itertools;

    get_files(&history.get_commits())
        .into_iter()
        .map(|file: File| file.directory())
        .unique()
        .collect()
}

pub fn get_directory_view<File: FileI>(directory: &File::Directory, files: &[File])
    -> Vec<File>
    where File: FileI + Sized + Clone,
{
    files.iter().filter(|file| directory.is_prefix_of(&file.directory())).cloned().collect()
}

pub fn find_author_commits<Commit>(commits: &[Commit], author: String) -> Vec<Commit>
    where Commit: CommitI + Clone,
{
    commits.iter().filter(|commit| commit.author() == author).cloned().collect()
}

#[cfg(test)]
pub(crate) mod properties {
    use std::collections::HashSet;

    use super::*;

    /// A new repo should always contain an empty commit history.
    ///
    /// repo = ∅
    /// repo.get_commit_histories() ≡ ∅
    pub(crate) fn prop_new_repo_has_empty_history<Repo>() -> bool
        where Repo: RepoI,
              Repo::CommitHistory: CommitHistoryI + PartialEq,
    {
        let repo: Repo = RepoI::new();
        let commit_history: Vec<Repo::CommitHistory> = repo.get_commit_histories();
        commit_history == Vec::new()
    }

    /// A directory should always be a prefix of itself.
    ///
    /// ∀ directory. directory.is_prefix_of(directory)
    pub(crate) fn prop_is_prefix_identity<Directory>(directory: Directory) -> bool
        where Directory: DirectoryI,
    {
        directory.is_prefix_of(&directory)
    }

    /// Trying to create files with no commits should result in no files.
    ///
    /// get_files(∅) ≡ ∅
    pub(crate) fn prop_no_commits_no_files<File>() -> bool
        where File: FileI + PartialEq,
              File::FileName: Ord + Eq + Clone,
              File::Commit: Clone,
    {
        get_files::<File>(&Vec::new()) == Vec::new()
    }

    /// A file can be reconstructed from its own commit history.
    ///
    /// ∀ file. get_files(file.history()) ≡ [file]
    pub(crate) fn prop_file_is_its_history<File>(file: File) -> bool
        where File: FileI + PartialEq,
              File::FileName: Ord + Eq,
              File::Commit: Clone,
    {
        let files: Vec<File> = get_files(&file.history());
        files == vec![file]
    }

    /// ∀ file history. file.history() ≡ history.get_commits()
    /// ⇒ file ∈ get_files(history.get_commits())
    pub(crate) fn prop_file_must_exist_in_history<File, CommitHistory>(default_filename: File::FileName, history: CommitHistory) -> bool
        where File: FileI + PartialEq,
              File::FileName: Ord + Eq + Clone,
              File::Commit: Clone,
              CommitHistory: CommitHistoryI<Commit = File::Commit>,
    {
        let commits: Vec<File::Commit> = history.get_commits();

        // Get the changes for the commits
        let mut changes = commits.iter().flat_map(|commit| commit.get_changes().into_iter());
        // Pick a filename to use
        let filename = changes.next().map(|change| change.get_filename()).unwrap_or(default_filename);
        /*
        let file_commits = commits.iter()
            .filter(|commit| commit.get_changes()
                                   .iter()
                                   .map(|change| change.get_filename())
                                   .collect::<Vec<_>>().contains(&filename))
            .collect().iter();
        */

        let file = File::to_file(filename, &commits);
        let files: Vec<File> = get_files(&commits);

        if files.is_empty() {
            true // Our history is empty so there's no files in here
        } else {
            get_files(&commits).contains(&file)
        }
    }

    /// Constructing directories from the commit history is equivalent to constructing all
    /// the files in that history and getting the directories of those files.
    ///
    /// ∀ history.
    ///   directory_history(history) ≡ get_files(history.get_commits()).map(|file| file.directory)
    pub(crate) fn prop_files_match_directories<File>(history: File::CommitHistory) -> bool
        where File: FileI,
              File::FileName: Ord + Eq,
              File::Commit: Clone,
              File::Directory: Hash + Eq + Clone + PartialEq,
    {
        let directories: Vec<File::Directory> = directory_history::<File>(&history);
        let file_directories: Vec<File::Directory> = get_files(&history.get_commits()).iter().map(|file: &File| file.directory()).collect();
        directories == file_directories
    }

    /// ∀ repo.
    ///   commit := pick_commit(repo) where commit.parents() != null
    ///
    ///   commit.children() ⊂ commit.parents().children()
    /// ∧ commit ∈ commit.parents().children()
    pub(crate) fn prop_children_of_commit_are_subset_of_parents_children<Commit>(repo: Commit::Repo) -> bool
        where Commit: CommitI + Hash + Eq,
    {
        let commits: Vec<_> = repo.get_commit_histories().iter().flat_map(|history| history.get_commits().into_iter()).collect();

        // Trivial case where the repo is empty
        if commits.is_empty() {
            return true
        }

        // Pick a commit
        let commit = &commits[commits.len() / 2];
        let parents = commit.parents();

        // If we have no parents then everything is our children
        if parents.is_empty() {
            return true
        }

        let children_commits: HashSet<Commit> = commit.children(&repo).into_iter().collect();
        let parents_children: HashSet<Commit> = parents.into_iter().flat_map(|c| c.children(&repo).into_iter()).collect();
        parents_children.contains(&commit) && children_commits.is_subset(&parents_children)
    }
}
