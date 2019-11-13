use crate::file_system::Directory;
use nonempty::NonEmpty;

#[derive(Clone)]
pub struct History<A>(pub NonEmpty<A>);

pub struct Repo<A>(pub Vec<History<A>>);

pub trait Snapshot<A> {
    fn apply_snapshot(history: &History<A>) -> Directory;
}

pub struct Browser<'a, A, S> {
    snapshot: &'a S,
    history: History<A>,
}

pub enum ViewResult {
    Success,
    Failure,
}

fn from_vec<T>(vec: Vec<T>) -> Option<NonEmpty<T>> {
    let mut vec = vec;
    let head = vec.pop();
    match head {
        Some(t) => {
            let mut result = NonEmpty::new(t);
            for u in vec {
                result.push(u)
            }
            Some(result)
        }
        None => None,
    }
}

impl<'a, A, S> Browser<'a, A, S> {
    pub fn get_history(&self) -> History<A>
    where
        A: Clone,
    {
        self.history.clone()
    }

    pub fn set_history(&mut self, history: History<A>) {
        self.history = history;
    }

    pub fn get_directory(&self) -> Directory
    where
        S: Snapshot<A>,
    {
        S::apply_snapshot(&self.history)
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
        let new_history: Option<NonEmpty<A>> = from_vec(
            self.history
                .0
                .iter()
                .cloned()
                .take_while(|current| *current != artifact)
                .collect(),
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
