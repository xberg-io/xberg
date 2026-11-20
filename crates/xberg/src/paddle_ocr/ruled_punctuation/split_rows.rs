use super::{
    Bounds, Edit, Grid, Site, bounds, eligible, grid, is_nonrule_ink, midpoint, owned_ink, point_bounds, spans,
};
use crate::types::Table;
use image::{GrayImage, RgbImage};
use xberg_paddle_ocr::DetailedTextBlock;

pub(in crate::paddle_ocr::backend) struct RowRecovery {
    stock: Vec<Vec<String>>,
    recovered: Vec<Vec<String>>,
    reread: Option<(Grid, Vec<Site>)>,
}

impl RowRecovery {
    pub(in crate::paddle_ocr::backend) fn punctuation_edits(
        &self,
        image: &RgbImage,
        blocks: &[DetailedTextBlock],
        stopped: impl Fn() -> bool,
        read: impl FnMut(&RgbImage) -> Option<String>,
    ) -> Option<Vec<Edit>> {
        let (grid, sites) = self.reread.as_ref()?;
        super::read_sites(
            image,
            (grid, &self.recovered),
            blocks,
            sites.iter().copied(),
            stopped,
            read,
        )
    }

    pub(in crate::paddle_ocr::backend) fn matches_with_edits(&self, revised: &Self, edits: &[Edit]) -> bool {
        let mut expected = self.recovered.clone();
        for (_, row, column, text) in edits {
            expected[*row][*column].clone_from(text);
        }
        revised.recovered == expected
    }

    pub(in crate::paddle_ocr::backend) fn apply(self, tables: &mut [Table]) {
        if let [table] = tables
            && table.cells == self.stock
        {
            table.markdown = crate::table_core::table_to_markdown(&self.recovered);
            table.cells = self.recovered;
        }
    }
}

fn without_whitespace(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().filter(|c| !c.is_whitespace())
}

fn numeric(text: &str) -> bool {
    text.bytes().any(|byte| byte.is_ascii_digit())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b".,+-()[]{}".contains(&byte))
}

fn split_column(stock: &[String], owned: &[&str]) -> Option<usize> {
    let mut empty = stock.iter().enumerate().filter(|(_, cell)| cell.trim().is_empty());
    let (right, _) = empty.next()?;
    if right < 2 || empty.next().is_some() {
        return None;
    }
    let left = right - 1;
    if !numeric(owned[left])
        || !numeric(owned[right])
        || !without_whitespace(&stock[left]).eq(owned[left].chars().chain(owned[right].chars()))
    {
        return None;
    }
    let matches_other_cells = stock
        .iter()
        .zip(owned)
        .enumerate()
        .all(|(col, (before, after))| match col {
            0 => without_whitespace(before).eq(without_whitespace(after)),
            col if col == left || col == right => true,
            _ => before.as_str() == *after,
        });
    matches_other_cells.then_some(left)
}

fn trailing_periods(text: &str) -> (&str, usize) {
    let prefix = text.trim_end_matches(|ch: char| ch == '.' || ch.is_whitespace());
    let count = text[prefix.len()..].bytes().filter(|&byte| byte == b'.').count();
    (prefix, count)
}

fn source_label_periods(stock: &[String], owned: &[&str]) -> bool {
    let (prefix, count) = trailing_periods(&stock[0]);
    !prefix.is_empty()
        && count >= 2
        && (prefix, count) == trailing_periods(owned[0])
        && stock[0].split_whitespace().ne(owned[0].split_whitespace())
        && stock[1..].iter().map(String::as_str).eq(owned[1..].iter().copied())
}

fn terminal_periods_owned(gray: &GrayImage, layout: &Grid, row: usize, label: Bounds, count: usize) -> bool {
    let (left, top, right, bottom) = label;
    let band = &layout.rows[row];
    if band.start <= top && bottom <= band.end {
        return true;
    }

    // Blank padding and proven ruling pixels may cross the band. Other ink could belong to a neighboring label. ~keep
    if (top..bottom)
        .filter(|y| !band.contains(y))
        .any(|y| (left..right).any(|x| is_nonrule_ink(gray, layout, x, y)))
    {
        return false;
    }

    let components = spans(right - left, |offset| {
        (top..bottom).any(|y| is_nonrule_ink(gray, layout, left + offset, y))
    });
    // The detector may include blank padding across a rule. Require actual terminal dot spans in its row. ~keep
    components.len() > count
        && components.iter().rev().take(count).all(|span| {
            if span.len() > super::MAX_PUNCTUATION_WIDTH as usize {
                return false;
            }
            let mut ink_rows = (top..bottom).filter(|&y| {
                span.clone()
                    .any(|offset| is_nonrule_ink(gray, layout, left + offset, y))
            });
            let Some(first) = ink_rows.next() else {
                return false;
            };
            let last = ink_rows.next_back().unwrap_or(first);
            band.contains(&first) && band.contains(&last) && last - first < 2
        })
}

fn contains(outer: Bounds, inner: Bounds) -> bool {
    outer.0 <= inner.0 && outer.1 <= inner.1 && inner.2 <= outer.2 && inner.3 <= outer.3
}

fn word_column(gray: &GrayImage, layout: &Grid, row: usize, bbox: Bounds) -> Option<usize> {
    let col = layout
        .columns
        .iter()
        .position(|column| column.start <= bbox.0 && bbox.2 <= column.end)?;
    if col == 0 {
        return None;
    }
    owned_ink(gray, layout, row, col, bbox).map(|_| col)
}

fn fused_word_pair<'a>(
    gray: &GrayImage,
    layout: &Grid,
    row: usize,
    block: &'a DetailedTextBlock,
    parent: Bounds,
) -> Option<[(usize, &'a str); 2]> {
    if row == 0 || parent.2 > gray.width() || parent.3 > gray.height() {
        return None;
    }
    let [left, right] = block.words.as_slice() else {
        return None;
    };
    let words = [left.word.text.as_str(), right.word.text.as_str()];
    if !words.into_iter().all(numeric) || !block.block.text.split_whitespace().eq(words) {
        return None;
    }

    if left.box_points.len() != 4 || right.box_points.len() != 4 {
        return None;
    }
    let left_box = point_bounds(&left.box_points)?;
    let right_box = point_bounds(&right.box_points)?;
    if !contains(parent, left_box) || !contains(parent, right_box) {
        return None;
    }
    let left_col = word_column(gray, layout, row, left_box)?;
    let right_col = word_column(gray, layout, row, right_box)?;
    if right_col != left_col + 1 {
        return None;
    }

    // The parent line can cross a gutter only if its words account for every non-rule pixel. ~keep
    let covered = (parent.1..parent.3).all(|y| {
        (parent.0..parent.2).all(|x| {
            !is_nonrule_ink(gray, layout, x, y)
                || contains(left_box, (x, y, x + 1, y + 1))
                || contains(right_box, (x, y, x + 1, y + 1))
        })
    });
    covered.then_some([(left_col, words[0]), (right_col, words[1])])
}

fn locate(layout: &Grid, (x, y, right, bottom): Bounds) -> Option<(usize, usize)> {
    let row = layout
        .rows
        .iter()
        .position(|band| band.contains(&midpoint(y, bottom)))?;
    let col = layout
        .columns
        .iter()
        .position(|column| column.contains(&midpoint(x, right)))?;
    Some((row, col))
}

fn needs_recovery(row: &[String]) -> bool {
    row.first().is_some_and(|label| trailing_periods(label).1 >= 2) || row.iter().any(String::is_empty)
}

fn independent_sites<'a>(
    lines: &'a [Option<Site>],
    stock: &[String],
    source: &[&str],
) -> impl Iterator<Item = Site> + 'a {
    let exact = lines.iter().all(Option::is_some)
        && without_whitespace(&stock[0]).eq(without_whitespace(source[0]))
        && stock[1..].iter().map(String::as_str).eq(source[1..].iter().copied());
    lines[1..].iter().copied().flatten().filter(move |_| exact)
}

pub(in crate::paddle_ocr::backend) fn recover(
    image: &RgbImage,
    table: Table,
    blocks: &[DetailedTextBlock],
    fused_words: bool,
) -> Option<RowRecovery> {
    if !eligible(image, blocks.len()) || !table.cells.iter().skip(1).any(|row| needs_recovery(row)) {
        return None;
    }
    let gray = image::imageops::grayscale(image);
    let layout = grid(&gray, &table)?;
    let cols = layout.columns.len();
    let mut owned = vec![vec![None; cols]; layout.rows.len()];
    let mut unsafe_rows = vec![false; layout.rows.len()];
    let mut label_bounds = vec![None; layout.rows.len()];
    let incomplete = fused_words && blocks.len() < layout.rows.len() * cols;
    let mut lines = incomplete.then(|| vec![vec![None; cols]; layout.rows.len()]);
    for (index, block) in blocks.iter().enumerate() {
        let parent = bounds(block)?;
        let (row, col) = locate(&layout, parent)?;
        let slots = &mut owned[row];
        if let Some(clipped) = owned_ink(&gray, &layout, row, col, parent) {
            if let Some(lines) = &mut lines {
                lines[row][col] = Some((index, row, col, clipped));
            }
            unsafe_rows[row] |= slots[col].replace(block.block.text.trim()).is_some();
            if col == 0 {
                label_bounds[row] = Some(parent);
            }
        } else if fused_words
            && let Some(pair) = fused_word_pair(&gray, &layout, row, block, parent)
            && pair.iter().all(|(column, _)| slots[*column].is_none())
        {
            for (column, text) in pair {
                slots[column] = Some(text);
            }
        } else {
            unsafe_rows[row] = true;
        }
    }

    let stock = table.cells;
    let mut recovered = stock.clone();
    let mut sites = Vec::new();
    let mut recovered_fused_row = false;
    for (row, cells) in recovered.iter_mut().enumerate().skip(1) {
        if unsafe_rows[row] {
            continue;
        }
        let Some(source) = owned[row].iter().copied().collect::<Option<Vec<_>>>() else {
            continue;
        };
        if let Some(left) = split_column(cells, &source) {
            recovered_fused_row |= lines
                .as_ref()
                .is_some_and(|lines| lines[row][left..=left + 1].iter().all(Option::is_none));
            cells[left] = source[left].to_owned();
            cells[left + 1] = source[left + 1].to_owned();
        }
        if let Some(lines) = &lines {
            sites.extend(independent_sites(&lines[row], cells, &source));
        }
        if source_label_periods(cells, &source)
            && let Some(label) = label_bounds[row]
            && terminal_periods_owned(&gray, &layout, row, label, trailing_periods(source[0]).1)
        {
            cells[0] = source[0].to_owned();
        }
    }
    (stock != recovered && same_tokens(&recovered, blocks)).then(|| RowRecovery {
        stock,
        recovered,
        reread: recovered_fused_row.then_some((layout, sites)),
    })
}

fn same_tokens(cells: &[Vec<String>], blocks: &[DetailedTextBlock]) -> bool {
    let mut table_tokens: Vec<_> = cells
        .iter()
        .flatten()
        .flat_map(|cell| cell.split_whitespace())
        .collect();
    let mut source_tokens: Vec<_> = blocks
        .iter()
        .flat_map(|block| block.block.text.split_whitespace())
        .collect();
    table_tokens.sort_unstable();
    source_tokens.sort_unstable();
    table_tokens == source_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_sites_require_independent_lines_and_exact_numeric_stock() {
        let sites = [0, 1, 2].map(|col| Some((col, 1, col, (0, 0, 1, 1))));
        let stock = ["Label", "1.250", "3.75"].map(str::to_owned);
        let matching = ["Label", "1.250", "3.75"];
        assert_eq!(
            independent_sites(&sites, &stock, &matching).collect::<Vec<_>>(),
            sites[1..].iter().copied().flatten().collect::<Vec<_>>()
        );
        assert_eq!(independent_sites(&[sites[0], None, None], &stock, &matching).count(), 0);
        assert_eq!(
            independent_sites(&sites, &stock, &["Label", "1.250", "3.76"]).count(),
            0
        );
    }

    fn word(text: &str, (x, y, right, bottom): Bounds) -> xberg_paddle_ocr::WordBlock {
        xberg_paddle_ocr::WordBlock {
            word: xberg_paddle_ocr::RecognizedWord {
                text: text.to_owned(),
                ..Default::default()
            },
            box_points: [(x, y), (right, y), (right, bottom), (x, bottom)]
                .map(|(x, y)| xberg_paddle_ocr::Point { x, y })
                .to_vec(),
        }
    }

    #[test]
    fn fused_line_requires_separate_word_owners_and_exact_line_tokens() {
        let layout = Grid {
            rows: vec![0..1, 1..10],
            columns: vec![0..10, 10..20, 20..30],
            ruling_segments: vec![Vec::new(); 10],
            ink_cutoff: 165,
        };
        let mut gray = GrayImage::from_pixel(30, 10, image::Luma([255]));
        for x in [14, 24] {
            gray.put_pixel(x, 5, image::Luma([0]));
        }
        let parent = (12, 1, 27, 9);
        let mut fused = DetailedTextBlock::default();
        fused.block.text = "811,070 [811.070]".to_owned();
        let left = word("811,070", (13, 2, 16, 8));
        let right = word("[811.070]", (23, 2, 26, 8));
        fused.words = vec![left.clone(), right.clone()];
        assert_eq!(
            fused_word_pair(&gray, &layout, 1, &fused, parent),
            Some([(1, "811,070"), (2, "[811.070]")])
        );

        for rejected in [
            word("[811.070]", (13, 2, 16, 8)),
            word("[811.070]", (19, 2, 26, 8)),
            word("[811,070]", (23, 2, 26, 8)),
        ] {
            fused.words[1] = rejected;
            assert!(fused_word_pair(&gray, &layout, 1, &fused, parent).is_none());
        }
        fused.words[1] = right;
        gray.put_pixel(20, 5, image::Luma([0]));
        assert!(fused_word_pair(&gray, &layout, 1, &fused, parent).is_none());
    }

    #[test]
    fn label_recovery_only_resegments_the_owned_terminal_periods() {
        let stock = ["Street repair. .", "10", "611,694", "47,576"].map(str::to_owned);
        assert!(source_label_periods(
            &stock,
            &["Street repair..", "10", "611,694", "47,576"]
        ));
        for rejected in [
            ["Streetrepair..", "10", "611,694", "47,576"],
            ["Street repair...", "10", "611,694", "47,576"],
            ["Street. repair.", "10", "611,694", "47,576"],
            ["Street repair..", "10", "611.694", "47,576"],
        ] {
            assert!(!source_label_periods(&stock, &rejected));
        }
    }

    #[test]
    fn split_requires_both_numeric_literals_and_preserves_other_cells() {
        let stock = ["Label", "43", "780.090-780.090", ""].map(str::to_owned);
        assert_eq!(split_column(&stock, &["Label", "43", "780.090", "-780.090"]), Some(2));
        assert_eq!(split_column(&stock, &["Label", "43", "780.090-780.090", ""]), None);
        assert_eq!(split_column(&stock, &["Label", "43", "780.090", "-780,090"]), None);
        assert_eq!(split_column(&stock, &["Label", "44", "780.090", "-780.090"]), None);
    }
}
