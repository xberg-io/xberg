use image::{GrayImage, RgbImage};
use std::ops::Range;
use xberg_paddle_ocr::DetailedTextBlock;

use crate::types::Table;

#[path = "ruled_punctuation/split_rows.rs"]
pub(super) mod split_rows;

const MAX_SIDE: u32 = 512;
const MAX_CELLS: usize = 80;
const MAX_REREAD_CELLS: usize = 32;
const MAX_CROP_ASPECT: u32 = 12;
const MIN_RULE_COVERAGE_PERCENT: u32 = 75;
const MIN_RULE_SEGMENT_PERCENT: u32 = 25;
const MIN_RULE_CONTRAST: u8 = 90;
const INK_BACKGROUND_RATIO: f64 = 0.65;
const BACKGROUND_QUANTILE: f64 = 0.8;
const MAX_PUNCTUATION_WIDTH: u32 = 2;

struct Grid {
    rows: Vec<Range<u32>>,
    columns: Vec<Range<u32>>,
    ruling_segments: Vec<Vec<Range<u32>>>,
    ink_cutoff: u8,
}

type Bounds = (u32, u32, u32, u32);
type Site = (usize, usize, usize, Bounds);
type Edit = (usize, usize, usize, String);

pub(super) fn eligible(image: &RgbImage, block_count: usize) -> bool {
    (1..=MAX_SIDE).contains(&image.width())
        && (1..=MAX_SIDE).contains(&image.height())
        && (9..=MAX_CELLS).contains(&block_count)
}

fn spans(end: u32, mut present: impl FnMut(u32) -> bool) -> Vec<Range<u32>> {
    let mut result: Vec<Range<u32>> = Vec::new();
    for x in (0..end).filter(|&x| present(x)) {
        if let Some(last) = result.last_mut().filter(|last| last.end == x) {
            last.end += 1;
        } else {
            result.push(x..x + 1);
        }
    }
    result
}

fn grid(gray: &GrayImage, table: &Table) -> Option<Grid> {
    let (width, height) = gray.dimensions();
    let ncols = table.cells.first()?.len();
    if !(3..=6).contains(&ncols) || table.cells.iter().any(|row| row.len() != ncols) {
        return None;
    }
    let mut edges: Vec<_> = [0, height - 1]
        .into_iter()
        .flat_map(|y| (0..width).map(move |x| gray[(x, y)][0]))
        .collect();
    edges.sort_unstable();
    let cutoff = edges[edges.len() / 2].saturating_sub(MIN_RULE_CONTRAST);
    let dark = |x, y| gray[(x, y)][0] <= cutoff;
    let rulings = spans(height, |y| {
        (0..width).filter(|&x| dark(x, y)).count() as u32 * 100 >= MIN_RULE_COVERAGE_PERCENT * width
    });
    if rulings.len() != table.cells.len() {
        return None;
    }
    let rows: Vec<_> = std::iter::once(0)
        .chain(rulings.iter().map(|r| r.end))
        .zip(rulings.iter().map(|r| r.start))
        .map(|(a, b)| a..b)
        .collect();
    if rows.iter().any(|row| !(8..=28).contains(&row.len())) {
        return None;
    }
    let mut gutters = spans(width, |x| rows.iter().all(|row| !row.clone().any(|y| dark(x, y))));
    gutters.retain(|span| span.start > 0 && span.end < width && span.len() >= 2);
    gutters.sort_by_key(|span| std::cmp::Reverse(span.len()));
    if gutters.len() < ncols - 1 {
        return None;
    }
    let mut boundaries: Vec<_> = gutters[..ncols - 1].iter().map(|r| (r.start + r.end) / 2).collect();
    boundaries.sort_unstable();
    let columns = std::iter::once(0)
        .chain(boundaries.iter().copied())
        .zip(boundaries.iter().copied().chain(std::iter::once(width)))
        .map(|(a, b)| a..b)
        .collect();
    // Only long, continuous strokes receive the ruling exemption. Short ink in a break still needs an owner. ~keep
    let mut ruling_segments = vec![Vec::new(); height as usize];
    for y in rulings.into_iter().flatten() {
        ruling_segments[y as usize] = spans(width, |x| dark(x, y))
            .into_iter()
            .filter(|span| (span.end - span.start) * 100 >= MIN_RULE_SEGMENT_PERCENT * width)
            .collect();
    }
    Some(Grid {
        rows,
        columns,
        ruling_segments,
        ink_cutoff: cutoff.max((f64::from(edges[edges.len() / 2]) * INK_BACKGROUND_RATIO) as u8),
    })
}

fn bounds(block: &DetailedTextBlock) -> Option<Bounds> {
    point_bounds(&block.block.box_points)
}

fn point_bounds(points: &[xberg_paddle_ocr::Point]) -> Option<Bounds> {
    Some((
        points.iter().map(|p| p.x).min()?,
        points.iter().map(|p| p.y).min()?,
        points.iter().map(|p| p.x).max()?,
        points.iter().map(|p| p.y).max()?,
    ))
}

fn owners(gray: &GrayImage, grid: &Grid, table: &Table, blocks: &[DetailedTextBlock]) -> Option<Vec<(usize, Bounds)>> {
    let cols = grid.columns.len();
    if grid.rows.len() * cols != blocks.len() {
        return None;
    }
    let mut owners = vec![None; blocks.len()];
    for (index, block) in blocks.iter().enumerate() {
        let (x, y, right, bottom) = bounds(block)?;
        let row = grid.rows.iter().position(|r| r.contains(&midpoint(y, bottom)))?;
        let col = grid.columns.iter().position(|r| r.contains(&midpoint(x, right)))?;
        let box_bounds = (x, y, right, bottom);
        let header_or_label = row == 0 || col == 0;
        let clipped = if header_or_label {
            clip_without_foreign_ink(gray, grid, row, col, box_bounds)?.0
        } else {
            owned_ink(gray, grid, row, col, box_bounds)?
        };
        let stock = &table.cells[row][col];
        let same = if header_or_label {
            stock
                .chars()
                .filter(|c| !c.is_whitespace())
                .eq(block.block.text.chars().filter(|c| !c.is_whitespace()))
        } else {
            stock == block.block.text.trim()
        };
        if !same || owners[row * cols + col].replace((index, clipped)).is_some() {
            return None;
        }
    }
    owners.into_iter().collect()
}

fn owned_ink(gray: &GrayImage, grid: &Grid, row: usize, col: usize, box_bounds: Bounds) -> Option<Bounds> {
    let (clipped, has_owned_ink) = clip_without_foreign_ink(gray, grid, row, col, box_bounds)?;
    has_owned_ink.then_some(clipped)
}

fn clip_without_foreign_ink(
    gray: &GrayImage,
    grid: &Grid,
    row: usize,
    col: usize,
    box_bounds: Bounds,
) -> Option<(Bounds, bool)> {
    let (x, y, right, bottom) = box_bounds;
    let band = &grid.rows[row];
    let column = &grid.columns[col];
    let clipped = (
        x.max(column.start),
        y.max(band.start),
        right.min(column.end),
        bottom.min(band.end),
    );
    if right > gray.width() || bottom > gray.height() || clipped.0 >= clipped.2 || clipped.1 >= clipped.3 {
        return None;
    }
    let mut has_owned_ink = false;
    let numeric_cell = row != 0 && col != 0;
    for yy in y..bottom {
        for xx in x..right {
            if !is_nonrule_ink(gray, grid, xx, yy) {
                continue;
            }
            let inside_column = column.contains(&xx);
            let inside_band = band.contains(&yy);
            if !inside_column || (numeric_cell && !inside_band) {
                return None;
            }
            has_owned_ink |= inside_band;
        }
    }
    let row_contained = band.start <= y && bottom <= band.end;
    (has_owned_ink || row_contained).then_some((clipped, has_owned_ink))
}

fn is_nonrule_ink(gray: &GrayImage, grid: &Grid, x: u32, y: u32) -> bool {
    gray[(x, y)][0] <= grid.ink_cutoff
        && !grid.ruling_segments[y as usize]
            .iter()
            .any(|segment| segment.contains(&x))
}

fn midpoint(a: u32, b: u32) -> u32 {
    a + (b - a) / 2
}

fn crop(image: &RgbImage, (x, y, right, bottom): Bounds) -> Option<RgbImage> {
    (right > x && bottom > y).then(|| image::imageops::crop_imm(image, x, y, right - x, bottom - y).to_image())
}

struct Ink {
    x: Range<u32>,
    top: u32,
    bottom: u32,
}

fn source_ink(image: &RgbImage) -> Vec<Ink> {
    let gray = image::imageops::grayscale(image);
    let mut values = gray.as_raw().clone();
    values.sort_unstable();
    let position = (values.len() - 1) as f64 * BACKGROUND_QUANTILE;
    let (lo, hi) = (position.floor() as usize, position.ceil() as usize);
    let bg = f64::from(values[lo]) + (position.fract()) * f64::from(values[hi] - values[lo]);
    let foreground = |x, y| f64::from(gray[(x, y)][0]) < bg * INK_BACKGROUND_RATIO;
    spans(gray.width(), |x| (0..gray.height()).any(|y| foreground(x, y)))
        .into_iter()
        .filter_map(|x| {
            let rows = || (0..gray.height()).filter(|&y| x.clone().any(|xx| foreground(xx, y)));
            Some(Ink {
                top: rows().next()?,
                bottom: rows().next_back()?,
                x,
            })
        })
        .collect()
}

impl Ink {
    fn is_inside(&self, (left, top, right, bottom): Bounds) -> bool {
        left <= self.x.start && self.x.end <= right && top <= self.top && self.bottom < bottom
    }
}

fn proposal(stock: &str, ink: &[Ink], detected: Bounds) -> Option<String> {
    let mut proposed = stock.as_bytes().to_vec();
    // Cell-wide spans index the stock text, so even a mark with owned neighbors needs an owned sequence. ~keep
    if !stock.is_ascii() || proposed.len() != ink.len() || !ink.iter().all(|glyph| glyph.is_inside(detected)) {
        return None;
    }
    for (i, chars) in stock.as_bytes().windows(3).enumerate() {
        let [prev, mark, next] = &ink[i..i + 3] else {
            return None;
        };
        if !chars[0].is_ascii_digit()
            || !matches!(chars[1], b'.' | b',')
            || !chars[2].is_ascii_digit()
            || prev.bottom != next.bottom
            || mark.x.len() > MAX_PUNCTUATION_WIDTH as usize
            || mark.top.saturating_add(1) < prev.bottom
            || mark.bottom < prev.bottom
        {
            continue;
        }
        proposed[i + 1] = if mark.bottom > prev.bottom { b',' } else { b'.' };
    }
    (proposed != stock.as_bytes()).then(|| String::from_utf8(proposed).expect("ASCII proposal"))
}

fn read_candidate(
    image: &RgbImage,
    band: &Range<u32>,
    column: &Range<u32>,
    detected: Bounds,
    stock: &str,
    read: &mut impl FnMut(&RgbImage) -> Option<String>,
) -> Option<String> {
    let interior = crop(image, (column.start, band.start, column.end, band.end))?;
    let ink = source_ink(&interior);
    let detected_in_cell = (
        detected.0.checked_sub(column.start)?,
        detected.1.checked_sub(band.start)?,
        detected.2.checked_sub(column.start)?,
        detected.3.checked_sub(band.start)?,
    );
    let suggested = proposal(stock, &ink, detected_in_cell)?;
    let clipped = crop(image, detected)?;
    let tight = crop(
        &interior,
        (
            ink.first()?.x.start.saturating_sub(1),
            ink.iter().map(|g| g.top).min()?.saturating_sub(1),
            (ink.last()?.x.end + 1).min(interior.width()),
            (ink.iter().map(|g| g.bottom).max()? + 2).min(interior.height()),
        ),
    )?;
    let reads = [read(&clipped)?, read(&interior)?, read(&tight)?];
    select(stock, &suggested, &reads)
}

fn select(stock: &str, suggested: &str, reads: &[String; 3]) -> Option<String> {
    let mut accepted = None;
    for candidate in reads {
        let allowed = candidate.len() == stock.len()
            && candidate != stock
            && stock
                .bytes()
                .zip(candidate.bytes())
                .zip(suggested.bytes())
                .all(|((old, new), predicted)| {
                    old == new || (matches!(old, b'.' | b',') && old != predicted && new == predicted)
                });
        if allowed
            && reads
                .iter()
                .filter(|other| other.as_str() == candidate.as_str())
                .count()
                >= 2
        {
            accepted = Some(candidate.clone());
        }
    }
    accepted
}

pub(super) fn corrections(
    image: &RgbImage,
    table: &Table,
    blocks: &[DetailedTextBlock],
    stopped: impl Fn() -> bool,
    read: impl FnMut(&RgbImage) -> Option<String>,
) -> Option<Vec<Edit>> {
    if stopped() || !eligible(image, blocks.len()) {
        return None;
    }
    let gray = image::imageops::grayscale(image);
    let grid = grid(&gray, table)?;
    let owners = owners(&gray, &grid, table, blocks)?;
    let cols = grid.columns.len();
    let sites = owners
        .into_iter()
        .enumerate()
        .map(|(offset, (index, detected))| (index, offset / cols, offset % cols, detected));
    read_sites(image, (&grid, &table.cells), blocks, sites, stopped, read)
}

fn read_sites(
    image: &RgbImage,
    source: (&Grid, &[Vec<String>]),
    blocks: &[DetailedTextBlock],
    sites: impl IntoIterator<Item = Site>,
    stopped: impl Fn() -> bool,
    mut read: impl FnMut(&RgbImage) -> Option<String>,
) -> Option<Vec<Edit>> {
    let (grid, cells) = source;
    let mut remaining = MAX_REREAD_CELLS * 3;
    let mut read_bounded = |crop: &RgbImage| {
        if stopped() || crop.width() > crop.height() * MAX_CROP_ASPECT {
            return None;
        }
        remaining = remaining.checked_sub(1)?;
        read(crop).filter(|_| !stopped())
    };
    let mut edits = Vec::new();
    for (index, row, col, detected) in sites {
        if stopped() {
            return None;
        }
        let stock = &cells[row][col];
        if row == 0 || col == 0 || !stock.bytes().any(|b| matches!(b, b'.' | b',')) {
            continue;
        }
        if let Some(text) = read_candidate(
            image,
            &grid.rows[row],
            &grid.columns[col],
            detected,
            stock,
            &mut read_bounded,
        ) {
            if !matches!(blocks[index].words.as_slice(), [word] if word.word.text==*stock) {
                return None;
            }
            edits.push((index, row, col, text));
        }
    }
    (!stopped()).then_some(edits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(text: &str, (x, y, right, bottom): Bounds) -> DetailedTextBlock {
        let mut block = DetailedTextBlock::default();
        block.block.text = text.to_owned();
        block.block.box_points = [(x, y), (right, y), (right, bottom), (x, bottom)]
            .map(|(x, y)| xberg_paddle_ocr::Point { x, y })
            .to_vec();
        block.words.push(xberg_paddle_ocr::WordBlock {
            word: xberg_paddle_ocr::RecognizedWord {
                text: text.to_owned(),
                ..Default::default()
            },
            ..Default::default()
        });
        block
    }

    struct RuledControl {
        image: RgbImage,
        table: Table,
        blocks: Vec<DetailedTextBlock>,
    }

    fn ruled_control() -> RuledControl {
        ruled_control_with_rows(3)
    }

    fn ruled_control_with_rows(rows: u32) -> RuledControl {
        let mut image = RgbImage::from_pixel(90, rows * 11, image::Rgb([255; 3]));
        let ink = image::Rgb([0; 3]);
        let mut blocks = Vec::new();
        for row in 0..rows {
            let top = row * 11;
            for x in 0..90 {
                image.put_pixel(x, top + 10, ink);
            }
            for col in 0..3 {
                let x = col * 30 + 11;
                for extra in [0, 4] {
                    for y in top + 2..=top + 6 {
                        image.put_pixel(x + extra, y, ink);
                    }
                }
                image.put_pixel(x + 2, top + 6, ink);
                image.put_pixel(x + 2, top + 7, ink);
                blocks.push(block("1.2", (x - 1, top + 1, x + 6, top + 9)));
            }
        }
        let table = Table {
            cells: vec![vec!["1.2".to_owned(); 3]; rows as usize],
            ..Default::default()
        };
        RuledControl { image, table, blocks }
    }

    #[tokio::test]
    async fn stop_after_worker_handoff_discards_commas_and_fused_row_plan_together() {
        use super::super::{PaddleRasterOcr, PaddleRuledRepair};
        use crate::cancellation::CancellationToken;
        use crate::core::extractor::cacheability::{blocking, ocr_stop_requested, with_ocr_cancellation};

        for stop in [false, true] {
            let token = CancellationToken::new();
            with_ocr_cancellation(Some(&token), async {
                let worker = blocking(|| {
                    let RuledControl {
                        image,
                        mut table,
                        mut blocks,
                    } = ruled_control_with_rows(4);
                    let mut fused = block("1.2 1.2", (40, 12, 77, 20));
                    fused.words = blocks[4..6]
                        .iter()
                        .map(|owner| {
                            let mut word = owner.words[0].clone();
                            word.box_points = owner.block.box_points.clone();
                            word
                        })
                        .collect();
                    drop(blocks.splice(4..6, [fused]));
                    table.cells[1][1] = "1.2 1.2".to_owned();
                    table.cells[1][2].clear();

                    let mut corrected = blocks.clone();
                    corrected[6].block.text = "1,2".to_owned();
                    corrected[6].words[0].word.text = "1,2".to_owned();
                    table.cells[2][1] = "1,2".to_owned();
                    let plan = split_rows::recover(&image, table.clone(), &corrected, true)
                        .expect("the worker produced a source-owned fused-row plan");
                    let raster = PaddleRasterOcr {
                        blocks,
                        width: image.width(),
                        height: image.height(),
                        repair: PaddleRuledRepair {
                            blocks: Some(corrected),
                            rows: Some(plan),
                        },
                    };
                    assert!(!ocr_stop_requested());
                    (raster, table)
                });
                let (raster, mut table) = tokio::task::spawn_blocking(worker).await.unwrap();

                if stop {
                    token.cancel();
                    table.cells[2][1] = "1.2".to_owned();
                }
                let (blocks, plan) = raster.adopt_ruled_repair();
                assert_eq!(plan.is_some(), !stop);
                if let Some(plan) = plan {
                    plan.apply(std::slice::from_mut(&mut table));
                }
                let expected = if stop { "1.2" } else { "1,2" };
                assert_eq!(blocks[6].block.text, expected);
                assert_eq!(blocks[6].words[0].word.text, expected);
                assert_eq!(table.cells[2][1], expected);
                assert_eq!(table.cells[1][1..], if stop { ["1.2 1.2", ""] } else { ["1.2", "1.2"] });
            })
            .await;
        }
    }

    #[test]
    fn count_preserving_gutter_crossing_with_neighbor_ink_abstains_before_recognition() {
        let RuledControl {
            image,
            table,
            mut blocks,
        } = ruled_control();
        let read = |_: &RgbImage| Some("1,2".to_owned());
        blocks[4] = block("1.2", (40, 12, 66, 20));
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert_eq!(owners(&gray, &layout, &table, &blocks).unwrap().len(), 9);

        blocks[4] = block("1.2", (40, 12, 74, 20));
        let (x, _, right, _) = bounds(&blocks[4]).unwrap();
        assert!(layout.columns[1].contains(&midpoint(x, right)));
        assert_eq!(blocks.len(), 9);
        assert_eq!(blocks[4].block.text, table.cells[1][1]);
        assert!(
            corrections(
                &image,
                &table,
                &blocks,
                || false,
                |_| { panic!("cross-cell ink must be rejected before rereading") }
            )
            .is_none()
        );
        assert!(corrections(&image, &table, &ruled_control().blocks, || false, read).is_some());
    }

    #[test]
    fn centered_box_cannot_claim_neighbor_ink_in_a_broken_ruling() {
        let RuledControl {
            mut image,
            mut table,
            mut blocks,
        } = ruled_control();
        for x in 57..69 {
            image.put_pixel(x, 21, image::Rgb([255; 3]));
        }
        let bbox = (40, 12, 66, 22);
        blocks[4] = block("1.2", bbox);
        table.cells[1][1] = "1.2 1.2".to_owned();
        table.cells[1][2].clear();
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(layout.rows[1].contains(&midpoint(bbox.1, bbox.3)));
        assert!(layout.columns[1].contains(&midpoint(bbox.0, bbox.2)));
        assert!(split_rows::recover(&image, table.clone(), &blocks, false).is_some());

        image.put_pixel(63, 21, image::Rgb([0; 3]));
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(!layout.columns[1].contains(&63));
        assert_eq!(owned_ink(&gray, &layout, 1, 1, bbox), None);
        assert!(split_rows::recover(&image, table.clone(), &blocks, false).is_none());

        for y in 12..20 {
            for x in 40..47 {
                image.put_pixel(x, y, image::Rgb([255; 3]));
            }
        }
        assert!(split_rows::recover(&image, table, &blocks, false).is_none());
    }

    #[test]
    fn contained_detection_requires_foreground() {
        let RuledControl { image, table, .. } = ruled_control();
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        let blank = (48, 13, 53, 19);
        assert!(layout.rows[1].contains(&blank.1) && layout.rows[1].contains(&(blank.3 - 1)));
        assert!(layout.columns[1].contains(&blank.0) && layout.columns[1].contains(&(blank.2 - 1)));
        assert_eq!(owned_ink(&gray, &layout, 1, 1, blank), None);
    }

    #[test]
    fn faint_header_or_label_preserves_punctuation_but_cannot_support_a_split() {
        let RuledControl { image, table, blocks } = ruled_control();
        let mut split_table = table.clone();
        split_table.cells[1][1] = "1.2 1.2".to_owned();
        split_table.cells[1][2].clear();
        assert!(split_rows::recover(&image, split_table.clone(), &blocks, false).is_some());

        for (row, col) in [(0, 2), (1, 0)] {
            let mut faint = image.clone();
            let bbox = bounds(&blocks[row * 3 + col]).unwrap();
            for y in bbox.1..bbox.3 {
                for x in bbox.0..bbox.2 {
                    if faint[(x, y)] == image::Rgb([0; 3]) {
                        faint.put_pixel(x, y, image::Rgb([192; 3]));
                    }
                }
            }
            let gray = image::imageops::grayscale(&faint);
            let layout = grid(&gray, &table).unwrap();
            assert!(layout.ink_cutoff < 192);
            assert_eq!(owned_ink(&gray, &layout, row, col, bbox), None);
            let mut reads = 0;
            let edits = corrections(
                &faint,
                &table,
                &blocks,
                || false,
                |_| {
                    reads += 1;
                    Some("1,2".to_owned())
                },
            );
            assert_eq!(
                edits,
                Some(vec![
                    (4, 1, 1, "1,2".into()),
                    (5, 1, 2, "1,2".into()),
                    (7, 2, 1, "1,2".into()),
                    (8, 2, 2, "1,2".into())
                ])
            );
            assert_eq!(reads, 12);
            if row == 1 {
                assert!(split_rows::recover(&faint, split_table.clone(), &blocks, false).is_none());
            }
        }
    }

    #[test]
    fn faint_label_cannot_claim_only_foreground_from_the_next_row() {
        let RuledControl {
            mut image,
            table,
            mut blocks,
        } = ruled_control();
        let contained = (10, 12, 17, 20);
        for y in contained.1..contained.3 {
            for x in contained.0..contained.2 {
                if image[(x, y)] == image::Rgb([0; 3]) {
                    image.put_pixel(x, y, image::Rgb([192; 3]));
                }
            }
        }
        let read = |_: &RgbImage| Some("1,2".to_owned());
        assert_eq!(corrections(&image, &table, &blocks, || false, read).unwrap().len(), 4);

        let crossed = (10, 12, 17, 26);
        blocks[3] = block("1.2", crossed);
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(layout.rows[1].contains(&midpoint(crossed.1, crossed.3)));
        assert!(layout.columns[0].contains(&midpoint(crossed.0, crossed.2)));
        assert!((12..21).all(|y| (10..17).all(|x| !is_nonrule_ink(&gray, &layout, x, y))));
        assert!(is_nonrule_ink(&gray, &layout, 11, 24));

        let mut reads = 0;
        let edits = corrections(
            &image,
            &table,
            &blocks,
            || false,
            |_| {
                reads += 1;
                Some("1,2".to_owned())
            },
        );
        assert_eq!(edits, None);
        assert_eq!(reads, 0);
    }

    #[test]
    fn faint_label_with_dim_neighbor_abstains_before_any_crop_read() {
        let RuledControl {
            mut image,
            table,
            mut blocks,
        } = ruled_control();
        for block in [&blocks[3], &blocks[6]] {
            let (x, y, right, bottom) = bounds(block).unwrap();
            for yy in y..bottom {
                for xx in x..right {
                    if image[(xx, yy)] == image::Rgb([0; 3]) {
                        image.put_pixel(xx, yy, image::Rgb([192; 3]));
                    }
                }
            }
        }

        let read = |_: &RgbImage| Some("1,2".to_owned());
        assert_eq!(corrections(&image, &table, &blocks, || false, read).unwrap().len(), 4);

        let crossed = (10, 12, 17, 26);
        blocks[3] = block("1.2", crossed);
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(layout.rows[1].contains(&midpoint(crossed.1, crossed.3)));
        assert!(layout.columns[0].contains(&midpoint(crossed.0, crossed.2)));
        assert!(!layout.rows[1].contains(&(crossed.3 - 1)));
        assert_eq!(gray[(11, 24)][0], 192);
        assert!((12..26).all(|y| (10..17).all(|x| !is_nonrule_ink(&gray, &layout, x, y))));

        let mut reads = 0;
        let edits = corrections(
            &image,
            &table,
            &blocks,
            || false,
            |_| {
                reads += 1;
                Some("1,2".to_owned())
            },
        );
        assert_eq!((edits, reads), (None, 0));
    }

    #[test]
    fn crossed_label_requires_its_terminal_periods_in_the_assigned_row() {
        let RuledControl {
            mut image,
            mut table,
            mut blocks,
        } = ruled_control();
        let label = (10, 12, 26, 26);
        blocks[3] = block("Label..", label);
        table.cells[1][0] = "Label. .".to_owned();
        for x in [20, 22] {
            image.put_pixel(x, 18, image::Rgb([0; 3]));
        }
        assert!(split_rows::recover(&image, table.clone(), &blocks, false).is_none());

        // Keep crossed vertical padding without putting the neighboring label under the detector. ~keep
        let (x, y, right, bottom) = bounds(&blocks[6]).unwrap();
        let offset = label.2 - x + 1;
        for yy in y..bottom {
            for xx in x..right {
                let pixel = image[(xx, yy)];
                image.put_pixel(xx + offset, yy, pixel);
                image.put_pixel(xx, yy, image::Rgb([255; 3]));
            }
        }
        blocks[6] = block("1.2", (x + offset, y, right + offset, bottom));
        let recovered = split_rows::recover(&image, table.clone(), &blocks, false).unwrap();
        let mut result = [table.clone()];
        recovered.apply(&mut result);
        assert_eq!(result[0].cells[1][0], "Label..");

        for x in [20, 22] {
            image.put_pixel(x, 18, image::Rgb([255; 3]));
            image.put_pixel(x, 24, image::Rgb([0; 3]));
        }
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(layout.rows[1].contains(&midpoint(label.1, label.3)));
        assert!(is_nonrule_ink(&gray, &layout, 11, 13));
        assert!(is_nonrule_ink(&gray, &layout, 20, 24));
        assert!(owned_ink(&gray, &layout, 1, 0, label).is_some());
        assert!(split_rows::recover(&image, table.clone(), &blocks, false).is_none());

        image.put_pixel(22, 24, image::Rgb([255; 3]));
        image.put_pixel(18, 24, image::Rgb([0; 3]));
        for x in [22, 24] {
            image.put_pixel(x, 18, image::Rgb([0; 3]));
        }
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        assert!(owned_ink(&gray, &layout, 1, 0, label).is_some());
        for (x, y) in [(18, 24), (20, 24), (22, 18), (24, 18)] {
            assert!(is_nonrule_ink(&gray, &layout, x, y));
        }
        assert!(split_rows::recover(&image, table, &blocks, false).is_none());
    }

    #[test]
    fn wider_crop_agreement_cannot_use_mark_or_digit_ink_outside_the_detection() {
        let RuledControl { image, table, blocks } = ruled_control();
        let gray = image::imageops::grayscale(&image);
        let layout = grid(&gray, &table).unwrap();
        let band = &layout.rows[1];
        let column = &layout.columns[1];
        let complete = bounds(&blocks[4]).unwrap();
        let stock = "1.2";
        let agreed = "1,2";

        for incomplete in [(40, 12, 45, 20), (40, 12, 47, 18)] {
            assert_eq!(owned_ink(&gray, &layout, 1, 1, incomplete), Some(incomplete));
            let mut calls = 0;
            let mut read = |crop: &RgbImage| {
                calls += 1;
                let clipped_size = (incomplete.2 - incomplete.0, incomplete.3 - incomplete.1);
                let text = if crop.dimensions() == clipped_size {
                    stock
                } else {
                    agreed
                };
                Some(text.to_owned())
            };
            assert_eq!(
                select(stock, agreed, &[stock.into(), agreed.into(), agreed.into()]).as_deref(),
                Some(agreed)
            );
            assert_eq!(read_candidate(&image, band, column, incomplete, stock, &mut read), None);
            assert_eq!(calls, 0);
        }
        assert_eq!(
            read_candidate(&image, band, column, complete, stock, &mut |_| Some(agreed.to_owned())).as_deref(),
            Some(agreed)
        );
    }

    #[test]
    fn exterior_span_cannot_compensate_for_merged_digits_in_a_longer_value() {
        let black = image::Rgb([0; 3]);
        let mut image = RgbImage::from_pixel(23, 10, image::Rgb([255; 3]));
        for x in [2, 5, 11, 13, 15] {
            for y in 2..=6 {
                image.put_pixel(x, y, black);
            }
        }
        image.put_pixel(8, 6, black);
        image.put_pixel(8, 7, black);
        let detected = (1, 1, 16, 9);
        let stock = "12.345";
        let candidate = "12,345";
        let mut vote = |crop: &RgbImage| Some(if crop.dimensions() == (15, 8) { stock } else { candidate }.to_owned());

        // The final digit touches the last included detector column. ~keep
        let owned = source_ink(&image);
        assert_eq!(owned.last().unwrap().x.end, detected.2);
        assert_eq!(
            read_candidate(&image, &(0..10), &(0..23), detected, stock, &mut vote).as_deref(),
            Some(candidate)
        );

        image.put_pixel(14, 5, black);
        image.put_pixel(20, 5, black);
        let aliased = source_ink(&image);
        assert_eq!(aliased.len(), stock.len());
        assert_eq!(aliased[4].x, 13..16);
        assert_eq!(aliased[5].x, 20..21);
        assert!(aliased[1..4].iter().all(|glyph| glyph.is_inside(detected)));
        assert!(
            read_candidate(&image, &(0..10), &(0..23), detected, stock, &mut |_| {
                panic!("misaligned source sequence must abstain before rereading")
            })
            .is_none()
        );
    }

    #[test]
    fn stop_between_crop_reads_discards_all_edits_and_keeps_stock() {
        let RuledControl { image, table, blocks } = ruled_control();
        let accepted = corrections(&image, &table, &blocks, || false, |_| Some("1,2".to_owned())).unwrap();
        assert_eq!(accepted.len(), 4);
        assert_eq!(accepted[0], (4, 1, 1, "1,2".to_owned()));

        for cancel in [false, true] {
            let token = crate::cancellation::CancellationToken::new();
            let mut calls = 0;
            let result = corrections(
                &image,
                &table,
                &blocks,
                || token.is_cancelled() || token.time_limit_reached(),
                |_| {
                    calls += 1;
                    if calls == 4 {
                        if cancel {
                            token.cancel();
                        } else {
                            token.set_deadline(std::time::Instant::now());
                        }
                    }
                    Some("1,2".to_owned())
                },
            );
            assert_eq!(calls, 4);
            assert_eq!(result, None);
            assert_eq!(blocks[4].block.text, "1.2");
            assert_eq!(table.cells[1][1], "1.2");
        }
    }

    #[test]
    fn only_geometry_supported_and_repeated_punctuation_edits_are_selected() {
        for (reads, expected) in [
            (["12,345", "12.345", "12,345"], Some("12,345")),
            (["12,345", "12.345", "12.345"], None),
            (["12,346", "12,346", "12.345"], None),
            (["12,345", "12;345", "12;345"], None),
        ] {
            assert_eq!(
                select("12.345", "12,345", &reads.map(str::to_owned)).as_deref(),
                expected
            );
        }
    }
}

#[cfg(all(test, feature = "paddle-ocr-ort"))]
#[path = "ruled_punctuation/interior_span_replay.rs"]
mod interior_span_replay;
