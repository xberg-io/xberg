//! Source-coordinate selection for a sparse/automatic page segmentation pair.

#[cfg(test)]
use std::ops::Range;

#[cfg(test)]
const MAX_WORDS: usize = 6_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Word<'a> {
    pub(super) text: &'a str,
    pub(super) bounds: [u32; 4],
    pub(super) confidence: f64,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Selection {
    pub(super) substitutions: Vec<(usize, usize)>,
    pub(super) detached_lines: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)>,
}

/// Plan changes by source occurrence. Line ranges must describe complete baseline
/// lines; a consumer must still verify ownership and line boundaries in the output.
#[cfg(test)]
pub(super) fn plan(baseline: &[Word<'_>], candidate: &[Word<'_>], lines: &[Range<usize>]) -> Selection {
    let mut selection = Selection {
        substitutions: Vec::new(),
        detached_lines: Vec::new(),
    };
    if baseline.is_empty() || baseline.len() > MAX_WORDS || candidate.len() > MAX_WORDS {
        return selection;
    }
    if baseline
        .iter()
        .chain(candidate)
        .any(|w| w.bounds[0] >= w.bounds[2] || w.bounds[1] >= w.bounds[3])
    {
        return selection;
    }
    let height = median_height(baseline);
    let partners = unique_partners(baseline, candidate);
    for (i, window) in partners.windows(3).enumerate() {
        if let [Some(left), Some(center), Some(right)] = window
            && *left + 1 == *center
            && *center + 1 == *right
            && same_line(baseline[i], baseline[i + 1], height)
            && same_line(baseline[i + 1], baseline[i + 2], height)
            && prefer_baseline(baseline[i + 1], candidate[*center])
        {
            selection.substitutions.push((i + 1, *center));
        }
    }
    selection.detached_lines = lines
        .iter()
        .take(baseline.len())
        .filter_map(|line| detached_line(line.clone(), baseline, candidate, &partners, height))
        .collect();
    selection
}

pub(super) fn median_height(baseline: &[Word<'_>]) -> f64 {
    let mut heights: Vec<_> = baseline.iter().map(|word| word.bounds[3] - word.bounds[1]).collect();
    let middle = heights.len() / 2;
    heights.select_nth_unstable(middle);
    if heights.len() % 2 == 0 {
        (f64::from(heights[middle]) + f64::from(*heights[..middle].iter().max().unwrap_or(&0))) / 2.0
    } else {
        f64::from(heights[middle])
    }
}

pub(super) fn unique_partners(baseline: &[Word<'_>], candidate: &[Word<'_>]) -> Vec<Option<usize>> {
    let mut incoming = vec![0u8; candidate.len()];
    let mut outgoing = vec![None; baseline.len()];
    for (i, old) in baseline.iter().enumerate() {
        let mut count = 0u8;
        for (j, new) in candidate.iter().enumerate() {
            if majority_overlap(old.bounds, new.bounds) {
                incoming[j] = incoming[j].saturating_add(1);
                count = count.saturating_add(1);
                outgoing[i] = Some(j);
            }
        }
        if count != 1 {
            outgoing[i] = None;
        }
    }
    outgoing
        .iter_mut()
        .for_each(|partner| *partner = partner.filter(|&j| incoming[j] == 1));
    outgoing
}

fn majority_overlap(a: [u32; 4], b: [u32; 4]) -> bool {
    let overlap = u64::from(a[2].min(b[2]).saturating_sub(a[0].max(b[0])))
        * u64::from(a[3].min(b[3]).saturating_sub(a[1].max(b[1])));
    let area = |x: [u32; 4]| u64::from(x[2] - x[0]) * u64::from(x[3] - x[1]);
    overlap > area(a).min(area(b)) / 2
}

#[cfg(test)]
pub(super) fn same_line(left: Word<'_>, right: Word<'_>, height: f64) -> bool {
    let a = left.bounds;
    let b = right.bounds;
    let overlap = a[3].min(b[3]).saturating_sub(a[1].max(b[1]));
    b[0].checked_sub(a[2]).is_some_and(|gap| f64::from(gap) <= 1.5 * height)
        && overlap > (a[3] - a[1]).min(b[3] - b[1]) / 2
}

#[cfg(test)]
fn prefer_baseline(old: Word<'_>, new: Word<'_>) -> bool {
    if old.text == new.text {
        return false;
    }
    let alphanumeric = |text: &str| -> String {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    if alphanumeric(old.text) == alphanumeric(new.text) {
        old.text.chars().count() == new.text.chars().count()
    } else {
        old.confidence > new.confidence
    }
}

#[cfg(test)]
fn detached_line(
    line: Range<usize>,
    baseline: &[Word<'_>],
    candidate: &[Word<'_>],
    partners: &[Option<usize>],
    height: f64,
) -> Option<(Range<usize>, Range<usize>)> {
    let source = baseline.get(line.clone())?;
    if source.len() < 4 || !source.windows(2).all(|pair| same_line(pair[0], pair[1], height)) {
        return None;
    }
    let mapped = partners.get(line)?.iter().copied().collect::<Option<Vec<_>>>()?;
    if !source
        .iter()
        .zip(&mapped)
        .all(|(word, &index)| word.text == candidate[index].text)
    {
        return None;
    }
    let mut splits = mapped.windows(2).enumerate().filter(|(_, pair)| pair[0] + 1 != pair[1]);
    let (cut, gap) = splits.next()?;
    let split = cut + 1;
    if splits.next().is_some() || split < 2 || mapped.len() - split < 2 || gap[0] >= gap[1] {
        return None;
    }
    Some((mapped[0]..gap[0] + 1, gap[1]..mapped.last()? + 1))
}

#[cfg(test)]
#[path = "column_selection_tests.rs"]
mod tests;
