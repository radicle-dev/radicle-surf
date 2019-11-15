use nonempty::NonEmpty;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub String);

impl Label {
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

pub struct Path(pub NonEmpty<Label>);

impl Path {
    pub fn root_path() -> Self {
        Path(NonEmpty::new(Label::root_label()))
    }

    pub fn is_root(&self) -> bool {
        *self.0.first() == Label::root_label() && *self.0.last() == Label::root_label()
    }

    pub fn append(&mut self, path: &mut Self) {
        path.0.iter().for_each(|l| self.0.push(l.clone()))
    }

    pub fn push(&mut self, label: Label) {
        self.0.push(label)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.0.iter()
    }

    pub fn split_first(&self) -> (&Label, &[Label]) {
        self.0.split_first()
    }

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

    pub fn from_labels(root: Label, labels: &[Label]) -> Path {
        let mut path = NonEmpty::new(root);
        labels.iter().cloned().for_each(|l| path.push(l));
        Path(path)
    }
}

pub trait RepoBackend
where
    Self: Sized,
{
    fn new() -> Directory<Self>;
}

#[derive(Debug, Clone)]
pub enum DirectoryContents<Repo> {
    SubDirectory(Box<Directory<Repo>>),
    File(File),
    Repo(Repo),
}

impl<Repo> DirectoryContents<Repo> {
    pub fn sub_directory(directory: Directory<Repo>) -> Self {
        DirectoryContents::SubDirectory(Box::new(directory))
    }

    pub fn file(filename: Label, contents: String) -> Self {
        DirectoryContents::File(File { filename, contents })
    }

    pub fn repo(repo: Repo) -> Self {
        DirectoryContents::Repo(repo)
    }
}

#[derive(Debug, Clone)]
pub struct Directory<Repo> {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents<Repo>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub filename: Label,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemType {
    File,
    Directory,
}

impl SystemType {
    pub fn file(label: Label) -> (Label, Self) {
        (label, SystemType::File)
    }

    pub fn directory(label: Label) -> (Label, Self) {
        (label, SystemType::Directory)
    }
}

impl<Repo> Directory<Repo> {
    pub fn empty_root() -> Self
    where
        Repo: RepoBackend,
    {
        Repo::new()
    }

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

    pub fn find_file(&self, path: Path) -> Option<File>
    where
        Repo: Clone + std::fmt::Debug,
    {
        let (path, filename) = path.split_last();
        let path = NonEmpty::from_slice(&path);

        let search_directory = match path {
            None => Some(self.clone()),
            Some(p) => self.find_directory(Path(p)),
        };

        search_directory.and_then(|dir| dir.file_in_directory(&filename))
    }

    pub fn find_directory(&self, path: Path) -> Option<Self>
    where
        Repo: Clone,
    {
        let mut search_directory = Some(self.clone());
        let (label, labels) = path.split_first();
        if *label == self.label {
            for label in labels {
                match search_directory {
                    None => return None,
                    Some(dir) => {
                        search_directory = dir.get_sub_directory(&label);
                    }
                }
            }
            search_directory
        } else {
            None
        }
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for now
    #[allow(dead_code)]
    pub fn fuzzy_find(_label: Label) -> Vec<Self> {
        unimplemented!()
    }

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

    fn get_sub_directory(&self, label: &Label) -> Option<Self>
    where
        Repo: Clone,
    {
        self.get_sub_directories()
            .iter()
            .cloned()
            .find(|directory| directory.label == *label)
    }

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
