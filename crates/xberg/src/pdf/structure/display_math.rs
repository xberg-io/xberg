//! Display formulas as blocks of their own.
//!
//! Native math rewriting leaves a display formula as one segment between
//! [`sentinels::DISPLAY_OPEN`] and [`sentinels::DISPLAY_CLOSE`], and paragraph building may
//! have flowed it into the prose around it. Each such segment becomes a formula paragraph,
//! splitting its paragraph into the prose before and after it.

use super::types::{PdfLine, PdfParagraph};
use crate::pdf::hierarchy::SegmentData;
use crate::pdf::native::math_latex::sentinels;

pub(super) fn split_display_formulas(pages: &mut [Vec<PdfParagraph>]) {
    for paragraphs in pages.iter_mut() {
        if !paragraphs.iter().any(holds_display_formula) {
            continue;
        }
        let source = std::mem::take(paragraphs);
        for paragraph in source {
            split_paragraph(paragraph, paragraphs);
        }
    }
}

fn is_display_formula(segment: &SegmentData) -> bool {
    segment.text.trim_start().starts_with(sentinels::DISPLAY_OPEN)
}

fn holds_display_formula(paragraph: &PdfParagraph) -> bool {
    paragraph.text.is_empty()
        && !paragraph.is_formula
        && !paragraph.is_list_item
        && !paragraph.is_code_block
        && paragraph.heading_level.is_none()
        && paragraph
            .lines
            .iter()
            .any(|line| line.segments.iter().any(is_display_formula))
}

fn split_paragraph(paragraph: PdfParagraph, out: &mut Vec<PdfParagraph>) {
    if !holds_display_formula(&paragraph) {
        out.push(paragraph);
        return;
    }
    let mut pending: Vec<PdfLine> = Vec::new();
    for line in &paragraph.lines {
        let mut partial: Vec<SegmentData> = Vec::new();
        for segment in &line.segments {
            if !is_display_formula(segment) {
                partial.push(segment.clone());
                continue;
            }
            if !partial.is_empty() {
                pending.push(with_segments(line, std::mem::take(&mut partial)));
            }
            flush(&paragraph, &mut pending, false, out);
            pending.push(with_segments(line, vec![segment.clone()]));
            flush(&paragraph, &mut pending, true, out);
        }
        if !partial.is_empty() {
            pending.push(with_segments(line, partial));
        }
    }
    flush(&paragraph, &mut pending, false, out);
}

fn with_segments(line: &PdfLine, segments: Vec<SegmentData>) -> PdfLine {
    PdfLine {
        segments,
        ..line.clone()
    }
}

/// Emit `lines` as a paragraph shaped like `source`, or as a formula paragraph.
fn flush(source: &PdfParagraph, lines: &mut Vec<PdfLine>, formula: bool, out: &mut Vec<PdfParagraph>) {
    if lines.is_empty() {
        return;
    }
    let lines = std::mem::take(lines);
    let word_count = PdfParagraph::compute_word_count("", &lines);
    out.push(PdfParagraph {
        lines,
        word_count,
        is_formula: formula,
        is_bold: source.is_bold && !formula,
        caption_for: None,
        ..source.clone()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str, x: f32) -> SegmentData {
        SegmentData {
            text: text.to_owned(),
            x,
            y: 700.0,
            width: 20.0,
            height: 10.0,
            font_size: 10.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
            ocr_heading_spans: None,
        }
    }

    fn line(segments: Vec<SegmentData>) -> PdfLine {
        PdfLine {
            segments,
            native_scripts: Vec::new(),
            baseline_y: Some(700.0),
            dominant_font_size: 10.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn paragraph(lines: Vec<PdfLine>) -> PdfParagraph {
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 10.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            ocr_source_spans: Vec::new(),
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    fn texts(paragraph: &PdfParagraph) -> Vec<&str> {
        paragraph
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn should_split_a_display_formula_out_of_its_paragraph() {
        let display = format!("{}z\\equiv 1{}", sentinels::DISPLAY_OPEN, sentinels::DISPLAY_CLOSE);
        let mut pages = vec![vec![paragraph(vec![
            line(vec![segment("we use the definition", 100.0)]),
            line(vec![segment(&display, 200.0), segment("(4)", 400.0)]),
            line(vec![segment("where r is the radius.", 100.0)]),
        ])]];
        split_display_formulas(&mut pages);
        let paragraphs = &pages[0];
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(texts(&paragraphs[0]), ["we use the definition"]);
        assert!(paragraphs[1].is_formula);
        assert_eq!(texts(&paragraphs[1]), [display.as_str()]);
        assert!(!paragraphs[2].is_formula);
        assert_eq!(texts(&paragraphs[2]), ["(4)", "where r is the radius."]);
    }

    #[test]
    fn should_leave_paragraphs_without_display_formulas_alone() {
        let inline = format!("{}r^{{2}}{}", sentinels::INLINE_OPEN, sentinels::INLINE_CLOSE);
        let mut pages = vec![vec![paragraph(vec![line(vec![
            segment("where", 100.0),
            segment(&inline, 130.0),
        ])])]];
        split_display_formulas(&mut pages);
        assert_eq!(pages[0].len(), 1);
        assert!(!pages[0][0].is_formula);
    }
}
