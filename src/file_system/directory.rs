use crate::file_system::path::*;
use crate::tree::*;
use nonempty::NonEmpty;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// `SystemType` is an enumeration over what can be
/// found in a [`Directory`](struct.Directory.html)
/// so we can report back to the caller a [`Label`](struct.Label)
/// and its type.
///
/// See [`SystemType::file`](struct.SystemType.html#method.file) and
/// [`SystemType::directory`](struct.SystemType.html#method.directory).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemType {
    File,
    Directory,
}

impl SystemType {
    /// A file name and [`SystemType::File`](enum.SystemType.html#variant.File).
    pub fn file(label: Label) -> (Label, Self) {
        (label, SystemType::File)
    }

    /// A directory name and [`SystemType::Directory`](enum.SystemType.html#variant.Directory).
    pub fn directory(label: Label) -> (Label, Self) {
        (label, SystemType::Directory)
    }
}

/// A `File` consists of its file name (a [`Label`](struct.Label.html)
/// and its file contents (a `Vec` of bytes).
#[derive(Clone, PartialEq, Eq)]
pub struct File {
    pub contents: Vec<u8>,
    pub(crate) size: usize,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut contents = self.contents.clone();
        contents.truncate(10);
        write!(
            f,
            "File {{ contents: {:?}, size: {} }}",
            contents, self.size
        )
    }
}

impl File {
    pub fn new(contents: &[u8]) -> Self {
        let size = contents.len();
        File {
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

#[derive(Debug, Clone)]
enum Location {
    Root,
    SubDirectory(Label),
}

/// A `Directory` consists of its [`Label`](struct.Label.html) and its entries.
/// The entries are a set of [`DirectoryContents`](struct.DirectoryContents.html)
/// and there should be at least one entry.
/// This is because empty directories do not generally exist in VCSes.
#[derive(Debug, Clone)]
pub struct Directory {
    current: Location,
    sub_directories: Forest<Label, File>,
}

impl Directory {
    pub fn root() -> Self {
        Directory {
            current: Location::Root,
            sub_directories: Forest::root(),
        }
    }

    /// List the current `Directory`'s files and sub-directories.
    pub fn list_directory(&self) -> Vec<(Label, SystemType)> {
        let forest = &self.sub_directories;
        match &forest.0 {
            None => vec![],
            Some(trees) => trees
                .0
                .iter()
                .map(|tree| match tree {
                    SubTree::Node { key: name, .. } => SystemType::file(name.clone()),
                    SubTree::Branch { key: name, .. } => SystemType::directory(name.clone()),
                })
                .collect(),
        }
    }

    /// Find a `File` in the directory given the `Path` to
    /// the `File`.
    ///
    /// This operation fails if the path does not lead to
    /// the `File`.
    pub fn find_file(&self, path: &Path) -> Option<File> {
        self.sub_directories.find_node(&path.0).cloned()
    }

    /// Find a `Directory` in the directory given the `Path` to
    /// the `Directory`.
    ///
    /// This operation fails if the path does not lead to
    /// the `Directory`.
    pub fn find_directory(&self, path: &Path) -> Option<Self> {
        self.sub_directories
            .find_branch(&path.0)
            .cloned()
            .map(|tree| {
                let (_, current) = path.split_last();
                Directory {
                    current: Location::SubDirectory(current),
                    sub_directories: tree.into(),
                }
            })
    }

    /// Get the `Label` of the current directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    /// root.insert_file(&unsound::path::new("main.rs"), File::new(b"println!(\"Hello, world!\")"));
    /// root.insert_file(&unsound::path::new("lib.rs"), File::new(b"struct Hello(String)"));
    /// root.insert_file(&unsound::path::new("test/mod.rs"), File::new(b"assert_eq!(1 + 1, 2);"));
    ///
    /// assert_eq!(root.current(), Label::root());
    ///
    /// let test = root.find_directory(
    ///     &unsound::path::new("test")
    /// ).expect("Missing test directory");
    /// assert_eq!(test.current(), unsound::label::new("test"));
    /// ```
    pub fn current(&self) -> Label {
        match &self.current {
            Location::Root => Label::root(),
            Location::SubDirectory(label) => label.clone(),
        }
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for now
    #[allow(dead_code)]
    fn fuzzy_find(_label: Label) -> Vec<Self> {
        unimplemented!()
    }

    /// Get the total size, in bytes, of a `Directory`. The size is
    /// the sum of all files that can be reached from this `Directory`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Directory, File};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    /// root.insert_files(
    ///     &[],
    ///     NonEmpty::from((
    ///         (
    ///             unsound::label::new("main.rs"),
    ///             File::new(b"println!(\"Hello, world!\")"),
    ///         ),
    ///         vec![
    ///             (
    ///                 unsound::label::new("lib.rs"),
    ///                 File::new(b"struct Hello(String)"),
    ///             ),
    ///         ],
    ///     )),
    /// );
    ///
    /// assert_eq!(root.size(), 45);
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::{Directory, File};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Directory::root();
    /// root.insert_file(&unsound::path::new("main.rs"), File::new(b"println!(\"Hello, world!\")"));
    /// root.insert_file(&unsound::path::new("lib.rs"), File::new(b"struct Hello(String)"));
    /// root.insert_file(&unsound::path::new("test/mod.rs"), File::new(b"assert_eq!(1 + 1, 2);"));
    ///
    /// assert_eq!(root.size(), 66);
    /// ```
    pub fn size(&self) -> usize {
        self.sub_directories
            .iter()
            .fold(0, |size, file| size + file.size())
    }

    pub fn insert_file(&mut self, path: &Path, file: File) {
        self.sub_directories.insert(&path.0, file)
    }

    pub fn insert_files(&mut self, directory_path: &[Label], files: NonEmpty<(Label, File)>) {
        match NonEmpty::from_slice(directory_path) {
            None => {
                for (file_name, file) in files.iter() {
                    self.insert_file(&Path::new(file_name.clone()), file.clone())
                }
            }
            Some(directory_path) => {
                for (file_name, file) in files.iter() {
                    let mut file_path = Path(directory_path.clone());
                    file_path.push(file_name.clone());

                    self.insert_file(&file_path, file.clone())
                }
            }
        }
    }

    pub(crate) fn from_hash_map(files: HashMap<Path, NonEmpty<(Label, File)>>) -> Self {
        let mut directory: Self = Directory::root();

        for (path, files) in files.into_iter() {
            for (file_name, file) in files.iter() {
                let mut file_path = path.clone();
                file_path.push(file_name.clone());
                if path.is_root() {
                    directory.insert_file(&Path::new(file_name.clone()), file.clone())
                } else {
                    directory.insert_file(&file_path, file.clone())
                }
            }
        }

        directory
    }
}

#[cfg(test)]
pub mod tests {
    use crate::file_system::unsound;
    use crate::file_system::*;
    use nonempty::NonEmpty;
    use pretty_assertions::assert_eq;
    use proptest::collection;
    use proptest::prelude::*;
    use std::collections::HashMap;

    #[test]
    fn test_find_added_file() {
        let file = File::new(b"module Banana ...");

        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("foo.hs"), file.clone());

        // Search for "~/foo.hs"
        assert_eq!(
            directory.find_file(&unsound::path::new("foo.hs")),
            Some(file)
        )
    }

    #[test]
    fn test_find_added_file_long_path() {
        let file_path = unsound::path::new("foo/bar/baz.rs");

        let file = File::new(b"module Banana ...");

        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("foo/bar/baz.rs"), file.clone());

        // Search for "~/foo/bar/baz.hs"
        assert_eq!(directory.find_file(&file_path), Some(file))
    }

    #[test]
    fn test_404_file_not_found() {
        let file_path = unsound::path::new("bar.hs");

        let file = File::new(b"module Banana ...");

        let mut directory = Directory::root();
        directory.insert_file(&unsound::path::new("foo.hs"), file);

        // Search for "~/bar.hs"
        assert_eq!(directory.find_file(&file_path), None)
    }

    #[test]
    fn test_list_directory() {
        let mut directory = Directory::root();
        directory.insert_file(
            &unsound::path::new("foo.hs"),
            File::new(b"module BananaFoo ..."),
        );
        directory.insert_file(
            &unsound::path::new("bar.hs"),
            File::new(b"module BananaBar ..."),
        );
        directory.insert_file(
            &unsound::path::new("baz.hs"),
            File::new(b"module BananaBaz ..."),
        );

        assert_eq!(
            directory.list_directory(),
            vec![
                SystemType::file(unsound::label::new("bar.hs")),
                SystemType::file(unsound::label::new("baz.hs")),
                SystemType::file(unsound::label::new("foo.hs")),
            ]
        );
    }

    #[test]
    fn test_create_and_list() {
        let mut directory = Directory::root();

        // Root files set up
        let root_files = NonEmpty::from((
            (unsound::label::new("foo.rs"), File::new(b"use crate::bar")),
            vec![(
                unsound::label::new("bar.rs"),
                File::new(b"fn hello_world()"),
            )],
        ));
        directory.insert_files(&[], root_files);

        // Haskell files set up
        let haskell_files = NonEmpty::from((
            (
                unsound::label::new("foo.hs"),
                File::new(b"module Foo where"),
            ),
            vec![(
                unsound::label::new("bar.hs"),
                File::new(b"module Bar where"),
            )],
        ));

        directory.insert_files(&[unsound::label::new("haskell")], haskell_files);

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
            .find_directory(&unsound::path::new("haskell"))
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

        let path1 = unsound::path::new("foo/bar/baz");
        let file1 = (unsound::label::new("monadic.rs"), File::new(&[]));
        let file2 = (unsound::label::new("oscoin.rs"), File::new(&[]));
        directory_map.insert(path1, NonEmpty::from((file1, vec![file2])));

        let path2 = unsound::path::new("foor/bar/quuz");
        let file3 = (unsound::label::new("radicle.rs"), File::new(&[]));

        directory_map.insert(path2, NonEmpty::new(file3));

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

    fn file_strategy() -> impl Strategy<Value = (Label, File)> {
        // ASCII regex, see: https://catonmat.net/my-favorite-regex
        (label_strategy(), "[ -~]*")
            .prop_map(|(name, contents)| (name, File::new(contents.as_bytes())))
    }

    fn directory_map_strategy(
        path_size: usize,
        n_files: usize,
        map_size: usize,
    ) -> impl Strategy<Value = HashMap<Path, NonEmpty<(Label, File)>>> {
        collection::hash_map(
            path_strategy(path_size),
            collection::vec(file_strategy(), 1..n_files).prop_map(|files| {
                NonEmpty::from_slice(&files).expect("Strategy generated files of length 0")
            }),
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

    fn prop_all_directories_and_files(
        directory_map: HashMap<Path, NonEmpty<(Label, File)>>,
    ) -> bool {
        let mut new_directory_map = HashMap::new();
        for (path, files) in directory_map {
            new_directory_map.insert(path.clone(), files.into());
        }

        let directory = Directory::from_hash_map(new_directory_map.clone());

        for (directory_path, files) in new_directory_map {
            for (file_name, _) in files.iter() {
                let mut path = directory_path.clone();
                if directory.find_directory(&path).is_none() {
                    return false;
                }

                path.push(file_name.clone());
                if directory.find_file(&path).is_none() {
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
        let path = unsound::path::new("foo/bar/~");
        let mut directory_map = HashMap::new();
        directory_map.insert(path, NonEmpty::new((Label::root(), File::new(b"root"))));

        assert!(prop_all_directories_and_files(directory_map));
    }
}
