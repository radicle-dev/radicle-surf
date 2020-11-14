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

use git2::Oid;
use std::{convert::TryFrom, str};

use radicle_git_ext::{self as ext, OneLevel, RefLike};

use crate::vcs::git::{self, Author};

/// An error occurred attempting to parse a [`Tag`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An unexpected [`git2:::Error`] occurred.
    #[error(transparent)]
    Git(#[from] git2::Error),
    /// The [`git::Reference`] name was invalid.
    #[error(transparent)]
    Name(#[from] ext::name::Error),
    /// The user tried to fetch a tag, but the name provided does not
    /// exist as a tag. This could mean that the tag does not exist
    /// or that a branch or commit was provided by accident.
    #[error("the reference `{0}` is not a tag")]
    NotTag(RefLike),
    /// The [`git::Reference`] could not successfully remove the namespace.
    #[error(transparent)]
    Strip(#[from] ext::name::StripPrefixError),
    /// Failed to parse UTF-8 while building a [`Tag`].
    #[error(transparent)]
    Utf8(#[from] str::Utf8Error),
}

/// The static information of a [`git2::Tag`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    /// A light-weight git tag.
    Light {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The name that references this `Tag`.
        name: OneLevel,
    },
    /// An annotated git tag.
    Annotated {
        /// The Object ID for the `Tag`, i.e the SHA1 digest.
        id: Oid,
        /// The name that references this `Tag`.
        name: OneLevel,
        /// The named author of this `Tag`, if the `Tag` was annotated.
        tagger: Option<Author>,
        /// The message with this `Tag`, if the `Tag` was annotated.
        message: Option<String>,
    },
}

impl Tag {
    /// Construct a [`Tag::Light`].
    pub fn light(id: Oid, name: impl Into<OneLevel>) -> Tag {
        Self::Light {
            id,
            name: name.into(),
        }
    }

    /// Construct a [`Tag::Annotated`].
    pub fn annotated(
        id: Oid,
        name: impl Into<OneLevel>,
        tagger: impl Into<Option<Author>>,
        message: impl Into<Option<String>>,
    ) -> Tag {
        Self::Annotated {
            id,
            name: name.into(),
            tagger: tagger.into(),
            message: message.into(),
        }
    }

    /// Get the `Oid` of the tag, regardless of its type.
    pub fn id(&self) -> Oid {
        match self {
            Self::Light { id, .. } => *id,
            Self::Annotated { id, .. } => *id,
        }
    }

    /// Get the `TagName` of the tag, regardless of its type.
    pub fn name(&self) -> OneLevel {
        match self {
            Self::Light { name, .. } => name.clone(),
            Self::Annotated { name, .. } => name.clone(),
        }
    }
}

impl<'repo> TryFrom<git2::Tag<'repo>> for Tag {
    type Error = Error;

    fn try_from(tag: git2::Tag) -> Result<Self, Self::Error> {
        let id = tag.id();

        let name = ext::RefLike::try_from(tag.name_bytes())?;

        let tagger = tag.tagger().map(Author::try_from).transpose()?;

        let message = tag
            .message_bytes()
            .map(str::from_utf8)
            .transpose()?
            .map(|message| message.into());

        Ok(Tag::annotated(id, name, tagger, message))
    }
}

impl<'repo> TryFrom<git2::Reference<'repo>> for Tag {
    type Error = Error;

    fn try_from(reference: git2::Reference) -> Result<Self, Self::Error> {
        let name = ext::RefLike::try_from(reference.name_bytes())?;
        let name = git::ext::remove_namespace(name)?;

        if !git::ext::is_tag(&reference) {
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
                    Ok(Tag::light(commit.id(), name))
                } else {
                    Err(err.into())
                }
            },
        }
    }
}
