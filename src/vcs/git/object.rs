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

use crate::vcs::git::error::Error;
pub use git2::Oid;
use std::{convert::TryFrom, fmt, str};

#[cfg(feature = "serialize")]
use serde::Serialize;

/// `Author` is the static information of a [`git2::Signature`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Author {
    /// Name of the author.
    pub name: String,
    /// Email of the author.
    pub email: String,
    /// Time the action was taken, e.g. time of commit.
    pub time: git2::Time,
}

impl std::fmt::Debug for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Signature {{ name: {}, email: {} }}",
            self.name, self.email
        )
    }
}

impl<'repo> TryFrom<git2::Signature<'repo>> for Author {
    type Error = str::Utf8Error;

    fn try_from(signature: git2::Signature) -> Result<Self, Self::Error> {
        let name = str::from_utf8(signature.name_bytes())?.into();
        let email = str::from_utf8(signature.email_bytes())?.into();
        let time = signature.when();

        Ok(Author { name, email, time })
    }
}

/// `Commit` is the static information of a [`git2::Commit`]. To get back the
/// original `Commit` in the repository we can use the [`Oid`] to retrieve
/// it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commit {
    /// Object ID of the Commit, i.e. the SHA1 digest.
    pub id: Oid,
    /// The author of the commit.
    pub author: Author,
    /// The actor who committed this commit.
    pub committer: Author,
    /// The long form message of the commit.
    pub message: String,
    /// The summary message of the commit.
    pub summary: String,
    /// The parents of this commit.
    pub parents: Vec<Oid>,
}

impl<'repo> TryFrom<git2::Commit<'repo>> for Commit {
    type Error = Error;

    fn try_from(commit: git2::Commit) -> Result<Self, Self::Error> {
        let id = commit.id();
        let author = Author::try_from(commit.author())?;
        let committer = Author::try_from(commit.committer())?;
        let message_raw = commit.message_bytes();
        let message = str::from_utf8(message_raw)?.into();
        let summary_raw = commit.summary_bytes().ok_or(Error::MissingSummary)?;
        let summary = str::from_utf8(summary_raw)?.into();
        let parents = commit.parent_ids().collect();

        Ok(Commit {
            id,
            author,
            committer,
            message,
            summary,
            parents,
        })
    }
}

/// Stats for a repository
#[cfg_attr(
    feature = "serialize",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Stats {
    /// Number of commits
    pub commits: usize,
    /// Number of local branches
    pub branches: usize,
    /// Number of contributors
    pub contributors: usize,
}

/// The signature of a commit
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signature(Vec<u8>);

impl Signature {
    pub(super) fn from_buf(buf: git2::Buf) -> Signature {
        Signature((*buf).into())
    }
}

/// A `Namespace` value allows us to switch the git namespace of
/// [`super::Browser`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace {
    /// Since namespaces can be nested we have a vector of strings.
    /// This means that the namespaces `"foo/bar"` is represented as
    /// `vec!["foo", "bar"]`.
    pub(super) values: Vec<String>,
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.values.join("/"))
    }
}

impl From<&str> for Namespace {
    fn from(namespace: &str) -> Namespace {
        let values = namespace.split('/').map(|n| n.to_string()).collect();
        Self { values }
    }
}

impl TryFrom<&[u8]> for Namespace {
    type Error = str::Utf8Error;

    fn try_from(namespace: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(namespace).map(Namespace::from)
    }
}

impl TryFrom<git2::Reference<'_>> for Namespace {
    type Error = str::Utf8Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let re = regex::Regex::new(r"refs/namespaces/([^/]+)/").unwrap();
        let ref_name = str::from_utf8(reference.name_bytes())?;
        let values = re
            .find_iter(ref_name)
            .map(|m| {
                String::from(
                    m.as_str()
                        .trim_start_matches("refs/namespaces/")
                        .trim_end_matches('/'),
                )
            })
            .collect::<Vec<_>>()
            .to_vec();

        Ok(Namespace { values })
    }
}

pub(super) mod git_ext {
    /// Try to strip any refs/namespaces, refs/heads, refs/remotes, and
    /// refs/tags. If this fails we return the original string.
    pub fn try_extract_refname(spec: &str) -> Result<&str, &str> {
        let re =
            regex::Regex::new(r"(refs/namespaces/.*)*(refs/heads/|refs/remotes/|refs/tags/)(.*)")
                .unwrap();

        re.captures(spec)
            .and_then(|c| c.get(3).map(|m| m.as_str()))
            .ok_or(spec)
    }

    /// [`git2::Reference::is_tag`] just does a check for the prefix of `tags/`.
    /// This issue with that is, as soon as we're in 'namespaces' ref that
    /// is a tag it will say that it's not a tag. Instead we do a regex check on
    /// `refs/tags/.*`.
    pub fn is_tag(reference: &git2::Reference) -> bool {
        let re = regex::Regex::new(r"refs/tags/.*").unwrap();
        // If we couldn't parse the name we say it's not a tag.
        match reference.name() {
            Some(name) => re.is_match(name),
            None => false,
        }
    }

    pub fn is_branch(reference: &git2::Reference) -> bool {
        let re = regex::Regex::new(r"refs/heads/.*|refs/remotes/.*/.*").unwrap();
        // If we couldn't parse the name we say it's not a branch.
        match reference.name() {
            Some(name) => re.is_match(name),
            None => false,
        }
    }
}
