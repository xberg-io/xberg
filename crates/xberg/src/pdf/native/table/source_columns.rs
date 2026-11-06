use crate::pdf::table_reconstruct::HocrWord;
use std::cell::OnceCell;
use std::collections::BTreeMap;
use xberg_native_pdf::elements::{PathContent, PathOperation};

#[path = "source_boundaries.rs"]
mod source_boundaries;

#[path = "source_row_bands.rs"]
mod source_row_bands;
use source_row_bands::{ExactRows, RuleFrame};

const MAX_RULE_PROOF_WORK: usize = 65_536;
const MAX_SOURCE_PROOF_WORDS: usize = 1_024;
const MAX_UNIQUE_VERTICAL_RULES: usize = 4_096;

#[derive(Default)]
struct RuleWork {
    visits: usize,
}

pub(super) fn supports_page_frame(segments: &[crate::pdf::hierarchy::SegmentData]) -> bool {
    segments.iter().all(crate::pdf::hierarchy::SegmentData::is_unrotated)
}

impl RuleWork {
    fn charge(&mut self, amount: usize) -> Option<()> {
        if amount > MAX_RULE_PROOF_WORK.saturating_sub(self.visits) {
            self.visits = MAX_RULE_PROOF_WORK;
            return None;
        }
        self.visits += amount;
        Some(())
    }
}

pub(super) struct VerticalRule {
    x: f32,
    bottom: f32,
    top: f32,
    stroke_width: f32,
}

impl VerticalRule {
    fn covers_gap(&self, left_edge: f64, right_edge: f64) -> bool {
        let center = f64::from(self.x);
        let half_width = f64::from(self.stroke_width) / 2.0;
        half_width > 0.0 && center - left_edge >= half_width && right_edge - center >= half_width
    }
}

pub(super) struct PageRules<'a> {
    paths: &'a [PathContent],
    eligible: bool,
    cached: OnceCell<Vec<VerticalRule>>,
    exact_rows: Option<ExactRows<'a>>,
}

impl<'a> PageRules<'a> {
    pub(super) fn new(paths: &'a [PathContent], eligible: bool) -> Self {
        Self {
            paths,
            eligible,
            cached: OnceCell::new(),
            exact_rows: None,
        }
    }

    pub(super) fn with_source_words(
        mut self,
        segments: &'a [crate::pdf::hierarchy::SegmentData],
        words: &'a [HocrWord],
        sources: &'a [Option<usize>],
    ) -> Self {
        self.exact_rows = Some(ExactRows::new(segments, words, sources));
        self
    }

    fn rules(&self) -> &[VerticalRule] {
        if !self.eligible {
            return &[];
        }
        self.cached.get_or_init(|| vertical_rules(self.paths))
    }
}

pub(super) fn vertical_rules(paths: &[PathContent]) -> Vec<VerticalRule> {
    vertical_rules_with_work(paths, &mut RuleWork::default()).unwrap_or_default()
}

fn vertical_rules_with_work(paths: &[PathContent], work: &mut RuleWork) -> Option<Vec<VerticalRule>> {
    if paths.len() > MAX_RULE_PROOF_WORK {
        return None;
    }
    let mut unique = BTreeMap::new();
    for path in paths {
        work.charge(1)?;
        if let Some(rule) = painted_vertical_rule(path) {
            let key = (
                rule.x.to_bits(),
                rule.bottom.to_bits(),
                rule.top.to_bits(),
                rule.stroke_width.to_bits(),
            );
            if !unique.contains_key(&key) && unique.len() >= MAX_UNIQUE_VERTICAL_RULES {
                return None;
            }
            unique.entry(key).or_insert(rule);
        }
    }
    let mut rules: Vec<_> = unique.into_values().collect();
    rules.sort_by(|left, right| left.x.total_cmp(&right.x));
    Some(rules)
}

fn painted_vertical_rule(path: &PathContent) -> Option<VerticalRule> {
    if path.matrix.is_some() || path.layer.is_some() || !path.has_stroke() || !path.stroke_width.is_finite() {
        return None;
    }
    if path.stroke_width > crate::pdf::rules::GRID_EDGE_ALIGN_TOLERANCE_PTS {
        return None;
    }
    let color = path.stroke_color?;
    if ![color.r, color.g, color.b].iter().all(|value| value.is_finite()) {
        return None;
    }
    let [PathOperation::MoveTo(x, y), PathOperation::LineTo(other_x, other_y)] = path.operations.as_slice() else {
        return None;
    };
    let coordinates = [
        *x,
        *y,
        *other_x,
        *other_y,
        path.bbox.x,
        path.bbox.y,
        path.bbox.width,
        path.bbox.height,
    ];
    if x != other_x || y == other_y || !coordinates.iter().all(|value| value.is_finite()) {
        return None;
    }
    Some(VerticalRule {
        x: *x,
        bottom: y.min(*other_y),
        top: y.max(*other_y),
        stroke_width: path.stroke_width,
    })
}

fn column_gap_limit_with_work(
    words: &[HocrWord],
    frame: RuleFrame<'_>,
    rules: &[VerticalRule],
    work: &mut RuleWork,
) -> Option<u32> {
    if !frame.page_height.is_finite() || rules.is_empty() || words.len() > MAX_SOURCE_PROOF_WORDS {
        return None;
    }
    for word in words {
        work.charge(word.text.len().saturating_add(1))?;
    }
    let mut ordered: Vec<_> = words.iter().collect();
    ordered.sort_by_key(|word| (word.top, word.left));
    let mut evidence = BTreeMap::new();
    for pair in ordered.windows(2) {
        work.charge(1)?;
        let (left, right) = (pair[0], pair[1]);
        if !same_numeric_row(left, right) {
            continue;
        }
        for rule in rules_in_gap(left, right, rules) {
            work.charge(1)?;
            if separates_pair(rule, left, right, frame) {
                // ~keep Clustering accepts equality, so the threshold must be strictly below this ruled-cell separation.
                let gap = right.left.saturating_sub(left.left).saturating_sub(1);
                let (first_row, repeated, minimum) = evidence.entry(rule.x.to_bits()).or_insert((left.top, false, gap));
                *repeated |= *first_row != left.top;
                *minimum = (*minimum).min(gap);
            }
        }
    }
    evidence
        .into_values()
        .filter_map(|(_, repeated, gap)| repeated.then_some(gap))
        .min()
}

fn rules_in_gap<'a>(left: &HocrWord, right: &HocrWord, rules: &'a [VerticalRule]) -> &'a [VerticalRule] {
    let left_edge = f64::from(left.left) + f64::from(left.width);
    let right_edge = f64::from(right.left);
    let start = rules.partition_point(|rule| f64::from(rule.x) < left_edge);
    let end = rules.partition_point(|rule| f64::from(rule.x) <= right_edge);
    rules.get(start..end).unwrap_or_default()
}

fn separates_pair(rule: &VerticalRule, left: &HocrWord, right: &HocrWord, frame: RuleFrame<'_>) -> bool {
    left.top == right.top
        && left.height == right.height
        && left.height > 0
        && left.width > 0
        && right.width > 0
        && rule.covers_gap(f64::from(left.left) + f64::from(left.width), f64::from(right.left))
        && frame.covers_pair(rule, left, right)
}

fn same_numeric_row(left: &HocrWord, right: &HocrWord) -> bool {
    left.top == right.top
        && left.height == right.height
        && left.height > 0
        && left.width > 0
        && right.width > 0
        && crate::table_core::text_is_value(&left.text)
        && crate::table_core::text_is_value(&right.text)
}

pub(super) fn reconstruct_region_table(
    region: &[HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
    page_rules: &PageRules<'_>,
) -> Option<crate::types::Table> {
    let original =
        super::reconstruct_region_table(region, page_height, page_number, allow_single_column, horizontal_rules)?;
    let rules = page_rules.rules();
    if rules.is_empty() {
        return Some(original);
    }
    let mut work = RuleWork::default();
    let attempt = |frame, work: &mut RuleWork| {
        let limit = column_gap_limit_with_work(region, frame, rules, work)?;
        let left = region.iter().map(|word| word.left).min().unwrap_or(0);
        let right = region
            .iter()
            .map(|word| word.left.saturating_add(word.width))
            .max()
            .unwrap_or(0);
        let gap = super::heuristic_column_gap(region, right.saturating_sub(left) as f32);
        if gap <= limit {
            return None;
        }
        let corrected = super::reconstruct_region_table_with_column_gap(
            region,
            page_height,
            page_number,
            allow_single_column,
            limit,
            horizontal_rules,
        )
        .ok()?;
        (same_cell_words(&original.cells, &corrected.cells)
            && source_boundaries::preserves_source_boundaries(
                &original.cells,
                &corrected.cells,
                region,
                rules,
                frame,
                work,
            )?)
        .then_some(corrected)
    };
    let corrected = attempt(page_height.into(), &mut work).or_else(|| {
        let exact = page_rules.exact_rows.as_ref()?.frame(page_height)?;
        let corrected = attempt(exact, &mut work)?;
        source_boundaries::preserves_old_partitions(&original.cells, &corrected.cells).then_some(corrected)
    });
    Some(corrected.unwrap_or(original))
}

fn same_cell_words(original: &[Vec<String>], corrected: &[Vec<String>]) -> bool {
    original.len() == corrected.len()
        && original.iter().zip(corrected).all(|(original_row, corrected_row)| {
            original_row
                .iter()
                .flat_map(|cell| cell.split_whitespace())
                .eq(corrected_row.iter().flat_map(|cell| cell.split_whitespace()))
        })
}

#[cfg(test)]
#[path = "source_columns_tests.rs"]
mod tests;
