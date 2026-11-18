//! Recover source-witnessed column spans after a painted row divider splits a header proposal.

use crate::pdf::table_reconstruct::HocrWord;

use super::super::{
    TATR_NEAR_WHITE_CHANNEL, TatrCrop, has_contiguous_tatr_rule_coverage, has_tatr_span_rule_run,
    tatr_pixel_below_white_cutoff, tatr_rule_band, tatr_rule_track,
};
use super::{PixelRange, SpanPaint, column_visible, span_boundary_geometry, word_masks};

impl SpanPaint<'_> {
    pub(super) fn has_column_opening(
        &self,
        row: usize,
        columns: std::ops::Range<usize>,
        model_end: usize,
        dense: &[Vec<String>],
    ) -> bool {
        let Some([cell, next_cell]) = self
            .inference
            .cells
            .get(row)
            .and_then(|cells| cells.get(columns.clone()))
        else {
            return false;
        };
        let Some(body) = self
            .inference
            .cells
            .get(model_end)
            .and_then(|cells| cells.get(columns.start))
        else {
            return false;
        };
        let Some(closing) = span_boundary_geometry(
            self.inference,
            self.crop,
            self.bands,
            (model_end, columns.start, columns.end),
        ) else {
            return false;
        };
        let Some(expected) = dense.get(row).and_then(|cells| cells.get(columns)) else {
            return false;
        };
        // First body words can have em boxes extending above the painted border. ~keep
        let before_body: Vec<_> = self
            .words
            .iter()
            .copied()
            .filter(|word| source_word_center_y(word) < closing.band.1)
            .collect();
        let horizontal = tatr_rule_track(self.crop, closing.track);
        let row_track = tatr_rule_track(self.crop, crop_x_to_page(self.crop, cell.x1, next_cell.x2));
        let source_center = |row| self.bands.get(row).copied().flatten().map(|(first, _)| first);
        let Some(centers) = source_center(row).zip(source_center(row + 1)) else {
            return false;
        };
        let Some(row_interior) = painted_header_row(self.crop, &row_track, (cell.y1, cell.y2), centers, self.words)
        else {
            return false;
        };
        let Some(label) = claimed_header_label(self.crop, self.words, &row_track, &row_interior, expected) else {
            return false;
        };
        let body_interior = interior_cell_rows(self.crop, body.y1, body.y2);
        let shoulder = (COLUMN_RULE_CONTACT_PT * self.crop.sx).ceil().max(1.0) as u32;
        let seam = column_boundary_track(self.crop, (cell.x2 + next_cell.x1) / 2.0, shoulder);
        if body_interior.len() < MIN_COLUMN_BAND_PIXELS
            || seam.clone().any(|x| vertical_rule_crosses(self.crop, x, &row_interior))
            || word_gap_has_rule(self.crop, &label, &row_interior)
        {
            return false;
        }

        let band = tatr_rule_band(self.crop, closing.band);
        let masks = word_masks(self.crop, &before_body, &horizontal, &band);
        has_tatr_span_rule_run(
            self.crop,
            horizontal,
            band,
            |x, rows| column_visible(&masks, x, rows),
            |rows| {
                seam.clone().any(|x| {
                    column_rule_contacts(self.crop, x, shoulder, rows.end, &body_interior)
                        && label_crosses_seam(self.crop, &label, x)
                        && column_background_visible(self.crop, x, &row_interior)
                })
            },
        )
    }
}

const COLUMN_RULE_CONTACT_PT: f32 = 2.0;
const MIN_COLUMN_BAND_PIXELS: usize = 3;

fn crop_x_to_page(crop: &TatrCrop, left: f32, right: f32) -> (f32, f32) {
    let page_x = |x| (x + crop.px_left as f32) / crop.sx;
    (page_x(left), page_x(right))
}

fn painted_header_row(
    crop: &TatrCrop,
    track: &PixelRange,
    edges: (f32, f32),
    below_centers: (f32, f32),
    words: &[&HocrWord],
) -> Option<PixelRange> {
    let start = header_row_cap(crop, track, edges.0, below_centers.0, words)?.end;
    let end = header_row_cap(crop, track, edges.1, below_centers.1, words)?.start;
    let interior = start..end;
    (interior.len() >= MIN_COLUMN_BAND_PIXELS).then_some(interior)
}

fn header_row_cap(
    crop: &TatrCrop,
    track: &PixelRange,
    edge: f32,
    below_center: f32,
    words: &[&HocrWord],
) -> Option<PixelRange> {
    const ROW_CAP_SEARCH_PT: f32 = 1.5;
    let radius = ROW_CAP_SEARCH_PT * crop.sy;
    let band =
        ((edge - radius).floor() as u32).max(1)..((edge + radius).ceil() as u32).min(crop.height.saturating_sub(1));
    // The lower row's em boxes may overlap the rule even when its glyphs do not. ~keep
    let above: Vec<_> = words
        .iter()
        .copied()
        .filter(|word| source_word_center_y(word) < below_center)
        .collect();
    let masks = word_masks(crop, &above, track, &band);
    let evidence = std::cell::RefCell::new((None, false));
    has_tatr_span_rule_run(
        crop,
        track.clone(),
        band,
        |x, rows| column_visible(&masks, x, rows),
        |rows| {
            let (cap, ambiguous) = &mut *evidence.borrow_mut();
            *ambiguous |= cap.as_ref().is_some_and(|previous| *previous != rows);
            *cap = Some(rows);
            false
        },
    );
    let (cap, ambiguous) = evidence.into_inner();
    cap.filter(|_| !ambiguous)
}

fn column_rule_contacts(crop: &TatrCrop, x: u32, shoulder: u32, end: u32, interior: &PixelRange) -> bool {
    let Some(gap) = interior.start.checked_sub(end) else {
        return false;
    };
    let max_gap = (COLUMN_RULE_CONTACT_PT * crop.sy).ceil() as u32;
    let ink = |y| tatr_pixel_below_white_cutoff(crop, x, y);
    // The painted vertical rule starts at the bottom edge of the same horizontal rule. ~keep
    gap <= max_gap
        && (end..=interior.start).all(ink)
        && has_contiguous_tatr_rule_coverage(interior.clone(), |y| {
            ink(y)
                && !tatr_pixel_below_white_cutoff(crop, x - shoulder, y)
                && !tatr_pixel_below_white_cutoff(crop, x + shoulder, y)
        })
}

fn vertical_rule_crosses(crop: &TatrCrop, x: u32, rows: &PixelRange) -> bool {
    has_contiguous_tatr_rule_coverage(rows.clone(), |y| tatr_pixel_below_white_cutoff(crop, x, y))
}

fn source_word_center_y(word: &HocrWord) -> f32 {
    word.top as f32 + word.height as f32 / 2.0
}

fn claimed_header_label<'a>(
    crop: &TatrCrop,
    words: &[&'a HocrWord],
    track: &PixelRange,
    interior: &PixelRange,
    expected: &[String],
) -> Option<Vec<&'a HocrWord>> {
    let mut claimed: Vec<_> = words
        .iter()
        .copied()
        .filter(|word| {
            let x = (word.left as f32 + word.width as f32 / 2.0) * crop.sx - crop.px_left as f32;
            let y = source_word_center_y(word) * crop.sy - crop.px_top as f32;
            x >= track.start as f32
                && x < track.end as f32
                && y >= interior.start as f32
                && y < interior.end as f32
                && !word.text.trim().is_empty()
        })
        .collect();
    claimed.sort_unstable_by_key(|word| (word.top, word.left));
    let source_text = claimed.iter().flat_map(|word| word.text.split_whitespace());
    let planned_text = expected.iter().flat_map(|cell| cell.split_whitespace());
    (!claimed.is_empty() && source_text.eq(planned_text)).then_some(claimed)
}

fn word_gap_has_rule(crop: &TatrCrop, words: &[&HocrWord], rows: &PixelRange) -> bool {
    // A divider can sit between two words away from the modeled column seam. ~keep
    words.windows(2).any(|pair| {
        let [left, right] = pair else {
            return false;
        };
        let gap_start = left.left.saturating_add(left.width);
        left.top == right.top
            && gap_start < right.left
            && tatr_rule_track(crop, (gap_start as f32, right.left as f32))
                .any(|x| vertical_rule_crosses(crop, x, rows))
    })
}

fn label_crosses_seam(crop: &TatrCrop, words: &[&HocrWord], x: u32) -> bool {
    // Every source line must be one compact label across the painted seam. ~keep
    let boundary = (x as f32 + 0.5 + crop.px_left as f32) / crop.sx;
    words.chunk_by(|left, right| left.top == right.top).all(|line| {
        line.iter().any(|word| {
            (word.left as f32) < boundary
                && boundary < word.left.saturating_add(word.width) as f32
                && word.text.chars().any(char::is_alphabetic)
        }) && line.windows(2).all(|pair| {
            let [left, right] = pair else {
                return false;
            };
            let gap = right.left.saturating_sub(left.left.saturating_add(left.width));
            gap <= left.height.min(right.height)
        })
    })
}

fn column_background_visible(crop: &TatrCrop, x: u32, interior: &PixelRange) -> bool {
    // Centered glyphs can cover the seam. Require visible light background as
    // well as the absence of a continuous crossing rule. ~keep
    const CLEAR_COLUMN_FRACTION_DENOMINATOR: usize = 4;
    interior
        .clone()
        .filter(|&y| {
            crop.image
                .get_pixel(x, y)
                .0
                .iter()
                .all(|&channel| channel >= TATR_NEAR_WHITE_CHANNEL)
        })
        .count()
        > interior.len() / CLEAR_COLUMN_FRACTION_DENOMINATOR
}

fn column_boundary_track(crop: &TatrCrop, x: f32, shoulder: u32) -> PixelRange {
    // Allow for drift between the model boundary and the painted rule. ~keep
    const COLUMN_RULE_SEARCH_PT: f32 = 4.0;
    let radius = COLUMN_RULE_SEARCH_PT * crop.sx;
    let left = ((x - radius).floor() as u32).max(shoulder);
    let right = ((x + radius).ceil() as u32).min(crop.width.saturating_sub(shoulder));
    left..right
}

fn interior_cell_rows(crop: &TatrCrop, top: f32, bottom: f32) -> PixelRange {
    // Leave out horizontal rules at the outer edges of the row. ~keep
    let inset = (bottom - top) / 8.0;
    let top = ((top + inset).ceil() as u32).min(crop.height);
    let bottom = ((bottom - inset).floor() as u32).min(crop.height);
    top..bottom
}

#[cfg(test)]
mod tests {
    use crate::pdf::table_reconstruct::HocrWord;

    use super::super::tests::boundary_crop;
    use super::super::{TatrCrop, TatrInference, apply_structure};

    #[test]
    fn split_rowspan_retains_only_column_spans_with_open_source_rules() {
        for (body_rule_start, row_rule, label, header_rows, expected) in [
            (20, None, "crossing", 2, [2, 2]),
            (20, Some(25), "crossing", 2, [1, 2]),
            (20, Some(22), "crossing", 2, [1, 2]),
            (20, None, "crossing-plus-independent", 2, [1, 2]),
            (20, None, "interleaved", 2, [1, 2]),
            (20, None, "opposite", 2, [1, 2]),
            (20, None, "empty", 2, [1, 2]),
            (22, None, "crossing", 2, [1, 1]),
            (30, None, "crossing", 2, [1, 1]),
            (20, None, "crossing", 0, [1, 1]),
        ] {
            let crop = split_column_crop(body_rule_start, row_rule, label == "crossing");
            let (mut dense, words) = split_column_labels(label);
            let words: Vec<_> = words.iter().collect();
            let bands = [Some((5.0, 8.0)), Some((15.0, 15.0)), Some((23.0, 27.0))];
            let inference = split_column_inference(header_rows);
            let grid = apply_structure(&mut dense, &bands, &words, &inference, &crop, 30.0);
            assert!(grid.cells.iter().all(|cell| cell.row_span == 1));
            let spans: Vec<_> = grid
                .cells
                .iter()
                .filter(|cell| cell.col == 0 && cell.row < 2)
                .map(|cell| cell.col_span)
                .collect();
            assert_eq!(spans, expected, "{body_rule_start} {row_rule:?} {label} {header_rows}");
            let second = grid.cells.iter().find(|cell| cell.row == 1 && cell.col == 0).unwrap();
            assert_eq!(second.content, "Second");
        }
    }

    #[test]
    fn split_header_rejects_a_displaced_rule_between_native_words() {
        let bands = [Some((5.0, 5.0)), Some((15.0, 15.0)), Some((23.0, 27.0))];
        let inference = split_column_inference(2);
        for (rule, expected) in [(None, 2), (Some(0), 2), (Some(19), 1)] {
            let crop = split_column_crop(20, rule, false);
            let (mut dense, words) = split_column_labels("compact-gap");
            let words: Vec<_> = words.iter().collect();
            let grid = apply_structure(&mut dense, &bands, &words, &inference, &crop, 30.0);
            let first = grid.cells.iter().find(|cell| cell.row == 0 && cell.col == 0).unwrap();
            assert_eq!((first.row_span, first.col_span), (1, expected), "rule x={rule:?}");
            let lower = grid.cells.iter().find(|cell| cell.row == 1 && cell.col == 0).unwrap();
            assert_eq!((lower.row_span, lower.col_span), (1, 2), "rule x={rule:?}");
        }
    }

    fn split_column_inference(header_row_count: usize) -> TatrInference {
        use crate::layout::models::tatr::{CellBBox, TableStructure};

        TatrInference {
            cells: [(2.0, 10.0), (10.0, 20.0), (20.0, 30.0)]
                .map(|(y1, y2)| {
                    [(0.0, 25.0), (25.0, 50.0)]
                        .map(|(x1, x2)| CellBBox { x1, y1, x2, y2 })
                        .to_vec()
                })
                .to_vec(),
            structure: TableStructure {
                spans: vec![(0, 2, 0, 2)],
                header_row_count,
                ..Default::default()
            },
        }
    }

    fn split_column_crop(body_rule_start: u32, row_rule: Option<u32>, crossing_label: bool) -> TatrCrop {
        let mut crop = boundary_crop(50, &[(2, 0, 50), (10, 0, 50), (20, 0, 50)]);
        for y in body_rule_start..30 {
            for x in 25..27 {
                crop.image.put_pixel(x, y, image::Rgb([0; 3]));
            }
        }
        // A centered glyph leaves fewer than half the pixels clear. ~keep
        for y in 3..8 {
            for x in 25..27 {
                crop.image.put_pixel(x, y, image::Rgb([0; 3]));
            }
        }
        if crossing_label {
            for y in 3..8 {
                crop.image.put_pixel(22, y, image::Rgb([0; 3]));
            }
        }
        if let Some(x) = row_rule {
            for y in 2..10 {
                crop.image.put_pixel(x, y, image::Rgb([0; 3]));
            }
        }
        crop
    }

    fn split_column_labels(first: &str) -> ([Vec<String>; 3], Vec<HocrWord>) {
        let word = |text: &str, left, top, width| HocrWord {
            text: text.to_owned(),
            left,
            top,
            width,
            height: 4,
            confidence: 99.0,
        };
        let mut words = vec![
            word("Second", 18, 13, 14),
            HocrWord {
                height: 8,
                ..word("Left", 9, 19, 8)
            },
            HocrWord {
                height: 8,
                ..word("Right", 33, 19, 8)
            },
        ];
        let first = match first {
            "crossing" => {
                words.push(HocrWord {
                    height: 6,
                    ..word("First", 18, 2, 14)
                });
                ["First", ""]
            }
            "compact-gap" => {
                words.extend([
                    HocrWord {
                        height: 6,
                        ..word("Lead", 10, 2, 7)
                    },
                    HocrWord {
                        height: 6,
                        ..word("First", 22, 2, 10)
                    },
                ]);
                ["Lead", "First"]
            }
            "crossing-plus-independent" => {
                words.extend([word("First", 18, 3, 14), word("Other", 2, 3, 5)]);
                ["Other", "First"]
            }
            "interleaved" => {
                words.extend([
                    word("Top", 18, 3, 14),
                    word("Tail", 32, 3, 7),
                    word("Lead", 10, 6, 7),
                    word("Bottom", 18, 6, 14),
                ]);
                ["Top Lead", "Tail Bottom"]
            }
            "opposite" => {
                words.extend([word("First", 12, 3, 8), word("Other", 30, 3, 8)]);
                ["First", "Other"]
            }
            _ => ["", ""],
        };
        let dense = [first, ["Second", ""], ["Left", "Right"]].map(|row| row.map(str::to_owned).to_vec());
        (dense, words)
    }
}
