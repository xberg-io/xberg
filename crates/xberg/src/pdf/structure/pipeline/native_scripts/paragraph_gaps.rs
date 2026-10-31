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
            page: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
        };
        let inherited = plan.calibrate_columns(&ordinary);
        plan.page = ColumnBands::new(ordinary.iter().copied(), Scope::Page, corridor).gaps(inherited);
        Some(plan)
    }

    fn calibrate_columns(&mut self, ordinary: &[&SegmentData]) -> Option<f32> {
        self.corridor?;
        let left = ColumnBands::new(
            ordinary
                .iter()
                .copied()
                .filter(|s| self.segment_scope(s) != Scope::Right),
            Scope::Left,
            self.corridor,
        );
        let right = ColumnBands::new(
            ordinary
                .iter()
                .copied()
                .filter(|s| self.segment_scope(s) != Scope::Left),
            Scope::Right,
            self.corridor,
        );
        let inherited = minimum_pitch(left.pitch.minimum, right.pitch.minimum);
        self.left = left.gaps(inherited);
        self.right = right.gaps(inherited);
        inherited
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
        segment_scope(segment, self.corridor)
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

fn segment_scope(segment: &SegmentData, corridor: Option<(f32, f32)>) -> Scope {
    extent_scope(segment.upright_advance_extent(), corridor)
}

fn extent_scope((start, end): (f32, f32), corridor: Option<(f32, f32)>) -> Scope {
    let Some((left, right)) = corridor else {
        return Scope::Page;
    };
    if end <= left {
        Scope::Left
    } else if start >= right {
        Scope::Right
    } else {
        Scope::Page
    }
}

#[derive(Clone, Copy, Default)]
struct BodyPitch {
    minimum: Option<f32>,
    repeated: bool,
}

struct ColumnBands<'a> {
    lines: Vec<super::super::LineBand<'a>>,
    median_height: Option<f32>,
    pitch: BodyPitch,
}

impl<'a> ColumnBands<'a> {
    fn new(segments: impl Iterator<Item = &'a SegmentData>, scope: Scope, corridor: Option<(f32, f32)>) -> Self {
        let lines = super::super::paragraph_line_bands(segments);
        let median_height = super::super::median_line_height(&lines);
        let pitch = median_height.map_or_else(BodyPitch::default, |height| body_pitch(&lines, height, scope, corridor));
        Self {
            lines,
            median_height,
            pitch,
        }
    }

    fn gaps(self, inherited: Option<f32>) -> Vec<f32> {
        let Some(height) = self.median_height else {
            return Vec::new();
        };
        let observed = self
            .pitch
            .minimum
            .unwrap_or_else(|| ordinary_body_leading(&self.lines, height));
        // Foreign evidence can tighten sparse columns, but never replace a smaller body observation. ~keep
        let leading = if self.pitch.repeated {
            observed
        } else {
            minimum_pitch(Some(observed), inherited).unwrap_or(observed)
        };
        let mut gaps = super::super::paragraph_gaps_with_leading(&self.lines, height, leading.max(height));
        gaps.sort_by(f32::total_cmp);
        gaps
    }
}

fn minimum_pitch(left: Option<f32>, right: Option<f32>) -> Option<f32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    }
}

fn body_pair_continues(pair: &[super::super::LineBand<'_>], scope: Scope, corridor: Option<(f32, f32)>) -> bool {
    pair.iter().all(|line| {
        line.source.ordinary
            && extent_scope(
                (
                    line.source.first.upright_advance_extent().0,
                    line.source.last.upright_advance_extent().1,
                ),
                corridor,
            ) == scope
    }) && super::super::super::paragraphs::source_rows_continue(pair[0].source.last, pair[1].source.first)
}

fn body_frame(lines: &[super::super::LineBand<'_>], scope: Scope, corridor: Option<(f32, f32)>) -> Option<(f32, f32)> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for pair in lines
        .windows(2)
        .filter(|pair| body_pair_continues(pair, scope, corridor))
    {
        for line in pair {
            left = left.min(line.source.first.upright_advance_extent().0);
            right = right.max(line.source.last.upright_advance_extent().1);
        }
    }
    (left.is_finite() && right.is_finite() && right > left).then_some((left, right))
}

fn body_pitch(
    lines: &[super::super::LineBand<'_>],
    median_height: f32,
    scope: Scope,
    corridor: Option<(f32, f32)>,
) -> BodyPitch {
    let Some(frame) = body_frame(lines, scope, corridor) else {
        return BodyPitch::default();
    };
    let mut pitch = BodyPitch::default();
    let mut previous: Option<f32> = None;
    for pair in lines.windows(2) {
        let advance = body_advance(pair, median_height, scope, corridor, frame);
        if let Some(advance) = advance {
            pitch.minimum = minimum_pitch(pitch.minimum, Some(advance));
            pitch.repeated |= previous
                .is_some_and(|previous| (previous - advance).abs() <= super::super::INLINE_STYLE_BASELINE_TOLERANCE);
        }
        // An unqualified physical pair breaks consecutive support. ~keep
        previous = advance;
    }
    pitch
}

fn body_advance(
    pair: &[super::super::LineBand<'_>],
    median_height: f32,
    scope: Scope,
    corridor: Option<(f32, f32)>,
    frame: (f32, f32),
) -> Option<f32> {
    let upper = &pair[0].source;
    let lower = &pair[1].source;
    let advance = pair[0].anchor_y - pair[1].anchor_y;
    let left_delta = (upper.first.upright_advance_extent().0 - lower.first.upright_advance_extent().0).abs();
    let tolerance = super::super::HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR
        * upper.first.font_size.max(lower.first.font_size).max(1.0);
    (advance.is_finite()
        && advance >= median_height
        && body_pair_continues(pair, scope, corridor)
        && super::super::heading_fills_column(upper.last, frame.1)
        && left_delta <= tolerance)
        .then_some(advance)
}

fn ordinary_body_leading(lines: &[super::super::LineBand<'_>], median_height: f32) -> f32 {
    // Fraction bars and stacked glyphs produce sub-line advances, not body pitch. ~keep
    let leading = lines
        .windows(2)
        .map(|pair| pair[0].anchor_y - pair[1].anchor_y)
        .filter(|advance| advance.is_finite() && *advance >= median_height)
        .fold(f32::INFINITY, f32::min);
    if leading.is_finite() { leading } else { median_height }
}
