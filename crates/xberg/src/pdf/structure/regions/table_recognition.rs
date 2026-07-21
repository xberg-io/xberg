//! Table structure recognition for native PDF pages (TATR + SLANeXT backends).

use super::super::geometry::Rect;

#[cfg(feature = "layout-detection")]
use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};
#[cfg(feature = "layout-detection")]
use crate::types::Table;
#[cfg(feature = "layout-detection")]
use crate::utils::escape_html_entities;

#[cfg(feature = "layout-detection")]
/// Calibrated on the BlackRock two-up table (stable 4 pt tracks, 27 pt center
/// whitespace) while keeping the Accenture five-column control unsplit. These are
/// intentionally evidence thresholds, not model confidence cutoffs.
const SPLIT_TRACK_BIN_PTS: f32 = 4.0;
#[cfg(feature = "layout-detection")]
const SPLIT_TRACK_TOLERANCE_PTS: f32 = 8.0;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_TRACK_ROWS: usize = 4;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_NUMERIC_TRACKS: usize = 2;
#[cfg(feature = "layout-detection")]
/// Each child must have enough source and consumed evidence for at least a 2x2
/// topology; coverage prevents a geometrically plausible but mostly empty crop.
const SPLIT_MIN_CHILD_ELIGIBLE_WORDS: usize = 8;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_CHILD_CONSUMED_WORDS: usize = 4;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_CHILD_FILLED_CELLS: usize = 4;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_CONSUMED_FRACTION: f32 = 0.5;
#[cfg(feature = "layout-detection")]
const SPLIT_MAX_CANDIDATES: usize = 3;
#[cfg(feature = "layout-detection")]
/// The BlackRock seam is ~5% of the hint width. Two percent admits rendering
/// jitter but still rejects ordinary inter-column spacing in the Accenture control.
const SPLIT_MIN_SEAM_FRACTION: f32 = 0.02;
#[cfg(feature = "layout-detection")]
const SPLIT_MIN_CHILD_WIDTH_FRACTION: f32 = 0.3;
#[cfg(feature = "layout-detection")]
const SPLIT_TRANSLATION_MIN_FRACTION: f32 = 0.4;
#[cfg(feature = "layout-detection")]
const SPLIT_TRANSLATION_MAX_FRACTION: f32 = 0.6;

#[cfg(feature = "layout-detection")]
struct RecognizedTatrTable {
    table: Table,
    eligible_word_ids: std::collections::BTreeSet<usize>,
    consumed_word_ids: std::collections::BTreeSet<usize>,
}

#[cfg(feature = "layout-detection")]
struct SideBySidePlan {
    children: [LayoutHint; 2],
    ownership: [std::collections::BTreeSet<usize>; 2],
}

/// Compute intersection-over-word-area between an HocrWord and a rectangular region.
///
/// Both word and region must be in the same coordinate space (image coords).
pub(in crate::pdf::structure) fn word_hint_iow(
    w: &crate::pdf::table_reconstruct::HocrWord,
    region_left: f32,
    region_top: f32,
    region_right: f32,
    region_bottom: f32,
) -> f32 {
    let word_rect = Rect::from_xywh(w.left as f32, w.top as f32, w.width as f32, w.height as f32);
    let region_rect = Rect::from_ltrb(region_left, region_top, region_right, region_bottom);
    if word_rect.area() <= 0.0 {
        return if region_rect.contains_point(word_rect.center_x(), word_rect.center_y()) {
            1.0
        } else {
            0.0
        };
    }
    word_rect.intersection_over_self(&region_rect)
}

/// Recognize tables on a native PDF page using TATR structure prediction.
///
/// Crops table regions from the rendered layout detection image, runs TATR
/// inference, then matches predicted cell bboxes against native PDF words.
///
/// # Coordinate conversion
///
/// Three coordinate spaces are involved:
/// - **PDF coords**: LayoutHint bboxes and HocrWord positions (y=0 at bottom for hints;
///   HocrWord uses image-coords with y=0 at top, converted via `page_height - pdf_top`).
/// - **Rendered image pixels**: The ~640px image used for layout detection.
/// - **TATR crop pixels**: Cell bboxes relative to the cropped table region.
#[cfg(feature = "layout-detection")]
pub(in crate::pdf::structure) fn recognize_tables_for_native_page(
    page_image: &image::RgbImage,
    hints: &[LayoutHint],
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_result: &crate::pdf::structure::types::PageLayoutResult,
    page_height: f32,
    page_index: usize,
    tatr_model: &mut crate::layout::models::tatr::TatrModel,
) -> Vec<Table> {
    let rgb_image = page_image;

    let table_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| {
            if h.class_name != LayoutHintClass::Table || h.confidence < 0.5 {
                return false;
            }
            true
        })
        .collect();

    let mut tables = Vec::new();
    let context = TatrPageContext {
        image: rgb_image,
        words,
        page_result,
        page_height,
        page_index,
    };

    for hint in table_hints {
        tables.extend(recognize_hint_with_optional_split(
            hint,
            words,
            page_height,
            |candidate, allowed_word_ids| recognize_tatr_hint(&context, candidate, allowed_word_ids, tatr_model),
        ));
    }

    tables
}

#[cfg(feature = "layout-detection")]
fn recognize_hint_with_optional_split<F>(
    hint: &LayoutHint,
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    mut recognize: F,
) -> Vec<Table>
where
    F: FnMut(&LayoutHint, Option<&std::collections::BTreeSet<usize>>) -> Option<RecognizedTatrTable>,
{
    let plans = side_by_side_table_plans(hint, words, page_height);
    tracing::debug!(
        split_candidates = plans.len(),
        hint_left = hint.left,
        hint_right = hint.right,
        "TATR side-by-side split candidates"
    );
    for plan in plans {
        let left = recognize(&plan.children[0], Some(&plan.ownership[0]));
        let right = recognize(&plan.children[1], Some(&plan.ownership[1]));
        tracing::debug!(
            seam = plan.children[0].right,
            left_eligible = plan.ownership[0].len(),
            right_eligible = plan.ownership[1].len(),
            left_recognized = left.is_some(),
            right_recognized = right.is_some(),
            "TATR side-by-side split recognition"
        );
        if let (Some(left), Some(right)) = (left, right)
            && split_child_is_credible(&left)
            && split_child_is_credible(&right)
            && left.consumed_word_ids.is_disjoint(&right.consumed_word_ids)
        {
            return vec![left.table, right.table];
        }
    }

    recognize(hint, None).map(|result| result.table).into_iter().collect()
}

#[cfg(feature = "layout-detection")]
fn split_child_is_credible(result: &RecognizedTatrTable) -> bool {
    let table = &result.table;
    let columns = table.cells.first().map_or(0, Vec::len);
    let filled_cells = table
        .cells
        .iter()
        .flat_map(|row| row.iter())
        .filter(|cell| !cell.trim().is_empty())
        .count();
    if table.cells.len() < 2
        || columns < 2
        || filled_cells < SPLIT_MIN_CHILD_FILLED_CELLS
        || result.consumed_word_ids.len() < SPLIT_MIN_CHILD_CONSUMED_WORDS
        || result.eligible_word_ids.len() < SPLIT_MIN_CHILD_ELIGIBLE_WORDS
    {
        return false;
    }
    result.consumed_word_ids.len() as f32 / result.eligible_word_ids.len() as f32 >= SPLIT_MIN_CONSUMED_FRACTION
}

#[cfg(feature = "layout-detection")]
struct TatrPageContext<'a> {
    image: &'a image::RgbImage,
    words: &'a [crate::pdf::table_reconstruct::HocrWord],
    page_result: &'a crate::pdf::structure::types::PageLayoutResult,
    page_height: f32,
    page_index: usize,
}

#[cfg(feature = "layout-detection")]
fn recognize_tatr_hint(
    context: &TatrPageContext<'_>,
    hint: &LayoutHint,
    allowed_word_ids: Option<&std::collections::BTreeSet<usize>>,
    tatr_model: &mut crate::layout::models::tatr::TatrModel,
) -> Option<RecognizedTatrTable> {
    let crop = prepare_tatr_crop(
        context.image,
        hint,
        context.words,
        context.page_result,
        context.page_height,
        context.page_index,
    )?;
    let cell_grid = infer_tatr_grid(tatr_model, &crop, context.page_index)?;
    assemble_tatr_table(
        &cell_grid,
        &crop,
        hint,
        context.words,
        allowed_word_ids,
        context.page_height,
        context.page_index,
    )
}

#[cfg(feature = "layout-detection")]
struct TatrCrop {
    image: image::RgbImage,
    px_left: u32,
    px_top: u32,
    width: u32,
    height: u32,
    sx: f32,
    sy: f32,
    extended_bottom_pt: f32,
}

#[cfg(feature = "layout-detection")]
fn prepare_tatr_crop(
    rgb_image: &image::RgbImage,
    hint: &LayoutHint,
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_result: &crate::pdf::structure::types::PageLayoutResult,
    page_height: f32,
    page_index: usize,
) -> Option<TatrCrop> {
    let img_w = rgb_image.width();
    let img_h = rgb_image.height();
    let sx = img_w as f32 / page_result.page_width_pts;
    let sy = img_h as f32 / page_result.page_height_pts;
    let extended_bottom_pt = extend_table_bottom_rows(
        words,
        hint.left,
        hint.right,
        (page_height - hint.top).max(0.0),
        (page_height - hint.bottom).max(0.0),
        page_height,
    );

    let px_left = (hint.left * sx).round().max(0.0) as u32;
    let px_top = ((page_height - hint.top) * sy).round().max(0.0) as u32;
    let px_right = (hint.right * sx).round().min(img_w as f32) as u32;
    let px_bottom = (extended_bottom_pt * sy).round().min(img_h as f32) as u32;

    let crop_w = px_right.saturating_sub(px_left);
    let crop_h = px_bottom.saturating_sub(px_top);
    tracing::debug!(
        page = page_index,
        extended_bottom_pt,
        sy,
        img_h,
        px_top,
        px_bottom,
        crop_h,
        "TATR crop bounds"
    );

    if crop_w < 10 || crop_h < 10 || (crop_w as u64) * (crop_h as u64) > 4_000_000 {
        tracing::debug!(
            page = page_index,
            crop_w,
            crop_h,
            "Skipping TATR for oversized table crop"
        );
        return None;
    }
    Some(TatrCrop {
        image: image::imageops::crop_imm(rgb_image, px_left, px_top, crop_w, crop_h).to_image(),
        px_left,
        px_top,
        width: crop_w,
        height: crop_h,
        sx,
        sy,
        extended_bottom_pt,
    })
}

#[cfg(feature = "layout-detection")]
fn infer_tatr_grid(
    tatr_model: &mut crate::layout::models::tatr::TatrModel,
    crop: &TatrCrop,
    page_index: usize,
) -> Option<Vec<Vec<crate::layout::models::tatr::CellBBox>>> {
    let tatr_result = match tatr_model.recognize(&crop.image) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("TATR inference failed for table on page {}: {e}", page_index);
            return None;
        }
    };

    if tatr_result.rows.is_empty() || tatr_result.columns.is_empty() {
        tracing::debug!(
            page = page_index,
            rows = tatr_result.rows.len(),
            columns = tatr_result.columns.len(),
            "TATR: no rows or columns detected"
        );
        return None;
    }

    let table_bbox_crop = [0.0_f32, 0.0, crop.width as f32, crop.height as f32];
    let cell_grid = crate::layout::models::tatr::build_cell_grid(&tatr_result, Some(table_bbox_crop));
    let num_rows = cell_grid.len();
    let num_cols = if num_rows > 0 { cell_grid[0].len() } else { 0 };

    tracing::debug!(
        page = page_index,
        detected_rows = tatr_result.rows.len(),
        detected_columns = tatr_result.columns.len(),
        grid_rows = num_rows,
        grid_cols = num_cols,
        crop = format!("{}x{}", crop.width, crop.height),
        "TATR inference result"
    );

    if num_rows == 0 || num_cols == 0 {
        return None;
    }
    Some(cell_grid)
}

#[cfg(feature = "layout-detection")]
fn assemble_tatr_table(
    cell_grid: &[Vec<crate::layout::models::tatr::CellBBox>],
    crop: &TatrCrop,
    hint: &LayoutHint,
    words: &[crate::pdf::table_reconstruct::HocrWord],
    allowed_word_ids: Option<&std::collections::BTreeSet<usize>>,
    page_height: f32,
    page_index: usize,
) -> Option<RecognizedTatrTable> {
    let indexed_table_words = collect_tatr_words(hint, words, allowed_word_ids, crop.extended_bottom_pt, page_height);
    let table_words: Vec<_> = indexed_table_words.iter().map(|(_, word)| *word).collect();
    let grid_output = build_tatr_grid_table(
        cell_grid,
        &table_words,
        crop.px_left as f32,
        crop.px_top as f32,
        crop.sx,
        crop.sy,
    );
    if !tatr_grid_is_credible(&grid_output, page_index) {
        return None;
    }
    let bounding_box = tatr_table_bbox(hint, &table_words, &grid_output, page_height);
    let eligible_word_ids = indexed_table_words.iter().map(|(word_id, _)| *word_id).collect();
    let consumed_word_ids = grid_output
        .consumed_word_indices
        .iter()
        .map(|local_id| indexed_table_words[*local_id].0)
        .collect();
    Some(RecognizedTatrTable {
        table: Table {
            cells: grid_output.grid,
            markdown: grid_output.markdown,
            page_number: (page_index + 1) as u32,
            bounding_box: Some(bounding_box),
        },
        eligible_word_ids,
        consumed_word_ids,
    })
}

#[cfg(feature = "layout-detection")]
fn collect_tatr_words<'a>(
    hint: &LayoutHint,
    words: &'a [crate::pdf::table_reconstruct::HocrWord],
    allowed_word_ids: Option<&std::collections::BTreeSet<usize>>,
    extended_bottom_pt: f32,
    page_height: f32,
) -> Vec<(usize, &'a crate::pdf::table_reconstruct::HocrWord)> {
    if let Some(allowed) = allowed_word_ids {
        return words
            .iter()
            .enumerate()
            .filter(|(word_id, word)| allowed.contains(word_id) && !word.text.trim().is_empty())
            .collect();
    }

    let hint_width = hint.right - hint.left;
    let hint_height = hint.top - hint.bottom;
    let pad_x = hint_width * 0.03;
    let pad_y = hint_height * 0.02;
    let padded_left = (hint.left - pad_x).max(0.0);
    let padded_right = hint.right + pad_x;
    let padded_top_pdf = hint.top + pad_y;
    let padded_bottom_pdf = (page_height - extended_bottom_pt - pad_y).max(0.0);

    let hint_img_top = (page_height - padded_top_pdf).max(0.0);
    let hint_img_bottom = (page_height - padded_bottom_pdf).max(0.0);

    words
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            if w.text.trim().is_empty() {
                return false;
            }
            word_hint_iow(w, padded_left, hint_img_top, padded_right, hint_img_bottom) >= 0.2
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
fn tatr_grid_is_credible(grid_output: &TatrGridOutput, page_index: usize) -> bool {
    tracing::debug!(
        page = page_index,
        grid_rows = grid_output.grid.len(),
        grid_cols = grid_output.grid.first().map_or(0, |r| r.len()),
        markdown_len = grid_output.markdown.len(),
        "TATR: word matching and markdown generation"
    );
    if grid_output.markdown.is_empty() {
        tracing::debug!(page = page_index, "TATR: empty markdown output");
        return false;
    }
    let total_cells = grid_output.model_grid_cell_count;
    let filled_cells = grid_output
        .grid
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| !c.trim().is_empty())
        .count();
    if total_cells > 4 && filled_cells < total_cells / 4 {
        tracing::debug!(
            page = page_index,
            total_cells,
            filled_cells,
            "TATR table rejected: too few filled cells"
        );
        return false;
    }
    true
}

#[cfg(feature = "layout-detection")]
fn tatr_table_bbox(
    hint: &LayoutHint,
    table_words: &[&crate::pdf::table_reconstruct::HocrWord],
    grid_output: &TatrGridOutput,
    page_height: f32,
) -> crate::types::BoundingBox {
    let table_width = hint.right - hint.left;
    let col_gap_for_tighten = compute_col_gap_for_word_refs(table_words, table_width);
    let tatr_num_cols = grid_output.grid.first().map_or(0, Vec::len);
    let min_column_gaps = (tatr_num_cols / 2).max(1);
    let tightened_y1 = tighten_table_bbox_top(
        table_words,
        (page_height - hint.top).max(0.0),
        hint.top,
        col_gap_for_tighten,
        min_column_gaps,
        page_height,
    );

    crate::types::BoundingBox {
        x0: hint.left as f64,
        y0: table_bbox_bottom_from_consumed(grid_output.consumed_bottom, hint.bottom, page_height),
        x1: hint.right as f64,
        y1: tightened_y1,
    }
}

#[cfg(feature = "layout-detection")]
fn side_by_side_table_plans(
    hint: &LayoutHint,
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
) -> Vec<SideBySidePlan> {
    let hint_width = hint.right - hint.left;
    if hint_width <= 0.0 {
        return Vec::new();
    }

    let hint_img_top = (page_height - hint.top).max(0.0);
    let hint_img_bottom = extend_table_bottom_rows(
        words,
        hint.left,
        hint.right,
        hint_img_top,
        (page_height - hint.bottom).max(0.0),
        page_height,
    );
    let indexed_table_words: Vec<_> = words
        .iter()
        .enumerate()
        .filter(|(_, word)| {
            !word.text.trim().is_empty()
                && word_hint_iow(word, hint.left, hint_img_top, hint.right, hint_img_bottom) >= 0.5
        })
        .collect();
    if indexed_table_words.is_empty() {
        return Vec::new();
    }
    let table_words: Vec<_> = indexed_table_words.iter().map(|(_, word)| *word).collect();
    let ownership_words = collect_tatr_words(hint, words, None, hint_img_bottom, page_height);

    let semantic_tracks = recurring_tracks(&table_words, |word| {
        word.text.chars().any(char::is_alphabetic).then_some(word.left as f32)
    });
    let numeric_tracks = recurring_tracks(&table_words, |word| {
        word.text
            .chars()
            .any(|ch| ch.is_ascii_digit())
            .then_some((word.left + word.width) as f32)
    });
    let mut seen_seams = std::collections::BTreeSet::new();
    let candidates = ranked_split_candidates(&semantic_tracks, &numeric_tracks, &table_words, hint);
    tracing::debug!(
        semantic_tracks = semantic_tracks.len(),
        numeric_tracks = numeric_tracks.len(),
        ranked_candidates = candidates.len(),
        "TATR side-by-side geometry evidence"
    );
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let seam_bin = (candidate.seam / SPLIT_TRACK_BIN_PTS).round() as i32;
            if !seen_seams.insert(seam_bin) {
                return None;
            }
            split_plan_for_seam(hint, &ownership_words, candidate.seam)
        })
        .take(SPLIT_MAX_CANDIDATES)
        .collect()
}

#[cfg(feature = "layout-detection")]
#[derive(Debug)]
struct RecurringTrack {
    x: f32,
    rows: std::collections::BTreeSet<i32>,
}

#[cfg(feature = "layout-detection")]
fn recurring_tracks<F>(words: &[&crate::pdf::table_reconstruct::HocrWord], mut edge: F) -> Vec<RecurringTrack>
where
    F: FnMut(&crate::pdf::table_reconstruct::HocrWord) -> Option<f32>,
{
    let mut bins: std::collections::BTreeMap<i32, std::collections::BTreeSet<i32>> = std::collections::BTreeMap::new();
    for word in words {
        let Some(x) = edge(word) else {
            continue;
        };
        let x_bin = (x / SPLIT_TRACK_BIN_PTS).round() as i32;
        let row_bin = (word.top as f32 / SPLIT_TRACK_BIN_PTS).round() as i32;
        bins.entry(x_bin).or_default().insert(row_bin);
    }

    bins.into_iter()
        .filter(|(_, rows)| rows.len() >= SPLIT_MIN_TRACK_ROWS)
        .map(|(bin, rows)| RecurringTrack {
            x: bin as f32 * SPLIT_TRACK_BIN_PTS,
            rows,
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
#[derive(Debug)]
struct SplitCandidate {
    seam: f32,
    semantic_support: usize,
    numeric_support: usize,
    half_width_distance: f32,
    gap_width: f32,
}

#[cfg(feature = "layout-detection")]
fn ranked_split_candidates(
    semantic_tracks: &[RecurringTrack],
    numeric_tracks: &[RecurringTrack],
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    hint: &LayoutHint,
) -> Vec<SplitCandidate> {
    let width = hint.right - hint.left;
    let midpoint = (hint.left + hint.right) * 0.5;
    let mut translations = std::collections::BTreeSet::new();
    for left in semantic_tracks.iter().filter(|track| track.x < midpoint) {
        for right in semantic_tracks.iter().filter(|track| track.x > midpoint) {
            let translation = right.x - left.x;
            if translation >= width * SPLIT_TRANSLATION_MIN_FRACTION
                && translation <= width * SPLIT_TRANSLATION_MAX_FRACTION
                && left.rows.intersection(&right.rows).count() >= SPLIT_MIN_TRACK_ROWS
            {
                translations.insert((translation / SPLIT_TRACK_BIN_PTS).round() as i32);
            }
        }
    }

    let mut candidates = Vec::new();
    for translation_bin in translations {
        let translation = translation_bin as f32 * SPLIT_TRACK_BIN_PTS;
        let semantic_support = mirrored_track_count(semantic_tracks, hint, translation);
        let numeric_support = mirrored_track_count(numeric_tracks, hint, translation);
        if semantic_support == 0 || numeric_support < SPLIT_MIN_NUMERIC_TRACKS {
            continue;
        }
        for (seam, gap_width) in central_whitespace_seams(words, hint, hint.left + translation) {
            candidates.push(SplitCandidate {
                seam,
                semantic_support,
                numeric_support,
                half_width_distance: (translation - width * 0.5).abs(),
                gap_width,
            });
        }
    }
    candidates.sort_by(|left, right| {
        right
            .semantic_support
            .cmp(&left.semantic_support)
            .then_with(|| right.numeric_support.cmp(&left.numeric_support))
            .then_with(|| left.half_width_distance.total_cmp(&right.half_width_distance))
            .then_with(|| right.gap_width.total_cmp(&left.gap_width))
            .then_with(|| left.seam.total_cmp(&right.seam))
    });
    candidates
}

#[cfg(feature = "layout-detection")]
fn mirrored_track_count(tracks: &[RecurringTrack], hint: &LayoutHint, translation: f32) -> usize {
    let midpoint = (hint.left + hint.right) * 0.5;
    tracks
        .iter()
        .filter(|left| left.x < midpoint)
        .filter(|left| {
            tracks.iter().any(|right| {
                right.x > midpoint
                    && (right.x - left.x - translation).abs() <= SPLIT_TRACK_TOLERANCE_PTS
                    && left.rows.intersection(&right.rows).count() >= SPLIT_MIN_TRACK_ROWS
            })
        })
        .count()
}

#[cfg(feature = "layout-detection")]
fn central_whitespace_seams(
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    hint: &LayoutHint,
    target: f32,
) -> Vec<(f32, f32)> {
    let width = hint.right - hint.left;
    let search_radius = width * 0.1;
    let mut intervals: Vec<_> = words
        .iter()
        .map(|word| (word.left as f32, (word.left + word.width) as f32))
        .collect();
    intervals.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut merged_right = hint.left;
    let mut seams = Vec::new();
    for (left, right) in intervals {
        if left > merged_right {
            let center = (merged_right + left) * 0.5;
            let gap = left - merged_right;
            if (center - target).abs() <= search_radius && gap >= width * SPLIT_MIN_SEAM_FRACTION {
                seams.push((center, gap));
            }
        }
        merged_right = merged_right.max(right);
    }
    seams
}

#[cfg(feature = "layout-detection")]
fn split_plan_for_seam(
    hint: &LayoutHint,
    words: &[(usize, &crate::pdf::table_reconstruct::HocrWord)],
    seam: f32,
) -> Option<SideBySidePlan> {
    let width = hint.right - hint.left;
    if seam - hint.left < width * SPLIT_MIN_CHILD_WIDTH_FRACTION
        || hint.right - seam < width * SPLIT_MIN_CHILD_WIDTH_FRACTION
    {
        return None;
    }

    let mut ownership = [std::collections::BTreeSet::new(), std::collections::BTreeSet::new()];
    for &(word_id, word) in words {
        let word_left = word.left as f32;
        let word_right = (word.left + word.width) as f32;
        if word_right <= seam {
            ownership[0].insert(word_id);
        } else if word_left >= seam {
            ownership[1].insert(word_id);
        } else {
            return None;
        }
    }
    if ownership
        .iter()
        .any(|child| child.len() < SPLIT_MIN_CHILD_ELIGIBLE_WORDS)
    {
        return None;
    }

    let mut left = hint.clone();
    left.right = seam;
    let mut right = hint.clone();
    right.left = seam;
    Some(SideBySidePlan {
        children: [left, right],
        ownership,
    })
}

/// Build markdown table from TATR cell grid + PDF words.
///
/// Cell bboxes are in crop-pixel space. Words are in PDF image-coord space
/// (HocrWord: left in PDF x-units, top = page_height - pdf_top).
/// Converts cell coords to word space via crop offset + scale factors.
///
/// Uses best-match assignment: each word is assigned to the single cell with
/// the highest IoW overlap, preventing duplication across cells.
///
/// The third return value is the bottom edge (image-y, `top + height`) of the
/// lowest word actually consumed into a cell, used to bound the emitted table
/// bbox to the recognized content.
#[cfg(feature = "layout-detection")]
struct TatrGridOutput {
    grid: Vec<Vec<String>>,
    markdown: String,
    consumed_bottom: Option<u32>,
    consumed_word_indices: Vec<usize>,
    model_grid_cell_count: usize,
}

#[cfg(feature = "layout-detection")]
fn build_tatr_grid_table(
    cell_grid: &[Vec<crate::layout::models::tatr::CellBBox>],
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    crop_offset_px_x: f32,
    crop_offset_px_y: f32,
    sx: f32,
    sy: f32,
) -> TatrGridOutput {
    if cell_grid.is_empty() {
        return TatrGridOutput {
            grid: Vec::new(),
            markdown: String::new(),
            consumed_bottom: None,
            consumed_word_indices: Vec::new(),
            model_grid_cell_count: 0,
        };
    }

    let num_rows = cell_grid.len();
    let num_cols = cell_grid[0].len();
    if num_cols == 0 {
        return TatrGridOutput {
            grid: Vec::new(),
            markdown: String::new(),
            consumed_bottom: None,
            consumed_word_indices: Vec::new(),
            model_grid_cell_count: 0,
        };
    }

    let mut converted_cells: Vec<Vec<(f32, f32, f32, f32)>> = Vec::with_capacity(num_rows);
    for row in cell_grid {
        let mut conv_row = Vec::with_capacity(num_cols);
        for cell in row {
            let cell_left = (cell.x1 + crop_offset_px_x) / sx;
            let cell_right = (cell.x2 + crop_offset_px_x) / sx;
            let cell_top = (cell.y1 + crop_offset_px_y) / sy;
            let cell_bottom = (cell.y2 + crop_offset_px_y) / sy;
            conv_row.push((cell_left, cell_top, cell_right, cell_bottom));
        }
        converted_cells.push(conv_row);
    }

    let mut cell_words: Vec<Vec<Vec<(usize, f32, f32)>>> = (0..num_rows)
        .map(|_| (0..num_cols).map(|_| Vec::new()).collect())
        .collect();
    let mut consumed_bottom: Option<u32> = None;
    let mut word_consumed = vec![false; words.len()];

    for (wi, &word) in words.iter().enumerate() {
        let mut best_iow: f32 = 0.0;
        let mut best_row: usize = 0;
        let mut best_col: usize = 0;

        for (ri, conv_row) in converted_cells.iter().enumerate() {
            for (ci, &(cl, ct, cr, cb)) in conv_row.iter().enumerate() {
                let iow = word_hint_iow(word, cl, ct, cr, cb);
                if iow > best_iow {
                    best_iow = iow;
                    best_row = ri;
                    best_col = ci;
                }
            }
        }

        if best_iow >= 0.2 {
            let cx = word.left as f32 + word.width as f32 / 2.0;
            let cy = word.top as f32 + word.height as f32 / 2.0;
            cell_words[best_row][best_col].push((wi, cx, cy));
            let word_bottom = word.top + word.height;
            consumed_bottom = Some(consumed_bottom.map_or(word_bottom, |b| b.max(word_bottom)));
            word_consumed[wi] = true;
        }
    }

    let mut grid: Vec<Vec<String>> = Vec::with_capacity(num_rows);
    for row_cells in &cell_words {
        let mut grid_row = vec![String::new(); num_cols];
        for (ci, cell_word_indices) in row_cells.iter().enumerate() {
            if cell_word_indices.is_empty() {
                continue;
            }
            let mut sorted = cell_word_indices.clone();
            sorted.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.total_cmp(&b.1)));
            let text: String = sorted
                .iter()
                .map(|(wi, _, _)| words[*wi].text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            grid_row[ci] = text;
        }
        grid.push(grid_row);
    }

    let consumed_bottom =
        append_unconsumed_aligned_rows(&mut grid, words, &mut word_consumed, consumed_bottom, &converted_cells);

    let markdown = render_grid_as_markdown(&grid);
    let consumed_word_indices = word_consumed
        .iter()
        .enumerate()
        .filter_map(|(word_id, consumed)| consumed.then_some(word_id))
        .collect();
    TatrGridOutput {
        grid,
        markdown,
        consumed_bottom,
        consumed_word_indices,
        model_grid_cell_count: num_rows * num_cols,
    }
}

/// Append word rows the recognizer failed to consume below the grid.
///
/// Walks unconsumed word rows below the consumed region top-to-bottom; a row
/// whose words mostly fall within the table's per-column x-spans is rebuilt as
/// a grid row (each word joined into the column containing its center, or the
/// nearest column). The walk stops at the first non-aligned row so trailing
/// prose is never absorbed. Returns the updated consumed-bottom.
#[cfg(feature = "layout-detection")]
fn append_unconsumed_aligned_rows(
    grid: &mut Vec<Vec<String>>,
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    word_consumed: &mut [bool],
    consumed_bottom: Option<u32>,
    converted_cells: &[Vec<(f32, f32, f32, f32)>],
) -> Option<u32> {
    /// A row is appended when at least this fraction of its words fall in column spans.
    const MIN_ALIGNED_FRACTION: f32 = 0.6;
    /// Rows with fewer words than this are never treated as table rows.
    const MIN_ROW_WORDS: usize = 3;
    /// Column x-spans are widened by this slack on both sides.
    const COLUMN_SPAN_SLACK_PTS: f32 = 4.0;

    let mut current_bottom = consumed_bottom?;
    let num_cols = grid.first().map_or(0, |r| r.len());
    if num_cols == 0 {
        return Some(current_bottom);
    }

    let mut col_spans: Vec<(f32, f32)> = Vec::with_capacity(num_cols);
    for col in 0..num_cols {
        let mut lefts: Vec<f32> = converted_cells.iter().filter_map(|r| r.get(col).map(|c| c.0)).collect();
        let mut rights: Vec<f32> = converted_cells.iter().filter_map(|r| r.get(col).map(|c| c.2)).collect();
        if lefts.is_empty() {
            return Some(current_bottom);
        }
        lefts.sort_by(f32::total_cmp);
        rights.sort_by(f32::total_cmp);
        col_spans.push((
            lefts[lefts.len() / 2] - COLUMN_SPAN_SLACK_PTS,
            rights[rights.len() / 2] + COLUMN_SPAN_SLACK_PTS,
        ));
    }

    let mut pending: Vec<usize> = (0..words.len())
        .filter(|&wi| !word_consumed[wi] && !words[wi].text.trim().is_empty() && words[wi].top >= current_bottom)
        .collect();
    if pending.is_empty() {
        return Some(current_bottom);
    }
    pending.sort_by_key(|&wi| words[wi].top);

    let same_row_tolerance = {
        let mut heights: Vec<u32> = pending.iter().map(|&wi| words[wi].height).collect();
        heights.sort_unstable();
        (heights[heights.len() / 2] / 2).clamp(2, 5)
    };

    let column_of = |w: &crate::pdf::table_reconstruct::HocrWord| -> Option<usize> {
        let center = w.left as f32 + w.width as f32 / 2.0;
        col_spans.iter().position(|&(l, r)| center >= l && center <= r)
    };
    let nearest_column = |w: &crate::pdf::table_reconstruct::HocrWord| -> usize {
        let center = w.left as f32 + w.width as f32 / 2.0;
        col_spans
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let d1 = (center - (a.0 + a.1) / 2.0).abs();
                let d2 = (center - (b.0 + b.1) / 2.0).abs();
                d1.total_cmp(&d2)
            })
            .map_or(0, |(i, _)| i)
    };

    let mut appended_rows = 0_usize;
    let mut row_start = 0_usize;
    while row_start < pending.len() {
        let row_anchor = words[pending[row_start]].top;
        let row_end = pending[row_start..]
            .iter()
            .position(|&wi| words[wi].top.saturating_sub(row_anchor) > same_row_tolerance)
            .map(|p| row_start + p)
            .unwrap_or(pending.len());

        let row = &pending[row_start..row_end];
        let aligned = row.iter().filter(|&&wi| column_of(words[wi]).is_some()).count();
        if row.len() < MIN_ROW_WORDS || (aligned as f32 / row.len() as f32) < MIN_ALIGNED_FRACTION {
            break;
        }

        let mut row_cells: Vec<Vec<(u32, &str)>> = vec![Vec::new(); num_cols];
        for &wi in row {
            let w = words[wi];
            let col = column_of(w).unwrap_or_else(|| nearest_column(w));
            row_cells[col].push((w.left, w.text.trim()));
            word_consumed[wi] = true;
        }
        let mut grid_row = Vec::with_capacity(num_cols);
        for mut cell in row_cells {
            cell.sort_by_key(|&(x, _)| x);
            grid_row.push(
                cell.iter()
                    .map(|&(_, t)| t)
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        grid.push(grid_row);
        appended_rows += 1;

        current_bottom = row
            .iter()
            .map(|&wi| words[wi].top + words[wi].height)
            .max()
            .unwrap_or(row_anchor)
            .max(current_bottom);
        row_start = row_end;
    }

    if appended_rows > 0 {
        tracing::debug!(appended_rows, "TATR: appended unconsumed aligned rows to grid");
    }
    Some(current_bottom)
}

/// Detect and fix vertically-oriented table header text.
///
/// PDFs with rotated column headers (common in wide tables) produce garbled
/// text when the PDF extractor extracts characters individually: "y t i r o h t u A o N"
/// instead of "No Authority". Detected by: ≥3 tokens, >70% single characters.
/// Fixed by joining characters and reversing (the chars are in bottom-to-top order).
#[cfg(feature = "layout-detection")]
fn fix_vertical_header_text(text: &str) -> String {
    let trimmed = text.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 3 {
        return text.to_string();
    }
    let single_chars = tokens.iter().filter(|t| t.len() == 1).count();
    let ratio = single_chars as f32 / tokens.len() as f32;
    if ratio > 0.7 {
        let joined: String = tokens.concat();
        joined.chars().rev().collect()
    } else {
        text.to_string()
    }
}

/// Render a grid of cell text strings as a markdown table.
#[cfg(feature = "layout-detection")]
fn render_grid_as_markdown(grid: &[Vec<String>]) -> String {
    if grid.is_empty() {
        return String::new();
    }

    let max_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return String::new();
    }

    let mut md = String::new();

    for (row_idx, row) in grid.iter().enumerate() {
        md.push('|');
        for col in 0..max_cols {
            let raw_cell = row.get(col).map(|s| s.as_str()).unwrap_or("");
            let cell = fix_vertical_header_text(raw_cell);
            let pipe_escaped = cell.replace('|', "\\|");
            let escaped = escape_html_entities(&pipe_escaped);
            md.push(' ');
            md.push_str(escaped.trim());
            md.push_str(" |");
        }
        md.push('\n');

        if row_idx == 0 {
            md.push('|');
            for _ in 0..max_cols {
                md.push_str(" --- |");
            }
            md.push('\n');
        }
    }

    if md.ends_with('\n') {
        md.pop();
    }
    md
}

/// Recognize tables on a native PDF page using SLANeXT structure prediction.
///
/// Unlike TATR (which works on cropped table regions), SLANeXT requires the
/// **full page image** to detect table structure. We run inference once per page,
/// then filter detected cells by RT-DETR table region bounding boxes.
///
/// Cell bboxes from SLANeXT are in full-page image coordinates. We match them
/// to RT-DETR table hint regions, then match words to cells within each table.
///
/// When `classifier` is provided, each table region is classified as wired or
/// wireless and the appropriate SLANeXT variant is used. The classifier runs on
/// the cropped table region (works on crops), then we run full-page inference
/// with the selected model.
#[cfg(feature = "layout-detection")]
#[allow(clippy::too_many_arguments)]
pub(in crate::pdf::structure) fn recognize_tables_slanet(
    page_image: &image::RgbImage,
    hints: &[LayoutHint],
    words: &[crate::pdf::table_reconstruct::HocrWord],
    page_result: &crate::pdf::structure::types::PageLayoutResult,
    page_height: f32,
    page_index: usize,
    slanet_model: &mut crate::layout::models::slanet::SlanetModel,
    classifier: Option<(
        &mut crate::layout::models::table_classifier::TableClassifier,
        &mut crate::layout::models::slanet::SlanetModel,
    )>,
) -> Vec<Table> {
    let rgb_image = page_image;
    let img_w = rgb_image.width();
    let img_h = rgb_image.height();

    let sx = img_w as f32 / page_result.page_width_pts;
    let sy = img_h as f32 / page_result.page_height_pts;

    let table_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| h.class_name == LayoutHintClass::Table && h.confidence >= 0.5)
        .collect();

    if table_hints.is_empty() {
        return Vec::new();
    }

    let active_model: &mut crate::layout::models::slanet::SlanetModel = if let Some((cls, alt_model)) = classifier {
        let first_hint = table_hints[0];
        let px_left = (first_hint.left * sx).round().max(0.0) as u32;
        let px_top = ((page_height - first_hint.top) * sy).round().max(0.0) as u32;
        let px_right = (first_hint.right * sx).round().min(img_w as f32) as u32;
        let px_bottom = ((page_height - first_hint.bottom) * sy).round().min(img_h as f32) as u32;
        let crop_w = px_right.saturating_sub(px_left).max(10);
        let crop_h = px_bottom.saturating_sub(px_top).max(10);
        let crop = image::imageops::crop_imm(rgb_image, px_left, px_top, crop_w, crop_h).to_image();

        match cls.classify(&crop) {
            Ok(crate::layout::models::table_classifier::TableType::Wireless) => {
                tracing::debug!(
                    page = page_index,
                    "TableClassifier: page classified as wireless, using wireless SLANeXT"
                );
                alt_model
            }
            Ok(crate::layout::models::table_classifier::TableType::Wired) => {
                tracing::debug!(
                    page = page_index,
                    "TableClassifier: page classified as wired, using wired SLANeXT"
                );
                slanet_model
            }
            Err(e) => {
                tracing::warn!(page = page_index, "TableClassifier failed: {e}, defaulting to wired");
                slanet_model
            }
        }
    } else {
        slanet_model
    };

    tracing::trace!(
        page = page_index,
        page_image_w = img_w,
        page_image_h = img_h,
        table_hints = table_hints.len(),
        "SLANeXT: running full-page inference"
    );

    let slanet_result = match active_model.recognize(rgb_image) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("SLANeXT inference failed on page {}: {e}", page_index);
            return Vec::new();
        }
    };

    if slanet_result.cells.is_empty() {
        tracing::debug!(
            page = page_index,
            tokens = slanet_result.structure_tokens.len(),
            confidence = format!("{:.3}", slanet_result.confidence),
            "SLANeXT: no cells detected on full page"
        );
        return Vec::new();
    }

    tracing::debug!(
        page = page_index,
        cells = slanet_result.cells.len(),
        rows = slanet_result.num_rows,
        cols = slanet_result.num_cols,
        confidence = format!("{:.3}", slanet_result.confidence),
        "SLANeXT: full-page inference result"
    );

    let mut tables = Vec::new();

    for hint in &table_hints {
        let extended_bottom_pt = extend_table_bottom_rows(
            words,
            hint.left,
            hint.right,
            (page_height - hint.top).max(0.0),
            (page_height - hint.bottom).max(0.0),
            page_height,
        );

        let hint_img_left = hint.left * sx;
        let hint_img_top = (page_height - hint.top) * sy;
        let hint_img_right = hint.right * sx;
        let hint_img_bottom = extended_bottom_pt * sy;

        let mut matching_cells: Vec<&crate::layout::models::slanet::SlanetCell> = Vec::new();
        for cell in &slanet_result.cells {
            let cx = (cell.bbox[0] + cell.bbox[2]) / 2.0;
            let cy = (cell.bbox[1] + cell.bbox[3]) / 2.0;
            if cx >= hint_img_left && cx <= hint_img_right && cy >= hint_img_top && cy <= hint_img_bottom {
                matching_cells.push(cell);
            }
        }

        if matching_cells.is_empty() {
            tracing::trace!(
                page = page_index,
                hint_left = format!("{:.0}", hint.left),
                hint_top = format!("{:.0}", hint.top),
                "SLANeXT: no cells overlap this table hint"
            );
            continue;
        }

        let max_row = matching_cells.iter().map(|c| c.row).max().unwrap_or(0);
        let max_col = matching_cells.iter().map(|c| c.col).max().unwrap_or(0);
        let num_rows = max_row + 1;
        let num_cols = max_col + 1;

        tracing::trace!(
            page = page_index,
            matching_cells = matching_cells.len(),
            num_rows,
            num_cols,
            "SLANeXT: cells matched to table hint"
        );

        let hint_img_top = (page_height - hint.top).max(0.0);
        let hint_img_bottom = extended_bottom_pt;

        let table_words: Vec<&crate::pdf::table_reconstruct::HocrWord> = words
            .iter()
            .filter(|w| {
                if w.text.trim().is_empty() {
                    return false;
                }
                word_hint_iow(w, hint.left, hint_img_top, hint.right, hint_img_bottom) >= 0.2
            })
            .collect();

        let (grid, markdown, consumed_bottom) =
            build_slanet_cells_table(&matching_cells, num_rows, num_cols, &table_words, sx, sy);

        if markdown.is_empty() {
            tracing::debug!(page = page_index, "SLANeXT: empty markdown output for table hint");
            continue;
        }

        let total_cells = num_rows * num_cols;
        let filled_cells = grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| !c.trim().is_empty())
            .count();
        if total_cells > 4 && filled_cells < total_cells / 4 {
            tracing::debug!(
                page = page_index,
                total_cells,
                filled_cells,
                "SLANeXT table rejected: too few filled cells"
            );
            continue;
        }

        let table_width = hint.right - hint.left;
        let col_gap_for_tighten = compute_col_gap_for_word_refs(&table_words, table_width);
        let slanet_num_cols = grid.first().map_or(0, |r| r.len());
        let min_column_gaps = (slanet_num_cols / 2).max(1);
        let tightened_y1 = tighten_table_bbox_top(
            &table_words,
            hint_img_top,
            hint.top,
            col_gap_for_tighten,
            min_column_gaps,
            page_height,
        );

        let bounding_box = Some(crate::types::BoundingBox {
            x0: hint.left as f64,
            y0: table_bbox_bottom_from_consumed(consumed_bottom, hint.bottom, page_height),
            x1: hint.right as f64,
            y1: tightened_y1,
        });

        tables.push(Table {
            cells: grid,
            markdown,
            page_number: (page_index + 1) as u32,
            bounding_box,
        });
    }

    tables
}

/// Build markdown table from SLANeXT cells matched to a single table region.
///
/// `cells` are already filtered to those overlapping the RT-DETR table hint.
/// Cell bboxes are in full-page image pixel coords; convert to PDF coords for
/// word matching.
#[cfg(feature = "layout-detection")]
fn build_slanet_cells_table(
    cells: &[&crate::layout::models::slanet::SlanetCell],
    num_rows: usize,
    num_cols: usize,
    words: &[&crate::pdf::table_reconstruct::HocrWord],
    sx: f32,
    sy: f32,
) -> (Vec<Vec<String>>, String, Option<u32>) {
    if cells.is_empty() || num_rows == 0 || num_cols == 0 {
        return (Vec::new(), String::new(), None);
    }

    let min_row = cells.iter().map(|c| c.row).min().unwrap_or(0);
    let min_col = cells.iter().map(|c| c.col).min().unwrap_or(0);

    let grid_rows = num_rows.min(cells.iter().map(|c| c.row - min_row + 1).max().unwrap_or(1));
    let grid_cols = num_cols.min(cells.iter().map(|c| c.col - min_col + 1).max().unwrap_or(1));

    let mut grid: Vec<Vec<String>> = (0..grid_rows).map(|_| vec![String::new(); grid_cols]).collect();

    let converted_cells: Vec<(usize, usize, f32, f32, f32, f32)> = cells
        .iter()
        .map(|cell| {
            let cell_left = cell.bbox[0] / sx;
            let cell_top = cell.bbox[1] / sy;
            let cell_right = cell.bbox[2] / sx;
            let cell_bottom = cell.bbox[3] / sy;
            (
                cell.row - min_row,
                cell.col - min_col,
                cell_left,
                cell_top,
                cell_right,
                cell_bottom,
            )
        })
        .collect();

    let mut word_assignments: Vec<(usize, usize, f32, f32)> = Vec::new();
    let mut consumed_bottom: Option<u32> = None;

    for (wi, &word) in words.iter().enumerate() {
        let mut best_iow: f32 = 0.0;
        let mut best_cell_idx: usize = 0;

        for (ci, &(_row, _col, cl, ct, cr, cb)) in converted_cells.iter().enumerate() {
            let iow = word_hint_iow(word, cl, ct, cr, cb);
            if iow > best_iow {
                best_iow = iow;
                best_cell_idx = ci;
            }
        }

        if best_iow >= 0.2 {
            let cx = word.left as f32 + word.width as f32 / 2.0;
            let cy = word.top as f32 + word.height as f32 / 2.0;
            word_assignments.push((wi, best_cell_idx, cx, cy));
            let word_bottom = word.top + word.height;
            consumed_bottom = Some(consumed_bottom.map_or(word_bottom, |b| b.max(word_bottom)));
        }
    }

    let mut cell_word_groups: Vec<Vec<(usize, f32, f32)>> = vec![Vec::new(); cells.len()];
    for &(wi, cell_idx, cx, cy) in &word_assignments {
        if cell_idx < cell_word_groups.len() {
            cell_word_groups[cell_idx].push((wi, cx, cy));
        }
    }

    let assigned_count = cell_word_groups.iter().filter(|g| !g.is_empty()).count();
    tracing::trace!(
        total_words = words.len(),
        assigned_words = word_assignments.len(),
        cells_with_words = assigned_count,
        total_cells = cells.len(),
        "SLANeXT: word-to-cell assignment complete"
    );

    for (ci, group) in cell_word_groups.iter_mut().enumerate() {
        group.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.total_cmp(&b.1)));
        let text: String = group
            .iter()
            .map(|(wi, _, _)| words[*wi].text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let (row, col) = (converted_cells[ci].0, converted_cells[ci].1);
        if row < grid_rows && col < grid_cols {
            grid[row][col] = text;
        }
    }

    let markdown = render_grid_as_markdown(&grid);
    (grid, markdown, consumed_bottom)
}

/// Compute the adaptive column-gap threshold for a slice of `&HocrWord` references.
///
/// Mirrors the logic in `tables::compute_adaptive_column_gap` for borrowed slices.
#[cfg(feature = "layout-detection")]
fn compute_col_gap_for_word_refs(words: &[&crate::pdf::table_reconstruct::HocrWord], table_width: f32) -> u32 {
    let mut gaps: Vec<u32> = Vec::new();

    if words.len() >= 4 {
        let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
        heights.sort_unstable();
        let median_h = heights[heights.len() / 2];
        let row_tolerance = (median_h / 2).max(3);

        let mut sorted: Vec<(u32, u32, u32)> = words
            .iter()
            .map(|w| {
                let yc = w.top + w.height / 2;
                (yc, w.left, w.left + w.width)
            })
            .collect();
        sorted.sort_by_key(|&(yc, x, _)| (yc, x));

        let mut row_start = 0;
        while row_start < sorted.len() {
            let row_yc = sorted[row_start].0;
            let mut row_end = row_start + 1;
            while row_end < sorted.len() && sorted[row_end].0.abs_diff(row_yc) <= row_tolerance {
                row_end += 1;
            }
            for i in row_start + 1..row_end {
                let prev_right = sorted[i - 1].2;
                let curr_left = sorted[i].1;
                if curr_left > prev_right {
                    gaps.push(curr_left - prev_right);
                }
            }
            row_start = row_end;
        }
    }

    if gaps.len() >= 3 {
        gaps.sort_unstable();
        let large_gaps: Vec<u32> = gaps.iter().copied().filter(|&g| g >= 40).collect();
        if !large_gaps.is_empty() {
            let median_gap = large_gaps[large_gaps.len() / 2];
            return (median_gap / 2).clamp(20, 60);
        } else {
            let median_gap = gaps[gaps.len() / 2];
            return (median_gap * 3).clamp(20, 60);
        }
    }

    if table_width < 200.0 {
        10
    } else if table_width < 400.0 {
        15
    } else if table_width < 600.0 {
        20
    } else {
        30
    }
}

/// Extend a table's bottom edge across word rows that continue its column structure.
///
/// The layout model's Table bbox often underestimates the table's bottom edge,
/// cutting off the last rows (e.g. the final states on the NICS form). Words
/// below the hint never reach table recognition — the TATR crop and the
/// word-to-cell matching are both bounded by the hint — and the missing rows
/// silently vanish from the output.
///
/// Strategy: learn the table's column x-positions from the words inside the
/// hint (left edges for text columns, right edges for right-aligned numeric
/// columns), then walk word rows strictly below the hint bottom in image-y
/// order. A row continues the table while most of its words align with known
/// column positions; the walk stops at the first non-aligned row so trailing
/// paragraphs are never swallowed. Alignment is robust for dense many-column
/// tables where inter-column gaps are too small for gap-based heuristics.
/// The extension is capped at half the hinted table height.
///
/// All coordinates are in HocrWord space (PDF-point units, image-oriented y).
/// Returns the extended bottom, `>= hint_img_bottom`.
#[cfg(feature = "layout-detection")]
fn extend_table_bottom_rows(
    words: &[crate::pdf::table_reconstruct::HocrWord],
    hint_left: f32,
    hint_right: f32,
    hint_img_top: f32,
    hint_img_bottom: f32,
    page_height: f32,
) -> f32 {
    /// Downward margin so the last continuation row's descenders stay inside the bbox.
    const TABLE_BBOX_BOTTOM_EXTEND_MARGIN_PTS: u32 = 4;
    /// The extension may grow the table by at most this fraction of its hinted height.
    const MAX_EXTENSION_FRACTION: f32 = 0.5;
    /// Column x-positions are quantized to bins of this width.
    const X_BIN_PTS: u32 = 4;
    /// A bin is a column position if words from at least this many rows hit it.
    const MIN_BIN_COUNT: u32 = 3;
    /// A row continues the table when at least this fraction of its words align.
    const MIN_ALIGNED_FRACTION: f32 = 0.6;
    /// Rows with fewer words than this are never treated as table rows.
    const MIN_ROW_WORDS: usize = 3;

    let horizontally_in_hint = |w: &crate::pdf::table_reconstruct::HocrWord| {
        !w.text.trim().is_empty() && (w.left + w.width) as f32 >= hint_left && (w.left as f32) <= hint_right
    };

    let in_hint: Vec<&crate::pdf::table_reconstruct::HocrWord> = words
        .iter()
        .filter(|w| horizontally_in_hint(w) && (w.top as f32) >= hint_img_top && (w.top as f32) < hint_img_bottom)
        .collect();
    if in_hint.len() < 4 {
        return hint_img_bottom;
    }

    let mut left_bins: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    let mut right_bins: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for w in &in_hint {
        *left_bins.entry(w.left / X_BIN_PTS).or_insert(0) += 1;
        *right_bins.entry((w.left + w.width) / X_BIN_PTS).or_insert(0) += 1;
    }
    let bin_hit = |bins: &std::collections::HashMap<u32, u32>, bin: u32| {
        (bin.saturating_sub(1)..=bin + 1).any(|b| bins.get(&b).copied().unwrap_or(0) >= MIN_BIN_COUNT)
    };
    let word_aligns = |w: &crate::pdf::table_reconstruct::HocrWord| {
        bin_hit(&left_bins, w.left / X_BIN_PTS) || bin_hit(&right_bins, (w.left + w.width) / X_BIN_PTS)
    };

    let same_row_tolerance = {
        let mut heights: Vec<u32> = in_hint.iter().map(|w| w.height).collect();
        heights.sort_unstable();
        (heights[heights.len() / 2] / 2).clamp(2, 5)
    };

    let hint_height = (hint_img_bottom - hint_img_top).max(0.0);
    let max_bottom = (hint_img_bottom + hint_height * MAX_EXTENSION_FRACTION).min(page_height);

    let mut below: Vec<&crate::pdf::table_reconstruct::HocrWord> = words
        .iter()
        .filter(|w| horizontally_in_hint(w) && (w.top as f32) >= hint_img_bottom && (w.top as f32) < max_bottom)
        .collect();
    tracing::debug!(
        hint_img_bottom,
        max_bottom,
        same_row_tolerance,
        in_hint_words = in_hint.len(),
        below_words = below.len(),
        "extend_table_bottom_rows: calibration"
    );
    if below.is_empty() {
        return hint_img_bottom;
    }
    below.sort_by_key(|w| w.top);

    let mut extended_bottom = hint_img_bottom;
    let mut row_start = 0_usize;
    while row_start < below.len() {
        let row_anchor = below[row_start].top;
        let row_end = below[row_start..]
            .iter()
            .position(|w| w.top.saturating_sub(row_anchor) > same_row_tolerance)
            .map(|p| row_start + p)
            .unwrap_or(below.len());

        let row_words = row_end - row_start;
        let aligned = below[row_start..row_end].iter().filter(|w| word_aligns(w)).count();
        let aligned_fraction = aligned as f32 / row_words as f32;
        tracing::debug!(row_anchor, row_words, aligned, "extend_table_bottom_rows: row");
        if row_words < MIN_ROW_WORDS || aligned_fraction < MIN_ALIGNED_FRACTION {
            break;
        }
        let row_bottom = below[row_start..row_end]
            .iter()
            .map(|w| w.top + w.height)
            .max()
            .unwrap_or(row_anchor);
        extended_bottom = ((row_bottom + TABLE_BBOX_BOTTOM_EXTEND_MARGIN_PTS) as f32).min(max_bottom);
        row_start = row_end;
    }

    tracing::debug!(hint_img_bottom, extended_bottom, "extend_table_bottom_rows: result");
    extended_bottom
}

/// Table bbox bottom edge (PDF y0) from the lowest word consumed into the grid.
///
/// Falls back to the raw hint bottom when the recognizer consumed no words.
/// Using the consumed extent instead of the hint keeps
/// `filter_segments_by_table_bboxes` from suppressing text the recognizer did
/// not actually place in the table (the silent-text-loss failure mode).
#[cfg(feature = "layout-detection")]
fn table_bbox_bottom_from_consumed(consumed_bottom: Option<u32>, hint_bottom_pdf: f32, page_height: f32) -> f64 {
    /// Small downward margin so the last row's descenders stay inside the bbox.
    /// Mirrors `TABLE_BBOX_TOP_TIGHTEN_MARGIN_PTS` on the top edge.
    const TABLE_BBOX_BOTTOM_MARGIN_PTS: u32 = 4;

    match consumed_bottom {
        Some(bottom) => (page_height - (bottom + TABLE_BBOX_BOTTOM_MARGIN_PTS) as f32).max(0.0) as f64,
        None => hint_bottom_pdf as f64,
    }
}

/// Tighten the table bounding-box top edge to the first row with genuine column structure.
///
/// The layout model hint bbox often extends above the actual table grid to cover
/// an adjacent header/metadata block (e.g. "Precinct RUN 12/3/2014" on election
/// pages).  Using raw `hint.top` as `bbox.y1` causes
/// `filter_segments_by_table_bboxes` to suppress those header paragraphs, making
/// them invisible in the extraction output.
///
/// Strategy: walk word rows in image-y order (ascending = top-of-page first).
/// The first row whose words span at least `min_column_gaps` gaps ≥ `col_gap` is
/// the first genuine table content row.  Setting `min_column_gaps` to
/// `(num_table_cols / 2).max(1)` lets header rows with 1–2 text blocks pass
/// through while still accepting sparse table rows.
///
/// Returns the tightened PDF y coordinate (≤ `hint_top_pdf`).
#[cfg(feature = "layout-detection")]
fn tighten_table_bbox_top(
    table_words: &[&crate::pdf::table_reconstruct::HocrWord],
    unpadded_hint_img_top: f32,
    hint_top_pdf: f32,
    col_gap: u32,
    min_column_gaps: usize,
    page_height: f32,
) -> f64 {
    /// Small upward margin (image pts) added to the first-row top so that the
    /// row's own top edge is fully inside the bbox.  Must match the constant
    /// `TABLE_BBOX_TOP_TIGHTEN_MARGIN_PTS` in `tables.rs`.
    const TABLE_BBOX_TOP_TIGHTEN_MARGIN_PTS: u32 = 4;
    const SAME_ROW_TOLERANCE_PTS: u32 = 5;

    let mut sorted: Vec<&crate::pdf::table_reconstruct::HocrWord> = table_words.to_vec();
    sorted.sort_by_key(|w| w.top);

    let mut first_table_row_top: Option<u32> = None;
    let mut row_start = 0_usize;
    while row_start < sorted.len() {
        let row_anchor = sorted[row_start].top;
        let row_end = sorted[row_start..]
            .iter()
            .position(|w| w.top.saturating_sub(row_anchor) > SAME_ROW_TOLERANCE_PTS)
            .map(|p| row_start + p)
            .unwrap_or(sorted.len());

        let mut left_rights: Vec<(u32, u32)> = sorted[row_start..row_end]
            .iter()
            .map(|w| (w.left, w.left + w.width))
            .collect();
        left_rights.sort_by_key(|&(l, _)| l);
        let n_col_gaps = left_rights
            .windows(2)
            .filter(|pair| pair[1].0.saturating_sub(pair[0].1) >= col_gap)
            .count();
        if n_col_gaps >= min_column_gaps {
            first_table_row_top = Some(row_anchor);
            break;
        }
        row_start = row_end;
    }

    let img_top = first_table_row_top.unwrap_or(unpadded_hint_img_top as u32);
    let img_top_with_margin = img_top.saturating_sub(TABLE_BBOX_TOP_TIGHTEN_MARGIN_PTS);
    let pdf_top = page_height - img_top_with_margin as f32;
    (pdf_top as f64).min(hint_top_pdf as f64)
}

#[cfg(test)]
#[cfg(feature = "layout-detection")]
mod tests {
    use super::{
        RecognizedTatrTable, TatrGridOutput, collect_tatr_words, compute_col_gap_for_word_refs,
        extend_table_bottom_rows, recognize_hint_with_optional_split, side_by_side_table_plans,
        split_child_is_credible, table_bbox_bottom_from_consumed, tatr_grid_is_credible, tighten_table_bbox_top,
    };
    use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};
    use crate::pdf::table_reconstruct::HocrWord;
    use crate::types::{BoundingBox, Table};

    fn make_word(text: &str, left: u32, top: u32, width: u32, height: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    fn table_hint() -> LayoutHint {
        LayoutHint {
            class_name: LayoutHintClass::Table,
            confidence: 0.95,
            left: 0.0,
            bottom: 0.0,
            right: 200.0,
            top: 100.0,
        }
    }

    fn side_by_side_words() -> Vec<HocrWord> {
        let mut words = Vec::new();
        for row in 0..5 {
            let top = 10 + row * 15;
            words.push(make_word("Fund", 10, top, 20, 8));
            words.push(make_word("1.0", 40, top, 10, 8));
            words.push(make_word("10.0", 70, top, 10, 8));
            words.push(make_word("Fund", 110, top, 20, 8));
            words.push(make_word("1.0", 140, top, 10, 8));
            words.push(make_word("10.0", 170, top, 10, 8));
        }
        words
    }

    fn recognized_table(hint: &LayoutHint, allowed: Option<&std::collections::BTreeSet<usize>>) -> RecognizedTatrTable {
        let eligible_word_ids = allowed
            .cloned()
            .unwrap_or_else(|| (0..30).collect::<std::collections::BTreeSet<_>>());
        RecognizedTatrTable {
            table: Table {
                cells: vec![
                    vec!["name".to_string(), "value".to_string()],
                    vec!["Fund".to_string(), "10.0".to_string()],
                ],
                markdown: "| name | value |\n| --- | --- |\n| Fund | 10.0 |".to_string(),
                page_number: 1,
                bounding_box: Some(BoundingBox {
                    x0: hint.left as f64,
                    y0: hint.bottom as f64,
                    x1: hint.right as f64,
                    y1: hint.top as f64,
                }),
            },
            consumed_word_ids: eligible_word_ids.clone(),
            eligible_word_ids,
        }
    }

    #[test]
    fn genuine_three_by_three_tables_split_on_mirrored_tracks() {
        let plans = side_by_side_table_plans(&table_hint(), &side_by_side_words(), 100.0);
        let plan = plans.first().expect("mirrored tables should split");

        assert!(plan.children[0].right > 80.0 && plan.children[0].right < 110.0);
        assert_eq!(plan.children[0].right, plan.children[1].left);
        assert!(plan.children[0].left < plan.children[1].left);
    }

    #[test]
    fn blackrock_observed_tracks_reach_live_center_seam() {
        let hint = LayoutHint {
            class_name: LayoutHintClass::Table,
            confidence: 0.95,
            left: 18.0,
            bottom: 0.0,
            right: 573.0,
            top: 120.0,
        };
        let mut words = Vec::new();
        for row in 0..5 {
            let top = 10 + row * 18;
            words.push(make_word("Holding", 27, top, 55, 9));
            words.push(make_word("1.0", 150, top, 20, 9));
            words.push(make_word("10.0", 215, top, 20, 9));
            words.push(make_word("100.0", 260, top, 22, 9));
            words.push(make_word("Holding", 318, top, 55, 9));
            words.push(make_word("1.0", 441, top, 20, 9));
            words.push(make_word("10.0", 506, top, 20, 9));
            words.push(make_word("100.0", 551, top, 22, 9));
        }

        let plans = side_by_side_table_plans(&hint, &words, 120.0);
        let seam = plans.first().expect("BlackRock geometry should reach split").children[0].right;
        assert!((282.0..318.0).contains(&seam), "unexpected seam: {seam}");
    }

    #[test]
    fn split_tries_later_translation_and_seam_candidates() {
        let mut words = side_by_side_words();
        for row in 0..5 {
            words.push(make_word("Decoy", 90, 10 + row * 15, 5, 8));
        }

        assert!(side_by_side_table_plans(&table_hint(), &words, 100.0).len() >= 2);
    }

    #[test]
    fn seam_partitions_every_word_exactly_once() {
        let words = side_by_side_words();
        let plans = side_by_side_table_plans(&table_hint(), &words, 100.0);
        let ownership = &plans.first().expect("split plan").ownership;
        let union: std::collections::BTreeSet<_> = ownership[0].union(&ownership[1]).copied().collect();

        assert!(ownership[0].is_disjoint(&ownership[1]));
        assert_eq!(union, (0..words.len()).collect());
    }

    #[test]
    fn padding_edge_words_are_owned_once_and_preserved_for_children() {
        let hint = LayoutHint {
            class_name: LayoutHintClass::Table,
            confidence: 0.95,
            left: 18.0,
            bottom: 0.0,
            right: 573.0,
            top: 120.0,
        };
        let mut words = Vec::new();
        for row in 0..5 {
            let top = 10 + row * 18;
            words.push(make_word("Holding", 27, top, 55, 9));
            words.push(make_word("1.0", 150, top, 20, 9));
            words.push(make_word("10.0", 260, top, 22, 9));
            words.push(make_word("Holding", 318, top, 55, 9));
            words.push(make_word("1.0", 441, top, 20, 9));
            words.push(make_word("10.0", 551, top, 22, 9));
        }
        words.push(make_word("left-pad", 10, 10, 10, 9));
        words.push(make_word("right-pad", 573, 10, 10, 9));

        let plan = side_by_side_table_plans(&hint, &words, 120.0)
            .into_iter()
            .next()
            .expect("split plan");
        let union: std::collections::BTreeSet<_> = plan.ownership[0].union(&plan.ownership[1]).copied().collect();
        assert!(plan.ownership[0].is_disjoint(&plan.ownership[1]));
        assert_eq!(union, (0..words.len()).collect());

        for (child, owned) in plan.children.iter().zip(plan.ownership.iter()) {
            let child_words = collect_tatr_words(child, &words, Some(owned), 120.0, 120.0);
            let child_ids: std::collections::BTreeSet<_> =
                child_words.into_iter().map(|(word_id, _)| word_id).collect();
            assert_eq!(&child_ids, owned);
        }

        let mut lossy_child = recognized_table(&plan.children[0], Some(&plan.ownership[0]));
        lossy_child.consumed_word_ids = lossy_child
            .eligible_word_ids
            .iter()
            .take(lossy_child.eligible_word_ids.len() / 2 - 1)
            .copied()
            .collect();
        assert!(!split_child_is_credible(&lossy_child));
    }

    #[test]
    fn sparse_appended_row_preserves_original_model_density() {
        let mut grid = vec![vec![String::new(); 4]; 6];
        for cell in grid.iter_mut().flatten().take(5) {
            *cell = "value".to_string();
        }
        let output = TatrGridOutput {
            grid,
            markdown: "| value |".to_string(),
            consumed_bottom: Some(10),
            consumed_word_indices: vec![0, 1, 2, 3, 4],
            model_grid_cell_count: 20,
        };

        assert!(tatr_grid_is_credible(&output, 0));
    }

    #[test]
    fn split_recognition_is_atomic_and_falls_back_to_parent() {
        let hint = table_hint();
        let mut calls = Vec::new();
        let tables = recognize_hint_with_optional_split(&hint, &side_by_side_words(), 100.0, |candidate, allowed| {
            calls.push((candidate.left, candidate.right));
            if candidate.left > 0.0 {
                None
            } else {
                Some(recognized_table(candidate, allowed))
            }
        });

        assert_eq!(tables.len(), 1);
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2], (hint.left, hint.right));
        assert_eq!(tables[0].bounding_box.as_ref().map(|bbox| bbox.x1), Some(200.0));
    }

    #[test]
    fn split_recognition_emits_children_left_to_right() {
        let hint = table_hint();
        let tables = recognize_hint_with_optional_split(&hint, &side_by_side_words(), 100.0, |candidate, allowed| {
            Some(recognized_table(candidate, allowed))
        });

        assert_eq!(tables.len(), 2);
        let left = tables[0].bounding_box.as_ref().expect("left bbox");
        let right = tables[1].bounding_box.as_ref().expect("right bbox");
        assert!(left.x0 < right.x0);
        assert_eq!(left.x1, right.x0);
    }

    #[test]
    fn split_recognition_rejects_sparse_child_topology() {
        let hint = table_hint();
        let mut calls = 0;
        let tables = recognize_hint_with_optional_split(&hint, &side_by_side_words(), 100.0, |candidate, allowed| {
            calls += 1;
            if calls == 2 {
                let mut sparse = recognized_table(candidate, allowed);
                sparse.table = Table {
                    cells: vec![vec!["value".to_string()]],
                    markdown: "| value |".to_string(),
                    page_number: 1,
                    bounding_box: None,
                };
                Some(sparse)
            } else {
                Some(recognized_table(candidate, allowed))
            }
        });

        assert_eq!(calls, 3);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].bounding_box.as_ref().map(|bbox| bbox.x1), Some(200.0));
    }

    #[test]
    fn split_recognition_rejects_low_consumed_word_coverage() {
        let hint = table_hint();
        let mut calls = 0;
        let tables = recognize_hint_with_optional_split(&hint, &side_by_side_words(), 100.0, |candidate, allowed| {
            calls += 1;
            let mut result = recognized_table(candidate, allowed);
            if calls == 2 {
                result.consumed_word_ids = result.eligible_word_ids.iter().take(2).copied().collect();
            }
            Some(result)
        });

        assert_eq!(calls, 3);
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn later_split_plan_can_succeed_after_first_fails() {
        let hint = table_hint();
        let mut words = side_by_side_words();
        for row in 0..5 {
            words.push(make_word("Decoy", 90, 10 + row * 15, 5, 8));
        }
        let mut calls = 0;
        let tables = recognize_hint_with_optional_split(&hint, &words, 100.0, |candidate, allowed| {
            calls += 1;
            if calls == 2 {
                None
            } else {
                Some(recognized_table(candidate, allowed))
            }
        });

        assert!(calls >= 4);
        assert_eq!(tables.len(), 2);
    }

    #[test]
    fn seam_crossing_spanning_word_rejects_split() {
        let mut words = side_by_side_words();
        words.push(make_word("Spanning header", 75, 2, 45, 8));

        assert!(side_by_side_table_plans(&table_hint(), &words, 100.0).is_empty());
    }

    #[test]
    fn unsplit_parent_recognition_has_parity() {
        let hint = table_hint();
        let words = vec![make_word("Only prose", 10, 10, 50, 8)];
        let mut calls = 0;
        let tables = recognize_hint_with_optional_split(&hint, &words, 100.0, |candidate, allowed| {
            calls += 1;
            assert!(allowed.is_none());
            Some(recognized_table(candidate, allowed))
        });

        assert_eq!(calls, 1);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].bounding_box.as_ref().map(|bbox| bbox.x1), Some(200.0));
    }

    #[test]
    fn accenture_shaped_five_column_table_does_not_split() {
        let mut words = Vec::new();
        for row in 0..5 {
            let top = 10 + row * 15;
            words.push(make_word("Revenue", 10, top, 20, 8));
            for right in [50, 85, 120, 155, 190] {
                words.push(make_word("10.0", right - 20, top, 20, 8));
            }
        }

        assert!(side_by_side_table_plans(&table_hint(), &words, 100.0).is_empty());
    }

    /// Verifies that a two-text-block header row (1 column gap) is skipped when
    /// `min_column_gaps = 2` (4-column table), and the first genuine table row
    /// (3 column gaps) is found instead.
    ///
    /// Models the la-precinct-bulletin-2014-p1 regression: the ballot header had
    /// two text blocks at widely-separated x positions, giving it a 181 pt gap
    /// that exceeded the col_gap threshold — making it look like a table row
    /// under the old `min_column_gaps = 1` logic.
    #[test]
    fn test_tighten_skips_two_block_header_finds_four_column_table_row() {
        let page_height = 612.0_f32;

        let header_precinct = make_word("Precinct", 34, 16, 47, 10);
        let header_registrar = make_word("REGISTRAR", 262, 16, 90, 10);

        let col1 = make_word("GOVERNOR", 33, 86, 47, 10);
        let col2 = make_word("COLUMN2", 217, 86, 70, 10);
        let col3 = make_word("COLUMN3", 400, 86, 70, 10);
        let col4 = make_word("COLUMN4", 580, 86, 70, 10);

        let all_words: Vec<&HocrWord> = vec![&header_precinct, &header_registrar, &col1, &col2, &col3, &col4];

        let hint_img_top = (page_height - 596.0_f32).max(0.0);
        let result = tighten_table_bbox_top(&all_words, hint_img_top, 596.0, 30, 2, page_height);

        assert!(
            (result - 530.0).abs() < 1.0,
            "expected tightened_y1 ≈ 530.0, got {result}"
        );
    }

    /// When `min_column_gaps = 1` (2-column table), a two-block header is
    /// accepted as the first table row — tightening stops there, which is the
    /// correct behaviour for tables that look exactly like a 2-block row.
    #[test]
    fn test_tighten_two_column_table_accepts_first_gap_row() {
        let page_height = 612.0_f32;

        let block_a = make_word("LEFT", 10, 16, 60, 10);
        let block_b = make_word("RIGHT", 200, 16, 60, 10);

        let all_words: Vec<&HocrWord> = vec![&block_a, &block_b];

        let hint_img_top = (page_height - 596.0_f32).max(0.0);
        let result = tighten_table_bbox_top(&all_words, hint_img_top, 596.0, 30, 1, page_height);

        assert!(
            (result - 596.0).abs() < 1.0,
            "expected tightened_y1 ≈ 596.0 (no tightening past hint), got {result}"
        );
    }

    /// When no words meet the min-column-gaps threshold, the function falls back
    /// to `unpadded_hint_img_top` — bbox top stays at the original hint top.
    #[test]
    fn test_tighten_no_qualifying_row_falls_back_to_hint_top() {
        let page_height = 612.0_f32;

        let w1 = make_word("word1", 10, 20, 40, 10);
        let w2 = make_word("word2", 55, 20, 40, 10);

        let all_words: Vec<&HocrWord> = vec![&w1, &w2];
        let hint_img_top = (page_height - 592.0_f32).max(0.0);
        let result = tighten_table_bbox_top(&all_words, hint_img_top, 592.0, 30, 2, page_height);

        assert!(
            (result - 592.0).abs() < 1.0,
            "expected fallback to hint_top_pdf=592.0, got {result}"
        );
    }

    #[test]
    fn test_compute_col_gap_for_word_refs_returns_sensible_gap() {
        let page_height = 800.0_f32;
        let w1 = make_word("A", 10, 10, 40, 10);
        let w2 = make_word("B", 60, 10, 40, 10);
        let w3 = make_word("C", 300, 10, 40, 10);
        let w4 = make_word("D", 350, 10, 40, 10);
        let _ = page_height;

        let words: Vec<&HocrWord> = vec![&w1, &w2, &w3, &w4];
        let col_gap = compute_col_gap_for_word_refs(&words, 400.0);
        assert_eq!(
            col_gap, 60,
            "expected col_gap=60 (large-gap median/2 clamped), got {col_gap}"
        );
    }

    /// Build a nics-shaped page: a 4-column table whose hint bbox cuts off the
    /// last rows. Column x-positions leave ~100pt gaps between columns.
    fn four_column_row(y: u32) -> Vec<HocrWord> {
        [50_u32, 200, 350, 500]
            .iter()
            .map(|&x| make_word("12,345", x, y, 60, 12))
            .collect()
    }

    /// Rows below the hint that continue the table's column structure must
    /// extend the bottom edge; the walk stops at the first prose row so
    /// trailing paragraphs are never swallowed (mimics the NICS truncation:
    /// Texas..Wyoming rows below the hint bottom).
    #[test]
    fn test_extend_bottom_covers_continuation_rows_and_stops_at_prose() {
        let page_height = 800.0_f32;
        let hint_left = 40.0_f32;
        let hint_right = 580.0_f32;
        let hint_img_top = 100.0_f32;
        let hint_img_bottom = 400.0_f32;

        let mut words: Vec<HocrWord> = Vec::new();
        for y in (120..390).step_by(30) {
            words.extend(four_column_row(y));
        }
        words.extend(four_column_row(410));
        words.extend(four_column_row(440));
        for x in [80_u32, 120, 240, 300, 440, 470] {
            words.push(make_word("footnote", x, 480, 10, 12));
        }
        words.extend(four_column_row(520));

        let extended = extend_table_bottom_rows(
            &words,
            hint_left,
            hint_right,
            hint_img_top,
            hint_img_bottom,
            page_height,
        );

        assert_eq!(
            extended, 456.0,
            "expected extension to the last continuation row, got {extended}"
        );
    }

    /// A hint with no rows below it stays unextended.
    #[test]
    fn test_extend_bottom_no_rows_below_returns_hint_bottom() {
        let page_height = 800.0_f32;
        let mut words: Vec<HocrWord> = Vec::new();
        for y in (120..390).step_by(30) {
            words.extend(four_column_row(y));
        }

        let extended = extend_table_bottom_rows(&words, 40.0, 580.0, 100.0, 400.0, page_height);
        assert_eq!(
            extended, 400.0,
            "no continuation rows → bottom unchanged, got {extended}"
        );
    }

    /// The extension is capped at half the hinted table height even when
    /// structured rows continue beyond the cap.
    #[test]
    fn test_extend_bottom_capped_at_half_hint_height() {
        let page_height = 2000.0_f32;
        let hint_img_top = 100.0_f32;
        let hint_img_bottom = 300.0_f32;

        let mut words: Vec<HocrWord> = Vec::new();
        for y in (120..290).step_by(30) {
            words.extend(four_column_row(y));
        }
        for y in (310..700).step_by(30) {
            words.extend(four_column_row(y));
        }

        let extended = extend_table_bottom_rows(&words, 40.0, 580.0, hint_img_top, hint_img_bottom, page_height);
        assert!(
            extended <= 400.0,
            "extension must be capped at hint_bottom + half hint height (400), got {extended}"
        );
        assert!(
            extended > 300.0,
            "cap must not prevent extension entirely, got {extended}"
        );
    }

    /// The emitted bbox bottom must reflect consumed words, not the hint.
    #[test]
    fn test_table_bbox_bottom_from_consumed() {
        let page_height = 800.0_f32;
        let y0 = table_bbox_bottom_from_consumed(Some(452), 200.0, page_height);
        assert_eq!(y0, 344.0, "consumed bottom must drive y0, got {y0}");
        let fallback = table_bbox_bottom_from_consumed(None, 200.0, page_height);
        assert_eq!(fallback, 200.0, "no consumed words → hint bottom, got {fallback}");
    }
}
