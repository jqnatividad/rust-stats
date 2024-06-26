use ahash::AHashMap;
use std::collections::hash_map::{Entry, Keys};
use std::fmt;
use std::hash::Hash;

use rayon::prelude::*;

use crate::Commute;
/// A commutative data structure for exact frequency counts.
#[derive(Clone)]
pub struct Frequencies<T> {
    data: AHashMap<T, u64>,
}

#[cfg(debug_assertions)]
impl<T: fmt::Debug + Eq + Hash> fmt::Debug for Frequencies<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

impl<T: Eq + Hash> Frequencies<T> {
    /// Create a new frequency table with no samples.
    #[must_use]
    pub fn new() -> Frequencies<T> {
        Default::default()
    }

    /// Add a sample to the frequency table.
    #[inline]
    pub fn add(&mut self, v: T) {
        match self.data.entry(v) {
            Entry::Vacant(count) => {
                count.insert(1);
            }
            Entry::Occupied(mut count) => {
                *count.get_mut() += 1;
            }
        }
    }

    /// Return the number of occurrences of `v` in the data.
    #[inline]
    #[must_use]
    pub fn count(&self, v: &T) -> u64 {
        self.data.get(v).copied().unwrap_or(0)
    }

    /// Return the cardinality (number of unique elements) in the data.
    #[inline]
    #[must_use]
    pub fn cardinality(&self) -> u64 {
        self.len() as u64
    }

    /// Returns the mode if one exists.
    #[inline]
    #[must_use]
    pub fn mode(&self) -> Option<&T> {
        let (counts, _) = self.most_frequent();
        if counts.is_empty() {
            return None;
        }
        // If there is a tie for the most frequent element, return None.
        if counts.len() >= 2 && counts[0].1 == counts[1].1 {
            None
        } else {
            Some(counts[0].0)
        }
    }

    /// Return a `Vec` of elements, their corresponding counts in
    /// descending order, and the total count.
    #[inline]
    #[must_use]
    pub fn most_frequent(&self) -> (Vec<(&T, u64)>, u64) {
        let mut total_count = 0_u64;
        let mut counts: Vec<_> = self
            .data
            .iter()
            .map(|(k, &v)| {
                total_count += v;
                (k, v)
            })
            .collect();
        counts.sort_unstable_by(|&(_, c1), &(_, c2)| c2.cmp(&c1));
        (counts, total_count)
    }

    /// Return a `Vec` of elements, their corresponding counts in
    /// ascending order, and the total count.
    #[inline]
    #[must_use]
    pub fn least_frequent(&self) -> (Vec<(&T, u64)>, u64) {
        let mut total_count = 0_u64;
        let mut counts: Vec<_> = self
            .data
            .iter()
            .map(|(k, &v)| {
                total_count += v;
                (k, v)
            })
            .collect();
        counts.sort_unstable_by(|&(_, c1), &(_, c2)| c1.cmp(&c2));
        (counts, total_count)
    }

    /// Return a `Vec` of elements, their corresponding counts in order
    /// based on the `least` parameter, and the total count. Uses parallel sort.
    #[inline]
    #[must_use]
    pub fn par_frequent(&self, least: bool) -> (Vec<(&T, u64)>, u64)
    where
        for<'a> (&'a T, u64): Send,
        T: Ord,
    {
        let mut total_count = 0_u64;
        let mut counts: Vec<_> = self
            .data
            .iter()
            .map(|(k, &v)| {
                total_count += v;
                (k, v)
            })
            .collect();
        // sort by counts asc/desc
        // if counts are equal, sort by values lexicographically
        // We need to do this because otherwise the values are not guaranteed to be in order for equal counts
        if least {
            // return counts in ascending order
            counts.par_sort_unstable_by(|&(v1, c1), &(v2, c2)| {
                let cmp = c1.cmp(&c2);
                if cmp == std::cmp::Ordering::Equal {
                    v1.cmp(v2)
                } else {
                    cmp
                }
            });
        } else {
            // return counts in descending order
            counts
                .par_sort_unstable_by(|&(v1, c1), &(v2, c2)| c2.cmp(&c1).then_with(|| v1.cmp(v2)));
        }
        (counts, total_count)
    }

    /// Returns the cardinality of the data.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if there is no frequency/cardinality data.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Return an iterator over the unique values of the data.
    #[must_use]
    pub fn unique_values(&self) -> UniqueValues<T> {
        UniqueValues {
            data_keys: self.data.keys(),
        }
    }
}

impl<T: Eq + Hash> Commute for Frequencies<T> {
    #[inline]
    fn merge(&mut self, v: Frequencies<T>) {
        for (k, v2) in v.data {
            match self.data.entry(k) {
                Entry::Vacant(v1) => {
                    v1.insert(v2);
                }
                Entry::Occupied(mut v1) => {
                    *v1.get_mut() += v2;
                }
            }
        }
    }
}

impl<T: Eq + Hash> Default for Frequencies<T> {
    #[inline]
    fn default() -> Frequencies<T> {
        Frequencies {
            data: AHashMap::with_capacity(10_000),
        }
    }
}

impl<T: Eq + Hash> FromIterator<T> for Frequencies<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Frequencies<T> {
        let mut v = Frequencies::new();
        v.extend(it);
        v
    }
}

impl<T: Eq + Hash> Extend<T> for Frequencies<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for sample in it {
            self.add(sample);
        }
    }
}

/// An iterator over unique values in a frequencies count.
pub struct UniqueValues<'a, K> {
    data_keys: Keys<'a, K, u64>,
}

impl<'a, K> Iterator for UniqueValues<'a, K> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        self.data_keys.next()
    }
}

#[cfg(test)]
mod test {
    use super::Frequencies;
    use std::iter::FromIterator;

    #[test]
    fn ranked() {
        let mut counts = Frequencies::new();
        counts.extend(vec![1usize, 1, 2, 2, 2, 2, 2, 3, 4, 4, 4].into_iter());
        let (most_count, most_total) = counts.most_frequent();
        assert_eq!(most_count[0], (&2, 5));
        assert_eq!(most_total, 11);
        let (least_count, least_total) = counts.least_frequent();
        assert_eq!(least_count[0], (&3, 1));
        assert_eq!(least_total, 11);
        // assert_eq!(counts.least_frequent()[0], (&3, 1));
    }

    #[test]
    fn ranked2() {
        let mut counts = Frequencies::new();
        counts.extend(vec![1usize, 1, 2, 2, 2, 2, 2, 3, 4, 4, 4].into_iter());
        let (most_count, most_total) = counts.par_frequent(false);
        assert_eq!(most_count[0], (&2, 5));
        assert_eq!(most_total, 11);
        let (least_count, least_total) = counts.par_frequent(true);
        assert_eq!(least_count[0], (&3, 1));
        assert_eq!(least_total, 11);
    }

    #[test]
    fn unique_values() {
        let freqs = Frequencies::from_iter(vec![8, 6, 5, 1, 1, 2, 2, 2, 3, 4, 7, 4, 4]);
        let mut unique: Vec<isize> = freqs.unique_values().copied().collect();
        unique.sort_unstable();
        assert_eq!(unique, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
