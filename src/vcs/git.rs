use crate::file_system;
use crate::vcs::{History, VCS};
use git2::{
    Blob, Branch, BranchType, Commit, Error, Oid, Repository, TreeEntry, TreeWalkMode,
    TreeWalkResult,
};
use nonempty::NonEmpty;
use std::collections::HashMap;

pub struct GitRepo(pub(crate) Repository);

impl file_system::RepoBackend for GitRepo {
    fn new() -> file_system::Directory {
        file_system::Directory {
            label: ".git".into(),
            entries: NonEmpty::new(file_system::DirectoryContents::Repo),
        }
    }
}

impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

impl<'repo> VCS<'repo, Commit<'repo>> for GitRepo {
    type RepoId = String;
    type History = Branch<'repo>;
    type HistoryId = (String, BranchType);
    type ArtefactId = Oid;

    fn get_repo(repo_id: &Self::RepoId) -> Option<Self> {
        Repository::open(repo_id).map(GitRepo).ok()
    }

    fn get_history(&'repo self, history_id: &Self::HistoryId) -> Option<Self::History> {
        self.0.find_branch(&history_id.0, history_id.1).ok()
    }

    fn get_histories(&'repo self) -> Vec<Self::History> {
        let mut histories = Vec::new();
        let _: Result<(), Error> = self.0.branches(None).map(|branches| {
            branches
                .filter_map(|branch| branch.ok())
                .for_each(|(branch, _)| {
                    println!("Grabbing branch: {:#?}", branch.name());
                    histories.push(branch)
                });
        });
        histories
    }

    fn get_identifier(artifact: &'repo Commit) -> Self::ArtefactId {
        artifact.id()
    }

    fn to_history(&'repo self, history: Self::History) -> Option<History<Commit<'repo>>> {
        let head = history.get().peel_to_commit().ok()?;
        let mut commits = NonEmpty::new(head.clone());
        let mut revwalk = self.0.revwalk().ok()?;

        revwalk.push(head.id()).ok()?;

        for commit_result_id in revwalk {
            let commit_id = commit_result_id.ok()?;
            let commit = self.0.find_commit(commit_id).ok()?;
            commits.push(commit.clone());
        }

        Some(History(commits))
    }
}

/*
pub fn list_branches(repo: Repository) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    let branches: Branches = repo.branches(None)?;
    for branch_result in branches {
        let (branch, branch_type) = branch_result?;
        let name = branch.name()?;
        if let Some(n) = name {
            let result = match branch_type {
                BranchType::Local => format!("local: {}", n),
                BranchType::Remote => format!("remote: {}", n),
            };
            names.push(result);
        }
    }
    Ok(names)
}
*/

type TreeDirectory = HashMap<file_system::Path, NonEmpty<file_system::File>>;

fn get_tree(repo: &Repository, commit: &Commit) -> Result<TreeDirectory, Error> {
    let mut dir: HashMap<file_system::Path, NonEmpty<file_system::File>> = HashMap::new();
    let tree = commit.as_object().peel_to_tree().unwrap();
    tree.walk(TreeWalkMode::PreOrder, |s, entry| {
        let path = file_system::Path::from_string(s);

        match entry.to_object(repo) {
            Ok(object) => {
                if let Some(blob) = object.as_blob() {
                    let filename = entry.name().map(|name| name.into()).unwrap();
                    let file = file_system::File {
                        filename,
                        contents: blob.content().to_owned(),
                    };
                    dir.entry(path)
                        .and_modify(|entries| entries.push(file.clone()))
                        .or_insert(NonEmpty::new(file));
                };
                TreeWalkResult::Ok
            }
            Err(_) => TreeWalkResult::Skip,
        }
    })?;
    Ok(dir)
}

/*
fn to_directory(tree_dir: &TreeDirectory) -> file_system::Directory<GitRepo> {
    let directory = Vec::new();
    for (dir_label, file) in tree_dir {
        file_system
    }
}
*/

#[cfg(test)]
mod tests {
    use crate::file_system::*;
    use crate::vcs::git::*;
    use git2::{BranchType, Commit, Error, Repository, TreeWalkMode, TreeWalkResult};

    #[test]
    fn test_dir() {
        let repo = Repository::open("/home/haptop/Developer/radicle-surf").unwrap();
        let dir = get_tree(&repo, &repo.head().unwrap().peel_to_commit().unwrap());
        let native_dir: Directory = Directory::from::<GitRepo>(dir.unwrap());
        println!("{:#?}", native_dir);
        /*
        let tree = repo.head().unwrap().peel_to_tree().unwrap();
        tree.walk(TreeWalkMode::PreOrder, |s, entry| {
            println!("What's this: {:#?}", s);
            println!("{:#?}", entry.name());
            println!("Kind: {:#?}", entry.kind());
            TreeWalkResult::Ok
        })
        .unwrap();
        */
        /*
        for (dirname, entries) in dir {
            for entry in entries {
                println!("{:#?}{:#?}", dirname, entry.name().unwrap());
            }
        }
        */

        assert!(false)
    }
    /*
    #[test]
    fn test_vcs() {
        let repo: GitRepo =
            VCS::get_repo(&String::from("/home/haptop/Developer/radicle-surf")).unwrap();
        let native_repo = repo.to_repo();
        for history in native_repo.0 {
            for commit in history.iter() {
                println!("{:#?}", commit.message());
            }
        }
        assert!(false)
    }
    */
}
