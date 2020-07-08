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

use crate::vcs::git::{error::Error, object::git_ext};
use std::{cmp::Ordering, convert::TryFrom, fmt, str};

/// The branch type we want to filter on.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum BranchType {
    /// Local branches that are under `refs/heads/*`
    Local,
    /// Remote branches that are under `refs/remotes/<name>/*` if the name is
    /// provided, otherwise `refs/remotes/**/*`.
    Remote {
        /// Name of the remote.
        name: Option<String>,
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

/// A newtype wrapper over `String` to separate out the fact that a caller wants
/// to fetch a branch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchName(pub(crate) String);

impl fmt::Display for BranchName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<&[u8]> for BranchName {
    type Error = str::Utf8Error;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        let name = str::from_utf8(name)?;
        let short_name = match git_ext::try_extract_refname(name) {
            Ok(stripped) => stripped,
            Err(original) => original,
        };
        Ok(Self(String::from(short_name)))
    }
}

impl BranchName {
    /// Create a new `BranchName`.
    pub fn new(name: &str) -> Self {
        BranchName(name.into())
    }

    /// Access the string value of the `BranchName`.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// The static information of a `git2::Branch`.
///
/// **Note**: The `PartialOrd` and `Ord` implementations compare on `BranchName`
/// only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Name identifier of the `Branch`.
    pub name: BranchName,
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

impl Branch {
    /// Helper to create a remote `Branch` with a name
    pub fn remote(name: BranchName, remote: String) -> Self {
        Branch {
            name,
            locality: BranchType::Remote { name: Some(remote) },
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

impl<'repo> TryFrom<git2::Reference<'repo>> for Branch {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let is_remote = reference.is_remote();
        let is_tag = reference.is_tag();
        let is_note = reference.is_note();
        let name = BranchName::try_from(reference.name_bytes())?;

        // Best effort to not return tags or notes. Assuming everything after that is a
        // branch.
        if is_tag || is_note {
            return Err(Error::NotBranch(name));
        }

        let (name, locality) = if is_remote {
            let mut split = name.0.split('/');
            let remote_name = split
                .next()
                .ok_or_else(|| Error::ParseRemoteBranch(name.clone()))?;
            let name = split
                .next()
                .ok_or_else(|| Error::ParseRemoteBranch(name.clone()))?;
            (
                BranchName(name.to_string()),
                BranchType::Remote {
                    name: Some(remote_name.to_string()),
                },
            )
        } else {
            (name, BranchType::Local)
        };

        Ok(Branch { name, locality })
    }
}