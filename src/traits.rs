
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

pub trait DirectoryI {
    type History: CommitHistoryI;

    // Let's be honest the return result is Self::History::Commit
    fn history(&self, history: Self::History)
        -> Vec<<<Self as DirectoryI>::History as CommitHistoryI>::Commit>;

    fn get_directories(history: Self::History) -> Vec<Self> where Self: Sized;

    fn is_prefix_of(&self, directory: &Self) -> bool;

}
