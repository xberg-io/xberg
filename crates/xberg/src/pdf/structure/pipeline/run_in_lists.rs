//! Render-aware run-in list planning with lossless native source projection.

use std::ops::Range;

use super::{HyphenWitnesses, paragraph_text_raw};
use crate::pdf::structure::assembly::{
    ParagraphRenderProjection, extract_text_and_annotations, extract_text_with_source_ranges,
};
use crate::pdf::structure::types::{LayoutHintClass, PdfLine, PdfParagraph};
use crate::types::document_structure::{AnnotationKind, TextAnnotation};

const MIN_LEAD_WORDS: usize = 4;
const MIN_ITEMS: usize = 2;
const MIN_ITEM_WORDS: usize = 3;

type SourceDestination = (usize, usize, usize);

struct ListReplacement {
    fragments: Vec<PdfParagraph>,
    destinations: Option<Vec<SourceDestination>>,
}

impl ListReplacement {
    fn move_source_records(&mut self, source: &mut PdfParagraph) -> bool {
        let Some(destinations) = &self.destinations else {
            return true;
        };
        if source.lines.iter().map(|line| line.segments.len()).sum::<usize>() != destinations.len() {
            return false;
        }
        let records = std::mem::take(&mut source.lines)
            .into_iter()
            .flat_map(|line| line.segments);
        for (segment, &(fragment, line, local)) in records.zip(destinations) {
            self.fragments[fragment].lines[line].segments[local] = segment;
        }
        true
    }
}

pub(super) fn split_with_witnesses(pages: &mut [Vec<PdfParagraph>], witnesses: &HyphenWitnesses) {
    for page in pages {
        let mut index = 0;
        while index < page.len() {
            if let Some(mut replacement) = try_split(&page[index], witnesses) {
                if !replacement.move_source_records(&mut page[index]) {
                    index += 1;
                    continue;
                }
                let inserted = replacement.fragments.len();
                page.splice(index..=index, replacement.fragments);
                index += inserted;
            } else {
                index += 1;
            }
        }
    }
}

fn try_split(source: &PdfParagraph, witnesses: &HyphenWitnesses) -> Option<ListReplacement> {
    if source.heading_level.is_some()
        || source.is_list_item
        || source.is_code_block
        || source.is_formula
        || source.is_page_furniture
    {
        return None;
    }
    if source.lines.iter().any(|line| !line.segments.is_empty()) {
        let rendered = extract_text_with_source_ranges(source, witnesses);
        let ranges = list_ranges(&rendered.text)?;
        return project_native_list(source, &rendered, &ranges, witnesses);
    }
    let text = paragraph_text_raw(source)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let ranges = list_ranges(&text)?;
    let fragments = ranges
        .into_iter()
        .enumerate()
        .map(|(index, range)| {
            let mut fragment = fragment_shell(source, index > 0);
            fragment.text = text[range].to_owned();
            fragment.word_count = fragment.text.split_whitespace().count();
            fragment
        })
        .collect();
    Some(ListReplacement {
        fragments,
        destinations: None,
    })
}

fn trimmed_range(text: &str, range: Range<usize>) -> Range<usize> {
    let slice = &text[range.clone()];
    let start = range.start + slice.len() - slice.trim_start().len();
    start..start + slice.trim().len()
}

fn list_ranges(text: &str) -> Option<Vec<Range<usize>>> {
    let colon = text.rfind(':')?;
    let lead = trimmed_range(text, 0..colon + 1);
    if text[lead.clone()].split_whitespace().count() < MIN_LEAD_WORDS {
        return None;
    }
    let mut ranges = vec![lead];
    let mut start = colon + 1;
    for clause in text[start..].split_inclusive(';') {
        let range = trimmed_range(text, start..start + clause.len());
        start += clause.len();
        if range.is_empty() {
            continue;
        }
        if !is_probable_item(&text[range.clone()]) {
            return None;
        }
        ranges.push(range);
    }
    (ranges.len() > MIN_ITEMS).then_some(ranges)
}

fn is_probable_item(item: &str) -> bool {
    item.split_whitespace().count() >= MIN_ITEM_WORDS
        && item.chars().next().is_some_and(char::is_lowercase)
        && matches!(item.chars().last(), Some(';' | '.'))
}

fn fragment_shell(source: &PdfParagraph, is_list_item: bool) -> PdfParagraph {
    PdfParagraph {
        text: String::new(),
        lines: Vec::new(),
        dominant_font_size: source.dominant_font_size,
        heading_level: None,
        is_bold: source.is_bold,
        is_list_item,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: source.is_page_furniture,
        layout_class: if is_list_item {
            Some(LayoutHintClass::ListItem)
        } else {
            source.layout_class
        },
        layout_region_path: source.layout_region_path,
        caption_for: source.caption_for,
        block_bbox: source.block_bbox,
        word_count: 0,
    }
}

fn source_fragments(segments: &[Range<usize>], ranges: &[Range<usize>]) -> Option<Vec<usize>> {
    let mut fragment = 0;
    let mut targets = Vec::with_capacity(segments.len());
    for segment in segments {
        if segment.is_empty() {
            targets.push(targets.last().copied().unwrap_or(0));
            continue;
        }
        while fragment + 1 < ranges.len() && segment.start >= ranges[fragment].end {
            fragment += 1;
        }
        if segment.start < ranges[fragment].start || segment.end > ranges[fragment].end {
            return None;
        }
        targets.push(fragment);
    }
    Some(targets)
}

fn empty_line(source: &PdfLine) -> PdfLine {
    PdfLine {
        segments: Vec::new(),
        native_scripts: Vec::new(),
        baseline_y: source.baseline_y,
        dominant_font_size: source.dominant_font_size,
        is_bold: source.is_bold,
        is_monospace: source.is_monospace,
    }
}

fn project_line(line: &PdfLine, targets: &[usize], fragments: &mut [PdfParagraph]) -> Option<Vec<SourceDestination>> {
    let mut positions = Vec::with_capacity(line.segments.len());
    let mut previous = None;
    for (segment, &target) in line.segments.iter().zip(targets) {
        let fragment = &mut fragments[target];
        if previous != Some(target) {
            fragment.lines.push(empty_line(line));
        }
        let line_index = fragment.lines.len().checked_sub(1)?;
        let output = &mut fragment.lines[line_index];
        positions.push((target, line_index, output.segments.len()));
        output.segments.push(segment.clone());
        previous = Some(target);
    }
    for owner in line.valid_native_scripts() {
        let &(base_fragment, base_line, base) = positions.get(owner.base_index)?;
        let &(script_fragment, script_line, script) = positions.get(owner.script_index)?;
        if (base_fragment, base_line) != (script_fragment, script_line) {
            return None;
        }
        let mut projected = owner.clone();
        projected.base_index = base;
        projected.script_index = script;
        fragments[base_fragment].lines[base_line].native_scripts.push(projected);
    }
    Some(positions)
}

fn annotation_slices(annotations: &[TextAnnotation], ranges: &[Range<usize>]) -> Option<Vec<Vec<TextAnnotation>>> {
    let mut result = vec![Vec::new(); ranges.len()];
    let mut first = 0;
    for annotation in annotations {
        let start = annotation.start as usize;
        let end = annotation.end as usize;
        while first + 1 < ranges.len() && ranges[first].end <= start {
            first += 1;
        }
        let mut index = first;
        while index < ranges.len() && ranges[index].start < end {
            let range = &ranges[index];
            let left = start.max(range.start);
            let right = end.min(range.end);
            if left < right {
                if (left != start || right != end)
                    && !matches!(annotation.kind, AnnotationKind::Bold | AnnotationKind::Italic)
                {
                    return None;
                }
                result[index].push(TextAnnotation {
                    start: (left - range.start) as u32,
                    end: (right - range.start) as u32,
                    kind: annotation.kind.clone(),
                });
            }
            index += 1;
        }
    }
    Some(result)
}

fn native_owner_count(paragraphs: &[PdfParagraph]) -> usize {
    paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .map(|line| line.valid_native_scripts().len())
        .sum()
}

fn project_native_list(
    source: &PdfParagraph,
    rendered: &ParagraphRenderProjection,
    ranges: &[Range<usize>],
    witnesses: &HyphenWitnesses,
) -> Option<ListReplacement> {
    let targets = source_fragments(&rendered.segment_ranges, ranges)?;
    let expected = annotation_slices(&rendered.annotations, ranges)?;
    let mut fragments: Vec<_> = ranges
        .iter()
        .enumerate()
        .map(|(index, range)| {
            let mut fragment = fragment_shell(source, index > 0);
            fragment.word_count = rendered.text[range.clone()].split_whitespace().count();
            fragment
        })
        .collect();
    let mut destinations = Vec::with_capacity(targets.len());
    for line in &source.lines {
        let offset = destinations.len();
        destinations.extend(project_line(
            line,
            targets.get(offset..offset + line.segments.len())?,
            &mut fragments,
        )?);
    }
    if destinations.len() != rendered.segment_ranges.len()
        || native_owner_count(&fragments) != native_owner_count(std::slice::from_ref(source))
    {
        return None;
    }
    // All source records remain whole. Context-sensitive spacing and annotations must also survive each cut. ~keep
    for ((fragment, range), annotations) in fragments.iter().zip(ranges).zip(expected) {
        let actual = extract_text_and_annotations(fragment, witnesses);
        if actual.0 != rendered.text[range.clone()] || actual.1 != annotations {
            return None;
        }
    }
    Some(ListReplacement {
        fragments,
        destinations: Some(destinations),
    })
}

#[cfg(test)]
mod tests;
