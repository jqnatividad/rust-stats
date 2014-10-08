use std::default::Default;
use std::f64;
use std::fmt;

use Crdt;

/// Compute the standard deviation of a stream in constant space.
pub fn stddev<T: ToPrimitive, I: Iterator<T>>(mut it: I) -> f64 {
    it.collect::<Variance>().stddev()
}

/// Compute the variance of a stream in constant space.
pub fn variance<T: ToPrimitive, I: Iterator<T>>(mut it: I) -> f64 {
    it.collect::<Variance>().variance()
}

/// Compute the mean of a stream in constant space.
pub fn mean<T: ToPrimitive, I: Iterator<T>>(mut it: I) -> f64 {
    it.collect::<Variance>().mean()
}

/// Online state for computing mean, variance and standard deviation.
#[deriving(Clone)]
pub struct Variance {
    size: u64,
    mean: f64,
    variance: f64,
}

impl Variance {
    /// Create initial state.
    ///
    /// Population size, variance and mean are set to `0`.
    pub fn new() -> Variance {
        Default::default()
    }

    /// Initializes variance from a sample.
    pub fn from_slice<T: ToPrimitive>(samples: &[T]) -> Variance {
        samples.iter().map(|n| n.to_f64().unwrap()).collect()
    }

    /// Return the current mean.
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Return the current standard deviation.
    pub fn stddev(&self) -> f64 {
        self.variance.sqrt()
    }

    /// Return the current variance.
    pub fn variance(&self) -> f64 {
        self.variance
    }

    /// Add a new sample.
    pub fn add<T: ToPrimitive>(&mut self, sample: T) {
        let sample = sample.to_f64().unwrap();

        // Taken from: http://goo.gl/JKeqvj
        // See also: http://goo.gl/qTtI3V
        let oldmean = self.mean;
        let prevq = self.variance * (self.size as f64);

        self.size += 1;
        self.mean += (sample - oldmean) / (self.size as f64);
        self.variance = (prevq + (sample - oldmean) * (sample - self.mean))
                        / (self.size as f64);
    }
}

impl Crdt for Variance {
    fn merge(&mut self, v: &Variance) {
        // Taken from: http://goo.gl/iODi28
        let (s1, s2) = (self.size as f64, v.size as f64);
        let meandiffsq = (self.mean - v.mean) * (self.mean - v.mean);
        let mean = ((s1 * self.mean) + (s2 * v.mean)) / (s1 + s2);
        let var = (((s1 * self.variance) + (s2 * v.variance))
                   / (s1 + s2))
                  +
                  ((s1 * s2 * meandiffsq) / ((s1 + s2) * (s1 + s2)));
        self.size += v.size;
        self.mean = mean;
        self.variance = var;
    }
}

impl Default for Variance {
    fn default() -> Variance {
        Variance {
            size: 0,
            mean: 0.0,
            variance: 0.0,
        }
    }
}

impl fmt::Show for Variance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} +/- {}",
               f64::to_str_digits(self.mean(), 10),
               f64::to_str_digits(self.stddev(), 10))
    }
}

impl Collection for Variance {
    fn len(&self) -> uint {
        self.size as uint
    }
}

impl Mutable for Variance {
    fn clear(&mut self) {
        self.size = 0;
        self.mean = 0.0;
        self.variance = 0.0;
    }
}

impl<T: ToPrimitive> FromIterator<T> for Variance {
    fn from_iter<I: Iterator<T>>(it: I) -> Variance {
        let mut v: Variance = Default::default();
        v.extend(it);
        v
    }
}

impl<T: ToPrimitive> Extendable<T> for Variance {
    fn extend<I: Iterator<T>>(&mut self, mut it: I) {
        for sample in it {
            self.add(sample)
        }
    }
}

#[cfg(test)]
mod test {
    use {Crdt, merge_all};
    use super::Variance;

    #[test]
    fn stddev() {
        // TODO: Convert this to a quickcheck test.
        let expected = Variance::from_slice([1u, 2, 3, 2, 4, 6]);

        let var1 = Variance::from_slice([1u, 2, 3]);
        let var2 = Variance::from_slice([2u, 4, 6]);
        let mut got = var1.clone();
        got.merge(&var2);
        assert_eq!(expected.stddev(), got.stddev());
    }

    #[test]
    fn stddev_many() {
        // TODO: Convert this to a quickcheck test.
        let expected = Variance::from_slice([1u, 2, 3, 2, 4, 6, 3, 6, 9]);

        let vars = vec![
            Variance::from_slice([1u, 2, 3]),
            Variance::from_slice([2u, 4, 6]),
            Variance::from_slice([3u, 6, 9]),
        ];
        assert_eq!(expected.stddev(), merge_all(vars.into_iter()).stddev());
    }
}
