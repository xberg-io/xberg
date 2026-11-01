use super::{PdfParagraph, SegmentData, spans_visual_line_break};
use crate::pdf::structure::{layout_classify::has_formula_evidence, types::PdfLine};

struct SourceGap<'a> {
    frame: &'a SegmentData,
    left: f32,
    right: f32,
    start: f32,
    end: f32,
}

struct RowEvidence<'a> {
    anchor: &'a SegmentData,
    paired_cells: bool,
    formula_cell: bool,
}

pub(super) fn has_recurring_formula_cells(candidate: &PdfParagraph, following: &PdfParagraph) -> bool {
    let Some(gap) = source_gap(candidate) else {
        return false;
    };
    let mut paired_first = None;
    let mut formula_first = None;
    let mut paired_repeats = false;
    let mut formula_repeats = false;
    let mut previous: Option<&SegmentData> = None;
    for line in &following.lines {
        let Some(row) = row_evidence(line, &gap) else {
            return false;
        };
        if previous.is_some_and(|previous| previous.upright_baseline() <= row.anchor.upright_baseline()) {
            return false;
        }
        previous = Some(row.anchor);
        if row.paired_cells {
            paired_repeats |= observe_distinct_row(&mut paired_first, row.anchor);
        }
        if row.formula_cell {
            formula_repeats |= observe_distinct_row(&mut formula_first, row.anchor);
        }
    }
    paired_repeats && formula_repeats
}

fn source_gap(candidate: &PdfParagraph) -> Option<SourceGap<'_>> {
    let [line] = candidate.lines.as_slice() else {
        return None;
    };
    if !line.valid_native_scripts().is_empty() {
        return None;
    }
    let mut spans = line.segments.iter().filter(|span| !span.text.trim().is_empty());
    let left = spans.next()?;
    let right = spans.next()?;
    if spans.next().is_some()
        || !left.is_bold
        || !right.is_bold
        || left.font_size != right.font_size
        || left.is_italic != right.is_italic
        || spans_visual_line_break(left, right)
    {
        return None;
    }
    let (left_edge, start) = source_extent(left, left)?;
    let (end, right_edge) = source_extent(right, left)?;
    let width = end - start;
    (width.is_finite() && width > 0.0).then_some(SourceGap {
        frame: left,
        left: left_edge,
        right: right_edge,
        start,
        end,
    })
}

fn source_extent(span: &SegmentData, frame: &SegmentData) -> Option<(f32, f32)> {
    let (start, end) = span.upright_advance_extent();
    (span.has_same_rotation(frame)
        && span.assigned_role.is_none()
        && !span.is_monospace
        && span.font_size.is_finite()
        && span.font_size > 0.0
        && span.upright_baseline().is_finite()
        && start.is_finite()
        && end.is_finite()
        && end > start)
        .then_some((start, end))
}

fn source_row_tolerance(line: &PdfLine) -> Option<f32> {
    if !line.baseline_y.is_finite() {
        return None;
    }
    let mut scale = 0.0_f32;
    for span in &line.segments {
        if !span.height.is_finite() {
            return None;
        }
        scale = scale.max(span.font_size.max(span.height).abs());
    }
    // ~keep Finished lines may be sorted by x. The saved baseline is the original grouping anchor;
    // the final maximum scale bounds every tolerance used before that sorting.
    Some(super::line_baseline_tolerance(scale, scale))
}

fn row_evidence<'a>(line: &'a PdfLine, gap: &SourceGap<'_>) -> Option<RowEvidence<'a>> {
    if !line.valid_native_scripts().is_empty() {
        return None;
    }
    let baseline_tolerance = source_row_tolerance(line)?;
    let seam = gap.start.midpoint(gap.end);
    let mut left_end = f32::NEG_INFINITY;
    let mut right_start = f32::INFINITY;
    let mut anchor = None;
    let mut text = String::new();
    for (index, span) in line.segments.iter().enumerate() {
        if !span.text.trim().is_empty() {
            let (start, end) = source_extent(span, gap.frame)?;
            if start < gap.left || end > gap.right || (start < seam && end > seam) {
                return None;
            }
            if (span.upright_baseline() - line.baseline_y).abs() > baseline_tolerance {
                return None;
            }
            anchor.get_or_insert(span);
            if end <= seam {
                left_end = left_end.max(end);
            } else {
                right_start = right_start.min(start);
            }
        }
        if index > 0 {
            text.push(' ');
        }
        text.push_str(&span.text);
    }
    let paired_cells = left_end.is_finite() && right_start.is_finite();
    if paired_cells && right_start - left_end < gap.end - gap.start {
        return None;
    }
    Some(RowEvidence {
        anchor: anchor?,
        paired_cells,
        formula_cell: !left_end.is_finite() && right_start.is_finite() && has_formula_evidence(&text),
    })
}

fn observe_distinct_row<'a>(first: &mut Option<&'a SegmentData>, current: &'a SegmentData) -> bool {
    if let Some(first) = first {
        spans_visual_line_break(first, current)
    } else {
        *first = Some(current);
        false
    }
}
