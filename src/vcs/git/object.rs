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

use crate::vcs::git::{error::Error, repo::RepositoryRef};
pub use git2::Oid;
use std::{cmp::Ordering, convert::TryFrom, fmt, str};

#[cfg(feature = "serialize")]
use serde::Serialize;

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
    type Error = str::Utf8Error;

    fn try_from(commit: git2::Commit) -> Result<Self, Self::Error> {
        let id = commit.id();
        let author = Author::try_from(commit.author())?;
        let committer = Author::try_from(commit.committer())?;
        let message_raw = commit.message_bytes();
        let message = str::from_utf8(message_raw)?.into();
        let summary_raw = commit.summary_bytes().expect("TODO");
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

/// A newtype wrapper over `String` to separate out the fact that a caller wants
/// to fetch a branch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchName(String);

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

/// A newtype wrapper over `String` to separate out the fact that a caller wants
/// to fetch a tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagName(String);

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<&[u8]> for TagName {
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

impl TagName {
    /// Create a new `TagName`.
    pub fn new(name: &str) -> Self {
        TagName(name.into())
    }

    /// Access the string value of the `TagName`.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// The static information of a [`git2::Tag`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    /// A light-weight git tag.
    Light {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The name that references this `Tag`.
        name: TagName,
    },
    /// An annotated git tag.
    Annotated {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The name that references this `Tag`.
        name: TagName,
        /// The named author of this `Tag`, if the `Tag` was annotated.
        tagger: Option<Author>,
        /// The message with this `Tag`, if the `Tag` was annotated.
        message: Option<String>,
    },
}

impl Tag {
    /// Get the `Oid` of the tag, regardless of its type.
    pub fn id(&self) -> Oid {
        match self {
            Self::Light { id, .. } => *id,
            Self::Annotated { id, .. } => *id,
        }
    }

    /// Get the `TagName` of the tag, regardless of its type.
    pub fn name(&self) -> TagName {
        match self {
            Self::Light { name, .. } => name.clone(),
            Self::Annotated { name, .. } => name.clone(),
        }
    }
}

impl<'repo> TryFrom<git2::Tag<'repo>> for Tag {
    type Error = str::Utf8Error;

    fn try_from(tag: git2::Tag) -> Result<Self, Self::Error> {
        let id = tag.id();

        let name = TagName::try_from(tag.name_bytes())?;

        let tagger = tag.tagger().map(Author::try_from).transpose()?;

        let message = tag
            .message_bytes()
            .map(str::from_utf8)
            .transpose()?
            .map(|message| message.into());

        Ok(Tag::Annotated {
            id,
            name,
            tagger,
            message,
        })
    }
}

impl<'repo> TryFrom<git2::Reference<'repo>> for Tag {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let name = TagName::try_from(reference.name_bytes())?;

        if !git_ext::is_tag(&reference) {
            return Err(Error::NotTag(name));
        }

        match reference.peel_to_tag() {
            Ok(tag) => Ok(Tag::try_from(tag)?),
            Err(err) => {
                // If we get an error peeling to a tag _BUT_ we also have confirmed the
                // reference is a tag, that means we have a lightweight tag,
                // i.e. a commit SHA and name.
                if err.class() == git2::ErrorClass::Object
                    && err.code() == git2::ErrorCode::InvalidSpec
                {
                    let commit = reference.peel_to_commit()?;
                    Ok(Tag::Light {
                        id: commit.id(),
                        name,
                    })
                } else {
                    Err(err.into())
                }
            },
        }
    }
}

/// A `RevObject` encapsulates the idea of providing a "revspec" to git and
/// getting back the desired object.
///
/// `RevObject` can in turn be used by [`rev`](type.Browser.html#method.rev) to
/// set the [`crate::vcs::git::Browser`]'s [`crate::vcs::git::History`] with the
/// object.
///
/// See here for the [specifying revision](https://git-scm.com/docs/git-rev-parse.html#_specifying_revisions).
pub enum RevObject {
    /// A [`Branch`] revision.
    Branch(Branch),
    /// A [`Tag`] revision.
    Tag(Tag),
    /// A [`Commit`] revision.
    Commit(Commit),
}

impl RevObject {
    /// Create a `RevObject` by calling
    /// [`revparse_ext`](https://docs.rs/git2/0.11.0/git2/struct.Repository.html#method.revparse_ext)
    /// and attempting to turn the resulting `Object` into a [`Tag`] or a
    /// [`Commit`]. If this fails we attempt to see if the
    /// [`git2::Reference`] is present and is a [`Branch`].
    ///
    /// # Errors
    ///
    /// * `Error::Git` if the `revspec` provided fails to parse
    /// * `Error::RevParseFailure` if conversion to a target object fail.
    pub fn from_revparse<'a>(repo: &RepositoryRef<'a>, spec: &str) -> Result<Self, Error> {
        let (object, optional_ref) = repo.repo_ref.revparse_ext(spec)?;

        let tag = object.into_tag().map(Tag::try_from);
        match tag {
            Ok(tag) => Ok(RevObject::Tag(tag?)),
            Err(object) => {
                let commit = object.into_commit().map(Commit::try_from);
                match commit {
                    Ok(commit) => Ok(RevObject::Commit(commit?)),
                    Err(_object) => match optional_ref {
                        Some(reference) => Branch::try_from(reference).map(RevObject::Branch),
                        None => Err(Error::RevParseFailure {
                            rev: spec.to_string(),
                        }),
                    },
                }
            },
        }
    }

    /// Peel back a `RevObject` into a [`git2::Commit`].
    ///
    /// In the case of the `RevObject` itself being a [`Commit`] it is trivial.
    /// In the case of the `RevObject` being a [`Tag`] or [`Branch`], we first
    /// get the object/reference and then get the commit it points to.
    pub(crate) fn into_commit(self, repo: &git2::Repository) -> Result<git2::Commit, Error> {
        match self {
            RevObject::Branch(branch) => {
                let reference = repo
                    .find_branch(&branch.name.0, branch.locality.into())?
                    .into_reference();
                let commit = reference.peel_to_commit()?;
                Ok(commit)
            },
            RevObject::Tag(tag) => {
                let object = repo.find_tag(tag.id())?.into_object();
                let commit = object.peel_to_commit()?;
                Ok(commit)
            },
            RevObject::Commit(commit) => Ok(repo.find_commit(commit.id)?),
        }
    }
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
