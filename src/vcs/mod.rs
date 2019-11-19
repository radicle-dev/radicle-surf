use crate::file_system::Directory;
use nonempty::NonEmpty;

pub mod git;

/// A non-empty bag of artifacts which are used to
/// derive a `Directory` view. Examples of artifacts
/// would be commits in Git or patches in Pijul.
#[derive(Clone)]
pub struct History<A>(pub NonEmpty<A>);

impl<A> History<A> {
    /// Iterator over the artifacts.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &A> + 'a {
        self.0.iter()
    }

    /// Given that the `History` is topological order from most
    /// recent artifact to least recent, `find_suffix` gets returns
    /// the history up until the point of the given artifact.
    ///
    /// This operation may fail if the artifact does not exist in
    /// the given `History`.
    pub fn find_suffix(&self, artifact: &A) -> Option<Self>
    where
        A: Clone + PartialEq,
    {
        let new_history: Option<NonEmpty<A>> = NonEmpty::from_slice(
            &self
                .iter()
                .cloned()
                .skip_while(|current| *current != *artifact)
                .collect::<Vec<_>>(),
        );

        new_history.map(History)
    }

    /// Find an atrifact in the given `History` using the artifacts ID.
    ///
    /// This operation may fail if the artifact does not exist in the history.
    pub fn find_in_history<Identifier, F>(&self, identifier: &Identifier, id_of: F) -> Option<A>
    where
        A: Clone,
        F: Fn(&A) -> &Identifier,
        Identifier: PartialEq,
    {
        self.iter()
            .find(|artifact| {
                let current_id = id_of(&artifact);
                *identifier == *current_id
            })
            .cloned()
    }

    /// Find all occurences of an artifact in a bag of `History`s.
    pub fn find_in_histories<Identifier, F>(
        histories: Vec<Self>,
        identifier: &Identifier,
        id_of: F,
    ) -> Vec<Self>
    where
        A: Clone,
        F: Fn(&A) -> &Identifier + Copy,
        Identifier: PartialEq,
    {
        histories
            .into_iter()
            .filter(|history| history.find_in_history(identifier, id_of).is_some())
            .collect()
    }
}

/// A `Repo` is a bag of `History`s. If the bag is empty
/// then the `Repo` is in its initial state.
pub struct Repo<A>(pub Vec<History<A>>);

/// A `Browser` is a way of rendering a `History` into a
/// `Directory` snapshot, and the current `History` it is
/// viewing.
pub struct Browser<'browser, Repo, A> {
    snapshot: Box<dyn Fn(&Repo, &History<A>) -> Directory + 'browser>,
    history: History<A>,
    repository: &'browser Repo,
}

impl<'browser, Repo, A> Browser<'browser, Repo, A> {
    /// Get the current `History` the `Browser` is viewing.
    pub fn get_history(&self) -> History<A>
    where
        A: Clone,
    {
        self.history.clone()
    }

    /// Set the `History` the `Browser` should view.
    pub fn set_history(&mut self, history: History<A>) {
        self.history = history;
    }

    /// Render the `Directory` for this `Browser`.
    pub fn get_directory(&self) -> Directory {
        (self.snapshot)(&self.repository, &self.history)
    }

    /// Modify the `History` in this `Browser`.
    pub fn modify_history<F>(&mut self, f: F)
    where
        F: Fn(&History<A>) -> History<A>,
    {
        self.history = f(&self.history)
    }

    /// Change the `Browser`'s view of `History` by modifying it, or
    /// using the default `History` provided if the operation fails.
    pub fn view_at<F>(&mut self, default_history: History<A>, f: F)
    where
        A: PartialEq + Clone,
        F: Fn(&History<A>) -> Option<History<A>>,
    {
        self.modify_history(|history| f(history).unwrap_or(default_history.clone()))
    }
}

pub trait VCS<'repo, A: 'repo>
where
    Self: 'repo + Sized,
{
    /// The way to identify a Repository.
    type RepoId;

    /// The History type to work with, e.g. Branch, Tag in git.
    type History;

    /// The way to identify a History.
    type HistoryId;

    /// The way to identify an artifact.
    type ArtefactId;

    /// Find a Repository
    fn get_repo(identifier: &Self::RepoId) -> Option<Self>;

    /// Find a History in a Repo given a way to identify it
    fn get_history(&'repo self, identifier: &Self::HistoryId) -> Option<Self::History>;

    /// Find all histories in a Repo
    fn get_histories(&'repo self) -> Vec<Self::History>;

    /// Identify artifacts of a Repository
    fn get_identifier(artifact: &'repo A) -> Self::ArtefactId;

    /// Turn a Repository History into a radicle-surf History
    fn to_history(&'repo self, history: Self::History) -> Option<History<A>>;

    /// Turn a Repository into a radicle-surf Repository
    fn to_repo(&'repo self) -> Repo<A> {
        let histories = self
            .get_histories()
            .into_iter()
            .filter_map(|h| self.to_history(h));
        Repo(histories.collect())
    }
}
