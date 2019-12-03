# radicle-surf

A code surfing library for VCS file systems 🏄‍♀️🏄‍♂️

Welcome to `radicle-surf`!

`radicle-surf` is a system to describe a file-system in a VCS world.
We have the concept of files and directories, but these objects can change over time while people iterate on them.
Thus, it is a file-system within history and we, the user, are viewing the file-system at a particular snapshot.
Alongside this, we will wish to take two snapshots and view their differences.

Let's start surfing (and apologies for the `unwrap`s):

```rust
use radicle_surf::vcs::git::{GitBrowser, GitRepository};
use radicle_surf::file_system::{Label, Path, SystemType};

// We're going to point to this repo.
let repo = GitRepository::new(".").unwrap();

// Here we initialise a new Broswer for a the git repo.
let browser = GitBrowser::new(&repo).unwrap();

// Get the snapshot of the directory for our current
// HEAD of history.
let directory = browser.get_directory().unwrap();

// Let's get a Path to this file
let this_file = Path::with_root(&["src".into(), "lib.rs".into()]);

// And assert that we can find it!
assert!(directory.find_file(&this_file).is_some());

let mut root_contents = directory.list_directory();
root_contents.sort();

assert_eq!(root_contents, vec![
  SystemType::directory(".buildkite".into()),
  SystemType::directory(".docker".into()),
  SystemType::directory(".git".into()),
  SystemType::file(".gitignore".into()),
  SystemType::file("Cargo.toml".into()),
  SystemType::file("README.md".into()),
  SystemType::directory("data".into()),
  SystemType::directory("docs".into()),
  SystemType::directory("src".into()),
]);

let src = directory.find_directory(&Path::with_root(&["src".into()])).unwrap();
let mut src_contents = src.list_directory();
src_contents.sort();

assert_eq!(src_contents, vec![
  SystemType::directory("diff".into()),
  SystemType::directory("file_system".into()),
  SystemType::file("lib.rs".into()),
  SystemType::directory("vcs".into()),
]);
```
