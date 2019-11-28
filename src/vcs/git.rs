//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Path, SystemType};
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//!
//! let repo = GitRepository::new("./data/git-golden")
//!     .expect("Could not retrieve ./data/git-golden as git repository");
//! let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
//! let directory = browser.get_directory().expect("Could not render Directory");
//! let mut directory_contents = directory.list_directory();
//! directory_contents.sort();
//!
//! assert_eq!(directory_contents, vec![
//!     SystemType::directory(".git".into()),
//!     SystemType::file(".gitignore".into()),
//!     SystemType::file("Cargo.toml".into()),
//!     SystemType::directory("src".into()),
//! ]);
//!
//! // find src directory in the Git directory and the in-memory directory
//! let src_directory = directory
//!     .find_directory(&Path::with_root(&["src".into()]))
//!     .unwrap();
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file("main.rs".into()),
//! ]);
//! ```

use crate::file_system;
use crate::vcs;
use crate::vcs::VCS;
use git2::{Commit, Error, Oid, Reference, Repository, TreeWalkMode, TreeWalkResult};
use nonempty::NonEmpty;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum GitError {
    EmptyCommitHistory,
    BranchDecode,
    NotBranch,
    NotTag,
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
    /// # Examples
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let branches = browser.list_branches();
    ///
    /// assert_eq!(
    ///     branches,
    ///     Ok(vec![
    ///         BranchName::new("master"),
    ///         BranchName::new("origin/HEAD"),
    ///         BranchName::new("origin/add-tests"),
    ///         BranchName::new("origin/master"),
    ///     ])
    /// );
    /// ```
    pub fn new(repo_uri: &str) -> Result<Self, GitError> {
        Repository::open(repo_uri)
            .map(GitRepository)
            .map_err(GitError::from)
    }

    /// Get a particular `Commit`.
    pub(crate) fn get_commit(&'repo self, sha: Sha1) -> Result<Commit<'repo>, GitError> {
        let oid = Oid::from_str(&sha.0)?;
        let commit = self.0.find_commit(oid)?;
        Ok(commit)
    }

    /// Build a `GitHistory` using the `head` reference.
    pub(crate) fn head(&'repo self) -> Result<GitHistory, GitError> {
        let head = self.0.head()?;
        self.to_history(&head)
    }

    /// Turn a `git2::Reference` into a `GitHistory` by completing
    /// a revwalk over the first commit in the reference.
    pub(crate) fn to_history(
        &'repo self,
        history: &Reference<'repo>,
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

/// A newtype wrapper over `String` to separate out
/// the fact that a caller wants to fetch a branch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(name: &str) -> Self {
        BranchName(name.into())
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }
}

/// A newtype wrapper over `String` to separate out
/// the fact that a caller wants to fetch a tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagName(String);

impl TagName {
    pub fn new(name: &str) -> Self {
        TagName(name.into())
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }
}

/// A newtype wrapper over `String` to separate out
/// the fact that a caller wants to fetch a commit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sha1(String);

impl Sha1 {
    pub fn new(name: &str) -> Self {
        Sha1(name.into())
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }
}

/// An enumeration of git objects we can fetch and turn
/// into a [`GitHistory`](struct.GitHistory.html).
#[derive(Debug, Clone)]
pub enum GitObject {
    Branch(BranchName),
    Tag(TagName),
}

impl GitObject {
    pub fn branch(name: &str) -> Self {
        GitObject::Branch(BranchName::new(name))
    }

    pub fn tag(name: &str) -> Self {
        GitObject::Tag(TagName::new(name))
    }

    fn get_name(&self) -> String {
        match self {
            GitObject::Branch(name) => name.0.clone(),
            GitObject::Tag(name) => name.0.clone(),
        }
    }
}

impl<'repo> vcs::VCS<'repo, Commit<'repo>, GitError> for GitRepository {
    type HistoryId = GitObject;
    type ArtefactId = Oid;

    fn get_history(&'repo self, history_id: Self::HistoryId) -> Result<GitHistory, GitError> {
        let reference = self
            .0
            .resolve_reference_from_short_name(&history_id.get_name())?;
        let to_history = |pred, err| {
            if pred {
                self.to_history(&reference)
            } else {
                Err(err)
            }
        };
        match history_id {
            GitObject::Branch(_) => to_history(
                reference.is_branch() || reference.is_remote(),
                GitError::NotBranch,
            ),
            GitObject::Tag(_) => to_history(reference.is_tag(), GitError::NotTag),
        }
    }

    fn get_histories(&'repo self) -> Result<Vec<GitHistory>, GitError> {
        self.0
            .references()
            .map_err(GitError::from)
            .and_then(|mut references| {
                references.try_fold(vec![], |mut acc, reference| {
                    reference.map_err(GitError::from).and_then(|r| {
                        let history = self.to_history(&r)?;
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

/// A `Browser` that uses [`GitRepository`](struct.GitRepository.html) as the underlying repository backend,
/// `git2::Commit` as the artifact, and [`GitError`](enum.GitError.html) for error reporting.
pub type GitBrowser<'repo> = vcs::Browser<'repo, GitRepository, Commit<'repo>, GitError>;

impl<'repo> GitBrowser<'repo> {
    /// Create a new browser to interact with.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let browser = GitBrowser::new(&repo).unwrap();
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

    /// Set the current `GitBrowser` history to the `HEAD` commit of the underlying repository.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
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

    /// Set the current `GitBrowser` history to the [`BranchName`](struct.BranchName.html)
    /// provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// // ensure we're on 'master'
    /// browser.branch(BranchName::new("master"));
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, GitBrowser, GitRepository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    /// browser
    ///     .branch(BranchName::new("origin/add-tests"))
    ///     .expect("Failed to change branch to add-tests");
    ///
    /// let directory = browser.get_directory().expect("Failed to get directory");
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::directory(".git".into()),
    ///         SystemType::file(".gitignore".into()),
    ///         SystemType::file("Cargo.toml".into()),
    ///         SystemType::directory("src".into()),
    ///         SystemType::directory("tests".into()),
    ///     ]
    /// );
    ///
    /// let tests = directory
    ///     .find_directory(&Path::with_root(&["tests".into()]))
    ///     .expect("tests not found");
    /// let mut tests_contents = tests.list_directory();
    /// tests_contents.sort();
    ///
    /// assert_eq!(
    ///     tests_contents,
    ///     vec![SystemType::file("mod.rs".into())]
    /// );
    /// ```
    pub fn branch(&mut self, branch_name: BranchName) -> Result<(), GitError> {
        let branch = self
            .repository
            .get_history(GitObject::Branch(branch_name))?;
        self.set_history(branch);
        Ok(())
    }

    /// Set the current `GitBrowser` history to the [`TagName`](struct.TagName.html)
    /// provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use git2::Oid;
    /// use radicle_surf::vcs::History;
    /// use radicle_surf::vcs::git::{TagName, GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// // Switch to "v0.0.1"
    /// browser.tag(TagName::new("v0.0.1"));
    ///
    /// let expected_history = History((
    ///     Oid::from_str("74ba370ee5643f310873fb288af1c99d639da8ca").unwrap(),
    ///     vec![
    ///         Oid::from_str("8eb5ace23086a588200d9aae1374f46c346bccec").unwrap(),
    ///         Oid::from_str("cd3971c01606a0b1df2f3429aeb5766d234d7893").unwrap(),
    ///     ]
    /// ).into());
    ///
    /// let history_ids = browser.get_history().map(|commit| commit.id());
    ///
    /// // We are able to render the directory
    /// assert_eq!(history_ids, expected_history);
    /// ```
    pub fn tag(&mut self, tag_name: TagName) -> Result<(), GitError> {
        let branch = self.repository.get_history(GitObject::Tag(tag_name))?;
        self.set_history(branch);
        Ok(())
    }

    /// Set the current `GitBrowser` history to the [`Sha1`](struct.Sha1.html)
    /// provided. The history will consist of a single [`Commit`](struct.Commit.html).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::SystemType;
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, Sha1};
    ///
    /// let repo = GitRepository::new("./data/git-golden")
    ///     .expect("Could not retrieve ./data/git-golden as git repository");
    /// let mut browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// // Set to the initial commit
    /// browser
    ///     .commit(Sha1::new("cd3971c01606a0b1df2f3429aeb5766d234d7893"))
    ///     .unwrap();
    ///
    /// let directory = browser.get_directory().unwrap();
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// // We should only have src and .git in our root
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::directory(".git".into()),
    ///         SystemType::directory("src".into()),
    ///     ]
    /// );
    ///
    /// // We have the single commit
    /// assert!(browser.get_history().0.len() == 1);
    /// ```
    pub fn commit(&mut self, sha: Sha1) -> Result<(), GitError> {
        let commit = self.repository.get_commit(sha)?;
        self.set_history(vcs::History(NonEmpty::new(commit)));
        Ok(())
    }

    /// List the names of the branches that are contained in the
    /// underlying [`GitRepository`](struct.GitRepository.hmtl).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let branches = browser.list_branches().unwrap();
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&BranchName::new("master")));
    /// ```
    pub fn list_branches(&self) -> Result<Vec<BranchName>, GitError> {
        self.repository
            .0
            .branches(None)
            .map_err(GitError::from)
            .and_then(|mut branches| {
                branches.try_fold(vec![], |mut acc, branch| {
                    let (branch, _) = branch?;
                    let branch_name = branch.name()?;
                    if let Some(name) = branch_name {
                        acc.push(BranchName(name.to_string()));
                        Ok(acc)
                    } else {
                        Err(GitError::BranchDecode)
                    }
                })
            })
    }

    /// List the names of the tags that are contained in the
    /// underlying [`GitRepository`](struct.GitRepository.hmtl).
    ///
    /// # Examples
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
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, TagName};
    ///
    /// let repo = GitRepository::new("./data/git-golden").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let tags = browser.list_tags().unwrap();
    ///
    /// // We currently have no tags :(
    /// assert_eq!(tags, vec![TagName::new("v0.0.1")]);
    /// ```
    pub fn list_tags(&self) -> Result<Vec<TagName>, GitError> {
        let tags = self.repository.0.tag_names(None)?;
        Ok(tags
            .into_iter()
            .filter_map(|tag| tag.map(TagName::new))
            .collect())
    }

    /// Given a [`Path`](../../file_system/struct.Path.html) to a file, return the last `Commit`
    /// that touched that file.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    ///
    /// let repo = GitRepository::new("./data/git-golden")
    ///     .expect("Could not retrieve ./data/git-test as git repository");
    /// let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    ///
    /// let toml_last_commit = browser
    ///     .last_commit(&Path::with_root(&["Cargo.toml".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(toml_last_commit, Some(head_commit.id()));
    ///
    /// let main_last_commit = browser
    ///     .last_commit(&Path::with_root(&["src".into(), "main.rs".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(main_last_commit, Some(head_commit.id()));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    ///
    /// let repo = GitRepository::new("./data/git-golden")
    ///     .expect("Could not retrieve ./data/git-golden as git repository");
    /// let mut browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// // Set the browser history to the initial commit
    /// browser.commit(Sha1::new("cd3971c01606a0b1df2f3429aeb5766d234d7893")).unwrap();
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    ///
    /// // Cargo.toml is commited second so it should not exist here.
    /// let toml_last_commit = browser
    ///     .last_commit(&Path::with_root(&["Cargo.toml".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(toml_last_commit, None);
    ///
    /// // src/main.rs exists in this commit.
    /// let main_last_commit = browser
    ///     .last_commit(&Path::with_root(&["src".into(), "main.rs".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(main_last_commit, Some(head_commit.id()));
    /// ```
    pub fn last_commit(&self, path: &file_system::Path) -> Option<Commit> {
        self.get_history()
            .find(|commit| self.commit_contains_path(commit.clone(), path))
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

    /// Check that a given `Commit` touches the given `Path`.
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
}
