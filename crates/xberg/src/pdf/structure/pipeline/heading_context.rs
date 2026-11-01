use std::collections::HashSet;

use super::{
    PdfParagraph, SegmentData, bbox_coordinates_are_finite, heading_fills_column, is_cjk_char,
    page_column_corridor_for_segments, spans_visual_line_break,
};
use crate::pdf::structure::paragraphs::source_rows_continue;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Column {
    Page,
    Left,
    Right,
}

type Following<'a> = Option<(usize, &'a PdfParagraph)>;

pub(super) struct HeadingContext<'a> {
    columns: Vec<Column>,
    left: Following<'a>,
    right: Following<'a>,
}

impl<'a> HeadingContext<'a> {
    pub(super) fn new(page: &[PdfParagraph]) -> Self {
        let ordinary: Vec<_> = page.iter().map(ordinary_segments).collect();
        let corridor = ordinary
            .iter()
            .flatten()
            .all(|segment| segment.is_unrotated())
            .then(|| page_column_corridor_for_segments(ordinary.iter().flatten().copied()))
            .flatten();
        let columns = page
            .iter()
            .zip(&ordinary)
            .map(|(paragraph, segments)| column(paragraph, segments, corridor))
            .collect();
        Self {
            columns,
            left: None,
            right: None,
        }
    }

    pub(super) fn following(&self, index: usize, page: usize, global: Following<'a>) -> Following<'a> {
        let next_page = || global.filter(|(following_page, _)| *following_page != page);
        match self.columns[index] {
            Column::Left => self.left.or_else(next_page),
            Column::Right => self.right.or_else(next_page),
            Column::Page => global,
        }
    }

    pub(super) fn observe_content(&mut self, index: usize, page: usize, paragraph: &'a PdfParagraph) {
        let following = Some((page, paragraph));
        match self.columns[index] {
            Column::Left => self.left = following,
            Column::Right => self.right = following,
            // Unknown or spanning content is a barrier; do not look past it for a more convenient body block. ~keep
            Column::Page => {
                self.left = following;
                self.right = following;
            }
        }
    }
}

fn ordinary_segments(paragraph: &PdfParagraph) -> Vec<&SegmentData> {
    if paragraph.is_page_furniture {
        return Vec::new();
    }
    let mut ordinary = Vec::new();
    for line in &paragraph.lines {
        let scripts: HashSet<_> = line
            .valid_native_scripts()
            .into_iter()
            .map(|owner| owner.script_index)
            .collect();
        ordinary.extend(
            line.segments
                .iter()
                .enumerate()
                .filter(|(index, segment)| !scripts.contains(index) && !segment.text.trim().is_empty())
                .map(|(_, segment)| segment),
        );
    }
    ordinary
}

fn column(paragraph: &PdfParagraph, ordinary: &[&SegmentData], corridor: Option<(f32, f32)>) -> Column {
    let Some((left, right)) = corridor else {
        return Column::Page;
    };
    if !paragraph.block_bbox.is_some_and(bbox_coordinates_are_finite) || ordinary.is_empty() {
        return Column::Page;
    }
    let mut common = None;
    for segment in ordinary {
        let (start, end) = segment.upright_advance_extent();
        let current = if !start.is_finite() || !end.is_finite() || end <= start {
            Column::Page
        } else if end <= left {
            Column::Left
        } else if start >= right {
            Column::Right
        } else {
            Column::Page
        };
        if common.is_some_and(|previous| previous != current) {
            return Column::Page;
        }
        common = Some(current);
    }
    common.unwrap_or(Column::Page)
}

pub(super) fn has_trusted_hyphen_continuation(candidate: &PdfParagraph, following: &PdfParagraph) -> bool {
    let Some(trailing) = candidate.lines.last().and_then(|line| line.segments.last()) else {
        return false;
    };
    let Some(leading) = following.lines.first().and_then(|line| line.segments.first()) else {
        return false;
    };
    let Some(stem) = trailing.text.trim_end().strip_suffix('-') else {
        return false;
    };
    let word = stem.split_whitespace().last().unwrap_or("");
    if word.is_empty()
        || !stem.ends_with(word)
        || !word.chars().all(char::is_alphabetic)
        || word.chars().last().is_some_and(is_cjk_char)
        || !leading
            .text
            .trim_start()
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() && !c.is_uppercase())
        || !source_rows_continue(trailing, leading)
        || !spans_visual_line_break(trailing, leading)
    {
        return false;
    }
    let (start, end) = trailing.upright_advance_extent();
    if !start.is_finite() || !end.is_finite() || end <= start {
        return false;
    }
    continuation_right_edge(following, trailing).is_some_and(|right| heading_fills_column(trailing, right))
}

fn continuation_right_edge(paragraph: &PdfParagraph, frame: &SegmentData) -> Option<f32> {
    paragraph
        .lines
        .iter()
        .flat_map(|line| &line.segments)
        .filter(|span| !span.text.trim().is_empty())
        .try_fold(f32::NEG_INFINITY, |right, span| {
            let (start, end) = span.upright_advance_extent();
            (span.has_same_rotation(frame) && start.is_finite() && end.is_finite() && end > start)
                .then_some(right.max(end))
        })
}
