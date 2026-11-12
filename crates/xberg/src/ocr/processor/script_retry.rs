//! Second Tesseract read of a page whose first read is noise in a script the configured list
//! cannot read (GH#2300), merged per line so every word the first read got right stays.

use std::collections::HashMap;
use std::path::Path;

use xberg_tesseract::{TessPageSegMode, TesseractAPI};

use super::api_pool::new_engine;
use super::validation::{TessdataEnv, existing_tessdata_dir_for, first_directory_holding, tessdata_search_dirs};
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::RetainedWordConfidenceStats;

/// The margin by which the mean word confidence of a retry line must exceed the first read's on
/// the same line. ~keep
pub(super) const RETRY_MIN_CONF_GAIN: f64 = 20.0;

/// A page takes the retry only when at least this share of its first-read lines is admitted: one
/// line in `PAGE_FLOOR_DENOMINATOR`. Fixed, never tuned per page. ~keep
const PAGE_FLOOR_DENOMINATOR: usize = 3;

/// The scripts the retry can read, as Tesseract's OSD names them, with the model of each.
const SCRIPT_MODELS: [(&str, &str); 1] = [("Han", "chi_sim")];

/// Tesseract OSD script names the retry knows a model for.
pub(super) fn model_for_script(script: &str) -> Option<&'static str> {
    SCRIPT_MODELS
        .iter()
        .find(|(name, _)| *name == script)
        .map(|(_, model)| *model)
}

/// Whether the second read of a page is wanted at all: the page has a first read of noise, the
/// settings ask for it, and the output is read from the word table. Table detection is on by
/// default, so it cannot turn the retry off: a merged page feeds table detection with its merged
/// word table instead.
pub(super) fn wanted(config: &crate::ocr::types::TesseractConfig, mean_text_conf: f64) -> bool {
    config
        .script_retry
        .as_ref()
        .is_some_and(|settings| mean_text_conf < settings.noisy_below)
        && matches!(config.output_format.as_str(), "text" | "markdown")
}

/// Pick the second read of a noisy page from what OSD said: the script and model OSD names when
/// the configured list cannot read it. An OSD that failed means one read. ~keep
fn choose(osd: Result<&str, &OcrError>, language: &str) -> Option<(String, &'static str)> {
    let script = match osd {
        Ok(script) => script,
        Err(error) => {
            tracing::debug!(%error, "script retry skipped: OSD failed; the first read stands");
            return None;
        }
    };
    let model = model_for_script(script)?;
    (!list_reads_script_of(language, model)).then(|| (script.to_string(), model))
}

/// Whether the configured `+`-joined list already reads the script of `model`.
pub(super) fn list_reads_script_of(language: &str, model: &str) -> bool {
    let readers: &[&str] = match model {
        "chi_sim" => &[
            "chi_sim",
            "chi_tra",
            "chi_sim_vert",
            "chi_tra_vert",
            "jpn",
            "jpn_vert",
            "kor",
            "kor_vert",
            "script/han",
            "script/hans",
            "script/hant",
            "script/japanese",
            "script/hangul",
        ],
        _ => &[],
    };
    language
        .split('+')
        .map(|item| item.trim().to_ascii_lowercase())
        .any(|item| item == "*" || readers.contains(&item.as_str()))
}

/// The retry's models (`osd` and the script models the configured list does not read) that some
/// directory of the tessdata search chain already holds, found without any download. Part of the
/// result key. ~keep
pub(super) fn present_models(
    language: &str,
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
) -> Vec<&'static str> {
    ["osd", "chi_sim"]
        .into_iter()
        .filter(|model| *model == "osd" || !list_reads_script_of(language, model))
        .filter(|model| existing_tessdata_dir_for(&[(*model).to_string()], override_path, tessdata_env).is_some())
        .collect()
}

/// The configured list with `model` appended.
fn retry_list(language: &str, model: &str) -> Vec<String> {
    let mut languages = language
        .split('+')
        .map(|language| language.trim().to_string())
        .collect::<Vec<_>>();
    languages.push(model.to_string());
    languages
}

/// The script OSD names on the first read's pixels, with an engine of its own from `directory`.
fn osd_script(image: &xberg_tesseract::Pix, pass: &Pass<'_>, directory: &str) -> Result<String, OcrError> {
    let osd = new_engine(directory, "osd", 3)?;
    osd.set_page_seg_mode(TessPageSegMode::PSM_OSD_ONLY)
        .map_err(|e| engine_error("OSD mode", e))?;
    osd.set_image_2(image.as_ptr())
        .map_err(|e| engine_error("OSD image", e))?;
    osd.set_source_resolution(pass.source_dpi)
        .map_err(|e| engine_error("OSD resolution", e))?;
    Ok(osd.detect_os().map_err(|e| engine_error("OSD", e))?.2)
}

/// A second read of a page that was merged with the first.
pub(super) struct Outcome {
    script: String,
    model: &'static str,
    pub(super) merged: Merged,
}

/// The models and settings of the page's first read, which the second read repeats.
pub(super) struct Pass<'a> {
    pub(super) config: &'a crate::ocr::types::TesseractConfig,
    pub(super) source_dpi: i32,
}

fn engine_error(step: &str, error: impl std::fmt::Display) -> OcrError {
    OcrError::ProcessingFailed(format!("script retry: {step}: {error}"))
}

/// Name the page's script with OSD on the pixels the first read saw, and when the configured list
/// cannot read that script, read the page again with its model added and merge line by line.
/// OSD and the retry run on engines of their own, built for the page and dropped with it, never
/// on the pool: the page holds its pooled first engine the whole time, and two pages that each
/// hold one and wait for a second pooled engine wait forever (measured: every document of a
/// corpus hit the 600 s limit). Both models are looked up in the installed tessdata only: a model
/// that is absent keeps the first read, and nothing is downloaded. ~keep
pub(super) fn run(first: &TesseractAPI, first_tsv: &str, pass: &Pass<'_>) -> Result<Option<Outcome>, OcrError> {
    let started = std::time::Instant::now();
    let search = tessdata_search_dirs(pass.config.tessdata_path.as_deref(), &TessdataEnv::from_process());
    let Some(osd_directory) = first_directory_holding(&search, &["osd".to_string()]) else {
        tracing::debug!("script retry skipped: osd.traineddata is not installed");
        return Ok(None);
    };
    let image = first
        .thresholded_image()
        .map_err(|e| engine_error("thresholded image", e))?;
    let osd = osd_script(&image, pass, &osd_directory);
    let Some((script, model)) = choose(osd.as_ref().map(String::as_str), &pass.config.language) else {
        return Ok(None);
    };
    let languages = retry_list(&pass.config.language, model);
    let Some(directory) = first_directory_holding(&search, &languages) else {
        tracing::debug!(model, "script retry skipped: the model is not installed");
        return Ok(None);
    };
    let retry = new_engine(&directory, &languages.join("+"), pass.config.oem)?;
    retry
        .set_page_seg_mode(TessPageSegMode::from_int(pass.config.psm as i32))
        .map_err(|e| engine_error("retry mode", e))?;
    super::config::apply_tesseract_variables(&retry, pass.config)?;
    retry
        .set_image_2(image.as_ptr())
        .map_err(|e| engine_error("retry image", e))?;
    retry
        .set_source_resolution(pass.source_dpi)
        .map_err(|e| engine_error("retry resolution", e))?;
    retry.recognize().map_err(|e| engine_error("retry recognize", e))?;
    let retry_tsv = retry.get_tsv_text(0).map_err(|e| engine_error("retry word table", e))?;
    let merged = merge(
        parse_words(first_tsv),
        parse_words(&retry_tsv),
        &script,
        RETRY_MIN_CONF_GAIN,
    );
    tracing::debug!(
        script,
        model,
        elapsed_ms = started.elapsed().as_millis() as u64,
        lines_admitted = merged.admitted,
        lines_refused = merged.refused,
        words_added = merged.added,
        "script retry read the page again"
    );
    Ok(Some(Outcome { script, model, merged }))
}

/// Record the outcome in the result metadata: the merge counts and, for a page that took the
/// retry, the merged confidence.
pub(super) fn record(metadata: &mut HashMap<String, serde_json::Value>, outcome: &Outcome) {
    let merged = &outcome.merged;
    metadata.insert(
        "script_retry".to_string(),
        serde_json::json!({
            "script": outcome.script,
            "model": outcome.model,
            "lines_admitted": merged.admitted,
            "lines_refused": merged.refused,
            "words_added": merged.added,
        }),
    );
    // A page that keeps its first read keeps the first read's confidence metadata too: the merged
    // table's stats describe the same words from the TSV, not the hOCR, and moved ndl_meiji's page
    // confidence with its text unchanged. ~keep
    if merged.changed() {
        let mut stats = RetainedWordConfidenceStats::default();
        for word in merged.words() {
            stats.record(word.conf);
        }
        super::execution::insert_retained_word_confidence_metadata(metadata, &stats);
    }
}

/// One level-5 row of a Tesseract TSV word table.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TsvWord {
    pub(super) line_key: (u32, u32, u32),
    pub(super) left: i64,
    pub(super) top: i64,
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) conf: f64,
    pub(super) text: String,
}

impl TsvWord {
    fn right(&self) -> i64 {
        self.left + self.width
    }

    fn bottom(&self) -> i64 {
        self.top + self.height
    }
}

pub(super) fn parse_words(tsv: &str) -> Vec<TsvWord> {
    tsv.lines()
        .skip(1)
        .filter_map(|row| {
            let fields: Vec<&str> = row.split('\t').collect();
            if fields.len() < 12 || fields[0] != "5" || fields[11].trim().is_empty() {
                return None;
            }
            let number = |index: usize| fields[index].parse::<i64>().ok();
            Some(TsvWord {
                line_key: (
                    fields[2].parse().ok()?,
                    fields[3].parse().ok()?,
                    fields[4].parse().ok()?,
                ),
                left: number(6)?,
                top: number(7)?,
                width: number(8)?,
                height: number(9)?,
                conf: fields[10].parse().ok()?,
                text: fields[11].to_string(),
            })
        })
        .collect()
}

/// The word without leading and trailing punctuation.
fn core(text: &str) -> &str {
    text.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_'))
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic() || ('\u{C0}'..='\u{24F}').contains(&c)
}

fn has_latin_or_digit(text: &str) -> bool {
    text.chars().any(|c| is_latin(c) || c.is_ascii_digit())
}

/// A first-read token with a Latin letter or a digit in it (`Ref`, `3124`, `It's`, `O'Brien`,
/// `50%`, `a_b`): protected at any confidence.
fn is_protected_text(text: &str) -> bool {
    has_latin_or_digit(text)
}

/// Whether `c` is a letter of the script OSD named, as the retry's models write it.
fn script_letter(script: &str, c: char) -> bool {
    match script {
        "Han" => ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c),
        _ => false,
    }
}

fn mean_conf(words: &[TsvWord]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    words.iter().map(|word| word.conf).sum::<f64>() / words.len() as f64
}

fn x_overlap(a: &TsvWord, b: &TsvWord) -> i64 {
    (a.right().min(b.right()) - a.left.max(b.left)).max(0)
}

/// The width of `word` that the union of the retry words covers.
fn covered_width(word: &TsvWord, retry: &[TsvWord]) -> i64 {
    let mut spans: Vec<(i64, i64)> = retry
        .iter()
        .filter(|candidate| x_overlap(word, candidate) > 0)
        .map(|candidate| (candidate.left.max(word.left), candidate.right().min(word.right())))
        .collect();
    spans.sort_unstable();
    let mut covered = 0;
    let mut end = word.left;
    for (left, right) in spans {
        let left = left.max(end);
        if right > left {
            covered += right - left;
            end = right;
        }
    }
    covered
}

fn y_overlap(top: i64, bottom: i64, word: &TsvWord) -> i64 {
    (bottom.min(word.bottom()) - top.max(word.top)).max(0)
}

struct Line {
    top: i64,
    bottom: i64,
    left: i64,
    right: i64,
    words: Vec<TsvWord>,
}

/// Put the words of one line in reading order: by left edge, then top. The word tables list
/// words in the order the engines found them, which the merged page must not depend on. ~keep
fn reading_order(words: &mut [TsvWord]) {
    words.sort_by_key(|word| (word.left, word.top));
}

/// Tesseract lines of a word table, in reading order: by top edge, then left edge, each line's
/// words left to right.
fn lines_of(words: Vec<TsvWord>) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut keys: Vec<(u32, u32, u32)> = Vec::new();
    for word in words {
        let index = match keys.iter().position(|key| *key == word.line_key) {
            Some(index) => index,
            None => {
                keys.push(word.line_key);
                lines.push(Line {
                    top: word.top,
                    bottom: word.bottom(),
                    left: word.left,
                    right: word.right(),
                    words: Vec::new(),
                });
                lines.len() - 1
            }
        };
        let line = &mut lines[index];
        line.top = line.top.min(word.top);
        line.bottom = line.bottom.max(word.bottom());
        line.left = line.left.min(word.left);
        line.right = line.right.max(word.right());
        line.words.push(word);
    }
    for line in &mut lines {
        reading_order(&mut line.words);
    }
    lines.sort_by_key(|line| (line.top, line.left));
    lines
}

/// Give each retry word to the first-pass line that holds at least half of its height and shares
/// its horizontal extent; the rest are words the first pass never read.
fn assign(retry: Vec<TsvWord>, first: &[Line]) -> (Vec<Vec<TsvWord>>, Vec<TsvWord>) {
    let mut by_line: Vec<Vec<TsvWord>> = first.iter().map(|_| Vec::new()).collect();
    let mut extra = Vec::new();
    for word in retry {
        let best = first
            .iter()
            .enumerate()
            .filter(|(_, line)| line.right.min(word.right()) > line.left.max(word.left))
            .map(|(index, line)| (y_overlap(line.top, line.bottom, &word), index))
            .filter(|(cover, _)| *cover > 0)
            .max_by_key(|(cover, index)| (*cover, std::cmp::Reverse(*index)));
        match best {
            Some((cover, index)) if cover * 2 >= word.height => by_line[index].push(word),
            _ => extra.push(word),
        }
    }
    (by_line, extra)
}

/// Why one line kept its first-pass words.
#[derive(Debug, PartialEq)]
enum Refusal {
    /// Half or fewer of the retry line's letters are in the target script.
    Script { target: usize, letters: usize },
    /// The retry line's mean confidence does not beat the first read's by the margin.
    Confidence { gain: f64 },
    /// A protected first-pass word is neither read again on its box nor covered by retry
    /// words in the target script: the retry read other Latin or digit text there, or nothing.
    Lost(String),
    /// Retry words cover less than half of a first-pass word's width.
    Unread(String),
}

/// More than half of the retry words' letters are in the target script. Exactly half
/// is not mostly: a two-letter reread of a stray mark (`@ +` as `% i 上`) would otherwise take a
/// Japanese page that is settled byte-equal. ~keep
fn mostly_script(retry: &[TsvWord], script: &str) -> Result<(), Refusal> {
    let letters: Vec<char> = retry
        .iter()
        .flat_map(|word| word.text.chars())
        .filter(|c| c.is_alphabetic())
        .collect();
    let target = letters.iter().filter(|c| script_letter(script, **c)).count();
    if target * 2 <= letters.len() {
        return Err(Refusal::Script {
            target,
            letters: letters.len(),
        });
    }
    Ok(())
}

/// The retry read a first-read word's box as letters of the target script only. A word
/// of Latin marks under a Han glyph is the first read's junk, not a word the retry lost; other
/// Latin or digit text on the box, or no retry word at all, is a lost word. ~keep
fn replaced_by_script_letters(word: &TsvWord, retry: &[TsvWord]) -> bool {
    let over: Vec<&TsvWord> = retry
        .iter()
        .filter(|candidate| x_overlap(word, candidate) > 0)
        .collect();
    !over.is_empty() && !over.iter().any(|candidate| has_latin_or_digit(&candidate.text))
}

/// The four line rules (script share, confidence gain, protected words, word coverage), each on one
/// first-read line and the retry words assigned to it; `Ok` carries the confidence gain. ~keep
fn admit_line(first: &[TsvWord], retry: &[TsvWord], script: &str, margin: f64) -> Result<f64, Refusal> {
    mostly_script(retry, script)?;
    let gain = mean_conf(retry) - mean_conf(first);
    if gain <= margin {
        return Err(Refusal::Confidence { gain });
    }
    let mut used = vec![false; retry.len()];
    for word in first.iter().filter(|word| is_protected_text(core(&word.text))) {
        let found = retry.iter().enumerate().find(|(index, candidate)| {
            !used[*index]
                && core(&candidate.text) == core(&word.text)
                && x_overlap(word, candidate) * 2 >= word.width.min(candidate.width)
        });
        match found {
            Some((index, _)) => used[index] = true,
            None if replaced_by_script_letters(word, retry) => {}
            None => return Err(Refusal::Lost(core(&word.text).to_string())),
        }
    }
    for word in first {
        if covered_width(word, retry) * 2 < word.width {
            return Err(Refusal::Unread(word.text.clone()));
        }
    }
    Ok(gain)
}

/// The merged page: lines in reading order, each from the retry or from the first pass.
#[derive(Debug, Default)]
pub(super) struct Merged {
    pub(super) lines: Vec<Vec<TsvWord>>,
    pub(super) admitted: usize,
    pub(super) refused: usize,
    pub(super) added: usize,
}

impl Merged {
    pub(super) fn changed(&self) -> bool {
        self.admitted > 0 || self.added > 0
    }

    pub(super) fn words(&self) -> impl Iterator<Item = &TsvWord> {
        self.lines.iter().flatten()
    }

    /// Lines joined by newlines; a gap wider than the median line height ends a paragraph.
    pub(super) fn text(&self) -> String {
        let mut heights: Vec<i64> = self
            .lines
            .iter()
            .filter_map(|line| line.iter().map(|word| word.height).max())
            .collect();
        heights.sort_unstable();
        let median_height = heights.get(heights.len() / 2).copied().unwrap_or(0);
        let mut text = String::new();
        let mut previous_bottom: Option<i64> = None;
        for line in &self.lines {
            let top = line.iter().map(|word| word.top).min().unwrap_or(0);
            let bottom = line.iter().map(|word| word.bottom()).max().unwrap_or(top);
            if let Some(previous) = previous_bottom {
                text.push('\n');
                if top - previous > median_height {
                    text.push('\n');
                }
            }
            let words: Vec<&str> = line.iter().map(|word| word.text.as_str()).collect();
            text.push_str(&words.join(" "));
            previous_bottom = Some(bottom);
        }
        text
    }

    /// A Tesseract TSV word table of the merged page. Every line keeps the block and paragraph
    /// numbers of the first-read line it stands for, and is numbered within its paragraph.
    pub(super) fn tsv(&self) -> String {
        let mut tsv = String::from(
            "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext",
        );
        let mut numbered: HashMap<(u32, u32), usize> = HashMap::new();
        for line in &self.lines {
            let Some(first) = line.first() else { continue };
            let (block, paragraph, _) = first.line_key;
            let line_number = numbered.entry((block, paragraph)).or_insert(0);
            *line_number += 1;
            for (word_index, word) in line.iter().enumerate() {
                tsv.push_str(&format!(
                    "\n5\t1\t{block}\t{paragraph}\t{line_number}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    word_index + 1,
                    word.left,
                    word.top,
                    word.width,
                    word.height,
                    word.conf,
                    word.text
                ));
            }
        }
        tsv
    }
}

/// Merge the retry into the first pass line by line: a line takes the retry's words only when
/// they are mostly letters of `script`, beat the first read's confidence by `margin`,
/// keep every protected first-pass word and cover half of every first-pass word.
/// A page takes any retry line only when at least one third of its first-read lines is admitted
/// (the page floor); a page below the floor keeps its first read whole. Retry words outside every
/// first-pass line are added only when the page takes the retry, and only as lines that are mostly
/// letters of `script`: the retry's junk between lines never joins a page. ~keep
pub(super) fn merge(first: Vec<TsvWord>, retry: Vec<TsvWord>, script: &str, margin: f64) -> Merged {
    let first_lines = lines_of(first);
    let next_block = first_lines
        .iter()
        .flat_map(|line| line.words.iter().map(|word| word.line_key.0))
        .max()
        .map_or(1, |block| block + 1);
    let (by_line, extra) = assign(retry, &first_lines);
    let mut merged = Merged::default();
    let mut lines: Vec<((i64, i64), Vec<TsvWord>)> = Vec::new();
    let mut kept: Vec<((i64, i64), Vec<TsvWord>)> = Vec::new();
    for (line, mut candidates) in first_lines.into_iter().zip(by_line) {
        let position = (line.top, line.left);
        match admit_line(&line.words, &candidates, script, margin) {
            Ok(gain) => {
                tracing::debug!(gain, "script retry: line takes the retry's read");
                merged.admitted += 1;
                reading_order(&mut candidates);
                let key = line.words.first().map(|word| word.line_key);
                if let Some(key) = key {
                    for word in &mut candidates {
                        word.line_key = key;
                    }
                }
                lines.push((position, candidates));
            }
            Err(refusal) => {
                tracing::debug!(?refusal, "script retry: line keeps its first read");
                merged.refused += 1;
                lines.push((position, line.words.clone()));
            }
        }
        kept.push((position, line.words));
    }
    if merged.admitted * PAGE_FLOOR_DENOMINATOR < merged.admitted + merged.refused || merged.admitted == 0 {
        tracing::debug!(
            admitted = merged.admitted,
            lines = merged.admitted + merged.refused,
            "script retry: the page is below the floor and keeps its first read"
        );
        merged.refused += merged.admitted;
        merged.admitted = 0;
        lines = kept;
    } else {
        for line in lines_of(extra) {
            if mostly_script(&line.words, script).is_ok() {
                merged.added += line.words.len();
                let words = line
                    .words
                    .into_iter()
                    .map(|word| TsvWord {
                        line_key: (next_block, 1, 1),
                        ..word
                    })
                    .collect();
                lines.push(((line.top, line.left), words));
            }
        }
    }
    lines.sort_by_key(|(position, _)| *position);
    merged.lines = lines.into_iter().map(|(_, words)| words).collect();
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAN_LINE: &str = "今天天气很好我们去公园散步";

    /// Words of one line from (text, conf, left, width), at `top` with height 40.
    fn line(specs: &[(&str, f64, i64, i64)], key: u32, top: i64) -> Vec<TsvWord> {
        specs
            .iter()
            .map(|(text, conf, left, width)| TsvWord {
                line_key: (1, 1, key),
                left: *left,
                top,
                width: *width,
                height: 40,
                conf: *conf,
                text: (*text).to_string(),
            })
            .collect()
    }

    fn joined(merged: &Merged) -> String {
        merged
            .words()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn han_count(text: &str) -> usize {
        text.chars().filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c)).count()
    }

    const HAN: &str = "Han";

    fn merge_han(first: Vec<TsvWord>, retry: Vec<TsvWord>) -> Merged {
        merge(first, retry, HAN, RETRY_MIN_CONF_GAIN)
    }

    /// Every protected first-pass occurrence is still in the output, counted per occurrence.
    fn missing_protected(first: &[TsvWord], merged: &Merged) -> Vec<String> {
        let mut have: Vec<&str> = merged.words().map(|word| core(&word.text)).collect();
        let mut missing = Vec::new();
        for word in first.iter().filter(|word| is_protected_text(core(&word.text))) {
            match have.iter().position(|text| *text == core(&word.text)) {
                Some(index) => {
                    have.swap_remove(index);
                }
                None => missing.push(core(&word.text).to_string()),
            }
        }
        missing
    }

    fn ref_fixture() -> (Vec<TsvWord>, Vec<TsvWord>) {
        let mut first = line(
            &[
                ("SRA", 35.0, 0, 200),
                ("TURF", 30.0, 220, 300),
                ("RINE", 33.0, 540, 300),
            ],
            0,
            20,
        );
        first.extend(line(
            &[("|", 10.0, 0, 150), ("Ref", 92.0, 180, 90), ("3124", 93.0, 290, 120)],
            1,
            100,
        ));
        let mut retry = line(
            &[(&HAN_LINE[..21], 93.0, 0, 400), (&HAN_LINE[21..], 93.0, 420, 420)],
            0,
            20,
        );
        retry.extend(line(&[("参考编号", 95.0, 0, 150), ("Ref3124", 95.0, 180, 230)], 1, 100));
        (first, retry)
    }

    /// A retry line whose letters are not mostly Han (p999: digits reread as digits, p906:
    /// `Client 01` reread as `Client oon`) keeps the first read, whatever its confidence.
    #[test]
    fn a_line_with_no_target_script_letters_keeps_its_first_read() {
        let first = line(
            &[
                ("Client", 60.0, 0, 120),
                ("01", 60.0, 140, 60),
                ("Information", 60.0, 220, 200),
            ],
            1,
            100,
        );
        let retry = line(
            &[
                ("Client", 95.0, 0, 120),
                ("oon", 95.0, 140, 60),
                ("Information", 95.0, 220, 200),
            ],
            1,
            100,
        );
        let merged = merge_han(first.clone(), retry);
        assert_eq!(joined(&merged), "Client 01 Information");
        assert!(!merged.changed());
        let digits = line(&[("10-50", 60.0, 0, 120), ("0000151", 60.0, 140, 160)], 1, 100);
        let reread = line(&[("18-58", 95.0, 0, 120), ("866866168:", 95.0, 140, 160)], 1, 100);
        assert_eq!(joined(&merge_han(digits, reread)), "10-50 0000151");
    }

    /// At the script-share bound: a retry line with one Han letter in two (jpn_vert: `@ +` reread as
    /// `% i 上`, gain 32.5) is not mostly Han, so the page admits no line and takes no retry-only
    /// line either, and stays byte-equal to its first read.
    #[test]
    fn a_retry_line_with_exactly_half_its_letters_in_the_script_keeps_its_first_read() {
        let first = line(&[("@", 20.0, 0, 40), ("+", 25.0, 60, 40)], 1, 100);
        let mut retry = line(&[("%", 60.0, 0, 30), ("i", 60.0, 40, 20), ("上", 60.0, 70, 30)], 1, 100);
        retry.extend(line(&[("语", 90.0, 0, 40)], 2, 300));
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged), "@ +");
        assert!(!merged.changed());
        assert_eq!((merged.admitted, merged.refused, merged.added), (0, 1, 0));
    }

    /// A Han retry line that does not beat the first read's confidence by the margin keeps
    /// the first read (ndl_meiji: a page gain of 4.3 points).
    #[test]
    fn a_retry_line_no_more_confident_than_the_first_keeps_its_first_read() {
        let first = line(&[("|", 70.0, 0, 120), ("~", 70.0, 140, 160)], 1, 100);
        let retry = line(&[("今天天", 80.0, 0, 120), ("气", 80.0, 140, 160)], 1, 100);
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged), "| ~");
        assert!(!merged.changed());
    }

    /// `Ref 3124` at confidence 0 is protected like any other Latin or digit token, and a
    /// digit line that the retry reads as other digits with one Han glyph keeps its first read.
    #[test]
    fn zero_confidence_ref_3124_survives() {
        let first = line(&[("Ref", 0.0, 0, 90), ("3124", 0.0, 110, 120)], 1, 100);
        let retry = line(&[("Rcf", 95.0, 0, 90), ("参考编号", 95.0, 110, 120)], 1, 100);
        assert_eq!(joined(&merge_han(first, retry)), "Ref 3124");
        let digits = line(&[("10-50", 0.0, 0, 120), ("0000151", 0.0, 140, 160)], 1, 100);
        let reread = line(&[("18-58", 95.0, 0, 120), ("一", 95.0, 140, 160)], 1, 100);
        assert_eq!(joined(&merge_han(digits, reread)), "10-50 0000151");
    }

    /// A token of letters and digits (`A3`) is protected.
    #[test]
    fn a_confident_a3_survives() {
        let first = line(&[("A3", 50.0, 0, 90), ("|", 50.0, 110, 120)], 1, 100);
        let retry = line(&[("A8", 95.0, 0, 90), ("项目编号", 95.0, 110, 120)], 1, 100);
        assert_eq!(joined(&merge_han(first, retry)), "A3 |");
        assert!(is_protected_text("A3") && is_protected_text("Ref3124"));
    }

    /// One retry `Ref` box that overlaps two first-read `Ref` words matches one of them.
    #[test]
    fn two_ref_occurrences_need_two_matches() {
        let first = line(
            &[("Ref", 50.0, 0, 90), ("Ref", 50.0, 100, 90), ("|", 50.0, 200, 300)],
            1,
            100,
        );
        let retry = line(&[("Ref", 95.0, 40, 150), ("参考资料", 95.0, 200, 300)], 1, 100);
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged), "Ref Ref |");
        assert!(!merged.changed());
    }

    /// A retry word 27 px wide does not cover a first-read word 99 px wide.
    #[test]
    fn a_27px_fragment_does_not_cover_a_99px_word() {
        let first = line(&[("|", 30.0, 0, 99)], 1, 100);
        let retry = line(&[("今天天气很好", 90.0, 0, 27)], 1, 100);
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged), "|");
        assert!(!merged.changed());
        let two = line(&[("今天天", 90.0, 0, 27), ("气很好", 90.0, 60, 20)], 1, 100);
        assert_eq!(joined(&merge_han(line(&[("|", 30.0, 0, 99)], 1, 100), two)), "|");
        let covering = line(&[("今天天", 90.0, 0, 27), ("气很好", 90.0, 27, 27)], 1, 100);
        assert!(merge_han(line(&[("|", 30.0, 0, 99)], 1, 100), covering).changed());
    }

    /// A faint English line (confidence 40) survives a Han retry.
    #[test]
    fn a_faint_english_line_survives() {
        let first = line(
            &[("The", 40.0, 0, 90), ("quick", 40.0, 110, 120), ("fox", 40.0, 250, 90)],
            1,
            100,
        );
        let retry = line(
            &[
                ("今天天气很好我们去公", 92.0, 0, 90),
                ("qvick", 92.0, 110, 120),
                ("fox", 92.0, 250, 90),
            ],
            1,
            100,
        );
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged), "The quick fox");
        assert_eq!(han_count(&joined(&merged)), 0);
    }

    /// A line the first read saw as marks only takes the Han read that covers it.
    #[test]
    fn a_line_of_marks_is_replaced_by_the_han_read() {
        let marks = line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 1, 100);
        let han = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 100);
        let merged = merge_han(marks, han);
        assert_eq!(han_count(&joined(&merged)), 4, "{}", joined(&merged));
        assert_eq!((merged.admitted, merged.refused, merged.added), (1, 0, 0));
    }

    #[test]
    fn u1_a_short_confident_fragment_does_not_replace_a_long_line() {
        let first: Vec<TsvWord> = (0..20)
            .map(|i| TsvWord {
                line_key: (1, 1, 1),
                left: 100 * i,
                top: 100,
                width: 90,
                height: 40,
                conf: 5.0,
                text: format!("x{i:04}"),
            })
            .collect();
        let retry = line(&[("今天天气很好", 90.0, 0, 90)], 1, 100);
        let merged = merge_han(first.clone(), retry);
        assert_eq!((merged.admitted, merged.refused, merged.added), (0, 1, 0));
        assert_eq!(merged.words().count(), 20, "{}", joined(&merged));
    }

    #[test]
    fn u2_invoice_tokens_survive_while_the_line_of_marks_turns_han() {
        let mut first: Vec<TsvWord> = (0..12)
            .map(|i| TsvWord {
                line_key: (1, 1, 1),
                left: 70 * i,
                top: 100,
                width: 60,
                height: 40,
                conf: 10.0,
                text: ["|", "~", "'"][(i as usize) % 3].to_string(),
            })
            .collect();
        first.extend(line(
            &[
                ("Invoice", 91.0, 0, 200),
                ("1042,", 88.0, 220, 120),
                ("total", 90.0, 360, 140),
                ("385.20", 86.0, 520, 170),
            ],
            2,
            200,
        ));
        let prose: Vec<char> = format!("{HAN_LINE}图书馆的新书放在二楼的左边").chars().collect();
        let mut retry: Vec<TsvWord> = (0..6)
            .map(|i| TsvWord {
                line_key: (1, 1, 1),
                left: 140 * i,
                top: 100,
                width: 130,
                height: 40,
                conf: 90.0,
                text: prose.iter().skip((i as usize) * 5).take(5).collect(),
            })
            .collect();
        retry.extend(line(
            &[
                ("发票", 70.0, 0, 200),
                ("一〇四二", 70.0, 220, 300),
                ("元", 70.0, 540, 150),
            ],
            2,
            200,
        ));
        let merged = merge_han(first.clone(), retry);
        assert!(missing_protected(&first, &merged).is_empty(), "{}", joined(&merged));
        assert!(han_count(&joined(&merged)) >= 6, "{}", joined(&merged));
        assert_eq!((merged.admitted, merged.refused), (1, 1));
    }

    #[test]
    fn u3_a_fused_ref_number_keeps_the_first_pass_line_only() {
        let (first, retry) = ref_fixture();
        let merged = merge_han(first, retry);
        assert!(joined(&merged).contains("Ref 3124"), "{}", joined(&merged));
        assert!(!joined(&merged).contains("Ref3124"), "{}", joined(&merged));
    }

    #[test]
    fn u6_the_same_token_on_another_line_does_not_stand_in_for_the_fused_one() {
        let (mut first, mut retry) = ref_fixture();
        let other = line(&[("Ref", 92.0, 0, 90), ("3124", 93.0, 110, 120)], 2, 200);
        first.extend(other.clone());
        retry.extend(other);
        let merged = merge_han(first, retry);
        assert_eq!(joined(&merged).matches("3124").count(), 2, "{}", joined(&merged));
    }

    #[test]
    fn u4_a_retry_that_keeps_every_protected_token_is_admitted() {
        let (first, _) = ref_fixture();
        let retry = line(
            &[
                ("参考编号", 95.0, 0, 150),
                ("Ref", 95.0, 180, 90),
                ("3124", 95.0, 290, 120),
            ],
            1,
            100,
        );
        let merged = merge_han(first.clone(), retry);
        assert!(missing_protected(&first, &merged).is_empty());
        assert_eq!(han_count(&joined(&merged)), 4, "{}", joined(&merged));
        assert_eq!(
            (merged.admitted, merged.refused),
            (1, 1),
            "the prose line has no retry words"
        );
    }

    #[test]
    fn u5_a_junk_latin_line_keeps_its_first_read() {
        let junk = line(&[("SRA", 31.0, 0, 120), ("TURF", 28.0, 140, 160)], 1, 100);
        let han = line(&[("SRB", 92.0, 0, 120), ("今天天气很好", 92.0, 140, 160)], 1, 100);
        let merged = merge_han(junk, han);
        assert_eq!(joined(&merged), "SRA TURF");
        assert!(!merged.changed());
    }

    #[test]
    fn retry_words_outside_every_first_pass_line_are_added_in_reading_order() {
        let first = line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 1, 300);
        let mut retry = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 300);
        retry.extend(line(&[("标题", 90.0, 0, 120)], 0, 20));
        let merged = merge_han(first, retry);
        assert_eq!((merged.admitted, merged.added), (1, 1));
        assert_eq!(merged.text(), "标题\n\n今天天 气");
    }

    /// A page whose every line keeps its first read takes no retry-only line (a Japanese page
    /// read with the Chinese model gained twelve junk lines between its own), and a page that
    /// admitted a line takes only the retry-only lines that are mostly its script.
    #[test]
    fn retry_only_lines_need_an_admitted_line_and_the_page_script() {
        let first = line(&[("Total", 95.0, 0, 120)], 1, 300);
        let mut retry = line(&[("Total", 95.0, 0, 120)], 1, 300);
        retry.extend(line(&[("标题", 90.0, 0, 120)], 0, 20));
        let merged = merge_han(first.clone(), retry);
        assert_eq!((merged.admitted, merged.added), (0, 0));
        assert!(!merged.changed());
        assert_eq!(merged.text(), "Total");
        let marks = line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 1, 300);
        let mut han = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 300);
        han.extend(line(&[("Ab", 60.0, 0, 50), ("ns", 60.0, 60, 50)], 0, 20));
        han.extend(line(&[("标题", 90.0, 0, 120)], 2, 500));
        let merged = merge_han(marks, han);
        assert_eq!((merged.admitted, merged.added), (1, 1));
        assert_eq!(merged.text(), "今天天 气\n\n标题");
    }

    /// A page that keeps its first read keeps the first read's confidence metadata: the record
    /// names the retry and counts its refusals, and writes word confidence stats only for a
    /// merged page (ndl_meiji's page confidence moved with its text unchanged).
    #[test]
    fn a_refused_page_keeps_the_first_reads_confidence_metadata() {
        let outcome = |first: Vec<TsvWord>, retry: Vec<TsvWord>| Outcome {
            script: HAN.to_string(),
            model: "chi_sim",
            merged: merge_han(first, retry),
        };
        let first_read_mean = serde_json::json!(31);
        let mut metadata = HashMap::from([("mean_text_conf".to_string(), first_read_mean.clone())]);
        let refused = line(&[("Client", 60.0, 0, 120)], 1, 100);
        let reread = line(&[("Client", 95.0, 0, 120)], 1, 100);
        record(&mut metadata, &outcome(refused, reread));
        assert_eq!(metadata["script_retry"]["lines_refused"], 1);
        assert_eq!(metadata["mean_text_conf"], first_read_mean);
        assert!(!metadata.contains_key("word_count"));

        let marks = line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 1, 300);
        let han = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 300);
        record(&mut metadata, &outcome(marks, han));
        assert_eq!(metadata["script_retry"]["lines_admitted"], 1);
        assert_eq!(metadata["mean_text_conf"], serde_json::json!(92));
        assert_eq!(metadata["word_count"], 2);
    }

    /// Probe: the order in which the two word tables list their words does not change the page,
    /// for a refused line, an admitted line, an added line and two columns on one row.
    #[test]
    fn the_merged_page_does_not_depend_on_the_order_of_the_word_tables() {
        let (mut first, mut retry) = ref_fixture();
        first.extend(line(&[("Col", 95.0, 1200, 90), ("B", 95.0, 1300, 40)], 2, 20));
        retry.extend(line(&[("Col", 95.0, 1200, 90), ("B", 95.0, 1300, 40)], 2, 20));
        first.extend(line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 3, 300));
        retry.extend(line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 3, 300));
        retry.extend(line(&[("标题", 90.0, 0, 120), ("二", 90.0, 140, 40)], 4, 400));
        let forward = merge_han(first.clone(), retry.clone());
        assert_eq!((forward.admitted, forward.refused, forward.added), (2, 2, 2));
        let mut first_reversed = first;
        first_reversed.reverse();
        let mut retry_reversed = retry;
        retry_reversed.reverse();
        let backward = merge_han(first_reversed, retry_reversed);
        assert_eq!(forward.text(), backward.text());
        assert_eq!(forward.tsv(), backward.tsv());
        assert!(forward.text().contains("| Ref 3124"), "{}", forward.text());
        assert!(forward.text().contains("Col B"), "{}", forward.text());
        assert!(forward.text().ends_with("标题 二"), "{}", forward.text());
    }

    /// GH#2300: OSD's script chooses the second read; an OSD that failed, a script the configured
    /// list reads and a script with no model each mean one read.
    #[test]
    fn the_second_read_is_chosen_from_osd_alone() {
        let han = Some(("Han".to_string(), "chi_sim"));
        assert_eq!(choose(Ok("Han"), "eng"), han);
        assert_eq!(
            choose(Ok("Khmer"), "eng"),
            None,
            "no model for Khmer: OSD never names it"
        );
        assert_eq!(choose(Ok("Latin"), "eng"), None);
        assert_eq!(choose(Ok("Han"), "eng+jpn"), None);
        let failed = OcrError::ProcessingFailed("script retry: OSD: boom".to_string());
        assert_eq!(choose(Err(&failed), "eng"), None);
    }

    /// The retry looks its models up in the installed directories only. With a
    /// search chain that holds `eng` alone, `osd` and `chi_sim` are absent and there is nothing
    /// to download from: the lookup returns no directory and creates none. With `osd` and
    /// `chi_sim` in the second directory, the retry's list resolves past the first pass's own
    /// directory.
    #[test]
    fn the_retry_finds_its_models_in_the_installed_directories_only() {
        let first_pass = tempfile::tempdir().unwrap();
        std::fs::write(first_pass.path().join("eng.traineddata"), b"model fixture").unwrap();
        let second = tempfile::tempdir().unwrap();
        for name in ["eng", "osd", "chi_sim"] {
            std::fs::write(second.path().join(format!("{name}.traineddata")), b"model fixture").unwrap();
        }
        let absent = tempfile::tempdir().unwrap().path().join("never-created");
        let only_eng = vec![
            absent.to_str().unwrap().to_string(),
            first_pass.path().to_str().unwrap().to_string(),
        ];
        assert_eq!(first_directory_holding(&only_eng, &["osd".to_string()]), None);
        assert_eq!(first_directory_holding(&only_eng, &retry_list("eng", "chi_sim")), None);
        assert!(!absent.exists(), "the lookup creates no directory");
        let both = vec![
            first_pass.path().to_str().unwrap().to_string(),
            second.path().to_str().unwrap().to_string(),
        ];
        assert_eq!(
            first_directory_holding(&both, &["eng".to_string()]).as_deref(),
            first_pass.path().to_str()
        );
        assert_eq!(
            first_directory_holding(&both, &retry_list("eng", "chi_sim")).as_deref(),
            second.path().to_str()
        );
        let env = TessdataEnv {
            tessdata_prefix: second.path().to_str().map(str::to_string),
            cache_dir: Some(absent.clone()),
        };
        let present = present_models("eng", Some(first_pass.path()), &env);
        assert!(present.contains(&"osd") && present.contains(&"chi_sim"), "{present:?}");
        assert!(!present_models("eng+chi_sim", Some(first_pass.path()), &env).contains(&"chi_sim"));
    }

    /// Probe: a protected token changed inside (`385.20` read as `385,20`, `Ref` as `Rcf`) is a
    /// lost token, so the line keeps its first read.
    #[test]
    fn a_protected_token_changed_inside_refuses_the_line() {
        let first = line(&[("total", 90.0, 0, 140), ("385.20", 86.0, 160, 170)], 1, 100);
        for changed in ["385,20", "385.2O", "38520"] {
            let retry = line(&[("total", 92.0, 0, 140), (changed, 92.0, 160, 170)], 1, 100);
            let merged = merge_han(first.clone(), retry);
            assert_eq!((merged.admitted, merged.refused), (0, 1), "{changed}");
            assert_eq!(merged.text(), "total 385.20");
        }
        let same_box_other_text = line(&[("Rcf", 95.0, 180, 90), ("3124", 95.0, 290, 120)], 1, 100);
        let first_ref = line(&[("Ref", 92.0, 180, 90), ("3124", 93.0, 290, 120)], 1, 100);
        assert_eq!(merge_han(first_ref, same_box_other_text).text(), "Ref 3124");
    }

    /// Protected class: a token with a Latin letter or a digit anywhere in it.
    #[test]
    fn protected_text_has_a_latin_letter_or_a_digit() {
        for text in [
            "Ref", "3124", "385.20", "A-7", "It's", "O'Brien", "50%", "a_b", "ÉTÉ", "L", "x_1",
        ] {
            assert!(is_protected_text(core(text)), "{text}");
        }
        for text in ["今天", "", "|", "~", "—"] {
            assert!(!is_protected_text(core(text)), "{text}");
        }
    }

    /// A Latin token the retry reads as other Latin or digit text, or not at
    /// all, is lost and keeps the line. `It's`, `O'Brien`, `50%` and `a_b` are protected like
    /// `Ref`. Each line has one Han word that would otherwise be admitted.
    #[test]
    fn a_token_with_an_apostrophe_a_percent_or_an_underscore_survives_a_han_retry() {
        for (token, reread) in [("It's", "Its"), ("O'Brien", "OBrien"), ("50%", "5O%"), ("a_b", "ab")] {
            let first = line(&[(token, 50.0, 0, 120), ("|", 50.0, 140, 120)], 1, 100);
            let retry = line(&[(reread, 95.0, 0, 120), ("今天天气很好我们", 95.0, 140, 120)], 1, 100);
            let merged = merge_han(first.clone(), retry);
            assert_eq!(joined(&merged), format!("{token} |"), "{token} read as {reread}");
            assert!(!merged.changed(), "{token}");
            assert_eq!(missing_protected(&first, &merged), Vec::<String>::new(), "{token}");
            let kept = line(&[(token, 50.0, 0, 120), ("|", 50.0, 140, 120)], 1, 100);
            let same = line(&[(token, 95.0, 0, 120), ("今天天气很好我们", 95.0, 140, 120)], 1, 100);
            assert_eq!(
                joined(&merge_han(kept, same)),
                format!("{token} 今天天气很好我们"),
                "{token} read again"
            );
        }
        let nothing = line(&[("It's", 50.0, 0, 120), ("|", 50.0, 140, 120)], 1, 100);
        let only_han = line(&[("今天天气很好我们", 95.0, 140, 120)], 1, 100);
        assert_eq!(joined(&merge_han(nothing, only_han)), "It's |");
    }

    /// Latin junk under a Han glyph is not a lost word, so the issue's page
    /// (every line junk) reads Han.
    #[test]
    fn latin_junk_under_a_han_read_is_replaced() {
        let first = line(&[("SRA", 35.0, 0, 200), ("TURF", 30.0, 220, 300)], 1, 100);
        let retry = line(&[("今天天气很好", 93.0, 0, 520)], 1, 100);
        let merged = merge_han(first, retry);
        assert_eq!(han_count(&joined(&merged)), 6, "{}", joined(&merged));
        assert_eq!((merged.admitted, merged.refused), (1, 0));
    }

    /// The page floor: a page takes the retry only when one third of its first-read lines is
    /// admitted. Three lines admit one (taken), a fourth line makes it one in four (refused whole,
    /// the admitted line too).
    #[test]
    fn a_page_takes_the_retry_only_with_a_third_of_its_lines_admitted() {
        let page = |lines: u32| {
            let mut first = Vec::new();
            let mut retry = Vec::new();
            for index in 0..lines {
                let top = 100 + 100 * i64::from(index);
                first.extend(line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], index, top));
                if index == 0 {
                    retry.extend(line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], index, top));
                }
            }
            merge_han(first, retry)
        };
        let taken = page(3);
        assert_eq!((taken.admitted, taken.refused), (1, 2));
        assert_eq!(han_count(&joined(&taken)), 4);
        let below = page(4);
        assert_eq!((below.admitted, below.refused, below.added), (0, 4, 0));
        assert!(!below.changed());
        assert_eq!(han_count(&joined(&below)), 0);
        assert_eq!(joined(&below), "| ~ | ~ | ~ | ~");
    }

    /// A merged page keeps the first read's block and paragraph numbers, with lines numbered within
    /// their paragraph; a retry-only line gets a block of its own.
    #[test]
    fn the_merged_table_keeps_the_first_reads_blocks_and_paragraphs() {
        let mut first = line(&[("|", 20.0, 0, 120), ("~", 25.0, 140, 160)], 1, 100);
        for word in &mut first {
            word.line_key = (2, 3, 1);
        }
        let mut second = line(&[("Total", 95.0, 0, 120)], 2, 300);
        for word in &mut second {
            word.line_key = (2, 3, 2);
        }
        first.extend(second);
        let mut retry = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 100);
        retry.extend(line(&[("Total", 95.0, 0, 120)], 2, 300));
        retry.extend(line(&[("标题", 90.0, 0, 120)], 0, 20));
        let merged = merge_han(first, retry);
        assert_eq!((merged.admitted, merged.refused, merged.added), (1, 1, 1));
        let keys: Vec<(u32, u32, u32)> = parse_words(&merged.tsv()).iter().map(|word| word.line_key).collect();
        assert_eq!(keys, [(3, 1, 1), (2, 3, 1), (2, 3, 1), (2, 3, 2)]);
    }

    /// The retry is wanted for a noisy page of a text or Markdown read, with table detection on
    /// (its default) or off, and not with no retry settings or on a page that read well.
    #[test]
    fn table_detection_is_on_by_default_and_does_not_stop_the_retry() {
        assert!(crate::ocr::types::TesseractConfig::default().enable_table_detection);
        let config = |detection: bool, format: &str, retry: bool| crate::ocr::types::TesseractConfig {
            enable_table_detection: detection,
            output_format: format.to_string(),
            script_retry: retry.then_some(crate::ocr::types::ScriptRetry { noisy_below: 75.0 }),
            ..Default::default()
        };
        assert!(wanted(&config(false, "text", true), 30.0));
        assert!(wanted(&config(false, "markdown", true), 30.0));
        assert!(
            wanted(&config(true, "text", true), 30.0),
            "table detection does not stop the retry"
        );
        assert!(!wanted(&config(false, "hocr", true), 30.0));
        assert!(!wanted(&config(false, "text", false), 30.0));
        assert!(!wanted(&config(false, "text", true), 90.0));
    }

    #[test]
    fn parse_words_keeps_level_five_rows_with_text() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   4\t1\t1\t1\t1\t0\t10\t20\t300\t40\t-1\t\n\
                   5\t1\t1\t1\t1\t1\t10\t20\t90\t40\t92\tRef\n\
                   5\t1\t1\t1\t1\t2\t120\t20\t100\t40\t93.5\t3124\n\
                   5\t1\t1\t1\t1\t3\t240\t20\t10\t40\t95\t ";
        let words = parse_words(tsv);
        assert_eq!(words.len(), 2);
        assert_eq!(
            (words[1].left, words[1].conf, words[1].text.as_str()),
            (120, 93.5, "3124")
        );
        let renumbered: Vec<TsvWord> = words
            .iter()
            .cloned()
            .map(|word| TsvWord {
                line_key: (1, 1, 1),
                ..word
            })
            .collect();
        assert_eq!(parse_words(&merge_han(words, Vec::new()).tsv()), renumbered);
    }

    #[test]
    fn the_configured_list_decides_which_models_are_candidates() {
        assert_eq!(model_for_script("Han"), Some("chi_sim"));
        assert_eq!(model_for_script("Latin"), None);
        assert!(list_reads_script_of("eng+chi_tra", "chi_sim"));
        assert!(list_reads_script_of("jpn", "chi_sim"));
        assert!(list_reads_script_of("*", "chi_sim"));
        assert!(!list_reads_script_of("eng+deu", "chi_sim"));
    }
}
