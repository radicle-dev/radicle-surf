use chrono::prelude::{NaiveDateTime, DateTime, Utc};
use quickcheck::{Arbitrary, Gen};
use rand::Rng;
use rand::distributions;
use rand::distributions::Distribution;

pub(crate) type Frequency = u32;

pub(crate)  fn frequency<G: Rng, A>(g: &mut G, xs: Vec<(Frequency, A)>) -> A {
    let mut tot: u32 = 0;

    for (f, _) in &xs {
        tot += f
    }

    let choice = g.gen_range(1, tot);
    pick(choice, xs)
}

fn pick<A>(n: u32, xs: Vec<(Frequency, A)>) -> A {
    let mut acc = n;

    for (k, x) in xs {
        if acc <= k {
            return x;
        } else {
            acc -= k;
        }
    }

    panic!("QuickCheck.pick used with an empty vector");
}

#[derive(Debug, Clone)]
pub(crate) struct Datetime {
    pub(crate) get_datetime: DateTime<Utc>
}

impl Arbitrary for Datetime {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let seconds = Arbitrary::arbitrary(g);
        let nano_seconds = Arbitrary::arbitrary(g);
        Datetime {
            get_datetime: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(seconds, nano_seconds), Utc)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SmallString {
    pub(crate) get_string: String,
}

impl SmallString {
    pub(crate) fn from(s: SmallString) -> String {
        s.get_string
    }
}

impl Arbitrary for SmallString {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let n = g.gen_range(1, 50);
        SmallString {
            get_string: distributions::Alphanumeric.sample_iter(g).take(n).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SmallVec<A> {
    pub get_small_vec: Vec<A>,
}

impl<A> SmallVec<A> {
    pub(crate) fn from(v: SmallVec<A> ) -> Vec<A>  {
        v.get_small_vec
    }
}

impl<A: Arbitrary> Arbitrary for SmallVec<A>
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let m = g.gen_range(1, 10);
        let mut n = 0;
        let mut xs = Vec::with_capacity(m);

        while n < m {
            xs.push(Arbitrary::arbitrary(g));
            n += 1;
        }

        SmallVec { get_small_vec: xs }
    }
}
