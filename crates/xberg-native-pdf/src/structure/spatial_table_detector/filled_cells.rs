use super::{PaintedCellOwnership, Table, TableDetectionConfig, TextSpan};
use crate::elements::{PathContent, PathOperation};
use crate::geometry::Rect;

mod record_bands;

pub(super) use record_bands::preserve as preserve_record_bands;

const MAX_PATHS: usize = 1_024;
const MAX_WORDS: usize = 4_096;
const MAX_WORK: usize = 131_072;
const EDGE_ROUNDOFF_EPSILONS: f32 = 2.;

pub(super) struct Prepared<'a> {
    pub(super) paths: Vec<PathContent>,
    words: Vec<Vec<&'a TextSpan>>,
    columns: usize,
    cells: Vec<Rect>,
    tolerance: f32,
    work: Work,
}

impl Prepared<'_> {
    pub(super) fn accepts(&mut self, tables: &[Table]) -> bool {
        let [table] = tables else { return false };
        if table.col_count != self.columns || table.rows.len() != self.words.len() / self.columns {
            return false;
        }
        let conserved = table
            .rows
            .iter()
            .zip(self.words.chunks_exact(self.columns))
            .all(|(row, expected)| {
                row.cells.len() == self.columns
                    && row
                        .cells
                        .iter()
                        .zip(expected)
                        .all(|(cell, words)| same_cell(cell, words, &mut self.work) == Some(true))
            });
        conserved && table.is_real_grid() && !crate::document::looks_like_prose_table(table)
    }

    pub(super) fn overhang_ownership(&self, table: &Table) -> Option<PaintedCellOwnership> {
        let words = table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .zip(self.cells.iter().zip(&self.words))
            .map(|(cell, (&paint, words))| {
                let [word] = words.as_slice() else { return None };
                certified_overhang(cell, paint, word, self.tolerance)
            })
            .collect::<Vec<_>>();
        words.iter().any(Option::is_some).then_some(PaintedCellOwnership {
            columns: self.columns,
            words,
            recovered_bounds: None,
            double_rule: false,
            replaces_empty_ruled_route: false,
        })
    }
}

fn certified_overhang(cell: &super::TableCell, paint: Rect, word: &TextSpan, tolerance: f32) -> Option<Rect> {
    let bounds = word.bbox;
    let (whole, fraction) = word.text.split_once('.')?;
    (cell.rowspan == 1
        && cell.colspan == 1
        && cell.text == word.text
        && !whole.is_empty()
        && !fraction.is_empty()
        && whole.bytes().chain(fraction.bytes()).all(|byte| byte.is_ascii_digit())
        && cell.bbox.is_some_and(|cell| same_bounds(cell, paint))
        && paint.left() <= bounds.left()
        && bounds.right() <= paint.right()
        && paint.top() < bounds.top()
        && bounds.top() < paint.bottom()
        && bounds.bottom() > paint.bottom()
        && bounds.bottom() - paint.bottom() <= tolerance
        && owns_word(paint, bounds, tolerance))
    .then_some(bounds)
}

fn same_cell(cell: &super::TableCell, words: &[&TextSpan], work: &mut Work) -> Option<bool> {
    work.step()?;
    if cell.spans.len() != words.len() {
        return Some(false);
    }
    for (actual, original) in cell.spans.iter().zip(words) {
        work.step()?;
        work.spend(actual.text.len())?;
        work.spend(actual.font_name.len())?;
        if actual.text != original.text
            || actual.bbox != original.bbox
            || actual.mcid != original.mcid
            || actual.font_name != original.font_name
            || actual.font_size != original.font_size
            || actual.font_weight != original.font_weight
            || actual.is_italic != original.is_italic
        {
            return Some(false);
        }
    }
    work.spend(cell.text.len())?;
    let visible = cell.text.chars().filter(|character| !character.is_whitespace());
    let source = words
        .iter()
        .flat_map(|span| span.text.chars())
        .filter(|character| !character.is_whitespace());
    Some(visible.eq(source))
}

struct Work(usize);

impl Work {
    fn new() -> Self {
        Self(MAX_WORK)
    }

    fn step(&mut self) -> Option<()> {
        self.spend(1)
    }

    fn spend(&mut self, units: usize) -> Option<()> {
        self.0 = self.0.checked_sub(units)?;
        Some(())
    }
}

fn same(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON * left.abs().max(right.abs()).max(1.) * EDGE_ROUNDOFF_EPSILONS
}

fn finite(bounds: Rect) -> bool {
    bounds.left() < bounds.right()
        && bounds.top() < bounds.bottom()
        && [bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
            .iter()
            .all(|value| value.is_finite())
}

fn contains(outer: Rect, inner: Rect) -> bool {
    (outer.left() <= inner.left() || same(outer.left(), inner.left()))
        && (outer.right() >= inner.right() || same(outer.right(), inner.right()))
        && (outer.top() <= inner.top() || same(outer.top(), inner.top()))
        && (outer.bottom() >= inner.bottom() || same(outer.bottom(), inner.bottom()))
}

fn same_bounds(left: Rect, right: Rect) -> bool {
    same(left.left(), right.left())
        && same(left.right(), right.right())
        && same(left.top(), right.top())
        && same(left.bottom(), right.bottom())
}

fn axis_aligned_fill(path: &PathContent) -> bool {
    let [PathOperation::Rectangle(x, y, width, height)] = path.operations.as_slice() else {
        return false;
    };
    if path.matrix.is_some()
        || path.layer.is_some()
        || path.stroke_color.is_some()
        || path.artifact_type.is_some()
        || path.dash_pattern.is_some()
        || !finite(path.bbox)
    {
        return false;
    }
    let drawn = Rect::new(*x, *y, *width, *height);
    finite(drawn)
        && same_bounds(path.bbox, drawn)
        && path.fill_color.is_some_and(|color| {
            [color.r, color.g, color.b]
                .iter()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
        })
}

fn maximal_cells<'a>(paths: &'a [PathContent], work: &mut Work) -> Option<Vec<&'a PathContent>> {
    let mut cells = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        work.step()?;
        let mut nested = false;
        for (other_index, other) in paths.iter().enumerate() {
            work.step()?;
            if index == other_index || !contains(other.bbox, path.bbox) {
                continue;
            }
            if same_bounds(other.bbox, path.bbox) {
                if path.fill_color != other.fill_color {
                    return None;
                }
                nested |= other_index < index;
            } else {
                nested = true;
            }
        }
        if !nested {
            cells.push(path);
        }
    }
    cells.sort_by(|left, right| {
        right
            .bbox
            .top()
            .total_cmp(&left.bbox.top())
            .then_with(|| left.bbox.left().total_cmp(&right.bbox.left()))
    });
    Some(cells)
}

fn tiled_rows(cells: &[&PathContent], config: &TableDetectionConfig, work: &mut Work) -> Option<usize> {
    let first = cells.first()?;
    let columns = cells
        .iter()
        .take_while(|cell| same(cell.bbox.top(), first.bbox.top()))
        .count();
    if columns < config.min_table_columns.max(2)
        || columns > config.max_table_columns
        || cells.len() < config.min_table_cells
        || !cells.len().is_multiple_of(columns)
        || cells.len() / columns < 2
    {
        return None;
    }
    let mut previous_top = first.bbox.bottom();
    let mut contrasting = false;
    for row in cells.chunks_exact(columns) {
        if !same(row[0].bbox.bottom(), previous_top) {
            return None;
        }
        for (column, cell) in row.iter().enumerate() {
            work.step()?;
            let template = cells[column];
            if !same(cell.bbox.top(), row[0].bbox.top())
                || !same(cell.bbox.bottom(), row[0].bbox.bottom())
                || !same(cell.bbox.left(), template.bbox.left())
                || !same(cell.bbox.right(), template.bbox.right())
                || cell.fill_color != row[0].fill_color
                || (column > 0 && !same(row[column - 1].bbox.right(), cell.bbox.left()))
            {
                return None;
            }
        }
        contrasting |= row[0].fill_color != first.fill_color;
        previous_top = row[0].bbox.top();
    }
    contrasting.then_some(columns)
}

fn unique_owner(cells: &[&PathContent], work: &mut Work, owns: impl Fn(Rect) -> bool) -> Option<usize> {
    let mut owner = None;
    for (index, cell) in cells.iter().enumerate() {
        work.step()?;
        if owns(cell.bbox) && owner.replace(index).is_some() {
            return None;
        }
    }
    owner
}

fn owns_word(cell: Rect, word: Rect, row_tolerance: f32) -> bool {
    let anchor = word.center().y;
    cell.top() < anchor
        && anchor < cell.bottom()
        && contains(
            Rect::new(
                cell.x,
                cell.y - row_tolerance,
                cell.width,
                cell.height + 2. * row_tolerance,
            ),
            word,
        )
}

fn owned_words<'a>(
    spans: &'a [TextSpan],
    cells: &[&PathContent],
    columns: usize,
    row_tolerance: f32,
    work: &mut Work,
) -> Option<Vec<Vec<&'a TextSpan>>> {
    let region = cells.first()?.bbox.union(&cells.last()?.bbox);
    let mut owned = vec![Vec::new(); cells.len()];
    for span in spans {
        work.step()?;
        work.spend(span.text.len())?;
        work.spend(span.font_name.len())?;
        work.spend(span.char_widths.len())?;
        work.spend(span.char_x_offsets.len())?;
        if span.text.trim().is_empty() {
            continue;
        }
        if !finite(span.bbox) {
            return None;
        }
        if !span.bbox.intersects(&region) {
            continue;
        }
        if span.rotation_degrees != 0. || span.wmode != 0 || span.page_rotation_applied != 0 {
            return None;
        }
        let owner = unique_owner(cells, work, |cell| owns_word(cell, span.bbox, row_tolerance))?;
        owned[owner].push(span);
    }
    owned
        .chunks_exact(columns)
        .all(|row| row.iter().filter(|words| !words.is_empty()).count() >= 2)
        .then_some(owned)
}

fn single_line_fill(bounds: Rect, words: &[&TextSpan], tolerance: f32, work: &mut Work) -> Option<bool> {
    let mut baseline = None;
    for word in words {
        work.step()?;
        let anchor = word.bbox.center();
        if anchor.y <= bounds.top() || anchor.y >= bounds.bottom() {
            continue;
        }
        if anchor.x <= bounds.left()
            || anchor.x >= bounds.right()
            || (*baseline.get_or_insert(anchor.y) - anchor.y).abs() > tolerance
        {
            return Some(false);
        }
    }
    Some(baseline.is_some())
}

fn underline(bounds: Rect, words: &[&TextSpan], tolerance: f32, work: &mut Work) -> Option<bool> {
    let mut line: Option<Rect> = None;
    for word in words {
        work.step()?;
        let gap = word.bbox.top() - bounds.bottom();
        if (0.0..=tolerance).contains(&gap) {
            line = Some(line.map_or(word.bbox, |previous| previous.union(&word.bbox)));
        }
    }
    Some(
        line.is_some_and(|line| {
            same(line.left(), bounds.left()) && (line.right() - bounds.right()).abs() <= bounds.height
        }),
    )
}

fn remove_nested(
    paths: &[PathContent],
    cells: &[&PathContent],
    words: &[Vec<&TextSpan>],
    tolerance: f32,
    work: &mut Work,
) -> Option<Vec<PathContent>> {
    let mut retained = Vec::with_capacity(paths.len());
    for path in paths {
        work.step()?;
        let owner = unique_owner(cells, work, |cell| contains(cell, path.bbox))?;
        let cell = cells[owner];
        let bounds = path.bbox;
        let thin = path.is_horizontal_line(super::SNAP_TOL);
        let boundary = thin
            && same(bounds.left(), cell.bbox.left())
            && same(bounds.right(), cell.bbox.right())
            && (same(bounds.top(), cell.bbox.top()) || same(bounds.bottom(), cell.bbox.bottom()));
        if same_bounds(bounds, cell.bbox) || boundary {
            retained.push(path);
            continue;
        }
        let inset = path.fill_color == cell.fill_color
            && same(bounds.left(), cell.bbox.left())
            && single_line_fill(bounds, &words[owner], tolerance, work)?;
        if !inset && !(thin && underline(bounds, &words[owner], tolerance, work)?) {
            return None;
        }
    }
    (retained.len() < paths.len()).then(|| retained.into_iter().cloned().collect())
}

/// A complete, contrasting source-cell tiling owns its contained text paint. Nested fills must
/// not create another grid; uncertain frames, words, strokes, or work bounds keep the old path. ~keep
pub(super) fn prepare<'a>(
    spans: &'a [TextSpan],
    paths: &[PathContent],
    config: &TableDetectionConfig,
) -> Option<Prepared<'a>> {
    let mut work = Work::new();
    let (paths, words, columns, cells, tolerance) = prepare_with_work(spans, paths, config, &mut work)?;
    Some(Prepared {
        paths,
        words,
        columns,
        cells,
        tolerance,
        work,
    })
}

type PreparedParts<'a> = (Vec<PathContent>, Vec<Vec<&'a TextSpan>>, usize, Vec<Rect>, f32);

fn prepare_with_work<'a>(
    spans: &'a [TextSpan],
    paths: &[PathContent],
    config: &TableDetectionConfig,
    work: &mut Work,
) -> Option<PreparedParts<'a>> {
    if paths.len() < config.min_table_cells
        || paths.len() > MAX_PATHS
        || spans.len() > MAX_WORDS
        || !config.row_tolerance.is_finite()
        || config.row_tolerance < 0.
    {
        return None;
    }
    for path in paths {
        work.step()?;
        if !axis_aligned_fill(path) {
            return None;
        }
    }
    let cells = maximal_cells(paths, work)?;
    if cells.len() == paths.len() {
        return None;
    }
    let columns = tiled_rows(&cells, config, work)?;
    let tolerance = config.row_tolerance.min(super::SNAP_TOL);
    let words = owned_words(spans, &cells, columns, tolerance, work)?;
    let paths = remove_nested(paths, &cells, &words, tolerance, work)?;
    Some((
        paths,
        words,
        columns,
        cells.into_iter().map(|cell| cell.bbox).collect(),
        tolerance,
    ))
}

#[cfg(test)]
#[path = "filled_cells_tests.rs"]
mod tests;
