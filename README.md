# radicle-surf

A code surfing library for VCS file systems 🏄‍♀️🏄‍♂️

Welcome to `radicle-surf`!

`radicle-surf` is a system to describe a file-system in a VCS world.
We have the concept of files and directories, but these objects can change over time while people iterate on them.
Thus, it is a file-system within history and we, the user, are viewing the file-system at a particular snapshot.
Alongside this, we will wish to take two snapshots and view their differences.

## Contributing

To get started on contributing you can check out our [devloping guide](./DEVELOPMENT.md), and also
our [LICENSE](./LICENSE) & [CONTRIBUTING](./CONTRIBUTING) files.

## The Community

Join our community disccussions at [radicle.community](https://radicle.community)!

# Example

To a taste for the capabilities of `radicle-surf` we provide an example below, but we also
keep our documentation and doc-tests up to date.

```rust
use radicle_surf::vcs::git;
use radicle_surf::file_system::{Label, Path, SystemType};

// We're going to point to this repo.
let repo = git::Repository::new(".")?;

// Here we initialise a new Browser for the git repo.
let mut browser = git::Browser::new(repo)?;

// Set the history to a particular commit
let commit = git::Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?;
browser.commit(commit)?;

// Get the snapshot of the directory for our current HEAD of history.
let directory = browser.get_directory()?;

// Let's get a Path to the lib.rs file
let lib = unsound::path::new("src/lib.rs");

// And assert that we can find it!
assert!(directory.find_file(&lib).is_some());

let root_contents = directory.list_directory();
root_contents.sort();

assert_eq!(root_contents, vec![
  SystemType::directory(".buildkite".into()),
  SystemType::directory(".docker".into()),
  SystemType::directory(".git".into()),
  SystemType::file(".gitignore".into()),
  SystemType::file(".gitmodules".into()),
  SystemType::file(".rustfmt.toml".into()),
  SystemType::file(".rust-toolchain".into()),
  SystemType::file("Cargo.toml".into()),
  SystemType::file("README.md".into()),
  SystemType::directory("data".into()),
  SystemType::directory("docs".into()),
  SystemType::directory("examples".into()),
  SystemType::directory("src".into()),
]);

let src = directory
  .find_directory(&Path::new(unsound::label::new("src")))
  .expect("failed to find src");

let src_contents = src.list_directory();
src_contents.sort();

assert_eq!(src_contents, vec![
  SystemType::directory("diff".into()),
  SystemType::directory("file_system".into()),
  SystemType::file("lib.rs".into()),
  SystemType::directory("vcs".into()),
]);
```
