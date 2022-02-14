use num_traits::ToPrimitive;
use std::default::Default;
use std::iter::{FromIterator, IntoIterator};

use {Commute, Partial};

/// Compute the exact median on a stream of data.
///
/// (This has time complexity `O(nlogn)` and space complexity `O(n)`.)
pub fn median<I>(it: I) -> Option<f64>
where
    I: Iterator,
    <I as Iterator>::Item: PartialOrd + ToPrimitive,
{
    it.collect::<Unsorted<_>>().median()
}

/// Compute the exact 1-, 2-, and 3-quartiles (Q1, Q2 a.k.a. median, and Q3) on a stream of data.
///
/// (This has time complexity `O(nlogn)` and space complexity `O(n)`.)
pub fn quartiles<I>(it: I) -> Option<(f64, f64, f64)>
where
    I: Iterator,
    <I as Iterator>::Item: PartialOrd + ToPrimitive,
{
    it.collect::<Unsorted<_>>().quartiles()
}

/// Compute the exact mode on a stream of data.
///
/// (This has time complexity `O(nlogn)` and space complexity `O(n)`.)
///
/// If the data does not have a mode, then `None` is returned.
pub fn mode<T, I>(it: I) -> Option<T>
where
    T: PartialOrd + Clone,
    I: Iterator<Item = T>,
{
    it.collect::<Unsorted<T>>().mode()
}

/// Compute the modes on a stream of data.
///
/// If there is a single mode, then only that value is returned in the `Vec`
/// however, if there multiple values tied for occuring the most amount of times
/// those values are returned.
///
/// ## Example
/// ```
/// use stats;
///
/// let vals = vec![1, 1, 2, 2, 3];
///
/// assert_eq!(stats::modes(vals.into_iter()), vec![1, 2]);
/// ```
/// This has time complexity `O(n)`
///
/// If the data does not have a mode, then an empty `Vec` is returned.
pub fn modes<T, I>(it: I) -> Vec<T>
where
    T: PartialOrd + Clone,
    I: Iterator<Item = T>,
{
    it.collect::<Unsorted<T>>().modes()
}

fn median_on_sorted<T>(data: &[T]) -> Option<f64>
where
    T: PartialOrd + ToPrimitive,
{
    Some(match data.len() {
        0 => return None,
        1 => data[0].to_f64().unwrap(),
        len if len % 2 == 0 => {
            let v1 = data[(len / 2) - 1].to_f64().unwrap();
            let v2 = data[len / 2].to_f64().unwrap();
            (v1 + v2) / 2.0
        }
        len => data[len / 2].to_f64().unwrap(),
    })
}

fn quartiles_on_sorted<T>(data: &[T]) -> Option<(f64, f64, f64)>
where
    T: PartialOrd + ToPrimitive,
{
    Some(match data.len() {
        0..=2 => return None,
        3 => unsafe {
            (
                data.get_unchecked(0).to_f64().unwrap(),
                data.get_unchecked(1).to_f64().unwrap(),
                data.get_unchecked(2).to_f64().unwrap(),
            )
        },
        len => {
            let r = len % 4;
            let k = (len - r) / 4;
            match r {
                // Let data = {x_i}_{i=0..4k} where k is positive integer.
                // Median q2 = (x_{2k-1} + x_{2k}) / 2.
                // If we divide data into two parts {x_i < q2} as L and
                // {x_i > q2} as R, #L == #R == 2k holds true. Thus,
                // q1 = (x_{k-1} + x_{k}) / 2 and q3 = (x_{3k-1} + x_{3k}) / 2.
                0 => unsafe {
                    let (q1_l, q1_r, q2_l, q2_r, q3_l, q3_r) = (
                        data.get_unchecked(k - 1).to_f64().unwrap(),
                        data.get_unchecked(k).to_f64().unwrap(),
                        data.get_unchecked(2 * k - 1).to_f64().unwrap(),
                        data.get_unchecked(2 * k).to_f64().unwrap(),
                        data.get_unchecked(3 * k - 1).to_f64().unwrap(),
                        data.get_unchecked(3 * k).to_f64().unwrap(),
                    );

                    ((q1_l + q1_r) / 2., (q2_l + q2_r) / 2., (q3_l + q3_r) / 2.)
                },
                // Let data = {x_i}_{i=0..4k+1} where k is positive integer.
                // Median q2 = x_{2k}.
                // If we divide data other than q2 into two parts {x_i < q2}
                // as L and {x_i > q2} as R, #L == #R == 2k holds true. Thus,
                // q1 = (x_{k-1} + x_{k}) / 2 and q3 = (x_{3k} + x_{3k+1}) / 2.
                1 => unsafe {
                    let (q1_l, q1_r, q2, q3_l, q3_r) = (
                        data.get_unchecked(k - 1).to_f64().unwrap(),
                        data.get_unchecked(k).to_f64().unwrap(),
                        data.get_unchecked(2 * k).to_f64().unwrap(),
                        data.get_unchecked(3 * k).to_f64().unwrap(),
                        data.get_unchecked(3 * k + 1).to_f64().unwrap(),
                    );
                    ((q1_l + q1_r) / 2., q2, (q3_l + q3_r) / 2.)
                },
                // Let data = {x_i}_{i=0..4k+2} where k is positive integer.
                // Median q2 = (x_{(2k+1)-1} + x_{2k+1}) / 2.
                // If we divide data into two parts {x_i < q2} as L and
                // {x_i > q2} as R, it's true that #L == #R == 2k+1.
                // Thus, q1 = x_{k} and q3 = x_{3k+1}.
                2 => unsafe {
                    let (q1, q2_l, q2_r, q3) = (
                        data.get_unchecked(k).to_f64().unwrap(),
                        data.get_unchecked(2 * k).to_f64().unwrap(),
                        data.get_unchecked(2 * k + 1).to_f64().unwrap(),
                        data.get_unchecked(3 * k + 1).to_f64().unwrap(),
                    );
                    (q1, (q2_l + q2_r) / 2., q3)
                }
                // Let data = {x_i}_{i=0..4k+3} where k is positive integer.
                // Median q2 = x_{2k+1}.
                // If we divide data other than q2 into two parts {x_i < q2}
                // as L and {x_i > q2} as R, #L == #R == 2k+1 holds true.
                // Thus, q1 = x_{k} and q3 = x_{3k+2}.
                _ => unsafe {
                    let (q1, q2, q3) = (
                        data.get_unchecked(k).to_f64().unwrap(),
                        data.get_unchecked(2 * k + 1).to_f64().unwrap(),
                        data.get_unchecked(3 * k + 2).to_f64().unwrap(),
                    );
                    (q1, q2, q3)
                }
            }
        }
    })
}

fn mode_on_sorted<T, I>(it: I) -> Option<T>
where
    T: PartialOrd,
    I: Iterator<Item = T>,
{
    // This approach to computing the mode works very nicely when the
    // number of samples is large and is close to its cardinality.
    // In other cases, a hashmap would be much better.
    // But really, how can we know this when given an arbitrary stream?
    // Might just switch to a hashmap to track frequencies. That would also
    // be generally useful for discovering the cardinality of a sample.
    let (mut mode, mut next) = (None, None);
    let (mut mode_count, mut next_count) = (0usize, 0usize);
    for x in it {
        if mode.as_ref().map(|y| y == &x).unwrap_or(false) {
            mode_count += 1;
        } else if next.as_ref().map(|y| y == &x).unwrap_or(false) {
            next_count += 1;
        } else {
            next = Some(x);
            next_count = 0;
        }

        #[allow(clippy::comparison_chain)]
        if next_count > mode_count {
            mode = next;
            mode_count = next_count;
            next = None;
            next_count = 0;
        } else if next_count == mode_count {
            mode = None;
            mode_count = 0usize;
        }
    }
    mode
}

fn modes_on_sorted<T, I>(it: I) -> Vec<T>
where
    T: PartialOrd,
    I: Iterator<Item = T>,
{
    let mut highest_mode = 1_u32;
    let mut modes: Vec<u32> = vec![];
    let mut values = vec![];
    let mut count = 0;
    for x in it {
        if values.is_empty() {
            values.push(x);
            modes.push(1);
            continue;
        }
        if x == values[count] {
            modes[count] += 1;
            if highest_mode < modes[count] {
                highest_mode = modes[count];
            }
        } else {
            values.push(x);
            modes.push(1);
            count += 1;
        }
    }
    modes
        .into_iter()
        .zip(values)
        .filter(|(cnt, _val)| *cnt == highest_mode && highest_mode > 1)
        .map(|(_, val)| val)
        .collect()
}

/// A commutative data structure for lazily sorted sequences of data.
///
/// The sort does not occur until statistics need to be computed.
///
/// Note that this works on types that do not define a total ordering like
/// `f32` and `f64`. When an ordering is not defined, an arbitrary order
/// is returned.
#[derive(Clone)]
pub struct Unsorted<T> {
    data: Vec<Partial<T>>,
    sorted: bool,
}

impl<T: PartialOrd> Unsorted<T> {
    /// Create initial empty state.
    #[inline]
    pub fn new() -> Unsorted<T> {
        Default::default()
    }

    /// Add a new element to the set.
    #[inline]
    pub fn add(&mut self, v: T) {
        self.dirtied();
        self.data.push(Partial(v))
    }

    /// Return the number of data points.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Return true if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    fn sort(&mut self) {
        if !self.sorted {
            self.data.sort_unstable();
        }
    }

    #[inline]
    fn dirtied(&mut self) {
        self.sorted = false;
    }
}

impl<T: PartialOrd + Eq + Clone> Unsorted<T> {
    #[inline]
    pub fn cardinality(&mut self) -> usize {
        self.sort();
        let mut set = self.data.clone();
        set.dedup();
        set.len()
    }
}

impl<T: PartialOrd + Clone> Unsorted<T> {
    /// Returns the mode of the data.
    #[inline]
    pub fn mode(&mut self) -> Option<T> {
        self.sort();
        mode_on_sorted(self.data.iter()).map(|p| p.0.clone())
    }

    /// Returns the modes of the data.
    #[inline]
    pub fn modes(&mut self) -> Vec<T> {
        self.sort();
        modes_on_sorted(self.data.iter())
            .into_iter()
            .map(|p| p.0.clone())
            .collect()
    }
}

impl<T: PartialOrd + ToPrimitive> Unsorted<T> {
    /// Returns the median of the data.
    #[inline]
    pub fn median(&mut self) -> Option<f64> {
        self.sort();
        median_on_sorted(&*self.data)
    }
}

impl<T: PartialOrd + ToPrimitive> Unsorted<T> {
    /// Returns the quartiles of the data.
    #[inline]
    pub fn quartiles(&mut self) -> Option<(f64, f64, f64)> {
        self.sort();
        quartiles_on_sorted(&*self.data)
    }
}

impl<T: PartialOrd> Commute for Unsorted<T> {
    #[inline]
    fn merge(&mut self, v: Unsorted<T>) {
        self.dirtied();
        self.data.extend(v.data.into_iter());
    }
}

impl<T: PartialOrd> Default for Unsorted<T> {
    #[inline]
    fn default() -> Unsorted<T> {
        Unsorted {
            data: Vec::with_capacity(1000),
            sorted: true,
        }
    }
}

impl<T: PartialOrd> FromIterator<T> for Unsorted<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Unsorted<T> {
        let mut v = Unsorted::new();
        v.extend(it);
        v
    }
}

impl<T: PartialOrd> Extend<T> for Unsorted<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        self.dirtied();
        self.data.extend(it.into_iter().map(Partial))
    }
}

#[cfg(test)]
mod test {
    use super::{median, mode, modes, quartiles};

    #[test]
    fn median_stream() {
        assert_eq!(median(vec![3usize, 5, 7, 9].into_iter()), Some(6.0));
        assert_eq!(median(vec![3usize, 5, 7].into_iter()), Some(5.0));
    }

    #[test]
    fn mode_stream() {
        assert_eq!(mode(vec![3usize, 5, 7, 9].into_iter()), None);
        assert_eq!(mode(vec![3usize, 3, 3, 3].into_iter()), Some(3));
        assert_eq!(mode(vec![3usize, 3, 3, 4].into_iter()), Some(3));
        assert_eq!(mode(vec![4usize, 3, 3, 3].into_iter()), Some(3));
        assert_eq!(mode(vec![1usize, 1, 2, 3, 3].into_iter()), None);
    }

    #[test]
    fn median_floats() {
        assert_eq!(median(vec![3.0f64, 5.0, 7.0, 9.0].into_iter()), Some(6.0));
        assert_eq!(median(vec![3.0f64, 5.0, 7.0].into_iter()), Some(5.0));
        assert_eq!(median(vec![1.0f64, 2.5, 3.0].into_iter()), Some(2.5));
    }

    #[test]
    fn mode_floats() {
        assert_eq!(mode(vec![3.0f64, 5.0, 7.0, 9.0].into_iter()), None);
        assert_eq!(mode(vec![3.0f64, 3.0, 3.0, 3.0].into_iter()), Some(3.0));
        assert_eq!(mode(vec![3.0f64, 3.0, 3.0, 4.0].into_iter()), Some(3.0));
        assert_eq!(mode(vec![4.0f64, 3.0, 3.0, 3.0].into_iter()), Some(3.0));
        assert_eq!(mode(vec![1.0f64, 1.0, 2.0, 3.0, 3.0].into_iter()), None);
    }

    #[test]
    fn modes_stream() {
        assert_eq!(modes(vec![3usize, 5, 7, 9].into_iter()), vec![]);
        assert_eq!(modes(vec![3usize, 3, 3, 3].into_iter()), vec![3]);
        assert_eq!(modes(vec![3usize, 3, 4, 4].into_iter()), vec![3, 4]);
        assert_eq!(modes(vec![4usize, 3, 3, 3].into_iter()), vec![3]);
        assert_eq!(modes(vec![1usize, 1, 2, 2].into_iter()), vec![1, 2]);
        let vec: Vec<u32> = vec![];
        assert_eq!(modes(vec.into_iter()), vec![]);
    }

    #[test]
    fn modes_floats() {
        assert_eq!(modes(vec![3_f64, 5.0, 7.0, 9.0].into_iter()), vec![]);
        assert_eq!(modes(vec![3_f64, 3.0, 3.0, 3.0].into_iter()), vec![3.0]);
        assert_eq!(
            modes(vec![3_f64, 3.0, 4.0, 4.0].into_iter()),
            vec![3.0, 4.0]
        );
        assert_eq!(
            modes(vec![1_f64, 1.0, 2.0, 3.0, 3.0].into_iter()),
            vec![1.0, 3.0]
        );
    }

    #[test]
    fn quartiles_stream() {
        assert_eq!(
            quartiles(vec![3usize, 5, 7].into_iter()),
            Some((3., 5., 7.))
        );
        assert_eq!(
            quartiles(vec![3usize, 5, 7, 9].into_iter()),
            Some((4., 6., 8.))
        );
        assert_eq!(
            quartiles(vec![1usize, 2, 7, 11].into_iter()),
            Some((1.5, 4.5, 9.))
        );
        assert_eq!(
            quartiles(vec![3usize, 5, 7, 9, 12].into_iter()),
            Some((4., 7., 10.5))
        );
        assert_eq!(
            quartiles(vec![2usize, 2, 3, 8, 10].into_iter()),
            Some((2., 3., 9.))
        );
        assert_eq!(
            quartiles(vec![3usize, 5, 7, 9, 12, 20].into_iter()),
            Some((5., 8., 12.))
        );
        assert_eq!(
            quartiles(vec![0usize, 2, 4, 8, 10, 11].into_iter()),
            Some((2., 6., 10.))
        );
        assert_eq!(
            quartiles(vec![3usize, 5, 7, 9, 12, 20, 21].into_iter()),
            Some((5., 9., 20.))
        );
        assert_eq!(
            quartiles(vec![1usize, 5, 6, 6, 7, 10, 19].into_iter()),
            Some((5., 6., 10.))
        );
    }

    #[test]
    fn quartiles_floats() {
        assert_eq!(
            quartiles(vec![3_f64, 5., 7.].into_iter()),
            Some((3., 5., 7.))
        );
        assert_eq!(
            quartiles(vec![3_f64, 5., 7., 9.].into_iter()),
            Some((4., 6., 8.))
        );
        assert_eq!(
            quartiles(vec![3_f64, 5., 7., 9., 12.].into_iter()),
            Some((4., 7., 10.5))
        );
        assert_eq!(
            quartiles(vec![3_f64, 5., 7., 9., 12., 20.].into_iter()),
            Some((5., 8., 12.))
        );
        assert_eq!(
            quartiles(vec![3_f64, 5., 7., 9., 12., 20., 21.].into_iter()),
            Some((5., 9., 20.))
        );
    }
}
