//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Path, SystemType};
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//!
//! let repo = GitRepository::new("./data/git-platinum")
//!     .expect("Could not retrieve ./data/git-platinum as git repository");
//! let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
//! let directory = browser.get_directory().expect("Could not render Directory");
//! let mut directory_contents = directory.list_directory();
//! directory_contents.sort();
//!
//! assert_eq!(directory_contents, vec![
//!     SystemType::file(".i-am-well-hidden".into()),
//!     SystemType::file(".i-too-am-hidden".into()),
//!     SystemType::file("README.md".into()),
//!     SystemType::directory("bin".into()),
//!     SystemType::directory("src".into()),
//!     SystemType::directory("text".into()),
//!     SystemType::directory("this".into()),
//! ]);
//!
//! // find src directory in the Git directory and the in-memory directory
//! let src_directory = directory
//!     .find_directory(&Path::new("src".into()))
//!     .unwrap();
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file("Eval.hs".into()),
//!     SystemType::file("Folder.svelte".into()),
//!     SystemType::file("memory.rs".into()),
//! ]);
//! ```

// Re-export git2 as sub-module
pub use git2;

use crate::file_system;
use crate::vcs;
use crate::vcs::VCS;
use git2::{BranchType, Commit, Error, Oid, Reference, Repository, TreeWalkMode, TreeWalkResult};
use nonempty::NonEmpty;
use std::cmp::Ordering;
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
    /// use radicle_surf::vcs::git::{Branch, BranchName, GitBrowser, GitRepository};
    ///
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
    /// let browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let mut branches = browser.list_branches(None).unwrap();
    /// branches.sort();
    ///
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::local(BranchName::new("master")),
    ///         Branch::remote(BranchName::new("origin/HEAD")),
    ///         Branch::remote(BranchName::new("origin/dev")),
    ///         Branch::remote(BranchName::new("origin/master")),
    ///     ]
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

/// The combination of a branch's name and where its locality (remote or local).
///
/// **Note**: The `PartialOrd` and `Ord` implementations compare on `BranchName`
/// only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: BranchName,
    pub locality: BranchType,
}

impl PartialOrd for Branch {
    fn partial_cmp(&self, other: &Branch) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Branch {
    fn cmp(&self, other: &Branch) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Branch {
    /// Helper to create a remote `Branch` with a name
    pub fn remote(name: BranchName) -> Self {
        Branch {
            name,
            locality: BranchType::Remote,
        }
    }

    /// Helper to create a remote `Branch` with a name
    pub fn local(name: BranchName) -> Self {
        Branch {
            name,
            locality: BranchType::Local,
        }
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
            name: file_system::Label {
                label: ".git".into(),
                hidden: true,
            },
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
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
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
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
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
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
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
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    /// browser
    ///     .branch(BranchName::new("origin/dev"))
    ///     .expect("Failed to change branch to dev");
    ///
    /// let directory = browser.get_directory().expect("Failed to get directory");
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::file(".i-am-well-hidden".into()),
    ///         SystemType::file(".i-too-am-hidden".into()),
    ///         SystemType::file("README.md".into()),
    ///         SystemType::directory("bin".into()),
    ///         SystemType::file("here-we-are-on-a-dev-branch.lol".into()),
    ///         SystemType::directory("src".into()),
    ///         SystemType::directory("text".into()),
    ///         SystemType::directory("this".into()),
    ///     ]
    /// );
    ///
    /// let tests = directory
    ///     .find_directory(&Path::new("bin".into()))
    ///     .expect("bin not found");
    /// let mut tests_contents = tests.list_directory();
    /// tests_contents.sort();
    ///
    /// assert_eq!(
    ///     tests_contents,
    ///     vec![
    ///         SystemType::file("cat".into()),
    ///         SystemType::file("ls".into()),
    ///         SystemType::file("test".into()),
    ///     ]
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
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// // Switch to "v0.3.0"
    /// browser.tag(TagName::new("v0.3.0")).expect("Failed to switch tag");
    ///
    /// let expected_history = History((
    ///     Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97").unwrap(),
    ///     vec![
    ///         Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap(),
    ///         Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap(),
    ///         Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3").unwrap(),
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
    /// let repo = GitRepository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// // Set to the initial commit
    /// browser
    ///     .commit(Sha1::new("e24124b7538658220b5aaf3b6ef53758f0a106dc"))
    ///     .expect("Missing commit");
    ///
    /// let directory = browser.get_directory().unwrap();
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// // We should only have src in our root
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::file("README.md".into()),
    ///         SystemType::directory("bin".into()),
    ///         SystemType::directory("src".into()),
    ///         SystemType::directory("this".into()),
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
    /// use radicle_surf::vcs::git::{Branch, BranchName, GitBrowser, GitRepository};
    /// use radicle_surf::vcs::git::git2::BranchType;
    ///
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let branches = browser.list_branches(None).unwrap();
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&Branch::local(BranchName::new("master"))));
    ///
    /// // Filter the branches by `Remote`.
    /// let branches = browser.list_branches(Some(BranchType::Remote)).unwrap();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::remote(BranchName::new("origin/HEAD")),
    ///     Branch::remote(BranchName::new("origin/dev")),
    ///     Branch::remote(BranchName::new("origin/master")),
    /// ]);
    /// ```
    pub fn list_branches(&self, filter: Option<BranchType>) -> Result<Vec<Branch>, GitError> {
        self.repository
            .0
            .branches(filter)
            .map_err(GitError::from)
            .and_then(|mut branches| {
                branches.try_fold(vec![], |mut acc, branch| {
                    let (branch, branch_type) = branch?;
                    let branch_name = branch.name()?;
                    if let Some(name) = branch_name {
                        let branch = Branch {
                            name: BranchName(name.to_string()),
                            locality: branch_type,
                        };
                        acc.push(branch);
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
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, TagName};
    ///
    /// let repo = GitRepository::new("./data/git-platinum").unwrap();
    /// let mut browser = GitBrowser::new(&repo).unwrap();
    ///
    /// let tags = browser.list_tags().unwrap();
    ///
    /// // We currently have no tags :(
    ///
    /// assert_eq!(
    ///     tags,
    ///     vec![
    ///         TagName::new("v0.1.0"),
    ///         TagName::new("v0.2.0"),
    ///         TagName::new("v0.3.0"),
    ///         TagName::new("v0.4.0"),
    ///         TagName::new("v0.5.0")
    ///     ]
    /// );
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
    /// let repo = GitRepository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-test as git repository");
    /// let browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    ///
    /// let readme_last_commit = browser
    ///     .last_commit(&Path::with_root(&["README.md".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(readme_last_commit, Some(head_commit.id()));
    ///
    /// let memory_last_commit = browser
    ///     .last_commit(&Path::with_root(&["src".into(), "memory.rs".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(memory_last_commit, Some(head_commit.id()));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{GitBrowser, GitRepository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    ///
    /// let repo = GitRepository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = GitBrowser::new(&repo).expect("Could not initialise Browser");
    ///
    /// // Set the browser history to the initial commit
    /// browser.commit(Sha1::new("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")).unwrap();
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    ///
    /// // memory.rs is commited later so it should not exist here.
    /// let memory_last_commit = browser
    ///     .last_commit(&Path::with_root(&["src".into(), "memory.rs".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(memory_last_commit, None);
    ///
    /// // README.md exists in this commit.
    /// let readme_last_commit = browser
    ///     .last_commit(&Path::with_root(&["README.md".into()]))
    ///     .map(|commit| commit.id());
    ///
    /// assert_eq!(readme_last_commit, Some(head_commit.id()));
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
                        entry.name().and_then(|name| {
                            let file = file_system::File {
                                name: name.into(),
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
        let (directory, name) = path.split_last();
        let commit_tree = commit.tree().ok()?;

        if directory == vec![file_system::Label::root()] {
            commit_tree.get_name(&name.label).map(|_| commit)
        } else {
            let mut directory_path = std::path::PathBuf::new();
            for dir in directory {
                if dir == file_system::Label::root() {
                    continue;
                }

                directory_path.push(dir.label);
            }

            let tree_entry = commit_tree.get_path(directory_path.as_path()).ok()?;
            let object = tree_entry.to_object(&self.repository.0).ok()?;
            let tree = object.as_tree().map(|t| t.get_name(&name.label));
            tree.map(|_| commit)
        }
    }
}
