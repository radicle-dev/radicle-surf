// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! ```
//! use nonempty::NonEmpty;
//! use radicle_surf::file_system::{Directory, File, Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use radicle_surf::vcs::git::*;
//! use std::collections::HashMap;
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let repo = Repository::new("./data/git-platinum")?;
//!
//! // Pin the browser to a parituclar commit.
//! let pin_commit = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
//! let mut browser = Browser::new(repo)?;
//! browser.commit(pin_commit)?;
//!
//! let directory = browser.get_directory()?;
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
//!     .find_directory(Path::new(unsound::label::new("src")))
//!     .expect("failed to find src");
//! let mut src_directory_contents = src_directory.list_directory();
//! src_directory_contents.sort();
//!
//! assert_eq!(src_directory_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! #
//! # Ok(())
//! # }
//! ```

// Re-export git2 as sub-module
pub use git2::{self, BranchType, Error as Git2Error, Oid, Time};

pub mod error;
mod object;

pub use crate::vcs::git::object::*;
use crate::{
    diff::*,
    file_system,
    file_system::directory,
    tree::*,
    vcs,
    vcs::{git::error::*, VCS},
};
use nonempty::NonEmpty;
use std::{cmp::Ordering, collections::HashMap, convert::TryFrom, str};

/// A `History` that uses `git2::Commit` as the underlying artifact.
pub type History = vcs::History<Commit>;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct Repository(pub(crate) git2::Repository);

/// OrderedCommit is to allow for us to identify an ordering of commit history
/// as we enumerate over a revwalk of commits, by assigning each commit an
/// identifier.
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

impl<'a> From<git2::DiffLine<'a>> for LineDiff {
    fn from(line: git2::DiffLine) -> Self {
        match (line.old_lineno(), line.new_lineno()) {
            (None, Some(n)) => Self::addition(line.content().to_owned(), n),
            (Some(n), None) => Self::deletion(line.content().to_owned(), n),
            (Some(_), Some(n)) => Self::context(line.content().to_owned(), n),
            (None, None) => unreachable!(),
        }
    }
}

impl<'repo> Repository {
    /// Open a git repository given its URI.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    pub fn new(repo_uri: &str) -> Result<Self, Error> {
        git2::Repository::open(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// List the branches within a repository, filtering out ones that do not
    /// parse correctly.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    pub fn list_branches(&self, filter: Option<BranchType>) -> Result<Vec<Branch>, Error> {
        self.0
            .branches(filter)
            .map_err(Error::from)
            .and_then(|mut branches| {
                branches.try_fold(vec![], |mut acc, branch| {
                    let (branch, branch_type) = branch?;
                    let name = BranchName::try_from(branch.name_bytes()?)?;
                    let branch = Branch {
                        name,
                        locality: branch_type,
                    };
                    acc.push(branch);
                    Ok(acc)
                })
            })
    }

    /// List the tags within a repository, filtering out ones that do not parse
    /// correctly.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    pub fn list_tags(&self) -> Result<Vec<TagName>, Error> {
        let tags = self.0.tag_names(None)?;
        Ok(tags
            .into_iter()
            .filter_map(|tag| tag.map(TagName::new))
            .collect())
    }

    /// Create a [`RevObject`] given a
    /// [`revspec`](https://git-scm.com/docs/git-rev-parse#_specifying_revisions) string.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::RevParseFailure`]
    pub fn rev(&self, spec: &str) -> Result<RevObject, Error> {
        RevObject::from_revparse(&self.0, spec)
    }

    /// Create a [`History`] given a
    /// [`revspec`](https://git-scm.com/docs/git-rev-parse#_specifying_revisions) string.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::RevParseFailure`]
    pub fn revspec(&self, spec: &str) -> Result<History, Error> {
        let rev = self.rev(spec)?;
        let commit = rev.into_commit(&self.0)?;
        self.commit_to_history(commit)
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: &'repo git2::Commit, to: &'repo git2::Commit) -> Result<Diff, Error> {
        use git2::{Delta, Patch};

        let mut diff = Diff::new();
        let git_diff = self.diff_commits(from, Some(to))?;

        for (idx, delta) in git_diff.deltas().enumerate() {
            match delta.status() {
                Delta::Added => {
                    let diff_file = delta.new_file();
                    let path = diff_file.path().expect("file should have a path");
                    let path = file_system::Path::try_from(path.to_path_buf()).unwrap();

                    diff.add_created_file(path);
                },
                Delta::Deleted => {
                    let diff_file = delta.old_file();
                    let path = diff_file.path().expect("file should have a path");
                    let path = file_system::Path::try_from(path.to_path_buf()).unwrap();

                    diff.add_deleted_file(path);
                },
                Delta::Modified => {
                    let diff_file = delta.new_file();
                    let path = diff_file.path().expect("file should have a path");
                    let path = file_system::Path::try_from(path.to_path_buf()).unwrap();

                    let patch = Patch::from_diff(&git_diff, idx)?;

                    if let Some(patch) = patch {
                        let mut hunks: Vec<Hunk> = Vec::new();

                        for h in 0..patch.num_hunks() {
                            let (hunk, hunk_lines) = patch.hunk(h)?;
                            let header = hunk.header().to_owned();
                            let mut lines: Vec<LineDiff> = Vec::new();

                            for l in 0..hunk_lines {
                                let line = patch.line_in_hunk(h, l)?;
                                lines.push(line.into());
                            }
                            hunks.push(Hunk { header, lines });
                        }
                        diff.add_modified_file(path, hunks);
                    } else if diff_file.is_binary() {
                        // TODO
                    } else {
                        unreachable!()
                    }
                },
                _ => {},
            }
            //
        }

        Ok(diff)
    }

    /// Get a particular `Commit`.
    pub(crate) fn get_commit(&'repo self, oid: Oid) -> Result<git2::Commit<'repo>, Error> {
        let commit = self.0.find_commit(oid)?;
        Ok(commit)
    }

    /// Build a [`History`] using the `head` reference.
    pub(crate) fn head(&'repo self) -> Result<History, Error> {
        let head = self.0.head()?;
        self.to_history(&head)
    }

    /// Turn a [`git2::Reference`] into a [`History`] by completing
    /// a revwalk over the first commit in the reference.
    pub(crate) fn to_history(
        &'repo self,
        history: &git2::Reference<'repo>,
    ) -> Result<History, Error> {
        let head = history.peel_to_commit()?;
        self.commit_to_history(head)
    }

    /// Turn a [`git2::Reference`] into a [`History`] by completing
    /// a revwalk over the first commit in the reference.
    pub(crate) fn commit_to_history(&'repo self, head: git2::Commit) -> Result<History, Error> {
        let head_id = head.id();
        let mut commits = NonEmpty::new(Commit::try_from(head)?);
        let mut revwalk = self.0.revwalk()?;

        // Set the revwalk to the head commit
        revwalk.push(head_id)?;

        for commit_result_id in revwalk {
            // The revwalk iter returns results so
            // we unpack these and push them to the history
            let commit_id: Oid = commit_result_id?;

            // Skip the head commit since we have processed it
            if commit_id == head_id {
                continue;
            }

            let commit = Commit::try_from(self.0.find_commit(commit_id)?)?;
            commits.push(commit);
        }

        Ok(vcs::History(commits))
    }

    /// Extract the signature from a commit
    ///
    /// # Arguments
    ///
    /// `commit_oid` - The object ID of the commit
    /// `field` - the name of the header field containing the signature block;
    ///           pass `None` to extract the default 'gpgsig'
    pub(crate) fn extract_signature(
        &'repo self,
        commit_oid: &Oid,
        field: Option<&str>,
    ) -> Result<Option<Signature>, Error> {
        // Match is necessary here because according to the documentation for
        // git_commit_extract_signature at
        // https://libgit2.org/libgit2/#HEAD/group/commit/git_commit_extract_signature
        // the return value for a commit without a signature will be GIT_ENOTFOUND
        match self.0.extract_signature(commit_oid, field) {
            Err(error) => {
                if error.code() == git2::ErrorCode::NotFound {
                    Ok(None)
                } else {
                    Err(error.into())
                }
            },
            Ok(sig) => Ok(Some(Signature::from_buf(sig.0))),
        }
    }

    /// Get the history of the file system where the head of the [`NonEmpty`] is
    /// the latest commit.
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
        commit_id: &Oid,
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
                let new = delta.new_file().path().ok_or(Error::LastCommitException)?;
                let path = file_system::Path::try_from(new.to_path_buf())?;
                touched_files.push(path);
            }

            Ok(())
        };

        match head {
            None => {
                let diff = self.diff_commits(&commit, None)?;
                add_deltas(diff)?;
            },
            Some(parent) => {
                let diff = self.diff_commits(&commit, Some(&parent))?;
                add_deltas(diff)?;

                for parent in parents {
                    let diff = self.diff_commits(&commit, Some(&parent))?;
                    add_deltas(diff)?;
                }
            },
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

impl VCS<Commit, Error> for Repository {
    type HistoryId = String;
    type ArtefactId = Oid;

    fn get_history(&self, history_id: Self::HistoryId) -> Result<History, Error> {
        self.revspec(&history_id)
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

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

/// A [`crate::vcs::Browser`] that uses [`Repository`] as the underlying
/// repository backend, [`git2::Commit`] as the artifact, and [`Error`] for
/// error reporting.
pub type Browser = vcs::Browser<Repository, Commit, Error>;

impl Browser {
    /// Create a new browser to interact with.
    ///
    /// It uses the current `HEAD` as the starting [`History`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let browser = Browser::new(repo)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(repository: Repository) -> Result<Self, Error> {
        let history = repository.head()?;
        let snapshot = Box::new(|repository: &Repository, history: &History| {
            let tree = Self::get_tree(&repository.0, history.0.first())?;
            Ok(directory::Directory::from_hash_map(tree))
        });
        Ok(vcs::Browser {
            snapshot,
            history,
            repository,
        })
    }

    /// Create a new browser to interact with.
    ///
    /// It uses the branch supplied as the starting `History`.
    /// If the branch does not exist an error will be returned.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let first_branch = repo
    ///     .list_branches(None)?
    ///     .first()
    ///     .cloned()
    ///     .expect("failed to get first branch");
    /// let browser = Browser::new_with_branch(repo, first_branch.name)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_branch(repository: Repository, branch_name: BranchName) -> Result<Self, Error> {
        let history = repository.get_history(branch_name.name().to_string())?;
        let snapshot = Box::new(|repository: &Repository, history: &History| {
            let tree = Self::get_tree(&repository.0, history.0.first())?;
            Ok(directory::Directory::from_hash_map(tree))
        });
        Ok(vcs::Browser {
            snapshot,
            history,
            repository,
        })
    }

    /// Set the current `Browser` history to the `HEAD` commit of the underlying
    /// repository.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// // ensure we're at HEAD
    /// browser.head();
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn head(&mut self) -> Result<(), Error> {
        let history = self.repository.head()?;
        self.set(history);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the given [`BranchName`]
    /// provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::NotBranch`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// // ensure we're on 'master'
    /// browser.branch(BranchName::new("master"));
    ///
    /// let directory = browser.get_directory();
    ///
    /// // We are able to render the directory
    /// assert!(directory.is_ok());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```
    /// use radicle_surf::vcs::git::{BranchName, Browser, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    /// browser.branch(BranchName::new("origin/dev"))?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert!(directory_contents.contains(
    ///     &SystemType::file(unsound::label::new("here-we-are-on-a-dev-branch.lol"))
    /// ));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn branch(&mut self, branch_name: BranchName) -> Result<(), Error> {
        let name = branch_name.name();
        let is_branch = self
            .repository
            .0
            .resolve_reference_from_short_name(name)
            .map(|reference| reference.is_branch() || reference.is_remote())?;

        if !is_branch {
            return Err(Error::NotBranch(branch_name));
        }

        let branch = self.get_history(name.to_string())?;
        self.set(branch);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the [`TagName`] provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::NotTag`]
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::vcs::History;
    /// use radicle_surf::vcs::git::{TagName, Browser, Oid, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// // Switch to "v0.3.0"
    /// browser.tag(TagName::new("v0.3.0"))?;
    ///
    /// let expected_history = History(NonEmpty::from((
    ///     Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")?,
    ///     vec![
    ///         Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2")?,
    ///         Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///         Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///     ]
    /// )));
    ///
    /// let history_ids = browser.get().map(|commit| commit.id);
    ///
    /// // We are able to render the directory
    /// assert_eq!(history_ids, expected_history);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn tag(&mut self, tag_name: TagName) -> Result<(), Error> {
        let name = tag_name.name();

        if !self
            .repository
            .0
            .resolve_reference_from_short_name(name)?
            .is_tag()
        {
            return Err(Error::NotTag(tag_name));
        }

        let tag = self.get_history(name.to_string())?;
        self.set(tag);
        Ok(())
    }

    /// Set the current `Browser`'s [`History`] to the [`Oid`] (SHA digest)
    /// provided.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use radicle_surf::vcs::git::{Browser, Oid, Repository};
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// // Set to the initial commit
    /// let commit = Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?;
    ///
    /// browser.commit(commit)?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    ///
    /// assert_eq!(
    ///     directory_contents,
    ///     vec![
    ///         SystemType::file(unsound::label::new("README.md")),
    ///         SystemType::directory(unsound::label::new("bin")),
    ///         SystemType::directory(unsound::label::new("src")),
    ///         SystemType::directory(unsound::label::new("this")),
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn commit(&mut self, oid: Oid) -> Result<(), Error> {
        let commit = self.repository.get_commit(oid)?;
        let history = self.repository.commit_to_history(commit)?;
        self.set(history);
        Ok(())
    }

    /// Set a `Browser`'s [`History`] based on a [revspec](https://git-scm.com/docs/git-rev-parse.html#_specifying_revisions).
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::RevParseFailure`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use radicle_surf::vcs::git::{Browser, Oid, Repository};
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// browser.revspec("refs/remotes/origin/dev")?;
    ///
    /// let directory = browser.get_directory()?;
    /// let mut directory_contents = directory.list_directory();
    /// directory_contents.sort();
    ///
    /// assert!(directory_contents.contains(
    ///     &SystemType::file(unsound::label::new("here-we-are-on-a-dev-branch.lol"))
    /// ));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn revspec(&mut self, spec: &str) -> Result<(), Error> {
        let history = self.get_history(spec.to_string())?;
        self.set(history);
        Ok(())
    }

    /// Set a `Browser`'s `History` based on a [`RevObject`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// This is useful if you already have a [`RevObject`], but
    /// [`revspec`](#method.revspec) would be a more common function to use.
    pub fn rev(&mut self, rev: RevObject) -> Result<(), Error> {
        let repository = &self.repository;
        let commit = rev.into_commit(&repository.0)?;
        let history = repository.commit_to_history(commit)?;
        self.set(history);
        Ok(())
    }

    /// List the names of the _branches_ that are contained in the underlying
    /// [`Repository`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Branch, BranchType, BranchName, Browser, Repository};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// let branches = browser.list_branches(None)?;
    ///
    /// // 'master' exists in the list of branches
    /// assert!(branches.contains(&Branch::local(BranchName::new("master"))));
    ///
    /// // Filter the branches by `Remote`.
    /// let mut branches = browser.list_branches(Some(BranchType::Remote))?;
    /// branches.sort();
    ///
    /// assert_eq!(branches, vec![
    ///     Branch::remote(BranchName::new("origin/HEAD")),
    ///     Branch::remote(BranchName::new("origin/dev")),
    ///     Branch::remote(BranchName::new("origin/master")),
    /// ]);
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_branches(&self, filter: Option<BranchType>) -> Result<Vec<Branch>, Error> {
        self.repository.list_branches(filter)
    }

    /// List the names of the _tags_ that are contained in the underlying
    /// [`Repository`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, TagName};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// let tags = browser.list_tags()?;
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
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_tags(&self) -> Result<Vec<TagName>, Error> {
        self.repository.list_tags()
    }

    /// Given a [`crate::file_system::Path`] to a file, return the last
    /// [`Commit`] that touched that file or directory.
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    /// * [`error::Error::LastCommitException`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Oid, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// // Clamp the Browser to a particular commit
    /// let commit = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
    /// browser.commit(commit)?;
    ///
    /// let head_commit = browser.get().first().clone();
    /// let expected_commit = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?;
    ///
    /// let readme_last_commit = browser
    ///     .last_commit(Path::with_root(&[unsound::label::new("README.md")]))?
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(readme_last_commit, Some(expected_commit));
    ///
    /// let expected_commit = Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?;
    ///
    /// let memory_last_commit = browser
    ///     .last_commit(Path::with_root(&[unsound::label::new("src"), unsound::label::new("memory.rs")]))?
    ///     .map(|commit| commit.id);
    ///
    /// assert_eq!(memory_last_commit, Some(expected_commit));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn last_commit(&self, path: file_system::Path) -> Result<Option<Commit>, Error> {
        let file_history = self.repository.file_history(self.get().first().clone())?;

        Ok(file_history.find(path.0).map(|tree| {
            tree.maximum_by(&|c: &NonEmpty<OrderedCommit>, d| c.first().compare_by_id(&d.first()))
                .first()
                .commit
                .clone()
        }))
    }

    /// Extract the signature for a commit
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit to extract the signature for
    /// * `field` - the name of the header field containing the signature block;
    ///   pass `None` to extract the default 'gpgsig'
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Oid, error};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(repo)?;
    ///
    /// let commit_with_signature_oid = Oid::from_str(
    ///     "e24124b7538658220b5aaf3b6ef53758f0a106dc"
    /// )?;
    ///
    /// browser.commit(commit_with_signature_oid)?;
    /// let history = browser.get();
    /// let commit_with_signature = history.first();
    /// let signature = browser.extract_signature(commit_with_signature, None)?;
    ///
    /// // We have a signature
    /// assert!(signature.is_some());
    ///
    /// let commit_without_signature_oid = Oid::from_str(
    ///     "80bacafba303bf0cdf6142921f430ff265f25095"
    /// )?;
    ///
    /// browser.commit(commit_without_signature_oid)?;
    /// let history = browser.get();
    /// let commit_without_signature = history.first();
    /// let signature = browser.extract_signature(commit_without_signature, None)?;
    ///
    /// // There is no signature
    /// assert!(signature.is_none());
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_signature(
        &self,
        commit: &Commit,
        field: Option<&str>,
    ) -> Result<Option<Signature>, Error> {
        self.repository.extract_signature(&commit.id, field)
    }

    /// Do a pre-order TreeWalk of the given commit. This turns a Tree
    /// into a HashMap of Paths and a list of Files. We can then turn that
    /// into a Directory.
    fn get_tree(
        repo: &git2::Repository,
        commit: &Commit,
    ) -> Result<HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>, Error>
    {
        let mut file_paths_or_error: Result<
            HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>,
            Error,
        > = Ok(HashMap::new());

        let commit = repo.find_commit(commit.id)?;
        let tree = commit.as_object().peel_to_tree()?;

        tree.walk(
            git2::TreeWalkMode::PreOrder,
            |s, entry| match Self::tree_entry_to_file_and_path(repo, s, entry) {
                Ok((path, name, file)) => {
                    match file_paths_or_error.as_mut() {
                        Ok(mut files) => Self::update_file_map(path, name, file, &mut files),

                        // We don't need to update, we want to keep the error.
                        Err(_err) => {},
                    }
                    git2::TreeWalkResult::Ok
                },
                Err(err) => match err {
                    // We want to continue if the entry was not a Blob.
                    TreeWalkError::NotBlob => git2::TreeWalkResult::Ok,

                    // We found a ObjectType::Commit (likely a submodule) and
                    // so we can skip it.
                    TreeWalkError::Commit => git2::TreeWalkResult::Ok,

                    // But we want to keep the error and abort otherwise.
                    TreeWalkError::Git(err) => {
                        file_paths_or_error = Err(err);
                        git2::TreeWalkResult::Abort
                    },
                },
            },
        )?;

        file_paths_or_error
    }

    fn update_file_map(
        path: file_system::Path,
        name: file_system::Label,
        file: directory::File,
        files: &mut HashMap<file_system::Path, NonEmpty<(file_system::Label, directory::File)>>,
    ) {
        files
            .entry(path)
            .and_modify(|entries| entries.push((name.clone(), file.clone())))
            .or_insert_with(|| NonEmpty::new((name, file)));
    }

    fn tree_entry_to_file_and_path(
        repo: &git2::Repository,
        tree_path: &str,
        entry: &git2::TreeEntry,
    ) -> Result<(file_system::Path, file_system::Label, directory::File), TreeWalkError> {
        // Account for the "root" of git being the empty string
        let path = if tree_path.is_empty() {
            Ok(file_system::Path::root())
        } else {
            file_system::Path::try_from(tree_path)
        }?;

        // We found a Commit object in the Tree, likely a submodule.
        // We will skip this entry.
        if let Some(git2::ObjectType::Commit) = entry.kind() {
            return Err(TreeWalkError::Commit);
        }

        let object = entry.to_object(repo)?;
        let blob = object.as_blob().ok_or(TreeWalkError::NotBlob)?;
        let name = str::from_utf8(entry.name_bytes())?;

        let name = file_system::Label::try_from(name).map_err(Error::FileSystem)?;

        Ok((
            path,
            name,
            directory::File {
                contents: blob.content().to_owned(),
                size: blob.size(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // An issue with submodules, see: https://github.com/radicle-dev/radicle-surf/issues/54
    fn test_submodule_failure() {
        let repo = Repository::new(".").unwrap();
        let browser = Browser::new(repo).unwrap();

        browser.get_directory().unwrap();
    }

    #[cfg(test)]
    mod rev {
        use super::{Browser, Error, Oid, Repository};

        // **FIXME**: This seems to break occasionally on
        // buildkite. For some reason the commit
        // 3873745c8f6ffb45c990eb23b491d4b4b6182f95, which is on master
        // (currently HEAD), is not found. It seems to load the history
        // with d6880352fc7fda8f521ae9b7357668b17bb5bad5 as the HEAD.
        //
        // To temporarily fix this, we need to select "New Build" from the build kite
        // build page that's failing.
        // * Under "Message" put whatever you want.
        // * Under "Branch" put in the branch you're working on.
        // * Expand "Options" and select "clean checkout".
        #[test]
        fn _master() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(repo)?;
            browser.revspec("master")?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(
                browser
                    .history
                    .find(|commit| if commit.id == commit1 {
                        Some(commit.clone())
                    } else {
                        None
                    })
                    .is_some(),
                "commit_id={}, history =\n{:#?}",
                commit1,
                browser.history
            );

            let commit2 = Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?;
            assert!(
                browser
                    .history
                    .find(|commit| if commit.id == commit2 {
                        Some(commit.clone())
                    } else {
                        None
                    })
                    .is_some(),
                "commit_id={}, history =\n{:#?}",
                commit2,
                browser.history
            );

            Ok(())
        }

        #[test]
        fn commit() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(repo)?;
            browser.revspec("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(browser
                .history
                .find(|commit| if commit.id == commit1 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some());

            Ok(())
        }

        #[test]
        fn commit_short() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(repo)?;
            browser.revspec("3873745c8")?;

            let commit1 = Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            assert!(browser
                .history
                .find(|commit| if commit.id == commit1 {
                    Some(commit.clone())
                } else {
                    None
                })
                .is_some());

            Ok(())
        }

        #[test]
        fn tag() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(repo)?;
            browser.revspec("v0.2.0")?;

            let commit1 = Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?;
            assert_eq!(browser.history.first().id, commit1);

            Ok(())
        }
    }

    #[cfg(test)]
    mod last_commit {
        use crate::{
            file_system::{unsound, Path},
            vcs::git::{Browser, Oid, Repository},
        };

        #[test]
        fn readme_missing_and_memory() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser = Browser::new(repo).expect("Could not initialise Browser");

            // Set the browser history to the initial commit
            let commit = Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")
                .expect("Failed to parse SHA");
            browser.commit(commit).unwrap();

            let head_commit = browser.get().0.first().clone();

            // memory.rs is commited later so it should not exist here.
            let memory_last_commit = browser
                .last_commit(Path::with_root(&[
                    unsound::label::new("src"),
                    unsound::label::new("memory.rs"),
                ]))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(memory_last_commit, None);

            // README.md exists in this commit.
            let readme_last_commit = browser
                .last_commit(Path::with_root(&[unsound::label::new("README.md")]))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(readme_last_commit, Some(head_commit.id));
        }

        #[test]
        fn folder_svelte() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser = Browser::new(repo).expect("Could not initialise Browser");

            // Check that last commit is the actual last commit even if head commit differs.
            let commit = Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")
                .expect("Could not parse SHA");
            browser.commit(commit).unwrap();

            let expected_commit_id =
                Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2").unwrap();

            let folder_svelte = browser
                .last_commit(unsound::path::new("~/examples/Folder.svelte"))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(folder_svelte, Some(expected_commit_id));
        }

        #[test]
        fn nest_directory() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let mut browser = Browser::new(repo).expect("Could not initialise Browser");

            // Check that last commit is the actual last commit even if head commit differs.
            let commit = Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")
                .expect("Failed to parse SHA");
            browser.commit(commit).unwrap();

            let expected_commit_id =
                Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977").unwrap();

            let nested_directory_tree_commit_id = browser
                .last_commit(unsound::path::new(
                    "~/this/is/a/really/deeply/nested/directory/tree",
                ))
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(nested_directory_tree_commit_id, Some(expected_commit_id));
        }

        #[test]
        fn root() {
            let repo = Repository::new("./data/git-platinum")
                .expect("Could not retrieve ./data/git-platinum as git repository");
            let browser = Browser::new(repo).expect("Could not initialise Browser");

            let root_last_commit_id = browser
                .last_commit(Path::root())
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(root_last_commit_id, Some(browser.get().first().id));
        }
    }

    #[test]
    fn test_diff() {
        use file_system::*;
        use pretty_assertions::assert_eq;

        let repo = Repository::new("./data/git-platinum").unwrap();

        let commit = repo
            .0
            .find_commit(Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095").unwrap())
            .unwrap();
        let parent = commit.parent(0).unwrap();

        let diff = repo.diff(&parent, &commit).unwrap();

        assert_eq!(
            Diff {
                created: vec![],
                deleted: vec![],
                moved: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
                    diff: FileDiff {
                        hunks: vec![Hunk {
                            header: b"@@ -1 +1,2 @@\n".to_vec(),
                            lines: vec![
                                LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                                LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                            ]
                        }]
                    }
                }]
            },
            diff
        );
    }
}
