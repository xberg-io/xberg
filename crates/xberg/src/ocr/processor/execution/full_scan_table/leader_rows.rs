use super::{
    MAX_FRAGMENTED_HOCR_INPUT_BYTES, SecurityLimits, Track, Word, alphabetic, bounded_words, disjoint_rows,
    literal_number, plain_ocr_fragment,
};
use crate::ocr::hocr_parser::hocr_matches_all_proved_table_fragments;
use crate::table_core::HocrWord;
use crate::types::{OcrTableBoundingBox, internal::InternalDocument};
use std::collections::HashSet;
use std::ops::Range;
use xberg_tesseract::Pix;

const MIN_LEADER_DOTS: usize = 5;
const MAX_LEADER_CROP_PIXELS: usize = 32_768;
const DOT_TO_TEXT_HEIGHT_DIVISOR: u32 = 3;

// Reuse the actual deskewed recognition frame, retaining only a bounded 1-bpp Pix. ~keep
pub(in crate::ocr::processor::execution) fn can_hold_leader_raster(
    raster: &Pix,
    (width, height): (u32, u32),
    encoded_bytes: usize,
    limits: &SecurityLimits,
) -> bool {
    if raster.depth() != 1
        || u32::try_from(raster.width()).ok() != Some(width)
        || u32::try_from(raster.height()).ok() != Some(height)
    {
        return false;
    }
    let bitmap = u64::from(width.div_ceil(32)) * 4 * u64::from(height);
    let extra = bitmap.saturating_add(MAX_LEADER_CROP_PIXELS as u64 * 2);
    super::super::native_image_budget::known_phase_peak(width, height, encoded_bytes, None)
        .ok()
        .flatten()
        .is_some_and(|peak| peak.saturating_add(extra) <= limits.max_content_size as u64)
}

pub(in crate::ocr::processor::execution) fn recover_dot_leader_rows(
    tsv: &str,
    content: &str,
    region: &OcrTableBoundingBox,
    limits: &SecurityLimits,
    (hocr, document): (&str, &InternalDocument),
    raster: &Pix,
) -> Option<Vec<Vec<String>>> {
    let page = document.elements.first()?.page?;
    if !document
        .elements
        .iter()
        .all(|element| element.page == Some(page) && plain_ocr_fragment(element))
        || tsv.len().checked_add(content.len())?.checked_add(hocr.len())?
            > MAX_FRAGMENTED_HOCR_INPUT_BYTES.min(limits.max_content_size)
    {
        return None;
    }
    let (words, budget) = bounded_words(tsv, content, region, limits)?;
    let mut budget = budget.checked_sub(hocr.len().checked_mul(4)?)?;
    let mut lines = line_envelopes(&words)?;
    lines.sort_unstable_by_key(|(_, line)| line.top);
    let rows = disjoint_rows(&mut lines)?;
    let grid = build_leader_grid(&rows, limits.max_table_cells, |label, leader| {
        raster_proves_dots(raster, leader, label.height, &mut budget)
    })?;
    if !super::super::has_table_grid(&grid)
        || !hocr_matches_all_proved_table_fragments(hocr, page, words.iter().map(|(_, word)| word).collect())
    {
        return None;
    }
    Some(grid)
}

fn line_envelopes(words: &[Word]) -> Option<Vec<Word>> {
    let mut keys = HashSet::new();
    let source_page = words.first()?.0?[0];
    words
        .chunk_by(|left, right| left.0 == right.0)
        .map(|group| {
            let key = group.first()?.0?;
            if key[0] != source_page || !keys.insert(key) {
                return None;
            }
            Some((Some(key), line_envelope(group)?))
        })
        .collect()
}

fn line_envelope(group: &[Word]) -> Option<HocrWord> {
    let first = &group.first()?.1;
    let mut top = first.top;
    let mut right = first.left + first.width;
    let mut bottom = first.top + first.height;
    let mut shared_top = top;
    let mut shared_bottom = bottom;
    let mut prior_left = first.left;
    for (_, word) in &group[1..] {
        if word.left <= prior_left {
            return None;
        }
        prior_left = word.left;
        top = top.min(word.top);
        right = right.max(word.left + word.width);
        bottom = bottom.max(word.top + word.height);
        shared_top = shared_top.max(word.top);
        shared_bottom = shared_bottom.min(word.top + word.height);
    }
    (shared_top < shared_bottom).then(|| HocrWord {
        text: group
            .iter()
            .map(|(_, word)| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        left: first.left,
        top,
        width: right - first.left,
        height: bottom - top,
        confidence: first.confidence,
    })
}

fn horizontal(word: &HocrWord) -> Range<u32> {
    word.left..word.left + word.width
}

fn build_leader_grid(
    rows: &[&mut [Word]],
    max_cells: usize,
    mut proves_dots: impl FnMut(&HocrWord, &HocrWord) -> Option<()>,
) -> Option<Vec<Vec<String>>> {
    let (header, body) = rows.split_first()?;
    if header.len() < 4
        || body.len() < 3
        || rows.len().checked_mul(header.len())? > max_cells
        || !header.iter().all(|(_, word)| alphabetic(&word.text))
    {
        return None;
    }
    let mut tracks: Vec<_> = header.iter().map(|(_, word)| Track::new(horizontal(word))).collect();
    let mut grid = vec![header.iter().map(|(_, word)| word.text.clone()).collect()];
    let mut leaders = None;
    for row in body {
        let [(_, label), (_, leader), values @ ..] = &**row else {
            return None;
        };
        if values.len() != tracks.len() - 1
            || !values.iter().all(|(_, word)| literal_number(&word.text))
            || !label.text.split_whitespace().all(alphabetic)
        {
            return None;
        }
        tracks[0].admit(horizontal(label))?;
        leaders
            .get_or_insert_with(|| Track::new(horizontal(leader)))
            .admit(horizontal(leader))?;
        proves_dots(label, leader)?;
        let mut cells = vec![format!("{} {}", label.text, leader.text)];
        for ((_, value), track) in values.iter().zip(&mut tracks[1..]) {
            track.admit(horizontal(value))?;
            cells.push(value.text.clone());
        }
        grid.push(cells);
    }
    let leaders = leaders?;
    (tracks[0].shared.end <= leaders.shared.start
        && leaders.extent.end <= tracks[1].extent.start
        && tracks.windows(2).all(|pair| pair[0].extent.end <= pair[1].extent.start))
    .then_some(grid)
}

fn raster_proves_dots(raster: &Pix, word: &HocrWord, label_height: u32, budget: &mut usize) -> Option<()> {
    let width = usize::try_from(word.width).ok()?;
    let height = usize::try_from(word.height).ok()?;
    let area = width.checked_mul(height)?;
    if area == 0 || area > MAX_LEADER_CROP_PIXELS || raster.depth() != 1 {
        return None;
    }
    *budget = budget.checked_sub(area.checked_mul(2)?)?;
    let left = i32::try_from(word.left).ok()?;
    let top = i32::try_from(word.top).ok()?;
    let w = i32::try_from(width).ok()?;
    let h = i32::try_from(height).ok()?;
    if left.checked_add(w)? > raster.width() || top.checked_add(h)? > raster.height() {
        return None;
    }
    let pixels = raster.clip_rectangle(left, top, w, h).ok()?.grayscale_bytes().ok()?;
    (pixels.len() == area && regular_dot_runs(&pixels, width, label_height as usize)).then_some(())
}

struct InkRun {
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    count: usize,
}

fn column_ink_runs(pixels: &[u8], width: usize) -> Vec<InkRun> {
    let mut runs: Vec<InkRun> = Vec::new();
    for x in 0..width {
        let mut ink = pixels.chunks_exact(width).enumerate().filter(|(_, row)| row[x] != 0);
        let Some((top, _)) = ink.next() else { continue };
        let (bottom, count) = ink.fold((top + 1, 1), |(_, count), (y, _)| (y + 1, count + 1));
        if let Some(run) = runs.last_mut().filter(|run| run.right == x) {
            run.right += 1;
            run.top = run.top.min(top);
            run.bottom = run.bottom.max(bottom);
            run.count += count;
        } else {
            runs.push(InkRun {
                left: x,
                right: x + 1,
                top,
                bottom,
                count,
            });
        }
    }
    runs
}

fn regular_dot_runs(pixels: &[u8], width: usize, text_height: usize) -> bool {
    let runs = column_ink_runs(pixels, width);
    if runs.len() < MIN_LEADER_DOTS {
        return false;
    }
    let max_dot = (text_height / DOT_TO_TEXT_HEIGHT_DIVISOR as usize).max(1);
    let compact = runs.iter().all(|run| {
        let width = run.right - run.left;
        let height = run.bottom - run.top;
        width <= max_dot && height <= max_dot && run.count * 4 >= width * height * 3
    });
    let shared_top = runs.iter().map(|run| run.top).max().unwrap_or_default();
    let shared_bottom = runs.iter().map(|run| run.bottom).min().unwrap_or_default();
    let advance = runs[1].left - runs[0].left;
    compact
        && shared_top < shared_bottom
        && runs.windows(2).all(|pair| {
            let gap = pair[1].left - pair[0].right;
            let step = pair[1].left - pair[0].left;
            gap >= (pair[0].right - pair[0].left).max(pair[1].right - pair[1].left)
                && step.abs_diff(advance) <= (advance / 5).max(1)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_proof_requires_compact_regular_ink_runs() {
        let mut dots = vec![0; 80 * 12];
        for x in (3..80).step_by(12) {
            for y in 5..8 {
                for x in x..x + 3 {
                    dots[y * 80 + x] = 1;
                }
            }
        }
        assert!(regular_dot_runs(&dots, 80, 27));
        for x in (3..80).step_by(12) {
            for y in 0..5 {
                dots[y * 80 + x] = 1;
            }
        }
        assert!(!regular_dot_runs(&dots, 80, 27));
    }

    #[test]
    fn raster_proof_rejects_true_small_text() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png"
        ));
        let image = image::load_from_memory(bytes).unwrap().to_luma8();
        let pixels: Vec<_> = (30..46)
            .flat_map(|y| (10..106).map(move |x| (x, y)))
            .map(|(x, y)| u8::from(image.get_pixel(x, y).0[0] < 130))
            .collect();
        assert!(column_ink_runs(&pixels, 96).len() >= MIN_LEADER_DOTS);
        assert!(!regular_dot_runs(&pixels, 96, 27));
    }
}
