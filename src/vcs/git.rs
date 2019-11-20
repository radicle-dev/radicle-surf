use crate::file_system;
use crate::vcs;
use crate::vcs::VCS;
use git2::{Commit, Error, Oid, Reference, Repository, TreeWalkMode, TreeWalkResult};
use nonempty::NonEmpty;
use std::collections::HashMap;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct GitRepository(pub(crate) Repository);

pub type GitRepo<'repo> = vcs::Repo<Commit<'repo>>;
pub type GitHistory<'repo> = vcs::History<Commit<'repo>>;
pub type GitBrowser<'repo> = vcs::Browser<'repo, GitRepository, Commit<'repo>, Error>;

impl<'repo> vcs::VCS<'repo, Commit<'repo>, Error> for GitRepository {
    type RepoId = String;
    type History = Reference<'repo>;
    type HistoryId = String;
    type ArtefactId = Oid;

    fn get_repo(repo_id: &Self::RepoId) -> Result<Self, Error> {
        Repository::open(repo_id).map(GitRepository)
    }

    fn get_history(&'repo self, history_id: &Self::HistoryId) -> Result<Self::History, Error> {
        self.0.resolve_reference_from_short_name(&history_id)
    }

    fn get_histories(&'repo self) -> Result<Vec<Self::History>, Error> {
        self.0.references().and_then(|mut references| {
            references.try_fold(vec![], |mut acc, reference| {
                reference.map(|r| {
                    acc.push(r);
                    acc
                })
            })
        })
    }

    fn get_identifier(artifact: &'repo Commit) -> Self::ArtefactId {
        artifact.id()
    }

    fn to_history(&'repo self, history: Self::History) -> Result<GitHistory, Error> {
        let head = history.peel_to_commit()?;
        let mut commits = NonEmpty::new(head.clone());
        let mut revwalk = self.0.revwalk()?;

        revwalk.push(head.id())?;

        for commit_result_id in revwalk {
            let commit_id = commit_result_id?;
            let commit = self.0.find_commit(commit_id)?;
            commits.push(commit.clone());
        }

        Ok(vcs::History(commits))
    }
}

impl<'repo> GitRepository {
    pub fn head(
        &'repo self,
    ) -> Result<<GitRepository as vcs::VCS<'repo, Commit<'repo>, Error>>::History, Error> {
        self.0.head()
    }
}

impl<'repo> GitBrowser<'repo> {
    pub fn new(repository: &'repo GitRepository) -> Result<Self, Error> {
        let head = repository.head()?;
        let history = repository.to_history(head)?;
        let snapshot = Box::new(|repository: &GitRepository, history: &GitHistory| {
            let tree = Self::get_tree(&repository.0, history.0.first())?;
            Ok(file_system::Directory::from::<GitRepository>(tree))
        });
        Ok(vcs::Browser {
            snapshot,
            history,
            repository: &repository,
        })
    }

    pub fn list_branches(&self) -> Result<Vec<String>, Error> {
        self.repository.0.branches(None).and_then(|mut branches| {
            branches.try_fold(vec![], |mut acc, branch| {
                let (branch, _) = branch?;
                let branch_name = branch.name()?;
                if let Some(name) = branch_name {
                    acc.push(name.to_string());
                    Ok(acc)
                } else {
                    Err(Error::from_str("Failed to decode branch name"))
                }
            })
        })
    }

    fn get_tree(
        repo: &Repository,
        commit: &Commit,
    ) -> Result<HashMap<file_system::Path, NonEmpty<file_system::File>>, Error> {
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
                            .or_insert_with(|| NonEmpty::new(file));
                    };
                    TreeWalkResult::Ok
                }
                Err(_) => TreeWalkResult::Skip,
            }
        })?;
        Ok(dir)
    }
}

impl file_system::RepoBackend for GitRepository {
    fn repo_directory() -> file_system::Directory {
        file_system::Directory {
            label: ".git".into(),
            entries: NonEmpty::new(file_system::DirectoryContents::Repo),
        }
    }
}

impl std::fmt::Debug for GitRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
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

#[cfg(test)]
mod tests {
    use crate::file_system::*;
    use crate::vcs::git::*;
    use git2::{IndexAddOption, IntoCString, Signature};
    use rm_rf;
    use std::panic;

    fn setup_golden_dir() {
        let repo =
            Repository::init("./data/git-test").expect("Failed to initialise './data/git-test'");
        repo.set_workdir(std::path::Path::new("./data/git-test"), true)
            .expect("Failed to set working dir for './data/git-test'");
        let mut index = repo.index().expect("Failed to get index");
        index
            .add_all("*".into_c_string(), IndexAddOption::DEFAULT, None)
            .expect("add all files failed");
        let tree_id = index.write_tree().expect("Failed to write Tree object");
        let signature = Signature::now("monadic.xyz", "test@monadic.xyz")
            .expect("Failed to initialise signature");
        let tree = repo
            .find_tree(tree_id)
            .expect("Failed to initialise Tree object");

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("Could not make first commit on './data/git-test'");
    }

    fn teardown_golden_dir() {
        rm_rf::ensure_removed("./data/git-test/.git")
            .expect("Failed to remove '.git' directory in './data/git-test'")
    }

    fn run_git_test<T>(test: T) -> ()
    where
        T: FnOnce() -> () + panic::UnwindSafe,
    {
        setup_golden_dir();

        let result = panic::catch_unwind(|| test());

        teardown_golden_dir();

        assert!(result.is_ok())
    }

    #[test]
    fn run_test_dir() {
        run_git_test(test_dir)
    }

    fn test_dir() {
        let repo: GitRepository = vcs::VCS::get_repo(&String::from("./data/git-test"))
            .expect("Could not retrieve ./data/git-test as git repository");
        let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
        let directory = browser.get_directory().expect("Could not render Directory");
        let mut directory_contents = directory.list_directory();
        directory_contents.sort();

        let mut directory_map = HashMap::new();

        // Root files set up, note that we're ignoring
        // file contents
        let mut root_files = NonEmpty::new(File {
            filename: "Cargo.toml".into(),
            contents: "".as_bytes().to_vec(),
        });
        root_files.push(File {
            filename: "Cargo.lock".into(),
            contents: "".as_bytes().to_vec(),
        });
        root_files.push(File {
            filename: ".gitignore".into(),
            contents: "".as_bytes().to_vec(),
        });
        directory_map.insert(Path::root(), root_files);

        // src files set up
        let src_files = NonEmpty::new(File {
            filename: "main.rs".into(),
            contents: "".as_bytes().to_vec(),
        });
        directory_map.insert(Path(NonEmpty::new("src".into())), src_files);

        let expected = file_system::Directory::from::<GitRepository>(directory_map);
        let mut expected_contents = expected.list_directory();
        expected_contents.sort();

        assert_eq!(directory_contents, expected_contents);

        // find src directory in the Git directory and the in-memory directory
        let src_directory = directory
            .find_directory(Path::from_labels("~".into(), &["src".into()]))
            .unwrap();
        let mut src_directory_contents = src_directory.list_directory();
        src_directory_contents.sort();

        let expected_src_directory = expected
            .find_directory(Path::from_labels("~".into(), &["src".into()]))
            .unwrap();
        let mut expected_src_directory_contents = expected_src_directory.list_directory();
        expected_src_directory_contents.sort();

        assert_eq!(src_directory_contents, expected_src_directory_contents);
    }
}
