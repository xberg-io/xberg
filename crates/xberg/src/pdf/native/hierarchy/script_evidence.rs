//! Private evidence over final preserved spans, with page-local ordinals rather than PDF identities.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use xberg_native_pdf::{fonts::MappingProvenance, layout::TextSpan};

use super::{ArtifactRetention, ScriptGeometry};
use crate::pdf::hierarchy::SegmentData;

mod decimal_power;
mod literal_prime;
pub(super) use literal_prime::{LiteralPrimeWitness, capture_literal_primes};

// The native SnapBaseSearch already admits `bfs * 0.5`. This private lane uses
// that domain; the shared core 0.35em script predicate remains unchanged. ~keep
const ROW_CANDIDATE_MAX_SHIFT_EM: f32 = 0.5;
const MAX_CANDIDATE_EDGES: usize = 64;
const NUMERIC_SUFFIX_OVERHANG_EM: f32 = 0.005;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SegmentKey {
    text: String,
    geometry: [u32; 7],
    style: [bool; 3],
    role: Option<u8>,
}

impl SegmentKey {
    pub(crate) fn is_single_glyph(&self) -> bool {
        let mut chars = self.text.chars();
        chars.next().is_some_and(|glyph| !glyph.is_whitespace()) && chars.next().is_none()
    }
}

impl From<&SegmentData> for SegmentKey {
    fn from(segment: &SegmentData) -> Self {
        Self {
            text: segment.text.clone(),
            geometry: [
                segment.x,
                segment.y,
                segment.width,
                segment.height,
                segment.font_size,
                segment.baseline_y,
                segment.rotation_degrees,
            ]
            .map(f32::to_bits),
            style: [segment.is_bold, segment.is_italic, segment.is_monospace],
            role: segment.assigned_role,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptOwner {
    pub(crate) base_ordinal: usize,
    pub(crate) script_ordinal: usize,
    base_source: Arc<TextSpan>,
    script_source: Arc<TextSpan>,
    pub(crate) base: Arc<SegmentData>,
    pub(crate) script: Arc<SegmentData>,
    run: Arc<Vec<SegmentKey>>,
    literal_prime_right: Option<SegmentKey>,
}

impl ScriptOwner {
    pub(crate) fn run_members(&self) -> &[SegmentKey] {
        &self.run
    }

    pub(crate) fn literal_prime_right(&self) -> Option<&SegmentKey> {
        self.literal_prime_right.as_ref()
    }

    pub(crate) fn matches_script(&self, segment: &SegmentData) -> bool {
        SegmentKey::from(segment) == SegmentKey::from(self.script.as_ref())
    }

    pub(crate) fn follows(&self, previous: &SegmentData, next: &SegmentData) -> bool {
        self.matches_script(next)
            && (SegmentKey::from(previous) == SegmentKey::from(self.base.as_ref())
                || self.run.contains(&SegmentKey::from(previous)))
    }

    pub(crate) fn survives(&self, segments: &[SegmentData]) -> bool {
        self.survives_inventory(&projection_counts(segments))
    }

    fn survives_inventory(&self, counts: &HashMap<SegmentKey, usize>) -> bool {
        let source_projection_matches = |source: &TextSpan, projected: &SegmentData| {
            source.text == projected.text
                && source.bbox.x.to_bits() == projected.x.to_bits()
                && (source.bbox.y + source.text_rise * source.font_size).to_bits() == projected.baseline_y.to_bits()
        };
        source_projection_matches(&self.base_source, &self.base)
            && source_projection_matches(&self.script_source, &self.script)
            && counts.get(&SegmentKey::from(self.base.as_ref())) == Some(&1)
            && self.run.iter().all(|key| counts.get(key) == Some(&1))
            && self
                .literal_prime_right
                .as_ref()
                .is_none_or(|key| counts.get(key) == Some(&1))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NativePageSegments {
    pub(crate) segments: Vec<SegmentData>,
    owners: Vec<ScriptOwner>,
    prose_rows: Vec<Vec<SegmentKey>>,
    #[cfg(feature = "formula-recognition")]
    pub(crate) inline_math_glyphs: Vec<SegmentKey>,
}

impl NativePageSegments {
    #[cfg(test)]
    pub(crate) fn from_final_spans(
        spans: Vec<TextSpan>,
        roles: &HashMap<u32, Option<u8>>,
        artifacts: ArtifactRetention,
    ) -> Self {
        let primes = capture_literal_primes(&spans);
        Self::from_final_spans_with_primes(spans, roles, artifacts, primes, None)
    }

    fn from_final_spans_with_primes(
        spans: Vec<TextSpan>,
        roles: &HashMap<u32, Option<u8>>,
        artifacts: ArtifactRetention,
        prime_witnesses: Vec<LiteralPrimeWitness>,
        decimal_authority: Option<decimal_power::Authority>,
    ) -> Self {
        let (spans, projected, owners) = script_projection(spans, roles, prime_witnesses, decimal_authority);
        let segments = super::dedupe_redrawn_segments(
            spans
                .iter()
                .zip(&projected)
                .filter(|(span, _)| {
                    !span.text.trim().is_empty()
                        && span
                            .artifact_type
                            .as_ref()
                            .is_none_or(|artifact| artifacts.keeps(artifact))
                })
                .map(|(_, segment)| segment.clone())
                .collect(),
        );
        let source_counts = projection_counts(&projected);
        let retained_counts = projection_counts(&segments);
        let owners = owners
            .into_iter()
            .filter(|owner| owner.survives_inventory(&source_counts) && owner.survives_inventory(&retained_counts))
            .collect();
        let prose_rows = certified_prose_row_keys(&spans, &projected);
        Self {
            #[cfg(feature = "formula-recognition")]
            inline_math_glyphs: Vec::new(),
            segments,
            owners,
            prose_rows,
        }
    }

    pub(crate) fn clear_prose_rows(&mut self) {
        self.prose_rows.clear();
    }

    pub(crate) fn surviving_prose_rows(&self, segments: &[SegmentData]) -> Option<Vec<Vec<SegmentData>>> {
        let expected = self.prose_rows.iter().flatten().count();
        if self.prose_rows.is_empty() || segments.len() != expected {
            return None;
        }
        let expected_inventory: HashMap<_, _> = self
            .prose_rows
            .iter()
            .flatten()
            .map(|key| (key.clone(), 1usize))
            .collect();
        if expected_inventory.len() != expected || projection_counts(segments) != expected_inventory {
            return None;
        }
        let by_key: HashMap<_, _> = segments
            .iter()
            .map(|segment| (SegmentKey::from(segment), segment))
            .collect();
        self.prose_rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|key| by_key.get(key).map(|segment| (*segment).clone()))
                    .collect()
            })
            .collect()
    }

    pub(crate) fn has_literal_primes(&self) -> bool {
        self.owners.iter().any(|owner| owner.literal_prime_right.is_some())
    }

    pub(crate) fn valid_owners(&self, segments: &[SegmentData]) -> Vec<ScriptOwner> {
        let counts = projection_counts(segments);
        self.owners
            .iter()
            .filter(|owner| owner.survives_inventory(&counts))
            .cloned()
            .collect()
    }

    pub(crate) fn retain_segments(&mut self, keep: impl FnMut(&SegmentData) -> bool) {
        self.segments.retain(keep);
        self.owners = self.valid_owners(&self.segments);
    }
}

fn script_projection(
    mut spans: Vec<TextSpan>,
    roles: &HashMap<u32, Option<u8>>,
    prime_witnesses: Vec<LiteralPrimeWitness>,
    decimal_authority: Option<decimal_power::Authority>,
) -> (Vec<TextSpan>, Vec<SegmentData>, Vec<ScriptOwner>) {
    let project = |spans: &[TextSpan]| {
        spans
            .iter()
            .cloned()
            .map(|span| super::project_final_span(span, roles, ScriptGeometry::Preserve))
            .collect::<Vec<_>>()
    };
    let mut projected = project(&spans);
    let mut owners = collect_owners(&spans, &projected);
    let primes = literal_prime::bind_literal_primes(prime_witnesses, &spans, &projected, &owners);
    owners.extend(primes);
    if let Some(authority) = decimal_authority {
        let runs = decimal_power::prepare_with_authority(&mut spans, &mut owners, || Some(authority));
        if !runs.is_empty() {
            projected = project(&spans);
            decimal_power::bind(runs, &spans, &projected, &mut owners);
        }
    }
    (spans, projected, owners)
}

fn projection_counts(segments: &[SegmentData]) -> HashMap<SegmentKey, usize> {
    let mut counts = HashMap::new();
    for segment in segments {
        *counts.entry(SegmentKey::from(segment)).or_insert(0_usize) += 1;
    }
    counts
}

fn baseline(span: &TextSpan) -> f32 {
    span.bbox.y + span.text_rise * span.font_size
}

fn valid_geometry(span: &TextSpan) -> bool {
    super::has_valid_span_geometry(span)
        && [span.bbox.right(), baseline(span), span.text_rise, span.rotation_degrees]
            .into_iter()
            .all(f32::is_finite)
        && super::is_horizontal_ltr(span)
        && !span.mirrored
        && !span.is_monospace
        && span.artifact_type.is_none()
}

fn same_context(base: &TextSpan, script: &TextSpan) -> bool {
    valid_geometry(base)
        && valid_geometry(script)
        && super::has_same_rotation(base, script)
        && base.heading_level == script.heading_level
        && base.mcid_scope == script.mcid_scope
        && base.page_rotation_applied == script.page_rotation_applied
}

fn compact_marker(span: &TextSpan) -> bool {
    !span.text.is_empty()
        && span.text.chars().take(super::INLINE_SCRIPT_MAX_CHARS + 1).count() <= super::INLINE_SCRIPT_MAX_CHARS
        && span
            .text
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, ',' | '*' | '†' | '‡' | '+' | '-' | '−'))
        && valid_geometry(span)
}

fn is_numeric_span(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn has_adjacent_numeric_source(base: &TextSpan, script: &TextSpan, occupied_sequences: &HashSet<usize>) -> bool {
    let numeric_base = is_numeric_span(&base.text);
    let numeric_suffix = script.text.strip_prefix('−').map_or_else(
        || is_numeric_span(&script.text),
        |digits| numeric_base && (digits.is_empty() || is_numeric_span(digits)),
    );
    let simple_base = numeric_base || (base.text.len() == 1 && base.text.as_bytes()[0].is_ascii_alphabetic());
    let bracket_base = base.text.ends_with([')', ']']) && is_numeric_span(&script.text);
    numeric_suffix
        && (simple_base || bracket_base)
        && (has_adjacent_source(base, script)
            // A separately positioned numeric suffix can start at the next text object. ~keep
            || (simple_base
                && base.sequence.checked_add(2) == Some(script.sequence)
                && !occupied_sequences.contains(&(script.sequence - 1))
                && has_matching_source_evidence(base, script)))
}

fn has_adjacent_ordinal_source(base: &TextSpan, suffix: &TextSpan) -> bool {
    let Some(last) = base.text.bytes().last().filter(u8::is_ascii_digit) else {
        return false;
    };
    if !is_numeric_span(&base.text) || !has_adjacent_source(base, suffix) {
        return false;
    }
    let teen = base.text.bytes().rev().nth(1) == Some(b'1');
    let expected = match (teen, last) {
        (false, b'1') => "st",
        (false, b'2') => "nd",
        (false, b'3') => "rd",
        _ => "th",
    };
    suffix.text == expected
}

fn has_adjacent_source(left: &TextSpan, right: &TextSpan) -> bool {
    // Preserved-geometry extraction leaves gaps at BT and absolute Tm.
    // Relative Td remains eligible for a raised continuation. ~keep
    left.sequence.checked_add(1) == Some(right.sequence) && has_matching_source_evidence(left, right)
}

fn has_matching_source_evidence(left: &TextSpan, right: &TextSpan) -> bool {
    left.mcid_scope.is_some()
        && left.mcid_scope == right.mcid_scope
        && left.mcid == right.mcid
        && [left, right].into_iter().all(|span| {
            span.provenance == Some(MappingProvenance::ToUnicode)
                && !span.split_boundary_before
                && !span.offset_semantic
                // Span splitting can synthesize offsets. Do not use those to admit overlap. ~keep
                && span.char_x_offsets.is_empty()
        })
}

fn eligible_base(base: &TextSpan, script: &TextSpan, occupied_sequences: &HashSet<usize>) -> bool {
    let shift = baseline(script) - baseline(base);
    let ratio = script.font_size / base.font_size;
    let gap = script.bbox.x - base.bbox.right();
    let supported_gap = if gap < 0.0 || script.text.starts_with('−') {
        gap >= -base.font_size * NUMERIC_SUFFIX_OVERHANG_EM
            && (has_adjacent_numeric_source(base, script, occupied_sequences)
                || has_adjacent_ordinal_source(base, script))
    } else {
        true
    };
    same_context(base, script)
        // A closing bracket carries a raised exponent as well, as in `(x + 3y)2`. ~keep
        && (base.text.chars().any(char::is_alphanumeric) || base.text.ends_with([')', ']']))
        && !base.text.ends_with(char::is_whitespace)
        && (super::INLINE_SCRIPT_MIN_FONT_RATIO..=super::INLINE_SCRIPT_MAX_FONT_RATIO).contains(&ratio)
        && shift > base.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
        && shift <= base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
        && supported_gap
        && gap <= base.font_size * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM
}

fn candidate_bases(
    spans: &[TextSpan],
    edges: &[(f32, usize)],
    script: usize,
    max_font: f32,
    occupied_sequences: &HashSet<usize>,
) -> Option<Vec<usize>> {
    let x = spans[script].bbox.x;
    let start = edges.partition_point(|(right, _)| *right < x - max_font * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM);
    let end = edges.partition_point(|(right, _)| *right <= x + max_font * NUMERIC_SUFFIX_OVERHANG_EM);
    if end - start > MAX_CANDIDATE_EDGES {
        return None;
    }
    Some(
        edges[start..end]
            .iter()
            .filter_map(|&(_, base)| {
                (base != script && eligible_base(&spans[base], &spans[script], occupied_sequences)).then_some(base)
            })
            .collect(),
    )
}

fn complete_run(
    spans: &[TextSpan],
    base: usize,
    first: usize,
    occupied_sequences: &HashSet<usize>,
) -> Option<Vec<usize>> {
    let mut row: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| {
            baseline(span).to_bits() == baseline(&spans[first]).to_bits()
                && span.font_size.to_bits() == spans[first].font_size.to_bits()
                && span.bbox.x >= spans[first].bbox.x
        })
        .map(|(index, _)| index)
        .collect();
    row.sort_by(|&left, &right| spans[left].bbox.x.total_cmp(&spans[right].bbox.x));
    let mut members = Vec::new();
    let mut right = spans[first].bbox.x;
    for index in row {
        let span = &spans[index];
        if span.bbox.x - right > spans[base].font_size * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM {
            break;
        }
        if !same_context(&spans[base], span)
            || !compact_marker(span)
            || members.len() == super::INLINE_SCRIPT_LOOKBACK
            || span.bbox.x < right
        {
            return None;
        }
        members.push(index);
        right = span.bbox.right();
    }
    if members.first() != Some(&first) {
        return None;
    }
    let has_overhang = spans[first].bbox.x < spans[base].bbox.right();
    let has_minus = members.iter().any(|&index| spans[index].text.contains('−'));
    let complete_ordinal = members.len() == 1 && has_adjacent_ordinal_source(&spans[base], &spans[first]);
    if (has_overhang || has_minus)
        && !complete_ordinal
        && !is_complete_numeric_suffix(&spans[base], spans, &members, occupied_sequences)
    {
        return None;
    }
    Some(members)
}

fn is_complete_numeric_suffix(
    base: &TextSpan,
    spans: &[TextSpan],
    members: &[usize],
    occupied_sequences: &HashSet<usize>,
) -> bool {
    let Some((&first, rest)) = members.split_first() else {
        return false;
    };
    let first = &spans[first];
    let digits = first.text.strip_prefix('−').unwrap_or(&first.text);
    has_adjacent_numeric_source(base, first, occupied_sequences)
        && (is_numeric_span(digits) || (digits.is_empty() && !rest.is_empty()))
        && rest.iter().all(|&index| is_numeric_span(&spans[index].text))
        && members
            .windows(2)
            .all(|pair| has_adjacent_source(&spans[pair[0]], &spans[pair[1]]))
}

fn has_stacked_counterpart(spans: &[TextSpan], base: usize, members: &[usize]) -> bool {
    let base = &spans[base];
    spans.iter().any(|lower| {
        let shift = baseline(base) - baseline(lower);
        let ratio = lower.font_size / base.font_size;
        // Opposing geometry invalidates the attachment proof even when semantic or style metadata differs. ~keep
        super::has_valid_span_geometry(lower)
            && super::has_same_rotation(base, lower)
            && (super::INLINE_SCRIPT_MIN_FONT_RATIO..=super::INLINE_SCRIPT_MAX_FONT_RATIO).contains(&ratio)
            && shift > 0.0
            && shift <= base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
            && members
                .iter()
                .any(|&member| lower.bbox.x < spans[member].bbox.right() && lower.bbox.right() > spans[member].bbox.x)
    })
}

fn has_unclaimed_cohort_record(spans: &[TextSpan], base: usize, members: &[usize]) -> bool {
    let Some((&first, &last)) = members.first().zip(members.last()) else {
        return true;
    };
    let base = &spans[base];
    let gap = base.font_size * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM;
    let left = spans[first].bbox.x - gap;
    let right = spans[last].bbox.right() + gap;
    // A different font or role cannot hide a missing member, including an overlapping decimal point. ~keep
    spans.iter().enumerate().any(|(index, span)| {
        let shift = baseline(span) - baseline(base);
        !members.contains(&index)
            && super::has_valid_span_geometry(span)
            && super::has_same_rotation(base, span)
            && shift > base.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
            && shift <= base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
            && span.bbox.x <= right
            && span.bbox.right() >= left
    })
}

fn collect_owners(spans: &[TextSpan], projected: &[SegmentData]) -> Vec<ScriptOwner> {
    let mut edges: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| valid_geometry(span))
        .map(|(index, span)| (span.bbox.right(), index))
        .collect();
    edges.sort_by(|left, right| left.0.total_cmp(&right.0));
    let max_font = spans
        .iter()
        .filter(|span| valid_geometry(span))
        .map(|span| span.font_size)
        .fold(0.0, f32::max);
    let mut owners = Vec::new();
    let mut claimed = HashSet::new();
    let occupied_sequences = spans.iter().map(|span| span.sequence).collect();
    let notes = super::super::text::numeric_notes(spans);
    for (first, _) in spans.iter().enumerate().filter(|(_, span)| compact_marker(span)) {
        if claimed.contains(&first) {
            continue;
        }
        let Some(candidates) = candidate_bases(spans, &edges, first, max_font, &occupied_sequences) else {
            continue;
        };
        let [base] = candidates.as_slice() else {
            continue;
        };
        let Some(members) = complete_run(spans, *base, first, &occupied_sequences) else {
            continue;
        };
        if has_stacked_counterpart(spans, *base, &members) || has_unclaimed_cohort_record(spans, *base, &members) {
            continue;
        }
        if members.iter().any(|&member| {
            projected[*base].assigned_role != projected[member].assigned_role
                || super::super::text::needs_numeric_script_boundary(&spans[*base], &spans[member], &notes, None)
                || claimed.contains(&member)
                || candidate_bases(spans, &edges, member, max_font, &occupied_sequences)
                    .is_none_or(|candidates| candidates.iter().any(|candidate| candidate != base))
        }) {
            continue;
        }
        let run = Arc::new(
            members
                .iter()
                .map(|&index| SegmentKey::from(&projected[index]))
                .collect(),
        );
        for index in members {
            claimed.insert(index);
            owners.push(ScriptOwner {
                base_ordinal: *base,
                script_ordinal: index,
                base_source: Arc::new(spans[*base].clone()),
                script_source: Arc::new(spans[index].clone()),
                base: Arc::new(projected[*base].clone()),
                script: Arc::new(projected[index].clone()),
                run: Arc::clone(&run),
                literal_prime_right: None,
            });
        }
    }
    let scripts: std::collections::HashSet<_> = owners.iter().map(|owner| owner.script_ordinal).collect();
    owners.retain(|owner| !scripts.contains(&owner.base_ordinal));
    owners
}

pub(crate) fn extract_all_segment_carriers(
    doc: &mut super::NativeDocument,
    artifacts: ArtifactRetention,
) -> crate::pdf::error::Result<(Vec<NativePageSegments>, bool)> {
    let (pages, used_tree) =
        super::extract_pages_with_structure_tree(doc, artifacts, ScriptGeometry::Preserve, extract_page, |page| {
            page.segments
                .iter()
                .filter(|segment| segment.assigned_role.is_some())
                .count()
        })?;
    if used_tree && !pages.is_empty() {
        return Ok((pages, true));
    }
    let count = doc
        .doc
        .page_count()
        .map_err(|error| crate::pdf::error::PdfError::TextExtractionFailed(error.to_string()))?;
    let pages = (0..count)
        .map(|index| extract_page(doc, index, &HashMap::new(), artifacts, ScriptGeometry::Preserve))
        .collect::<crate::pdf::error::Result<Vec<_>>>()?;
    Ok((pages, false))
}

fn certified_prose_row_keys(spans: &[TextSpan], projected: &[SegmentData]) -> Vec<Vec<SegmentKey>> {
    if projected.iter().any(|segment| segment.assigned_role.is_some()) {
        return Vec::new();
    }
    let Some(rows) = super::super::text::certified_stepped_panel_rows(spans) else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|index| SegmentKey::from(&projected[index]))
                .collect()
        })
        .collect()
}

fn extract_page(
    doc: &mut super::NativeDocument,
    page: usize,
    roles: &HashMap<u32, Option<u8>>,
    artifacts: ArtifactRetention,
    geometry: ScriptGeometry,
) -> crate::pdf::error::Result<NativePageSegments> {
    let mut prime_witnesses = Vec::new();
    let mut cmsy = Vec::new();
    let capture = (cfg!(feature = "formula-recognition") && matches!(doc.doc.get_page_rotation(page), Ok(0)))
        .then_some(&mut cmsy);
    let spans =
        super::extract_final_spans_from_page_with_primes(doc, page, geometry, Some(&mut prime_witnesses), capture)?;
    #[cfg(feature = "formula-recognition")]
    let inline_math_glyphs = surviving_inline_math_glyphs(&cmsy, &spans, roles, geometry);
    #[cfg(feature = "formula-recognition")]
    tracing::trace!(target: "xberg::pdf::cmsy", page, captured = cmsy.len(), surviving = inline_math_glyphs.len(), "native span witness inventory");

    let decimal_authority = decimal_power::authority(&doc.doc, page, &spans);
    let mut captured =
        NativePageSegments::from_final_spans_with_primes(spans, roles, artifacts, prime_witnesses, decimal_authority);
    #[cfg(feature = "formula-recognition")]
    {
        captured.inline_math_glyphs = inline_math_glyphs;
    }
    if !captured.prose_rows.is_empty()
        && (!doc
            .doc
            .get_annotations(page)
            .is_ok_and(|annotations| annotations.is_empty())
            || !super::super::text::untagged_upright_page(&doc.doc, page))
    {
        captured.clear_prose_rows();
    }
    Ok(captured)
}

#[cfg(test)]
mod tests;

#[cfg(feature = "formula-recognition")]
fn surviving_inline_math_glyphs(
    witnesses: &[TextSpan],
    spans: &[TextSpan],
    roles: &HashMap<u32, Option<u8>>,
    geometry: ScriptGeometry,
) -> Vec<SegmentKey> {
    witnesses
        .iter()
        .filter_map(|witness| {
            let mut candidates = spans.iter().filter(|span| span.sequence == witness.sequence);
            let Some(span) = candidates.next() else {
                tracing::trace!(target: "xberg::pdf::cmsy", sequence = witness.sequence, "source sequence missing");
                return None;
            };
            if candidates.next().is_some()
                || span.text != witness.text
                || span.bbox != witness.bbox
                || span.font_name != witness.font_name
                || span.mcid_scope != witness.mcid_scope
            {
                tracing::trace!(target: "xberg::pdf::cmsy", sequence = witness.sequence,
                    same_text = span.text == witness.text, same_bounds = span.bbox == witness.bbox,
                    same_font = span.font_name == witness.font_name, same_scope = span.mcid_scope == witness.mcid_scope,
                    "source sequence no longer identifies the glyph uniquely");
                return None;
            }
            Some(SegmentKey::from(&super::project_final_span(
                span.clone(),
                roles,
                geometry,
            )))
        })
        .collect()
}

#[cfg(all(test, feature = "formula-recognition"))]
#[test]
fn inline_math_witness_distinguishes_source_slots_and_occurrences() {
    fn pdf(font_entries: &str, cmsy_cmap: &str, content: &str) -> xberg_native_pdf::PdfDocument {
        let cmap = "1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <41> <D835DCB1> endbfchar";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>".into(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+CMSY10 {font_entries} >>"),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 7 0 R >>".into(),
            format!("<< /Length {} >>\nstream\n{cmap}\nendstream", cmap.len()),
            format!("<< /Length {} >>\nstream\n{cmsy_cmap}\nendstream", cmsy_cmap.len()),
        ];
        let mut bytes = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
        }
        let xref = bytes.len();
        bytes.push_str("xref\n0 9\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.push_str(&format!("{offset:010} 00000 n \n"));
        }
        bytes.push_str(&format!(
            "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        ));
        xberg_native_pdf::PdfDocument::from_bytes(bytes.into_bytes()).unwrap()
    }

    let content = "BT /F1 12 Tf 40 700 Td (V) Tj 0 -30 Td [(V)] TJ /F2 12 Tf 0 -30 Td (A) Tj ET";
    let codespace = "1 begincodespacerange <00> <FF> endcodespacerange";
    let identity = format!("{codespace} 1 beginbfrange <41> <5A> <0041> endbfrange");
    let conflicting = format!("{codespace} 1 beginbfchar <56> <0058> endbfchar");
    let missing_slot = format!("{codespace} 1 beginbfchar <41> <0041> endbfchar");
    for (case, font_entries, cmap, glyph, expected) in [
        ("built-in encoding", "", "", "V", 2),
        ("identity range", "/ToUnicode 8 0 R", identity.as_str(), "V", 2),
        ("conflicting mapping", "/ToUnicode 8 0 R", conflicting.as_str(), "X", 0),
        ("missing slot", "/ToUnicode 8 0 R", missing_slot.as_str(), "V", 0),
        ("unparseable CMap", "/ToUnicode 8 0 R", "invalid cmap data", "V", 0),
        ("named encoding", "/Encoding /WinAnsiEncoding", "", "V", 0),
        (
            "encoding differences",
            "/Encoding << /Differences [86 /V] >>",
            "",
            "V",
            0,
        ),
    ] {
        let doc = pdf(font_entries, cmap, content);
        let mut witnesses = Vec::new();
        let spans = doc
            .extract_page_text_with_cmsy_capture(0, xberg_native_pdf::ReadingOrder::TopToBottom, Some(&mut witnesses))
            .unwrap()
            .spans;
        assert!(spans.iter().any(|span| span.text == "𝒱"));
        assert_eq!(spans.iter().filter(|span| span.text == glyph).count(), 2, "{case}");
        assert_eq!(witnesses.len(), expected, "{case}");
        let keys = surviving_inline_math_glyphs(&witnesses, &spans, &HashMap::new(), ScriptGeometry::Preserve);
        assert_eq!(keys.len(), expected, "{case}");
        if expected == 2 {
            assert_ne!(witnesses[0].sequence, witnesses[1].sequence);
            assert_ne!(keys[0], keys[1]);
            assert!(witnesses.iter().all(|witness| witness.font_name == "ABCDEF+CMSY10"));
            for other_font in ["Helvetica", "GHIJKL+CMSY10"] {
                let mut competing = spans.clone();
                let candidate = competing
                    .iter_mut()
                    .find(|span| span.sequence == witnesses[0].sequence)
                    .unwrap();
                candidate.font_name = other_font.into();
                assert_eq!(
                    surviving_inline_math_glyphs(&witnesses, &competing, &HashMap::new(), ScriptGeometry::Preserve),
                    vec![keys[1].clone()],
                    "{case}: {other_font}"
                );
            }
            let mut ambiguous = spans.clone();
            ambiguous.push(witnesses[0].clone());
            assert_eq!(
                surviving_inline_math_glyphs(&witnesses, &ambiguous, &HashMap::new(), ScriptGeometry::Preserve).len(),
                1
            );
            assert!(keys.iter().all(|key| key.text == "V"));
        }
    }
    let natural = pdf("", "", "BT /F2 12 Tf 40 700 Td (A) Tj ET");
    let mut witnesses = Vec::new();
    let spans = natural
        .extract_page_text_with_cmsy_capture(0, xberg_native_pdf::ReadingOrder::TopToBottom, Some(&mut witnesses))
        .unwrap()
        .spans;
    assert!(spans.iter().any(|span| span.text == "𝒱"));
    assert!(witnesses.is_empty());
}
