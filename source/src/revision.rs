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

use std::convert::TryFrom;

use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};

use radicle_surf::vcs::git::{self, BranchType, Browser, Rev};

use crate::{
    branch::{branches, Branch},
    error::Error,
    oid::Oid,
    tag::{tags, Tag},
};

pub enum Category<P, U> {
    Local { identifier: P, user: U },
    Remote { identifier: P, user: U },
}

/// A revision selector for a `Browser`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Revision<P> {
    /// Select a tag under the name provided.
    #[serde(rename_all = "camelCase")]
    Tag {
        /// Name of the tag.
        name: String,
    },
    /// Select a branch under the name provided.
    #[serde(rename_all = "camelCase")]
    Branch {
        /// Name of the branch.
        name: String,
        /// The remote peer, if specified.
        identifier: Option<P>,
    },
    /// Select a SHA1 under the name provided.
    #[serde(rename_all = "camelCase")]
    Sha {
        /// The SHA1 value.
        sha: Oid,
    },
}

impl<P> TryFrom<Revision<P>> for Rev
where
    P: ToString,
{
    type Error = Error;

    fn try_from(other: Revision<P>) -> Result<Self, Self::Error> {
        match other {
            Revision::Tag { name } => Ok(git::TagName::new(&name).into()),
            Revision::Branch { name, identifier } => Ok(match identifier {
                Some(peer) => {
                    git::Branch::remote(&format!("heads/{}", name), &peer.to_string()).into()
                },
                None => git::Branch::local(&name).into(),
            }),
            Revision::Sha { sha } => {
                let oid: git2::Oid = sha.into();
                Ok(oid.into())
            },
        }
    }
}

/// Bundled response to retrieve both [`Branch`]es and [`Tag`]s for a user's
/// repo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revisions<P, U> {
    /// The peer identifier for the user.
    pub identifier: P,
    /// The user who owns these revisions.
    pub user: U,
    /// List of [`git::Branch`].
    pub branches: NonEmpty<Branch>,
    /// List of [`git::Tag`].
    pub tags: Vec<Tag>,
}

/// Provide the [`Revisions`] for the given `identifier`, looking for the
/// branches as [`BranchType::Remote`].
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn remote<P, U>(
    browser: &Browser,
    identifier: P,
    user: U,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let remote_branches = branches(browser, Some(into_branch_type(Some(identifier.clone()))))?;
    Ok(
        NonEmpty::from_vec(remote_branches).map(|branches| Revisions {
            identifier,
            user,
            branches,
            // TODO(rudolfs): implement remote peer tags once we decide how
            // https://radicle.community/t/git-tags/214
            tags: vec![],
        }),
    )
}

/// Provide the [`Revisions`] for the given `identifier`, looking for the
/// branches as [`BranchType::Local`].
///
/// If there are no branches then this returns `None`.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn local<P, U>(
    browser: &Browser,
    identifier: P,
    user: U,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let local_branches = branches(browser, Some(BranchType::Local))?;
    let tags = tags(browser)?;
    Ok(
        NonEmpty::from_vec(local_branches).map(|branches| Revisions {
            identifier,
            user,
            branches,
            tags,
        }),
    )
}

/// Provide the [`Revisions`] of a peer.
///
/// If the peer is [`Category::Local`], meaning that is the current person doing
/// the browsing and no remote is set for the reference.
///
/// Othewise, the peer is [`Category::Remote`], meaning that we are looking into
/// a remote part of a reference.
///
/// # Errors
///
///   * If we cannot get the branches from the `Browser`
pub fn revisions<P, U>(
    browser: &Browser,
    peer: Category<P, U>,
) -> Result<Option<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    match peer {
        Category::Local { identifier, user } => local(browser, identifier, user),
        Category::Remote { identifier, user } => remote(browser, identifier, user),
    }
}

/// Turn an `Option<P>` into a [`BranchType`]. If the `P` is present then this
/// is set as the remote of the `BranchType`. Otherwise, it's local branch.
#[must_use]
pub fn into_branch_type<P>(identifier: Option<P>) -> BranchType
where
    P: ToString,
{
    identifier.map_or(BranchType::Local, |identifier| BranchType::Remote {
        // We qualify the remotes as the PeerId + heads, otherwise we would grab the tags too.
        name: Some(format!("{}/heads", identifier.to_string())),
    })
}
