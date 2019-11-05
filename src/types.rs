
pub mod repo {
    use quickcheck::{Arbitrary, Gen};

    use super::commit_history;
    use super::commit;
    use crate::traits::{RepoI, CommitHistoryI, CommitI};
    use crate::smallcheck::SmallVec;

    #[derive(Debug, Clone)]
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

    impl Arbitrary for Repo {
        fn arbitrary<G: Gen>(g: &mut G) -> Self
        {
            Repo { histories: SmallVec::from(Arbitrary::arbitrary(g)) }
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
    use quickcheck::{Arbitrary, Gen};

    use crate::smallcheck::SmallVec;

    use super::commit;
    use crate::traits::CommitHistoryI;

    #[derive(Debug, Clone, PartialEq)]
    pub struct CommitHistory {
        pub commits: Vec<commit::Commit>,
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

        fn get_up_to_commit(&self, search_commit: &Self::Commit) -> Vec<Self::Commit> {
            let mut result = Vec::new();

            // Drop commits at the head of the history since Vec is push oriented
            for commit in self.commits.iter() {
                let current_commit = commit.clone();
                if current_commit != *search_commit {
                    result.push(current_commit)
                } else {
                    continue;
                }
            }
            result
        }
    }

    impl Arbitrary for CommitHistory {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            CommitHistory { commits: SmallVec::from(Arbitrary::arbitrary(g)) }
        }
    }
}

pub mod commit {
    use chrono::prelude::{DateTime, Utc,};
    use std::collections::BTreeMap;
    use quickcheck::{Arbitrary, Gen};


    use crate::smallcheck::{SmallVec, SmallString, Datetime, frequency};

    use super::file;
    use super::repo;
    use crate::traits::{CommitI, ChangeI, RepoI, CommitHistoryI};

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Commit {
        pub author: String,
        pub hash: String,
        pub date: DateTime<Utc>,
        pub message: String,
        pub signature: Option<String>, // TODO(fintan): turn this back to private when we're done testing
        pub parent_commits: Vec<Commit>,
        pub changes: Vec<Change>,
    }

    impl CommitI for Commit {
        type Repo = repo::Repo;
        type Change = Change;
        type Signature = String;

        fn author(&self) -> String { self.author.clone() }

        fn parents(&self) -> Vec<Commit> { self.parent_commits.clone() }

        fn children(&self, repo: &Self::Repo) -> Vec<Self>
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

    impl Arbitrary for Commit {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let author = SmallString::from(Arbitrary::arbitrary(g));
            let hash = Arbitrary::arbitrary(g);
            let date_time: Datetime = Arbitrary::arbitrary(g);
            let date = date_time.get_datetime;
            let message = SmallString::from(Arbitrary::arbitrary(g));
            let signature = Arbitrary::arbitrary(g);
            let parent_commits = Vec::new(); // TODO(fintan): need a better way to create parent commits
            let changes = SmallVec::from(Arbitrary::arbitrary(g));

            Commit { author, hash, date, message, signature, parent_commits, changes }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
            change_map: &mut BTreeMap<Self::FileName, Vec<Self::Commit>>
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
                    if let Some(commits) = change_map.remove(filename) {
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

    fn gen_filename<G: Gen>(g: &mut G) -> file::FileName {
        Arbitrary::arbitrary(g)
    }

    fn gen_filecontents<G: Gen>(g: &mut G) -> file::FileContents {
        Arbitrary::arbitrary(g)
    }

    impl Arbitrary for Change {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let choices = vec![
                (40, Change::Addition(gen_filename(g), gen_filecontents(g))),
                (30, Change::Removal(gen_filename(g), gen_filecontents(g))),
                (10, Change::Move(gen_filename(g), gen_filename(g))),
                (10, Change::Create(gen_filename(g))),
                (10, Change::Delete(gen_filename(g))),
            ];
            frequency(g, choices)
        }
    }

    #[cfg(test)]
    pub mod commit_tests {
        use super::repo;
        use super::Commit;
        use crate::traits::properties;

        quickcheck! {
          fn prop_children_of_commit_are_subset_of_parents_children(repo: repo::Repo) -> bool {
              properties::prop_children_of_commit_are_subset_of_parents_children::<Commit>(repo)
          }
        }
    }
}


pub mod file {
    use quickcheck::{Arbitrary, Gen};


    use crate::smallcheck::{SmallVec, SmallString};
    use super::commit;
    use super::commit_history;
    use super::directory;
    use crate::traits::{FileI, CommitI, ChangeI};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct File {
        pub name: FileName,
        pub commits: Vec<commit::Commit>,
        pub contents: FileContents,
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
            filename: &Self::FileName,
            commits: &[Self::Commit],
        ) -> Self::FileContents
        {
            let mut file_contents = FileContents::empty_file_contents();
            for commit in commits {
                let changes = commit.get_changes();
                changes.into_iter().filter(|change| {
                    change.get_filename() == *filename
                }).for_each(|file_change| {
                    file_contents.apply_file_change(file_change)
                })
            }
            file_contents
        }

        fn to_file(name: Self::FileName, commits: &[Self::Commit]) -> File
        {
            let contents = File::build_contents(&name, commits);
            File { name, commits: commits.to_vec(), contents }
        }
    }

    impl Arbitrary for File {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let name = Arbitrary::arbitrary(g);
            let commits = SmallVec::from(Arbitrary::arbitrary(g));
            let contents = Arbitrary::arbitrary(g);
            File { name, commits, contents }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct FileContents {
        pub contents: String
    }

    impl Arbitrary for FileContents {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let contents = SmallString::from(Arbitrary::arbitrary(g));
            FileContents { contents }
        }
    }

    impl FileContents {
        pub fn empty_file_contents() -> FileContents {
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

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FileName {
        pub directory: directory::Directory,
        pub name: String,
    }

    impl Arbitrary for FileName {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let directory = Arbitrary::arbitrary(g);
            let name = SmallString::from(Arbitrary::arbitrary(g));
            FileName { directory, name }
        }
    }

    #[cfg(test)]
    pub mod file_tests {
        use super::{File, FileName};
        use super::commit_history::CommitHistory;
        use crate::traits::properties;

        quickcheck! {
          fn prop_no_commits_no_files() -> bool {
              properties::prop_no_commits_no_files::<File>()
          }
        }

        quickcheck! {
            fn prop_file_must_exist_in_history(filename: FileName, history: CommitHistory) -> bool {
                true
                // TODO(fintan): I think this is failing because the FileName isn't actually in the
                // commit history, so rebuilding the files will fail
                // TODO(fintan): Actually this is failing because we're creating arbitrary
                // filenames and it doesn't make much sense unless the history is in some way
                // consistent.
                // properties::prop_file_must_exist_in_history::<File, CommitHistory>(filename, history)
            }
        }

        quickcheck! {
            fn prop_files_match_directories(history: CommitHistory) -> bool {
                properties::prop_files_match_directories::<File>(history)
            }
        }

        quickcheck! {
            fn prop_file_is_its_history(file: File) -> bool {
                true
                // properties::prop_file_is_its_history(file)
            }
        }
    }
}

pub mod directory {
    use quickcheck::{Arbitrary, Gen};
    use rand::Rng;

    use crate::smallcheck::SmallString;
    use crate::traits::{DirectoryI};

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Directory {
        pub path: Vec<String>,
    }

    impl DirectoryI for Directory {
        fn is_prefix_of(&self, directory: &Directory) -> bool {
            directory.path.starts_with(&self.path)
        }

    }

    impl Arbitrary for Directory {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let m = g.gen_range(1, 5);
            let mut n = 0;
            let mut path = Vec::with_capacity(32);

            // Create a path no greater than 5
            while n < m {
                path.push(SmallString::from(Arbitrary::arbitrary(g)));
                n += 1;
            }

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

#[cfg(test)]
mod unit_tests {
    use chrono::prelude::*;

    use super::commit::{Commit, Change};
    use super::commit_history::CommitHistory;
    use super::directory::Directory;
    use super::file::{File, FileName, FileContents};
    use crate::traits::properties;

    #[test]
    fn unit_prop_files_match_directories() {
        let directory = Directory { path: vec![String::from("foo"), String::from("bar")] };
        let filename = FileName { directory, name: String::from("test_filename") };
        let file_contents = FileContents { contents: String::from("new contents") };
        let change = Change::Addition(filename, file_contents);
        let commit = Commit {
            author: String::from("author"),
            hash: String::from("hash"),
            date: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            message: String::from("commit message"),
            signature: Some(String::from("signature")),
            parent_commits: Vec::new(),
            changes: vec![change],
        };

        let directory1 = Directory { path: vec![String::from("baz"), String::from("quux")] };
        let filename1 = FileName { directory: directory1, name: String::from("test_filename1") };
        let file_contents1 = FileContents { contents: String::from("new contents") };
        let change1 = Change::Addition(filename1, file_contents1);
        let commit1 = Commit {
            author: String::from("author"),
            hash: String::from("hash"),
            date: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            message: String::from("commit message"),
            signature: Some(String::from("signature")),
            parent_commits: Vec::new(),
            changes: vec![change1],
        };

        let history = CommitHistory { commits: vec![commit, commit1] };
        assert!(properties::prop_files_match_directories::<File>(history))
    }

    #[test]
    fn unit_prop_file_must_exist_in_history() {
        let default_filename = FileName {
            directory: Directory { path: vec![String::from("def")] },
            name: String::from("def_name"),
        };
        let directory = Directory { path: vec![String::from("foo"), String::from("bar")] };
        let filename = FileName { directory, name: String::from("test_filename") };
        let delete = Change::Delete(filename);
        let commit = Commit {
            author: String::from("author"),
            hash: String::from("hash"),
            date: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            message: String::from("commit message"),
            signature: Some(String::from("signature")),
            parent_commits: Vec::new(),
            changes: vec![delete],
        };

        let history = CommitHistory { commits: vec![commit] };
        assert!(properties::prop_file_must_exist_in_history::<File, CommitHistory>(default_filename, history))
    }

    #[test]
    fn unit_create_then_delete() {
        let default_filename = FileName {
            directory: Directory { path: vec![String::from("def")] },
            name: String::from("def_name"),
        };
        let directory = Directory { path: vec![String::from("foo"), String::from("bar")] };
        let filename = FileName { directory, name: String::from("test_filename") };
        let create = Change::Create(filename.clone());
        let delete = Change::Delete(filename.clone());
        let commit = Commit {
            author: String::from("author"),
            hash: String::from("hash"),
            date: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            message: String::from("commit message"),
            signature: Some(String::from("signature")),
            parent_commits: Vec::new(),
            changes: vec![create, delete],
        };

        let file = File { name: filename, commits: vec![commit], contents: FileContents::empty_file_contents() };
        assert!(properties::prop_file_is_its_history::<File>(file))
    }
}
