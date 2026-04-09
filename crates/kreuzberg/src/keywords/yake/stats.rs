// Inlined from streaming-stats 0.2.3 — Welford's online algorithm for mean/stddev,
// plus a simple median function. Replaces the streaming-stats + num_traits dependencies.

/// Online statistics accumulator using Welford's algorithm.
#[derive(Debug, Clone)]
pub(crate) struct OnlineStats {
    n: u64,
    mean: f64,
    m2: f64,
}

impl OnlineStats {
    #[inline]
    pub fn new() -> Self {
        Self {
            n: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    #[inline]
    pub fn add(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    #[inline]
    pub fn mean(&self) -> f64 {
        self.mean
    }

    #[inline]
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    #[inline]
    fn variance(&self) -> f64 {
        if self.n < 2 { 0.0 } else { self.m2 / self.n as f64 }
    }
}

impl FromIterator<f64> for OnlineStats {
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        let mut stats = OnlineStats::new();
        for x in iter {
            stats.add(x);
        }
        stats
    }
}

/// Compute the median of an iterator of f64 values.
/// Returns `None` if the iterator is empty.
pub(crate) fn median(iter: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<f64> = iter.collect();
    if values.is_empty() {
        return None;
    }
    let len = values.len();
    let mid = len / 2;
    // Use partial sort via select_nth_unstable for O(n) average
    values.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if len % 2 == 1 {
        Some(values[mid])
    } else {
        let right = values[mid];
        let left = values[..mid]
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        Some((left + right) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_stats_basic() {
        let stats: OnlineStats = [1.0, 2.0, 3.0, 4.0, 5.0].into_iter().collect();
        assert!((stats.mean() - 3.0).abs() < 1e-10);
        assert!((stats.stddev() - (2.0_f64).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn median_odd() {
        assert_eq!(median([3.0, 1.0, 2.0].into_iter()), Some(2.0));
    }

    #[test]
    fn median_even() {
        assert_eq!(median([4.0, 1.0, 3.0, 2.0].into_iter()), Some(2.5));
    }

    #[test]
    fn median_empty() {
        assert_eq!(median(std::iter::empty()), None);
    }
}
