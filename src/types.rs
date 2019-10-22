use chrono::prelude::*;

use crate::traits;

struct Repo {
    histories: Vec<CommitHistory>,
}

impl traits::RepoI for Repo {
    type History = CommitHistory;

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
        -> Option<<<Repo as traits::RepoI>::History as traits::CommitHistoryI>::Commit> {
        panic!("Unimplemented!");
    }
}

#[derive(Debug, Clone)]
struct CommitHistory {
    commits: Vec<Commit>,
}

impl traits::CommitHistoryI for CommitHistory {
    type Commit = Commit;

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

struct Branch {
    name: String,
    commit_history: CommitHistory,
}

struct Tag {
    name: String,
    commit_history: CommitHistory,
}

#[derive(Debug, Clone)]
struct Commit {
    author: String,
    hash: String,
    date: DateTime<Utc>,
    message: String,
    signature: Option<String>,
    parent_commits: Vec<Commit>,
    changes: Vec<Change>,
}

impl traits::CommitI for Commit {
    type Repo = Repo;
    type History = CommitHistory;
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
enum Change {
    Addition(FileName, Location, FileContents),
    Removal(FileName, Location),
    Move(FileName, FileName),
    Create(FileName),
    Delete(FileName),
}

// type Location = Location(uint32);

#[derive(Debug, Clone)]
struct Location {}

struct File {
    name: FileName,
    commits: Vec<Commit>,
}

#[derive(Debug, Clone)]
struct FileContents {
    contents: String
}

#[derive(Debug, Clone)]
struct FileName {
    directory: Directory,
    name: String,
}

#[derive(Debug, Clone)]
struct Directory {
    path: Vec<String>,
}

impl traits::DirectoryI for Directory {
    type History = CommitHistory;

    // Let's be honest the return result is Self::History::Commit
    fn history(&self, history: Self::History)
        -> Vec<<<Self as traits::DirectoryI>::History as traits::CommitHistoryI>::Commit> {
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
