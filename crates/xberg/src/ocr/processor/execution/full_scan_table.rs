//! Recover a rejected full-scan table with literal TSV line and column evidence.

use super::{
    MAX_FRAGMENTED_HOCR_INPUT_BYTES, MAX_FRAGMENTED_HOCR_WORDS, SecurityLimits, TesseractConfig, post_process_table,
};
use crate::ocr::table::tsv_parser::{TextLine, words_on_lines};
use crate::table_core::HocrWord;
use crate::types::OcrTableBoundingBox;
use std::collections::HashMap;
use std::ops::Range;

type Word = (Option<TextLine>, HocrWord);

pub(super) fn uses_default_table_thresholds(config: &TesseractConfig) -> bool {
    let defaults = TesseractConfig::default();
    config.table_min_confidence == defaults.table_min_confidence
        && config.table_column_threshold == defaults.table_column_threshold
        && config.table_row_threshold_ratio == defaults.table_row_threshold_ratio
}

pub(super) fn recover(
    tsv: &str,
    content: &str,
    region: &OcrTableBoundingBox,
    limits: &SecurityLimits,
) -> Option<Vec<Vec<String>>> {
    certify(tsv, content, region, limits).map(|(cells, _)| cells)
}

fn certify(
    tsv: &str,
    content: &str,
    region: &OcrTableBoundingBox,
    limits: &SecurityLimits,
) -> Option<(Vec<Vec<String>>, Vec<Word>)> {
    let bytes = tsv.len().checked_add(content.len())?;
    if bytes > MAX_FRAGMENTED_HOCR_INPUT_BYTES.min(limits.max_content_size) {
        return None;
    }
    let budget = limits.max_iterations.checked_sub(bytes.saturating_mul(4))?;
    let mut words = words_on_lines(tsv, 0.0);
    if words.is_empty()
        || words.len() > MAX_FRAGMENTED_HOCR_WORDS
        || words.len().saturating_mul(words.len()) > budget
        || !words.iter().all(|(_, word)| word.confidence.is_finite())
        || !same_word_inventory(content, &words)
        || !words.iter().all(|(_, word)| inside(word, region))
    {
        return None;
    }
    words.sort_unstable_by_key(|(_, word)| word.top);
    let rows = disjoint_rows(&mut words)?;
    let grid = build_grid(&rows, limits.max_table_cells)?;
    let cleaned = post_process_table(grid.clone(), false, false)?;
    (cleaned == grid).then_some((cleaned, words))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn recover_standalone(
    tsv: &str,
    content: &str,
    bbox: &OcrTableBoundingBox,
    limits: &SecurityLimits,
    hocr: &str,
    document: &super::InternalDocument,
) -> Option<Vec<Vec<String>>> {
    let page_number = document.elements.first()?.page?;
    let proof_bytes = tsv.len().checked_add(content.len())?.checked_add(hocr.len())?;
    if proof_bytes > MAX_FRAGMENTED_HOCR_INPUT_BYTES.min(limits.max_content_size)
        || !document
            .elements
            .iter()
            .all(|element| element.page == Some(page_number) && plain_ocr_fragment(element))
    {
        return None;
    }
    let mut limits = limits.clone();
    limits.max_iterations = limits.max_iterations.checked_sub(hocr.len().checked_mul(4)?)?;
    let (cells, words) = certify(tsv, content, bbox, &limits)?;
    if !super::has_table_grid(&cells) {
        return None;
    }
    // Reparse with the exact geometry and text of the already-certified TSV occurrences.
    // A remaining hOCR fragment leaves the original OCR result untouched.
    let claimed = super::parse_hocr_without_proved_table_fragments(
        hocr,
        page_number,
        words.iter().map(|(_, word)| word).collect(),
    );
    claimed.elements.is_empty().then_some(cells)
}

#[cfg(not(target_arch = "wasm32"))]
fn plain_ocr_fragment(element: &crate::types::internal::InternalElement) -> bool {
    use crate::ocr::hocr_parser::{
        HOCR_BOLD_FRACTION_ATTRIBUTE, HOCR_ITALIC_FRACTION_ATTRIBUTE, HOCR_TEXT_ANGLE_ATTRIBUTE,
    };
    use crate::types::internal::ElementKind;

    matches!(element.kind, ElementKind::OcrText { .. })
        && element.depth == 0
        && element.layer == crate::types::ContentLayer::Body
        && element.annotations.is_empty()
        && element.anchor.is_none()
        && element.attributes.as_ref().is_none_or(|attributes| {
            [
                HOCR_BOLD_FRACTION_ATTRIBUTE,
                HOCR_ITALIC_FRACTION_ATTRIBUTE,
                HOCR_TEXT_ANGLE_ATTRIBUTE,
            ]
            .iter()
            .all(|key| {
                attributes
                    .get(*key)
                    .is_none_or(|value| value.parse::<f64>().ok() == Some(0.0))
            })
        })
}

fn inside(word: &HocrWord, region: &OcrTableBoundingBox) -> bool {
    word.width > 0
        && word.height > 0
        && word.left >= region.left
        && word.top >= region.top
        && word
            .left
            .checked_add(word.width)
            .is_some_and(|right| right <= region.right)
        && word
            .top
            .checked_add(word.height)
            .is_some_and(|bottom| bottom <= region.bottom)
}

fn same_word_inventory(content: &str, words: &[Word]) -> bool {
    let mut counts = HashMap::new();
    for token in content.split_whitespace() {
        *counts.entry(token).or_insert(0usize) += 1;
    }
    for (_, word) in words {
        let Some(count) = counts.get_mut(word.text.as_str()).filter(|count| **count > 0) else {
            return false;
        };
        *count -= 1;
    }
    counts.values().all(|count| *count == 0)
}

/// Each band has a common vertical intersection and its entire extent is disjoint from
/// the next band. A tall word cannot bridge two bands and authorize an invented row. ~keep
fn disjoint_rows(words: &mut [Word]) -> Option<Vec<&mut [Word]>> {
    let mut rows = Vec::new();
    let mut remaining = words;
    while let Some((_, first)) = remaining.first() {
        let mut common_bottom = first.top + first.height;
        let mut band_bottom = common_bottom;
        let mut end = 1;
        for (_, word) in &remaining[1..] {
            if word.top >= common_bottom {
                if word.top < band_bottom {
                    return None;
                }
                break;
            }
            let bottom = word.top + word.height;
            common_bottom = common_bottom.min(bottom);
            band_bottom = band_bottom.max(bottom);
            end += 1;
        }
        let (row, rest) = remaining.split_at_mut(end);
        row.sort_unstable_by_key(|(_, word)| word.left);
        if row
            .windows(2)
            .any(|pair| pair[0].1.left + pair[0].1.width > pair[1].1.left)
        {
            return None;
        }
        rows.push(row);
        remaining = rest;
    }
    Some(rows)
}

struct Track {
    shared: Range<u32>,
    extent: Range<u32>,
}

impl Track {
    fn new(bounds: Range<u32>) -> Self {
        Self {
            shared: bounds.clone(),
            extent: bounds,
        }
    }

    fn admit(&mut self, bounds: Range<u32>) -> Option<()> {
        self.shared.start = self.shared.start.max(bounds.start);
        self.shared.end = self.shared.end.min(bounds.end);
        self.extent.start = self.extent.start.min(bounds.start);
        self.extent.end = self.extent.end.max(bounds.end);
        (self.shared.start < self.shared.end).then_some(())
    }
}

fn build_grid(rows: &[&mut [Word]], max_cells: usize) -> Option<Vec<Vec<String>>> {
    let (header, body) = rows.split_first()?;
    // The independent evidence consists of a named label and at least three recurring values. ~keep
    if header.len() < 4
        || body.len() < 3
        || rows.len().checked_mul(header.len())? > max_cells
        || !header
            .iter()
            .all(|(line, word)| line.is_some() && alphabetic(&word.text))
    {
        return None;
    }
    let mut tracks: Vec<_> = header
        .iter()
        .map(|(_, word)| Track::new(word.left..word.left + word.width))
        .collect();
    let mut grid = vec![header.iter().map(|(_, word)| word.text.clone()).collect()];
    let mut line_rows = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        for (line, _) in row.iter() {
            if line_rows
                .insert((*line)?, index)
                .is_some_and(|previous| previous != index)
            {
                return None;
            }
        }
    }
    for row in body {
        grid.push(body_cells(row, &mut tracks)?);
    }
    tracks
        .windows(2)
        .all(|pair| pair[0].extent.end <= pair[1].extent.start)
        .then_some(grid)
}

fn body_cells(row: &[Word], tracks: &mut [Track]) -> Option<Vec<String>> {
    let (label, values) = row.split_at(row.len().checked_sub(tracks.len() - 1)?);
    let line = label.first()?.0?;
    if !label
        .iter()
        .all(|(key, word)| *key == Some(line) && alphabetic(&word.text))
        || !values.iter().all(|(_, word)| literal_number(&word.text))
    {
        return None;
    }
    let (left, right) = (&label.first()?.1, &label.last()?.1);
    tracks[0].admit(left.left..right.left + right.width)?;
    let label = label
        .iter()
        .map(|(_, word)| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut cells = vec![label];
    for ((_, value), track) in values.iter().zip(&mut tracks[1..]) {
        track.admit(value.left..value.left + value.width)?;
        cells.push(value.text.clone());
    }
    Some(cells)
}

fn alphabetic(text: &str) -> bool {
    text.chars().all(char::is_alphabetic)
}

fn literal_number(text: &str) -> bool {
    text.bytes().any(|byte| byte.is_ascii_digit())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b",.()+-".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    const HEADER: &str =
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";
    const BODY: [[&str; 5]; 3] = [
        ["Street", "repair", "79", "828,296", "(592)"],
        ["Library", "hours", "42", "50,406", "666.17"],
        ["Fleet", "fuel", "69", "8,534", "21,958"],
    ];
    const HEADER_CELLS: [&str; 4] = ["Item", "Count", "Budget", "Change"];
    const REGION: OcrTableBoundingBox = OcrTableBoundingBox {
        left: 180,
        top: 280,
        right: 1300,
        bottom: 545,
    };

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn standalone_recovery_and_default_ocr_have_distinct_cold_and_warm_entries() {
        use super::super::{ImageOcrPolicy, TessPageSegMode, process_image_with_cache_and_policy};
        use crate::core::config::OutputFormat;
        use crate::ocr::cache::OcrCache;
        use crate::ocr::processor::api_pool::TesseractApiPool;

        let directory = tempfile::tempdir().unwrap();
        let cache = OcrCache::new(Some(directory.path().to_path_buf())).unwrap();
        let pool = TesseractApiPool::new();
        let image = include_bytes!("../../../../tests/fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png");
        let config = TesseractConfig {
            psm: TessPageSegMode::PSM_SPARSE_TEXT as u8,
            use_cache: true,
            ..Default::default()
        };
        let extract = |policy| {
            process_image_with_cache_and_policy(image, &config, &cache, &pool, Some(OutputFormat::Plain), policy)
                .unwrap()
        };
        for _ in 0..2 {
            assert!(extract(ImageOcrPolicy::Default).tables.is_empty());
            let recovered = extract(ImageOcrPolicy::StandaloneSparseRows);
            let [table] = recovered.tables.as_slice() else {
                panic!("expected exactly one recovered table");
            };
            assert_eq!(table.cells[7], ["Storm drains", "86", "346,028", "(1.463)"]);
            assert!(recovered.internal_document.unwrap().ocr_text_claimed_by_tables);
            assert_eq!(cache.get_stats().unwrap().total_files, 2);
        }
    }

    fn fixture() -> (String, String) {
        let mut tsv = HEADER.to_owned();
        let mut content = HEADER_CELLS.join(" ");
        for (column, (cell, (left, width))) in HEADER_CELLS
            .into_iter()
            .zip([(192, 80), (724, 110), (930, 132), (1146, 142)])
            .enumerate()
        {
            writeln!(
                tsv,
                "5\t1\t1\t1\t{}\t1\t{left}\t286\t{width}\t28\t96\t{cell}",
                column + 1
            )
            .unwrap();
        }
        for (index, cells) in BODY.into_iter().enumerate() {
            content.push('\n');
            content.push_str(&cells.join(" "));
            for (column, (cell, (left, width))) in cells
                .into_iter()
                .zip([(190, 109), (320, 110), (788, 43), (932, 128), (1160, 127)])
                .enumerate()
            {
                let line = index * 4 + 5 + column.saturating_sub(1);
                let top = 358 + index * 72;
                writeln!(
                    tsv,
                    "5\t1\t1\t1\t{line}\t{}\t{left}\t{top}\t{width}\t28\t96\t{cell}",
                    column + 1
                )
                .unwrap();
            }
        }
        (tsv, content)
    }

    #[test]
    fn complete_label_lines_and_separate_numeric_tracks_keep_every_row() {
        let (tsv, content) = fixture();
        let actual = recover(&tsv, &content, &REGION, &SecurityLimits::default()).unwrap();
        assert_eq!(
            actual,
            [
                ["Item", "Count", "Budget", "Change"],
                ["Street repair", "79", "828,296", "(592)"],
                ["Library hours", "42", "50,406", "666.17"],
                ["Fleet fuel", "69", "8,534", "21,958"],
            ]
        );
    }

    #[test]
    fn differing_literal_inventory_does_not_claim_hocr() {
        let (tsv, content) = fixture();
        let limits = SecurityLimits::default();
        for changed in [
            content.replace("828,296", "828,297"),
            format!("{content}\nOutside prose"),
        ] {
            assert!(recover(&tsv, &changed, &REGION, &limits).is_none());
        }
    }

    #[test]
    fn ambiguous_word_ownership_does_not_merge_rows_or_columns() {
        let (tsv, content) = fixture();
        let limits = SecurityLimits::default();
        for (from, to) in [
            ("5\t1\t1\t1\t5\t2\t", "5\t1\t1\t1\t90\t2\t"),
            ("\t932\t358\t128\t", "\t840\t358\t40\t"),
            ("\t190\t358\t109\t28\t", "\t190\t358\t109\t100\t"),
        ] {
            assert!(tsv.contains(from));
            assert!(recover(&tsv.replace(from, to), &content, &REGION, &limits).is_none());
        }
    }

    #[test]
    fn respects_explicit_table_thresholds() {
        let defaults = TesseractConfig::default();
        assert!(uses_default_table_thresholds(&defaults));
        for configured in [
            TesseractConfig {
                table_min_confidence: 1.0,
                ..defaults.clone()
            },
            TesseractConfig {
                table_column_threshold: defaults.table_column_threshold + 1,
                ..defaults.clone()
            },
            TesseractConfig {
                table_row_threshold_ratio: 0.75,
                ..defaults
            },
        ] {
            assert!(!uses_default_table_thresholds(&configured));
        }
    }

    #[test]
    fn configured_proof_budgets_still_apply() {
        let (tsv, content) = fixture();
        for limits in [
            SecurityLimits {
                max_table_cells: 15,
                ..SecurityLimits::default()
            },
            SecurityLimits {
                max_iterations: 4 * (tsv.len() + content.len()),
                ..SecurityLimits::default()
            },
        ] {
            assert!(recover(&tsv, &content, &REGION, &limits).is_none());
        }
    }
}
