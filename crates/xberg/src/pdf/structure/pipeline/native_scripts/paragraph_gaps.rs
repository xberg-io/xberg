use super::{SegmentData, SegmentScriptOwner};
use crate::pdf::structure::types::PdfParagraph;

const MIN_SUPPORTED_BODY_LINE_COUNT: usize = 3;

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
        let (page, page_leading) = calibrated_gaps(ordinary.iter().copied(), None);
        let mut plan = Self {
            corridor,
            page,
            left: Vec::new(),
            right: Vec::new(),
        };
        if corridor.is_some() {
            plan.left = calibrated_gaps(
                ordinary
                    .iter()
                    .copied()
                    .filter(|s| plan.segment_scope(s) != Scope::Right),
                page_leading,
            )
            .0;
            plan.right = calibrated_gaps(
                ordinary
                    .iter()
                    .copied()
                    .filter(|s| plan.segment_scope(s) != Scope::Left),
                page_leading,
            )
            .0;
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
        let mut common = None;
        for line in &paragraph.lines {
            let scripts: std::collections::HashSet<_> = line
                .valid_native_scripts()
                .into_iter()
                .map(|owner| owner.script_index)
                .collect();
            for (index, segment) in line.segments.iter().enumerate() {
                if scripts.contains(&index) || segment.text.trim().is_empty() {
                    continue;
                }
                let scope = self.segment_scope(segment);
                if common.is_some_and(|previous| previous != scope) {
                    return Scope::Page;
                }
                common = Some(scope);
            }
        }
        common.unwrap_or(Scope::Page)
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

fn calibrated_gaps<'a>(
    segments: impl Iterator<Item = &'a SegmentData>,
    fallback_leading: Option<f32>,
) -> (Vec<f32>, Option<f32>) {
    let bands = super::super::paragraph_line_bands(segments);
    let mut observed_leading = None;
    let mut gaps = super::super::paragraph_gaps_from_bands(&bands, |lines, median_height| {
        let conservative = ordinary_body_leading(lines, median_height);
        // Page evidence may tighten an isolated local gap, never enlarge it. ~keep
        let inherited = fallback_leading.map_or(conservative, |page| page.min(conservative));
        let leading = supported_body_leading(lines, median_height)
            .unwrap_or(inherited)
            .max(median_height);
        observed_leading = Some(leading);
        leading
    });
    gaps.sort_by(f32::total_cmp);
    (gaps, observed_leading)
}

fn supported_body_leading(lines: &[super::super::LineBand], median_height: f32) -> Option<f32> {
    // Two agreeing advances distinguish repeated physical rows from an isolated paragraph gap. ~keep
    lines
        .windows(MIN_SUPPORTED_BODY_LINE_COUNT)
        .filter_map(|run| {
            let first = run[0].anchor_y - run[1].anchor_y;
            let next = run[1].anchor_y - run[2].anchor_y;
            (first.is_finite()
                && next.is_finite()
                && first >= median_height
                && next >= median_height
                && (first - next).abs() <= super::super::INLINE_STYLE_BASELINE_TOLERANCE)
                .then_some(first.min(next))
        })
        .min_by(f32::total_cmp)
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
