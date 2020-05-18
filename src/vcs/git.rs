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
//! let mut browser = Browser::new(&repo)?;
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

mod repo;
pub use repo::{History, Repository, RepositoryRef};

pub mod error;
mod object;

pub use crate::{
    diff::Diff,
    vcs::git::object::{Branch, BranchName, Commit, RevObject, Signature, TagName},
};

use crate::{
    file_system,
    file_system::directory,
    vcs,
    vcs::{git::error::*, VCS},
};
use nonempty::NonEmpty;
use std::{collections::HashMap, convert::TryFrom, str};

/// A [`crate::vcs::Browser`] that uses [`Repository`] as the underlying
/// repository backend, [`git2::Commit`] as the artifact, and [`Error`] for
/// error reporting.
pub type Browser<'a> = vcs::Browser<RepositoryRef<'a>, Commit, Error>;

impl<'a> Browser<'a> {
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
    /// let browser = Browser::new(&repo)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(repository: impl Into<RepositoryRef<'a>>) -> Result<Self, Error> {
        let repository = repository.into();
        let history = repository.head()?;
        let snapshot = Box::new(|repository: &RepositoryRef<'a>, history: &History| {
            let tree = Self::get_tree(&repository.repo_ref, history.0.first())?;
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
    ///     .as_ref()
    ///     .list_branches(None)?
    ///     .first()
    ///     .cloned()
    ///     .expect("failed to get first branch");
    /// let browser = Browser::new_with_branch(&repo, first_branch.name)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_branch(
        repository: impl Into<RepositoryRef<'a>>,
        branch_name: BranchName,
    ) -> Result<Self, Error> {
        let repository = repository.into();
        let history = repository.get_history(branch_name.name().to_string())?;
        let snapshot = Box::new(|repository: &RepositoryRef<'a>, history: &History| {
            let tree = Self::get_tree(&repository.repo_ref, history.0.first())?;
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
    /// let mut browser = Browser::new(&repo)?;
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
    /// let mut browser = Browser::new(&repo)?;
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
    /// let mut browser = Browser::new(&repo)?;
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
            .repo_ref
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
    /// let mut browser = Browser::new(&repo)?;
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
            .repo_ref
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
    /// let mut browser = Browser::new(&repo)?;
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
    /// let mut browser = Browser::new(&repo)?;
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
        let commit = rev.into_commit(&repository.repo_ref)?;
        let history = repository.commit_to_history(commit)?;
        self.set(history);
        Ok(())
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: &'a git2::Commit, to: &'a git2::Commit) -> Result<Diff, Error> {
        self.repository.diff(from, to)
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
    /// let mut browser = Browser::new(&repo)?;
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
    /// let mut browser = Browser::new(&repo)?;
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
    /// let mut browser = Browser::new(&repo)?;
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
            tree.maximum_by(&|c: &NonEmpty<repo::OrderedCommit>, d| {
                c.first().compare_by_id(&d.first())
            })
            .first()
            .commit
            .clone()
        }))
    }

    /// Get the commit history for a file _or_ directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::vcs::git::{Browser, Oid, Repository};
    /// use radicle_surf::file_system::{Label, Path, SystemType};
    /// use radicle_surf::file_system::unsound;
    /// use std::str::FromStr;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let mut browser = Browser::new(&repo)?;
    ///
    /// // Clamp the Browser to a particular commit
    /// let commit = Oid::from_str("223aaf87d6ea62eef0014857640fd7c8dd0f80b5")?;
    /// browser.commit(commit)?;
    ///
    /// let root_commits: Vec<Oid> = browser
    ///     .file_history(unsound::path::new("~"))?
    ///     .into_iter()
    ///     .map(|commit| commit.id)
    ///     .collect();
    ///
    /// assert_eq!(root_commits,
    ///     vec![
    ///         Oid::from_str("223aaf87d6ea62eef0014857640fd7c8dd0f80b5")?,
    ///         Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095")?,
    ///         Oid::from_str("a57846bbc8ced6587bf8329fc4bce970eb7b757e")?,
    ///         Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?,
    ///         Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?,
    ///         Oid::from_str("91b69e00cd8e5a07e20942e9e4457d83ce7a3ff1")?,
    ///         Oid::from_str("1820cb07c1a890016ca5578aa652fd4d4c38967e")?,
    ///         Oid::from_str("1e0206da8571ca71c51c91154e2fee376e09b4e7")?,
    ///         Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?,
    ///         Oid::from_str("19bec071db6474af89c866a1bd0e4b1ff76e2b97")?,
    ///         Oid::from_str("f3a089488f4cfd1a240a9c01b3fcc4c34a4e97b2")?,
    ///         Oid::from_str("2429f097664f9af0c5b7b389ab998b2199ffa977")?,
    ///         Oid::from_str("d3464e33d75c75c99bfb90fa2e9d16efc0b7d0e3")?,
    ///     ]
    /// );
    ///
    /// let eval_commits: Vec<Oid> = browser
    ///     .file_history(unsound::path::new("~/src/Eval.hs"))?
    ///     .into_iter()
    ///     .map(|commit| commit.id)
    ///     .collect();
    ///
    /// assert_eq!(eval_commits,
    ///     vec![
    ///         Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?,
    ///         Oid::from_str("e24124b7538658220b5aaf3b6ef53758f0a106dc")?,
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn file_history(&self, path: file_system::Path) -> Result<Vec<Commit>, Error> {
        self.repository
            .file_history(self.get().first().clone())
            .and_then(|history| {
                let subtree = history
                    .find(path.clone().0)
                    .ok_or_else(|| Error::PathNotFound(path))?;
                let mut commits: Vec<repo::OrderedCommit> =
                    NonEmpty::flatten(subtree.to_nonempty()).into();
                commits.sort_by(|commit, other| commit.id.cmp(&other.id));
                commits.dedup_by(|commit, other| commit.id == other.id);

                Ok(commits
                    .into_iter()
                    .map(|ordered_commit| ordered_commit.commit)
                    .collect())
            })
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
    /// let mut browser = Browser::new(&repo)?;
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

    /// List the [`Branch`]es, which contain the provided [`Commit`].
    ///
    /// # Errors
    ///
    /// * [`error::Error::Git`]
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::vcs::git::{Browser, Repository, Branch, BranchName, Oid};
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let repo = Repository::new("./data/git-platinum")?;
    /// let browser = Browser::new(&repo)?;
    ///
    ///
    /// let branches = browser.revision_branches("27acd68c7504755aa11023300890bb85bbd69d45")?;
    /// assert_eq!(
    ///     branches,
    ///     vec![Branch::local(BranchName::new("dev"))]
    /// );
    ///
    /// // TODO(finto): I worry that this test will fail as other branches get added
    /// let branches = browser.revision_branches("1820cb07c1a890016ca5578aa652fd4d4c38967e")?;
    /// assert_eq!(
    ///     branches,
    ///     vec![
    ///         Branch::local(BranchName::new("dev")),
    ///         Branch::local(BranchName::new("master")),
    ///     ]
    /// );
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn revision_branches(&self, revspec: &str) -> Result<Vec<Branch>, Error> {
        let rev = self.repository.rev(revspec)?;
        let commit = rev.into_commit(&self.repository.repo_ref)?;
        self.repository.revision_branches(&commit.id())
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
        let browser = Browser::new(&repo).unwrap();

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
            let mut browser = Browser::new(&repo)?;
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
            let mut browser = Browser::new(&repo)?;
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
        fn commit_parents() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo)?;
            browser.revspec("3873745c8f6ffb45c990eb23b491d4b4b6182f95")?;
            let commit = browser.history.first();

            assert_eq!(
                commit.parents,
                vec![Oid::from_str("d6880352fc7fda8f521ae9b7357668b17bb5bad5")?]
            );

            Ok(())
        }

        #[test]
        fn commit_short() -> Result<(), Error> {
            let repo = Repository::new("./data/git-platinum")?;
            let mut browser = Browser::new(&repo)?;
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
            let mut browser = Browser::new(&repo)?;
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
            let mut browser = Browser::new(&repo).expect("Could not initialise Browser");

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
            let mut browser = Browser::new(&repo).expect("Could not initialise Browser");

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
            let mut browser = Browser::new(&repo).expect("Could not initialise Browser");

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
            let browser = Browser::new(&repo).expect("Could not initialise Browser");

            let root_last_commit_id = browser
                .last_commit(Path::root())
                .expect("Failed to get last commit")
                .map(|commit| commit.id);

            assert_eq!(root_last_commit_id, Some(browser.get().first().id));
        }
    }

    #[cfg(test)]
    mod diff {
        use crate::{diff::*, vcs::git::*};

        #[test]
        fn test_diff() -> Result<(), Error> {
            use file_system::*;
            use pretty_assertions::assert_eq;

            let repo = Repository::new("./data/git-platinum")?;
            let commit = repo
                .0
                .find_commit(Oid::from_str("80bacafba303bf0cdf6142921f430ff265f25095")?)
                .unwrap();
            let parent = commit.parent(0)?;

            let bro = Browser::new(&repo)?;

            let diff = bro.diff(&parent, &commit)?;

            let expected_diff = Diff {
                created: vec![],
                deleted: vec![],
                moved: vec![],
                copied: vec![],
                modified: vec![ModifiedFile {
                    path: Path::with_root(&[unsound::label::new("README.md")]),
                    diff: FileDiff::Plain(
                        vec![Hunk {
                            header: b"@@ -1 +1,2 @@\n".to_vec(),
                            lines: vec![
                                LineDiff::deletion(b"This repository is a data source for the Upstream front-end tests.\n".to_vec(), 1),
                                LineDiff::addition(b"This repository is a data source for the Upstream front-end tests and the\n".to_vec(), 1),
                                LineDiff::addition(b"[`radicle-surf`](https://github.com/radicle-dev/git-platinum) unit tests.\n".to_vec(), 2),
                            ]
                        }]
                    )
                }]
            };
            assert_eq!(expected_diff, diff);

            Ok(())
        }
    }

    #[cfg(test)]
    mod threading {
        use crate::vcs::git::*;
        use std::sync::{Mutex, MutexGuard};

        #[test]
        fn basic_test() -> Result<(), Error> {
            let shared_repo = Mutex::new(Repository::new("./data/git-platinum")?);
            let locked_repo: MutexGuard<Repository> = shared_repo.lock().unwrap();
            let bro = Browser::new(&*locked_repo)?;
            let mut branches = bro.list_branches(None)?;
            branches.sort();

            assert_eq!(
                branches,
                vec![
                    Branch::local(BranchName::new("dev")),
                    Branch::local(BranchName::new("master")),
                    Branch::remote(BranchName::new("origin/HEAD")),
                    Branch::remote(BranchName::new("origin/dev")),
                    Branch::remote(BranchName::new("origin/master")),
                ]
            );

            Ok(())
        }
    }
}
