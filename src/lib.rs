//! Welcome to `radicle-surf`!
//!
//! `radicle-surf` is a system to describe a file-system in a VCS world.
//! We have the concept of files and directories, but these objects can change over time while people iterate on them.
//! Thus, it is a file-system within history and we, the user, are viewing the file-system at a particular snapshot.
//! Alongside this, we will wish to take two snapshots and view their differences.
//!
//! Let's start surfing (and apologies for the `unwrap`s):
//!
//! ```
//! use radicle_surf::vcs::git::{GitBrowser, GitRepository};
//! use radicle_surf::file_system::{Label, Path};
//!
//! // We're going to point to this repo.
//! let repo = GitRepository::new(".").unwrap();
//!
//! // Here we initialise a new Broswer for a the git repo.
//! let browser = GitBrowser::new(&repo).unwrap();
//!
//! // Get the snapshot of the directory for our current
//! // HEAD of history.
//! let directory = browser.get_directory().unwrap();
//!
//! // Let's get a Path to this file
//! let this_file = Path::from_labels(Label::root(), &["src".into(), "lib.rs".into()]);
//!
//! // And assert that we can find it!
//! assert!(directory.find_file(&this_file).is_some());
//! ```
pub mod diff;
pub mod file_system;
pub mod vcs;

pub use crate::vcs::git;

#[cfg(test)]
extern crate quickcheck;
