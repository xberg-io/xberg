// Vendored from yake-rust 1.0.3 (MIT) — https://github.com/quesurifn/yake-rust
// Inlined levenshtein distance to eliminate the levenshtein crate dependency.

use super::Candidate;

/// A scored key phrase extracted from the text.
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct ResultItem {
    /// First occurrence (words joined by space, approximate).
    pub raw: String,
    /// Lowercased key phrase.
    pub keyword: String,
    /// Importance score (lower = more important).
    pub score: f64,
}

impl From<Candidate<'_>> for ResultItem {
    fn from(c: Candidate) -> Self {
        ResultItem {
            raw: c.raw.join(" "),
            keyword: c.lc_terms.join(" "),
            score: c.score,
        }
    }
}

pub(crate) fn remove_duplicates(threshold: f64, results: Vec<ResultItem>, n: usize) -> Vec<ResultItem> {
    let mut unique: Vec<ResultItem> = Vec::with_capacity(n.min(results.len()));

    for res in results {
        if unique.len() >= n {
            break;
        }
        let is_duplicate = unique
            .iter()
            .any(|it| levenshtein_ratio(&it.keyword, &res.keyword) > threshold);
        if !is_duplicate {
            unique.push(res);
        }
    }

    unique
}

/// Levenshtein ratio: 0.0 = completely different, 1.0 = identical.
fn levenshtein_ratio(a: &str, b: &str) -> f64 {
    let dist = levenshtein_distance(a, b);
    let len = a.len().max(b.len());
    if len == 0 {
        return 1.0;
    }
    1.0 - (dist as f64 / len as f64)
}

/// Compute Levenshtein edit distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let m = a.len();
    let n = b.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Single-row DP
    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row = vec![0usize; n + 1];

    for i in 1..=m {
        curr_row[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1).min(curr_row[j - 1] + 1).min(prev_row[j - 1] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_one_edit() {
        assert_eq!(levenshtein_distance("kitten", "sitten"), 1);
    }

    #[test]
    fn levenshtein_multiple() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn ratio_identical() {
        assert!((levenshtein_ratio("test", "test") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ratio_empty() {
        assert!((levenshtein_ratio("", "") - 1.0).abs() < f64::EPSILON);
    }
}
