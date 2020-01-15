use crate::nonempty::split_last;
use nonempty::NonEmpty;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path;
use std::str::FromStr;

pub mod error;
pub use crate::file_system::error as file_error;

pub mod unsound;

/// A label for [`Directory`](struct.Directory.html)
/// and [`File`](struct.File.html) to allow for search.
///
/// These are essentially directory and file names.
///
/// # Examples
///
/// ```
/// use radicle_surf::file_system::error as file_error;
/// use radicle_surf::file_system::{Label, Path};
/// use std::convert::TryFrom;
///
/// fn build_lib_path() -> Result<Path, file_error::Error> {
///     let lib_filename = Label::try_from("lib.rs")?;
///     let src_directory_name = Label::try_from("src")?;
///     Ok(Path::from_labels(src_directory_name, &[lib_filename]))
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
    pub label: String,
    pub(crate) hidden: bool,
}

impl Label {
    /// The root label for the root directory, i.e. `"~"`.
    ///
    /// Prefer creating a root [`Path`](struct.Path.html),
    /// by using [`Path::root`](struct.Path.html#method.root).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    ///
    /// let root = Path::root();
    /// assert_eq!(*root.split_first().0, Label::root());
    /// ```
    pub fn root() -> Self {
        Label {
            label: "~".into(),
            hidden: false,
        }
    }

    pub fn is_root(&self) -> bool {
        *self == Self::root()
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl TryFrom<&str> for Label {
    type Error = file_error::Error;

    fn try_from(item: &str) -> Result<Self, Self::Error> {
        if item.is_empty() {
            Err(file_error::EMPTY_LABEL)
        } else if item.contains('/') {
            Err(file_error::CONTAINS_SLASH)
        } else {
            Ok(Label {
                label: item.into(),
                hidden: false,
            })
        }
    }
}

impl FromStr for Label {
    type Err = file_error::Error;

    fn from_str(item: &str) -> Result<Self, Self::Err> {
        Label::try_from(item)
    }
}

/// A non-empty set of [`Label`](struct.Label.html)s to define a path
/// in a directory or file search.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub NonEmpty<Label>);

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (prefix, suffix) = self.split_last();
        for p in prefix {
            write!(f, "{}/", p)?;
        }
        write!(f, "{}", suffix)
    }
}

impl TryFrom<&str> for Path {
    type Error = file_error::Error;

    fn try_from(item: &str) -> Result<Self, Self::Error> {
        let mut path = Vec::new();

        for label in item.trim_end_matches('/').split('/') {
            let l = Label::try_from(label)?;
            path.push(l);
        }

        NonEmpty::from_slice(&path)
            .ok_or(file_error::EMPTY_PATH)
            .map(Path)
    }
}

impl FromStr for Path {
    type Err = file_error::Error;

    fn from_str(item: &str) -> Result<Self, Self::Err> {
        Path::try_from(item)
    }
}

impl From<Path> for Vec<Label> {
    fn from(path: Path) -> Self {
        path.0.into()
    }
}

impl Path {
    pub fn new(label: Label) -> Path {
        Path(NonEmpty::new(label))
    }

    /// The root path is a `Path` made up of the single
    /// root label (see: [`Label::root`](stuct.Label.html#method.root).
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    ///
    /// let root = Path::root();
    /// assert_eq!(*root.split_first().0, Label::root());
    /// ```
    pub fn root() -> Self {
        Path(NonEmpty::new(Label::root()))
    }

    /// Check that this is the root path.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    /// use std::convert::TryFrom;
    ///
    /// let root = Path::root();
    /// let not_root = unsound::path::new("src/lib.rs");
    ///
    /// assert!(root.is_root());
    /// assert!(!not_root.is_root());
    /// ```
    pub fn is_root(&self) -> bool {
        *self == Self::root()
    }

    /// Append two `Path`s together.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::Path;
    /// use radicle_surf::file_system::unsound;
    /// use std::convert::TryFrom;
    ///
    /// let mut path1 = unsound::path::new("foo/bar");
    /// let mut path2 = unsound::path::new("baz/quux");
    /// path1.append(&mut path2);
    /// let expected = unsound::path::new("foo/bar/baz/quux");
    /// assert_eq!(path1, expected);
    /// ```
    pub fn append(&mut self, path: &mut Self) {
        let mut other = path.0.clone().into();
        self.0.append(&mut other)
    }

    /// Push a new [`Label`](struct.Label.html) onto the `Path`.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let mut root = Path::root();
    /// root.push(unsound::label::new("src"));
    /// root.push(unsound::label::new("lib.rs"));
    ///
    /// assert_eq!(root, Path::with_root(&[unsound::label::new("src"), unsound::label::new("lib.rs")]));
    /// ```
    pub fn push(&mut self, label: Label) {
        self.0.push(label)
    }

    pub fn pop(&mut self) -> Option<Label> {
        self.0.pop()
    }

    /// Iterator over the [`Label`](struct.Label.html)s in the `Path`.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("~/src/lib.rs");
    /// let mut path_iter = path.iter();
    ///
    /// assert_eq!(path_iter.next(), Some(&Label::root()));
    /// assert_eq!(path_iter.next(), Some(&unsound::label::new("src")));
    /// assert_eq!(path_iter.next(), Some(&unsound::label::new("lib.rs")));
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.0.iter()
    }

    /// Get the first [`Label`](struct.Label.html) in the `Path`
    /// and the rest of the [`Label`](struct.Label.html)s after it.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("~/src/lib.rs");
    ///
    /// assert_eq!(
    ///     path.split_first(),
    ///     (&Label::root(), &[unsound::label::new("src"), unsound::label::new("lib.rs")][..])
    /// );
    /// ```
    pub fn split_first(&self) -> (&Label, &[Label]) {
        self.0.split_first()
    }

    /// Get the prefix of the [`Label`](struct.Label.html)s and
    /// the last [`Label`](struct.Label.html).
    ///
    /// This is useful when the prefix is a directory path
    /// and the last label a file name.
    ///
    /// # Examples
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("foo");
    /// assert_eq!(path.split_last(), (vec![], unsound::label::new("foo")));
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("~/src/lib.rs");
    /// assert_eq!(path.split_last(), (vec![Label::root(), unsound::label::new("src")], unsound::label::new("lib.rs")));
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("foo/bar/baz");
    /// assert_eq!(
    ///     path.split_last(),
    ///     (vec![unsound::label::new("foo"), unsound::label::new("bar")], unsound::label::new("baz"))
    /// );
    /// ```
    ///
    /// ```
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// // An interesting case for when first == last, but doesn't imply a singleton Path.
    /// let path = unsound::path::new("foo/bar/foo");
    /// assert_eq!(
    ///     path.split_last(),
    ///     (vec![unsound::label::new("foo"), unsound::label::new("bar")], unsound::label::new("foo"))
    /// );
    /// ```
    pub fn split_last(&self) -> (Vec<Label>, Label) {
        split_last(&self.0)
    }

    /// Construct a `Path` given at least one [`Label`](struct.Label)
    /// followed by 0 or more [`Label`](struct.Label)s.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Path, Label};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("~/foo/bar/baz.rs");
    ///
    /// let mut expected = Path::root();
    /// expected.push(unsound::label::new("foo"));
    /// expected.push(unsound::label::new("bar"));
    /// expected.push(unsound::label::new("baz.rs"));
    ///
    /// assert_eq!(path, expected);
    /// let path_vec: Vec<Label> = path.0.into();
    /// assert_eq!(
    ///     path_vec,
    ///     vec![Label::root(), unsound::label::new("foo"), unsound::label::new("bar"),
    ///     unsound::label::new("baz.rs")]
    /// );
    /// ```
    pub fn from_labels(root: Label, labels: &[Label]) -> Path {
        Path((root, labels.to_vec()).into())
    }

    /// Construct a `Path` using [`Label::root`](struct.Label.html#method.root)
    /// as the head of the `Path.
    ///
    /// # Examples
    ///
    /// ```
    /// use nonempty::NonEmpty;
    /// use radicle_surf::file_system::{Label, Path};
    /// use radicle_surf::file_system::unsound;
    ///
    /// let path = unsound::path::new("~/foo/bar/baz.rs");
    ///
    /// let mut expected = Path::root();
    /// expected.push(unsound::label::new("foo"));
    /// expected.push(unsound::label::new("bar"));
    /// expected.push(unsound::label::new("baz.rs"));
    ///
    /// assert_eq!(path, expected);
    /// let path_vec: Vec<Label> = path.0.into();
    /// assert_eq!(
    ///     path_vec,
    ///     vec![Label::root(), unsound::label::new("foo"), unsound::label::new("bar"),
    ///     unsound::label::new("baz.rs")]
    /// );
    /// ```
    pub fn with_root(labels: &[Label]) -> Path {
        Path::from_labels(Label::root(), labels)
    }
}

impl TryFrom<path::PathBuf> for Path {
    type Error = file_error::Error;

    fn try_from(path_buf: path::PathBuf) -> Result<Self, Self::Error> {
        let mut path = Path::root();
        for p in path_buf.iter() {
            let p = p.to_str().ok_or(file_error::INVALID_UTF8)?;
            let l = Label::try_from(p)?;
            path.push(l);
        }

        Ok(path)
    }
}

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
