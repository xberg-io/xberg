use std::collections::HashSet;

use super::SegmentData;
use super::source_projection::{ProjectionStore, ProjectionView};
use crate::pdf::structure::types::{PdfLine, PdfParagraph};

struct Row {
    members: HashSet<usize>,
    first: usize,
    last: usize,
    top: f32,
    bottom: f32,
    height: f32,
    monospace: bool,
}

pub(super) struct SourceRows<'a> {
    view: ProjectionView<'a>,
    rows: Vec<Row>,
    membership: Vec<Option<usize>>,
    complete: Vec<bool>,
}

impl<'a> SourceRows<'a> {
    pub(super) fn new(store: &'a ProjectionStore, paragraphs: &[PdfParagraph]) -> Option<Self> {
        let view = store.view(paragraphs);
        if !view.records().iter().all(|record| {
            super::paragraph_gaps::finite_source_segment(&record.segment)
                && (record.segment.y + record.segment.height).is_finite()
        }) {
            return None;
        }
        let (rows, membership) = physical_rows(&view, paragraphs)?;
        let complete = vec![false; rows.len()];
        let mut result = Self {
            view,
            rows,
            membership,
            complete,
        };
        for line in paragraphs.iter().flat_map(|paragraph| &paragraph.lines) {
            if let Some(row) = result.complete_row(line) {
                result.complete[row] = true;
            }
        }
        Some(result)
    }

    pub(super) fn complete_row(&self, line: &PdfLine) -> Option<usize> {
        line.native_source_origin?;
        if !line.baseline_y.is_finite() {
            return None;
        }
        let scripts: HashSet<_> = proved_scripts(&self.view, line)
            .iter()
            .map(|owner| owner.script_index)
            .collect();
        let mut members = HashSet::new();
        let mut row = None;
        for (_, segment) in line
            .segments
            .iter()
            .enumerate()
            .filter(|(index, _)| !scripts.contains(index))
        {
            let slot = self.view.resolve(segment)?;
            let current = self.membership[slot]?;
            if row.is_some_and(|previous| previous != current) {
                return None;
            }
            row = Some(current);
            members.insert(slot);
        }
        let row = row?;
        let expected = &self.rows[row];
        if members != expected.members {
            return None;
        }
        let first = self.segment(expected.first);
        (members.iter().all(|&slot| {
            !self
                .segment(slot)
                .text
                .chars()
                .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
        }) && members
            .iter()
            .all(|&slot| super::same_ordinary_row(first, self.segment(slot)))
            && (line.baseline_y - first.upright_baseline()).abs() <= super::super::INLINE_STYLE_BASELINE_TOLERANCE)
            .then_some(row)
    }

    pub(super) fn actual_pair_breaks(&self, upper: &PdfLine, lower: &PdfLine, height: f32, leading: f32) -> bool {
        let Some((a, b)) = self.complete_row(upper).zip(self.complete_row(lower)) else {
            return false;
        };
        if a == b {
            return false;
        }
        let bands = [
            self.band(a, self.segment(self.rows[a].last).upright_baseline()),
            self.band(b, self.segment(self.rows[b].first).upright_baseline()),
        ];
        (bands[0].anchor_y - bands[1].anchor_y).is_finite()
            && !super::super::paragraph_gaps_with_leading(&bands, height, leading).is_empty()
    }

    pub(super) fn starts_indented_paragraph(&self, line: &PdfLine, in_scope: impl Fn(&SegmentData) -> bool) -> bool {
        let Some(row) = self.complete_row(line) else {
            return false;
        };
        let Some(next) = self.rows.get(row + 1).filter(|_| self.complete[row + 1]) else {
            return false;
        };
        let current = &self.rows[row];
        if !current
            .members
            .iter()
            .chain(&next.members)
            .all(|&slot| in_scope(self.segment(slot)))
        {
            return false;
        }
        if !self.view.text_unchanged(current.last) || !self.view.text_unchanged(next.first) {
            return false;
        }
        let first = self.segment(current.first);
        let last = self.segment(current.last);
        let following = self.segment(next.first);
        let tolerance = super::super::HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR
            * first.font_size.max(following.font_size).max(1.0);
        first.upright_advance_extent().0 - following.upright_advance_extent().0 > tolerance
            && super::super::super::paragraphs::source_rows_continue(last, following)
            && super::super::heading_fills_column(last, self.segment(next.last).upright_advance_extent().1)
    }

    fn segment(&self, slot: usize) -> &SegmentData {
        &self.view.records()[slot].segment
    }

    fn band(&self, row: usize, baseline: f32) -> super::super::LineBand<'_> {
        let source = &self.rows[row];
        super::super::LineBand {
            top: source.top,
            bottom: source.bottom,
            height: source.height,
            anchor_y: baseline,
            monospace: source.monospace,
            source: super::super::LineBandSource {
                first: self.segment(source.first),
                last: self.segment(source.last),
                ordinary: true,
            },
        }
    }
}

fn valid_script_slots(view: &ProjectionView<'_>, paragraphs: &[PdfParagraph]) -> HashSet<usize> {
    paragraphs
        .iter()
        .flat_map(|p| &p.lines)
        .flat_map(|line| {
            proved_scripts(view, line)
                .into_iter()
                .filter_map(move |owner| view.script_slot(owner, line))
        })
        .collect()
}

fn proved_scripts<'a>(view: &ProjectionView<'_>, line: &'a PdfLine) -> Vec<&'a super::SegmentScriptOwner> {
    super::complete_script_runs(
        line.valid_native_scripts()
            .into_iter()
            .filter(|owner| view.script_slot(owner, line).is_some())
            .collect(),
    )
}

type Membership = Vec<Option<usize>>;

fn physical_rows(view: &ProjectionView<'_>, paragraphs: &[PdfParagraph]) -> Option<(Vec<Row>, Membership)> {
    let scripts = valid_script_slots(view, paragraphs);
    let order = source_order(view, &scripts);
    let bands = super::super::paragraph_line_bands(order.iter().map(|&slot| &view.records()[slot].segment));
    let mut rows: Vec<_> = bands
        .iter()
        .map(|band| Row {
            members: HashSet::new(),
            first: 0,
            last: 0,
            top: band.top,
            bottom: band.bottom,
            height: band.height,
            monospace: band.monospace,
        })
        .collect();
    let mut membership = vec![None; view.records().len()];
    let mut row = 0;
    for slot in order {
        let segment = &view.records()[slot].segment;
        while row < bands.len() && !super::super::segment_matches_paragraph_band(segment, bands[row].anchor_y) {
            row += 1;
        }
        rows.get_mut(row)?.include(slot, view);
        membership[slot] = Some(row);
    }
    drop(bands);
    Some((rows, membership))
}

fn source_order(view: &ProjectionView<'_>, scripts: &HashSet<usize>) -> Vec<usize> {
    let mut order: Vec<_> = (0..view.records().len())
        .filter(|slot| !scripts.contains(slot))
        .collect();
    order.sort_by(|&left, &right| {
        view.records()[right]
            .segment
            .y
            .partial_cmp(&view.records()[left].segment.y)
            .unwrap()
    });
    order
}

impl Row {
    fn include(&mut self, slot: usize, view: &ProjectionView<'_>) {
        let segment = &view.records()[slot].segment;
        if self.members.is_empty() {
            self.first = slot;
            self.last = slot;
        }
        if segment.upright_advance_extent().0 < view.records()[self.first].segment.upright_advance_extent().0 {
            self.first = slot;
        }
        if segment.upright_advance_extent().1 > view.records()[self.last].segment.upright_advance_extent().1 {
            self.last = slot;
        }
        self.members.insert(slot);
    }
}
