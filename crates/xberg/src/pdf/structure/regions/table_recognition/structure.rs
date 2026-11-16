use std::collections::BTreeMap;

use crate::pdf::table_reconstruct::HocrWord;
use crate::types::{BoundingBox, GridCell, TableGrid};

use super::{TatrCrop, TatrInference, has_tatr_rule_coverage, has_tatr_rule_run, tatr_rule_band, tatr_rule_track};

type SpanPlan = Vec<Vec<Option<(usize, usize)>>>;

struct CellRegion {
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
}

pub(super) fn apply_structure(
    dense: &mut [Vec<String>],
    source_row_bands: &[Option<(f32, f32)>],
    words: &[&HocrWord],
    inference: &TatrInference,
    crop: &TatrCrop,
    page_height: f32,
) -> TableGrid {
    let rows = dense.len();
    let cols = dense.first().map_or(0, Vec::len);
    let mut plan = vec![vec![Some((1, 1)); cols]; rows];
    let detected_headers = apply_model_spans(dense, &mut plan, source_row_bands, words, inference, crop);
    let header_rows = header_row_count(dense, &plan, detected_headers);
    let mut cells = Vec::new();
    for (row, columns) in plan.into_iter().enumerate() {
        for (col, span) in columns.into_iter().enumerate() {
            let Some((row_span, col_span)) = span else { continue };
            let region = CellRegion {
                row,
                col,
                row_span,
                col_span,
            };
            cells.push(grid_cell(
                dense,
                inference,
                crop,
                page_height,
                &region,
                row < header_rows,
            ));
        }
    }
    TableGrid {
        rows: rows as u32,
        cols: cols as u32,
        cells,
    }
}

fn apply_model_spans(
    dense: &mut [Vec<String>],
    plan: &mut SpanPlan,
    bands: &[Option<(f32, f32)>],
    words: &[&HocrWord],
    inference: &TatrInference,
    crop: &TatrCrop,
) -> usize {
    let mut detected_headers = inference.structure.header_row_count;
    let mut paint = SpanPaint {
        bands,
        words,
        inference,
        crop,
        dividers: BTreeMap::new(),
        openings: BTreeMap::new(),
    };
    for &(row_start, row_end, col_start, col_end) in &inference.structure.spans {
        if row_start >= row_end || row_end > dense.len() || col_start >= col_end {
            continue;
        }
        let boundaries = row_start + 1..row_end;
        if !boundaries.clone().any(|row| paint.divides((row, col_start, col_end))) {
            merge_span(dense, plan, (row_start, row_end, col_start, col_end));
            continue;
        }

        // Once paint contradicts the proposal, only bridge independently open boundaries. ~keep
        let mut start = row_start;
        for row in boundaries.chain(std::iter::once(row_end)) {
            let boundary = (row, col_start, col_end);
            let divided = row != row_end && paint.divides(boundary);
            if divided && row < detected_headers && separates_years_from_amounts(&dense[row - 1], &dense[row]) {
                detected_headers = row;
            }
            if row != row_end && !divided && paint.opens(boundary) {
                continue;
            }
            if row - start > 1 {
                merge_span(dense, plan, (start, row, col_start, col_end));
            }
            start = row;
        }
    }
    detected_headers
}

type SpanBoundary = (usize, usize, usize);

struct SpanPaint<'a> {
    bands: &'a [Option<(f32, f32)>],
    words: &'a [&'a HocrWord],
    inference: &'a TatrInference,
    crop: &'a TatrCrop,
    dividers: BTreeMap<SpanBoundary, bool>,
    openings: BTreeMap<SpanBoundary, bool>,
}

impl SpanPaint<'_> {
    fn divides(&mut self, boundary: SpanBoundary) -> bool {
        *self.dividers.entry(boundary).or_insert_with(|| {
            span_boundary_geometry(self.inference, self.crop, self.bands, boundary)
                .is_some_and(|geometry| geometry.has_divider(self.crop, self.words))
        })
    }

    fn opens(&mut self, boundary: SpanBoundary) -> bool {
        *self.openings.entry(boundary).or_insert_with(|| {
            span_boundary_geometry(self.inference, self.crop, self.bands, boundary)
                .is_some_and(|geometry| geometry.has_open_track(self.crop, self.words))
        })
    }
}

struct SpanBoundaryGeometry {
    table: (f32, f32),
    band: (f32, f32),
    track: (f32, f32),
}

impl SpanBoundaryGeometry {
    fn has_divider(&self, crop: &TatrCrop, words: &[&HocrWord]) -> bool {
        let track = tatr_rule_track(crop, self.track);
        let band = tatr_rule_band(crop, self.band);
        let masks = word_masks(crop, words, &track, &band);
        has_tatr_rule_run(crop, track, band, |x, rows| column_visible(&masks, x, rows), |_| true)
    }

    fn has_open_track(&self, crop: &TatrCrop, words: &[&HocrWord]) -> bool {
        const MIN_NEIGHBOR_RULE_PT: f32 = 10.0;
        let band = tatr_rule_band(crop, self.band);
        let mut neighbors = [(self.table.0, self.track.0), (self.track.1, self.table.1)]
            .into_iter()
            .filter(|&(left, right)| right - left >= MIN_NEIGHBOR_RULE_PT)
            .map(|track| {
                let track = tatr_rule_track(crop, track);
                let masks = word_masks(crop, words, &track, &band);
                (track, masks)
            });
        let Some((first, first_masks)) = neighbors.next() else {
            return false;
        };
        let second = neighbors.next();
        let track = tatr_rule_track(crop, self.track);
        let masks = word_masks(crop, words, &track, &band);
        let clear = band.clone().all(|y| {
            track.clone().all(|x| {
                !column_visible(&masks, x, &(y..y + 1))
                    || crop.image.get_pixel(x, y).0.iter().all(|&channel| channel >= 220)
            })
        });
        let visibly_open = |rows: PixelRange| {
            let visible = track.clone().filter(|&x| column_visible(&masks, x, &rows)).count() as u32;
            has_tatr_rule_coverage(visible, track.end.saturating_sub(track.start))
        };
        clear
            && has_tatr_rule_run(
                crop,
                first,
                band.clone(),
                |x, rows| column_visible(&first_masks, x, rows),
                |rows| {
                    second.as_ref().map_or_else(
                        || visibly_open(rows.clone()),
                        |(second, second_masks)| {
                            has_tatr_rule_run(
                                crop,
                                second.clone(),
                                band.clone(),
                                |x, rows| column_visible(second_masks, x, rows),
                                |other| {
                                    let overlap = rows.start.max(other.start)..rows.end.min(other.end);
                                    !overlap.is_empty() && visibly_open(overlap)
                                },
                            )
                        },
                    )
                },
            )
    }
}

type PixelRange = std::ops::Range<u32>;

fn column_visible(masks: &[(PixelRange, PixelRange)], x: u32, rows: &PixelRange) -> bool {
    !masks.iter().any(|(columns, masked_rows)| {
        columns.contains(&x) && rows.start < masked_rows.end && masked_rows.start < rows.end
    })
}

fn word_masks(
    crop: &TatrCrop,
    words: &[&HocrWord],
    track: &PixelRange,
    band: &PixelRange,
) -> Vec<(PixelRange, PixelRange)> {
    let pixel_x = |x: u32| x as f32 * crop.sx - crop.px_left as f32;
    let pixel_y = |y: u32| y as f32 * crop.sy - crop.px_top as f32;
    words
        .iter()
        .filter_map(|word| {
            let columns =
                pixel_x(word.left).floor() as u32..pixel_x(word.left.saturating_add(word.width)).ceil() as u32;
            let rows = pixel_y(word.top).floor() as u32..pixel_y(word.top.saturating_add(word.height)).ceil() as u32;
            let columns = columns.start.max(track.start)..columns.end.min(track.end);
            let rows = rows.start.max(band.start)..rows.end.min(band.end);
            (!columns.is_empty() && !rows.is_empty()).then_some((columns, rows))
        })
        .collect()
}

fn span_boundary_geometry(
    inference: &TatrInference,
    crop: &TatrCrop,
    bands: &[Option<(f32, f32)>],
    (boundary, col_start, col_end): SpanBoundary,
) -> Option<SpanBoundaryGeometry> {
    let upper = boundary.checked_sub(1)?;
    let (_, above) = bands.get(upper).copied().flatten()?;
    let (below, _) = bands.get(boundary).copied().flatten()?;
    const MAX_ADJACENT_BAND_GAP_PT: f32 = 24.0;
    if below - above > MAX_ADJACENT_BAND_GAP_PT {
        return None;
    }
    let row = inference.cells.get(upper)?;
    let left = row.get(col_start)?;
    let right = row.get(col_end.checked_sub(1)?)?;
    let page_x = |x| (x + crop.px_left as f32) / crop.sx;
    Some(SpanBoundaryGeometry {
        table: (page_x(row.first()?.x1), page_x(row.last()?.x2)),
        band: (above, below),
        track: (page_x(left.x1), page_x(right.x2)),
    })
}

fn separates_years_from_amounts(years: &[String], amounts: &[String]) -> bool {
    years.len() >= 3
        && has_distinct_year_labels(years)
        && is_year_aligned_record(amounts)
        && amounts[1..]
            .iter()
            .all(|amount| amount.bytes().any(|byte| byte.is_ascii_digit()))
        && amounts[1..].iter().any(|amount| amount.contains(['$', '€', '£', '¥']))
}

fn has_distinct_year_labels(years: &[String]) -> bool {
    years[1..]
        .iter()
        .all(|text| text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit()))
        && years[1..].windows(2).all(|pair| pair[0] != pair[1])
}

// Two consecutive record labels in a later section corroborate the earlier
// section that the model included in the header band.
fn header_row_count(dense: &[Vec<String>], plan: &SpanPlan, detected: usize) -> usize {
    for (row, bands) in dense.windows(4).enumerate().take(detected.saturating_sub(1)) {
        let [years, section, first, second] = bands else {
            continue;
        };
        if years.len() >= 3
            && has_distinct_year_labels(years)
            && is_section_stub(section)
            && plan[row..row + 4].iter().flatten().all(|span| *span == Some((1, 1)))
            && [first, second].into_iter().all(|record| is_year_aligned_record(record))
            && first[0].trim() != second[0].trim()
            && dense[(row + 4).max(detected).min(dense.len())..]
                .windows(3)
                .any(|later| {
                    is_section_stub(&later[0])
                        && later[0][0].trim() != section[0].trim()
                        && later[1][0].trim() == first[0].trim()
                        && is_year_aligned_record(&later[1])
                        && later[2][0].trim() == second[0].trim()
                        && has_year_aligned_values(&later[2])
                        && later[2][1..].iter().any(|value| !value.trim().is_empty())
                })
        {
            return row + 1;
        }
    }
    detected
}

fn is_section_stub(row: &[String]) -> bool {
    row[0].chars().any(char::is_alphabetic) && row[1..].iter().all(|text| text.trim().is_empty())
}

fn is_year_aligned_record(row: &[String]) -> bool {
    row[0].chars().any(char::is_alphabetic)
        && has_year_aligned_values(row)
        && row[1..]
            .iter()
            .any(|value| value.bytes().any(|byte| byte.is_ascii_digit()))
}

fn has_year_aligned_values(row: &[String]) -> bool {
    row[1..].iter().all(|value| {
        let value = value.trim();
        value.is_empty()
            || matches!(value, "-" | "–" | "—")
            || (value.bytes().any(|byte| byte.is_ascii_digit())
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || "$€£¥,.()+-% ".contains(ch)))
    })
}

fn merge_span(
    dense: &mut [Vec<String>],
    plan: &mut SpanPlan,
    (row_start, row_end, col_start, col_end): (usize, usize, usize, usize),
) {
    let cols = dense.first().map_or(0, Vec::len);
    if row_start >= row_end || col_start >= col_end || row_end > dense.len() || col_end > cols {
        return;
    }
    if plan[row_start..row_end]
        .iter()
        .any(|row| row[col_start..col_end].iter().any(|cell| *cell != Some((1, 1))))
    {
        return;
    }
    let mut fragments = Vec::new();
    for row in row_start..row_end {
        for col in col_start..col_end {
            plan[row][col] = None;
            let text = std::mem::take(&mut dense[row][col]);
            if !text.is_empty() {
                fragments.push(text);
            }
        }
    }
    dense[row_start][col_start] = fragments.join(" ");
    plan[row_start][col_start] = Some((row_end - row_start, col_end - col_start));
}

fn grid_cell(
    dense: &[Vec<String>],
    inference: &TatrInference,
    crop: &TatrCrop,
    page_height: f32,
    region: &CellRegion,
    is_header: bool,
) -> GridCell {
    GridCell {
        content: dense[region.row][region.col].clone(),
        row: region.row as u32,
        col: region.col as u32,
        row_span: region.row_span as u32,
        col_span: region.col_span as u32,
        is_header,
        bbox: cell_bbox(inference, crop, page_height, region),
        heading_level: None,
        style_name: None,
    }
}

fn cell_bbox(inference: &TatrInference, crop: &TatrCrop, page_height: f32, region: &CellRegion) -> Option<BoundingBox> {
    let first = inference.cells.get(region.row)?.get(region.col)?;
    let last = inference
        .cells
        .get(region.row + region.row_span - 1)?
        .get(region.col + region.col_span - 1)?;
    Some(BoundingBox {
        x0: f64::from((first.x1 + crop.px_left as f32) / crop.sx),
        y0: f64::from(page_height - (last.y2 + crop.px_top as f32) / crop.sy),
        x1: f64::from((last.x2 + crop.px_left as f32) / crop.sx),
        y1: f64::from(page_height - (first.y1 + crop.px_top as f32) / crop.sy),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary_crop(width: u32, strokes: &[(u32, u32, u32)]) -> TatrCrop {
        let height = 30;
        let mut image = image::RgbImage::from_pixel(width, height, image::Rgb([255; 3]));
        for &(y, left, right) in strokes {
            for x in left..right {
                image.put_pixel(x, y, image::Rgb([0; 3]));
            }
        }
        TatrCrop {
            image,
            px_left: 0,
            px_top: 0,
            width,
            height,
            sx: 1.0,
            sy: 1.0,
            extended_bottom_pt: height as f32,
        }
    }

    fn boundary_word(left: u32, width: u32) -> HocrWord {
        HocrWord {
            text: "-".to_owned(),
            left,
            top: 14,
            width,
            height: 2,
            confidence: 99.0,
        }
    }

    #[test]
    fn narrow_track_glyph_ink_does_not_count_as_a_divider() {
        use crate::layout::models::tatr::CellBBox;

        let inference = TatrInference {
            cells: [(0.0, 15.0), (15.0, 30.0)]
                .map(|(y1, y2)| {
                    vec![CellBBox {
                        x1: 23.0,
                        y1,
                        x2: 27.0,
                        y2,
                    }]
                })
                .to_vec(),
            structure: Default::default(),
        };
        let crop = boundary_crop(50, &[(15, 23, 27)]);
        let divides = |words: &[&HocrWord]| {
            SpanPaint {
                bands: &[Some((2.0, 8.0)), Some((22.0, 28.0))],
                words,
                inference: &inference,
                crop: &crop,
                dividers: BTreeMap::new(),
                openings: BTreeMap::new(),
            }
            .divides((1, 0, 1))
        };
        assert!(divides(&[]));
        assert!(!divides(&[&boundary_word(22, 6)]));
    }

    #[test]
    fn staggered_flank_rules_do_not_prove_an_opening() {
        let geometry = SpanBoundaryGeometry {
            table: (0.0, 50.0),
            band: (8.0, 22.0),
            track: (15.0, 35.0),
        };
        let aligned = boundary_crop(50, &[(13, 0, 15), (13, 35, 50)]);
        let staggered = boundary_crop(50, &[(13, 0, 15), (16, 35, 50)]);
        assert!(geometry.has_open_track(&aligned, &[]));
        assert!(!geometry.has_open_track(&staggered, &[]));
    }

    #[test]
    fn masked_divider_does_not_prove_an_opening() {
        let geometry = SpanBoundaryGeometry {
            table: (0.0, 50.0),
            band: (8.0, 22.0),
            track: (15.0, 35.0),
        };
        let crop = boundary_crop(50, &[(15, 0, 50)]);
        let word = boundary_word(15, 20);
        assert!(!geometry.has_divider(&crop, &[&word]));
        assert!(!geometry.has_open_track(&crop, &[&word]));
    }

    #[test]
    fn glyph_ink_in_either_flank_does_not_prove_an_opening() {
        let geometry = SpanBoundaryGeometry {
            table: (0.0, 50.0),
            band: (8.0, 22.0),
            track: (15.0, 35.0),
        };
        let aligned = boundary_crop(50, &[(15, 0, 15), (15, 35, 50)]);
        assert!(geometry.has_open_track(&aligned, &[]));
        for left in [0, 35] {
            assert!(!geometry.has_open_track(&aligned, &[&boundary_word(left, 15)]));
        }

        let edge = SpanBoundaryGeometry {
            track: (15.0, 50.0),
            ..geometry
        };
        let one_flank = boundary_crop(50, &[(15, 0, 15)]);
        assert!(edge.has_open_track(&one_flank, &[]));
        assert!(!edge.has_open_track(&one_flank, &[&boundary_word(0, 15)]));
    }

    #[test]
    fn aligned_flank_rules_require_at_least_eighteen_of_twenty_visible_columns() {
        let geometry = SpanBoundaryGeometry {
            table: (0.0, 50.0),
            band: (8.0, 22.0),
            track: (15.0, 35.0),
        };
        let open = boundary_crop(50, &[(15, 0, 15), (15, 35, 50)]);
        assert!(geometry.has_open_track(&open, &[&boundary_word(15, 2)]));
        assert!(!geometry.has_open_track(&open, &[&boundary_word(15, 3)]));
    }
}
