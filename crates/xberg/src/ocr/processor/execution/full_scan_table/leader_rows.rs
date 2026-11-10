use super::{
    MAX_FRAGMENTED_HOCR_INPUT_BYTES, SecurityLimits, Track, Word, bounded_words, disjoint_rows, plain_ocr_fragment,
};
use crate::ocr::hocr_parser::hocr_matches_all_proved_table_fragments;
use crate::table_core::HocrWord;
use crate::types::{ImagePreprocessingConfig, OcrTableBoundingBox, internal::InternalDocument};
use std::collections::HashSet;
use std::ops::Range;
use xberg_tesseract::Pix;

const MIN_LEADER_DOTS: usize = 5;
const MAX_LEADER_CROP_PIXELS: usize = 32_768;
const DOT_TO_TEXT_HEIGHT_DIVISOR: usize = 3;
const MAX_DOT_ASPECT_RATIO: (usize, usize) = (3, 2);

// Reuse the actual deskewed recognition frame. Preprocessing and recognition are disjoint
// phases: `image_data` is dropped before `recognize`, so the retained 1-bpp Pix sits inside what
// the preprocessing peak already reserved and only the proof crops are added to that peak. An
// unmodelled policy has no known peak and declines. ~keep
pub(in crate::ocr::processor::execution) fn can_hold_leader_raster(
    raster: &Pix,
    (width, height): (u32, u32),
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
    limits: &SecurityLimits,
) -> bool {
    raster.depth() == 1
        && u32::try_from(raster.width()).ok() == Some(width)
        && u32::try_from(raster.height()).ok() == Some(height)
        && leader_budget_fits(width, height, encoded_bytes, preprocessing, limits.max_content_size)
}

/// Declines when the policy has no modelled peak, which includes `auto_rotate`.
fn leader_budget_fits(
    width: u32,
    height: u32,
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
    max_content_size: usize,
) -> bool {
    preprocessing.is_some_and(|options| {
        super::super::native_image_budget::known_phase_peak(width, height, encoded_bytes, Some(options))
            .ok()
            .flatten()
            .is_some_and(|peak| peak.saturating_add(MAX_LEADER_CROP_PIXELS as u64 * 2) <= max_content_size as u64)
    })
}

// Runs only after the stock recovery declines a known full-page sparse-text scan. A wider gate
// would let a text column pass for a leader, so every leader must also be proved as dots on the
// raster and every word must match an hOCR occurrence exactly. ~keep
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
    let grid = build_leader_grid(&rows, limits.max_table_cells, |leader, text_height| {
        raster_proves_dots(raster, leader, text_height, &mut budget)
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
    let mut right = first.left.saturating_add(first.width);
    let mut bottom = first.top.saturating_add(first.height);
    let mut shared_top = top;
    let mut shared_bottom = bottom;
    let mut prior_left = first.left;
    for (_, word) in &group[1..] {
        if word.left <= prior_left {
            return None;
        }
        prior_left = word.left;
        top = top.min(word.top);
        right = right.max(word.left.saturating_add(word.width));
        bottom = bottom.max(word.top.saturating_add(word.height));
        shared_top = shared_top.max(word.top);
        shared_bottom = shared_bottom.min(word.top.saturating_add(word.height));
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
    word.left..word.left.saturating_add(word.width)
}

fn build_leader_grid(
    rows: &[&mut [Word]],
    max_cells: usize,
    mut proves_dots: impl FnMut(&HocrWord, u32) -> Option<()>,
) -> Option<Vec<Vec<String>>> {
    let (header, body) = rows.split_first()?;
    if header.len() < 4 || body.len() < 3 || rows.len().checked_mul(header.len())? > max_cells {
        return None;
    }
    let mut tracks: Vec<_> = header.iter().map(|(_, word)| Track::new(horizontal(word))).collect();
    let mut grid = vec![header.iter().map(|(_, word)| word.text.clone()).collect()];
    let mut leaders = None;
    // The median keeps a label without ascenders short of the reference without letting one stray tall box set it. ~keep
    let mut heights: Vec<u32> = body
        .iter()
        .filter_map(|row| row.first())
        .map(|(_, label)| label.height)
        .collect();
    heights.sort_unstable();
    let text_height = *heights.get(heights.len() / 2)?;
    for row in body {
        let [(_, label), (_, leader), values @ ..] = &**row else {
            return None;
        };
        if values.len() != tracks.len() - 1 {
            return None;
        }
        tracks[0].admit(horizontal(label))?;
        leaders
            .get_or_insert_with(|| Track::new(horizontal(leader)))
            .admit(horizontal(leader))?;
        proves_dots(leader, text_height)?;
        let mut cells = vec![label.text.clone()];
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

impl InkRun {
    fn width(&self) -> usize {
        self.right - self.left
    }

    fn is_compact(&self, max_dot: usize) -> bool {
        let width = self.width();
        let height = self.bottom - self.top;
        let longer = width.max(height);
        longer <= max_dot
            && longer * MAX_DOT_ASPECT_RATIO.1 <= width.min(height) * MAX_DOT_ASPECT_RATIO.0
            && self.count * 4 >= width * height * 3
    }
}

fn column_ink_runs(pixels: &[u8], width: usize) -> impl Iterator<Item = InkRun> {
    let mut columns = (0..width)
        .filter_map(move |x| {
            let mut ink = pixels.chunks_exact(width).enumerate().filter(|(_, row)| row[x] != 0);
            let (top, _) = ink.next()?;
            let (bottom, count) = ink.fold((top + 1, 1), |(_, count), (y, _)| (y + 1, count + 1));
            Some((x, top, bottom, count))
        })
        .peekable();
    std::iter::from_fn(move || {
        let (left, top, bottom, count) = columns.next()?;
        let mut run = InkRun {
            left,
            right: left + 1,
            top,
            bottom,
            count,
        };
        while let Some((_, top, bottom, count)) = columns.next_if(|column| column.0 == run.right) {
            run.right += 1;
            run.top = run.top.min(top);
            run.bottom = run.bottom.max(bottom);
            run.count += count;
        }
        Some(run)
    })
}

fn regular_dot_runs(pixels: &[u8], width: usize, text_height: usize) -> bool {
    let mut runs = column_ink_runs(pixels, width);
    let Some(mut previous) = runs.next() else { return false };
    let max_dot = (text_height / DOT_TO_TEXT_HEIGHT_DIVISOR).max(1);
    if !previous.is_compact(max_dot) {
        return false;
    }
    let mut shared_top = previous.top;
    let mut shared_bottom = previous.bottom;
    let mut count = 1;
    let mut advance = None;
    for run in runs {
        let gap = run.left - previous.right;
        let step = run.left - previous.left;
        let first_step = *advance.get_or_insert(step);
        shared_top = shared_top.max(run.top);
        shared_bottom = shared_bottom.min(run.bottom);
        if !run.is_compact(max_dot)
            || shared_top >= shared_bottom
            || gap < previous.width().max(run.width())
            || step.abs_diff(first_step) > (first_step / 5).max(1)
        {
            return false;
        }
        previous = run;
        count += 1;
    }
    count >= MIN_LEADER_DOTS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::processor::execution::tests::word_table_markup;

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

    fn word(text: &str, left: u32, width: u32, height: u32) -> Word {
        let word = HocrWord {
            text: text.to_owned(),
            left,
            top: 0,
            width,
            height,
            confidence: 90.0,
        };
        (None, word)
    }

    fn proof_heights(label_heights: &[u32]) -> Vec<u32> {
        let mut rows: Vec<Vec<Word>> = vec![vec![
            word("Item", 0, 40, 20),
            word("Count", 100, 40, 20),
            word("Budget", 200, 40, 20),
            word("Change", 300, 40, 20),
        ]];
        rows.extend(label_heights.iter().map(|height| {
            vec![
                word("Tree", 0, 40, *height),
                word("...", 60, 30, 5),
                word("10", 100, 30, 20),
                word("20", 200, 30, 20),
                word("30", 300, 30, 20),
            ]
        }));
        let rows: Vec<&mut [Word]> = rows.iter_mut().map(Vec::as_mut_slice).collect();
        let mut heights = Vec::new();
        let grid = build_leader_grid(&rows, 100, |_, height| {
            heights.push(height);
            Some(())
        });
        assert!(grid.is_some());
        heights
    }

    fn leader_grid(header: [&str; 4], label: &str, values: [&str; 3], proved: bool) -> Option<Vec<Vec<String>>> {
        let mut rows: Vec<Vec<Word>> = vec![
            header
                .iter()
                .enumerate()
                .map(|(index, text)| word(text, index as u32 * 100, 40, 20))
                .collect(),
        ];
        rows.extend((0..3).map(|_| {
            let mut row = vec![word(label, 0, 40, 20), word("...", 60, 30, 5)];
            row.extend(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, text)| word(text, (index as u32 + 1) * 100, 30, 20)),
            );
            row
        }));
        let rows: Vec<&mut [Word]> = rows.iter_mut().map(Vec::as_mut_slice).collect();
        build_leader_grid(&rows, 100, |_, _| proved.then_some(()))
    }

    #[test]
    fn leader_rows_follow_proved_geometry_not_label_letters_or_value_digits() {
        for (header, label, values) in [
            (["Item", "Count", "Budget", "Change"], "Tree", ["10", "20", "30"]),
            (["No.", "Qty", "$", "%"], "4(b) net", ["$1,010", "3.2%", "-7"]),
            (["Item", "Count", "Budget", "Change"], "Tree", ["n/a", "yes", "none"]),
        ] {
            let grid = leader_grid(header, label, values, true).unwrap_or_else(|| panic!("{header:?} {label:?}"));
            assert_eq!(grid.len(), 4);
            assert_eq!(grid[1], [label, values[0], values[1], values[2]]);
            assert!(leader_grid(header, label, values, false).is_none(), "unproved leader");
        }
    }

    #[test]
    fn leader_rows_refuse_values_outside_their_header_columns() {
        let mut rows: Vec<Vec<Word>> = vec![vec![
            word("A", 0, 40, 20),
            word("B", 100, 40, 20),
            word("C", 200, 40, 20),
            word("D", 300, 40, 20),
        ]];
        rows.extend((0..3).map(|_| {
            vec![
                word("x", 0, 40, 20),
                word("...", 60, 30, 5),
                word("1", 100, 30, 20),
                word("2", 200, 30, 20),
                word("3", 340, 30, 20),
            ]
        }));
        let rows: Vec<&mut [Word]> = rows.iter_mut().map(Vec::as_mut_slice).collect();
        assert!(build_leader_grid(&rows, 100, |_, _| Some(())).is_none());
    }

    /// One scanned dot-leader table: a symbol header, and three rows of a two-word label, a run of
    /// dots and three symbol values. Each cell is its own OCR line, as a sparse-text read gives.
    struct LeaderScan {
        tsv: String,
        hocr: String,
        content: String,
        raster: Pix,
    }

    fn leader_scan(with_dots: bool) -> LeaderScan {
        const HEADER: [(&str, u32); 4] = [("No.", 0), ("Qty", 200), ("$", 300), ("%", 400)];
        const VALUES: [&str; 3] = ["$1,010", "3.2%", "-7"];
        // (text, left, top, width, height, tesseract line)
        let mut words: Vec<(&str, u32, u32, u32, u32, usize)> = Vec::new();
        for (index, (text, left)) in HEADER.into_iter().enumerate() {
            words.push((text, left, 10, 40, 20, index));
        }
        // Dark ink on white paper: the one-bit raster reads ink as 1. The speck keeps both classes present when a
        // scan has no dots.
        let mut dots = vec![255u8; 500 * 170];
        dots[500 * 170 - 1] = 0;
        for (row, top) in [50u32, 90, 130].into_iter().enumerate() {
            let line = 10 * (row + 1);
            words.push(("Item", 0, top, 30, 20, line));
            words.push(("4(b)", 35, top, 25, 20, line));
            words.push(("........", 70, top + 14, 100, 4, line + 1));
            for (column, value) in VALUES.into_iter().enumerate() {
                words.push((value, 200 + 100 * column as u32, top, 30, 20, line + 2 + column));
            }
            let columns = (73..160).step_by(12).flat_map(|x| x..x + 3);
            let ink = columns.flat_map(|x| (top + 14..top + 17).map(move |y| y as usize * 500 + x));
            for index in ink {
                dots[index] = if with_dots { 0 } else { 255 };
            }
        }
        let (hocr, tsv) = word_table_markup(words.iter().copied());
        let content = words.iter().map(|word| word.0).collect::<Vec<_>>().join(" ");
        let gray = Pix::from_grayscale_bytes(&dots, 500, 170).expect("grayscale canvas");
        let raster = gray.otsu_threshold().expect("one-bit raster");
        assert_eq!(raster.depth(), 1);
        LeaderScan {
            tsv,
            hocr,
            content,
            raster,
        }
    }

    fn recover_leader_scan(scan: &LeaderScan) -> Option<Vec<Vec<String>>> {
        let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&scan.hocr);
        let region = OcrTableBoundingBox {
            left: 0,
            top: 0,
            right: 500,
            bottom: 170,
        };
        recover_dot_leader_rows(
            &scan.tsv,
            &scan.content,
            &region,
            &SecurityLimits::default(),
            (&scan.hocr, &document),
            &scan.raster,
        )
    }

    #[test]
    fn the_entry_reads_a_symbol_table_only_where_the_raster_shows_dots() {
        let grid = recover_leader_scan(&leader_scan(true)).expect("dots on the raster prove every leader");
        assert_eq!(grid.len(), 4);
        assert_eq!(grid[0], ["No.", "Qty", "$", "%"]);
        for row in &grid[1..] {
            assert_eq!(row, &["Item 4(b)", "$1,010", "3.2%", "-7"]);
        }
        assert!(
            recover_leader_scan(&leader_scan(false)).is_none(),
            "a leader word with no dots under it is not a leader"
        );
    }

    #[test]
    fn the_entry_reads_a_leader_scan_only_where_every_hocr_word_matches_the_table() {
        let mut scan = leader_scan(true);
        assert!(
            recover_leader_scan(&scan).is_some(),
            "control: the hOCR words match the table"
        );
        let misread = scan.hocr.replacen(">Qty<", ">Qxy<", 1);
        assert_ne!(misread, scan.hocr);
        scan.hocr = misread;
        assert!(
            recover_leader_scan(&scan).is_none(),
            "a table word that the hOCR reads differently is not claimed"
        );
    }

    #[test]
    fn leader_proof_uses_the_median_label_height_in_the_table() {
        assert_eq!(proof_heights(&[27, 27, 14, 27]), [27, 27, 27, 27]);
    }

    #[test]
    fn one_stray_tall_label_does_not_inflate_the_dot_limit() {
        let heights = proof_heights(&[27, 27, 60, 27, 27]);
        assert_eq!(heights, [27; 5]);
        let mut blob = vec![0; 160 * 20];
        for x in (3..140).step_by(32) {
            for y in 2..17 {
                for x in x..x + 15 {
                    blob[y * 160 + x] = 1;
                }
            }
        }
        assert!(!regular_dot_runs(&blob, 160, heights[0] as usize));
        assert!(regular_dot_runs(&blob, 160, 60));
    }

    #[test]
    fn raster_proof_rejects_four_by_two_hyphen_marks() {
        let mut hyphens = vec![0; 80 * 12];
        for x in (3..70).step_by(8) {
            for y in 5..7 {
                hyphens[y * 80 + x..y * 80 + x + 4].fill(1);
            }
        }
        assert!(!regular_dot_runs(&hyphens, 80, 27));
    }

    #[test]
    fn leader_budget_charges_the_applied_binarization_not_the_raw_path() {
        let options = ImagePreprocessingConfig::default();
        let (width, height) = (2550, 3300);
        let applied =
            super::super::super::native_image_budget::known_phase_peak(width, height, 227_360, Some(&options))
                .unwrap()
                .unwrap();
        let raw = super::super::super::native_image_budget::known_phase_peak(width, height, 227_360, None)
            .unwrap()
            .unwrap();
        assert!(applied < raw);
        let crops = MAX_LEADER_CROP_PIXELS as u64 * 2;
        let exact = (applied + crops) as usize;
        assert!(leader_budget_fits(width, height, 227_360, Some(&options), exact));
        assert!(!leader_budget_fits(width, height, 227_360, Some(&options), exact - 1));
    }

    #[test]
    fn retained_bitmap_and_crops_fit_inside_the_applied_peak_beyond_the_rgb_copy() {
        let options = ImagePreprocessingConfig::default();
        let (width, height) = (2550u64, 3300u64);
        let peak = super::super::super::native_image_budget::known_phase_peak(2550, 3300, 227_360, Some(&options))
            .unwrap()
            .unwrap();
        let bitmap = width * height / 8;
        assert!(2 * bitmap + MAX_LEADER_CROP_PIXELS as u64 * 2 <= peak - width * height * 3);
        assert!(peak - width * height * 3 >= 5 * width * height);
    }

    #[test]
    fn leader_budget_declines_an_unmodelled_preprocessing_policy() {
        let unmodelled = ImagePreprocessingConfig {
            denoise: true,
            ..Default::default()
        };
        assert!(!leader_budget_fits(100, 100, 1000, Some(&unmodelled), usize::MAX));
        assert!(!leader_budget_fits(100, 100, 1000, None, usize::MAX));
        assert!(leader_budget_fits(
            100,
            100,
            1000,
            Some(&ImagePreprocessingConfig::default()),
            usize::MAX
        ));
    }

    #[test]
    fn raster_proof_accepts_dots_beside_a_label_without_ascenders() {
        let mut dots = vec![0; 80 * 12];
        for x in (3..70).step_by(12) {
            for y in 3..9 {
                for x in x..x + 6 {
                    dots[y * 80 + x] = 1;
                }
            }
        }
        assert!(!regular_dot_runs(&dots, 80, 14));
        assert!(regular_dot_runs(&dots, 80, 27));
    }

    #[test]
    fn raster_proof_rejects_regular_horizontal_dashes() {
        let mut dashes = vec![0; 80 * 12];
        for x in (3..73).step_by(12) {
            for y in 5..7 {
                dashes[y * 80 + x..y * 80 + x + 6].fill(1);
            }
        }
        assert_eq!(column_ink_runs(&dashes, 80).count(), 6);
        assert!(!regular_dot_runs(&dashes, 80, 27));
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
        assert!(column_ink_runs(&pixels, 96).count() >= MIN_LEADER_DOTS);
        assert!(!regular_dot_runs(&pixels, 96, 27));
    }
}
