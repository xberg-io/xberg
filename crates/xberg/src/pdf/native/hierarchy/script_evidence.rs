//! Private evidence over final preserved spans, with page-local ordinals rather than PDF identities.

use std::collections::HashMap;
use std::sync::Arc;

use xberg_native_pdf::{fonts::MappingProvenance, layout::TextSpan};

use super::{ArtifactRetention, ScriptGeometry};
use crate::pdf::hierarchy::SegmentData;

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
}

impl NativePageSegments {
    #[cfg(test)]
    pub(crate) fn from_final_spans(
        spans: Vec<TextSpan>,
        roles: &HashMap<u32, Option<u8>>,
        artifacts: ArtifactRetention,
    ) -> Self {
        let primes = capture_literal_primes(&spans);
        Self::from_final_spans_with_primes(spans, roles, artifacts, primes)
    }

    fn from_final_spans_with_primes(
        spans: Vec<TextSpan>,
        roles: &HashMap<u32, Option<u8>>,
        artifacts: ArtifactRetention,
        prime_witnesses: Vec<LiteralPrimeWitness>,
    ) -> Self {
        let projected: Vec<_> = spans
            .iter()
            .cloned()
            .map(|span| super::project_final_span(span, roles, ScriptGeometry::Preserve))
            .collect();
        let mut owners = collect_owners(&spans, &projected);
        let primes = literal_prime::bind_literal_primes(prime_witnesses, &spans, &projected, &owners);
        owners.extend(primes);
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
        Self { segments, owners }
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

fn has_adjacent_numeric_source(base: &TextSpan, script: &TextSpan) -> bool {
    let numeric_base = is_numeric_span(&base.text);
    let numeric_suffix = script.text.strip_prefix('−').map_or_else(
        || is_numeric_span(&script.text),
        |digits| numeric_base && (digits.is_empty() || is_numeric_span(digits)),
    );
    (numeric_base || (base.text.len() == 1 && base.text.as_bytes()[0].is_ascii_alphabetic()))
        && numeric_suffix
        && has_adjacent_source(base, script)
}

fn has_adjacent_source(left: &TextSpan, right: &TextSpan) -> bool {
    left.sequence.checked_add(1) == Some(right.sequence)
        && left.mcid == right.mcid
        && [left, right].into_iter().all(|span| {
            span.provenance == Some(MappingProvenance::ToUnicode)
                && !span.split_boundary_before
                && !span.offset_semantic
                // Span splitting can synthesize offsets. Do not use those to admit overlap. ~keep
                && span.char_x_offsets.is_empty()
        })
}

fn eligible_base(base: &TextSpan, script: &TextSpan) -> bool {
    let shift = baseline(script) - baseline(base);
    let ratio = script.font_size / base.font_size;
    let gap = script.bbox.x - base.bbox.right();
    let supported_gap = if gap < 0.0 || script.text.starts_with('−') {
        gap >= -base.font_size * NUMERIC_SUFFIX_OVERHANG_EM && has_adjacent_numeric_source(base, script)
    } else {
        true
    };
    same_context(base, script)
        && base.text.chars().any(char::is_alphanumeric)
        && !base.text.ends_with(char::is_whitespace)
        && (super::INLINE_SCRIPT_MIN_FONT_RATIO..=super::INLINE_SCRIPT_MAX_FONT_RATIO).contains(&ratio)
        && shift > base.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
        && shift <= base.font_size * ROW_CANDIDATE_MAX_SHIFT_EM
        && supported_gap
        && gap <= base.font_size * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM
}

fn candidate_bases(spans: &[TextSpan], edges: &[(f32, usize)], script: usize, max_font: f32) -> Option<Vec<usize>> {
    let x = spans[script].bbox.x;
    let start = edges.partition_point(|(right, _)| *right < x - max_font * super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM);
    let end = edges.partition_point(|(right, _)| *right <= x + max_font * NUMERIC_SUFFIX_OVERHANG_EM);
    if end - start > MAX_CANDIDATE_EDGES {
        return None;
    }
    Some(
        edges[start..end]
            .iter()
            .filter_map(|&(_, base)| (base != script && eligible_base(&spans[base], &spans[script])).then_some(base))
            .collect(),
    )
}

fn complete_run(spans: &[TextSpan], base: usize, first: usize) -> Option<Vec<usize>> {
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
    if (has_overhang || has_minus) && !is_complete_numeric_suffix(&spans[base], spans, &members) {
        return None;
    }
    Some(members)
}

fn is_complete_numeric_suffix(base: &TextSpan, spans: &[TextSpan], members: &[usize]) -> bool {
    let Some((&first, rest)) = members.split_first() else {
        return false;
    };
    let first = &spans[first];
    let digits = first.text.strip_prefix('−').unwrap_or(&first.text);
    has_adjacent_numeric_source(base, first)
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
    let mut claimed = std::collections::HashSet::new();
    let notes = super::super::text::numeric_notes(spans);
    for (first, _) in spans.iter().enumerate().filter(|(_, span)| compact_marker(span)) {
        if claimed.contains(&first) {
            continue;
        }
        let Some(candidates) = candidate_bases(spans, &edges, first, max_font) else {
            continue;
        };
        let [base] = candidates.as_slice() else {
            continue;
        };
        let Some(members) = complete_run(spans, *base, first) else {
            continue;
        };
        if has_stacked_counterpart(spans, *base, &members) || has_unclaimed_cohort_record(spans, *base, &members) {
            continue;
        }
        if members.iter().any(|&member| {
            projected[*base].assigned_role != projected[member].assigned_role
                || super::super::text::needs_numeric_script_boundary(&spans[*base], &spans[member], &notes, None)
                || claimed.contains(&member)
                || candidate_bases(spans, &edges, member, max_font)
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

fn extract_page(
    doc: &mut super::NativeDocument,
    page: usize,
    roles: &HashMap<u32, Option<u8>>,
    artifacts: ArtifactRetention,
    geometry: ScriptGeometry,
) -> crate::pdf::error::Result<NativePageSegments> {
    let mut prime_witnesses = Vec::new();
    let spans = super::extract_final_spans_from_page_with_primes(doc, page, geometry, Some(&mut prime_witnesses))?;
    Ok(NativePageSegments::from_final_spans_with_primes(
        spans,
        roles,
        artifacts,
        prime_witnesses,
    ))
}

#[cfg(test)]
mod tests;
