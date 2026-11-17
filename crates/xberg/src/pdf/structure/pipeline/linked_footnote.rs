//! Classify a short footnote confirmed by a superscript reference and a native bottom-pagination rule.
//! No hyperlink is required. ~keep

use super::{HyphenWitnesses, LayoutHint, LayoutHintClass, PdfParagraph, finite_paragraph_bbox};
use crate::types::{BoundingBox, Table, document_structure::AnnotationKind};
use xberg_native_pdf::{PageRotation, PdfDocument, geometry::Rect};

const BOTTOM_PAGE_FRACTION: f32 = 0.15;
// Spatial marker merging needs a stronger size contrast to join separate paragraphs.
// This classifier already has a single grouped note, a painted rule and a matching reference.
// Keep the 6.5pt marker on 8pt body text (0.8125) in the witnessed note. ~keep
const MAX_MARKER_FONT_RATIO: f32 = 0.85;
const MAX_MARKER_RISE_EM: f32 = 0.75;
const MAX_RULE_GAP_EM: f32 = 2.0;
const MAX_NOTE_HEIGHT_EM: f32 = 2.0;
const SAME_TEXT_ROW_EM: f32 = 0.2;

pub(super) fn page_separator(document: &PdfDocument, page: usize) -> Option<Rect> {
    if !matches!(
        document.get_page_rotation_status(page).ok()?,
        PageRotation::Absent | PageRotation::Valid(0)
    ) {
        return None;
    }
    let (left, bottom, right, top) = document.get_page_media_box(page).ok()?;
    let rules = document.bottom_pagination_separator_bounds(page).ok()?;
    let [rule] = rules.as_slice() else {
        return None;
    };
    let margin_top = bottom + (top - bottom) * BOTTOM_PAGE_FRACTION;
    (rule.x >= left && rule.right() <= right && rule.y >= bottom && rule.bottom() <= margin_top).then_some(*rule)
}

pub(super) struct PageRegions<'a> {
    pub(super) page_number: u32,
    pub(super) tables: &'a [Table],
    pub(super) hints: &'a [LayoutHint],
}

impl PageRegions<'_> {
    fn overlaps_figure_or_table(&self, bounds: (f32, f32, f32, f32)) -> bool {
        let intersects = |bbox: BoundingBox| super::table_paragraph_overlap_fraction(bounds, &bbox) > 0.0;
        self.tables
            .iter()
            .any(|table| table.page_number == self.page_number && table.bounding_box.is_some_and(intersects))
            || self.hints.iter().any(|hint| {
                matches!(
                    hint.class_name,
                    LayoutHintClass::Picture | LayoutHintClass::Table | LayoutHintClass::Caption
                ) && intersects(BoundingBox {
                    x0: f64::from(hint.left),
                    y0: f64::from(hint.bottom),
                    x1: f64::from(hint.right),
                    y1: f64::from(hint.top),
                })
            })
    }
}

pub(super) fn classify(
    paragraphs: &mut [PdfParagraph],
    include_footnotes: bool,
    separator: impl FnOnce() -> Option<Rect>,
    image_placements: impl FnOnce() -> Option<Vec<Rect>>,
    regions: PageRegions<'_>,
    hyphens: &HyphenWitnesses,
) {
    if include_footnotes {
        return;
    }
    let mut candidates = paragraphs
        .iter()
        .enumerate()
        .filter_map(|(index, paragraph)| compact_note(paragraph).map(|note| (index, note)))
        .peekable();
    if candidates.peek().is_none() {
        return;
    }
    let Some(rule) = separator() else {
        return;
    };
    let matching = candidates.find(|(_, note)| note.below_rule(rule) && !regions.overlaps_figure_or_table(note.bounds));
    if let Some((index, note)) = matching
        && is_only_text_below_rule(paragraphs, index, rule)
        && exactly_one_reference_above_rule(paragraphs, note.marker, rule.bottom(), hyphens)
        && !super::page_furniture_exceeds_budget(paragraphs, |i, paragraph| i == index || paragraph.is_page_furniture)
        && image_placements().is_some_and(|placements| !placements.iter().any(|image| note.overlaps_image(*image)))
    {
        let paragraph = &mut paragraphs[index];
        paragraph.layout_class = Some(LayoutHintClass::Footnote);
        paragraph.is_page_furniture = true;
    }
}

fn is_only_text_below_rule(paragraphs: &[PdfParagraph], note_index: usize, rule: Rect) -> bool {
    paragraphs.iter().enumerate().all(|(index, paragraph)| {
        index == note_index
            || paragraph.is_page_furniture
            || !has_text(paragraph)
            || finite_paragraph_bbox(paragraph).is_some_and(|(left, bottom, right, _)| {
                // Path bounds use PDF y-up coordinates. A short rule cannot place indented text in another column. ~keep
                bottom >= rule.bottom()
                    || ((right < rule.left() || left > rule.right())
                        && matches!(
                            paragraph.layout_class,
                            Some(LayoutHintClass::Caption | LayoutHintClass::Picture | LayoutHintClass::Table)
                        ))
            })
    })
}

fn has_text(paragraph: &PdfParagraph) -> bool {
    !paragraph.text.trim().is_empty()
        || paragraph
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .any(|segment| !segment.text.trim().is_empty())
}

struct Note<'a> {
    marker: &'a str,
    bounds: (f32, f32, f32, f32),
    font_size: f32,
}

impl Note<'_> {
    fn overlaps_image(&self, image: Rect) -> bool {
        let (left, bottom, right, top) = self.bounds;
        left < image.right() && right > image.left() && bottom < image.bottom() && top > image.top()
    }

    fn below_rule(&self, rule: Rect) -> bool {
        let (left, bottom, right, top) = self.bounds;
        let gap = rule.y - top;
        (left - rule.x).abs() <= self.font_size * SAME_TEXT_ROW_EM
            && right <= rule.right()
            && (0.0..=self.font_size * MAX_NOTE_HEIGHT_EM).contains(&(top - bottom))
            && (0.0..=self.font_size * MAX_RULE_GAP_EM).contains(&gap)
    }
}

fn compact_note(paragraph: &PdfParagraph) -> Option<Note<'_>> {
    if paragraph.is_page_furniture
        || paragraph.heading_level.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || paragraph.caption_for.is_some()
        || !matches!(
            paragraph.layout_class,
            None | Some(LayoutHintClass::Text | LayoutHintClass::Footnote)
        )
    {
        return None;
    }
    let mut segments = paragraph.lines.iter().flat_map(|line| &line.segments);
    let marker = segments.next()?;
    let body = segments.next()?;
    let text = marker.text.trim();
    let em = body.font_size;
    let advance_gap = body.x - (marker.x + marker.width);
    let rise = marker.baseline_y - body.baseline_y;
    if !super::is_compact_footnote_marker(text)
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || !marker.is_unrotated()
        || !body.is_unrotated()
        || body.text.trim().is_empty()
        || !(0.0..=em * super::FOOTNOTE_MARKER_MAX_GAP_EM).contains(&advance_gap)
        || !(em * super::FOOTNOTE_MARKER_MIN_RISE_EM..=em * MAX_MARKER_RISE_EM).contains(&rise)
        || !(0.0..=em * MAX_MARKER_FONT_RATIO).contains(&marker.font_size)
        || segments.any(|segment| {
            !segment.is_unrotated() || (segment.baseline_y - body.baseline_y).abs() > em * SAME_TEXT_ROW_EM
        })
    {
        return None;
    }
    Some(Note {
        marker: text,
        bounds: finite_paragraph_bbox(paragraph)?,
        font_size: em,
    })
}

fn exactly_one_reference_above_rule(
    paragraphs: &[PdfParagraph],
    marker: &str,
    rule_y: f32,
    hyphens: &HyphenWitnesses,
) -> bool {
    let mut count = 0;
    for paragraph in paragraphs.iter().filter(|paragraph| {
        !paragraph.is_page_furniture
            && !paragraph.is_formula
            && finite_paragraph_bbox(paragraph).is_some_and(|(_, bottom, _, _)| bottom > rule_y)
    }) {
        let mut above = false;
        let mut other = false;
        for segment in paragraph.lines.iter().flat_map(|line| &line.segments) {
            if segment.text.trim() == marker {
                if segment.is_unrotated() && segment.baseline_y > rule_y {
                    above = true;
                } else {
                    other = true;
                }
            }
        }
        if !above {
            continue;
        }
        let references = count_annotated_references(paragraph, marker, hyphens);
        // An annotation can cover text from several segments. When identical
        // runs straddle the rule, their rendered offsets alone cannot identify the one above. ~keep
        if references > 0 && other {
            return false;
        }
        count += references;
        if count > 1 {
            return false;
        }
    }
    count == 1
}

fn count_annotated_references(paragraph: &PdfParagraph, marker: &str, hyphens: &HyphenWitnesses) -> usize {
    let (text, annotations) = super::super::assembly::extract_text_and_annotations(paragraph, hyphens);
    annotations
        .iter()
        .filter(|annotation| {
            let start = annotation.start as usize;
            annotation.kind == AnnotationKind::Superscript
                && text.get(start..annotation.end as usize) == Some(marker)
                && text
                    .get(..start)
                    .is_some_and(|prefix| prefix.ends_with(char::is_alphanumeric))
        })
        .take(2)
        .count()
}

#[cfg(test)]
#[path = "linked_footnote_tests.rs"]
mod tests;
