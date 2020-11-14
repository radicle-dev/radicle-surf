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

use std::{convert::TryFrom, fmt, str};
use thiserror::Error;

use radicle_git_ext::{self as ext, OneLevel, Qualified, RefLike};

use crate::vcs::git::{repo::RepositoryRef, Namespace};

pub(super) mod glob;

/// A revision within the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rev {
    /// A reference to a branch or tag.
    Ref(Ref),
    /// A particular commit identifier.
    Oid(git2::Oid),
}

impl<R> From<R> for Rev
where
    R: Into<Ref>,
{
    fn from(other: R) -> Self {
        Self::Ref(other.into())
    }
}

impl From<git2::Oid> for Rev {
    fn from(other: git2::Oid) -> Self {
        Self::Oid(other)
    }
}

/// A structured way of referring to a git reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ref {
    /// A git tag, which can be found under `.git/refs/tags/`.
    Tag {
        /// The name of the tag, e.g. `v1.0.0`.
        name: OneLevel,
    },
    /// A git branch, which can be found under `.git/refs/heads/`.
    LocalBranch {
        /// The name of the branch, e.g. `master`.
        name: Qualified,
    },
    /// A git branch, which can be found under `.git/refs/remotes/`.
    RemoteBranch {
        /// The remote name, e.g. `origin`.
        remote: RefLike,
        /// The name of the branch, e.g. `master`.
        name: OneLevel,
    },
    /// A git namespace, which can be found under `.git/refs/namespaces/`.
    ///
    /// Note that namespaces can be nested.
    Namespace {
        /// The name value of the namespace.
        namespace: String,
        /// The reference under that namespace, e.g. The
        /// `refs/remotes/origin/master/ portion of `refs/namespaces/
        /// moi/refs/remotes/origin/master`.
        reference: Box<Ref>,
    },
}

impl Ref {
    /// Add a [`Namespace`] to a `Ref`.
    pub fn namespaced(self, Namespace { values: namespaces }: Namespace) -> Self {
        let mut ref_namespace = self;
        for namespace in namespaces.into_iter().rev() {
            ref_namespace = Self::Namespace {
                namespace,
                reference: Box::new(ref_namespace.clone()),
            };
        }

        ref_namespace
    }

    /// We try to find a [`git2::Reference`] based off of a `Ref` by turning the
    /// ref into a fully qualified ref (e.g. refs/remotes/**/master).
    pub fn find_ref<'a>(
        &self,
        repo: &RepositoryRef<'a>,
    ) -> Result<git2::Reference<'a>, git2::Error> {
        repo.repo_ref.find_reference(&self.to_string())
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag { name } => write!(f, "refs/tags/{}", name),
            Self::LocalBranch { name } => write!(f, "{}", name),
            Self::RemoteBranch { remote, name } => write!(f, "refs/remotes/{}/{}", remote, name),
            Self::Namespace {
                namespace,
                reference,
            } => write!(f, "refs/namespaces/{}/{}", namespace, reference),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Name(#[from] ext::name::Error),
    #[error(transparent)]
    StripPrefix(#[from] ext::name::StripPrefixError),
    #[error("was able to parse 'refs/remotes' but failed to parse the remote name, perhaps you're missing 'origin/'")]
    MissingRemote(RefLike),
    #[error("was able to parse 'refs/namespaces' but failed to parse the namespace name, a valid form would be 'refs/namespaces/moi/refs/heads/master'")]
    MissingNamespace,
    #[error("the ref provided '{0}' was malformed")]
    MalformedRef(RefLike),
    #[error("while attempting to parse a commit SHA we encountered an error: {0:?}")]
    Sha(#[from] git2::Error),
}

impl str::FromStr for Ref {
    type Err = ParseError;

    fn from_str(reference: &str) -> Result<Self, Self::Err> {
        let reference = RefLike::try_from(reference)?;

        if reference.starts_with("refs/heads/") {
            return Ok(Ref::LocalBranch {
                name: Qualified::from(reference),
            });
        }

        if reference.starts_with("refs/tags/") {
            return Ok(Ref::Tag {
                name: OneLevel::from(reference),
            });
        }

        if reference.starts_with("refs/remotes/") {
            let suffix = reference.strip_prefix("refs/remotes/")?;
            let remote = suffix
                .parent()
                .ok_or(ParseError::MissingRemote(reference))?;
            let name = suffix.strip_prefix(format!("{}/", remote.display()))?;
            return Ok(Ref::RemoteBranch {
                name: OneLevel::from(name),
                remote: RefLike::try_from(remote)?,
            });
        }

        if reference.starts_with("refs/namespaces/") {
            let suffix = reference.strip_prefix("refs/namespaces/")?;
            let namespace = suffix
                .components()
                .take_while(|c| c.as_os_str() != "refs")
                .fold(std::path::PathBuf::new(), |n, c| n.join(c.as_os_str()));
            let reference = Ref::from_str(
                suffix
                    .strip_prefix(format!("{}/", namespace.display()))?
                    .as_str(),
            )?;
            return Ok(Ref::Namespace {
                namespace: format!("{}", namespace.display()),
                reference: Box::new(reference),
            });
        }

        Err(ParseError::MalformedRef(reference))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parse_ref() -> Result<(), ParseError> {
        assert_eq!(
            Ref::from_str("refs/remotes/origin/master").unwrap(),
            Ref::RemoteBranch {
                remote: reflike!("origin"),
                name: OneLevel::from(reflike!("master")),
            }
        );

        assert_eq!(
            Ref::from_str("refs/heads/master").unwrap(),
            Ref::LocalBranch {
                name: reflike!("master").into(),
            }
        );

        assert_eq!(
            Ref::from_str("refs/heads/xla/handle-disconnect").unwrap(),
            Ref::LocalBranch {
                name: reflike!("xla/handle-disconnect").into(),
            }
        );

        assert_eq!(
            Ref::from_str("refs/tags/v0.0.1").unwrap(),
            Ref::Tag {
                name: reflike!("v0.0.1").into()
            }
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/moi/refs/remotes/origin/master").unwrap(),
            Ref::Namespace {
                namespace: "moi".to_string(),
                reference: Box::new(Ref::RemoteBranch {
                    remote: reflike!("origin"),
                    name: reflike!("master").into()
                })
            }
        );

        assert_eq!(
            Ref::from_str("refs/namespaces/moi/refs/namespaces/toi/refs/tags/v1.0.0").unwrap(),
            Ref::Namespace {
                namespace: "moi".to_string(),
                reference: Box::new(Ref::Namespace {
                    namespace: "toi".to_string(),
                    reference: Box::new(Ref::Tag {
                        name: reflike!("v1.0.0").into()
                    })
                })
            }
        );

        assert!(Ref::from_str("refs/remotes/master").is_err());

        assert!(Ref::from_str("refs/namespaces/refs/remotes/origin/master").is_err(),);

        Ok(())
    }
}
