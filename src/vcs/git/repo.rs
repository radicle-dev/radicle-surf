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

use crate::{
    diff,
    diff::*,
    file_system,
    vcs,
    vcs::{
        git::{
            error::*,
            object::{Branch, Commit, Namespace, RevObject, Signature, Tag},
            reference::glob::RefGlob,
        },
        VCS,
    },
};
use git2::{BranchType, Oid};
use nonempty::NonEmpty;
use std::{convert::TryFrom, str};

/// This is for flagging to the `file_history` function that it should
/// stop at the first (i.e. Last) commit it finds for a file.
pub(super) enum CommitHistory {
    Full,
    Last,
}

/// A `History` that uses `git2::Commit` as the underlying artifact.
pub type History = vcs::History<Commit>;

/// Wrapper around the `git2`'s `git2::Repository` type.
/// This is to to limit the functionality that we can do
/// on the underlying object.
pub struct Repository(pub(super) git2::Repository);

/// A reference-only `Repository`. This means that we cannot mutate the
/// underlying `Repository`. Not being able to mutate the `Repository` means
/// that the functions defined for `RepositoryRef` should be thread-safe.
///
/// # Construction
///
/// Use the `From<&'a git2::Repository>` implementation to construct a
/// `RepositoryRef`.
pub struct RepositoryRef<'a> {
    pub(super) repo_ref: &'a git2::Repository,
}

// RepositoryRef should be safe to transfer across thread boundaries since it
// only holds a reference to git2::Repository. git2::Repository is also Send
// (see: https://docs.rs/git2/0.13.5/src/git2/repo.rs.html#46)
unsafe impl<'a> Send for RepositoryRef<'a> {}

impl<'a> From<&'a git2::Repository> for RepositoryRef<'a> {
    fn from(repo_ref: &'a git2::Repository) -> Self {
        RepositoryRef { repo_ref }
    }
}

impl<'a> RepositoryRef<'a> {
    /// What is the current namespace we're browsing in.
    pub fn which_namespace(&self) -> Result<Option<Namespace>, Error> {
        Ok(self
            .repo_ref
            .namespace_bytes()
            .map(Namespace::try_from)
            .transpose()?)
    }

    /// List the branches within a repository, filtering out ones that do not
    /// parse correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn list_branches(&self, filter: Option<BranchType>) -> Result<Vec<Branch>, Error> {
        let ref_glob = filter.map_or(RefGlob::Branch, RefGlob::from_branch_type);

        ref_glob
            .references(&self)?
            .iter()
            .try_fold(vec![], |mut acc, reference| {
                let branch = Branch::try_from(reference?)?;
                acc.push(branch);
                Ok(acc)
            })
    }

    /// List the tags within a repository, filtering out ones that do not parse
    /// correctly.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn list_tags(&self) -> Result<Vec<Tag>, Error> {
        RefGlob::Tag
            .references(&self)?
            .iter()
            .try_fold(vec![], |mut acc, reference| {
                let tag = Tag::try_from(reference?)?;
                acc.push(tag);
                Ok(acc)
            })
    }

    /// Create a [`RevObject`] given a
    /// [`revspec`](https://git-scm.com/docs/git-rev-parse#_specifying_revisions) string.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    /// * [`Error::RevParseFailure`]
    pub fn rev(&self, spec: &str) -> Result<RevObject, Error> {
        RevObject::from_revparse(&self.repo_ref, spec)
    }

    /// Create a [`History`] given a
    /// [`revspec`](https://git-scm.com/docs/git-rev-parse#_specifying_revisions) string.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    /// * [`Error::RevParseFailure`]
    pub fn revspec(&self, spec: &str) -> Result<History, Error> {
        let rev = self.rev(spec)?;
        let commit = rev.into_commit(&self.repo_ref)?;
        self.commit_to_history(commit)
    }

    /// Get the [`Diff`] between two commits.
    pub fn diff(&self, from: &'a git2::Commit, to: &'a git2::Commit) -> Result<Diff, Error> {
        use git2::{Delta, Patch};

        let mut diff = Diff::new();
        let git_diff = self.diff_commits(None, Some(from), to)?;

        for (idx, delta) in git_diff.deltas().enumerate() {
            match delta.status() {
                Delta::Added => {
                    let diff_file = delta.new_file();
                    let path = diff_file.path().ok_or(diff::git::Error::PathUnavailable)?;
                    let path = file_system::Path::try_from(path.to_path_buf())?;

                    diff.add_created_file(path);
                },
                Delta::Deleted => {
                    let diff_file = delta.old_file();
                    let path = diff_file.path().ok_or(diff::git::Error::PathUnavailable)?;
                    let path = file_system::Path::try_from(path.to_path_buf())?;

                    diff.add_deleted_file(path);
                },
                Delta::Modified => {
                    let diff_file = delta.new_file();
                    let path = diff_file.path().ok_or(diff::git::Error::PathUnavailable)?;
                    let path = file_system::Path::try_from(path.to_path_buf())?;

                    let patch = Patch::from_diff(&git_diff, idx)?;

                    if let Some(patch) = patch {
                        let mut hunks: Vec<Hunk> = Vec::new();

                        for h in 0..patch.num_hunks() {
                            let (hunk, hunk_lines) = patch.hunk(h)?;
                            let header = hunk.header().to_owned();
                            let mut lines: Vec<LineDiff> = Vec::new();

                            for l in 0..hunk_lines {
                                let line = patch.line_in_hunk(h, l)?;
                                let line = LineDiff::try_from(line)?;
                                lines.push(line);
                            }
                            hunks.push(Hunk { header, lines });
                        }
                        diff.add_modified_file(path, hunks);
                    } else if diff_file.is_binary() {
                        diff.add_modified_binary_file(path);
                    } else {
                        return Err(diff::git::Error::PatchUnavailable(path).into());
                    }
                },
                Delta::Renamed => {
                    let old = delta
                        .old_file()
                        .path()
                        .ok_or(diff::git::Error::PathUnavailable)?;
                    let new = delta
                        .new_file()
                        .path()
                        .ok_or(diff::git::Error::PathUnavailable)?;

                    let old_path = file_system::Path::try_from(old.to_path_buf())?;
                    let new_path = file_system::Path::try_from(new.to_path_buf())?;

                    diff.add_moved_file(old_path, new_path);
                },
                Delta::Copied => {
                    let old = delta
                        .old_file()
                        .path()
                        .ok_or(diff::git::Error::PathUnavailable)?;
                    let new = delta
                        .new_file()
                        .path()
                        .ok_or(diff::git::Error::PathUnavailable)?;

                    let old_path = file_system::Path::try_from(old.to_path_buf())?;
                    let new_path = file_system::Path::try_from(new.to_path_buf())?;

                    diff.add_copied_file(old_path, new_path);
                },
                status => {
                    return Err(diff::git::Error::DeltaUnhandled(status).into());
                },
            }
        }

        Ok(diff)
    }

    pub(super) fn switch_namespace(&self, namespace: &str) -> Result<(), Error> {
        Ok(self.repo_ref.set_namespace(namespace)?)
    }

    /// Get a particular `Commit`.
    pub(super) fn get_commit(&self, oid: Oid) -> Result<git2::Commit<'a>, Error> {
        let commit = self.repo_ref.find_commit(oid)?;
        Ok(commit)
    }

    /// Build a [`History`] using the `head` reference.
    pub(super) fn head(&self) -> Result<History, Error> {
        let head = self.repo_ref.head()?;
        self.to_history(&head)
    }

    /// Turn a [`git2::Reference`] into a [`History`] by completing
    /// a revwalk over the first commit in the reference.
    pub(super) fn to_history(&self, history: &git2::Reference<'a>) -> Result<History, Error> {
        let head = history.peel_to_commit()?;
        self.commit_to_history(head)
    }

    /// Turn a [`git2::Reference`] into a [`History`] by completing
    /// a revwalk over the first commit in the reference.
    pub(super) fn commit_to_history(&self, head: git2::Commit) -> Result<History, Error> {
        let head_id = head.id();
        let mut commits = NonEmpty::new(Commit::try_from(head)?);
        let mut revwalk = self.repo_ref.revwalk()?;

        // Set the revwalk to the head commit
        revwalk.push(head_id)?;

        for commit_result_id in revwalk {
            // The revwalk iter returns results so
            // we unpack these and push them to the history
            let commit_id: Oid = commit_result_id?;

            // Skip the head commit since we have processed it
            if commit_id == head_id {
                continue;
            }

            let commit = Commit::try_from(self.repo_ref.find_commit(commit_id)?)?;
            commits.push(commit);
        }

        Ok(vcs::History(commits))
    }

    /// Extract the signature from a commit
    ///
    /// # Arguments
    ///
    /// `commit_oid` - The object ID of the commit
    /// `field` - the name of the header field containing the signature block;
    ///           pass `None` to extract the default 'gpgsig'
    pub(super) fn extract_signature(
        &self,
        commit_oid: &Oid,
        field: Option<&str>,
    ) -> Result<Option<Signature>, Error> {
        // Match is necessary here because according to the documentation for
        // git_commit_extract_signature at
        // https://libgit2.org/libgit2/#HEAD/group/commit/git_commit_extract_signature
        // the return value for a commit without a signature will be GIT_ENOTFOUND
        match self.repo_ref.extract_signature(commit_oid, field) {
            Err(error) => {
                if error.code() == git2::ErrorCode::NotFound {
                    Ok(None)
                } else {
                    Err(error.into())
                }
            },
            Ok(sig) => Ok(Some(Signature::from_buf(sig.0))),
        }
    }

    pub(crate) fn revision_branches(&self, oid: &Oid) -> Result<Vec<Branch>, Error> {
        let branches = self
            .repo_ref
            .branches(Some(BranchType::Local))?
            .collect::<Result<Vec<(git2::Branch, BranchType)>, git2::Error>>()?;

        let mut contained_branches = vec![];

        branches.into_iter().try_for_each(|(branch, locality)| {
            self.reachable_from(&branch.get(), &oid)
                .and_then(|contains| {
                    if contains {
                        let branch = Branch::from_git_branch(branch, locality)?;
                        contained_branches.push(branch);
                    }
                    Ok(())
                })
        })?;

        Ok(contained_branches)
    }

    fn reachable_from(&self, reference: &git2::Reference, oid: &Oid) -> Result<bool, Error> {
        let other = reference.peel_to_commit()?.id();
        let is_descendant = self.repo_ref.graph_descendant_of(other, *oid)?;

        Ok(other == *oid || is_descendant)
    }

    /// Get the history of the file system where the head of the [`NonEmpty`] is
    /// the latest commit.
    pub(super) fn file_history(
        &self,
        path: &file_system::Path,
        commit_history: CommitHistory,
        commit: Commit,
    ) -> Result<Vec<Commit>, Error> {
        let mut revwalk = self.repo_ref.revwalk()?;
        let mut commits = vec![];

        // Set the revwalk to the head commit
        revwalk.push(commit.id)?;

        for commit in revwalk {
            let parent_id: Oid = commit?;
            let parent = self.repo_ref.find_commit(parent_id)?;
            let paths = self.diff_commit_and_parents(path, &parent)?;
            if let Some(_path) = paths {
                commits.push(Commit::try_from(parent)?);
                match &commit_history {
                    CommitHistory::Last => break,
                    CommitHistory::Full => {},
                }
            }
        }

        Ok(commits)
    }

    fn diff_commit_and_parents(
        &self,
        path: &file_system::Path,
        commit: &git2::Commit,
    ) -> Result<Option<file_system::Path>, Error> {
        let mut parents = commit.parents();
        let parent = parents.next();

        let diff = self.diff_commits(Some(path), parent.as_ref(), &commit)?;
        if let Some(_delta) = diff.deltas().next() {
            Ok(Some(path.clone()))
        } else {
            Ok(None)
        }
    }

    fn diff_commits(
        &self,
        path: Option<&file_system::Path>,
        old_tree: Option<&'a git2::Commit>,
        new_tree: &'a git2::Commit,
    ) -> Result<git2::Diff, Error> {
        let new_tree = new_tree.tree()?;
        let old_tree = old_tree.map_or(Ok(None), |commit| commit.tree().map(Some))?;

        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            opts.pathspec(path);
            // We're skipping the binary pass because we won't be inspecting deltas.
            opts.skip_binary_check(true);
        }

        let diff =
            self.repo_ref
                .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))?;

        Ok(diff)
    }
}

impl<'a> VCS<Commit, Error> for RepositoryRef<'a> {
    type HistoryId = String;
    type ArtefactId = Oid;

    fn default_history_id(&self) -> Result<Self::HistoryId, Error> {
        let reference = self.repo_ref.find_reference("refs/remotes/origin/HEAD")?;
        Ok(reference.symbolic_target().unwrap().to_string())
    }

    fn get_history(&self, history_id: Self::HistoryId) -> Result<History, Error> {
        self.revspec(&history_id)
    }

    fn get_histories(&self) -> Result<Vec<History>, Error> {
        self.repo_ref
            .references()
            .map_err(Error::from)
            .and_then(|mut references| {
                references.try_fold(vec![], |mut acc, reference| {
                    reference.map_err(Error::from).and_then(|r| {
                        let history = self.to_history(&r)?;
                        acc.push(history);
                        Ok(acc)
                    })
                })
            })
    }

    fn get_identifier(artifact: &Commit) -> Self::ArtefactId {
        artifact.id
    }
}

impl<'a> std::fmt::Debug for RepositoryRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

impl Repository {
    /// Open a git repository given its URI.
    ///
    /// # Errors
    ///
    /// * [`Error::Git`]
    pub fn new(repo_uri: &str) -> Result<Self, Error> {
        git2::Repository::open(repo_uri)
            .map(Repository)
            .map_err(Error::from)
    }

    /// Since our operations are read-only when it comes to surfing a repository
    /// we have a separate struct called [`RepositoryRef`]. This turns an owned
    /// [`Repository`], the one returend by [`Repository::new`], into a
    /// [`RepositoryRef`].
    pub fn as_ref(&'_ self) -> RepositoryRef<'_> {
        RepositoryRef { repo_ref: &self.0 }
    }
}

impl<'a> From<&'a Repository> for RepositoryRef<'a> {
    fn from(repo: &'a Repository) -> Self {
        repo.as_ref()
    }
}

impl vcs::GetVCS<Error> for Repository {
    type RepoId = String;

    fn get_repo(repo_id: Self::RepoId) -> Result<Self, Error> {
        git2::Repository::open(&repo_id)
            .map(Repository)
            .map_err(Error::from)
    }
}

impl From<git2::Repository> for Repository {
    fn from(repo: git2::Repository) -> Self {
        Repository(repo)
    }
}

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".git")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcs::{git::error::Error, VCS};

    #[test]
    fn default_history_id() -> Result<(), Error> {
        let repo = Repository::new("./data/git-platinum")?;
        let repo = repo.as_ref();
        assert_eq!(
            repo.default_history_id(),
            Ok("refs/remotes/origin/master".to_string())
        );
        Ok(())
    }
}
