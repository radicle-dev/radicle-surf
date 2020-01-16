use nonempty::NonEmpty;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub mod error;
pub use crate::file_system::error as file_error;
pub mod directory;
pub mod path;
pub mod unsound;

pub use self::directory::*;
pub use self::path::*;

/// A trait to say how to intitialise a Repository `Directory`.
/// For example, Git would initialise with the `.git` folder and
/// the files contained in it.
pub(crate) trait RepoBackend
where
    Self: Sized,
{
    /// Should result in a root directory
    /// with a `DirectoryContents::Repo` as
    /// its entry.
    ///
    /// For example:
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Path, Directory, DirectoryContents, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let repo = Directory {
    ///     name: Label::root(),
    ///     entries: NonEmpty::new(
    ///         DirectoryContents::SubDirectory(Box::new(
    ///             Directory {
    ///                 name: unsound::label::new(".git"),
    ///                 entries: NonEmpty::new(DirectoryContents::Repo),
    ///             }
    ///         ))
    ///     )
    /// };
    /// ```
    fn repo_directory() -> Directory;
}

/// A `DirectoryContents` is made up of either:
/// * A `SubDirectory`
/// * A `File`
/// * A `Repo`, which is expected to be the
///   special Repository directory, but is opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryContents {
    SubDirectory(Box<Directory>),
    File(File),
    Repo,
}

impl DirectoryContents {
    /// Helper constructor for a `SubDirectory`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{File, Directory, DirectoryContents, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let src = unsound::label::new("src");
    /// let lib_rs = unsound::label::new("lib.rs");
    ///
    /// let lib = Directory {
    ///     name: src,
    ///     entries: NonEmpty::new(DirectoryContents::File(File::new(
    ///         lib_rs,
    ///         b"pub mod file_system;",
    ///     )))
    /// };
    ///
    /// let sub_dir = DirectoryContents::sub_directory(lib.clone());
    ///
    /// assert_eq!(sub_dir, DirectoryContents::SubDirectory(Box::new(lib)));
    /// ```
    pub fn sub_directory(directory: Directory) -> Self {
        DirectoryContents::SubDirectory(Box::new(directory))
    }

    /// Helper constructor for a `File`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{File, Directory, DirectoryContents, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let src = unsound::label::new("src");
    /// let lib_rs = unsound::label::new("lib.rs");
    ///
    /// let lib = Directory {
    ///     name: src,
    ///     entries: NonEmpty::new(DirectoryContents::file(lib_rs, b"pub mod file_system;")),
    /// };
    ///
    /// let sub_dir = DirectoryContents::sub_directory(lib.clone());
    ///
    /// assert_eq!(sub_dir, DirectoryContents::SubDirectory(Box::new(lib)));
    /// ```
    pub fn file(name: Label, contents: &[u8]) -> Self {
        DirectoryContents::File(File::new(name, contents))
    }

    pub fn label(&self) -> Option<&Label> {
        match self {
            DirectoryContents::SubDirectory(dir) => Some(&dir.name),
            DirectoryContents::File(file) => Some(&file.name),
            DirectoryContents::Repo => None,
        }
    }
}

/// A `Directory` consists of its [`Label`](struct.Label.html) and its entries.
/// The entries are a set of [`DirectoryContents`](struct.DirectoryContents.html)
/// and there should be at least one entry.
/// This is because empty directories do not generally exist in VCSes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub name: Label,
    pub entries: NonEmpty<DirectoryContents>,
}

/// A `File` consists of its file name (a [`Label`](struct.Label.html)
/// and its file contents (a `Vec` of bytes).
#[derive(Clone, PartialEq, Eq)]
pub struct File {
    pub name: Label,
    pub contents: Vec<u8>,
    pub(crate) size: usize,
}

impl File {
    pub fn new(name: Label, contents: &[u8]) -> Self {
        let size = contents.len();
        File {
            name,
            contents: contents.to_vec(),
            size,
        }
    }

    /// Get the size of the `File` corresponding to
    /// the number of bytes in the file contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let file = File::new(
    ///     unsound::label::new("lib.rs"),
    ///     b"pub mod diff;\npub mod file_system;\npub mod vcs;\npub use crate::vcs::git;\n",
    /// );
    ///
    /// assert_eq!(file.size(), 73);
    /// ```
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn checksum(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.contents.hash(&mut hasher);
        hasher.finish()
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File {{ name: {:#?} }}", self.name)
    }
}

impl Directory {
    /// An empty root `Directory`, just containing the special repository directory.
    fn empty_root<Repo>() -> Self
    where
        Repo: RepoBackend,
    {
        Directory::mkdir(Label::root(), Repo::repo_directory())
    }

    /// Get the total size, in bytes, of a `Directory`. The size is
    /// the sum of all files that can be reached from this `Directory`.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, DirectoryContents, File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let files = (
    ///     DirectoryContents::file(unsound::label::new("main.rs"), b"println!(\"Hello, world!\")"),
    ///     vec![DirectoryContents::file(unsound::label::new("lib.rs"), b"struct Hello(String)")],
    /// ).into();
    ///
    /// let directory = Directory {
    ///     name: Label::root(),
    ///     entries: files,
    /// };
    ///
    /// assert_eq!(directory.size(), 45);
    /// ```
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Directory, DirectoryContents, File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut entries: NonEmpty<DirectoryContents> = (
    ///     DirectoryContents::file(unsound::label::new("main.rs"), b"println!(\"Hello, world!\")"),
    ///     vec![DirectoryContents::file(unsound::label::new("lib.rs"), b"struct Hello(String)")],
    /// ).into();
    ///
    /// let subdir = DirectoryContents::sub_directory(Directory {
    ///     name: unsound::label::new("test"),
    ///     entries: NonEmpty::new(DirectoryContents::file(
    ///         unsound::label::new("mod.rs"),
    ///         b"assert_eq!(1 + 1, 2);",
    ///     )),
    /// });
    ///
    /// entries.push(subdir);
    ///
    /// let directory = Directory {
    ///     name: Label::root(),
    ///     entries: entries,
    /// };
    ///
    /// assert_eq!(directory.size(), 66);
    /// ```
    pub fn size(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| match entry {
                DirectoryContents::Repo => 0,
                DirectoryContents::File(file) => file.size(),
                DirectoryContents::SubDirectory(directory) => directory.size(),
            })
            .sum()
    }

    /// List the current `Directory`'s files and sub-directories.
    pub fn list_directory(&self) -> Vec<(Label, SystemType)> {
        self.entries
            .iter()
            .cloned()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => {
                    let name = dir.name;
                    if !name.hidden {
                        Some(SystemType::directory(name))
                    } else {
                        None
                    }
                }
                DirectoryContents::File(file) => Some(SystemType::file(file.name)),
                DirectoryContents::Repo => None,
            })
            .collect()
    }

    fn add_contents(&mut self, entries: NonEmpty<DirectoryContents>) {
        self.entries.append(&mut entries.into())
    }

    /// Find a `File` in the directory given the `Path` to
    /// the `File`.
    ///
    /// This operation fails if the path does not lead to
    /// the `File`.
    pub fn find_file(&self, path: &Path) -> Option<File> {
        let (path, name) = path.split_last();
        let path = NonEmpty::from_slice(&path);

        // Find the file in the current directoy if the prefix path is empty.
        // Otherwise find it in the directory found in the given path (if it exists).
        path.map_or(Some(self.clone()), |p| self.find_directory(&Path(p)))
            .and_then(|dir| dir.file_in_directory(&name))
    }

    /// Find a `Directory` in the directory given the `Path` to
    /// the `Directory`.
    ///
    /// This operation fails if the path does not lead to
    /// the `Directory`.
    pub fn find_directory(&self, path: &Path) -> Option<Self> {
        // recursively dig down into sub-directories
        path.iter()
            .try_fold(self.clone(), |dir, label| dir.sub_directory(&label))
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for now
    #[allow(dead_code)]
    fn fuzzy_find(_label: Label) -> Vec<Self> {
        unimplemented!()
    }

    /// Get the sub directories of a `Directory`.
    fn sub_directories(&self) -> Vec<Self> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(*dir.clone()),
                DirectoryContents::File(_) => None,
                DirectoryContents::Repo => None,
            })
            .collect()
    }

    /// Get the sub directories of a `Directory`.
    fn sub_directories_mut(&mut self) -> Vec<&mut Self> {
        self.entries
            .iter_mut()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(dir.as_mut()),
                DirectoryContents::File(_) => None,
                DirectoryContents::Repo => None,
            })
            .collect()
    }

    /// Get the a sub directory of a `Directory` given its name.
    ///
    /// This operation fails if the directory does not exist.
    fn sub_directory(&self, label: &Label) -> Option<Self> {
        self.sub_directories()
            .iter()
            .cloned()
            .find(|directory| directory.name == *label)
    }

    /// Get the a sub directory of a `Directory` given its name.
    ///
    /// This operation fails if the directory does not exist.
    fn sub_directory_mut(&mut self, label: &Label) -> Option<&mut Self> {
        self.sub_directories_mut()
            .into_iter()
            .find(|directory| directory.name == *label)
    }

    /// Get the `File` in the current `Directory` if it exists in
    /// the entries.
    ///
    /// The operation fails if the `File` does not exist in the `Directory`.
    fn file_in_directory(&self, label: &Label) -> Option<File> {
        for entry in self.entries.iter() {
            match entry {
                DirectoryContents::File(file) if file.name == *label => {
                    return Some(file.clone());
                }
                DirectoryContents::File(..) => {}
                DirectoryContents::SubDirectory(_) => {}
                DirectoryContents::Repo => {}
            }
        }
        None
    }

    /// Helper function for creating a `Directory` with a given sub-directory.
    pub(crate) fn mkdir(name: Label, dir: Self) -> Self {
        Directory {
            name,
            entries: NonEmpty::new(DirectoryContents::sub_directory(dir)),
        }
    }

    pub(crate) fn from<Repo>(paths: HashMap<Path, NonEmpty<File>>) -> Self
    where
        Repo: RepoBackend,
    {
        let mut root = Directory::empty_root::<Repo>();
        for (dir, files) in paths {
            let file_entries: NonEmpty<DirectoryContents> =
                files.map(|f| DirectoryContents::File(f.clone()));

            // Root level files can get added directly
            if dir.is_root() {
                root.add_contents(file_entries)
            } else {
                // If our file location is ~/foo/bar/baz.hs we
                // first create bar containing baz.hs then recursively
                // build up from there.
                let (prefix, current) = dir.split_last();

                let mut directory = Directory {
                    name: current,
                    entries: file_entries,
                };
                for label in prefix.into_iter().rev() {
                    directory = Directory::mkdir(label, directory);
                }

                root.combine(&directory)
            }
        }
        root
    }

    fn combine(&mut self, other: &Directory) {
        match self.sub_directory_mut(&other.name) {
            Some(ref mut subdir) => {
                for entry in other.entries.iter() {
                    match entry {
                        DirectoryContents::File(file) => {
                            subdir.entries.push(DirectoryContents::File(file.clone()))
                        }
                        DirectoryContents::Repo => subdir.entries.push(DirectoryContents::Repo),
                        DirectoryContents::SubDirectory(ref dir) => {
                            subdir.combine(dir);
                        }
                    }
                }
            }
            None => {
                self.entries
                    .push(DirectoryContents::sub_directory(other.clone()));
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::file_system::unsound;
    use crate::file_system::*;
    use pretty_assertions::assert_eq;
    use proptest::collection;
    use proptest::prelude::*;
    use std::convert::TryFrom;

    // Safe labels to use in tests
    fn bar() -> Label {
        Label::try_from("bar").expect("bar could not be converted to Label")
    }

    #[derive(Debug, Clone)]
    struct TestRepo {}

    impl RepoBackend for TestRepo {
        fn repo_directory() -> Directory {
            Directory {
                name: Label {
                    label: "test".into(),
                    hidden: true,
                },
                entries: NonEmpty::new(DirectoryContents::Repo),
            }
        }
    }

    #[test]
    fn test_find_added_file() {
        let file_path = Path::new(unsound::label::new("foo.hs"));

        let file = File::new(unsound::label::new("foo.hs"), b"module Banana ...");

        let directory: Directory = Directory {
            name: Label::root(),
            entries: NonEmpty::new(DirectoryContents::File(file.clone())),
        };

        // Search for "~/foo.hs"
        assert_eq!(directory.find_file(&file_path), Some(file))
    }

    #[test]
    fn test_find_added_file_long_path() {
        let file_path = unsound::path::new("foo/bar/baz.hs");

        let file = File::new(unsound::label::new("baz.hs"), b"module Banana ...");

        let directory: Directory = Directory::mkdir(
            Label::root(),
            Directory::mkdir(
                unsound::label::new("foo"),
                Directory {
                    name: bar(),
                    entries: NonEmpty::new(DirectoryContents::File(file.clone())),
                },
            ),
        );

        // Search for "~/foo/bar/baz.hs"
        assert_eq!(directory.find_file(&file_path), Some(file))
    }

    #[test]
    fn test_404_file_not_found() {
        let file_path = Path::with_root(&[unsound::label::new("bar.hs")]);

        let directory: Directory = Directory {
            name: Label::root(),
            entries: NonEmpty::new(DirectoryContents::file(
                unsound::label::new("foo.hs"),
                "module Banana ...".as_bytes(),
            )),
        };

        // Search for "~/bar.hs"
        assert_eq!(directory.find_file(&file_path), None)
    }

    #[test]
    fn test_list_directory() {
        let foo = DirectoryContents::file(
            unsound::label::new("foo.hs"),
            "module Banana ...".as_bytes(),
        );
        let bar = DirectoryContents::file(
            unsound::label::new("bar.hs"),
            "module Banana ...".as_bytes(),
        );
        let baz = DirectoryContents::file(
            unsound::label::new("baz.hs"),
            "module Banana ...".as_bytes(),
        );

        let directory: Directory = Directory {
            name: Label::root(),
            entries: (foo, vec![bar, baz]).into(),
        };

        assert_eq!(
            directory.list_directory(),
            vec![
                SystemType::file(unsound::label::new("foo.hs")),
                SystemType::file(unsound::label::new("bar.hs")),
                SystemType::file(unsound::label::new("baz.hs")),
            ]
        );
    }

    #[test]
    fn test_create_and_list() {
        let mut directory_map = HashMap::new();

        // Root files set up
        let root_files = (
            File::new(unsound::label::new("foo.rs"), b"use crate::bar"),
            vec![File::new(
                unsound::label::new("bar.rs"),
                b"fn hello_world()",
            )],
        )
            .into();
        directory_map.insert(Path::root(), root_files);

        // Haskell files set up
        let haskell_files = (
            File::new(unsound::label::new("foo.hs"), b"module Foo where"),
            vec![File::new(
                unsound::label::new("bar.hs"),
                b"module Bar where",
            )],
        )
            .into();

        directory_map.insert(
            Path(NonEmpty::new(unsound::label::new("haskell"))),
            haskell_files,
        );

        let directory = Directory::from::<TestRepo>(directory_map);
        let mut directory_contents = directory.list_directory();
        directory_contents.sort();

        assert_eq!(
            directory_contents,
            vec![
                SystemType::file(unsound::label::new("bar.rs")),
                SystemType::file(unsound::label::new("foo.rs")),
                SystemType::directory(unsound::label::new("haskell")),
            ]
        );

        let sub_directory = directory
            .find_directory(&Path::new(unsound::label::new("haskell")))
            .expect("Could not find sub-directory");
        let mut sub_directory_contents = sub_directory.list_directory();
        sub_directory_contents.sort();

        assert_eq!(
            sub_directory_contents,
            vec![
                SystemType::file(unsound::label::new("bar.hs")),
                SystemType::file(unsound::label::new("foo.hs")),
            ]
        );
    }

    #[test]
    fn test_all_directories_and_files() {
        let mut directory_map = HashMap::new();

        let path1 = Path::from_labels(
            unsound::label::new("foo"),
            &[bar(), unsound::label::new("baz")],
        );
        let file1 = File::new(unsound::label::new("monadic.rs"), &[]);
        let file2 = File::new(unsound::label::new("oscoin.rs"), &[]);
        directory_map.insert(path1, (file1, vec![file2]));

        let path2 = Path::from_labels(
            unsound::label::new("foo"),
            &[bar(), unsound::label::new("quux")],
        );
        let file3 = File::new(unsound::label::new("radicle.rs"), &[]);

        directory_map.insert(path2, (file3, vec![]));

        assert!(prop_all_directories_and_files(directory_map))
    }

    fn label_strategy() -> impl Strategy<Value = Label> {
        // ASCII regex, excluding '/' because of posix file paths
        "[ -.|0-~]+".prop_map(|label| unsound::label::new(&label))
    }

    fn path_strategy(max_size: usize) -> impl Strategy<Value = Path> {
        (
            label_strategy(),
            collection::vec(label_strategy(), 0..max_size),
        )
            .prop_map(|(label, labels)| Path((label, labels).into()))
    }

    fn file_strategy() -> impl Strategy<Value = File> {
        // ASCII regex, see: https://catonmat.net/my-favorite-regex
        (label_strategy(), "[ -~]*")
            .prop_map(|(name, contents)| File::new(name, contents.as_bytes()))
    }

    fn directory_map_strategy(
        path_size: usize,
        n_files: usize,
        map_size: usize,
    ) -> impl Strategy<Value = HashMap<Path, (File, Vec<File>)>> {
        collection::hash_map(
            path_strategy(path_size),
            (
                file_strategy(),
                collection::vec(file_strategy(), 0..n_files),
            ),
            0..map_size,
        )
    }

    // TODO(fintan): This is a bit slow. Could be time to benchmark some functions.
    proptest! {
        #[test]
        fn prop_test_all_directories_and_files(directory_map in directory_map_strategy(10, 10, 10)) {
            prop_all_directories_and_files(directory_map);
        }
    }

    fn prop_all_directories_and_files(directory_map: HashMap<Path, (File, Vec<File>)>) -> bool {
        let mut new_directory_map = HashMap::new();
        for (path, files) in directory_map {
            new_directory_map.insert(path.clone(), files.into());
        }

        let directory = Directory::from::<TestRepo>(new_directory_map.clone());

        for (directory_path, files) in new_directory_map {
            for file in files.iter() {
                let mut path = directory_path.clone();
                if !directory.find_directory(&path).is_some() {
                    return false;
                }

                path.push(file.name.clone());
                if !directory.find_file(&path).is_some() {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_file_name_is_same_as_root() {
        // This test ensures that if the name is the same the root of the
        // directory, that search_path.split_last() doesn't toss away the prefix.
        let path = Path::from_labels(unsound::label::new("foo"), &[bar()]);
        let files = (File::new(Label::root(), &[]), vec![]);
        let mut directory_map = HashMap::new();
        directory_map.insert(path, files);

        assert!(prop_all_directories_and_files(directory_map));
    }

    #[test]
    /// Given:
    /// foo
    /// `-- bar
    ///     `-- baz
    ///         `-- quux.rs
    ///
    /// And:
    /// foo
    /// `-- bar
    ///     `-- quux
    ///         `-- hallo.rs
    ///
    /// We expect:
    /// foo
    /// `-- bar
    ///     |-- baz
    ///     |   `-- quux.rs
    ///     `-- quux
    ///         `-- hallo.r
    fn test_combine_dirs() {
        let mut root = Directory::empty_root::<TestRepo>();
        let quux = Directory::mkdir(
            unsound::label::new("foo"),
            Directory::mkdir(
                bar(),
                Directory {
                    name: unsound::label::new("baz"),
                    entries: NonEmpty::new(DirectoryContents::file(
                        unsound::label::new("quux.rs"),
                        b"",
                    )),
                },
            ),
        );
        root.entries.push(DirectoryContents::sub_directory(quux));

        let hallo = Directory::mkdir(
            unsound::label::new("foo"),
            Directory::mkdir(
                bar(),
                Directory {
                    name: unsound::label::new("quux"),
                    entries: NonEmpty::new(DirectoryContents::file(
                        unsound::label::new("hallo.rs"),
                        b"",
                    )),
                },
            ),
        );

        let mut expected_root = Directory::empty_root::<TestRepo>();
        let expected_quux = DirectoryContents::sub_directory(Directory {
            name: unsound::label::new("baz"),
            entries: NonEmpty::new(DirectoryContents::file(unsound::label::new("quux.rs"), b"")),
        });
        let expected_hallo = DirectoryContents::sub_directory(Directory {
            name: unsound::label::new("quux"),
            entries: NonEmpty::new(DirectoryContents::file(
                unsound::label::new("hallo.rs"),
                b"",
            )),
        });

        let subdirs = (expected_quux, vec![expected_hallo]).into();

        let expected = Directory::mkdir(
            unsound::label::new("foo"),
            Directory {
                name: bar(),
                entries: subdirs,
            },
        );
        expected_root
            .entries
            .push(DirectoryContents::sub_directory(expected));

        root.combine(&hallo);

        assert_eq!(root, expected_root)
    }

    #[test]
    /// Given:
    /// foo
    /// `-- bar
    ///     `-- baz.rs
    /// And:
    /// foo
    /// `-- bar
    ///     `-- quux.rs
    ///
    /// We expect:
    /// foo
    /// `-- bar
    ///     |-- baz.rs
    ///     `-- quux.rs
    fn test_combine_files() {
        let mut root = Directory::empty_root::<TestRepo>();
        let baz = Directory::mkdir(
            unsound::label::new("foo"),
            Directory {
                name: bar(),
                entries: NonEmpty::new(DirectoryContents::file(unsound::label::new("baz.rs"), b"")),
            },
        );
        root.entries.push(DirectoryContents::sub_directory(baz));

        let quux = Directory::mkdir(
            unsound::label::new("foo"),
            Directory {
                name: bar(),
                entries: NonEmpty::new(DirectoryContents::file(
                    unsound::label::new("quux.rs"),
                    b"",
                )),
            },
        );

        let mut expected_root = Directory::empty_root::<TestRepo>();
        let files = (
            DirectoryContents::file(unsound::label::new("baz.rs"), b""),
            vec![DirectoryContents::file(unsound::label::new("quux.rs"), b"")],
        )
            .into();
        let expected = Directory::mkdir(
            unsound::label::new("foo"),
            Directory {
                name: bar(),
                entries: files,
            },
        );
        expected_root
            .entries
            .push(DirectoryContents::sub_directory(expected));

        root.combine(&quux);

        assert_eq!(root, expected_root)
    }
}
