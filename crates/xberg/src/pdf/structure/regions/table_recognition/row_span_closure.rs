//! Recover an interior native cell only when source paint and word ownership enclose it.

use std::cell::Cell;
use std::ops::Range;

use crate::pdf::table_reconstruct::HocrWord;

use super::super::{TatrCrop, has_tatr_rule_coverage, has_tatr_span_rule_run, tatr_rule_band, tatr_rule_track};
use super::{PixelRange, SpanPaint, column_visible, span_boundary_geometry, word_masks};

const SHOULDER_PIXELS: u32 = 2;
const MAX_SEARCH_PIXELS: usize = 96;
const MAX_CELL_PIXELS: u32 = 512;
const MIN_RULE_WIDTH_PT: f32 = 24.0;
const MAX_DARK_LUMA: u16 = 127;
const MIN_SHOULDER_CONTRAST: u16 = 24;

#[derive(Clone, Copy)]
pub(in crate::pdf::structure::regions::table_recognition) struct SourceOwnership<'a> {
    placements: &'a [Option<(usize, usize)>],
    page_words: &'a [HocrWord],
}

impl<'a> SourceOwnership<'a> {
    pub(in crate::pdf::structure::regions::table_recognition) fn new(
        placements: &'a [Option<(usize, usize)>],
        page_words: &'a [HocrWord],
    ) -> Self {
        Self { placements, page_words }
    }
}

impl SpanPaint<'_> {
    pub(super) fn closed_body_cell(
        &self,
        rows: Range<usize>,
        column: usize,
        source: SourceOwnership<'_>,
    ) -> Option<ClosedCell> {
        let upper = span_boundary_geometry(self.inference, self.crop, self.bands, (rows.start, column, column + 1))?;
        let lower = span_boundary_geometry(self.inference, self.crop, self.bands, (rows.end, column, column + 1))?;
        let words = CellWords::from_placements(self.words, source.placements, &rows, column)?;
        let track = tatr_rule_track(self.crop, upper.track);
        let band = tatr_rule_band(self.crop, upper.band);
        let masks = word_masks(self.crop, self.words, &track, &band);
        let top = Cell::new(None);
        has_tatr_span_rule_run(
            self.crop,
            track,
            band,
            |x, y| column_visible(&masks, x, y),
            |rows| {
                top.set(Some(rows));
                true
            },
        );
        let [left, right] = words.gutters(self.crop)?;
        let search = ClosureSearch {
            left,
            right,
            top: top.into_inner()?,
            bottom: tatr_rule_band(self.crop, lower.band),
        };
        let cell = search.find(self.crop)?;
        (cell.owns_exactly(self.crop, source.page_words, &words.owned) && !cell.has_interior_rule(self.crop))
            .then_some(cell)
    }
}

#[derive(Default)]
struct CellWords<'a> {
    owned: Vec<&'a HocrWord>,
    left_end: Option<u32>,
    right_start: Option<u32>,
}

impl<'a> CellWords<'a> {
    fn from_placements(
        words: &[&'a HocrWord],
        placements: &[Option<(usize, usize)>],
        rows: &Range<usize>,
        target: usize,
    ) -> Option<Self> {
        let left = target.checked_sub(1)?;
        let right = target + 1;
        if words.len() != placements.len() {
            return None;
        }
        let mut result = Self::default();
        let mut label_row = None;
        for (&word, &placement) in words.iter().zip(placements) {
            let Some((row, col)) = placement.filter(|(row, _)| rows.contains(row)) else {
                continue;
            };
            if col == target {
                if label_row.is_some_and(|previous| previous != row) {
                    return None;
                }
                label_row = Some(row);
                result.owned.push(word);
            } else if col == left {
                let edge = word.left.saturating_add(word.width);
                result.left_end = Some(result.left_end.map_or(edge, |previous| previous.max(edge)));
            } else if col == right {
                result.right_start = Some(result.right_start.map_or(word.left, |previous| previous.min(word.left)));
            }
        }
        (!result.owned.is_empty()).then_some(result)
    }

    fn gutters(&self, crop: &TatrCrop) -> Option<[PixelRange; 2]> {
        let start = self.owned.iter().map(|word| word.left).min()?;
        let end = self
            .owned
            .iter()
            .map(|word| word.left.saturating_add(word.width))
            .max()?;
        Some([
            interior_gutter(crop, self.left_end?, start),
            interior_gutter(crop, end, self.right_start?),
        ])
    }
}

fn interior_gutter(crop: &TatrCrop, start: u32, end: u32) -> PixelRange {
    let pixel = |value| value as f32 * crop.sx - crop.px_left as f32;
    let first = ((pixel(start).floor() + 1.0) as u32).max(SHOULDER_PIXELS);
    let last = (pixel(end).ceil() as u32).min(crop.width.saturating_sub(SHOULDER_PIXELS));
    first..last
}

struct ClosureSearch {
    left: PixelRange,
    right: PixelRange,
    top: PixelRange,
    bottom: PixelRange,
}

impl ClosureSearch {
    fn find(self, crop: &TatrCrop) -> Option<ClosedCell> {
        if [&self.left, &self.right, &self.top, &self.bottom]
            .into_iter()
            .any(|band| band.is_empty() || band.len() > MAX_SEARCH_PIXELS)
        {
            return None;
        }
        let middle = self.top.end.midpoint(self.bottom.start);
        let left = unique(self.left.filter(|&x| stroke(crop, x, middle, (SHOULDER_PIXELS, 0))))?;
        let right = unique(self.right.filter(|&x| stroke(crop, x, middle, (SHOULDER_PIXELS, 0))))?;
        let width = right.checked_sub(left)?;
        if width > MAX_CELL_PIXELS || width < (MIN_RULE_WIDTH_PT * crop.sx).ceil() as u32 {
            return None;
        }
        let cap = |y: u32| {
            y >= SHOULDER_PIXELS
                && y < crop.height.saturating_sub(SHOULDER_PIXELS)
                && luma(crop, left, y) <= MAX_DARK_LUMA
                && luma(crop, right, y) <= MAX_DARK_LUMA
                && (left + 1..right).all(|x| stroke(crop, x, y, (0, SHOULDER_PIXELS)))
        };
        let top = unique(self.top.filter(|&y| cap(y)))?;
        let bottom = unique(self.bottom.filter(|&y| cap(y)))?;
        let height = bottom.checked_sub(top)?;
        if height > MAX_CELL_PIXELS || height <= 1 {
            return None;
        }
        let walls_close = (top + 1..bottom).all(|y| {
            [left, right]
                .into_iter()
                .all(|x| stroke(crop, x, y, (SHOULDER_PIXELS, 0)))
        });
        walls_close.then_some(ClosedCell {
            left,
            right,
            top,
            bottom,
        })
    }
}

fn unique(mut matches: impl Iterator<Item = u32>) -> Option<u32> {
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

#[derive(Clone, Copy)]
pub(super) struct ClosedCell {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl ClosedCell {
    fn owns_exactly(self, crop: &TatrCrop, page_words: &[HocrWord], owned: &[&HocrWord]) -> bool {
        let x = |value: u32| value as f32 * crop.sx - crop.px_left as f32;
        let y = |value: u32| value as f32 * crop.sy - crop.px_top as f32;
        let left = self.left as f32;
        let right = self.right as f32;
        let top = self.top as f32;
        let bottom = self.bottom as f32;
        let mut found = 0;
        for word in page_words.iter().filter(|word| !word.text.trim().is_empty()) {
            let word_left = x(word.left);
            let word_right = x(word.left.saturating_add(word.width));
            let word_top = y(word.top);
            let word_bottom = y(word.top.saturating_add(word.height));
            if word_left >= right || word_right <= left || word_top >= bottom || word_bottom <= top {
                continue;
            }
            let inside = left < word_left
                && word_left < word_right
                && word_right < right
                && top < word_top
                && word_top < word_bottom
                && word_bottom < bottom;
            if !inside || !owned.iter().any(|&claim| std::ptr::eq(word, claim)) {
                return false;
            }
            found += 1;
        }
        found == owned.len()
    }

    fn has_interior_rule(self, crop: &TatrCrop) -> bool {
        let min_width = (MIN_RULE_WIDTH_PT * crop.sx).ceil() as u32;
        let horizontal = (self.top + 1..self.bottom).any(|y| {
            let mut run = 0;
            (self.left + 1..self.right).any(|x| {
                run = if stroke(crop, x, y, (0, SHOULDER_PIXELS)) {
                    run + 1
                } else {
                    0
                };
                run >= min_width
            })
        });
        horizontal
            || (self.left + 1..self.right).any(|x| {
                let rows = self.top + 1..self.bottom;
                // A glyph can cover a shoulder without interrupting the separator. ~keep
                rows.clone().all(|y| luma(crop, x, y) <= MAX_DARK_LUMA)
                    && has_tatr_rule_coverage(
                        rows.clone()
                            .filter(|&y| stroke(crop, x, y, (SHOULDER_PIXELS, 0)))
                            .count() as u32,
                        rows.end - rows.start,
                    )
            })
    }
}

fn luma(crop: &TatrCrop, x: u32, y: u32) -> u16 {
    let [r, g, b] = crop.image.get_pixel(x, y).0.map(u16::from);
    (54 * r + 183 * g + 19 * b) / 256
}

fn stroke(crop: &TatrCrop, x: u32, y: u32, (dx, dy): (u32, u32)) -> bool {
    let ink = luma(crop, x, y);
    ink <= MAX_DARK_LUMA
        && luma(crop, x - dx, y - dy)
            .min(luma(crop, x + dx, y + dy))
            .saturating_sub(ink)
            >= MIN_SHOULDER_CONTRAST
}

#[cfg(test)]
mod tests {
    use crate::layout::models::tatr::{CellBBox, TableStructure};

    use super::super::{StructureWords, TatrInference, apply_structure_with_source};
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Paint {
        Closed,
        OpenWall,
        TintOnly,
        ParallelWall,
        InteriorRule,
        InteriorColumnRule,
        OccludedInteriorColumnRule,
        OutsideColumnRule,
        ForeignWord,
        WordOnCap,
    }

    #[test]
    fn closed_native_rowspan_preserves_heading_and_rejects_incomplete_evidence() {
        let inference = inference();
        let bands = [
            Some((11.0, 11.0)),
            Some((29.0, 29.0)),
            Some((49.0, 49.0)),
            Some((69.0, 69.0)),
            Some((89.0, 89.0)),
            Some((111.0, 111.0)),
        ];
        for paint in [
            Paint::Closed,
            Paint::OpenWall,
            Paint::TintOnly,
            Paint::ParallelWall,
            Paint::InteriorRule,
            Paint::InteriorColumnRule,
            Paint::OccludedInteriorColumnRule,
            Paint::OutsideColumnRule,
            Paint::ForeignWord,
            Paint::WordOnCap,
        ] {
            let crop = paint_crop(paint);
            let (mut dense, words, placements) = native_words(paint);
            let table_words: Vec<_> = words[..placements.len()].iter().collect();
            let source = StructureWords {
                words: &table_words,
                ownership: Some(SourceOwnership::new(&placements, &words)),
            };
            let grid = apply_structure_with_source(&mut dense, &bands, source, &inference, &crop, 120.0);
            assert_recovery(&grid, paint);
        }
    }

    fn assert_recovery(grid: &crate::types::TableGrid, paint: Paint) {
        let target: Vec<_> = grid
            .cells
            .iter()
            .filter(|cell| cell.col == 1)
            .map(|cell| (cell.row, cell.row_span, cell.content.as_str()))
            .collect();
        if matches!(paint, Paint::Closed | Paint::OutsideColumnRule) {
            assert_eq!(target, [(0, 1, "Subject"), (1, 4, "Shared label"), (5, 1, "Later")]);
            let frame = grid
                .cells
                .iter()
                .find(|cell| cell.row == 1 && cell.col == 1)
                .unwrap()
                .bbox
                .as_ref()
                .unwrap();
            assert_eq!((frame.x0, frame.y0, frame.x1, frame.y1), (61.0, 20.0, 174.0, 100.0));
        } else {
            assert_eq!(
                target,
                [
                    (0, 1, "Subject"),
                    (1, 1, ""),
                    (2, 1, ""),
                    (3, 1, "Shared label"),
                    (4, 1, ""),
                    (5, 1, "Later")
                ],
                "{paint:?}"
            );
        }
        assert_eq!(
            grid.cells
                .iter()
                .filter(|cell| cell.col != 1 && cell.row_span != 1)
                .count(),
            0
        );
        for row in 1..5 {
            for (column, text) in [(0, "Left"), (2, "Right")] {
                assert_eq!(
                    grid.cells
                        .iter()
                        .find(|cell| cell.row == row && cell.col == column)
                        .unwrap()
                        .content,
                    text
                );
            }
        }
    }

    fn inference() -> TatrInference {
        TatrInference {
            cells: (0..6)
                .map(|row| {
                    [(0.0, 60.0), (61.0, 174.0), (180.0, 240.0)]
                        .into_iter()
                        .map(|(x1, x2)| CellBBox {
                            x1,
                            x2,
                            y1: row as f32 * 20.0,
                            y2: (row + 1) as f32 * 20.0,
                        })
                        .collect()
                })
                .collect(),
            structure: TableStructure {
                spans: vec![(0, 5, 1, 2)],
                header_row_count: 1,
                ..Default::default()
            },
        }
    }

    fn paint_crop(paint: Paint) -> TatrCrop {
        const ORANGE: image::Rgb<u8> = image::Rgb([255, 217, 102]);
        const BLUE: image::Rgb<u8> = image::Rgb([143, 170, 220]);
        const RULE: image::Rgb<u8> = image::Rgb([0, 0, 0]);
        let mut image = image::RgbImage::from_pixel(240, 120, image::Rgb([255; 3]));
        for y in 21..120 {
            for x in 0..240 {
                image.put_pixel(x, y, if y < 100 { ORANGE } else { BLUE });
            }
        }
        for y in 20..=100 {
            image.put_pixel(60, y, RULE);
            image.put_pixel(180, y, RULE);
        }
        for x in 0..240 {
            image.put_pixel(x, 20, RULE);
        }
        if paint != Paint::TintOnly {
            for x in 60..=180 {
                image.put_pixel(x, 100, image::Rgb([99, 99, 90]));
            }
        }
        match paint {
            Paint::OpenWall => image.put_pixel(180, 41, ORANGE),
            Paint::ParallelWall => {
                for y in 21..100 {
                    image.put_pixel(64, y, RULE);
                }
            }
            Paint::InteriorRule => {
                for x in 90..125 {
                    image.put_pixel(x, 56, RULE);
                }
            }
            Paint::InteriorColumnRule | Paint::OccludedInteriorColumnRule | Paint::OutsideColumnRule => {
                let x = if paint == Paint::OutsideColumnRule { 225 } else { 122 };
                for y in 20..=100 {
                    image.put_pixel(x, y, RULE);
                }
                if paint == Paint::OccludedInteriorColumnRule {
                    image.put_pixel(x + SHOULDER_PIXELS, 67, RULE);
                }
            }
            _ => {}
        }
        TatrCrop {
            image,
            px_left: 0,
            px_top: 0,
            width: 240,
            height: 120,
            sx: 1.0,
            sy: 1.0,
            extended_bottom_pt: 120.0,
        }
    }

    type NativeWords = (Vec<Vec<String>>, Vec<HocrWord>, Vec<Option<(usize, usize)>>);

    fn native_words(paint: Paint) -> NativeWords {
        let mut dense = vec![vec![String::new(); 3]; 6];
        let mut words = Vec::new();
        let mut placements = Vec::new();
        let word = |text: &str, left, top, width| HocrWord {
            text: text.to_owned(),
            left,
            top,
            width,
            height: 6,
            confidence: 100.0,
        };
        let mut add = |row: usize, col: usize, text: &str, x, y, width| {
            if !dense[row][col].is_empty() {
                dense[row][col].push(' ');
            }
            dense[row][col].push_str(text);
            words.push(word(text, x, y, width));
            placements.push(Some((row, col)));
        };
        add(0, 1, "Subject", 90, 8, 28);
        for row in 1..5 {
            add(row, 0, "Left", 12, row as u32 * 20 + 6, 24);
            add(row, 2, "Right", 196, row as u32 * 20 + 6, 24);
        }
        add(3, 1, "Shared", 94, 66, 21);
        add(3, 1, "label", 132, 66, 18);
        add(5, 1, "Later", 94, 108, 24);
        if paint == Paint::ForeignWord {
            words.push(word("foreign", 112, 45, 14));
        }
        if paint == Paint::WordOnCap {
            words.push(word("foreign", 112, 98, 14));
        }
        (dense, words, placements)
    }
}
