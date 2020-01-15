//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//!
//! let repo = Repository::new("./data/git-platinum")
//!     .expect("Could not retrieve ./data/git-platinum as git repository");
//! let browser = Browser::new(repo).expect("Could not initialise Browser");
//! let directory = browser.get_directory().expect("Could not render Directory");
//! let mut directory_contents = directory.list_directory();
//! directory_contents.sort();
//!
//! assert_eq!(directory_contents, vec![
//!     SystemType::file(unsound::label::new(".i-am-well-hidden")),
//!     SystemType::file(unsound::label::new(".i-too-am-hidden")),
//!     SystemType::file(unsound::label::new("README.md")),
//!     SystemType::directory(unsound::label::new("bin")),
//!     SystemType::directory(unsound::label::new("src")),
//!     SystemType::directory(unsound::label::new("text")),
//!     SystemType::directory(unsound::label::new("this")),
//! ]);
//!
//! // find src directory in the Git directory and the in-memory directory
//! let src_directory = directory
//!     .find_directory(&Path::new(unsound::label::new("src")))
//!     .unwrap();
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! ```

// Re-export git2 as sub-module
pub use git2;
pub use git2::{BranchType, Error as Git2Error, Oid, Time};

pub mod error;

use crate::file_system;
use crate::tree::*;
use crate::vcs;
use crate::vcs::git::error::*;
use crate::vcs::VCS;
use nonempty::NonEmpty;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::str;

#[derive(Clone)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub time: git2::Time,
}

impl<'repo> TryFrom<git2::Signature<'repo>> for Signature {
    type Error = Error;

    fn try_from(signature: git2::Signature) -> Result<Self, Self::Error> {
        let name = str::from_utf8(signature.name_bytes())?.into();
        let email = str::from_utf8(signature.email_bytes())?.into();
        let time = signature.when();

        Ok(Signature { name, email, time })
    }
}

#[derive(Clone)]
pub struct Commit {
    pub id: git2::Oid,
    pub author: Signature,
    pub committer: Signature,
    pub message: String,
    pub summary: String,
}

impl<'repo> TryFrom<git2::Commit<'repo>> for Commit {
    type Error = Error;

    fn try_from(commit: git2::Commit) -> Result<Self, Self::Error> {
        let id = commit.id();
        let author = Signature::try_from(commit.author())?;
        let committer = Signature::try_from(commit.committer())?;
        let message_raw = commit.message_bytes();
        let message = str::from_utf8(message_raw)?.into();
        let summary_raw = commit.summary_bytes().expect("TODO");
        let summary = str::from_utf8(summary_raw)?.into();

        Ok(Commit {
            id,
            author,
            committer,
            message,
            summary,
        })
    }
}

/// A `History` that uses `git2::Commit` as the underlying artifact.
pub type History = vcs::History<Commit>;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct Repository(pub(crate) git2::Repository);

#[derive(Clone)]
struct OrderedCommit {
    id: usize,
    commit: Commit,
}

impl std::fmt::Debug for OrderedCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrderedCommit {{ id: {}, commit: {} }}",
            self.id, self.commit.id
        )
    }
}

impl OrderedCommit {
    fn compare_by_id(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id).reverse()
    }
}

impl From<OrderedCommit> for Commit {
    fn from(ordered_commit: OrderedCommit) -> Self {
        ordered_commit.commit
    }
}

impl<'repo> Repository {
    /// Open a git repository given its URI.
    ///
    /// # Examples
    /// ```
    /// use radicle_surf::vcs::git::{Branch, BranchName, Browser, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let browser = Browser::new(repo).unwrap();
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
    pub fn new(repo_uri: &str) -> Result<Self, Error> {
        git2::Repository::open(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// Get a particular `Commit`.
    pub(crate) fn get_commit(&'repo self, sha: Sha1) -> Result<git2::Commit<'repo>, Error> {
        let oid = git2::Oid::from_str(&sha.0)?;
        let commit = self.0.find_commit(oid)?;
        Ok(commit)
    }

    /// Build a `History` using the `head` reference.
    pub(crate) fn head(&'repo self) -> Result<History, Error> {
        let head = self.0.head()?;
        self.to_history(&head)
    }

    /// Turn a `git2::Reference` into a `History` by completing
    /// a revwalk over the first commit in the reference.
    pub(crate) fn to_history(
        &'repo self,
        history: &git2::Reference<'repo>,
    ) -> Result<History, Error> {
        let head = history.peel_to_commit()?;
        let mut commits = Vec::new();
        let mut revwalk = self.0.revwalk()?;

        // Set the revwalk to the head commit
        revwalk.push(head.id())?;

        for commit_result_id in revwalk {
            // The revwalk iter returns results so
            // we unpack these and push them to the history
            let commit_id: git2::Oid = commit_result_id?;
            let commit = Commit::try_from(self.0.find_commit(commit_id)?)?;
            commits.push(commit);
        }

        NonEmpty::from_slice(&commits)
            .map(vcs::History)
            .ok_or(Error::EmptyCommitHistory)
    }

    fn file_history(
        &'repo self,
        commit: Commit,
    ) -> Result<Forest<file_system::Label, NonEmpty<OrderedCommit>>, Error> {
        let mut file_histories = Forest::root();
        self.collect_file_history(&commit.id, &mut file_histories)?;
        Ok(file_histories)
    }

    fn collect_file_history(
        &'repo self,
        commit_id: &git2::Oid,
        file_histories: &mut Forest<file_system::Label, NonEmpty<OrderedCommit>>,
    ) -> Result<(), Error> {
        let mut revwalk = self.0.revwalk()?;

        // Set the revwalk to the head commit
        revwalk.push(commit_id.clone())?;

        for (id, commit_result) in revwalk.enumerate() {
            let parent_id = commit_result?;

            let parent = self.0.find_commit(parent_id)?;
            let paths = self.diff_commit_and_parents(&parent)?;
            let parent_commit = Commit::try_from(parent)?;
            for path in paths {
                let parent_commit = OrderedCommit {
                    id,
                    commit: parent_commit.clone(),
                };

                file_histories.insert_with(
                    path.0,
                    NonEmpty::new(parent_commit.clone()),
                    |commits| commits.push(parent_commit),
                );
            }
        }
        Ok(())
    }

    fn diff_commit_and_parents(
        &'repo self,
        commit: &'repo git2::Commit,
    ) -> Result<Vec<file_system::Path>, Error> {
        let mut parents = commit.parents();
        let head = parents.next();
        let mut touched_files = vec![];

        let mut add_deltas = |diff: git2::Diff| -> Result<(), Error> {
            let deltas = diff.deltas();

            for delta in deltas {
                let new = delta.new_file().path().ok_or(Error::FileDiffException)?;
                let path = file_system::Path::try_from(new.to_path_buf())?;
                touched_files.push(path);
            }

            Ok(())
        };

        match head {
            None => {
                let diff = self.diff_commits(&commit, None)?;
                add_deltas(diff)?;
            }
            Some(parent) => {
                let diff = self.diff_commits(&commit, Some(&parent))?;
                add_deltas(diff)?;

                for parent in parents {
                    let diff = self.diff_commits(&commit, Some(&parent))?;
                    add_deltas(diff)?;
                }
            }
        }

        Ok(touched_files)
    }

    fn diff_commits(
        &'repo self,
        left: &'repo git2::Commit,
        right: Option<&'repo git2::Commit>,
    ) -> Result<git2::Diff, Error> {
        let left_tree = left.tree()?;
        let right_tree = right.map_or(Ok(None), |commit| commit.tree().map(Some))?;

        let diff = self
            .0
            .diff_tree_to_tree(Some(&left_tree), right_tree.as_ref(), None)?;

        Ok(diff)
    }
}

impl vcs::GetVCS<Error> for Repository {
    type RepoId = String;

    fn get_repo(repo_id: Self::RepoId) -> Result<Self, Error> {
        git2::Repository::open(&repo_id)
            .map(Repository)
            .map_err(Error::from)
    }
}

impl From<git2::Repository> for Repository {
    fn from(repo: git2::Repository) -> Self {
        Repository(repo)
    }
}

/// The combination of a branch's name and where its locality (remote or local).
///
/// **Note**: The `PartialOrd` and `Ord` implementations compare on `BranchName`
/// only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: BranchName,
    pub locality: git2::BranchType,
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
            locality: git2::BranchType::Remote,
        }
    }

    /// Helper to create a remote `Branch` with a name
    pub fn local(name: BranchName) -> Self {
        Branch {
            name,
            locality: git2::BranchType::Local,
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
/// into a [`History`](struct.History.html).
#[derive(Debug, Clone)]
pub enum Object {
    Branch(BranchName),
    Tag(TagName),
}

impl Object {
    pub fn branch(name: &str) -> Self {
        Object::Branch(BranchName::new(name))
    }

    pub fn tag(name: &str) -> Self {
        Object::Tag(TagName::new(name))
    }

    fn get_name(&self) -> String {
        match self {
            Object::Branch(name) => name.0.clone(),
            Object::Tag(name) => name.0.clone(),
        }
    }
}

impl vcs::VCS<Commit, Error> for Repository {
    type HistoryId = Object;
    type ArtefactId = git2::Oid;

    fn get_history(&self, history_id: Self::HistoryId) -> Result<History, Error> {
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
            Object::Branch(_) => to_history(
                reference.is_branch() || reference.is_remote(),
                Error::NotBranch,
            ),
            Object::Tag(_) => to_history(reference.is_tag(), Error::NotTag),
        }
    }

    fn get_histories(&self) -> Result<Vec<History>, Error> {
        self.0
            .references()
            .map_err(Error::from)
            .and_then(|mut references| {
                references.try_fold(vec![], |mut acc, reference| {
                    reference.map_err(Error::from).and_then(|r| {
                        let history = self.to_history(&r)?;
                        acc.push(history);
                        Ok(acc)
                    })
                })
            })
    }

    fn get_identifier(artifact: &Commit) -> Self::ArtefactId {
        artifact.id
    }
}

impl file_system::RepoBackend for Repository {
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

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

/// A `Browser` that uses [`Repository`](struct.Repository.html) as the underlying repository backend,
/// `git2::Commit` as the artifact, and [`Error`](enum.Error.html) for error reporting.
pub type Browser = vcs::Browser<Repository, Commit, Error>;

impl Browser {
    /// Create a new browser to interact with.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let browser = Browser::new(repo).unwrap();
    /// ```
    pub fn new(repository: Repository) -> Result<Self, Error> {
        let history = repository.head()?;
        let snapshot = Box::new(|repository: &Repository, history: &History| {
            let tree = Self::get_tree(&repository.0, history.0.first())?;
            Ok(file_system::Directory::from::<Repository>(tree))
        });
        Ok(vcs::Browser {
            snapshot,
            history,
            repository,
        })
    }

    /// Set the current `Browser` history to the `HEAD` commit of the underlying repository.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
    ///
    /// // ensure we're at HEAD
    /// browser.head();
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// ```
    pub fn head(&mut self) -> Result<(), Error> {
        let history = self.repository.head()?;
        self.set_history(history);
        Ok(())
    }

    /// Set the current `Browser` history to the [`BranchName`](struct.BranchName.html)
    /// provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
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
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
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
    ///         SystemType::file(unsound::label::new(".i-am-well-hidden")),
    ///         SystemType::file(unsound::label::new(".i-too-am-hidden")),
    ///         SystemType::file(unsound::label::new("README.md")),
    ///         SystemType::directory(unsound::label::new("bin")),
    ///         SystemType::file(unsound::label::new("here-we-are-on-a-dev-branch.lol")),
    ///         SystemType::directory(unsound::label::new("src")),
    ///         SystemType::directory(unsound::label::new("text")),
    ///         SystemType::directory(unsound::label::new("this")),
    ///     ]
    /// );
    ///
    /// let tests = directory
    ///     .find_directory(&Path::new(unsound::label::new("bin")))
    ///     .expect("bin not found");
    /// let mut tests_contents = tests.list_directory();
    /// tests_contents.sort();
    ///
    /// assert_eq!(
    ///     tests_contents,
    ///     vec![
    ///         SystemType::file(unsound::label::new("cat")),
    ///         SystemType::file(unsound::label::new("ls")),
    ///         SystemType::file(unsound::label::new("test")),
    ///     ]
    /// );
    /// ```
    pub fn branch(&mut self, branch_name: BranchName) -> Result<(), Error> {
        let branch = self.repository.get_history(Object::Branch(branch_name))?;
        self.set_history(branch);
        Ok(())
    }

    /// Set the current `Browser` history to the [`TagName`](struct.TagName.html)
    /// provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::History;
    /// use radicle_surf::vcs::git::{TagName, Browser, Oid, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
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
    /// let history_ids = browser.get_history().map(|commit| commit.id);
    ///
    /// // We are able to render the directory
    /// assert_eq!(history_ids, expected_history);
    /// ```
    pub fn tag(&mut self, tag_name: TagName) -> Result<(), Error> {
        let branch = self.repository.get_history(Object::Tag(tag_name))?;
        self.set_history(branch);
        Ok(())
    }

    /// Set the current `Browser` history to the [`Sha1`](struct.Sha1.html)
    /// provided. The history will consist of a single [`Commit`](struct.Commit.html).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, SystemType};
    /// use radicle_surf::vcs::git::{Browser, Repository, Sha1};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
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
    ///         SystemType::file(unsound::label::new("README.md")),
    ///         SystemType::directory(unsound::label::new("bin")),
    ///         SystemType::directory(unsound::label::new("src")),
    ///         SystemType::directory(unsound::label::new("this")),
    ///     ]
    /// );
    ///
    /// // We have the single commit
    /// assert!(browser.get_history().0.len() == 1);
    /// ```
    pub fn commit(&mut self, sha: Sha1) -> Result<(), Error> {
        let commit = Commit::try_from(self.repository.get_commit(sha)?)?;
        self.set_history(vcs::History(NonEmpty::new(commit)));
        Ok(())
    }

    /// List the names of the branches that are contained in the
    /// underlying [`Repository`](struct.Repository.hmtl).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, BranchType, BranchName, Browser, Repository};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
    ///
    /// let branches = browser.list_branches(None).unwrap();
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&Branch::local(BranchName::new("master"))));
    ///
    /// // Filter the branches by `Remote`.
    /// let mut branches = browser.list_branches(Some(BranchType::Remote)).unwrap();
    /// branches.sort();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::remote(BranchName::new("origin/HEAD")),
    ///     Branch::remote(BranchName::new("origin/dev")),
    ///     Branch::remote(BranchName::new("origin/master")),
    /// ]);
    /// ```
    pub fn list_branches(&self, filter: Option<git2::BranchType>) -> Result<Vec<Branch>, Error> {
        self.repository
            .0
            .branches(filter)
            .map_err(Error::from)
            .and_then(|mut branches| {
                branches.try_fold(vec![], |mut acc, branch| {
                    let (branch, branch_type) = branch?;
                    let name = str::from_utf8(branch.name_bytes()?)?;
                    let branch = Branch {
                        name: BranchName(name.to_string()),
                        locality: branch_type,
                    };
                    acc.push(branch);
                    Ok(acc)
                })
            })
    }

    /// List the names of the tags that are contained in the
    /// underlying [`Repository`](struct.Repository.hmtl).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, TagName};
    ///
    /// let repo = Repository::new("./data/git-platinum").unwrap();
    /// let mut browser = Browser::new(repo).unwrap();
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
    pub fn list_tags(&self) -> Result<Vec<TagName>, Error> {
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
    /// use radicle_surf::vcs::git::{Browser, Repository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// use git2;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-test as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
    ///
    /// // Clamp the Browser to a particular commit
    /// browser.commit(Sha1::new("d6880352fc7fda8f521ae9b7357668b17bb5bad5")).expect("Failed to set
    /// commit");
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    /// let expected_commit = git2::Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")
    ///     .expect("Failed to create Oid");
    ///
    /// let readme_last_commit = browser
    ///     .last_commit(&Path::with_root(&[unsound::label::new("README.md")]))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(readme_last_commit, Some(expected_commit));
    ///
    /// let expected_commit = git2::Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")
    ///     .expect("Failed to create Oid");
    ///
    /// let memory_last_commit = browser
    ///     .last_commit(&Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(memory_last_commit, Some(expected_commit));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
    ///
    /// // Set the browser history to the initial commit
    /// browser.commit(Sha1::new("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")).unwrap();
    ///
    /// let head_commit = browser.get_history().0.first().clone();
    ///
    /// // memory.rs is commited later so it should not exist here.
    /// let memory_last_commit = browser
    ///     .last_commit(&Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(memory_last_commit, None);
    ///
    /// // README.md exists in this commit.
    /// let readme_last_commit = browser
    ///     .last_commit(&Path::with_root(&[unsound::label::new("README.md")]))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(readme_last_commit, Some(head_commit.id));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Oid, Repository, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
    ///
    /// // Check that last commit is the actual last commit even if head commit differs.
    /// browser.commit(Sha1::new("19bec071db6474af89c866a1bd0e4b1ff76e2b97")).unwrap();
    ///
    /// let expected_commit_id =
    ///     Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap();
    ///
    /// let gitignore_last_commit_id = browser
    ///     .last_commit(&unsound::path::new("~/examples/Folder.svelte"))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(gitignore_last_commit_id, Some(expected_commit_id));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository, Sha1};
    /// use radicle_surf::vcs::git::git2::{Oid};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
    ///
    /// // Check that last commit is the actual last commit even if head commit differs.
    /// browser.commit(Sha1::new("19bec071db6474af89c866a1bd0e4b1ff76e2b97")).unwrap();
    ///
    /// let expected_commit_id =
    ///     Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap();
    ///
    /// let nested_directory_tree_commit_id = browser
    ///     .last_commit(&unsound::path::new("~/this/is/a/really/deeply/nested/directory/tree"))
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(nested_directory_tree_commit_id, Some(expected_commit_id));
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository, Oid, Sha1};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Repository::new("./data/git-platinum")
    ///     .expect("Could not retrieve ./data/git-platinum as git repository");
    /// let mut browser = Browser::new(repo).expect("Could not initialise Browser");
    ///
    /// let expected_commit_id =
    ///     Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95").unwrap();
    ///
    /// let root_last_commit_id = browser
    ///     .last_commit(&Path::root())
    ///     .expect("Failed to get last commit")
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(root_last_commit_id, Some(expected_commit_id));
    pub fn last_commit(&self, path: &file_system::Path) -> Result<Option<Commit>, Error> {
        let file_history = self
            .repository
            .file_history(self.get_history().first().clone())?;

        Ok(file_history.find(&path.0).map(|tree| {
            tree.maximum_by(&|c: &NonEmpty<OrderedCommit>, d| c.first().compare_by_id(&d.first()))
                .first()
                .commit
                .clone()
        }))
    }

    /// Do a pre-order TreeWalk of the given commit. This turns a Tree
    /// into a HashMap of Paths and a list of Files. We can then turn that
    /// into a Directory.
    fn get_tree(
        repo: &git2::Repository,
        commit: &Commit,
    ) -> Result<HashMap<file_system::Path, NonEmpty<file_system::File>>, Error> {
        let mut file_paths_or_error: Result<
            HashMap<file_system::Path, NonEmpty<file_system::File>>,
            Error,
        > = Ok(HashMap::new());

        let commit = repo.find_commit(commit.id)?;
        let tree = commit.as_object().peel_to_tree()?;

        tree.walk(
            git2::TreeWalkMode::PreOrder,
            |s, entry| match Self::tree_entry_to_file_and_path(repo, s, entry) {
                Ok((path, file)) => {
                    match file_paths_or_error.as_mut() {
                        Ok(mut files) => Self::update_file_map(path, file, &mut files),

                        // We don't need to update, we want to keep the error.
                        Err(_err) => {}
                    }
                    git2::TreeWalkResult::Ok
                }
                Err(err) => match err {
                    // We want to continue if the entry was not a Blob.
                    TreeWalkError::NotBlob => git2::TreeWalkResult::Ok,

                    // But we want to keep the error and abort otherwise.
                    TreeWalkError::Git(err) => {
                        file_paths_or_error = Err(err);
                        git2::TreeWalkResult::Abort
                    }
                },
            },
        )?;

        file_paths_or_error
    }

    fn update_file_map(
        path: file_system::Path,
        file: file_system::File,
        files: &mut HashMap<file_system::Path, NonEmpty<file_system::File>>,
    ) {
        files
            .entry(path)
            .and_modify(|entries| entries.push(file.clone()))
            .or_insert_with(|| NonEmpty::new(file));
    }

    fn tree_entry_to_file_and_path(
        repo: &git2::Repository,
        tree_path: &str,
        entry: &git2::TreeEntry,
    ) -> Result<(file_system::Path, file_system::File), TreeWalkError> {
        // Account for the "root" of git being the empty string
        let path = if tree_path.is_empty() {
            Ok(file_system::Path::root())
        } else {
            file_system::Path::try_from(tree_path)
        }?;

        let object = entry.to_object(repo)?;
        let blob = object.as_blob().ok_or(TreeWalkError::NotBlob)?;
        let name = str::from_utf8(entry.name_bytes())?;

        let name = file_system::Label::try_from(name).map_err(Error::FileSystem)?;

        Ok((
            path,
            file_system::File {
                name,
                contents: blob.content().to_owned(),
                size: blob.size(),
            },
        ))
    }
}
