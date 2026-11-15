use super::{Bounds, Grid, bounds, eligible, grid, is_nonrule_ink, midpoint, owned_ink, point_bounds};
use crate::types::Table;
use image::{GrayImage, RgbImage};
use xberg_paddle_ocr::DetailedTextBlock;

pub(in crate::paddle_ocr::backend) struct RowRecovery {
    stock: Vec<Vec<String>>,
    recovered: Vec<Vec<String>>,
}

impl RowRecovery {
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

pub(in crate::paddle_ocr::backend) fn recover(
    image: &RgbImage,
    table: Table,
    blocks: &[DetailedTextBlock],
    fused_words: bool,
) -> Option<RowRecovery> {
    if !eligible(image, blocks.len()) || !table.cells.iter().flatten().any(|cell| cell.is_empty()) {
        return None;
    }
    let gray = image::imageops::grayscale(image);
    let layout = grid(&gray, &table)?;
    let cols = layout.columns.len();
    let mut owned = vec![vec![None; cols]; layout.rows.len()];
    let mut unsafe_rows = vec![false; layout.rows.len()];
    for block in blocks {
        let (x, y, right, bottom) = bounds(block)?;
        let row = layout
            .rows
            .iter()
            .position(|band| band.contains(&midpoint(y, bottom)))?;
        let col = layout
            .columns
            .iter()
            .position(|gutter| gutter.contains(&midpoint(x, right)))?;
        let parent = (x, y, right, bottom);
        let slots = &mut owned[row];
        if owned_ink(&gray, &layout, row, col, parent).is_some() {
            unsafe_rows[row] |= slots[col].replace(block.block.text.trim()).is_some();
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
    for (row, cells) in recovered.iter_mut().enumerate().skip(1) {
        if unsafe_rows[row] {
            continue;
        }
        let Some(source) = owned[row].iter().copied().collect::<Option<Vec<_>>>() else {
            continue;
        };
        if let Some(left) = split_column(cells, &source) {
            cells[left] = source[left].to_owned();
            cells[left + 1] = source[left + 1].to_owned();
        }
    }
    (stock != recovered && same_tokens(&recovered, blocks)).then_some(RowRecovery { stock, recovered })
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
    fn split_requires_both_numeric_literals_and_preserves_other_cells() {
        let stock = ["Label", "43", "780.090-780.090", ""].map(str::to_owned);
        assert_eq!(split_column(&stock, &["Label", "43", "780.090", "-780.090"]), Some(2));
        assert_eq!(split_column(&stock, &["Label", "43", "780.090-780.090", ""]), None);
        assert_eq!(split_column(&stock, &["Label", "43", "780.090", "-780,090"]), None);
        assert_eq!(split_column(&stock, &["Label", "44", "780.090", "-780.090"]), None);
    }
}
