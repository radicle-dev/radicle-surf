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

/// [`git2::Reference::is_tag`] just does a check for the prefix of `tags/`.
/// The issue with that is, as soon as we're in 'namespaces' ref that
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

pub fn is_remote(reference: &git2::Reference) -> bool {
    let re = regex::Regex::new(r"refs/remotes/.*/.*").unwrap();
    // If we couldn't parse the name we say it's not a remote branch.
    match reference.name() {
        Some(name) => re.is_match(name),
        None => false,
    }
}

pub fn remove_namespace(
    reflike: radicle_git_ext::RefLike,
) -> Result<radicle_git_ext::RefLike, radicle_git_ext::name::StripPrefixError> {
    if !reflike.starts_with("refs/namespaces/") {
        return Ok(reflike);
    }

    let suffix = reflike.strip_prefix("refs/namespaces/")?;

    let namespace = suffix
        .components()
        .take_while(|c| c.as_os_str() != "refs")
        .fold(std::path::PathBuf::new(), |n, c| n.join(c.as_os_str()));

    remove_namespace(suffix.strip_prefix(namespace)?)
}
