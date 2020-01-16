use crate::file_system::path::*;
use crate::tree::*;
use std::collections::hash_map::DefaultHasher;
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
pub struct FileT {
    pub contents: Vec<u8>,
    pub(crate) size: usize,
}

impl FileT {
    pub fn new(contents: &[u8]) -> Self {
        let size = contents.len();
        FileT {
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

pub type DirectoryT = Forest<Label, FileT>;

pub type SubDirectoryT = SubTree<Label, FileT>;

impl DirectoryT {
    /// List the current `Directory`'s files and sub-directories.
    pub fn list_directory(&self) -> Vec<(Label, SystemType)> {
        match self.0.as_ref() {
            None => vec![],
            Some(forest) => forest
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
    pub fn find_file(&self, path: &Path) -> Option<FileT> {
        self.find_node(&path.0).cloned()
    }

    /// Find a `Directory` in the directory given the `Path` to
    /// the `Directory`.
    ///
    /// This operation fails if the path does not lead to
    /// the `Directory`.
    pub fn find_directory(&self, path: &Path) -> Option<SubDirectoryT> {
        self.find(&path.0).cloned()
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
        self.iter().fold(0, |size, file| size + file.size())
    }

    /*
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
            self.0
                .map_or(0, |trees| trees.iter().map(|tree| unimplemented!()))
            /*
            self.entries
                .iter()
                .map(|entry| match entry {
                    DirectoryContents::Repo => 0,
                    DirectoryContents::File(file) => file.size(),
                    DirectoryContents::SubDirectory(directory) => directory.size(),
                })
                .sum()
                */
        }
    */

    /*
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
    */
}
