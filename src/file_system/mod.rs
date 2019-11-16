use nonempty::NonEmpty;

/// A label for `Directory` and `File` to
/// allow for search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub String);

impl Label {
    /// The root label for the root directory, i.e. `"~"`.
    pub fn root_label() -> Self {
        "~".into()
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path(pub NonEmpty<Label>);

impl Path {
    /// The root path is the singleton containing the
    /// root label (see: `root_label`).
    pub fn root_path() -> Self {
        Path(NonEmpty::new(Label::root_label()))
    }

    /// Check that this is the root path.
    pub fn is_root(&self) -> bool {
        *self == Self::root_path()
    }

    /// Append two `Path`s together.
    pub fn append(&mut self, path: &mut Self) {
        let mut other = path.0.clone().into();
        path.0.append(&mut other)
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
    pub fn split_last(&self) -> (Vec<Label>, Label) {
        let (first, middle, last) = self.0.split();

        // first == last, so drop first
        if middle.is_empty() {
            (vec![], last.clone())
        } else {
            // Create the prefix vector
            let mut vec = middle.to_vec();
            vec.insert(0, first.clone());

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
    ///
    /// let path = Path::from_labels(Label::root_label(), &["foo".into(), "bar".into(), "baz.rs".into()]);
    /// println!("{:#?}", path);
    /// ```
    pub fn from_labels(root: Label, labels: &[Label]) -> Path {
        let mut path = Path(NonEmpty::new(root));
        labels.iter().cloned().for_each(|l| path.push(l));
        path
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
    fn new() -> Directory<Self>;
}

/// A `DirectoryContents` is made up of either:
/// * A `SubDirectory`
/// * A `File`
/// * A `Repo`, which is expected to be the
///   special Repository directory, but is opaque
///   to the user.
#[derive(Debug, Clone)]
pub enum DirectoryContents<Repo> {
    SubDirectory(Box<Directory<Repo>>),
    File(File),
    Repo(Repo),
}

impl<Repo> DirectoryContents<Repo> {
    /// Helper constructor for a `SubDirectory`.
    pub fn sub_directory(directory: Directory<Repo>) -> Self {
        DirectoryContents::SubDirectory(Box::new(directory))
    }

    /// Helper constructor for a `File`.
    pub fn file(filename: Label, contents: String) -> Self {
        DirectoryContents::File(File { filename, contents })
    }

    /// Helper constructor for a `Repo`.
    pub fn repo(repo: Repo) -> Self {
        DirectoryContents::Repo(repo)
    }
}

/// A `Directory` consists of its `Label` and its entries.
/// The entries are a set of `DirectoryContents` and there
/// should be at least on entry. This is because empty
/// directories doe not exist in VCSes.
#[derive(Debug, Clone)]
pub struct Directory<Repo> {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents<Repo>>,
}

/// A `File` consists of its file name (a `Label`) and
/// its file contents.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub filename: Label,
    pub contents: String,
}

/// `SystemType` is an enumeration over what can be
/// found in a `Directory` so we can report back to
/// the caller a `Label` and its type.
///
/// See `SystemType::file` and `SystemType::directory`.
#[derive(Debug, Clone, PartialEq)]
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

impl<Repo> Directory<Repo> {
    /// An empty root `Directory`, just containing
    /// the special repository directory.
    pub fn empty_root() -> Self
    where
        Repo: RepoBackend,
    {
        Repo::new()
    }

    /// List the current `Directory`'s files and sub-directories.
    pub fn list_directory(&self) -> Vec<(Label, SystemType)>
    where
        Repo: Clone,
    {
        self.entries
            .iter()
            .cloned()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(SystemType::directory(dir.label)),
                DirectoryContents::File(file) => Some(SystemType::file(file.filename)),
                DirectoryContents::Repo(_) => None,
            })
            .collect()
    }

    /// Find a `File` in the directory given the `Path` to
    /// the `File`.
    ///
    /// This operation fails if the path does not lead to
    /// the `File`.
    pub fn find_file(&self, path: Path) -> Option<File>
    where
        Repo: Clone + std::fmt::Debug,
    {
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
    pub fn find_directory(&self, path: Path) -> Option<Self>
    where
        Repo: Clone,
    {
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
    fn get_sub_directories(&self) -> Vec<Self>
    where
        Repo: Clone,
    {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some(*dir.clone()),
                DirectoryContents::File(_) => None,
                DirectoryContents::Repo(_) => None,
            })
            .collect()
    }

    /// Get the a sub directory of a `Directory` given its name.
    ///
    /// This operation fails if the directory does not exist.
    fn get_sub_directory(&self, label: &Label) -> Option<Self>
    where
        Repo: Clone,
    {
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
                DirectoryContents::Repo(_) => {}
            }
        }
        None
    }

    /// Helper function for creating a `Directory` with a given sub-directory.
    #[allow(dead_code)]
    pub(crate) fn mkdir(label: Label, dir: Self) -> Self {
        Directory {
            label,
            entries: NonEmpty::new(DirectoryContents::sub_directory(dir)),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::file_system::*;

    #[derive(Debug, Clone)]
    struct TestRepo {}

    impl RepoBackend for TestRepo {
        fn new() -> Directory<TestRepo> {
            Directory {
                label: Label::root_label(),
                entries: NonEmpty::new(DirectoryContents::Repo(TestRepo {})),
            }
        }
    }

    #[test]
    fn find_added_file() {
        let file_path = Path::from_labels(Label::root_label(), &["foo.hs".into()]);

        let file = File {
            filename: "foo.hs".into(),
            contents: "module Banana ...".into(),
        };

        let directory: Directory<TestRepo> = Directory {
            label: Label::root_label(),
            entries: NonEmpty::new(DirectoryContents::File(file.clone())),
        };

        // Search for "~/foo.hs"
        assert_eq!(directory.find_file(file_path), Some(file))
    }

    #[test]
    fn find_added_file_long_path() {
        let file_path = Path::from_labels(
            Label::root_label(),
            &["foo".into(), "bar".into(), "baz.hs".into()],
        );

        let file = File {
            filename: "baz.hs".into(),
            contents: "module Banana ...".into(),
        };

        let directory: Directory<TestRepo> = Directory::mkdir(
            Label::root_label(),
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
    fn _404_file_not_found() {
        let file_path = Path::from_labels(Label::root_label(), &["bar.hs".into()]);

        let directory: Directory<TestRepo> = Directory {
            label: Label::root_label(),
            entries: NonEmpty::new(DirectoryContents::file(
                "foo.hs".into(),
                "module Banana ...".into(),
            )),
        };

        // Search for "~/bar.hs"
        assert_eq!(directory.find_file(file_path), None)
    }

    #[test]
    fn list_directory() {
        let foo = DirectoryContents::file("foo.hs".into(), "module Banana ...".into());
        let bar = DirectoryContents::file("bar.hs".into(), "module Banana ...".into());
        let baz = DirectoryContents::file("baz.hs".into(), "module Banana ...".into());

        let mut files = NonEmpty::new(foo);
        files.push(bar);
        files.push(baz);

        let directory: Directory<TestRepo> = Directory {
            label: Label::root_label(),
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
}
