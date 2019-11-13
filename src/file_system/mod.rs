use nonempty::NonEmpty;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub String);

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

#[derive(Debug, Clone)]
pub struct Directory<Repo> {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents<Repo>>,
}

#[derive(Debug, Clone)]
pub struct File {
    pub filename: Label,
    pub contents: String,
}

#[derive(Debug, Clone)]
pub enum SystemType {
    IsFile,
    IsDirectory,
}

impl<Repo> Directory<Repo> {
    pub fn empty_root() -> Directory<Repo>
    where
        Repo: RepoBackend,
    {
        Repo::new()
    }

    pub fn root_label() -> Label {
        Label(String::from("~"))
    }

    pub fn list_directory(&self) -> Vec<(Label, SystemType)>
    where
        Repo: Clone,
    {
        self.entries
            .iter()
            .cloned()
            .filter_map(|entry| match entry {
                DirectoryContents::SubDirectory(dir) => Some((dir.label, SystemType::IsDirectory)),
                DirectoryContents::File(file) => Some((file.filename, SystemType::IsFile)),
                DirectoryContents::Repo(_) => None,
            })
            .collect()
    }

    pub fn find_file(&self, path: NonEmpty<Label>) -> Option<File>
    where
        Repo: Clone,
    {
        let mut file = None;
        let mut search_directory = Some(self.clone());
        for label in path.iter() {
            match search_directory {
                // We could not find a sub-directory so we bail out
                None => return None,

                // We have a viable sub-directory that we will search in
                Some(dir) => {
                    // Really all this is doing is making sure that when we get to the last
                    // label we check that the file is in this directory. Its returned on the
                    // outside of the loop.
                    file = dir.file_in_directory(label);

                    // Update the sub-directory to search.
                    search_directory = dir.get_sub_directory(label);
                }
            }
        }
        file
    }

    pub fn find_directory(&self, path: NonEmpty<Label>) -> Option<Self>
    where
        Repo: Clone,
    {
        let mut search_directory = Some(self.clone());
        for label in path.iter() {
            match search_directory {
                None => return None,
                Some(dir) => {
                    // Update the sub-directory to search.
                    search_directory = dir.get_sub_directory(label);
                }
            }
        }
        search_directory
    }

    // TODO(fintan): This is going to be a bit trickier so going to leave it out for now
    #[allow(dead_code)]
    pub fn fuzzy_find(_label: Label) -> Vec<Self> {
        panic!("TODO")
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
                DirectoryContents::File(file) => {
                    if file.filename == *label {
                        return Some(file.clone());
                    } else {
                        continue;
                    }
                }
                DirectoryContents::SubDirectory(_) => continue,
                DirectoryContents::Repo(_) => continue,
            }
        }
        None
    }
}
