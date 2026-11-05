//! Decimal powers whose separate point fills a source-proved glyph advance gap.

use super::{ScriptOwner, SegmentKey, baseline, same_context, valid_geometry};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use xberg_native_pdf::{
    PdfDocument,
    fonts::{FontInfo, MappingProvenance},
    layout::{TextChar, TextSpan},
    structure::McidScope,
};

// Separately serialized TD/Tm coordinates in the source differ by less than a
// thousandth of a point after matrix arithmetic; this is not a font-size band. ~keep
const COORDINATE_TOLERANCE_PT: f32 = 0.001;

pub(super) struct DecimalRun {
    base: usize,
    members: Vec<usize>,
}

struct Proposal {
    base: usize,
    number: usize,
    point: usize,
    sign: Option<usize>,
    left: TextSpan,
    right: TextSpan,
}

pub(super) struct Authority {
    chars: Vec<TextChar>,
    by_y: Vec<usize>,
    fonts: HashMap<String, Option<Arc<FontInfo>>>,
}

impl Authority {
    fn load(doc: &PdfDocument, page: usize) -> Option<Self> {
        let resources = doc.get_page_resources(page).ok()?;
        let font_objects = doc.resolve_object(resources.as_dict()?.get("Font")?).ok()?;
        let mut fonts: HashMap<String, Option<Arc<FontInfo>>> = HashMap::new();
        for object in font_objects.as_dict()?.values() {
            let font = doc.get_or_load_font_for_rendering(object).ok()?;
            match fonts.entry(font.base_font.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(Some(font));
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry
                        .get()
                        .as_ref()
                        .is_none_or(|existing| !Arc::ptr_eq(existing, &font))
                    {
                        entry.insert(None);
                    }
                }
            }
        }
        let chars = doc.extract_chars(page).ok()?;
        let mut by_y: Vec<_> = (0..chars.len()).filter(|&i| chars[i].origin_y.is_finite()).collect();
        by_y.sort_by(|&a, &b| chars[a].origin_y.total_cmp(&chars[b].origin_y));
        Some(Self { chars, by_y, fonts })
    }

    fn row(&self, y: f32) -> Option<Vec<&TextChar>> {
        let start = self
            .by_y
            .partition_point(|&i| self.chars[i].origin_y < y - COORDINATE_TOLERANCE_PT);
        let end = self
            .by_y
            .partition_point(|&i| self.chars[i].origin_y <= y + COORDINATE_TOLERANCE_PT);
        (end - start <= super::MAX_CANDIDATE_EDGES)
            .then(|| self.by_y[start..end].iter().map(|&i| &self.chars[i]).collect())
    }

    fn matches(&self, glyph: &TextChar, span: &TextSpan) -> bool {
        let Some(font) = self.fonts.get(&span.font_name).and_then(Option::as_ref) else {
            return false;
        };
        font.has_explicit_widths()
            && trusted_mapping(span)
            && glyph.mcid == span.mcid
            && glyph.font_name == span.font_name
            && glyph.font_size == span.font_size
            && glyph.font_weight == span.font_weight
            && glyph.is_italic == span.is_italic
            && glyph.is_monospace == span.is_monospace
            && glyph.color == span.color
            && glyph.rotation_degrees == 0.0
            && glyph.origin_x.is_finite()
            && glyph.advance_width.is_finite()
            && glyph.advance_width > 0.0
            && ascii_advance_matches(glyph, font)
    }
}

fn ascii_advance_matches(glyph: &TextChar, font: &FontInfo) -> bool {
    if !glyph.char.is_ascii() {
        return true;
    }
    let xberg_native_pdf::fonts::font_dict::Encoding::Standard(encoding) = &font.encoding else {
        return false;
    };
    matches!(
        encoding.as_str(),
        "MacRomanEncoding" | "WinAnsiEncoding" | "StandardEncoding"
    ) && close(
        glyph.advance_width,
        font.get_glyph_width(glyph.char as u16) * glyph.font_size / 1000.0,
    )
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() <= COORDINATE_TOLERANCE_PT
}

fn trusted_mapping(span: &TextSpan) -> bool {
    matches!(
        span.provenance,
        Some(MappingProvenance::EncodingName | MappingProvenance::ToUnicode)
    ) && span.mcid.is_none()
        && matches!(span.mcid_scope, Some(McidScope::Page(_)))
        && !span.split_boundary_before
        && !span.offset_semantic
        && span.char_x_offsets.is_empty()
        && span.rotation_degrees == 0.0
        && span.page_rotation_applied == 0
}

fn is_number(span: &TextSpan) -> bool {
    valid_geometry(span)
        && trusted_mapping(span)
        && span.text.len() >= 2
        && span.text.len() <= super::super::INLINE_SCRIPT_LOOKBACK
        && span.text.bytes().all(|byte| byte.is_ascii_digit())
        && span.char_widths.len() == span.text.len()
        && span.char_widths.iter().all(|width| width.is_finite() && *width > 0.0)
}

fn possible_pairs(spans: &[TextSpan]) -> Vec<(usize, usize)> {
    let mut points: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| span.text == "." && valid_geometry(span))
        .map(|(index, _)| index)
        .collect();
    points.sort_by(|&a, &b| spans[a].bbox.x.total_cmp(&spans[b].bbox.x));
    let mut pairs = Vec::new();
    for (number, span) in spans.iter().enumerate().filter(|(_, span)| is_number(span)) {
        let start = points.partition_point(|&i| spans[i].bbox.x <= span.bbox.x);
        let end = points.partition_point(|&i| spans[i].bbox.x < span.bbox.right());
        if end - start > super::super::INLINE_SCRIPT_LOOKBACK {
            continue;
        }
        for &point in &points[start..end] {
            let dot = &spans[point];
            if same_context(span, dot)
                && trusted_mapping(dot)
                && span.font_name == dot.font_name
                && span.font_size == dot.font_size
                && close(baseline(span), baseline(dot))
            {
                pairs.push((number, point));
                if pairs.len() > super::MAX_CANDIDATE_EDGES {
                    return pairs;
                }
            }
        }
    }
    pairs
}

fn raised_base_geometry(base: &TextSpan, script: &TextSpan) -> bool {
    let ratio = script.font_size / base.font_size;
    let shift = baseline(script) - baseline(base);
    same_context(base, script)
        && trusted_mapping(base)
        && base.font_name == script.font_name
        && terminal_operand(base).is_some()
        && (super::super::INLINE_SCRIPT_MIN_FONT_RATIO..=super::super::INLINE_SCRIPT_MAX_FONT_RATIO).contains(&ratio)
        && shift > base.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR
        && shift <= base.font_size * super::ROW_CANDIDATE_MAX_SHIFT_EM
}

fn candidate_pairs(spans: &[TextSpan]) -> Vec<(usize, usize)> {
    let mut pairs = possible_pairs(spans);
    if pairs.len() <= super::MAX_CANDIDATE_EDGES {
        pairs.retain(|&(number, _)| spans.iter().any(|base| raised_base_geometry(base, &spans[number])));
    }
    pairs
}

pub(super) fn authority(doc: &PdfDocument, page: usize, spans: &[TextSpan]) -> Option<Authority> {
    let pairs = candidate_pairs(spans);
    if pairs.is_empty() || pairs.len() > super::MAX_CANDIDATE_EDGES {
        return None;
    }
    crate::pdf::native::guard_native_panic(|| Ok::<_, ()>(Authority::load(doc, page)), |_| ())
        .ok()
        .flatten()
}

pub(super) fn prepare_with_authority(
    spans: &mut Vec<TextSpan>,
    owners: &mut [ScriptOwner],
    source: impl FnOnce() -> Option<Authority>,
) -> Vec<DecimalRun> {
    let pairs = candidate_pairs(spans);
    if pairs.is_empty() || pairs.len() > super::MAX_CANDIDATE_EDGES {
        return Vec::new();
    }
    let Some(authority) = source() else {
        return Vec::new();
    };
    let forbidden: HashSet<_> = owners
        .iter()
        .flat_map(|owner| [owner.base_ordinal, owner.script_ordinal])
        .collect();
    let mut claimed = HashSet::new();
    let mut proposals = Vec::new();
    for (number, point) in pairs {
        let Some(proposal) = prove_power(spans, &authority, number, point) else {
            continue;
        };
        let mut endpoints = vec![proposal.base, number, point];
        endpoints.extend(proposal.sign);
        if endpoints
            .iter()
            .any(|index| claimed.contains(index) || forbidden.contains(index))
        {
            continue;
        }
        claimed.extend(endpoints);
        proposals.push(proposal);
    }
    apply_proposals(spans, proposals, owners)
}

fn number_glyphs<'a>(authority: &'a Authority, span: &TextSpan) -> Option<Vec<&'a TextChar>> {
    let mut glyphs: Vec<_> = authority
        .row(baseline(span))?
        .into_iter()
        .filter(|glyph| {
            glyph.char.is_ascii_digit()
                && authority.matches(glyph, span)
                && glyph.origin_x >= span.bbox.x - COORDINATE_TOLERANCE_PT
                && glyph.origin_x < span.bbox.right()
        })
        .collect();
    glyphs.sort_by(|a, b| a.origin_x.total_cmp(&b.origin_x));
    if glyphs.iter().map(|glyph| glyph.char).collect::<String>() != span.text {
        return None;
    }
    let mut x = span.bbox.x;
    for (glyph, width) in glyphs.iter().zip(&span.char_widths) {
        if !close(glyph.origin_x, x) {
            return None;
        }
        x += width;
    }
    Some(glyphs)
}

fn unique_point<'a>(authority: &'a Authority, point: &TextSpan) -> Option<&'a TextChar> {
    let points: Vec<_> = authority
        .row(baseline(point))?
        .into_iter()
        .filter(|glyph| {
            glyph.char == '.'
                && authority.matches(glyph, point)
                && close(glyph.origin_x, point.bbox.x)
                && close(glyph.advance_width, point.bbox.width)
        })
        .collect();
    (points.len() == 1).then(|| points[0])
}

fn split_at_point(span: &TextSpan, point: &TextChar, glyphs: &[&TextChar]) -> Option<(TextSpan, TextSpan)> {
    let slots: Vec<_> = glyphs
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| {
            close(pair[0].origin_x + pair[0].advance_width, point.origin_x)
                && close(point.origin_x + point.advance_width, pair[1].origin_x)
        })
        .map(|(index, _)| index + 1)
        .collect();
    let [split] = slots.as_slice() else {
        return None;
    };
    Some((
        glyph_fragment(span, &glyphs[..*split]),
        glyph_fragment(span, &glyphs[*split..]),
    ))
}

fn glyph_fragment(source: &TextSpan, glyphs: &[&TextChar]) -> TextSpan {
    let mut fragment = source.clone();
    fragment.text = glyphs.iter().map(|glyph| glyph.char).collect();
    fragment.bbox.x = glyphs[0].origin_x;
    let last = glyphs.last().unwrap();
    fragment.bbox.width = last.origin_x + last.advance_width - fragment.bbox.x;
    fragment.char_x_offsets = glyphs.iter().map(|glyph| glyph.origin_x).collect();
    fragment.char_widths = glyphs.iter().map(|glyph| glyph.advance_width).collect();
    fragment
}

fn sign_index(spans: &[TextSpan], authority: &Authority, number: usize) -> Option<Option<usize>> {
    let span = &spans[number];
    let signs: Vec<_> = spans
        .iter()
        .enumerate()
        .filter(|(_, sign)| {
            sign.text == "−"
                && same_context(span, sign)
                && sign.provenance == Some(MappingProvenance::ToUnicode)
                && trusted_mapping(sign)
                && sign.font_size == span.font_size
                && close(baseline(sign), baseline(span))
                && span.bbox.x - sign.bbox.right() >= -span.font_size * super::NUMERIC_SUFFIX_OVERHANG_EM
                && span.bbox.x - sign.bbox.right() <= span.font_size * super::super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM
        })
        .map(|(index, _)| index)
        .collect();
    if signs.len() > 1 {
        return None;
    }
    if let Some(&index) = signs.first() {
        let sign = &spans[index];
        let glyphs: Vec<_> = authority
            .row(baseline(sign))?
            .into_iter()
            .filter(|glyph| {
                glyph.char == '−'
                    && authority.matches(glyph, sign)
                    && close(glyph.origin_x, sign.bbox.x)
                    && close(glyph.advance_width, sign.bbox.width)
            })
            .collect();
        if glyphs.len() != 1 {
            return None;
        }
    }
    Some(signs.first().copied())
}

fn terminal_operand(span: &TextSpan) -> Option<String> {
    if span.text.ends_with(')') {
        return span.text.contains('(').then(|| ")".into());
    }
    let mut letters: Vec<_> = span.text.chars().rev().take_while(char::is_ascii_alphabetic).collect();
    if letters.is_empty() || letters.len() > super::super::INLINE_SCRIPT_LOOKBACK {
        return None;
    }
    letters.reverse();
    Some(letters.into_iter().collect())
}

fn terminal_edge(span: &TextSpan, authority: &Authority) -> Option<f32> {
    let operand: Vec<_> = terminal_operand(span)?.chars().collect();
    let mut glyphs: Vec<_> = authority
        .row(baseline(span))?
        .into_iter()
        .filter(|glyph| {
            authority.matches(glyph, span)
                && glyph.origin_x >= span.bbox.x - COORDINATE_TOLERANCE_PT
                && glyph.origin_x + glyph.advance_width <= span.bbox.right() + COORDINATE_TOLERANCE_PT
        })
        .collect();
    glyphs.sort_by(|a, b| a.origin_x.total_cmp(&b.origin_x));
    let endings: Vec<_> = glyphs
        .windows(operand.len())
        .filter(|window| {
            window.iter().map(|glyph| glyph.char).eq(operand.iter().copied())
                && window
                    .windows(2)
                    .all(|pair| close(pair[0].origin_x + pair[0].advance_width, pair[1].origin_x))
        })
        .collect();
    let [operand_glyphs] = endings.as_slice() else {
        return None;
    };
    let last = operand_glyphs.last()?;
    let end = last.origin_x + last.advance_width;
    let start = operand_glyphs[0].origin_x;
    let unclaimed = authority.row(baseline(span))?.into_iter().any(|glyph| {
        !glyph.char.is_whitespace()
            && glyph.origin_x + glyph.advance_width > start + COORDINATE_TOLERANCE_PT
            && glyph.origin_x < span.bbox.right() - COORDINATE_TOLERANCE_PT
            && !operand_glyphs.iter().any(|claimed| std::ptr::eq(*claimed, glyph))
    });
    (!unclaimed).then_some(end)
}

fn base_candidates(spans: &[TextSpan], authority: &Authority, number: usize, sign: Option<usize>) -> Vec<usize> {
    let script = &spans[number];
    let left = spans[sign.unwrap_or(number)].bbox.x;
    spans
        .iter()
        .enumerate()
        .filter(|(_, base)| {
            if !raised_base_geometry(base, script) {
                return false;
            }
            terminal_edge(base, authority).is_some_and(|edge| {
                left - edge >= -base.font_size * super::NUMERIC_SUFFIX_OVERHANG_EM
                    && left - edge <= base.font_size * super::super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM
            })
        })
        .map(|(index, _)| index)
        .collect()
}

fn unclaimed_source_glyph(
    authority: &Authority,
    spans: &[TextSpan],
    base: usize,
    members: &[usize],
    digits: &[&TextChar],
    dot: &TextChar,
) -> Option<bool> {
    let sign = members.iter().copied().find(|&index| spans[index].text == "−");
    let sign_glyph = sign.and_then(|index| {
        authority.row(baseline(&spans[index]))?.into_iter().find(|glyph| {
            glyph.char == '−' && authority.matches(glyph, &spans[index]) && close(glyph.origin_x, spans[index].bbox.x)
        })
    });
    let mut expected = digits.to_vec();
    expected.push(dot);
    expected.extend(sign_glyph);
    let margin = spans[base].font_size * super::super::INLINE_SCRIPT_MAX_SUFFIX_GAP_EM;
    let left = spans[*members.first()?].bbox.x - margin;
    let last = digits.last()?;
    let right = last.origin_x + last.advance_width + margin;
    Some(authority.row(digits.first()?.origin_y)?.into_iter().any(|glyph| {
        !glyph.char.is_whitespace()
            && glyph.origin_x <= right
            && glyph.origin_x + glyph.advance_width >= left
            && !expected.iter().any(|claimed| std::ptr::eq(*claimed, glyph))
    }))
}

fn prove_power(spans: &[TextSpan], authority: &Authority, number: usize, point: usize) -> Option<Proposal> {
    let digits = number_glyphs(authority, &spans[number])?;
    let dot = unique_point(authority, &spans[point])?;
    let (left, right) = split_at_point(&spans[number], dot, &digits)?;
    let sign = sign_index(spans, authority, number)?;
    let candidates = base_candidates(spans, authority, number, sign);
    let [base] = candidates.as_slice() else {
        return None;
    };
    let mut members = vec![number, point];
    members.extend(sign);
    members.sort_by(|&a, &b| spans[a].bbox.x.total_cmp(&spans[b].bbox.x));
    if unclaimed_source_glyph(authority, spans, *base, &members, &digits, dot)? {
        return None;
    }
    // The interior point overlaps the complete digit span. Preserve both envelope
    // endpoints for the existing guard without dropping any exclusion member. ~keep
    let rightmost = *members
        .iter()
        .max_by(|&&a, &&b| spans[a].bbox.right().total_cmp(&spans[b].bbox.right()))?;
    members.push(rightmost);
    if super::has_stacked_counterpart(spans, *base, &members)
        || super::has_unclaimed_cohort_record(spans, *base, &members)
    {
        return None;
    }
    Some(Proposal {
        base: *base,
        number,
        point,
        sign,
        left,
        right,
    })
}

fn apply_proposals(spans: &mut Vec<TextSpan>, proposals: Vec<Proposal>, owners: &mut [ScriptOwner]) -> Vec<DecimalRun> {
    if proposals.is_empty() {
        return Vec::new();
    }
    let mut splits: HashMap<_, _> = proposals
        .iter()
        .map(|proposal| (proposal.number, (proposal.left.clone(), proposal.right.clone())))
        .collect();
    let mut locations = Vec::new();
    let mut repaired = Vec::new();
    for (index, span) in std::mem::take(spans).into_iter().enumerate() {
        let start = repaired.len();
        if let Some((left, right)) = splits.remove(&index) {
            repaired.extend([left, right]);
        } else {
            repaired.push(span);
        }
        locations.push(start..repaired.len());
    }
    let runs = proposals
        .into_iter()
        .map(|proposal| {
            let mut members = vec![
                locations[proposal.number].start,
                locations[proposal.point].start,
                locations[proposal.number].end - 1,
            ];
            members.extend(proposal.sign.map(|index| locations[index].start));
            members.sort_by(|&a, &b| repaired[a].bbox.x.total_cmp(&repaired[b].bbox.x));
            DecimalRun {
                base: locations[proposal.base].start,
                members,
            }
        })
        .collect();
    for owner in owners {
        owner.base_ordinal = locations[owner.base_ordinal].start;
        owner.script_ordinal = locations[owner.script_ordinal].start;
    }
    *spans = repaired;
    runs
}

pub(super) fn bind(
    runs: Vec<DecimalRun>,
    spans: &[TextSpan],
    projected: &[crate::pdf::hierarchy::SegmentData],
    owners: &mut Vec<ScriptOwner>,
) {
    let claimed: HashSet<_> = owners
        .iter()
        .flat_map(|owner| [owner.base_ordinal, owner.script_ordinal])
        .collect();
    for run in runs {
        if claimed.contains(&run.base)
            || run.members.iter().any(|member| {
                claimed.contains(member) || projected[*member].assigned_role != projected[run.base].assigned_role
            })
        {
            continue;
        }
        let keys = Arc::new(
            run.members
                .iter()
                .map(|&index| SegmentKey::from(&projected[index]))
                .collect(),
        );
        for index in run.members {
            owners.push(ScriptOwner {
                base_ordinal: run.base,
                script_ordinal: index,
                base_source: Arc::new(spans[run.base].clone()),
                script_source: Arc::new(spans[index].clone()),
                base: Arc::new(projected[run.base].clone()),
                script: Arc::new(projected[index].clone()),
                run: Arc::clone(&keys),
                literal_prime_right: None,
            });
        }
    }
}

#[cfg(test)]
mod tests;
