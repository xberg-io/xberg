//! Whole-span witnesses for a literal prime drawn between two adjacent operands.

use super::{ROW_CANDIDATE_MAX_SHIFT_EM, ScriptOwner, SegmentKey, baseline, same_context};
use crate::pdf::hierarchy::SegmentData;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use xberg_native_pdf::{fonts::MappingProvenance, layout::TextSpan, structure::McidScope};

pub(in crate::pdf::native::hierarchy) struct LiteralPrimeWitness([TextSpan; 3]);

fn unique_sequences(spans: &[TextSpan]) -> HashMap<usize, Option<usize>> {
    let mut sequences = HashMap::with_capacity(spans.len());
    for (index, span) in spans.iter().enumerate() {
        sequences
            .entry(span.sequence)
            .and_modify(|value| *value = None)
            .or_insert(Some(index));
    }
    sequences
}

fn unique_index(sequences: &HashMap<usize, Option<usize>>, sequence: usize) -> Option<usize> {
    sequences.get(&sequence).copied().flatten()
}

fn same_prime_context(left: &TextSpan, right: &TextSpan) -> bool {
    same_context(left, right)
        && matches!(left.mcid_scope, Some(McidScope::Page(_)))
        && left.mcid == right.mcid
        && left.rotation_degrees == 0.0
        && right.rotation_degrees == 0.0
        && left.page_rotation_applied == 0
        && !left.split_boundary_before
        && !right.split_boundary_before
        && !left.offset_semantic
        && !right.offset_semantic
        && !left.rtl_draw_logical
        && !right.rtl_draw_logical
        && left.font_weight == right.font_weight
        && left.is_italic == right.is_italic
        && left.color == right.color
}

fn is_literal_prime_triple(left: &TextSpan, mark: &TextSpan, right: &TextSpan) -> bool {
    if !same_prime_context(left, mark) || !same_prime_context(left, right) {
        return false;
    }
    let gap = 0.0..=mark.bbox.width;
    let shift = baseline(mark) - baseline(left);
    let ratio = mark.font_size / left.font_size;
    mark.provenance == Some(MappingProvenance::ToUnicode)
        && left.text.chars().all(|character| character.is_ascii_alphabetic())
        && !left.text.is_empty()
        && right
            .text
            .starts_with(|character: char| character.is_ascii_alphabetic() || character == ')')
        && left.font_size.to_bits() == right.font_size.to_bits()
        && baseline(left).to_bits() == baseline(right).to_bits()
        && gap.contains(&(mark.bbox.x - left.bbox.right()))
        && gap.contains(&(right.bbox.x - mark.bbox.right()))
        && (super::super::INLINE_SCRIPT_MIN_FONT_RATIO..=super::super::INLINE_SCRIPT_MAX_FONT_RATIO).contains(&ratio)
        && shift > left.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
        && shift <= left.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
}

pub(in crate::pdf::native::hierarchy) fn capture_literal_primes(spans: &[TextSpan]) -> Vec<LiteralPrimeWitness> {
    if !spans.iter().any(|span| span.text == "′") {
        return Vec::new();
    }
    let sequences = unique_sequences(spans);
    spans
        .iter()
        .filter(|mark| mark.text == "′")
        .filter_map(|mark| {
            unique_index(&sequences, mark.sequence)?;
            let left = &spans[unique_index(&sequences, mark.sequence.checked_sub(1)?)?];
            let right = &spans[unique_index(&sequences, mark.sequence.checked_add(1)?)?];
            is_literal_prime_triple(left, mark, right)
                .then(|| LiteralPrimeWitness([left.clone(), mark.clone(), right.clone()]))
        })
        .collect()
}

fn unchanged_endpoint(source: &TextSpan, retained: &TextSpan) -> bool {
    same_prime_context(source, retained)
        && source.text == retained.text
        && source.font_name == retained.font_name
        && source.provenance == retained.provenance
        && source.primary_detected == retained.primary_detected
        && [
            source.bbox.x,
            source.bbox.y,
            source.bbox.width,
            source.bbox.height,
            source.font_size,
            source.text_rise,
            source.char_spacing,
            source.word_spacing,
            source.horizontal_scaling,
        ]
        .map(f32::to_bits)
            == [
                retained.bbox.x,
                retained.bbox.y,
                retained.bbox.width,
                retained.bbox.height,
                retained.font_size,
                retained.text_rise,
                retained.char_spacing,
                retained.word_spacing,
                retained.horizontal_scaling,
            ]
            .map(f32::to_bits)
        && [
            (&source.char_widths, &retained.char_widths),
            (&source.char_x_offsets, &retained.char_x_offsets),
        ]
        .into_iter()
        .all(|(before, after)| {
            before
                .iter()
                .map(|value| value.to_bits())
                .eq(after.iter().map(|value| value.to_bits()))
        })
}

pub(super) fn bind_literal_primes(
    witnesses: Vec<LiteralPrimeWitness>,
    spans: &[TextSpan],
    projected: &[SegmentData],
    scripts: &[ScriptOwner],
) -> Vec<ScriptOwner> {
    if witnesses.is_empty() {
        return Vec::new();
    }
    let sequences = unique_sequences(spans);
    let claimed: HashSet<_> = scripts
        .iter()
        .flat_map(|owner| [owner.base_ordinal, owner.script_ordinal])
        .collect();
    let bind = |source: &TextSpan| {
        let index = unique_index(&sequences, source.sequence)?;
        (!claimed.contains(&index) && unchanged_endpoint(source, &spans[index])).then_some(index)
    };
    witnesses
        .into_iter()
        .filter_map(|LiteralPrimeWitness([left, mark, right])| {
            let base = bind(&left)?;
            let script = bind(&mark)?;
            let right = bind(&right)?;
            let role = projected[base].assigned_role;
            if projected[script].assigned_role != role || projected[right].assigned_role != role {
                return None;
            }
            Some(ScriptOwner {
                base_ordinal: base,
                script_ordinal: script,
                base_source: Arc::new(left),
                script_source: Arc::new(mark),
                base: Arc::new(projected[base].clone()),
                script: Arc::new(projected[script].clone()),
                run: Arc::new(vec![SegmentKey::from(&projected[script])]),
                literal_prime_right: Some(SegmentKey::from(&projected[right])),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::native::hierarchy::{ArtifactRetention, script_evidence::NativePageSegments};
    use xberg_native_pdf::geometry::Rect;

    fn prime_triple() -> Vec<TextSpan> {
        [
            ("′", 121.0, 704.5, 2.0, 8.0, 12),
            ("RP", 100.0, 700.0, 20.0, 12.0, 11),
            (")", 124.0, 700.0, 4.0, 12.0, 13),
        ]
        .map(|(text, x, y, width, size, sequence)| TextSpan {
            text: text.to_owned(),
            bbox: Rect::new(x, y, width, size),
            font_size: size,
            sequence,
            mcid_scope: Some(McidScope::Page(0)),
            provenance: Some(MappingProvenance::ToUnicode),
            ..Default::default()
        })
        .to_vec()
    }

    #[test]
    fn should_require_a_unique_consecutive_two_sided_prime_witness() {
        let valid = prime_triple();
        assert_eq!(capture_literal_primes(&valid).len(), 1);

        let mut no_right = valid.clone();
        no_right.pop();
        assert!(capture_literal_primes(&no_right).is_empty());

        let mut duplicate = valid.clone();
        let mut extra = valid[2].clone();
        extra.bbox.x += 1.0;
        duplicate.push(extra);
        assert!(capture_literal_primes(&duplicate).is_empty());

        let mut distant = valid;
        distant[2].bbox.x += 6.0;
        assert!(capture_literal_primes(&distant).is_empty());
    }

    #[test]
    fn should_reject_a_prime_in_a_different_or_rotated_frame() {
        let mut different = prime_triple();
        different[2].mcid_scope = Some(McidScope::Page(1));
        assert!(capture_literal_primes(&different).is_empty());

        let mut rotated = prime_triple();
        for span in &mut rotated {
            span.rotation_degrees = 90.0;
        }
        assert!(capture_literal_primes(&rotated).is_empty());
    }

    #[test]
    fn should_reject_rebound_prime_after_any_whole_endpoint_changes() {
        let original = prime_triple();
        for endpoint in 0..original.len() {
            let mut changed = original.clone();
            changed[endpoint].bbox.width += 0.25;
            let page = NativePageSegments::from_final_spans_with_primes(
                changed,
                &HashMap::new(),
                ArtifactRetention::default(),
                capture_literal_primes(&original),
            );
            assert!(page.valid_owners(&page.segments).is_empty());
            assert_eq!(page.segments.len(), original.len());
        }
    }
}
