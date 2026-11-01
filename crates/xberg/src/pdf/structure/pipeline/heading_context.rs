use std::collections::HashSet;

use super::{PdfParagraph, SegmentData, bbox_coordinates_are_finite, page_column_corridor_for_segments};

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
