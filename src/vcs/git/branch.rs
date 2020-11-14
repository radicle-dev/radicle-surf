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

use std::{cmp::Ordering, convert::TryFrom};

use radicle_git_ext::{self as ext, OneLevel, Qualified, RefLike};

use crate::vcs::git::{self, reference::Ref};

/// An error occurred attempting to parse a [`Branch`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The [`git::Reference`] name was invalid.
    #[error(transparent)]
    Name(#[from] ext::name::Error),
    /// We tried to convert a name into its remote and branch name parts.
    #[error("no remote could be determined for `{0}`")]
    NoRemote(RefLike),
    /// The user tried to fetch a branch, but the name provided does not
    /// exist as a branch. This could mean that the branch does not exist
    /// or that a tag or commit was provided by accident.
    #[error("the reference `{0}` is not a branch")]
    NotBranch(RefLike),
    /// The [`git::Reference`] could not successfully remove the namespace.
    #[error(transparent)]
    Strip(#[from] ext::name::StripPrefixError),
}

/// The branch type we want to filter on.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum BranchType {
    /// Local branches that are under `refs/heads/*`
    Local,
    /// Remote branches that are under `refs/remotes/<name>/*` if the name is
    /// provided, otherwise `refs/remotes/**/*`.
    Remote {
        /// Name of the remote.
        name: Option<RefLike>, // TODO(finto): Needs to be Either<RefspecPattern, RefLike>
    },
}

impl From<BranchType> for git2::BranchType {
    fn from(other: BranchType) -> Self {
        match other {
            BranchType::Local => git2::BranchType::Local,
            BranchType::Remote { .. } => git2::BranchType::Remote,
        }
    }
}

impl From<git2::BranchType> for BranchType {
    fn from(other: git2::BranchType) -> Self {
        match other {
            git2::BranchType::Local => BranchType::Local,
            git2::BranchType::Remote => BranchType::Remote { name: None },
        }
    }
}

/// The static information of a `git2::Branch`.
///
/// **Note**: The `PartialOrd` and `Ord` implementations compare on `BranchName`
/// only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Name identifier of the `Branch`.
    pub name: OneLevel,
    /// Whether the `Branch` is `Remote` or `Local`.
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

impl From<Branch> for Ref {
    fn from(other: Branch) -> Self {
        match other.locality {
            BranchType::Local => Self::LocalBranch {
                name: Qualified::from(other.name),
            },
            BranchType::Remote { name } => Self::RemoteBranch {
                name: other.name,
                remote: name.unwrap_or_else(|| todo!()), // "**".to_string()),
            },
        }
    }
}

impl Branch {
    /// Helper to create a remote `Branch` with a name
    pub fn remote(name: impl Into<OneLevel>, remote: impl Into<RefLike>) -> Self {
        Self {
            name: name.into(),
            locality: BranchType::Remote {
                name: Some(remote.into()),
            },
        }
    }

    /// Helper to create a remote `Branch` with a name
    pub fn local(name: impl Into<OneLevel>) -> Self {
        Self {
            name: name.into(),
            locality: BranchType::Local,
        }
    }

    /// Get the name of the `Branch`.
    pub fn name(&self) -> OneLevel {
        let branch_name = self.name.clone();
        match self.locality {
            BranchType::Local => branch_name,
            BranchType::Remote { ref name } => match name {
                None => branch_name,
                Some(remote_name) => OneLevel::from(remote_name.join(branch_name)),
            },
        }
    }
}

impl<'repo> TryFrom<git2::Reference<'repo>> for Branch {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let is_remote = git::ext::is_remote(&reference);
        let is_tag = reference.is_tag();
        let is_note = reference.is_note();
        let name = git::ext::remove_namespace(RefLike::try_from(reference.name_bytes())?)?;

        // Best effort to not return tags or notes. Assuming everything after that is a
        // branch.
        if is_tag || is_note {
            return Err(Error::NotBranch(name));
        }

        if is_remote {
            let (remote, name) = extract_remote(name.clone())?.ok_or(Error::NoRemote(name))?;
            Ok(Self {
                name,
                locality: BranchType::Remote { name: Some(remote) },
            })
        } else {
            Ok(Self {
                name: OneLevel::from(name),
                locality: BranchType::Local,
            })
        }
    }
}

fn extract_remote(reflike: RefLike) -> Result<Option<(RefLike, OneLevel)>, Error> {
    if !reflike.starts_with("refs/remotes/") {
        return Ok(None);
    }

    let suffix = reflike.strip_prefix("refs/remotes/")?;
    let mut components = suffix.components();
    let remote = components
        .next()
        .ok_or(Error::NoRemote(reflike))
        .and_then(|c| {
            RefLike::try_from(
                c.as_os_str()
                    .to_str()
                    .expect("reflike components are valid os str"),
            )
            .map_err(Error::from)
        })?;

    let name = RefLike::try_from(components.as_path())?;
    let name = if name.starts_with("heads/") {
        name.strip_prefix("heads/")?
    } else {
        name
    };
    Ok(Some((remote, name.into())))
}
