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
    type Change;
    type Signature;

    fn author(&self) -> String;

    fn parents(&self) -> Vec<Self>;

    fn children(&self, repo: Self::Repo) -> Vec<Self>;

    fn match_hash(&self, hash: String) -> Option<Self>;

    fn get_changes(&self) -> Vec<Self::Change>;

    fn sign_commit(&mut self, key: Self::Signature);
}

pub trait FileI {
    type FileContents;
    type FileName;
    type Directory: DirectoryI;
    type Commit: CommitI;
    type CommitHistory: CommitHistoryI<Commit = Self::Commit>;

    fn history(&self) -> Vec<Self::Commit>;

    fn directory(&self) -> Self::Directory;

    fn get_contents(&self, commits: Vec<Self::Commit>) -> Self::FileContents;
}

pub trait DirectoryI {
    fn is_prefix_of(&self, directory: &Self) -> bool;
}

pub fn get_files<File>(commits: Vec<File::Commit>) -> Vec<File>
    where File: FileI,
{
    panic!("Unimplemented!")
}

pub fn insert_change_map<File>(
    commit: File::Commit,
    change: <File::Commit as CommitI>::Change,
    change_map: HashMap<File::FileName, Vec<File::Commit>>
    ) -> HashMap<File::FileName, Vec<File::Commit>>
    where File: FileI
{
    /* TODO(fintan): What do we do here?
    match change {
        Change::Addition(_, _, _, _) => panic!("Unimplemented!"),
        Change::Removal(_, _,  _) => panic!("Unimplemented!"),
        Change::Move(_, _) => panic!("Unimplemented!"),
        Change::Create(_) => panic!("Unimplemented!"),
        Change::Delete(_) => panic!("Unimplemented!"),
    }
    */
    panic!("Unimplemented!")
}

pub fn directory_commits<File>(history: File::CommitHistory, directory: File::Directory)
    -> Vec<File::Commit>
    where File: FileI,
{
    get_directory_view(directory, get_files(history.get_commits()))
        .into_iter()
        .flat_map(|file: File| file.history().into_iter())
        .collect()
}

pub fn directory_history<File>(history: File::CommitHistory) -> Vec<File::Directory>
    where File: FileI,
          File::Directory: Clone + Eq + Hash
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
