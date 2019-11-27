use crate::file_system;
use crate::vcs;
use crate::vcs::VCS;
use git2::{Commit, Error, Oid, Reference, Repository, TreeWalkMode, TreeWalkResult};
use nonempty::NonEmpty;
use std::collections::HashMap;

#[derive(Debug)]
pub enum GitError {
    EmptyCommitHistory,
    BranchDecode,
    Internal(Error),
}

impl From<Error> for GitError {
    fn from(err: Error) -> Self {
        GitError::Internal(err)
    }
}

/// A `History` that uses `git2::Commit` as the underlying artifact.
pub type GitHistory<'repo> = vcs::History<Commit<'repo>>;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct GitRepository(pub(crate) Repository);

impl<'repo> GitRepository {
    /// Open a git repository given its URI.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let browser = GitBrowser::new(&repo).unwrap();
    ///
    /// for branch in browser.list_branches().unwrap() {
    ///     println!("Branch: {}", branch);
    /// }
    /// ```
    pub fn new(repo_uri: &str) -> Result<Self, GitError> {
        Repository::open(repo_uri)
            .map(GitRepository)
            .map_err(GitError::from)
    }

    pub(crate) fn head(&'repo self) -> Result<GitHistory, GitError> {
        let head = self.0.head()?;
        self.to_history(head)
    }

    pub(crate) fn to_history(
        &'repo self,
        history: Reference<'repo>,
    ) -> Result<GitHistory, GitError> {
        let head = history.peel_to_commit()?;
        let mut commits = Vec::new();
        let mut revwalk = self.0.revwalk()?;

        // Set the revwalk to the head commit
        revwalk.push(head.id())?;

        for commit_result_id in revwalk {
            // The revwalk iter returns results so
            // we unpack these and push them to the history
            let commit_id: Oid = commit_result_id?;
            let commit = self.0.find_commit(commit_id)?;
            commits.push(commit.clone());
        }

        NonEmpty::from_slice(&commits)
            .map(vcs::History)
            .ok_or(GitError::EmptyCommitHistory)
    }
}

impl<'repo> vcs::GetVCS<'repo, GitError> for GitRepository {
    type RepoId = &'repo str;

    fn get_repo(repo_id: Self::RepoId) -> Result<Self, GitError> {
        Repository::open(repo_id)
            .map(GitRepository)
            .map_err(GitError::from)
    }
}

impl<'repo> vcs::VCS<'repo, Commit<'repo>, GitError> for GitRepository {
    type HistoryId = &'repo str;
    type ArtefactId = Oid;

    fn get_history(&'repo self, history_id: Self::HistoryId) -> Result<GitHistory, GitError> {
        self.0
            .resolve_reference_from_short_name(&history_id)
            .map_err(GitError::from)
            .and_then(|reference| self.to_history(reference))
    }

    fn get_histories(&'repo self) -> Result<Vec<GitHistory>, GitError> {
        self.0
            .references()
            .map_err(GitError::from)
            .and_then(|mut references| {
                references.try_fold(vec![], |mut acc, reference| {
                    reference.map_err(GitError::from).and_then(|r| {
                        let history = self.to_history(r)?;
                        acc.push(history);
                        Ok(acc)
                    })
                })
            })
    }

    fn get_identifier(artifact: &'repo Commit) -> Self::ArtefactId {
        artifact.id()
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

/// A `Browser` that uses `GitRepository` as the underlying repository backend,
/// `git2::Commit` as the artifact, and `git2::Error` for error reporting.
pub type GitBrowser<'repo> = vcs::Browser<'repo, GitRepository, Commit<'repo>, GitError>;

impl<'repo> GitBrowser<'repo> {
    /// Create a new browser to interact with.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let browser = GitBrowser::new(&repo).unwrap();
    ///
    /// for branch in browser.list_tags().unwrap() {
    ///     println!("Branch: {}", branch);
    /// }
    /// ```
    pub fn new(repository: &'repo GitRepository) -> Result<Self, GitError> {
        let history = repository.head()?;
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

    /// Set the current `GitBrowser` history to the
    /// HEAD commit of the underlying repository.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// // ensure we're at HEAD
    /// browser.head();
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// ```
    pub fn head(&mut self) -> Result<(), GitError> {
        let history = self.repository.head()?;
        self.set_history(history);
        Ok(())
    }

    /// Set the current `GitBrowser` history to the
    /// branch name provided.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// // ensure we're at 'master'
    /// browser.branch("master");
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// ```
    pub fn branch(&mut self, branch_name: &'repo str) -> Result<(), GitError> {
        let branch = self.repository.get_history(branch_name)?;
        self.set_history(branch);
        Ok(())
    }

    /// List the names of the branches that are contained in the
    /// underlying `GitRepository`.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let branches = browser.list_branches().unwrap();
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&"master".to_string()));
    /// ```
    pub fn list_branches(&self) -> Result<Vec<String>, GitError> {
        self.repository
            .0
            .branches(None)
            .map_err(GitError::from)
            .and_then(|mut branches| {
                branches.try_fold(vec![], |mut acc, branch| {
                    let (branch, _) = branch?;
                    let branch_name = branch.name()?;
                    if let Some(name) = branch_name {
                        acc.push(name.to_string());
                        Ok(acc)
                    } else {
                        Err(GitError::BranchDecode)
                    }
                })
            })
    }

    /// List the names of the tags that are contained in the
    /// underlying `GitRepository`.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new(".").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let tags = browser.list_tags().unwrap();
    ///
    /// // We currently have no tags :(
    /// assert!(tags.is_empty());
    /// ```
    pub fn list_tags(&self) -> Result<Vec<String>, GitError> {
        let tags = self.repository.0.tag_names(None)?;
        Ok(tags
            .into_iter()
            .filter_map(|tag| tag.map(String::from))
            .collect())
    }

    /// Do a pre-order TreeWalk of the given commit. This turns a Tree
    /// into a HashMap of Paths and a list of Files. We can then turn that
    /// into a Directory.
    fn get_tree(
        repo: &Repository,
        commit: &Commit,
    ) -> Result<HashMap<file_system::Path, NonEmpty<file_system::File>>, GitError> {
        let mut dir: HashMap<file_system::Path, NonEmpty<file_system::File>> = HashMap::new();
        let tree = commit.as_object().peel_to_tree()?;
        tree.walk(TreeWalkMode::PreOrder, |s, entry| {
            let path = file_system::Path::from_string(s);

            entry
                .to_object(repo)
                .map(|object| {
                    object.as_blob().and_then(|blob| {
                        entry.name().and_then(|filename| {
                            let file = file_system::File {
                                filename: filename.into(),
                                contents: blob.content().to_owned(),
                                size: blob.size(),
                            };
                            dir.entry(path)
                                .and_modify(|entries| entries.push(file.clone()))
                                .or_insert_with(|| NonEmpty::new(file));
                            Some(TreeWalkResult::Ok)
                        })
                    });
                    TreeWalkResult::Ok
                })
                .unwrap_or(TreeWalkResult::Skip)
        })?;
        Ok(dir)
    }

    fn commit_contains_path(
        &self,
        commit: Commit<'repo>,
        path: &file_system::Path,
    ) -> Option<Commit<'repo>> {
        let (directory, filename) = path.split_last();
        let commit_tree = commit.tree().ok()?;

        if directory == vec![file_system::Label::root()] {
            commit_tree.get_name(&filename.0).map(|_| commit)
        } else {
            let mut directory_path = std::path::PathBuf::new();
            for dir in directory {
                if dir == file_system::Label::root() {
                    continue;
                }

                directory_path.push(dir.0);
            }

            let tree_entry = commit_tree.get_path(directory_path.as_path()).ok()?;
            let object = tree_entry.to_object(&self.repository.0).ok()?;
            let tree = object.as_tree().map(|t| t.get_name(&filename.0));
            tree.map(|_| commit)
        }
    }

    pub fn last_commit(&self, path: &file_system::Path) -> Option<Commit> {
        self.get_history()
            .find(|commit| self.commit_contains_path(commit.clone(), path))
    }
}

#[cfg(test)]
mod tests {
    use crate::file_system::*;
    use crate::vcs::git::*;
    use std::panic;

    #[test]
    fn test_dir() {
        let repo = GitRepository::new("./data/git-golden")
            .expect("Could not retrieve ./data/git-golden as git repository");
        let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
        let directory = browser.get_directory().expect("Could not render Directory");
        let mut directory_contents = directory.list_directory();
        directory_contents.sort();

        let mut directory_map = HashMap::new();

        // Root files set up, note that we're ignoring
        // file contents
        let root_files = (
            File::new("Cargo.toml".into(), &[]),
            vec![File::new(".gitignore".into(), &[])],
        )
            .into();
        directory_map.insert(Path::root(), root_files);

        // src files set up
        let src_files = NonEmpty::new(File::new("main.rs".into(), &[]));
        directory_map.insert(Path(NonEmpty::new("src".into())), src_files);

        let expected = file_system::Directory::from::<GitRepository>(directory_map);
        let mut expected_contents = expected.list_directory();
        expected_contents.sort();

        assert_eq!(directory_contents, expected_contents);

        // find src directory in the Git directory and the in-memory directory
        let src_directory = directory
            .find_directory(&Path::from_labels("~".into(), &["src".into()]))
            .unwrap();
        let mut src_directory_contents = src_directory.list_directory();
        src_directory_contents.sort();

        let expected_src_directory = expected
            .find_directory(&Path::from_labels("~".into(), &["src".into()]))
            .unwrap();
        let mut expected_src_directory_contents = expected_src_directory.list_directory();
        expected_src_directory_contents.sort();

        assert_eq!(src_directory_contents, expected_src_directory_contents);
    }

    #[test]
    fn test_last_commit_head() {
        let repo = GitRepository::new("./data/git-golden")
            .expect("Could not retrieve ./data/git-test as git repository");
        let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
        let head = browser.get_history().0.first().clone();

        let toml_last_commit = browser
            .last_commit(&Path::from_labels(Label::root(), &["Cargo.toml".into()]))
            .map(|commit| commit.id());

        assert_eq!(toml_last_commit, Some(head.id()));

        let main_last_commit = browser
            .last_commit(&Path::from_labels(
                Label::root(),
                &["src".into(), "main.rs".into()],
            ))
            .map(|commit| commit.id());

        assert_eq!(main_last_commit, Some(head.id()));
    }

    #[test]
    fn test_last_commit_before_head() {
        let repo = GitRepository::new("./data/git-golden")
            .expect("Could not retrieve ./data/git-golden as git repository");
        let mut browser = GitBrowser::new(&repo).expect("Could not initialise Browser");

        // Set history to HEAD~2
        browser.view_at(browser.get_history(), |history| {
            let history_vec: Vec<Commit> = history.0.clone().into();
            Some(vcs::History(NonEmpty::new(history_vec[2].clone())))
        });
        let head = browser.get_history().0.first().clone();

        // Cargo.toml is commited second so it should not exist here.
        let toml_last_commit = browser
            .last_commit(&Path::from_labels(Label::root(), &["Cargo.toml".into()]))
            .map(|commit| commit.id());

        assert_eq!(toml_last_commit, None);

        // src/main.rs exists in this commit.
        let main_last_commit = browser
            .last_commit(&Path::from_labels(
                Label::root(),
                &["src".into(), "main.rs".into()],
            ))
            .map(|commit| commit.id());

        assert_eq!(main_last_commit, Some(head.id()));
    }
}
