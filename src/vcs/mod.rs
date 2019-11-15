use crate::file_system::Directory;
use nonempty::NonEmpty;

#[derive(Clone)]
pub struct History<A>(pub NonEmpty<A>);

impl<A> History<A> {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &A> + 'a {
        self.0.iter()
    }
}

pub struct Repo<A>(pub Vec<History<A>>);

pub struct Browser<'browser, Repo, A> {
    snapshot: Box<dyn Fn(&History<A>) -> Directory<Repo> + 'browser>,
    history: History<A>,
}

pub enum ViewResult {
    Success,
    Failure,
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

    pub fn view_at(&mut self, artifact: A) -> ViewResult
    where
        A: PartialEq + Clone,
    {
        let new_history: Option<NonEmpty<A>> = NonEmpty::from_slice(
            &self
                .history
                .iter()
                .cloned()
                .take_while(|current| *current != artifact)
                .collect::<Vec<_>>(),
        );
        match new_history {
            Some(h) => {
                self.set_history(History(h));
                ViewResult::Success
            }
            None => ViewResult::Failure,
        }
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
        let history: Vec<A> = history.0.into();
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
