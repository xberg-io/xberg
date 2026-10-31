//! Bounded, complete raised runs; an accepted prefix cannot stand in for a run.

use std::collections::HashMap;

use xberg_native_pdf::extractors::text::ScriptRowCandidate;

use super::super::INLINE_SCRIPT_SAME_BASELINE_TOLERANCE_EM;
use super::*;

pub(super) struct CohortIndex {
    starts: Vec<(f64, usize)>,
    by_base: HashMap<usize, Vec<usize>>,
    base_for: HashMap<usize, usize>,
}

impl CohortIndex {
    pub(super) fn new(spans: &[TextSpan], candidates: &[ScriptRowCandidate]) -> Self {
        let mut starts: Vec<_> = spans
            .iter()
            .enumerate()
            .filter(|(_, span)| valid_continuation_geometry(span))
            .map(|(index, span)| (f64::from(span.bbox.x), index))
            .collect();
        starts.sort_unstable_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        let mut by_base: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut base_for = HashMap::new();
        for candidate in candidates {
            by_base
                .entry(candidate.base_index())
                .or_default()
                .push(candidate.script_index());
            base_for.insert(candidate.script_index(), candidate.base_index());
        }
        Self {
            starts,
            by_base,
            base_for,
        }
    }

    pub(super) fn complete_run(&self, spans: &[TextSpan], base: usize, supported: bool) -> Option<Vec<usize>> {
        let candidates = self.by_base.get(&base)?;
        if candidates.len() > MAX_CONTINUATION_PROBES {
            return None;
        }
        let mut raised: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|&index| raised_member_geometry(&spans[base], &spans[index]))
            .collect();
        raised.sort_by(|&left, &right| spans[left].bbox.x.total_cmp(&spans[right].bbox.x));
        let first = *raised.first()?;
        suffix_script_range(&spans[base], &spans[first], supported)?;
        let mut members = vec![first];
        while let Some(next) = self.next_member(spans, base, first, *members.last()?)? {
            members.push(next);
            if members.len() > INLINE_SCRIPT_MAX_CHARS {
                return None;
            }
        }
        if raised.iter().any(|index| !members.contains(index)) || !valid_run_text(spans, &members) {
            return None;
        }
        Some(members)
    }

    fn next_member(&self, spans: &[TextSpan], base: usize, first: usize, current: usize) -> Option<Option<usize>> {
        let previous = &spans[current];
        let precision = f64::from(f32::EPSILON) * f64::from(previous.bbox.x).abs().max(1.0);
        let left = f64::from(previous.bbox.x) - precision;
        let right = f64::from(previous.bbox.right())
            + f64::from(spans[base].font_size) * f64::from(INLINE_SCRIPT_MAX_SUFFIX_GAP_EM);
        let begin = self.starts.partition_point(|entry| entry.0 < left);
        let end = self.starts.partition_point(|entry| entry.0 <= right);
        if end - begin > MAX_CONTINUATION_PROBES {
            return None;
        }
        let mut next = None;
        for &(x, index) in &self.starts[begin..end] {
            if index == current || index == base || !raised_member_geometry(&spans[base], &spans[index]) {
                continue;
            }
            if x <= f64::from(previous.bbox.x) + precision
                || !same_raised_row(&spans[base], &spans[first], &spans[index])
                || self.base_for.get(&index) != Some(&base)
                || next.is_some()
            {
                return None;
            }
            next = Some(index);
        }
        Some(next)
    }
}

fn same_raised_row(base: &TextSpan, first: &TextSpan, next: &TextSpan) -> bool {
    owner_roles_are_compatible(base, next)
        && (effective_baseline(first).unwrap_or(f32::NAN) - effective_baseline(next).unwrap_or(f32::NAN)).abs()
            <= base.font_size * INLINE_SCRIPT_SAME_BASELINE_TOLERANCE_EM
        && (first.font_size - next.font_size).abs() <= base.font_size * INLINE_SCRIPT_SAME_BASELINE_TOLERANCE_EM
}

fn valid_run_text(spans: &[TextSpan], members: &[usize]) -> bool {
    let mut text = String::new();
    for (position, &index) in members.iter().enumerate() {
        let source = &spans[index].text;
        if position + 1 != members.len() && source.ends_with(char::is_whitespace) {
            return false;
        }
        let trimmed = source.trim_end();
        if text.chars().count() + trimmed.chars().take(INLINE_SCRIPT_MAX_CHARS + 1).count() > INLINE_SCRIPT_MAX_CHARS {
            return false;
        }
        text.push_str(trimmed);
    }
    let mut operand = false;
    for (position, character) in text.chars().enumerate() {
        if character.is_ascii_alphanumeric() {
            operand = true;
        } else if matches!(character, '+' | '-' | '−' | '/' | '×' | '÷') && operand {
            operand = false;
        } else if position != 0 || !matches!(character, '+' | '-' | '−') {
            return false;
        }
    }
    operand
}
