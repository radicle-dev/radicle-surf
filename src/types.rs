
pub mod repo {
    use crate::traits::{RepoI, CommitHistoryI};
    use super::commit_history;

    pub struct Repo {
        histories: Vec<commit_history::CommitHistory>,
    }

    impl RepoI for Repo {
        type History = commit_history::CommitHistory;

        fn new() -> Repo {
            Repo { histories: Vec::new() }
        }

        fn add_commit_history(&mut self, history: Self::History) {
            self.histories.push(history);
        }

        fn get_commit_histories(&self) -> Vec<Self::History> {
            self.histories.clone()
        }

        fn get_commit(&self, hash: String)
            -> Option<<<Repo as RepoI>::History as CommitHistoryI>::Commit> {
            panic!("Unimplemented!");
        }
    }
}

pub mod commit_history {
    use super::commit;
    use crate::traits::CommitHistoryI;

    #[derive(Debug, Clone)]
    pub struct CommitHistory {
        commits: Vec<commit::Commit>,
    }

    impl CommitHistoryI for CommitHistory {
        type Commit = commit::Commit;

        fn new() -> CommitHistory {
            CommitHistory { commits: Vec::new() }
        }

        fn add_commit(&mut self, commit: Self::Commit) {
            self.commits.push(commit);
        }

        fn get_commits(&self) -> Vec<Self::Commit> {
            self.commits.clone()
        }

        fn get_up_to_commit(&self, commit: Self::Commit) -> Vec<Self::Commit> {
            panic!("Unimplemented!");
        }
    }

    pub struct Branch {
        name: String,
        commit_history: CommitHistory,
    }

    pub struct Tag {
        name: String,
        commit_history: CommitHistory,
    }
}

pub mod commit {
    use chrono::prelude::{DateTime, Utc,};
    use super::commit_history;
    use super::file;
    use super::repo;
    use crate::traits::CommitI;

    #[derive(Debug, Clone)]
    pub struct Commit {
        author: String,
        hash: String,
        date: DateTime<Utc>,
        message: String,
        signature: Option<String>,
        parent_commits: Vec<Commit>,
        changes: Vec<Change>,
    }

    impl CommitI for Commit {
        type Repo = repo::Repo;
        type History = commit_history::CommitHistory;
        type Change = Change;
        type Signature = String;

        fn parents(&self) -> Vec<Commit> { self.parent_commits.clone() }

        fn children(&self, repo: Self::Repo) -> Vec<Self> { panic!("Unimplemented!"); }

        fn find_author_commits(commits: Vec<Commit>, author: String) -> Vec<Commit> {
            commits.into_iter().filter(|commit| commit.author == author).collect()
        }

        fn find_commit_by_hash(commits: Vec<Commit>, hash: String) -> Option<Self> {
            commits.into_iter().find(|commit| commit.hash == hash)
        }

        fn get_changes(&self) -> Vec<Self::Change> {
            self.changes.clone()
        }

        fn sign_commit(&mut self, key: Self::Signature) {
            self.signature = Some(key);
        }
    }

    #[derive(Debug, Clone)]
    pub enum Change {
        Addition(file::FileName, file::Location, file::FileContents),
        Removal(file::FileName, file::Location),
        Move(file::FileName, file::FileName),
        Create(file::FileName),
        Delete(file::FileName),
    }
}


pub mod file {
    use super::commit;
    use super::directory;
    use crate::traits::FileI;

    #[derive(Debug, Clone)]
    pub struct Location {}

    pub struct File {
        name: FileName,
        commits: Vec<commit::Commit>,
    }

    impl FileI for File {
        type FileContents = FileContents;
        type FileName = FileName;
        type Commit = commit::Commit;
        type Directory = directory::Directory;

        fn history(&self) -> Vec<Self::Commit> {
            self.commits.clone()
        }

        fn directory(&self) -> Self::Directory {
            self.name.directory.clone()
        }

        fn get_files(commits: Vec<Self::Commit>) -> Vec<Self> {
            panic!("Unimplemented!")
        }

        fn directory_view(directory: Self::Directory, files: Vec<Self>)
            -> Vec<Self> {
            panic!("Unimplemented!")
        }

        fn get_contents(file_name: Self::FileName, commits: Vec<Self::Commit>)
            -> Self::FileContents {
            panic!("Unimplemented!")
        }
    }

    #[derive(Debug, Clone)]
    pub struct FileContents {
        contents: String
    }

    #[derive(Debug, Clone)]
    pub struct FileName {
        directory: directory::Directory,
        name: String,
    }
}

pub mod directory {
    use super::commit_history;
    use crate::traits::{DirectoryI, CommitHistoryI};

    #[derive(Debug, Clone)]
    pub struct Directory {
        path: Vec<String>,
    }

    impl DirectoryI for Directory {
        type History = commit_history::CommitHistory;

        // Let's be honest the return result is Self::History::Commit
        fn history(&self, history: Self::History)
            -> Vec<<<Self as DirectoryI>::History as CommitHistoryI>::Commit> {
            panic!("Unimplemented!");
        }

        fn get_directories(history: Self::History) -> Vec<Directory> {
            //traits::FileI::get_files(history.get_commits()).map(|file| file.directory)
            panic!("Unimplemented!");
        }

        fn is_prefix_of(&self, directory: &Directory) -> bool {
            directory.path.starts_with(&self.path)
        }

    }
}
