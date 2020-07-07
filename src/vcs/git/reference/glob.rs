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

use crate::vcs::git::{
    error,
    object::{BranchType, Namespace},
    repo::RepositoryRef,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefGlob {
    /// When calling [`RefGlob::references`] this will return the references via
    /// the glob `refs/tags/*`.
    Tag,
    /// When calling [`RefGlob::references`] this will return the references via
    /// the glob `refs/heads/*`.
    LocalBranch,
    /// When calling [`RefGlob::references`] this will return the references via
    /// either of the following globs:
    ///     * `refs/remotes/**/*`
    ///     * `refs/remotes/{remote}/*`
    RemoteBranch {
        /// If `remote` is `None` then the `**` wildcard will be used, otherwise
        /// the provided remote name will be used.
        remote: Option<String>,
    },
    /// When calling [`RefGlob::references`] this will return the references via
    /// the globs `refs/heads/*` and `refs/remotes/**/*`.
    Branch,
    /// refs/namespaces/**
    Namespace,
}

impl From<BranchType> for RefGlob {
    fn from(other: BranchType) -> Self {
        match other {
            BranchType::Remote { name } => Self::RemoteBranch { remote: name },
            BranchType::Local => Self::LocalBranch,
        }
    }
}

/// Iterator chaining multiple [`git2::References`]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct References<'a> {
    inner: Vec<git2::References<'a>>,
}

impl<'a> References<'a> {
    pub fn iter(self) -> impl Iterator<Item = Result<git2::Reference<'a>, git2::Error>> {
        self.inner.into_iter().flatten()
    }
}

impl RefGlob {
    pub fn references<'a>(&self, repo: &RepositoryRef<'a>) -> Result<References<'a>, error::Error> {
        let namespace = repo
            .which_namespace()?
            .unwrap_or(Namespace { values: vec![] });
        self.with_namespace_glob(namespace, repo)
    }

    fn with_namespace_glob<'a>(
        &self,
        namespace: Namespace,
        repo: &RepositoryRef<'a>,
    ) -> Result<References<'a>, error::Error> {
        let mut namespace_glob = "".to_string();
        for n in namespace.values {
            namespace_glob.push_str(&format!("refs/namespaces/{}/", n));
        }

        Ok(match self {
            Self::Branch => {
                let remotes = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    Self::RemoteBranch { remote: None }.to_string()
                ))?;

                let locals = repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    &Self::LocalBranch.to_string()
                ))?;
                References {
                    inner: vec![remotes, locals],
                }
            },
            other => References {
                inner: vec![repo.repo_ref.references_glob(&format!(
                    "{}{}",
                    namespace_glob,
                    other.to_string()
                ))?],
            },
        })
    }
}

impl fmt::Display for RefGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag => write!(f, "refs/tags/*"),
            Self::LocalBranch => write!(f, "refs/heads/*"),
            Self::RemoteBranch { remote } => {
                write!(f, "refs/remotes/")?;
                match remote {
                    None => write!(f, "**/*"),
                    Some(remote) => write!(f, "{}/*", remote),
                }
            },
            // Note: the glob below would be used, but libgit doesn't care for union globs.
            // write!(f, "refs/{{remotes/**/*,heads/*}}")
            Self::Branch => panic!(
                "fatal: `Display` should not be called on `RefGlob::Branch`. Since this `enum` is
                private to the repository, it should not be called from the outside.
                Unfortunately, libgit does not support union of globs
                otherwise this would display refs/{{remotes/**/*,heads/*}}"
            ),
            Self::Namespace => write!(f, "refs/namespaces/**"),
        }
    }
}
