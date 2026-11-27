//! Horizontal-extent overlap shared by table reconstruction and the native reading order:
//! columns of overlapping extents, and the stacked header band rule built on them. A header
//! whose cells stack a different number of centred label lines interleaves the cells' lines
//! into rows closer together than a line of text; adjacent rows never overlap horizontally,
//! while a cell's own lines, two rows apart, do.

use std::cmp::Ordering;

/// Fewest rows in a stacked header band.
pub(crate) const STACKED_BAND_MIN_ROWS: usize = 3;
/// Fewest words in each row of a stacked header band.
pub(crate) const STACKED_BAND_MIN_ROW_WORDS: usize = 2;
/// Fewest columns whose label stacks two or more of the band's rows.
pub(crate) const STACKED_BAND_MIN_STACKED_COLUMNS: usize = 2;
/// Largest step between two rows of a stacked header band, as a fraction of their line height.
/// Lines of text step at least their height; pb_sample_page_16's band steps 0.63 to 0.67 of
/// its glyph height, and 0.75 once its coordinates are rounded to whole points.
pub(crate) const STACKED_BAND_MAX_ROW_STEP: f32 = 1.0;

/// A horizontal coordinate: page points or image pixels.
pub(crate) trait Coordinate: Copy + PartialOrd {
    fn order(&self, other: &Self) -> Ordering;
    fn larger(self, other: Self) -> Self;
}

impl Coordinate for f32 {
    fn order(&self, other: &Self) -> Ordering {
        self.total_cmp(other)
    }

    fn larger(self, other: Self) -> Self {
        self.max(other)
    }
}

impl Coordinate for u32 {
    fn order(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }

    fn larger(self, other: Self) -> Self {
        self.max(other)
    }
}

/// One column of overlapping extents: its span and the indices of its members.
pub(crate) struct OverlapColumn<T> {
    #[cfg_attr(not(feature = "pdf"), allow(dead_code))]
    pub(crate) left: T,
    pub(crate) right: T,
    pub(crate) members: Vec<usize>,
}

/// Group `(left, right)` extents into columns, left to right: in left-edge order (ties keep
/// their input order), an extent joins the column before it when it starts left of that
/// column's right edge. Members are indices into `extents`, in left-edge order.
pub(crate) fn overlap_columns<T: Coordinate>(extents: &[(T, T)]) -> Vec<OverlapColumn<T>> {
    let mut order: Vec<usize> = (0..extents.len()).collect();
    order.sort_by(|&first, &second| extents[first].0.order(&extents[second].0));
    let mut columns: Vec<OverlapColumn<T>> = Vec::new();
    for index in order {
        let (left, right) = extents[index];
        match columns.last_mut() {
            Some(column) if left < column.right => {
                column.members.push(index);
                column.right = column.right.larger(right);
            }
            _ => columns.push(OverlapColumn {
                left,
                right,
                members: vec![index],
            }),
        }
    }
    columns
}

/// Whether the horizontal ranges the two rows cover, leftmost left edge to rightmost right
/// edge, intersect: the guard that keeps two blocks set side by side out of a stacked header
/// band. A band's cells share one header, so each row's labels sit between the other's; rows
/// of two tables standing side by side cover disjoint ranges, and where the two tables' rows
/// sit at offset baselines (pb_fqr-retail-blackrock page 8) the rows alternate between the
/// tables less than a line apart and never overlap, which alone read as a band.
pub(crate) fn ranges_intersect<T: Coordinate>(
    upper: impl IntoIterator<Item = (T, T)>,
    lower: impl IntoIterator<Item = (T, T)>,
) -> bool {
    match (covered_range(upper), covered_range(lower)) {
        (Some((left, right)), Some((other_left, other_right))) => left < other_right && other_left < right,
        _ => false,
    }
}

fn covered_range<T: Coordinate>(extents: impl IntoIterator<Item = (T, T)>) -> Option<(T, T)> {
    extents.into_iter().reduce(|(left, right), (other_left, other_right)| {
        let left = if other_left.order(&left) == Ordering::Less {
            other_left
        } else {
            left
        };
        (left, right.larger(other_right))
    })
}

/// Whether an extent of `upper` overlaps an extent of `lower` horizontally: the guard that
/// keeps two adjacent rows of one column out of a stacked header band.
pub(crate) fn extents_overlap<T: Coordinate>(
    upper: impl IntoIterator<Item = (T, T)>,
    lower: impl Iterator<Item = (T, T)> + Clone,
) -> bool {
    upper.into_iter().any(|(left, right)| {
        lower
            .clone()
            .any(|(other_left, other_right)| left < other_right && other_left < right)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_extents_share_a_column_in_left_edge_order() {
        let columns = overlap_columns(&[(30.0_f32, 40.0), (0.0, 10.0), (35.0, 50.0), (5.0, 8.0), (10.0, 12.0)]);
        let spans: Vec<(f32, f32, Vec<usize>)> = columns
            .into_iter()
            .map(|column| (column.left, column.right, column.members))
            .collect();
        assert_eq!(
            spans,
            [(0.0, 10.0, vec![1, 3]), (10.0, 12.0, vec![4]), (30.0, 50.0, vec![0, 2])],
            "an extent starting at a column's right edge starts a new column"
        );
    }

    #[test]
    fn row_ranges_intersect_when_the_rows_interleave() {
        assert!(ranges_intersect(
            [(25.0_f32, 40.0), (150.0, 170.0)],
            [(71.0, 89.0), (95.0, 113.0)]
        ));
        assert!(
            !ranges_intersect([(18.0_f32, 126.6), (200.1, 282.0)], [(309.0, 417.6), (482.0, 573.0)]),
            "rows of two tables side by side cover disjoint ranges"
        );
        assert!(!ranges_intersect([(0_u32, 10)], [(10_u32, 20)]));
        assert!(!ranges_intersect(Vec::<(u32, u32)>::new(), [(0_u32, 10)]));
    }

    #[test]
    fn rows_overlap_only_when_an_extent_pair_intersects() {
        let upper = [(0_u32, 10), (20, 30)];
        assert!(extents_overlap(upper, [(9_u32, 15)].into_iter()));
        assert!(!extents_overlap(upper, [(10_u32, 20), (30, 40)].into_iter()));
    }
}
