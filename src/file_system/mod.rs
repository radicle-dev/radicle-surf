use either::Either;
use nonempty::NonEmpty;

pub struct Label(pub String);

pub trait IsRepo {
    fn new() -> Directory;
}

pub struct Repo {}

pub enum DirectoryContents {
    SubDirectory(Box<Directory>),
    File(File),
    Repo(Repo),
}

pub struct Directory {
    pub label: Label,
    pub entries: NonEmpty<DirectoryContents>,
}

pub struct File {
    pub filename: Label,
    pub contents: String,
}

pub enum SystemType {
    IsFile,
    IsDirectory,
}

pub fn root_label() -> Label {
    Label(String::from("~"))
}

pub fn list_directory(directory: Directory) -> NonEmpty<(Label, SystemType)> {
    /*
    directory
        .entries
        .into()
        .iter()
        .map(|entry: Either<Box<Directory>, File>| {
            entry.either(
                |dir| (dir.label, SystemType::IsDirectory),
                |file| (file.filename, SystemType::IsFile),
            )
        })
    */
    panic!("TODO")
}

pub fn find_file(path: NonEmpty<Label>, directory: Directory) -> Option<File> {
    panic!("TODO")
}

pub fn find_directory(path: NonEmpty<Label>, directory: Directory) -> Option<Directory> {
    panic!("TODO")
}

pub fn fuzzy_find(label: Label) -> Vec<Directory> {
    panic!("TODO")
}

fn only_directory(entries: NonEmpty<Either<Box<Directory>, File>>) -> Vec<Directory> {
    panic!("TODO")
}

fn get_sub_directories(directory: Directory) -> Vec<Directory> {
    panic!("TODO")
}
