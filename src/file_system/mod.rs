use nonempty::NonEmpty;
use std::collections::HashMap;

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

/// A label for `Directory` and `File` to
/// allow for search.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub String);

impl Label {
    /// The root label for the root directory, i.e. `"~"`.
    pub fn root() -> Self {
        "~".into()
    }
}

#[cfg(test)]
impl Arbitrary for Label {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Label(Arbitrary::arbitrary(g))
    }
}

impl From<&str> for Label {
    fn from(item: &str) -> Self {
        Label(item.into())
    }
}

impl From<String> for Label {
    fn from(item: String) -> Self {
        Label(item)
    }
}

/// A non-empty set of labels to define a path
/// in a directory search.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub NonEmpty<Label>);

#[cfg(test)]
impl Arbitrary for Path {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let head = Arbitrary::arbitrary(g);
        let tail: Vec<Label> = Arbitrary::arbitrary(g);
        Path::from_labels(head, &tail)
    }
}

impl Path {
    /// The root path is the singleton containing the
    /// root label (see: `Label::root`).
    pub fn root() -> Self {
        Path(NonEmpty::new(Label::root()))
    }

    /// Check that this is the root path.
    pub fn is_root(&self) -> bool {
        *self == Self::root()
    }

    /// Append two `Path`s together.
    ///
    /// # Example
    /// ```
    /// use radicle_surf::file_system::Path;
    ///
    /// let mut path1 = Path::from_labels("foo".into(), &["bar".into()]);
    /// path1.append(&mut Path::from_labels("baz".into(), &["quux".into()]));
    /// assert_eq!(path1, Path::from_labels("foo".into(), &["bar".into(), "baz".into(), "quux".into()]));
    /// ```
    pub fn append(&mut self, path: &mut Self) {
        let mut other = path.0.clone().into();
        self.0.append(&mut other)
    }

    /// Push a new `Label` onto the `Path`.
    pub fn push(&mut self, label: Label) {
        self.0.push(label)
    }

    /// Iterator over the `Label`s.
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.0.iter()
    }

    /// Get the first `Label` and the rest of the `Label`s.
    pub fn split_first(&self) -> (&Label, &[Label]) {
        self.0.split_first()
    }

    /// Get the prefix of the `Label`s and the last `Label`.
    /// This is useful since the prefix could be a directory path
    /// and the last label a file name.
    ///
    /// # Example
    ///
    /// ```
    /// use radicle_surf::file_system::Path;
    ///
    /// let path = Path::from_labels("foo".into(), &[]);
    /// assert_eq!(path.split_last(), (vec![], "foo".into()));
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::Path;
    ///
    /// let path = Path::from_labels("foo".into(), &["bar".into()]);
    /// assert_eq!(path.split_last(), (vec!["foo".into()], "bar".into()));
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::Path;
    ///
    /// let path = Path::from_labels("foo".into(), &["bar".into(), "baz".into()]);
    /// assert_eq!(path.split_last(), (vec!["foo".into(), "bar".into()], "baz".into()));
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::Path;
    ///
    /// let path = Path::from_labels("foo".into(), &["bar".into(), "foo".into()]);
    /// assert_eq!(path.split_last(), (vec!["foo".into(), "bar".into()], "foo".into()));
    /// ```
    pub fn split_last(&self) -> (Vec<Label>, Label) {
        let (first, middle, last) = self.0.split();

        // first == last, so drop first
        if first == last && middle.is_empty() {
            (vec![], last.clone())
        } else {
            // Create the prefix vector
            let mut vec = vec![first.clone()];
            let mut middle = middle.to_vec();
            vec.append(&mut middle);
            (vec, last.clone())
        }
    }

    /// Constructor given at least one `Label` to work from followed
    /// by 0 or more `Label`s to add to the `Path`.
    ///
    /// # Example
    ///
    /// ```
    /// use radicle_surf::file_system::{Path, Label};
    /// use nonempty::NonEmpty;
    ///
    /// let path = Path::from_labels(Label::root(), &["foo".into(), "bar".into(), "baz.rs".into()]);
    ///
    /// let mut expected = Path::root();
    /// expected.push("foo".into());
    /// expected.push("bar".into());
    /// expected.push("baz.rs".into());
    ///
    /// assert_eq!(path, expected);
    /// let path_vec: Vec<Label> = path.0.into();
    /// assert_eq!(path_vec, vec!["~".into(), "foo".into(), "bar".into(), "baz.rs".into()]);
    /// ```
    pub fn from_labels(root: Label, labels: &[Label]) -> Path {
        let mut path = Path(NonEmpty::new(root));
        labels.iter().cloned().for_each(|l| path.push(l));
        path
    }

    /// Convert a raw string literal to a `Path`.
    ///
    /// # Example
    ///
    /// ```
    /// use radicle_surf::file_system::{Path};
    ///
    /// let path = Path::from_string("foo/bar/baz.rs");
    ///
    /// let expected = Path::from_labels("foo".into(), &["bar".into(), "baz.rs".into()]);
    ///
    /// assert_eq!(path, expected);
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::{Path};
    ///
    /// let path = Path::from_string("foo/bar/baz/");
    ///
    /// let expected = Path::from_labels("foo".into(), &["bar".into(), "baz".into()]);
    ///
    /// assert_eq!(path, expected);
    /// ```
    pub fn from_string(path: &str) -> Self {
        if path.is_empty() {
            Path::root()
        } else {
            NonEmpty::from_slice(
                &path
                    .trim_matches('/')
                    .split('/')
                    .map(|l| l.into())
                    .collect::<Vec<_>>(),
            )
            .map_or(Path::root(), Path)
        }
    }
}

/// A trait to say how to intitialise a
/// Repository `Directory`. For example, Git
/// would initialise with the `.git` folder and
/// the files contained in it.
pub trait RepoBackend
where
    Self: Sized,
{
    fn repo_directory() -> Directory;
}

/// A `DirectoryContents` is made up of either:
/// * A `SubDirectory`
/// * A `File`
/// * A `Repo`, which is expected to be the
///   special Repository directory, but is opaque
///   to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryContents {
    SubDirectory(Box<Directory>),
    File(File),
    Repo,
}

impl DirectoryContents {
    /// Helper constructor for a `SubDirectory`.
    pub fn sub_directory(directory: Directory) -> Self {
        DirectoryContents::SubDirectory(Box::new(directory))
    }

    /// Helper constructor for a `File`.
    pub fn file(filename: Label, contents: &[u8]) -> Self {
        DirectoryContents::File(File {
            filename,
            contents: contents.to_owned(),
        })
    }

    /// Helper constructor for a `Repo`.
    pub fn repo() -> Self {
        DirectoryContents::Repo
    }
}

/// A `Directory` consists of its `Label` and its entries.
/// The entries are a set of `DirectoryContents` and there
/// should be at least on entry. This is because empty
/// directories doe not exist in VCSes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents>,
}

/// A `File` consists of its file name (a `Label`) and
/// its file contents.
#[derive(Clone, PartialEq, Eq)]
pub struct File {
    pub filename: Label,
    pub contents: Vec<u8>,
}

#[cfg(test)]
impl Arbitrary for File {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let filename = Arbitrary::arbitrary(g);
        let contents = Arbitrary::arbitrary(g);
        File { filename, contents }
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File {{ filename: {:#?} }}", self.filename)
    }
}

/// `SystemType` is an enumeration over what can be
/// found in a `Directory` so we can report back to
/// the caller a `Label` and its type.
///
/// See `SystemType::file` and `SystemType::directory`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemType {
    File,
    Directory,
}

impl SystemType {
    /// A file name and `SystemType::File`.
    pub fn file(label: Label) -> (Label, Self) {
        (label, SystemType::File)
    }

    /// A directory name and `SystemType::File`.
    pub fn directory(label: Label) -> (Label, Self) {
        (label, SystemType::Directory)
    }
}

impl Directory {
    /// An empty root `Directory`, just containing
    /// the special repository directory.
    pub fn empty_root<Repo>() -> Self
    where
        Repo: RepoBackend,
    {
        Directory::mkdir(Label::root(), Repo::repo_directory())
    }

    /// List the current `Directory`'s files and sub-directories.
    pub fn list_directory(&self) -> Vec<(Label, SystemType)> {
        self.entries
            .iter()
            .cloned()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(SystemType::directory(dir.label)),
                DirectoryContents::File(file) => Some(SystemType::file(file.filename)),
                DirectoryContents::Repo => None,
            })
            .collect()
    }

    pub fn add_contents(&mut self, entries: NonEmpty<DirectoryContents>) {
        self.entries.append(&mut entries.into())
    }

    /// Find a `File` in the directory given the `Path` to
    /// the `File`.
    ///
    /// This operation fails if the path does not lead to
    /// the `File`.
    pub fn find_file(&self, path: Path) -> Option<File> {
        let (path, filename) = path.split_last();
        let path = NonEmpty::from_slice(&path);

        // Find the file in the current directoy if the prefix path is empty.
        // Otherwise find it in the directory found in the given path (if it exists).
        path.map_or(Some(self.clone()), |p| self.find_directory(Path(p)))
            .and_then(|dir| dir.file_in_directory(&filename))
    }

    /// Find a `Directory` in the directory given the `Path` to
    /// the `Directory`.
    ///
    /// This operation fails if the path does not lead to
    /// the `Directory`.
    pub fn find_directory(&self, path: Path) -> Option<Self> {
        let (label, labels) = path.split_first();
        if *label == self.label {
            // recursively dig down into sub-directories
            labels
                .iter()
                .try_fold(self.clone(), |dir, label| dir.get_sub_directory(&label))
        } else {
            None
        }
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for now
    #[allow(dead_code)]
    fn fuzzy_find(_label: Label) -> Vec<Self> {
        unimplemented!()
    }

    /// Get the sub directories of a `Directory`.
    fn get_sub_directories(&self) -> Vec<Self> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(*dir.clone()),
                DirectoryContents::File(_) => None,
                DirectoryContents::Repo => None,
            })
            .collect()
    }

    /// Get the a sub directory of a `Directory` given its name.
    ///
    /// This operation fails if the directory does not exist.
    fn get_sub_directory(&self, label: &Label) -> Option<Self> {
        self.get_sub_directories()
            .iter()
            .cloned()
            .find(|directory| directory.label == *label)
    }

    /// Get the `File` in the current `Directory` if it exists in
    /// the entries.
    ///
    /// The operation fails if the `File` does not exist in the `Directory`.
    fn file_in_directory(&self, label: &Label) -> Option<File> {
        for entry in self.entries.iter() {
            match entry {
                DirectoryContents::File(file) if file.filename == *label => {
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
    pub(crate) fn mkdir(label: Label, dir: Self) -> Self {
        Directory {
            label,
            entries: NonEmpty::new(DirectoryContents::sub_directory(dir)),
        }
    }

    pub fn from<Repo>(paths: HashMap<Path, NonEmpty<File>>) -> Self
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
                    label: current,
                    entries: file_entries,
                };
                for label in prefix {
                    directory = Directory::mkdir(label, directory);
                }
                root.entries
                    .push(DirectoryContents::SubDirectory(Box::new(directory)))
            }
        }
        root
    }
}

#[cfg(test)]
pub mod tests {
    use crate::file_system::*;

    #[derive(Debug, Clone)]
    struct TestRepo {}

    impl RepoBackend for TestRepo {
        fn repo_directory() -> Directory {
            Directory {
                label: ".test".into(),
                entries: NonEmpty::new(DirectoryContents::Repo),
            }
        }
    }

    #[test]
    fn test_find_added_file() {
        let file_path = Path::from_labels(Label::root(), &["foo.hs".into()]);

        let file = File {
            filename: "foo.hs".into(),
            contents: "module Banana ...".into(),
        };

        let directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::File(file.clone())),
        };

        // Search for "~/foo.hs"
        assert_eq!(directory.find_file(file_path), Some(file))
    }

    #[test]
    fn test_find_added_file_long_path() {
        let file_path = Path::from_labels(
            Label::root(),
            &["foo".into(), "bar".into(), "baz.hs".into()],
        );

        let file = File {
            filename: "baz.hs".into(),
            contents: "module Banana ...".into(),
        };

        let directory: Directory = Directory::mkdir(
            Label::root(),
            Directory::mkdir(
                "foo".into(),
                Directory {
                    label: "bar".into(),
                    entries: NonEmpty::new(DirectoryContents::File(file.clone())),
                },
            ),
        );

        // Search for "~/foo/bar/baz.hs"
        assert_eq!(directory.find_file(file_path), Some(file))
    }

    #[test]
    fn test_404_file_not_found() {
        let file_path = Path::from_labels(Label::root(), &["bar.hs".into()]);

        let directory: Directory = Directory {
            label: Label::root(),
            entries: NonEmpty::new(DirectoryContents::file(
                "foo.hs".into(),
                "module Banana ...".as_bytes(),
            )),
        };

        // Search for "~/bar.hs"
        assert_eq!(directory.find_file(file_path), None)
    }

    #[test]
    fn test_list_directory() {
        let foo = DirectoryContents::file("foo.hs".into(), "module Banana ...".as_bytes());
        let bar = DirectoryContents::file("bar.hs".into(), "module Banana ...".as_bytes());
        let baz = DirectoryContents::file("baz.hs".into(), "module Banana ...".as_bytes());

        let mut files = NonEmpty::new(foo);
        files.push(bar);
        files.push(baz);

        let directory: Directory = Directory {
            label: Label::root(),
            entries: files,
        };

        assert_eq!(
            directory.list_directory(),
            vec![
                SystemType::file("foo.hs".into()),
                SystemType::file("bar.hs".into()),
                SystemType::file("baz.hs".into()),
            ]
        );
    }

    #[test]
    fn test_create_and_list() {
        let mut directory_map = HashMap::new();

        // Root files set up
        let mut root_files = NonEmpty::new(File {
            filename: "foo.rs".into(),
            contents: "use crate::bar".as_bytes().to_vec(),
        });
        root_files.push(File {
            filename: "bar.rs".into(),
            contents: "fn hello_world()".as_bytes().to_vec(),
        });
        directory_map.insert(Path::root(), root_files);

        // Haskell files set up
        let mut haskell_files = NonEmpty::new(File {
            filename: "foo.hs".into(),
            contents: "module Foo where".as_bytes().to_vec(),
        });
        haskell_files.push(File {
            filename: "bar.hs".into(),
            contents: "module Bar where".as_bytes().to_vec(),
        });
        directory_map.insert(Path(NonEmpty::new("haskell".into())), haskell_files);

        let directory = Directory::from::<TestRepo>(directory_map);
        let mut directory_contents = directory.list_directory();
        directory_contents.sort();

        assert_eq!(
            directory_contents,
            vec![
                SystemType::directory(".test".into()),
                SystemType::file("bar.rs".into()),
                SystemType::file("foo.rs".into()),
                SystemType::directory("haskell".into()),
            ]
        );

        let sub_directory = directory
            .find_directory(Path::from_labels("~".into(), &["haskell".into()]))
            .unwrap();
        let mut sub_directory_contents = sub_directory.list_directory();
        sub_directory_contents.sort();

        assert_eq!(
            sub_directory_contents,
            vec![
                SystemType::file("bar.hs".into()),
                SystemType::file("foo.hs".into()),
            ]
        );
    }

    /* TODO(fintan): this quickcheck takes far too long to complete
    #[quickcheck]
    fn prop_all_directories_and_files_quickcheck(
        directory_map: SmallHashMap<Path, (File, Vec<File>)>,
    ) -> bool {
        prop_all_directories_and_files(directory_map.get_small_hashmap)
    }
    */
}
