//! Suffix ownership evidence over final preserved native spans.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use xberg_native_pdf::extractors::text::script_row_candidates;
use xberg_native_pdf::layout::TextSpan;

use super::{
    INLINE_SCRIPT_MAX_CHARS, INLINE_SCRIPT_MAX_FONT_RATIO, INLINE_SCRIPT_MAX_SUFFIX_GAP_EM,
    INLINE_SCRIPT_MIN_BASELINE_SHIFT_EM, INLINE_SCRIPT_MIN_FONT_RATIO,
};
use crate::pdf::native::span_geometry::{has_same_rotation, is_horizontal_ltr};
use crate::pdf::native::text::numeric_notes;
use crate::types::document_structure::AnnotationKind;

#[derive(Debug, Clone)]
struct NativeSpanWitness {
    original_index: usize,
    snapshot: Arc<TextSpan>,
}

#[derive(Debug, Clone)]
struct NativeScriptOwner {
    base_witness: NativeSpanWitness,
    script_witness: NativeSpanWitness,
    logical_baseline: f32,
    kind: AnnotationKind,
    script_byte_range: Range<usize>,
}

// Keep explicit text rise inside the native row candidate's half-em domain. ~keep
const ROW_CANDIDATE_MAX_SHIFT_EM: f32 = 0.5;
const MAX_CONTINUATION_PROBES: usize = 64;

struct BaseFacts {
    supported: bool,
    numbered_prose: bool,
}

impl BaseFacts {
    fn from_span(span: &TextSpan) -> Self {
        let numbered_prose = !span.text.ends_with(char::is_whitespace)
            && span
                .text
                .trim_end()
                .rsplit_once(char::is_whitespace)
                .is_some_and(|(prose, number)| {
                    prose.chars().any(char::is_alphabetic) && number.bytes().all(|character| character.is_ascii_digit())
                });
        Self {
            supported: span.text.is_ascii() && span.text.bytes().any(|character| character.is_ascii_alphanumeric()),
            numbered_prose,
        }
    }
}

struct ContinuationIndex {
    starts: Vec<(f64, usize)>,
}

impl ContinuationIndex {
    fn new(spans: &[TextSpan]) -> Self {
        let mut starts: Vec<_> = spans
            .iter()
            .enumerate()
            .filter(|(_, span)| {
                span.text.trim_start().starts_with(['+', '-', '−', '×', '÷', '=', '^'])
                    && valid_continuation_geometry(span)
            })
            .map(|(index, span)| (f64::from(span.bbox.x), index))
            .collect();
        starts.sort_unstable_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        Self { starts }
    }

    fn has_unique_operator(&self, spans: &[TextSpan], base: &TextSpan, script: &TextSpan) -> bool {
        let Some(base_baseline) = effective_baseline(base) else {
            return false;
        };
        let right = f64::from(script.bbox.right());
        let radius = f64::from(base.font_size);
        let precision = f64::from(f32::EPSILON) * right.abs().max(1.0);
        let start = self.starts.partition_point(|entry| entry.0 < right - precision);
        let end = self.starts.partition_point(|entry| entry.0 <= right + radius);
        // An incomplete neighborhood cannot establish a unique continuation. ~keep
        if end - start > MAX_CONTINUATION_PROBES {
            return false;
        }
        // Rank compatible operators by distance from the script's right edge;
        // source-coordinate precision ties cannot prove a unique expression. ~keep
        let mut best = f64::INFINITY;
        let mut best_x = right;
        let mut tied = false;
        for &(x, index) in &self.starts[start..end] {
            let next = &spans[index];
            let Some(next_baseline) = effective_baseline(next) else {
                continue;
            };
            if std::ptr::eq(next, base)
                || std::ptr::eq(next, script)
                || !owner_roles_are_compatible(base, next)
                || (next_baseline - base_baseline).abs() > base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
            {
                continue;
            }
            let distance = (x - right).abs();
            let pair_precision = f64::from(f32::EPSILON) * right.abs().max(x.abs()).max(best_x.abs()).max(1.0);
            if distance + pair_precision < best {
                best = distance;
                best_x = x;
                tied = false;
            } else if (distance - best).abs() <= pair_precision {
                tied = true;
            }
        }
        best.is_finite() && !tied
    }
}

fn valid_continuation_geometry(span: &TextSpan) -> bool {
    [
        span.bbox.x,
        span.bbox.y,
        span.bbox.width,
        span.bbox.height,
        span.bbox.right(),
        span.bbox.bottom(),
        span.font_size,
        span.text_rise,
        span.rotation_degrees,
    ]
    .into_iter()
    .all(f32::is_finite)
        && span.bbox.width > 0.0
        && span.bbox.height > 0.0
        && span.font_size > 0.0
        && !span.mirrored
        && effective_baseline(span).is_some()
}

fn collect_suffix_script_owners(spans: &[TextSpan]) -> Vec<NativeScriptOwner> {
    let notes = numeric_notes(spans);
    let base_facts: Vec<_> = spans.iter().map(BaseFacts::from_span).collect();
    let continuations = ContinuationIndex::new(spans);
    let mut snapshots = HashMap::new();
    script_row_candidates(spans)
        .into_iter()
        .filter_map(|candidate| {
            let base_index = candidate.base_index();
            let script_index = candidate.script_index();
            let base = &spans[base_index];
            let script = &spans[script_index];
            let script_byte_range = suffix_script_range(base, script, base_facts[base_index].supported)?;
            if is_numeric_note_reference(base, script, &notes, &base_facts[base_index])
                && !continuations.has_unique_operator(spans, base, script)
            {
                return None;
            }
            Some(NativeScriptOwner {
                base_witness: span_witness(spans, base_index, &mut snapshots),
                script_witness: span_witness(spans, script_index, &mut snapshots),
                logical_baseline: effective_baseline(base)?,
                kind: AnnotationKind::Superscript,
                script_byte_range,
            })
        })
        .collect()
}

fn owner_roles_are_compatible(base: &TextSpan, script: &TextSpan) -> bool {
    base.artifact_type.is_none()
        && script.artifact_type.is_none()
        && !base.is_monospace
        && !script.is_monospace
        && is_horizontal_ltr(base)
        && is_horizontal_ltr(script)
        && has_same_rotation(base, script)
        && base.heading_level == script.heading_level
        && base.mcid_scope == script.mcid_scope
        && base.page_rotation_applied == script.page_rotation_applied
}

fn effective_baseline(span: &TextSpan) -> Option<f32> {
    let baseline = span.bbox.y + span.text_rise * span.font_size;
    baseline.is_finite().then_some(baseline)
}

fn suffix_script_range(base: &TextSpan, script: &TextSpan, supported_base: bool) -> Option<Range<usize>> {
    let text = script.text.trim_end();
    if !owner_roles_are_compatible(base, script)
        || !supported_base
        || text.is_empty()
        || text.len() > INLINE_SCRIPT_MAX_CHARS
        || !text.bytes().all(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    let font_ratio = script.font_size / base.font_size;
    if !(INLINE_SCRIPT_MIN_FONT_RATIO..=INLINE_SCRIPT_MAX_FONT_RATIO).contains(&font_ratio) {
        return None;
    }
    let shift = effective_baseline(script)? - effective_baseline(base)?;
    if !(base.font_size * INLINE_SCRIPT_MIN_BASELINE_SHIFT_EM..=base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM)
        .contains(&shift)
    {
        return None;
    }
    let base_right = f64::from(base.bbox.right());
    let script_x = f64::from(script.bbox.x);
    let gap = script_x - base_right;
    let precision = f64::from(f32::EPSILON) * base_right.abs().max(script_x.abs()).max(1.0);
    if gap < -precision || gap > f64::from(base.font_size * INLINE_SCRIPT_MAX_SUFFIX_GAP_EM) {
        return None;
    }
    Some(0..text.len())
}

fn is_numeric_note_reference(
    base: &TextSpan,
    script: &TextSpan,
    notes: &HashMap<&str, &TextSpan>,
    facts: &BaseFacts,
) -> bool {
    let Some(note) = notes.get(script.text.trim_end()) else {
        return false;
    };
    if !facts.numbered_prose
        || note.font_size >= base.font_size * 0.9
        || base.bbox.y - note.bbox.y < base.font_size * 1.5
        || (script.font_size >= base.font_size * 0.8 && script.text_rise.abs() < 0.10)
    {
        return false;
    }
    let gap = script.bbox.x - base.bbox.right();
    gap >= -0.1 * base.font_size
        && gap <= 0.25 * base.font_size
        && (base.bbox.y - script.bbox.y).abs() <= base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
}

fn span_witness(spans: &[TextSpan], index: usize, snapshots: &mut HashMap<usize, Arc<TextSpan>>) -> NativeSpanWitness {
    // Clone a source span once per ordinal, even when several powers share it.
    // Reused stream sequence IDs and identical text are not witness identity. ~keep
    let snapshot = snapshots.entry(index).or_insert_with(|| Arc::new(spans[index].clone()));
    NativeSpanWitness {
        original_index: index,
        snapshot: Arc::clone(snapshot),
    }
}

#[cfg(test)]
mod page_segments;

#[cfg(test)]
mod tests;
