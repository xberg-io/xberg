use super::source_projection::ProjectionStore;
use super::source_rows::SourceRows;
use super::{SegmentData, SegmentScriptOwner};
use crate::pdf::structure::types::{PdfLine, PdfParagraph};

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
    original: OriginalSourceGaps,
    projection: Option<ProjectionStore>,
}

impl NativeParagraphGaps {
    pub(super) fn new(
        segments: &[SegmentData],
        original_indices: &[usize],
        scripts: &std::collections::HashMap<usize, SegmentScriptOwner>,
        original_gaps: &[f32],
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
            original: OriginalSourceGaps::new(segments, &ordinary, corridor, original_gaps),
            projection: ProjectionStore::new(segments, original_indices, scripts),
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

    pub(in crate::pdf::structure::pipeline) fn observe_repair(&mut self, segment: &SegmentData, repaired: &str) {
        if let Some(projection) = &mut self.projection {
            projection.observe_repair(segment, repaired);
        }
    }

    pub(in crate::pdf::structure::pipeline) fn merge(&self, paragraphs: &mut Vec<PdfParagraph>) {
        let source_rows = self
            .projection
            .as_ref()
            .and_then(|projection| SourceRows::new(projection, paragraphs));
        super::super::merge_continuation_paragraphs_by_boundary(paragraphs, |previous, next| {
            let scope = boundary_scope(self.paragraph_scope(previous), self.paragraph_scope(next));
            scope.is_none_or(|scope| {
                super::super::paragraphs_cross_source_gap(previous, next, self.gaps(scope))
                    || self.retains_original_boundary(previous, next, scope, source_rows.as_ref())
                    || source_rows.as_ref().is_some_and(|rows| {
                        rows.has_logical_terminal_boundary(previous, next, |segment| {
                            self.segment_scope(segment) == scope
                        })
                    })
            })
        });
    }

    fn retains_original_boundary(
        &self,
        previous: &PdfParagraph,
        next: &PdfParagraph,
        scope: Scope,
        rows: Option<&SourceRows<'_>>,
    ) -> bool {
        if !super::super::paragraphs_cross_source_gap(previous, next, &self.original.provided)
            || !super::super::paragraphs_cross_source_gap(previous, next, self.original.gaps(scope))
        {
            return false;
        }
        let Some((upper, lower)) = previous.lines.last().zip(next.lines.first()) else {
            return false;
        };
        let Some(rows) = rows else {
            return false;
        };
        let Some(height) = self.original.height else {
            return false;
        };
        if !rows.actual_pair_breaks(upper, lower, height, self.original.leading) {
            return false;
        }
        let indented = rows.starts_indented_paragraph(lower, |segment| self.segment_scope(segment) == scope);
        let Some(upper) = self.ordinary_row(upper, scope) else {
            return false;
        };
        let Some(lower) = self.ordinary_row(lower, scope) else {
            return false;
        };
        let numeric_rows = [upper.as_slice(), lower.as_slice()].into_iter().all(|row| {
            crate::pdf::native::table::is_dense_numeric_row(
                row.iter().flat_map(|segment| segment.text.split_whitespace()),
            )
        });
        numeric_rows || (indented && !row_ran_out_of_room(&upper, &lower))
    }

    fn ordinary_row<'a>(&self, line: &'a PdfLine, scope: Scope) -> Option<Vec<&'a SegmentData>> {
        if line.native_source_origin.is_none() || !line.baseline_y.is_finite() {
            return None;
        }
        let scripts: std::collections::HashSet<_> = line
            .valid_native_scripts()
            .into_iter()
            .map(|owner| owner.script_index)
            .collect();
        let ordinary: Vec<_> = line
            .segments
            .iter()
            .enumerate()
            .filter(|(index, segment)| !scripts.contains(index) && !segment.text.trim().is_empty())
            .map(|(_, segment)| segment)
            .collect();
        let first = *ordinary.first()?;
        ordinary
            .iter()
            .all(|segment| {
                finite_source_segment(segment)
                    && !segment
                        .text
                        .chars()
                        .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
                    && self.segment_scope(segment) == scope
                    && super::same_ordinary_row(first, segment)
            })
            .then_some(ordinary)
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

fn row_ran_out_of_room(upper: &[&SegmentData], lower: &[&SegmentData]) -> bool {
    let upper_end = upper
        .iter()
        .copied()
        .max_by(|left, right| {
            left.upright_advance_extent()
                .1
                .total_cmp(&right.upright_advance_extent().1)
        })
        .expect("ordinary source row is nonempty");
    let lower_right = lower
        .iter()
        .map(|segment| segment.upright_advance_extent().1)
        .fold(f32::NEG_INFINITY, f32::max);
    super::super::heading_line_ran_out_of_room(upper_end, lower[0], lower_right)
}

#[derive(Default)]
struct OriginalSourceGaps {
    height: Option<f32>,
    leading: f32,
    provided: Vec<f32>,
    page: Vec<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl OriginalSourceGaps {
    fn new(segments: &[SegmentData], ordinary: &[&SegmentData], corridor: Option<(f32, f32)>, gaps: &[f32]) -> Self {
        if gaps.is_empty()
            || segments
                .iter()
                .any(|segment| !segment.y.is_finite() || !segment.height.is_finite() || segment.height <= 0.0)
        {
            return Self::default();
        }
        let bands = super::super::paragraph_line_bands(segments.iter());
        let Some(height) = super::super::median_line_height(&bands).filter(|height| *height > 0.0) else {
            return Self::default();
        };
        let leading = super::super::body_leading(&bands, height);
        let mut provided: Vec<_> = gaps.iter().copied().filter(|gap| gap.is_finite()).collect();
        provided.sort_by(f32::total_cmp);
        Self {
            height: Some(height),
            leading,
            provided,
            page: original_scope_gaps(ordinary, corridor, Scope::Page, height, leading),
            left: original_scope_gaps(ordinary, corridor, Scope::Left, height, leading),
            right: original_scope_gaps(ordinary, corridor, Scope::Right, height, leading),
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

fn original_scope_gaps(
    ordinary: &[&SegmentData],
    corridor: Option<(f32, f32)>,
    scope: Scope,
    height: f32,
    leading: f32,
) -> Vec<f32> {
    let local: Vec<_> = ordinary
        .iter()
        .copied()
        .filter(|segment| segment_scope(segment, corridor) == scope)
        .collect();
    if !local.iter().all(|segment| finite_source_segment(segment)) {
        return Vec::new();
    }
    // A foreign page-wide marker is not proof of a gap between this column's own rows. ~keep
    let bands = super::super::paragraph_line_bands(local.into_iter());
    let mut gaps = super::super::paragraph_gaps_with_leading(&bands, height, leading);
    gaps.sort_by(f32::total_cmp);
    gaps
}

pub(super) fn finite_source_segment(segment: &SegmentData) -> bool {
    let (start, end) = segment.upright_advance_extent();
    segment.is_unrotated()
        && [
            segment.y,
            segment.height,
            segment.baseline_y,
            segment.font_size,
            start,
            end,
        ]
        .into_iter()
        .all(f32::is_finite)
        && segment.font_size > 0.0
        && segment.height > 0.0
        && end > start
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
