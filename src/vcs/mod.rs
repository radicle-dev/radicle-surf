use crate::file_system::Directory;
use nonempty::NonEmpty;

#[derive(Clone)]
pub struct History<A>(pub NonEmpty<A>);

impl<A> History<A> {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &A> + 'a {
        self.0.iter()
    }

    pub fn find_suffix(&self, artifact: &A) -> Option<Self>
    where
        A: Clone + PartialEq,
    {
        let new_history: Option<NonEmpty<A>> = NonEmpty::from_slice(
            &self
                .iter()
                .cloned()
                .take_while(|current| *current != *artifact)
                .collect::<Vec<_>>(),
        );

        new_history.map(History)
    }
}

pub struct Repo<A>(pub Vec<History<A>>);

pub struct Browser<'browser, Repo, A> {
    snapshot: Box<dyn Fn(&History<A>) -> Directory<Repo> + 'browser>,
    history: History<A>,
}

impl<'browser, Repo, A> Browser<'browser, Repo, A> {
    pub fn get_history(&self) -> History<A>
    where
        A: Clone,
    {
        self.history.clone()
    }

    pub fn set_history(&mut self, history: History<A>) {
        self.history = history;
    }

    pub fn get_directory(&self) -> Directory<Repo> {
        (self.snapshot)(&self.history)
    }

    pub fn modify_history<F>(&mut self, f: F)
    where
        F: Fn(&History<A>) -> History<A>,
    {
        self.history = f(&self.history)
    }

    pub fn view_at<F>(&mut self, default_history: History<A>, f: F)
    where
        A: PartialEq + Clone,
        F: Fn(&History<A>) -> Option<History<A>>,
    {
        self.modify_history(|history| f(history).unwrap_or(default_history.clone()))
    }
}

pub trait GetRepo<A> {
    type RepoId;
    fn get_repo(identifier: &Self::RepoId) -> Repo<A>;
}

pub trait GetHistory<A> {
    type HistoryId;
    type ArtefactId;

    fn get_history(identifier: &Self::HistoryId, repo: Repo<A>) -> History<A>;

    fn get_identifier(artifact: &A) -> &Self::ArtefactId;

    fn find_in_history(identifier: &Self::ArtefactId, history: History<A>) -> Option<A>
    where
        A: Clone,
        Self::ArtefactId: PartialEq,
    {
        history
            .iter()
            .find(|artifact| {
                let current_id: &Self::ArtefactId = Self::get_identifier(&artifact);
                *identifier == *current_id
            })
            .cloned()
    }

    fn find_in_histories(
        identifier: &Self::ArtefactId,
        histories: Vec<History<A>>,
    ) -> Vec<History<A>>
    where
        A: Clone,
        Self::ArtefactId: PartialEq,
    {
        histories
            .into_iter()
            .filter(|history| Self::find_in_history(identifier, history.clone()).is_some())
            .collect()
    }
}
