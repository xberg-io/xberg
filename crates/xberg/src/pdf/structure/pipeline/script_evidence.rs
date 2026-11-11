//! Attach proved native suffixes only after ordinary paragraph semantics have been finalized.

use std::collections::{HashMap, HashSet};

use crate::pdf::hierarchy::SegmentData;
use crate::pdf::native::hierarchy::script_evidence::{NativePageSegments, ScriptOwner, SegmentKey};
use crate::pdf::structure::types::{LayoutHintClass, PdfLine, PdfParagraph};

pub(super) fn associate_surviving_suffixes(paragraphs: &mut Vec<PdfParagraph>, page: &NativePageSegments) -> bool {
    let scripts = associate_matching_suffixes(paragraphs, page, false);
    let primes = page.has_literal_primes() && associate_matching_suffixes(paragraphs, page, true);
    scripts || primes
}

fn associate_matching_suffixes(
    paragraphs: &mut Vec<PdfParagraph>,
    page: &NativePageSegments,
    literal_primes: bool,
) -> bool {
    let segments: Vec<_> = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| line.segments.iter().cloned())
        .collect();
    let positions: HashMap<_, _> = paragraphs
        .iter()
        .enumerate()
        .flat_map(|(index, paragraph)| {
            paragraph
                .lines
                .iter()
                .flat_map(|line| &line.segments)
                .map(move |segment| (SegmentKey::from(segment), index))
        })
        .collect();
    let owners: Vec<_> = page
        .valid_owners(&segments)
        .into_iter()
        .filter(|owner| owner.literal_prime_right().is_some() == literal_primes)
        .filter(|owner| run_context_survives(owner, paragraphs, &positions))
        .collect();
    if owners.is_empty() {
        return false;
    }
    let moved: HashSet<_> = owners
        .iter()
        .map(|owner| SegmentKey::from(owner.script.as_ref()))
        .collect();
    let mut insertions = sorted_insertions(owners);
    if !preserves_existing_scripts(paragraphs, &moved, &insertions) {
        return false;
    }
    paragraphs.retain_mut(|paragraph| {
        let had_lines = !paragraph.lines.is_empty();
        move_paragraph_suffixes(paragraph, &moved, &mut insertions);
        !had_lines || !paragraph.lines.is_empty()
    });
    true
}

fn sorted_insertions(owners: Vec<ScriptOwner>) -> HashMap<SegmentKey, Vec<ScriptOwner>> {
    let mut insertions: HashMap<SegmentKey, Vec<ScriptOwner>> = HashMap::new();
    for owner in owners {
        insertions
            .entry(SegmentKey::from(owner.base.as_ref()))
            .or_default()
            .push(owner);
    }
    for children in insertions.values_mut() {
        children.sort_by(|left, right| left.script.x.total_cmp(&right.script.x));
    }
    insertions
}

fn preserves_existing_scripts(
    paragraphs: &[PdfParagraph],
    moved: &HashSet<SegmentKey>,
    insertions: &HashMap<SegmentKey, Vec<ScriptOwner>>,
) -> bool {
    let base_of_script = owning_bases(insertions);
    paragraphs.iter().all(|paragraph| {
        let before: Vec<_> = paragraph.lines.iter().flat_map(|line| &line.segments).collect();
        let mut after = Vec::new();
        let mut original_positions = Vec::new();
        for (index, segment) in before.iter().enumerate() {
            let key = SegmentKey::from(*segment);
            if moved.contains(&key) {
                if super::super::assembly::segment_script_kind(&before, index).is_some()
                    && !already_follows_its_base(&before, index, &base_of_script, insertions, moved)
                {
                    return false;
                }
                continue;
            }
            original_positions.push((index, after.len()));
            after.push(*segment);
            if let Some(children) = insertions.get(&key) {
                after.extend(children.iter().map(|owner| owner.script.as_ref()));
            }
        }
        // New records can displace or replace a legacy base in the lookback.
        // Preserve both its joining classification and its emitted markup. ~keep
        original_positions.into_iter().all(|(old, new)| {
            super::super::assembly::segment_script(&before, old) == super::super::assembly::segment_script(&after, new)
        })
    })
}

/// The base and the rank among its scripts of each script that has exactly one owning base. A
/// script that two bases claim is left out, so it is never taken for already placed.
fn owning_bases(insertions: &HashMap<SegmentKey, Vec<ScriptOwner>>) -> HashMap<SegmentKey, (&SegmentKey, usize)> {
    let mut owned: HashMap<SegmentKey, Option<(&SegmentKey, usize)>> = HashMap::new();
    for (base, owners) in insertions {
        for (rank, owner) in owners.iter().enumerate() {
            owned
                .entry(SegmentKey::from(owner.script.as_ref()))
                .and_modify(|claim| *claim = None)
                .or_insert(Some((base, rank)));
        }
    }
    owned
        .into_iter()
        .filter_map(|(script, claim)| claim.map(|claim| (script, claim)))
        .collect()
}

/// Whether the moved script at `index` already sits where the insertion would put it: right after
/// its base and the scripts that rank before it, with the base itself staying where it is. The move
/// then changes nothing and cannot disturb a script that the paragraph already reads as one.
fn already_follows_its_base(
    before: &[&SegmentData],
    index: usize,
    base_of_script: &HashMap<SegmentKey, (&SegmentKey, usize)>,
    insertions: &HashMap<SegmentKey, Vec<ScriptOwner>>,
    moved: &HashSet<SegmentKey>,
) -> bool {
    let Some(&(base, rank)) = base_of_script.get(&SegmentKey::from(before[index])) else {
        return false;
    };
    let Some(start) = index.checked_sub(rank + 1) else {
        return false;
    };
    !moved.contains(base)
        && SegmentKey::from(before[start]) == *base
        && insertions[base]
            .iter()
            .take(rank)
            .zip(&before[start + 1..index])
            .all(|(owner, segment)| SegmentKey::from(owner.script.as_ref()) == SegmentKey::from(*segment))
}

fn run_context_survives(
    owner: &ScriptOwner,
    paragraphs: &[PdfParagraph],
    positions: &HashMap<SegmentKey, usize>,
) -> bool {
    let base_key = SegmentKey::from(owner.base.as_ref());
    let Some(&base) = positions.get(&base_key) else {
        return false;
    };
    if let Some(right) = owner.literal_prime_right()
        && !paragraphs[base].lines.iter().any(|line| {
            line.segments
                .windows(2)
                .any(|pair| SegmentKey::from(&pair[0]) == base_key && SegmentKey::from(&pair[1]) == *right)
        })
    {
        return false;
    }
    owner.run_members().iter().all(|key| {
        positions.get(key).is_some_and(|&script| {
            let base = &paragraphs[base];
            let script = &paragraphs[script];
            base.layout_region_path == script.layout_region_path
                && base.is_page_furniture == script.is_page_furniture
                && furniture_class(base) == furniture_class(script)
        })
    })
}

fn furniture_class(paragraph: &PdfParagraph) -> Option<LayoutHintClass> {
    paragraph.layout_class.filter(|class| {
        matches!(
            class,
            LayoutHintClass::Footnote | LayoutHintClass::PageHeader | LayoutHintClass::PageFooter
        )
    })
}

fn move_paragraph_suffixes(
    paragraph: &mut PdfParagraph,
    moved: &HashSet<SegmentKey>,
    insertions: &mut HashMap<SegmentKey, Vec<ScriptOwner>>,
) {
    let touched = paragraph.lines.iter().flat_map(|line| &line.segments).any(|segment| {
        let key = SegmentKey::from(segment);
        moved.contains(&key) || insertions.contains_key(&key)
    });
    if !touched {
        return;
    }
    paragraph.lines = std::mem::take(&mut paragraph.lines)
        .into_iter()
        .filter_map(|mut line| {
            let had_segments = !line.segments.is_empty();
            move_line_suffixes(&mut line, moved, insertions);
            (!had_segments || !line.segments.is_empty()).then_some(line)
        })
        .collect();
    paragraph.text.clear();
    paragraph.word_count = PdfParagraph::compute_word_count(&paragraph.text, &paragraph.lines);
    if paragraph.block_bbox.take().is_some() {
        paragraph.block_bbox = super::paragraph_geometry_bbox(paragraph);
    }
}

fn move_line_suffixes(
    line: &mut PdfLine,
    moved: &HashSet<SegmentKey>,
    insertions: &mut HashMap<SegmentKey, Vec<ScriptOwner>>,
) {
    let mut records = Vec::new();
    line.native_scripts
        .retain(|owner| !moved.contains(&SegmentKey::from(owner.script.as_ref())));
    for segment in std::mem::take(&mut line.segments) {
        let key = SegmentKey::from(&segment);
        if moved.contains(&key) {
            continue;
        }
        let children = insertions.remove(&key).unwrap_or_default();
        records.push(segment);
        records.extend(children.iter().map(|owner| owner.script.as_ref().clone()));
        line.native_scripts.extend(children);
    }
    line.segments = records;
}

#[cfg(test)]
fn native_paragraphs(
    segments: Vec<SegmentData>,
    page: &NativePageSegments,
    heading_map: &[(f32, Option<u8>)],
    gaps: &[f32],
) -> Option<Vec<PdfParagraph>> {
    let mut paragraphs = super::blocks_to_paragraphs(segments, heading_map, gaps);
    associate_surviving_suffixes(&mut paragraphs, page).then_some(paragraphs)
}

#[cfg(test)]
mod tests;
