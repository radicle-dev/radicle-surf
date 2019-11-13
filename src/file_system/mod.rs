use nonempty::NonEmpty;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub String);

pub trait IsRepo {
    fn new() -> Directory;
}

#[derive(Debug, Clone)]
pub struct Repo {}

#[derive(Debug, Clone)]
pub enum DirectoryContents {
    SubDirectory(Box<Directory>),
    File(File),
    Repo(Repo),
}

#[derive(Debug, Clone)]
pub struct Directory {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents>,
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

pub fn root_label() -> Label {
    Label(String::from("~"))
}

pub fn list_directory(directory: Directory) -> Vec<(Label, SystemType)> {
    directory
        .entries
        .iter()
        .cloned()
        .filter_map(|entry| match entry {
            DirectoryContents::SubDirectory(dir) => Some((dir.label, SystemType::IsDirectory)),
            DirectoryContents::File(file) => Some((file.filename, SystemType::IsFile)),
            DirectoryContents::Repo(_) => None,
        })
        .collect()
}

pub fn find_file(path: NonEmpty<Label>, directory: Directory) -> Option<File> {
    let file = None;
    let search_directory = directory;
    for label in path.iter() {
        // Really all this is doing is making sure that when we get to the last
        // label we check that the file is in this directory. Its returned on the
        // outside of the loop.
        let file = file_in_directory(label, &search_directory);

        // Update the sub-directory to search.
        let search_directory = get_sub_directory(label, &search_directory);
    }
    file
}

pub fn find_directory(path: NonEmpty<Label>, directory: Directory) -> Option<Directory> {
    panic!("TODO")
}

pub fn fuzzy_find(label: Label) -> Vec<Directory> {
    panic!("TODO")
}

fn only_directory(entries: NonEmpty<DirectoryContents>) -> Vec<Directory> {
    panic!("TODO")
}

fn get_sub_directories(directory: &Directory) -> Vec<Directory> {
    directory
        .entries
        .iter()
        .filter_map(|entry| match entry {
            DirectoryContents::SubDirectory(dir) => {
                let val: Directory = *dir;
                Some(val)
            }
            DirectoryContents::File(_) => None,
            DirectoryContents::Repo(_) => None,
        })
        .collect()
}

fn get_sub_directory(label: &Label, directory: &Directory) -> Option<Directory> {
    get_sub_directories(directory)
        .iter()
        .cloned()
        .find(|directory| directory.label == *label)
}

fn file_in_directory(label: &Label, directory: &Directory) -> Option<File> {
    for entry in directory.entries.iter() {
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
