use chrono::prelude::*;

struct Repo {
    histories: Vec<CommitHistory>,
}

impl Repo {
    fn new() -> Repo {
        Repo { histories: Vec::new() }
    }

    fn add_commit_history(&mut self, history: CommitHistory) {
        self.histories.push(history)
    }
}

struct CommitHistory {
    commits: Vec<Commit>,
}

struct Branch {
    name: String,
    commit_history: CommitHistory,
}

struct Tag {
    name: String,
    commit_history: CommitHistory,
}

struct Commit {
    author: String,
    hash: String,
    date: DateTime<Utc>,
    message: String,
    signature: Option<String>,
    parent_commits: Vec<Commit>,
    changes: Vec<Change>,
}

enum Change {
    Addition(FileName, Location, FileContents),
    Removal(FileName, Location),
    Move(FileName, FileName),
    Create(FileName),
    Delete(FileName),
}

// type Location = Location(uint32);
struct Location {}

struct File {
    name: FileName,
    commits: Vec<Commit>,
}

struct FileContents {
    contents: String
}

struct FileName {
    directory: Directory,
    name: String,
}

struct Directory {
    path: Vec<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
