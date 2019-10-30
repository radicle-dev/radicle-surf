
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

    #[cfg(test)]
    pub mod repo_tests {
        use super::Repo;
        use crate::traits::properties;

        quickcheck! {
          fn prop_new_repo_has_empty_history() -> bool {
              properties::prop_new_repo_has_empty_history::<Repo>()
          }
        }
    }
}

pub mod commit_history {
    use super::commit;
    use crate::traits::CommitHistoryI;

    #[derive(Debug, Clone, PartialEq)]
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
    use std::collections::HashMap;
    use std::hash::Hash;

    use super::file;
    use super::repo;
    use crate::traits::{CommitI, ChangeI, RepoI, CommitHistoryI};

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
        Addition(file::FileName, file::FileContents),
        Removal(file::FileName, file::FileContents),
        Move(file::FileName, file::FileName),
        Create(file::FileName),
        Delete(file::FileName),
    }

    impl ChangeI for Change {
        type FileName = file::FileName;
        type Commit = Commit;

        fn get_filename(&self) -> Self::FileName
            where Self::FileName: Hash,
        {
            match self {
                Change::Addition(filename, _) => filename,
                Change::Removal(filename, _)  => filename,
                Change::Move(_, new_filename) => new_filename,
                Change::Create(filename)      => filename,
                Change::Delete(filename)      => filename,
            }.clone()
        }

        fn add_change(
            &self,
            commit: Self::Commit,
            change_map: &mut HashMap<Self::FileName, Vec<Self::Commit>>
        ) {
            let singleton = vec![commit.clone()];
            match self {
                Change::Addition(filename, _) => {
                    change_map.entry(filename.clone())
                        .and_modify(|commits| commits.push(commit))
                        .or_insert(singleton);
                },
                Change::Removal(filename, _) => {
                    change_map.entry(filename.clone())
                        .and_modify(|commits| commits.push(commit))
                        .or_insert(singleton);
                }
                Change::Move(filename, new_filename) => {
                    if let Some((_, commits)) = change_map.remove_entry(filename) {
                        let mut new_commits = commits;
                        new_commits.push(commit);
                        change_map.insert(new_filename.clone(), new_commits);
                    } else {
                        change_map.insert(new_filename.clone(), singleton);
                    }
                },
                Change::Create(filename) => {
                    change_map.entry(filename.clone())
                        .and_modify(|commits| commits.push(commit))
                        .or_insert(singleton);
                }
                Change::Delete(filename) => {
                    change_map.remove(filename);
                }
            }
        }
    }
}


pub mod file {
    use super::commit;
    use super::commit_history;
    use super::directory;
    use crate::traits::{FileI, CommitI, ChangeI};

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
        type Change = commit::Change;

        fn history(&self) -> Vec<Self::Commit> {
            self.commits.clone()
        }

        fn directory(&self) -> Self::Directory {
            self.name.directory.clone()
        }

        fn build_contents(
            filename: Self::FileName,
            commits: Vec<Self::Commit>,
        ) -> Self::FileContents
        {
            let mut file_contents = FileContents::empty_file_contents();
            for commit in commits {
                let changes = commit.get_changes();
                changes.into_iter().filter(|change| {
                    change.get_filename() == filename
                }).for_each(|file_change| {
                    file_contents.apply_file_change(file_change)
                })
            }
            file_contents
        }

        fn to_file(name: Self::FileName, commits: Vec<Self::Commit>) -> File
        {
            let contents = File::build_contents(name.clone(), commits.clone());
            File { name, commits, contents }
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
                commit::Change::Addition(_, file_contents)  => {
                    self.contents = file_contents.contents;
                },
                commit::Change::Removal(_, file_contents) => {
                    self.contents = file_contents.contents;
                },
                commit::Change::Move(_, _) => {},
                commit::Change::Create(_)  => {},
                commit::Change::Delete(_)  => {},
            };
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct FileName {
        directory: directory::Directory,
        name: String,
    }

    #[cfg(test)]
    pub mod file_tests {
        use super::File;
        use crate::traits::properties;

        quickcheck! {
          fn prop_no_commits_no_files() -> bool {
              properties::prop_no_commits_no_files::<File>()
          }
        }
    }
}

pub mod directory {
    use quickcheck::Arbitrary;
    use quickcheck::Gen;

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

    impl Arbitrary for Directory {
      fn arbitrary<G: Gen>(g: &mut G) -> Self {
          let path = Arbitrary::arbitrary(g);
          Directory { path }
      }
    }

    #[cfg(test)]
    pub mod directory_tests {
        use super::Directory;
        use crate::traits::properties;

        quickcheck! {
          fn prop_is_prefix_identity(directory: Directory) -> bool {
              properties::prop_is_prefix_identity(directory)
          }
        }
    }
}
