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
use crate::pdf::native::text::{needs_numeric_script_boundary, numeric_notes};
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

fn collect_suffix_script_owners(spans: &[TextSpan]) -> Vec<NativeScriptOwner> {
    let notes = numeric_notes(spans);
    let supported_bases: Vec<bool> = spans
        .iter()
        .map(|span| span.text.is_ascii() && span.text.bytes().any(|character| character.is_ascii_alphanumeric()))
        .collect();
    let mut snapshots = HashMap::new();
    script_row_candidates(spans)
        .into_iter()
        .filter_map(|candidate| {
            let base_index = candidate.base_index();
            let script_index = candidate.script_index();
            let base = &spans[base_index];
            let script = &spans[script_index];
            let script_byte_range = suffix_script_range(base, script, supported_bases[base_index])?;
            if is_numeric_note_reference(base, script, &notes, spans.get(script_index + 1)) {
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
    following: Option<&TextSpan>,
) -> bool {
    let text = script.text.trim_end();
    if !notes.contains_key(text) {
        return false;
    }
    if text.len() == script.text.len() {
        return needs_numeric_script_boundary(base, script, notes, following);
    }
    // A comparison view lets the existing note lookup see the marker number;
    // witnesses retain the complete source text and every original glyph field. ~keep
    let comparison = TextSpan {
        text: text.to_owned(),
        ..script.clone()
    };
    needs_numeric_script_boundary(base, &comparison, notes, following)
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
mod tests;
