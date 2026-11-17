use std::collections::BTreeMap;

use crate::pdf::table_reconstruct::HocrWord;
use crate::types::{BoundingBox, GridCell, TableGrid};

use super::{
    TATR_NEAR_WHITE_CHANNEL, TatrCrop, TatrInference, has_tatr_rule_coverage, has_tatr_span_rule_run, tatr_rule_band,
    tatr_rule_track,
};

#[path = "column_spans.rs"]
mod column_spans;

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
    apply_model_spans(dense, &mut plan, source_row_bands, words, inference, crop);
    let header_rows = header_row_count(dense, &plan, inference.structure.header_row_count);
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
) {
    if inference.structure.spans.is_empty() {
        return;
    }
    let occupied: Vec<Vec<bool>> = dense
        .iter()
        .map(|row| row.iter().map(|cell| !cell.trim().is_empty()).collect())
        .collect();
    let mut paint = SpanPaint {
        occupied: &occupied,
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
        let dividers: Vec<_> = (row_start + 1..row_end)
            .filter(|&row| paint.divides((row, col_start, col_end)))
            .collect();
        if dividers.is_empty() {
            merge_span(dense, plan, (row_start, row_end, col_start, col_end));
            continue;
        }

        if row_end - row_start == 2 && col_end - col_start == 2 && row_end <= inference.structure.header_row_count {
            for row in row_start..row_end {
                if paint.has_column_opening(row, col_start..col_end, row_end, dense) {
                    merge_span(dense, plan, (row, row + 1, col_start, col_end));
                }
            }
            continue;
        }

        // Once paint contradicts the proposal, only bridge independently open boundaries. ~keep
        let mut start = row_start;
        for end in dividers.into_iter().chain(std::iter::once(row_end)) {
            let boundaries = start + 1..end;
            for row in boundaries {
                if paint.opens(start..end, (row, col_start, col_end)) {
                    continue;
                }
                if row - start > 1 {
                    merge_span(dense, plan, (start, row, col_start, col_end));
                }
                start = row;
            }
            if end - start > 1 {
                merge_span(dense, plan, (start, end, col_start, col_end));
            }
            start = end;
        }
    }
}

type SpanBoundary = (usize, usize, usize);

struct SpanPaint<'a> {
    occupied: &'a [Vec<bool>],
    bands: &'a [Option<(f32, f32)>],
    words: &'a [&'a HocrWord],
    inference: &'a TatrInference,
    crop: &'a TatrCrop,
    dividers: BTreeMap<SpanBoundary, bool>,
    openings: BTreeMap<(usize, usize, SpanBoundary), bool>,
}

impl SpanPaint<'_> {
    fn divides(&mut self, boundary: SpanBoundary) -> bool {
        *self.dividers.entry(boundary).or_insert_with(|| {
            span_boundary_geometry(self.inference, self.crop, self.bands, boundary)
                .is_some_and(|geometry| geometry.has_divider(self.crop, self.words))
        })
    }

    fn opens(&mut self, segment: std::ops::Range<usize>, boundary: SpanBoundary) -> bool {
        let key = (segment.start, segment.end, boundary);
        if let Some(&open) = self.openings.get(&key) {
            return open;
        }
        let open = span_boundary_geometry(self.inference, self.crop, self.bands, boundary).is_some_and(|geometry| {
            geometry.has_open_track(self.crop, self.words)
                || self.has_adjacent_open_tracks(&segment, boundary, &geometry)
        });
        self.openings.insert(key, open);
        open
    }

    fn has_adjacent_open_tracks(
        &self,
        segment: &std::ops::Range<usize>,
        (row, left, right): SpanBoundary,
        center: &SpanBoundaryGeometry,
    ) -> bool {
        if !self.has_clear_continuation(segment, left, right) {
            return false;
        }
        let mut track = center.track;
        for expand_left in [true, false] {
            let mut edge = if expand_left { left } else { right };
            while let Some((start, end, rows)) = self.adjacent_track(segment, row, edge, expand_left) {
                // Check this boundary before looking farther down for a bottom-aligned label. ~keep
                if !self.has_clear_continuation(&(rows.start..row + 1), start, end)
                    && !self.has_clear_continuation(&rows, start, end)
                {
                    break;
                }
                let Some(neighbor) = span_boundary_geometry(self.inference, self.crop, self.bands, (row, start, end))
                else {
                    break;
                };
                if expand_left {
                    edge = start;
                    track.0 = neighbor.track.0;
                } else {
                    edge = end;
                    track.1 = neighbor.track.1;
                }
            }
        }
        track != center.track && SpanBoundaryGeometry { track, ..*center }.has_open_track(self.crop, self.words)
    }

    fn adjacent_track(
        &self,
        segment: &std::ops::Range<usize>,
        row: usize,
        edge: usize,
        expand_left: bool,
    ) -> Option<(usize, usize, std::ops::Range<usize>)> {
        let mut candidates = self
            .inference
            .structure
            .spans
            .iter()
            .filter_map(|&(start, bottom, left, right)| {
                let touches = if expand_left { right == edge } else { left == edge };
                let rows = start.max(segment.start)..bottom.min(segment.end);
                (rows.start < row && row < rows.end && left < right && touches).then_some((left, right, rows))
            });
        let track = candidates.next()?;
        candidates.all(|candidate| candidate == track).then_some(track)
    }

    fn has_clear_continuation(&self, segment: &std::ops::Range<usize>, left: usize, right: usize) -> bool {
        let mut labels = 0;
        for row in segment.clone() {
            let Some(cells) = self.occupied.get(row).and_then(|cells| cells.get(left..right)) else {
                return false;
            };
            labels += usize::from(cells.iter().any(|&occupied| occupied));
            if labels > 1 {
                return false;
            }
        }
        // The label may be vertically aligned anywhere within the segment. Require clear raster
        // at each internal boundary before letting the union corroborate the outer flanks. ~keep
        labels == 1
            && (segment.start + 1..segment.end).all(|row| {
                span_boundary_geometry(self.inference, self.crop, self.bands, (row, left, right))
                    .is_some_and(|geometry| geometry.has_open_background(self.crop, self.words))
            })
    }
}

#[derive(Clone, Copy)]
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
        has_tatr_span_rule_run(crop, track, band, |x, rows| column_visible(&masks, x, rows), |_| true)
    }

    fn has_open_background(&self, crop: &TatrCrop, words: &[&HocrWord]) -> bool {
        let band = tatr_rule_band(crop, self.band);
        let track = tatr_rule_track(crop, self.track);
        let shoulders = band.start.saturating_sub(1)..band.end + 1;
        let masks = word_masks(crop, words, &track, &shoulders);
        // Label glyphs can overlap the wide search band. Check unmasked background
        // here and require clear shoulders around the actual flank rule in the union. ~keep
        band_has_open_background(crop, &track, &band, &masks)
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
        let shoulders = band.start.saturating_sub(1)..band.end + 1;
        let masks = word_masks(crop, words, &track, &shoulders);
        if !band_has_open_background(crop, &track, &band, &masks) {
            return false;
        }
        let visibly_open = |rows: PixelRange| {
            let rows = rows.start.saturating_sub(1)..rows.end + 1;
            let visible = track.clone().filter(|&x| column_visible(&masks, x, &rows)).count() as u32;
            has_tatr_rule_coverage(visible, track.end.saturating_sub(track.start))
        };
        has_tatr_span_rule_run(
            crop,
            first,
            band.clone(),
            |x, rows| column_visible(&first_masks, x, rows),
            |rows| {
                second.as_ref().map_or_else(
                    || visibly_open(rows.clone()),
                    |(second, second_masks)| {
                        has_tatr_span_rule_run(
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

fn band_has_open_background(
    crop: &TatrCrop,
    track: &PixelRange,
    band: &PixelRange,
    masks: &[(PixelRange, PixelRange)],
) -> bool {
    band_is_near_white(crop, track, band, masks) || band_matches_local_background(crop, track, band, masks)
}

fn band_is_near_white(
    crop: &TatrCrop,
    track: &PixelRange,
    band: &PixelRange,
    masks: &[(PixelRange, PixelRange)],
) -> bool {
    let mut visible = 0;
    for x in track.clone() {
        let mut column = band
            .clone()
            .filter(|&y| column_visible(masks, x, &(y..y + 1)))
            .peekable();
        if column.peek().is_none() {
            continue;
        }
        visible += 1;
        if !column.all(|y| {
            crop.image
                .get_pixel(x, y)
                .0
                .iter()
                .all(|&channel| channel >= TATR_NEAR_WHITE_CHANNEL)
        }) {
            return false;
        }
    }
    has_tatr_rule_coverage(visible, track.end.saturating_sub(track.start))
}

fn band_matches_local_background(
    crop: &TatrCrop,
    track: &PixelRange,
    band: &PixelRange,
    masks: &[(PixelRange, PixelRange)],
) -> bool {
    if band.is_empty() || track.is_empty() {
        return false;
    }
    const MAX_FILL_COLOR_DELTA: u8 = 8;
    const MIN_FILL_CHANNEL: u8 = 128;
    let upper = band.start - 1;
    let lower = band.end;
    let mut visible = 0;
    for x in track.clone() {
        let pixel_visible = |y| column_visible(masks, x, &(y..y + 1));
        // A label can mask one search-band edge without covering the actual crossing. ~keep
        let Some(reference) = [upper, lower].into_iter().find(|&y| pixel_visible(y)) else {
            continue;
        };
        let fill = crop.image.get_pixel(x, reference).0;
        // Dark vertical strokes can persist through an open track, but cannot
        // account for most of the visible background. ~keep
        if fill.iter().all(|&channel| channel < MIN_FILL_CHANNEL) {
            continue;
        }
        let mut visible_band_pixel = false;
        for y in (upper..=lower).filter(|&y| pixel_visible(y)) {
            visible_band_pixel |= band.contains(&y);
            if crop
                .image
                .get_pixel(x, y)
                .0
                .iter()
                .zip(fill)
                .any(|(&channel, background)| channel.abs_diff(background) > MAX_FILL_COLOR_DELTA)
            {
                return false;
            }
        }
        visible += u32::from(visible_band_pixel);
    }
    has_tatr_rule_coverage(visible, track.end.saturating_sub(track.start))
}

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

    pub(super) fn boundary_crop(width: u32, strokes: &[(u32, u32, u32)]) -> TatrCrop {
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
                occupied: &[],
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
    fn near_white_opening_requires_visible_band_columns() {
        let crop = boundary_crop(50, &[]);
        let track = 15..35;
        let band = 8..22;
        assert!(band_is_near_white(&crop, &track, &band, &[(15..17, band.clone())]));
        assert!(!band_is_near_white(&crop, &track, &band, &[(15..18, band.clone())]));
        assert!(!band_is_near_white(
            &crop,
            &track,
            &band,
            &[(track.clone(), band.clone())]
        ));
    }

    #[test]
    fn near_white_opening_checks_word_masks_on_rule_shoulders() {
        let geometry = SpanBoundaryGeometry {
            table: (0.0, 50.0),
            band: (8.0, 22.0),
            track: (15.0, 35.0),
        };
        let crop = boundary_crop(50, &[(15, 0, 15), (15, 35, 50)]);
        assert!(geometry.has_open_track(&crop, &[]));
        let before_rule = HocrWord {
            height: 1,
            ..boundary_word(15, 3)
        };
        assert!(!geometry.has_open_track(&crop, &[&before_rule]));
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
