use super::{SegmentData, SegmentScriptOwner};
use crate::pdf::structure::types::PdfParagraph;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Page,
    Left,
    Right,
}

pub(in crate::pdf::structure::pipeline) struct NativeParagraphGaps {
    corridor: Option<(f32, f32)>,
    page: Vec<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl NativeParagraphGaps {
    pub(super) fn new(
        segments: &[SegmentData],
        scripts: &std::collections::HashMap<usize, SegmentScriptOwner>,
    ) -> Option<Self> {
        if scripts.is_empty() || !segments.iter().all(SegmentData::is_unrotated) {
            return None;
        }
        let ordinary: Vec<_> = segments
            .iter()
            .enumerate()
            .filter(|(index, _)| !scripts.contains_key(index))
            .map(|(_, segment)| segment)
            .collect();
        let corridor = super::super::page_column_corridor_for_segments(ordinary.iter().copied());
        let mut plan = Self {
            corridor,
            page: calibrated_gaps(ordinary.iter().copied()),
            left: Vec::new(),
            right: Vec::new(),
        };
        if corridor.is_some() {
            plan.left = calibrated_gaps(
                ordinary
                    .iter()
                    .copied()
                    .filter(|s| plan.segment_scope(s) != Scope::Right),
            );
            plan.right = calibrated_gaps(
                ordinary
                    .iter()
                    .copied()
                    .filter(|s| plan.segment_scope(s) != Scope::Left),
            );
        }
        Some(plan)
    }

    pub(super) fn crosses_rows(&self, previous: &SegmentData, next: &SegmentData) -> bool {
        match boundary_scope(self.segment_scope(previous), self.segment_scope(next)) {
            Some(scope) => super::super::baselines_cross_source_gap(
                previous.upright_baseline(),
                next.upright_baseline(),
                self.gaps(scope),
            ),
            None => true,
        }
    }

    pub(in crate::pdf::structure::pipeline) fn merge(&self, paragraphs: &mut Vec<PdfParagraph>) {
        super::super::merge_continuation_paragraphs_by_boundary(paragraphs, |previous, next| {
            let scope = boundary_scope(self.paragraph_scope(previous), self.paragraph_scope(next));
            scope.is_none_or(|scope| super::super::paragraphs_cross_source_gap(previous, next, self.gaps(scope)))
        });
    }

    fn segment_scope(&self, segment: &SegmentData) -> Scope {
        let Some((left, right)) = self.corridor else {
            return Scope::Page;
        };
        let (start, end) = segment.upright_advance_extent();
        if end <= left {
            Scope::Left
        } else if start >= right {
            Scope::Right
        } else {
            Scope::Page
        }
    }

    fn paragraph_scope(&self, paragraph: &PdfParagraph) -> Scope {
        let mut scopes = paragraph
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .filter(|segment| !segment.text.trim().is_empty())
            .map(|segment| self.segment_scope(segment));
        let first = scopes.next().unwrap_or(Scope::Page);
        if scopes.all(|scope| scope == first) {
            first
        } else {
            Scope::Page
        }
    }

    fn gaps(&self, scope: Scope) -> &[f32] {
        match scope {
            Scope::Page => &self.page,
            Scope::Left => &self.left,
            Scope::Right => &self.right,
        }
    }
}

fn boundary_scope(previous: Scope, next: Scope) -> Option<Scope> {
    match (previous, next) {
        (Scope::Left, Scope::Right) | (Scope::Right, Scope::Left) => None,
        (left, right) if left == right => Some(left),
        _ => Some(Scope::Page),
    }
}

fn calibrated_gaps<'a>(segments: impl Iterator<Item = &'a SegmentData>) -> Vec<f32> {
    let bands = super::super::paragraph_line_bands(segments);
    let mut gaps = super::super::paragraph_gaps_from_bands(&bands, ordinary_body_leading);
    gaps.sort_by(f32::total_cmp);
    gaps
}

fn ordinary_body_leading(lines: &[super::super::LineBand], median_height: f32) -> f32 {
    // Fraction bars and stacked glyphs produce sub-line advances, not body pitch. ~keep
    let leading = lines
        .windows(2)
        .map(|pair| pair[0].anchor_y - pair[1].anchor_y)
        .filter(|advance| advance.is_finite() && *advance >= median_height)
        .fold(f32::INFINITY, f32::min);
    if leading.is_finite() { leading } else { median_height }
}
