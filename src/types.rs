

pub mod repo {
    use super::commit_history;
    use super::commit;
    use crate::traits::{RepoI, CommitHistoryI, CommitI};

    pub struct Repo {
        histories: Vec<commit_history::CommitHistory>,
    }

    impl RepoI for Repo {
        type CommitHistory = commit_history::CommitHistory;
        type Commit = commit::Commit;

        fn new() -> Repo {
            Repo { histories: Vec::new() }
        }

        fn add_commit_history(&mut self, history: Self::CommitHistory) {
            self.histories.push(history);
        }

        fn get_commit_histories(&self) -> Vec<Self::CommitHistory> {
            self.histories.clone()
        }

        fn get_commit(&self, hash: String)
            -> Option<Self::Commit> {
              self
                .get_commit_histories()
                .into_iter()
                .map(|history| {
                    // Find the first commit that is matched in this history
                    history.get_commits().iter()
                           .find_map(|commit| commit.match_hash(hash.clone()))
                })
                .find_map(|commit| commit)
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

    impl IntoIterator for CommitHistory {
        type Item = commit::Commit;
        type IntoIter = std::vec::IntoIter<commit::Commit>;

        fn into_iter(self) -> std::vec::IntoIter<commit::Commit> {
            self.commits.into_iter()
        }
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

        fn get_up_to_commit(&self, search_commit: Self::Commit) -> Vec<Self::Commit> {
            let mut result = Vec::new();

            // Drop commits at the head of the history since Vec is push oriented
            for commit in self.commits.iter() {
                let current_commit = commit.clone();
                if current_commit != search_commit {
                    result.push(current_commit)
                } else {
                    continue;
                }
            }
            result
        }
    }
}

pub mod commit {
    use chrono::prelude::{DateTime, Utc,};
    use super::file;
    use super::repo;
    use crate::traits::{CommitI, RepoI, CommitHistoryI};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Commit {
        pub author: String,
        pub hash: String,
        pub date: DateTime<Utc>,
        pub message: String,
        signature: Option<String>,
        pub parent_commits: Vec<Commit>,
        pub changes: Vec<Change>,
    }

    impl CommitI for Commit {
        type Repo = repo::Repo;
        type Change = Change;
        type Signature = String;

        fn author(&self) -> String { self.author.clone() }

        fn parents(&self) -> Vec<Commit> { self.parent_commits.clone() }

        fn children(&self, repo: Self::Repo) -> Vec<Self>
        {
            repo.get_commit_histories().iter().flat_map(|history| {
                let commits = history.get_commits().into_iter();
                commits.take_while(|commit| commit != self)
            }).collect()
        }

        fn match_hash(&self, hash: String) -> Option<Self> {
            if self.hash == hash { Some(self.clone()) } else { None }
        }

        fn get_changes(&self) -> Vec<Self::Change> {
            self.changes.clone()
        }

        fn sign_commit(&mut self, key: Self::Signature) {
            self.signature = Some(key);
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Change {
        Addition(file::FileName, file::Location, file::FileContents),
        Removal(file::FileName, file::Location),
        Move(file::FileName, file::FileName),
        Create(file::FileName),
        Delete(file::FileName),
    }

    impl Change {
        pub fn get_filename(&self) -> file::FileName
        {
            match self {
                Change::Addition(filename, _, _)  => filename,
                Change::Removal(filename, _)      => filename,
                Change::Move(filename, filename_) => filename,
                Change::Create(filename)          => filename,
                Change::Delete(filename)          => filename,
            }.clone()
        }
    }
}


pub mod file {
    use super::commit;
    use super::commit_history;
    use super::directory;
    use crate::traits::{FileI, CommitI};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Location {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct File {
        name: FileName,
        commits: Vec<commit::Commit>,
        contents: FileContents,
    }

    impl FileI for File {
        type FileContents = FileContents;
        type FileName = FileName;
        type Directory = directory::Directory;
        type CommitHistory = commit_history::CommitHistory;
        type Commit = commit::Commit;

        fn history(&self) -> Vec<Self::Commit> {
            self.commits.clone()
        }

        fn directory(&self) -> Self::Directory {
            self.name.directory.clone()
        }

        fn get_contents(
            &self,
            commits: Vec<Self::Commit>
            ) -> Self::FileContents
        {
            let mut file_contents = FileContents::empty_file_contents();
            for commit in commits {
                let changes = commit.get_changes();
                changes.into_iter().filter(|change| {
                    change.get_filename() == self.name
                }).for_each(|file_change| {
                    file_contents.apply_file_change(file_change)
                })
            }
            file_contents
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FileContents {
        contents: String
    }

    impl FileContents {
        fn empty_file_contents() -> FileContents {
            FileContents { contents: String::from("") }
        }

        fn apply_file_change(&mut self, change: <commit::Commit as CommitI>::Change)
        {
            match change {
                commit::Change::Addition(filename, _, _)  =>
                    panic!("Unimplemented!"),
                commit::Change::Removal(filename, _)      =>
                    panic!("Unimplemented!"),
                commit::Change::Move(filename, filename_) =>
                    panic!("Unimplemented!"),
                commit::Change::Create(filename)          =>
                    panic!("Unimplemented!"),
                commit::Change::Delete(filename)          =>
                    panic!("Unimplemented!"),
            };
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FileName {
        directory: directory::Directory,
        name: String,
    }
}

pub mod directory {
    use crate::traits::{DirectoryI};

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Directory {
        path: Vec<String>,
    }

    impl DirectoryI for Directory {
        fn is_prefix_of(&self, directory: &Directory) -> bool {
            directory.path.starts_with(&self.path)
        }

    }
}
