//! Native chart legend row association.

use super::{PdfParagraph, union_block_bbox};
use crate::pdf::hierarchy::SegmentData;

/// Rejoin a raised letter marker only inside a locally witnessed chart legend.
pub(in crate::pdf::structure) fn reattach_chart_legend_markers(paragraphs: &mut Vec<PdfParagraph>) {
    let markers = paragraphs
        .windows(4)
        .enumerate()
        .filter_map(|(before, rows)| chart_legend_pair(rows).then_some(before + 1))
        .collect::<Vec<_>>();

    for &index in &markers {
        let (leading, trailing) = paragraphs.split_at_mut(index + 1);
        let marker = &mut leading[index];
        let label = &mut trailing[0];
        label.lines[0].segments.insert(0, marker.lines[0].segments.remove(0));
        label.block_bbox = union_block_bbox(marker.block_bbox, label.block_bbox);
        label.text.clear();
        label.word_count = PdfParagraph::compute_word_count("", &label.lines);
    }
    if markers.is_empty() {
        return;
    }
    let mut markers = markers.into_iter().peekable();
    let mut index = 0;
    paragraphs.retain(|_| {
        let keep = markers.next_if_eq(&index).is_none();
        index += 1;
        keep
    });
}

fn chart_legend_pair(paragraphs: &[PdfParagraph]) -> bool {
    let [before, marker, label, after] = paragraphs else {
        return false;
    };
    let Some(marker_run) = plain_single_run(marker) else {
        return false;
    };
    let Some(label_run) = plain_single_run(label) else {
        return false;
    };
    let Some((number, None)) = legend_label_number(label_run.text.trim_start()) else {
        return false;
    };
    let marker_text = marker_run.text.trim();
    marker_text.len() == 1
        && marker_text.as_bytes()[0].is_ascii_uppercase()
        && label_run.text.starts_with(char::is_whitespace)
        && raised_marker_geometry(marker_run, label_run)
        && witness_block(before, marker_run, label_run, true)
            .is_some_and(|(previous, suffix)| previous.checked_add(1) == Some(number) && suffix.is_none())
        && witness_block(after, marker_run, label_run, false) == Some((number, Some('a')))
}

fn ordinary(paragraph: &PdfParagraph) -> bool {
    !paragraph.is_formula
        && !paragraph.is_code_block
        && !paragraph.is_page_furniture
        && paragraph.heading_level.is_none()
        && paragraph.caption_for.is_none()
        && paragraph.layout_class.is_none()
}

fn single_run(line: &super::super::types::PdfLine) -> Option<&SegmentData> {
    let [run] = line.segments.as_slice() else { return None };
    (line.native_scripts.is_empty() && run.assigned_role.is_none() && valid_upright_run(run)).then_some(run)
}

fn valid_upright_run(run: &SegmentData) -> bool {
    run.is_unrotated()
        && run.baseline_y.is_finite()
        && [run.width, run.height, run.font_size]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        && [run.x, run.y, run.x + run.width, run.y + run.height, 2.0 * run.font_size]
            .into_iter()
            .all(f32::is_finite)
}

fn plain_single_run(paragraph: &PdfParagraph) -> Option<&SegmentData> {
    let [line] = paragraph.lines.as_slice() else {
        return None;
    };
    if paragraph.is_list_item || !ordinary(paragraph) {
        return None;
    }
    single_run(line)
}

type LegendNumber = (u16, Option<char>);

fn legend_label_number(text: &str) -> Option<LegendNumber> {
    let (prefix, title) = text.split_once(',')?;
    if !compact_legend_title(title) {
        return None;
    }
    let suffix = prefix.chars().next_back().filter(char::is_ascii_lowercase);
    let digits = suffix.map_or(prefix, |ch| prefix.strip_suffix(ch).unwrap_or(prefix));
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((digits.parse().ok()?, suffix))
}

fn compact_legend_title(title: &str) -> bool {
    let words = title.split_whitespace();
    let count = words.clone().count();
    (1..=4).contains(&count)
        && words.into_iter().all(|word| {
            word.chars().any(char::is_alphabetic) && word.chars().all(|c| c.is_alphabetic() || matches!(c, '/' | '-'))
        })
}

fn raised_marker_geometry(marker: &SegmentData, label: &SegmentData) -> bool {
    let size = label.font_size;
    let rise = marker.baseline_y - label.baseline_y;
    let (m_bottom, m_top) = marker.upright_cross_extent();
    let (l_bottom, l_top) = label.upright_cross_extent();
    marker.font_size < size * 0.8
        && (size * 0.25..=size * 0.55).contains(&rise)
        && (label.x - (marker.x + marker.width)).abs() <= size * 0.05
        && m_top.min(l_top) - m_bottom.max(l_bottom) >= marker.height * 0.5
}

const MAX_WITNESS_ROWS: usize = 12;

fn witness_block(
    paragraph: &PdfParagraph,
    marker: &SegmentData,
    label: &SegmentData,
    above: bool,
) -> Option<LegendNumber> {
    if !paragraph.is_list_item || !ordinary(paragraph) || !(2..=MAX_WITNESS_ROWS).contains(&paragraph.lines.len()) {
        return None;
    }
    let size = label.font_size;
    let mut previous_y = if above { None } else { Some(label.baseline_y) };
    let mut edge = None;
    let mut previous_code = None;
    for (index, line) in paragraph.lines.iter().enumerate() {
        let row = single_run(line)?;
        let pitch = size..=size * 2.0;
        if previous_y.is_some_and(|y| !pitch.contains(&(y - row.baseline_y))) {
            return None;
        }
        previous_y = Some(row.baseline_y);
        let (code, marker_kind) = witness_row(row, marker, label)?;
        if previous_code.is_some_and(|previous| code < previous) {
            return None;
        }
        previous_code = Some(code);
        let is_edge = if above {
            index + 1 == paragraph.lines.len()
        } else {
            index == 0
        };
        if is_edge {
            if marker_kind != Some(MarkerKind::Symbol)
                || (above && !pitch.contains(&(row.baseline_y - label.baseline_y)))
            {
                return None;
            }
            edge = Some(code);
        }
    }
    edge
}

#[derive(PartialEq, Eq)]
enum MarkerKind {
    Symbol,
    Letter,
}

fn witness_row(
    row: &SegmentData,
    marker: &SegmentData,
    label: &SegmentData,
) -> Option<(LegendNumber, Option<MarkerKind>)> {
    let size = label.font_size;
    let right_limit = label.x + label.width + size;
    if !right_limit.is_finite() || (row.font_size - size).abs() > size * 0.1 || row.x + row.width > right_limit {
        return None;
    }
    let text = row.text.trim_start();
    let dx = row.x - marker.x;
    if let Some((symbol, rest)) = text.split_once(char::is_whitespace)
        && let Some(kind) = marker_kind(symbol)
    {
        return (dx.abs() <= size * 0.2).then_some((legend_label_number(rest.trim_start())?, Some(kind)));
    }
    (size * 0.75..=size * 2.0)
        .contains(&dx)
        .then_some((legend_label_number(text)?, None))
}

fn marker_kind(text: &str) -> Option<MarkerKind> {
    if matches!(text, "*" | "•" | "·") {
        Some(MarkerKind::Symbol)
    } else if text.len() == 1 && text.as_bytes()[0].is_ascii_uppercase() {
        Some(MarkerKind::Letter)
    } else {
        None
    }
}
