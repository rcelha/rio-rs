use std::{
    collections::{BTreeSet, HashMap},
    hash::Hash,
    sync::Arc,
};

pub struct SortedSet<V> {
    storage: HashMap<V, usize>,
    by_score: BTreeSet<(usize, V)>,
}

pub type ArcSortedSet<V> = SortedSet<Arc<V>>;

impl<V> SortedSet<Arc<V>>
where
    V: Eq + Hash + Ord,
{
    /// Creates a new empty SortedSet
    pub fn new() -> SortedSet<Arc<V>> {
        SortedSet {
            storage: HashMap::new(),
            by_score: BTreeSet::new(),
        }
    }

    /// Adds a value to the set with the given score
    pub fn add(&mut self, value: V, score: usize) -> Arc<V> {
        let new_score = if let Some(curr_score) = self.score(&value) {
            curr_score + score
        } else {
            score
        };

        let inner = Arc::new(value);
        let previous_score = self.storage.insert(inner.clone(), new_score);
        if let Some(previous_score) = previous_score {
            self.by_score.remove(&(previous_score, inner.clone()));
        }
        self.by_score.insert((new_score, inner.clone()));
        inner
    }

    /// Adds a value to the set with the given score
    pub fn add_ref(&mut self, value: Arc<V>, score: usize) -> Arc<V> {
        let new_score = if let Some(curr_score) = self.storage.get(&value) {
            curr_score + score
        } else {
            score
        };
        let previous_score = self.storage.insert(value.clone(), new_score);
        if let Some(previous_score) = previous_score {
            self.by_score.remove(&(previous_score, value.clone()));
        }
        self.by_score.insert((new_score, value.clone()));
        value
    }

    /// Gets the rank of the value based on its score
    ///
    /// This is log(N) as it iterates over all the scores
    /// but it should drop to log(log N) when we replace it with a skiplist
    pub fn rank(&mut self, value: &V) -> Option<usize> {
        if let Some(score) = self.storage.get(value) {
            let count = self
                .by_score
                .iter()
                .skip_while(|x| &x.0 != score && x.1.as_ref() != value)
                .count()
                - 1;
            Some(count)
        } else {
            None
        }
    }

    pub fn score(&self, value: &V) -> Option<usize> {
        self.storage.get(value).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&V, &usize)> {
        self.storage.iter().map(|x| (x.0.as_ref(), x.1))
    }

    pub fn iter_rank(&self) -> impl Iterator<Item = &'_ V> + use<'_, V> {
        self.by_score.iter().map(|x| x.1.as_ref())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fixture() -> ArcSortedSet<&'static str> {
        let mut set = ArcSortedSet::new();
        set.add("M1", 100);
        set.add("M2", 200);
        set.add("M3", 10);
        set
    }

    #[test]
    fn strings() {
        let mut set = ArcSortedSet::new();
        set.add("M1".to_string(), 100);
        set.add("M2".to_string(), 200);
        set.add("M3".to_string(), 10);
    }

    #[test]
    fn subsequent_add() {
        let mut set = fixture();

        let rank = set.rank(&"M1");
        assert_eq!(rank, Some(1));

        let val = set.add("M1", 10);

        let rank = set.rank(&"M1");
        assert_eq!(rank, Some(1));

        set.add_ref(val, 100);
        let rank = set.rank(&"M1");
        assert_eq!(rank, Some(0));
    }

    #[test]
    fn get_rank() {
        let mut set = fixture();

        let rank = set.rank(&"M2");
        assert_eq!(rank, Some(0));

        let rank = set.rank(&"M1");
        assert_eq!(rank, Some(1));

        let rank = set.rank(&"M3");
        assert_eq!(rank, Some(2));

        let rank = set.rank(&"Member 4");
        assert!(rank.is_none())
    }

    #[test]
    fn get_score() {
        let mut set = fixture();

        let score = set.score(&"M2");
        assert_eq!(score, Some(200));

        let score = set.score(&"M1");
        assert_eq!(score, Some(100));

        let score = set.score(&"M3");
        assert_eq!(score, Some(10));

        let rank = set.rank(&"Member 4");
        assert!(rank.is_none())
    }

    #[test]
    fn iter() {
        let set = fixture();
        let mut iter = set.iter();

        let test_ = |x: (&&str, &usize)| {
            if x.0 == &"M1" {
                assert_eq!(x.1, &100);
            }

            if x.0 == &"M2" {
                assert_eq!(x.1, &200);
            }

            if x.0 == &"M3" {
                assert_eq!(x.1, &10);
            }
        };

        test_(iter.next().unwrap());
        test_(iter.next().unwrap());
        test_(iter.next().unwrap());
        assert!(iter.next().is_none());
    }
}
