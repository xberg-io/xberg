//! Table assembly in recognition coordinates, without rotating the caller's raster.

use super::*;
use std::fmt::Write;
use xberg_tesseract::{TessOrientation, TessPageIteratorLevel, TessTextlineOrder, TessWritingDirection};

const MAX_ORIENTED_WORDS: usize = 1024;
const MAX_ORIENTED_TSV_BYTES: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    orientation: TessOrientation,
    width: u32,
    height: u32,
}

impl Frame {
    fn rect(self, left: u32, top: u32, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let right = left.checked_add(width)?;
        let bottom = top.checked_add(height)?;
        if width == 0 || height == 0 || right > self.width || bottom > self.height {
            return None;
        }
        use TessOrientation::*;
        Some(match self.orientation {
            ORIENTATION_PAGE_LEFT => (self.height - bottom, left, height, width),
            ORIENTATION_PAGE_RIGHT => (top, self.width - right, height, width),
            ORIENTATION_PAGE_DOWN => (self.width - right, self.height - bottom, width, height),
            ORIENTATION_PAGE_UP => return None,
        })
    }

    fn project_tsv(self, tsv: &str, limit: usize) -> Option<String> {
        let mut output = String::with_capacity(tsv.len());
        let mut lines = tsv.lines();
        writeln!(output, "{}", lines.next()?).ok()?;
        for line in lines {
            let fields: Vec<_> = line.splitn(12, '\t').collect();
            if fields.len() != 12 {
                return None;
            }
            if fields[0] != "5" {
                writeln!(output, "{line}").ok()?;
            } else {
                let (left, top, width, height) = self.rect(
                    fields[6].parse().ok()?,
                    fields[7].parse().ok()?,
                    fields[8].parse().ok()?,
                    fields[9].parse().ok()?,
                )?;
                writeln!(
                    output,
                    "{}\t{left}\t{top}\t{width}\t{height}\t{}\t{}",
                    fields[..6].join("\t"),
                    fields[10],
                    fields[11]
                )
                .ok()?;
            }
            if output.len() > limit {
                return None;
            }
        }
        Some(output)
    }
}

fn uniform_frame(api: &TesseractAPI, dimensions: (u32, u32), limit: usize) -> Option<Frame> {
    let iterator = api.get_page_iterator().ok()?;
    iterator.begin().ok()?;
    let mut agreed = None;
    for _ in 0..limit {
        if !matches!(
            iterator.block_type().ok()?,
            TessPolyBlockType::PT_FLOWING_TEXT
                | TessPolyBlockType::PT_HEADING_TEXT
                | TessPolyBlockType::PT_PULLOUT_TEXT
                | TessPolyBlockType::PT_TABLE
                | TessPolyBlockType::PT_CAPTION_TEXT
        ) {
            return None;
        }
        let (orientation, direction, order, deskew) = iterator.orientation().ok()?;
        tracing::debug!(
            ?orientation,
            ?direction,
            ?order,
            deskew,
            "Recognized text-block orientation"
        );
        agreed = Some(agree(agreed, (orientation, direction, order, deskew))?);
        if !iterator.next(TessPageIteratorLevel::RIL_BLOCK).ok()? {
            return Some(Frame {
                orientation,
                width: dimensions.0,
                height: dimensions.1,
            });
        }
    }
    None
}

fn agree(
    previous: Option<TessOrientation>,
    evidence: (TessOrientation, TessWritingDirection, TessTextlineOrder, f32),
) -> Option<TessOrientation> {
    let (orientation, direction, order, deskew) = evidence;
    (orientation != TessOrientation::ORIENTATION_PAGE_UP
        && direction == TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT
        && order == TessTextlineOrder::TEXTLINE_ORDER_TOP_TO_BOTTOM
        && deskew.is_finite()
        && previous.is_none_or(|previous| previous == orientation))
    .then_some(orientation)
}

pub(super) fn tables(
    api: &TesseractAPI,
    tsv: &str,
    original: &TableWords,
    config: &TesseractConfig,
    dimensions: (u32, u32),
    limits: &SecurityLimits,
) -> Option<Vec<OcrTable>> {
    let byte_limit = MAX_ORIENTED_TSV_BYTES.min(limits.max_content_size);
    let word_limit = MAX_ORIENTED_WORDS.min(limits.max_iterations);
    if !supports(config, tsv.len(), original, limits) {
        return None;
    }
    tracing::debug!(
        psm = config.psm,
        words = original.words.len(),
        "Qualifying recognition-frame table assembly"
    );
    let frame = uniform_frame(api, dimensions, word_limit)?;
    let projected = frame.project_tsv(tsv, byte_limit)?;
    let mut words = extract_table_words_from_tsv(&projected, config.table_min_confidence, &[]).ok()?;
    if !words
        .words
        .iter()
        .map(|word| &word.text)
        .eq(original.words.iter().map(|word| &word.text))
    {
        return None;
    }
    words.read_boxes.clone_from(&original.read_boxes);
    complete_table(&words, config, limits).map(|table| vec![table])
}

fn supports(config: &TesseractConfig, bytes: usize, words: &TableWords, limits: &SecurityLimits) -> bool {
    config.psm == 1
        && config.output_format == "markdown"
        && config.preprocessing.is_none()
        && bytes <= MAX_ORIENTED_TSV_BYTES.min(limits.max_content_size)
        && words.words.len() <= MAX_ORIENTED_WORDS.min(limits.max_iterations)
        && words.words.len() == words.read_boxes.len()
        && words
            .words
            .iter()
            .zip(&words.read_boxes)
            .all(|(word, raw)| word.text == raw.text)
}

fn within_grid_budget(words: &[HocrWord], config: &TesseractConfig, limits: &SecurityLimits) -> bool {
    let rows = detect_rows(words, config.table_row_threshold_ratio);
    // Cell-token grouping cannot create more columns than there are source words.
    let Some(cells) = rows.len().checked_mul(words.len()) else {
        return false;
    };
    cells <= limits.max_table_cells
        && cells
            .checked_mul(size_of::<String>())
            .is_some_and(|bytes| bytes <= limits.max_content_size)
}

fn complete_table(words: &TableWords, config: &TesseractConfig, limits: &SecurityLimits) -> Option<OcrTable> {
    // Until partial-region retention is proved, only replace a single complete table.
    let mut regions = table_regions(words);
    let [region] = regions.as_mut_slice() else {
        return None;
    };
    if region.words.len() != words.words.len() || !has_semantic_ocr_table_content(&region.words) {
        return None;
    }
    if !within_grid_budget(&region.words, config, limits) {
        return None;
    }
    let (table, _) = reconstruct_cleaned_table(&region.words, config);
    if quantity_retry_column_index(&table).is_some() {
        return None;
    }
    #[cfg(feature = "pdf")]
    let table = post_process_table(table, false, false)?;
    tracing::debug!(
        rows = table.len(),
        columns = table.first().map_or(0, Vec::len),
        retained = exact_inventory(&region.words, &table),
        "Recognition-frame table inventory"
    );
    if !has_table_grid(&table)
        || table.len() < 3
        || table.first().is_none_or(|row| row.len() < 2)
        || table.iter().any(|row| row.iter().any(|cell| cell.trim().is_empty()))
        || !exact_inventory(&region.words, &table)
        || table
            .iter()
            .try_fold(0usize, |count, row| count.checked_add(row.len()))?
            > limits.max_table_cells
    {
        return None;
    }
    Some(OcrTable {
        markdown: table_to_markdown(&table),
        cells: table,
        page_number: config.page_number,
        bounding_box: Some(region.bounding_box()),
    })
}

fn exact_inventory(words: &[HocrWord], table: &[Vec<String>]) -> bool {
    let mut inventory = HashMap::new();
    for token in words.iter().flat_map(|word| word.text.split_whitespace()) {
        *inventory.entry(token).or_insert(0usize) += 1;
    }
    for token in table.iter().flatten().flat_map(|cell| cell.split_whitespace()) {
        let Some(remaining) = inventory.get_mut(token).filter(|count| **count > 0) else {
            return false;
        };
        *remaining -= 1;
    }
    inventory.values().all(|remaining| *remaining == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, left: u32, top: u32) -> HocrWord {
        HocrWord {
            text: text.into(),
            left,
            top,
            width: 35,
            height: 14,
            confidence: 99.0,
        }
    }

    fn grid() -> TableWords {
        let rows = [
            ["Item", "Code", "Amount"],
            ["Alpha", "AA", "10"],
            ["Bravo", "BB", "20"],
            ["Charlie", "CC", "30"],
        ];
        let words: Vec<_> = rows
            .iter()
            .enumerate()
            .flat_map(|(row, cells)| {
                cells
                    .iter()
                    .enumerate()
                    .map(move |(column, text)| word(text, 10 + column as u32 * 100, 60 + row as u32 * 30))
            })
            .collect();
        TableWords {
            read_boxes: words.clone(),
            words,
        }
    }

    fn config() -> TesseractConfig {
        TesseractConfig {
            psm: 1,
            output_format: "markdown".into(),
            ..Default::default()
        }
    }

    #[test]
    fn projection_uses_page_orientation_and_checks_original_bounds() {
        use TessOrientation::*;
        for (orientation, expected) in [
            (ORIENTATION_PAGE_LEFT, (130, 20, 40, 10)),
            (ORIENTATION_PAGE_RIGHT, (30, 70, 40, 10)),
            (ORIENTATION_PAGE_DOWN, (70, 130, 10, 40)),
        ] {
            let frame = Frame {
                orientation,
                width: 100,
                height: 200,
            };
            assert_eq!(frame.rect(20, 30, 10, 40), Some(expected));
            assert!(frame.rect(u32::MAX, 0, 2, 1).is_none());
            assert!(frame.rect(95, 0, 10, 1).is_none());
            assert!(frame.rect(0, 0, 0, 1).is_none());
        }
    }

    #[test]
    fn ambiguous_upright_vertical_or_invalid_orientation_is_not_projected() {
        use TessOrientation::*;
        let evidence = |orientation| {
            (
                orientation,
                TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT,
                TessTextlineOrder::TEXTLINE_ORDER_TOP_TO_BOTTOM,
                0.0,
            )
        };
        assert_eq!(
            agree(None, evidence(ORIENTATION_PAGE_LEFT)),
            Some(ORIENTATION_PAGE_LEFT)
        );
        assert!(agree(None, evidence(ORIENTATION_PAGE_UP)).is_none());
        assert!(agree(Some(ORIENTATION_PAGE_LEFT), evidence(ORIENTATION_PAGE_RIGHT)).is_none());
        let mut invalid = evidence(ORIENTATION_PAGE_LEFT);
        invalid.3 = f32::NAN;
        assert!(agree(None, invalid).is_none());
        invalid = evidence(ORIENTATION_PAGE_LEFT);
        invalid.1 = TessWritingDirection::WRITING_DIRECTION_TOP_TO_BOTTOM;
        assert!(agree(None, invalid).is_none());
    }

    #[test]
    fn projection_keeps_exact_text_and_refuses_an_oversized_string() {
        let frame = Frame {
            orientation: TessOrientation::ORIENTATION_PAGE_LEFT,
            width: 100,
            height: 200,
        };
        let tsv = "level\tpage\tblock\tpar\tline\tword\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t20\t30\t10\t40\t99\t-417 12%\n";
        let projected = frame.project_tsv(tsv, 1024).unwrap();
        assert!(projected.ends_with("130\t20\t40\t10\t99\t-417 12%\n"));
        assert!(frame.project_tsv(tsv, 1).is_none());
    }

    #[test]
    fn explicit_payload_modes_and_preprocessing_keep_their_existing_assembly() {
        let words = grid();
        let limits = SecurityLimits::default();
        let mut options = config();
        assert!(supports(&options, 100, &words, &limits));
        for mode in ["text", "hocr", "tsv"] {
            options.output_format = mode.into();
            assert!(!supports(&options, 100, &words, &limits));
        }
        options = config();
        options.psm = 3;
        assert!(!supports(&options, 100, &words, &limits));
        options = config();
        options.preprocessing = Some(Default::default());
        assert!(!supports(&options, 100, &words, &limits));
        options = config();
        let exhausted = SecurityLimits {
            max_iterations: 0,
            ..limits
        };
        assert!(!supports(&options, 100, &words, &exhausted));
    }

    #[test]
    fn exact_inventory_preserves_duplicate_words_signs_units_and_rich_text() {
        let words = [
            word("-417", 0, 0),
            word("12%", 40, 0),
            word("caption", 0, 30),
            word("caption", 60, 30),
        ];
        let cells = |values: &[&str]| vec![values.iter().map(|value| (*value).to_owned()).collect()];
        assert!(exact_inventory(&words, &cells(&["-417", "12%", "caption", "caption"])));
        for values in [
            vec!["417", "12%", "caption", "caption"],
            vec!["-417", "12", "caption", "caption"],
            vec!["-417", "12%", "caption"],
        ] {
            assert!(!exact_inventory(&words, &cells(&values)));
        }
    }

    #[test]
    fn complete_table_abstains_when_budget_or_extra_prose_cannot_be_preserved() {
        let words = grid();
        let options = config();
        let limits = SecurityLimits::default();
        assert!(complete_table(&words, &options, &limits).is_some());
        let exhausted = SecurityLimits {
            max_table_cells: 1,
            ..limits.clone()
        };
        assert!(complete_table(&words, &options, &exhausted).is_none());
        let mut rich = grid();
        let mut caption = word("Important caption remains prose", 10, 10);
        caption.width = 250;
        rich.words.push(caption.clone());
        rich.read_boxes.push(caption);
        assert!(complete_table(&rich, &options, &limits).is_none());
    }
}
