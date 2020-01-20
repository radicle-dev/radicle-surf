#![deny(unused_import_braces, unused_qualifications, warnings)]

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
//! use radicle_surf::vcs::git;
//! use radicle_surf::file_system::{Label, Path, SystemType};
//! use radicle_surf::file_system::unsound;
//! use pretty_assertions::assert_eq;
//!
//! // We're going to point to this repo.
//! let repo = git::Repository::new("./data/git-platinum").expect("Failed to initialise repo");
//!
//! // Here we initialise a new Broswer for a the git repo.
//! let mut browser = git::Browser::new(repo).expect("Failed to initialise browser");
//!
//! // Set the history to a particular commit
//! browser.commit(git::Sha1::new("80ded66281a4de2889cc07293a8f10947c6d57fe"))
//!        .expect("Failed to set commit");
//!
//! // Get the snapshot of the directory for our current
//! // HEAD of history.
//! let directory = browser.get_directory().expect("Failed to get directory");
//!
//! // Let's get a Path to this file
//! let this_file = Path::from_labels(unsound::label::new("src"), &[unsound::label::new("memory.rs")]);
//!
//! // And assert that we can find it!
//! assert!(directory.find_file(&this_file).is_some());
//!
//! let mut root_contents = directory.list_directory();
//! root_contents.sort();
//!
//! assert_eq!(root_contents, vec![
//!     SystemType::file(unsound::label::new(".i-am-well-hidden")),
//!     SystemType::file(unsound::label::new(".i-too-am-hidden")),
//!     SystemType::file(unsound::label::new("README.md")),
//!     SystemType::directory(unsound::label::new("bin")),
//!     SystemType::directory(unsound::label::new("src")),
//!     SystemType::directory(unsound::label::new("text")),
//!     SystemType::directory(unsound::label::new("this")),
//! ]);
//!
//! let src = directory.find_directory(
//!     &Path::new(unsound::label::new("src"))
//! ).expect("Failed to find src");
//!
//! let mut src_contents = src.list_directory();
//! src_contents.sort();
//!
//! assert_eq!(src_contents, vec![
//!     SystemType::file(unsound::label::new("Eval.hs")),
//!     SystemType::file(unsound::label::new("Folder.svelte")),
//!     SystemType::file(unsound::label::new("memory.rs")),
//! ]);
//! ```
pub mod diff;
pub mod file_system;
pub mod vcs;

// Private modules
mod nonempty;
mod tree;

pub use crate::vcs::git;
