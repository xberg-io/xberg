use super::super::error::OcrError;
use super::super::utils::{TSV_MIN_FIELDS, TSV_WORD_LEVEL};
use crate::table_core::{HocrWord, is_value, median_word_height};
use std::collections::HashMap;
use std::ops::Range;
use xberg_tesseract::WordSymbols;

/// Extract words from Tesseract TSV output and convert to HocrWord format.
///
/// This parses Tesseract's TSV format (level, page_num, block_num, ...) and
/// converts it to the HocrWord format used for table reconstruction.
pub(crate) fn extract_words_from_tsv(tsv_data: &str, min_confidence: f64) -> Result<Vec<HocrWord>, OcrError> {
    Ok(words_on_lines(tsv_data, min_confidence)
        .into_iter()
        .map(|(_, word)| word)
        .collect())
}

/// The page, block, paragraph and line numbers Tesseract gives a word: words with one key sit on
/// one text line.
type TextLine = [u32; 4];

/// The words of [`extract_words_from_tsv`], each with its [`TextLine`], or `None` when a line
/// field does not parse.
fn words_on_lines(tsv_data: &str, min_confidence: f64) -> Vec<(Option<TextLine>, HocrWord)> {
    let mut words = Vec::new();

    for (line_num, line) in tsv_data.lines().enumerate() {
        if line_num == 0 {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < TSV_MIN_FIELDS {
            continue;
        }

        let level = fields[0].parse::<u32>().unwrap_or(0);
        if level != TSV_WORD_LEVEL {
            continue;
        }

        let conf = fields[10].parse::<f64>().unwrap_or(-1.0);
        if conf < min_confidence {
            continue;
        }

        let text = fields[11].trim();
        if text.is_empty() {
            continue;
        }

        let word = HocrWord {
            text: text.to_string(),
            left: fields[6].parse().unwrap_or(0),
            top: fields[7].parse().unwrap_or(0),
            width: fields[8].parse().unwrap_or(0),
            height: fields[9].parse().unwrap_or(0),
            confidence: conf,
        };
        let line = match [fields[1], fields[2], fields[3], fields[4]].map(str::parse::<u32>) {
            [Ok(page), Ok(block), Ok(paragraph), Ok(line)] => Some([page, block, paragraph, line]),
            _ => None,
        };

        words.push((line, word));
    }

    words
}

/// Extract the words table reconstruction reads: [`extract_words_from_tsv`], with the underscore
/// marks that sit against a value cut out of each word (xberg-io/xberg#1833).
///
/// Tesseract reads the edge of a shaded row as underscores fused onto a value (`___7,073_`) or
/// between two values (`(2,100)__(2,163)`), and the fused box closes the gap between two columns.
/// A leading or trailing run is a mark when the text beside it is a value. An interior run of two
/// or more is a mark when the text on both sides is a value. Other underscores are text
/// (`file_name`, `ID__001`, `__init__`, a `____` blank). Each piece takes the box of its own
/// symbols from `symbols` when they spell the word, else the share of the box its characters
/// span. ~keep
pub(crate) fn extract_table_words_from_tsv(
    tsv_data: &str,
    min_confidence: f64,
    symbols: &[WordSymbols],
) -> Result<TableWords, OcrError> {
    let words = words_on_lines(tsv_data, min_confidence);
    let mut table_words = Vec::with_capacity(words.len());
    let mut lines = Vec::with_capacity(words.len());
    let mut read_boxes = Vec::with_capacity(words.len());
    for (line, word) in words {
        let read_box = word.clone();
        push_without_underscore_marks(word, symbols, &mut table_words);
        lines.resize(table_words.len(), line);
        read_boxes.resize(table_words.len(), read_box);
    }
    put_line_words_on_one_band(&mut table_words, &lines);
    Ok(TableWords {
        words: table_words,
        read_boxes,
    })
}

/// The words of [`extract_table_words_from_tsv`], with the box Tesseract read for each one.
#[derive(Debug)]
pub(crate) struct TableWords {
    /// The words rows and columns are built from. Every word on one Tesseract text line has the
    /// vertical box of that line.
    pub(crate) words: Vec<HocrWord>,
    /// The box Tesseract read for the word at the same index of `words`: for a piece cut from a
    /// word at an underscore mark, the box of the whole word. The table bounding box comes from
    /// these boxes because the text outside the table is picked by the centres of the same boxes:
    /// a stretched edge-row word whose box `words` moved, or an edge value whose mark was cut off,
    /// would otherwise fall outside its own table and print twice (xberg-io/xberg#1834). ~keep
    pub(crate) read_boxes: Vec<HocrWord>,
}

/// Give the words on one Tesseract text line the vertical box, the band, of the line's word of
/// typical height (xberg-io/xberg#1834).
///
/// Table rows group words by the centre of their box. Shading can stretch one word's box over the
/// row below, and its centre then passes the row threshold, so the word starts a row of its own
/// while Tesseract reads it on one line with the rest of its label. The band is the box of the
/// line's word whose height is nearest the median and between half and one and a half times the
/// median. A word outside that range, such as a full stop or a box stretched over the next row,
/// never sets it, and a line with no word in the range keeps its boxes. A word moves onto the band
/// only when its own box overlaps the band, so a line that Tesseract runs across two table rows
/// does not join them. ~keep
fn put_line_words_on_one_band(words: &mut [HocrWord], lines: &[Option<TextLine>]) {
    let median_height = u64::from(median_word_height(words));
    let is_typical = |height: u32| (median_height..=3 * median_height).contains(&(2 * u64::from(height)));
    let mut bands: HashMap<TextLine, (u32, u32)> = HashMap::new();
    for (word, line) in words.iter().zip(lines) {
        let Some(line) = line else {
            continue;
        };
        if !is_typical(word.height) {
            continue;
        }
        let distance = |height: u32| u64::from(height).abs_diff(median_height);
        let band = bands.entry(*line).or_insert((word.top, word.height));
        if distance(word.height) < distance(band.1) {
            *band = (word.top, word.height);
        }
    }
    for (word, line) in words.iter_mut().zip(lines) {
        if let Some(&(top, height)) = line.as_ref().and_then(|line| bands.get(line))
            && word.top < top.saturating_add(height)
            && top < word.top.saturating_add(word.height)
        {
            word.top = top;
            word.height = height;
        }
    }
}

fn push_without_underscore_marks(word: HocrWord, symbols: &[WordSymbols], out: &mut Vec<HocrWord>) {
    if !word.text.contains('_') {
        out.push(word);
        return;
    }
    let chars: Vec<char> = word.text.chars().collect();
    let marks = underscore_mark_runs(&chars);
    if marks.is_empty() {
        out.push(word);
        return;
    }
    let char_spans = symbol_char_spans(&word, &chars, symbols);
    let mut piece_start = 0;
    for mark in marks.into_iter().chain(std::iter::once(chars.len()..chars.len())) {
        if mark.start > piece_start {
            out.push(word_piece(
                &word,
                &chars,
                piece_start..mark.start,
                char_spans.as_deref(),
            ));
        }
        piece_start = mark.end;
    }
}

/// The underscore runs of `chars` that are marks rather than text, in order.
fn underscore_mark_runs(chars: &[char]) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let run_end = chars[index..]
            .iter()
            .position(|&ch| ch != '_')
            .map_or(chars.len(), |offset| index + offset);
        if run_end > index {
            runs.push(index..run_end);
            index = run_end;
        } else {
            index += 1;
        }
    }
    let texts: Vec<&[char]> = (0..=runs.len())
        .map(|slot| {
            let start = if slot == 0 { 0 } else { runs[slot - 1].end };
            let end = runs.get(slot).map_or(chars.len(), |run| run.start);
            &chars[start..end]
        })
        .collect();
    runs.into_iter()
        .enumerate()
        .filter(|(slot, run)| match (texts[*slot], texts[slot + 1]) {
            ([], []) => false,
            ([], after) => is_value(after),
            (before, []) => is_value(before),
            (before, after) => run.len() >= 2 && is_value(before) && is_value(after),
        })
        .map(|(_, run)| run)
        .collect()
}

/// The horizontal extent of each character of `word`, from the symbols Tesseract reported for it,
/// or `None` when no reported word matches its box or its symbols do not spell its text.
fn symbol_char_spans(word: &HocrWord, chars: &[char], symbols: &[WordSymbols]) -> Option<Vec<(u32, u32)>> {
    let reported = symbols.iter().find(|reported| {
        reported.text == word.text
            && u32::try_from(reported.left).ok() == Some(word.left)
            && u32::try_from(reported.top).ok() == Some(word.top)
            && u32::try_from(reported.right - reported.left).ok() == Some(word.width)
            && u32::try_from(reported.bottom - reported.top).ok() == Some(word.height)
    })?;
    let mut spans = Vec::with_capacity(chars.len());
    let mut spelled = Vec::with_capacity(chars.len());
    for symbol in &reported.symbols {
        let span = (u32::try_from(symbol.left).ok()?, u32::try_from(symbol.right).ok()?);
        for ch in symbol.text.chars() {
            spans.push(span);
            spelled.push(ch);
        }
    }
    (spelled == chars).then_some(spans)
}

fn word_piece(word: &HocrWord, chars: &[char], piece: Range<usize>, char_spans: Option<&[(u32, u32)]>) -> HocrWord {
    let (left, right) = match char_spans {
        Some(spans) => (
            spans[piece.clone()]
                .iter()
                .map(|span| span.0)
                .min()
                .unwrap_or(word.left),
            spans[piece.clone()]
                .iter()
                .map(|span| span.1)
                .max()
                .unwrap_or(word.left),
        ),
        None => {
            let share = |offset: usize| (u64::from(word.width) * offset as u64 / chars.len() as u64) as u32;
            (word.left + share(piece.start), word.left + share(piece.end))
        }
    };
    HocrWord {
        text: chars[piece].iter().collect(),
        left,
        top: word.top,
        width: right.saturating_sub(left),
        height: word.height,
        confidence: word.confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TSV_HEADER: &str =
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";

    fn table_words_with_symbols(tsv_rows: &str, symbols: &[WordSymbols]) -> Vec<(String, u32, u32)> {
        extract_table_words_from_tsv(&format!("{TSV_HEADER}{tsv_rows}"), 0.0, symbols)
            .unwrap()
            .words
            .into_iter()
            .map(|word| (word.text, word.left, word.width))
            .collect()
    }

    fn table_words(tsv_rows: &str) -> Vec<(String, u32, u32)> {
        table_words_with_symbols(tsv_rows, &[])
    }

    fn owned(words: &[(&str, u32, u32)]) -> Vec<(String, u32, u32)> {
        words
            .iter()
            .map(|&(text, left, width)| (text.to_string(), left, width))
            .collect()
    }

    /// The symbols Tesseract reports for one word: each `(text, left, right)` becomes a symbol.
    fn reported_word(
        text: &str,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        symbols: &[(&str, i32, i32)],
    ) -> WordSymbols {
        WordSymbols {
            text: text.to_string(),
            left,
            top,
            right: left + width,
            bottom: top + height,
            symbols: symbols
                .iter()
                .map(|&(text, left, right)| xberg_tesseract::SymbolBox {
                    text: text.to_string(),
                    left,
                    top,
                    right,
                    bottom: top + height,
                })
                .collect(),
        }
    }

    /// The symbols of `(2,100)__(2,163)` at x 1000, with the second value further right than its
    /// character share (1090) puts it.
    fn fused_pair_symbols(text_of_last_symbol: &str) -> WordSymbols {
        let mut symbols: Vec<(&str, i32, i32)> = ["(", "2", ",", "1", "0", "0", ")"]
            .iter()
            .zip((1000..).step_by(9))
            .map(|(&text, left)| (text, left, left + 8))
            .collect();
        symbols.extend([("_", 1061, 1062), ("_", 1062, 1063)]);
        symbols.extend(
            ["(", "2", ",", "1", "6", "3"]
                .iter()
                .zip((1100..).step_by(9))
                .map(|(&text, left)| (text, left, left + 8)),
        );
        symbols.push((text_of_last_symbol, 1146, 1160));
        reported_word("(2,100)__(2,163)", 1000, 100, 160, 40, &symbols)
    }

    const FUSED_PAIR_ROW: &str = "5\t1\t0\t0\t0\t0\t1000\t100\t160\t40\t0\t(2,100)__(2,163)\n";

    #[test]
    fn table_words_trim_underscore_marks_against_a_value_and_narrow_the_box() {
        let words = table_words(
            "5\t1\t0\t0\t0\t0\t1000\t100\t90\t60\t8\t___7,073_\n5\t1\t0\t0\t0\t1\t1300\t100\t60\t30\t8\t_5,017\n",
        );
        assert_eq!(words, owned(&[("7,073", 1030, 50), ("5,017", 1310, 50)]));
    }

    #[test]
    fn table_words_split_one_word_fused_across_an_underscore_run_between_two_values() {
        assert_eq!(
            table_words(FUSED_PAIR_ROW),
            owned(&[("(2,100)", 1000, 70), ("(2,163)", 1090, 70)])
        );
    }

    #[test]
    fn table_words_take_each_piece_box_from_its_own_symbols() {
        assert_eq!(
            table_words_with_symbols(FUSED_PAIR_ROW, &[fused_pair_symbols(")")]),
            owned(&[("(2,100)", 1000, 62), ("(2,163)", 1100, 60)])
        );
    }

    #[test]
    fn table_words_fall_back_to_the_character_share_when_the_symbols_do_not_spell_the_word() {
        assert_eq!(
            table_words_with_symbols(FUSED_PAIR_ROW, &[fused_pair_symbols("]")]),
            owned(&[("(2,100)", 1000, 70), ("(2,163)", 1090, 70)])
        );
    }

    /// Two words with the same fused text in different rows: each piece takes the character boxes
    /// reported at its own word's position, not those of the first word with that text.
    #[test]
    fn table_words_with_the_same_text_take_the_symbols_reported_at_their_own_box() {
        let symbols = [
            reported_word(
                "___41_",
                1000,
                100,
                60,
                30,
                &[
                    ("_", 1000, 1006),
                    ("_", 1006, 1012),
                    ("_", 1012, 1018),
                    ("4", 1020, 1027),
                    ("1", 1028, 1036),
                    ("_", 1040, 1058),
                ],
            ),
            reported_word(
                "___41_",
                1300,
                260,
                60,
                30,
                &[
                    ("_", 1300, 1302),
                    ("_", 1302, 1304),
                    ("_", 1304, 1306),
                    ("4", 1310, 1317),
                    ("1", 1318, 1326),
                    ("_", 1330, 1358),
                ],
            ),
        ];
        assert_eq!(
            table_words_with_symbols(
                "5\t1\t0\t0\t0\t0\t1000\t100\t60\t30\t90\t___41_\n\
5\t1\t0\t0\t1\t0\t1300\t260\t60\t30\t90\t___41_\n",
                &symbols,
            ),
            owned(&[("41", 1020, 16), ("41", 1310, 16)])
        );
    }

    #[test]
    fn table_words_keep_underscores_that_are_text() {
        let words = table_words(
            "5\t1\t0\t0\t0\t0\t100\t100\t70\t30\t90\tID__001\n\
5\t1\t0\t0\t0\t1\t300\t100\t80\t30\t90\t__init__\n\
5\t1\t0\t0\t0\t2\t500\t100\t40\t4\t50\t____\n\
5\t1\t0\t0\t0\t3\t700\t100\t90\t30\t90\tfile_name\n\
5\t1\t0\t0\t0\t4\t900\t100\t60\t30\t90\tAB__12\n\
5\t1\t0\t0\t0\t5\t1100\t100\t50\t30\t90\t4,871\n\
5\t1\t0\t0\t0\t6\t1300\t100\t60\t30\t90\t12_345\n",
        );
        assert_eq!(
            words,
            owned(&[
                ("ID__001", 100, 70),
                ("__init__", 300, 80),
                ("____", 500, 40),
                ("file_name", 700, 90),
                ("AB__12", 900, 60),
                ("4,871", 1100, 50),
                ("12_345", 1300, 60),
            ])
        );
    }

    /// Two values with a shading mark fused onto the second: the mark's box closes the gap
    /// between the columns, so without the trim the cell merge joins both values into one cell.
    /// Each row sits on a text line of its own, as Tesseract reports it.
    #[test]
    fn underscore_marks_between_two_values_do_not_glue_them_into_one_cell() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t0\t0\t0\t0\t100\t100\t60\t30\t90\tYear\n\
5\t1\t0\t0\t0\t1\t300\t100\t60\t30\t90\tYear\n\
5\t1\t0\t0\t1\t0\t100\t160\t80\t30\t60\t6,867\n\
5\t1\t0\t0\t1\t1\t184\t160\t240\t30\t8\t_____7,073__\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap().words;
        let table = crate::table_core::reconstruct_table(&words, 20, 0.5);
        assert!(
            table.iter().flatten().all(|cell| !cell.contains('_')),
            "no cell may keep an underscore mark: {table:?}"
        );
        let value_row = table.iter().find(|row| row.iter().any(|cell| cell == "6,867")).unwrap();
        assert!(
            value_row.iter().any(|cell| cell == "7,073"),
            "the second value must sit in a cell of its own: {table:?}"
        );
    }

    /// A three-row table whose middle label is two words. The first label word's box is stretched
    /// down over the whole next row, as shading does, and `tail_line` sets the text line the second
    /// label word sits on. The stretched box covers both row bands in full and its centre is nearer
    /// the next row, so row geometry alone puts the word in the next row: only the text line keeps
    /// it with its label. The first row's values arrive fused across an underscore mark, so the
    /// words after it only keep their own lines if each piece of a split word keeps its line. ~keep
    fn stretched_label_table(tail_line: &str) -> Vec<Vec<String>> {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t100\t90\t26\t90\tAlpha\n\
5\t1\t2\t1\t1\t1\t600\t100\t300\t26\t90\t10__20\n\
5\t1\t4\t1\t1\t1\t100\t150\t100\t110\t90\tBravo\n\
5\t{tail_line}\t2\t210\t150\t80\t26\t90\tTail\n\
5\t1\t5\t1\t1\t1\t600\t150\t60\t26\t90\t30\n\
5\t1\t6\t1\t1\t1\t800\t150\t60\t26\t90\t40\n\
5\t1\t7\t1\t1\t1\t100\t230\t90\t26\t90\tCharlie\n\
5\t1\t8\t1\t1\t1\t600\t230\t60\t26\t90\t50\n\
5\t1\t9\t1\t1\t1\t800\t230\t60\t26\t90\t60\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap().words;
        crate::table_core::reconstruct_table(&words, 20, 0.5)
    }

    #[test]
    fn words_on_one_tesseract_line_share_a_row_when_shading_stretches_one_box() {
        let table = stretched_label_table("1\t4\t1\t1");
        assert_eq!(
            table,
            [
                ["Alpha", "10", "20"],
                ["Bravo Tail", "30", "40"],
                ["Charlie", "50", "60"]
            ],
            "the stretched label word must stay in the row of its line"
        );
    }

    /// The negative twin: the same boxes on two text lines. The stretched word keeps its own box
    /// and goes to the next row, so the line is what joins it with its label. ~keep
    #[test]
    fn a_stretched_word_on_a_tesseract_line_of_its_own_goes_to_the_next_row() {
        let table = stretched_label_table("1\t10\t1\t1");
        let row_of = |text: &str| {
            table
                .iter()
                .position(|row| row.iter().any(|cell| cell.split_whitespace().any(|word| word == text)))
        };
        assert_eq!(
            row_of("Bravo"),
            row_of("Charlie"),
            "the stretched word on a line of its own goes to the row its box is nearer: {table:?}"
        );
        assert_eq!(
            row_of("Tail"),
            row_of("30"),
            "the typical word stays in the row of its values: {table:?}"
        );
    }

    /// Tesseract gives one text line to the words of two table rows. No word's box overlaps the
    /// other row, so each row keeps its own boxes and the two rows stay apart.
    #[test]
    fn one_tesseract_line_over_two_table_rows_does_not_join_them() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t120\t90\t26\t90\tAlpha\n\
5\t1\t1\t1\t1\t2\t600\t120\t60\t26\t90\t10\n\
5\t1\t1\t1\t1\t3\t800\t120\t60\t26\t90\t20\n\
5\t1\t1\t1\t1\t4\t100\t170\t90\t26\t90\tBravo\n\
5\t1\t1\t1\t1\t5\t600\t170\t60\t26\t90\t30\n\
5\t1\t1\t1\t1\t6\t800\t170\t60\t26\t90\t40\n\
5\t1\t2\t1\t1\t1\t100\t220\t90\t26\t90\tCharlie\n\
5\t1\t2\t1\t1\t2\t600\t220\t60\t26\t90\t50\n\
5\t1\t2\t1\t1\t3\t800\t220\t60\t26\t90\t60\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap().words;
        assert_eq!(
            crate::table_core::reconstruct_table(&words, 20, 0.5),
            [["Alpha", "10", "20"], ["Bravo", "30", "40"], ["Charlie", "50", "60"]],
            "each table row keeps its own boxes"
        );
    }

    /// Tesseract gives one text line to the words of two table rows, and the band comes from the
    /// lower row because its words are nearer the median height. The upper row lies wholly above
    /// the band, so it keeps its own boxes and the two rows stay apart.
    #[test]
    fn a_line_band_taken_from_the_lower_row_does_not_join_two_table_rows() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t120\t90\t28\t90\tAlpha\n\
5\t1\t1\t1\t1\t2\t600\t120\t60\t28\t90\t10\n\
5\t1\t1\t1\t1\t3\t800\t120\t60\t28\t90\t20\n\
5\t1\t1\t1\t1\t4\t100\t170\t90\t26\t90\tBravo\n\
5\t1\t1\t1\t1\t5\t600\t170\t60\t26\t90\t30\n\
5\t1\t1\t1\t1\t6\t800\t170\t60\t26\t90\t40\n\
5\t1\t2\t1\t1\t1\t100\t220\t90\t26\t90\tCharlie\n\
5\t1\t2\t1\t1\t2\t600\t220\t60\t26\t90\t50\n\
5\t1\t2\t1\t1\t3\t800\t220\t60\t26\t90\t60\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap().words;
        assert_eq!(
            crate::table_core::reconstruct_table(&words, 20, 0.5),
            [["Alpha", "10", "20"], ["Bravo", "30", "40"], ["Charlie", "50", "60"]],
            "each table row keeps its own boxes"
        );
    }

    /// The `(text, top, height)` of each word on text line 1-9-1-1 after the band is applied. Two
    /// rows of typical words on other lines set the median word height to 26.
    fn line_boxes_after_banding(line_rows: &str) -> Vec<(String, u32, u32)> {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t120\t100\t90\t26\t90\tAlpha\n\
5\t1\t1\t1\t1\t2\t620\t100\t60\t26\t90\t10\n\
5\t1\t1\t1\t1\t3\t820\t100\t60\t26\t90\t20\n\
5\t1\t2\t1\t1\t1\t120\t300\t90\t26\t90\tCharlie\n\
5\t1\t2\t1\t1\t2\t620\t300\t60\t26\t90\t50\n\
5\t1\t2\t1\t1\t3\t820\t300\t60\t26\t90\t60\n\
{line_rows}"
        );
        let first_row = ["Alpha", "10", "20", "Charlie", "50", "60"];
        extract_table_words_from_tsv(&tsv, 0.0, &[])
            .unwrap()
            .words
            .into_iter()
            .filter(|word| !first_row.contains(&word.text.as_str()))
            .map(|word| (word.text, word.top, word.height))
            .collect()
    }

    /// A stretched word, a short word, a word of typical height and a full stop on one line: the
    /// band comes from the word of typical height, not from the shortest or tallest box.
    #[test]
    fn the_band_of_a_line_is_its_word_of_typical_height() {
        let boxes = line_boxes_after_banding(
            "5\t1\t9\t1\t1\t1\t120\t150\t90\t62\t90\tBravo\n\
5\t1\t9\t1\t1\t2\t220\t158\t20\t18\t90\tof\n\
5\t1\t9\t1\t1\t3\t250\t150\t80\t26\t90\tTail\n\
5\t1\t9\t1\t1\t4\t340\t172\t4\t4\t90\t.\n",
        );
        assert_eq!(
            boxes,
            [
                ("Bravo".to_string(), 150, 26),
                ("of".to_string(), 150, 26),
                ("Tail".to_string(), 150, 26),
                (".".to_string(), 150, 26),
            ]
        );
    }

    /// A full stop beside a stretched word: neither is of typical height, so neither sets a band
    /// and both keep their own boxes.
    #[test]
    fn a_line_with_no_word_of_typical_height_keeps_its_boxes() {
        let boxes = line_boxes_after_banding(
            "5\t1\t9\t1\t1\t1\t120\t150\t90\t62\t90\tBravo\n\
5\t1\t9\t1\t1\t2\t215\t172\t4\t4\t90\t.\n",
        );
        assert_eq!(boxes, [("Bravo".to_string(), 150, 62), (".".to_string(), 172, 4)]);
    }

    /// A line number that does not parse is unknown, never line 0 shared with other words.
    #[test]
    fn a_word_whose_line_does_not_parse_keeps_its_own_box() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t4\t1\tx\t1\t100\t150\t100\t62\t90\tBravo\n\
5\t1\t4\t1\tx\t2\t210\t150\t80\t26\t90\tTail\n"
        );
        let boxes: Vec<(u32, u32)> = extract_table_words_from_tsv(&tsv, 0.0, &[])
            .unwrap()
            .words
            .iter()
            .map(|word| (word.top, word.height))
            .collect();
        assert_eq!(boxes, [(150, 62), (150, 26)]);
    }

    #[test]
    fn test_extract_words_basic() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5	Hello
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);

        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
        assert_eq!(words[0].top, 50);
        assert_eq!(words[0].confidence, 95.5);

        assert_eq!(words[1].text, "World");
        assert_eq!(words[1].left, 190);
    }

    #[test]
    fn test_extract_words_confidence_filter() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5	Hello
5	1	0	0	0	1	190	50	70	30	50.0	World
5	1	0	0	0	2	270	50	60	30	92.3	Test"#;

        let words = extract_words_from_tsv(tsv, 90.0).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "Test");
    }

    #[test]
    fn test_extract_words_level_filter() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
3	1	0	0	0	0	100	50	80	30	95.5	Paragraph
5	1	0	0	0	0	100	50	80	30	95.5	Hello
4	1	0	0	0	1	190	50	70	30	92.3	Line"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
    }

    #[test]
    fn test_hocr_word_methods() {
        let word = HocrWord {
            text: "Hello".to_string(),
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            confidence: 95.5,
        };

        assert_eq!(word.right(), 180);
        assert_eq!(word.bottom(), 80);
        assert_eq!(word.y_center(), 65.0);
        assert_eq!(word.x_center(), 140.0);
    }

    #[test]
    fn test_extract_words_empty_text() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "World");
    }

    #[test]
    fn test_extract_words_malformed() {
        let tsv = r#"level	page_num	block_num
5	1	0	0	0	0	100	50	80	30	95.5	Hello
invalid line
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "World");
    }
}
