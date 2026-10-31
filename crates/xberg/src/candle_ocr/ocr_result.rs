//! Shared `ExtractedDocument` construction for candle-based OCR backends (issue #179).
//!
//! GLM-OCR, PaddleOCR-VL, TrOCR, and DeepSeek-OCR each independently returned a
//! bare `ExtractedDocument { content, formulas, mime_type, ..Default::default() }`,
//! silently dropping OCR metadata (`FormatMetadata::Ocr`, `ocr_used`, processed
//! image dimensions), `detected_languages`, and any GFM tables embedded in
//! `content` by GLM-OCR / PaddleOCR-VL. `sceptre_ocr::build_metadata` already
//! builds the metadata half of this correctly for Sceptre; this module is the
//! equivalent for the candle-based backends so the same logic isn't pasted a
//! fifth time (issue #179 explicitly calls out "a sixth copy is a regression").
//!
//! Not shared with `llm::vlm_ocr`: that backend lives behind the `liter-llm`
//! feature, a separate feature domain from `candle-*`, and pulling it in here
//! (or pulling this in there) would make VLM-only builds depend on candle-ocr
//! or vice versa. `vlm_ocr::process_image` builds its own (smaller) metadata
//! inline for that reason.

use std::borrow::Cow;
use std::sync::LazyLock;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::core::config::OcrConfig;
use crate::ocr_script::{Script, scripts_for_language};
use crate::types::{ExtractedDocument, FormatMetadata, Formula, Metadata, OcrMetadata, ProcessingWarning, Table};

// Taken from the ungated `crate::ocr_metadata_keys` rather than `crate::ocr`, which is
// gated on `feature = "ocr"` / `"ocr-wasm"` — neither of which any `candle-*` backend
// feature implies, so candle-only builds (e.g. `candle-trocr` alone) cannot see it. ~keep
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY as PROCESSED_HEIGHT_KEY;
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY as PROCESSED_WIDTH_KEY;

/// Default OCR language when none is configured, matching
/// `crate::core::config::ocr::DEFAULT_OCR_LANGUAGE`. Not reused directly for the
/// same feature-gating reason as the metadata keys above:
/// `OcrConfig::effective_languages` is only compiled under `feature = "ocr"` /
/// `"ocr-wasm"` / `paddle_ocr` / `liter-llm`. ~keep
const DEFAULT_LANGUAGE: &str = "eng";

/// Separator Tesseract uses between language codes in one string (`eng+deu`).
const TESSERACT_LANGUAGE_SEPARATOR: char = '+';

/// The languages the caller requested, with blanks dropped and the documented
/// default applied when none remain. These candle backends do not perform
/// independent language detection, so this reports the requested languages
/// rather than a detected set — matching the Sceptre backend's `languages`
/// field for the same reason.
///
/// A `+`-joined Tesseract list (`eng+deu`) is split the way `OcrConfig`'s
/// deserializer splits it: the CLI `--ocr-language` override stores the raw flag value,
/// so this is the only place that shape is normalised before the script allowlist reads it.
fn effective_languages(config: &OcrConfig) -> Vec<String> {
    let langs: Vec<String> = config
        .language
        .iter()
        .flat_map(|lang| lang.split(TESSERACT_LANGUAGE_SEPARATOR))
        .map(str::trim)
        .filter(|lang| !lang.is_empty())
        .map(str::to_string)
        .collect();
    if langs.is_empty() {
        vec![DEFAULT_LANGUAGE.to_string()]
    } else {
        langs
    }
}

/// Call-site context for [`build_ocr_document`]: everything a backend knows before OCR
/// runs, as opposed to `content`/`formulas` (what OCR produced). Grouped into one struct
/// so the function stays under poly's parameter-count limit.
pub(crate) struct OcrDocumentContext {
    /// The document mime type the backend's OCR content is encoded as (e.g.
    /// `"text/markdown"`, `"text/plain"`).
    pub(crate) mime_type: Cow<'static, str>,
    /// The same string each backend's `OcrBackend::name()` returns (e.g.
    /// `"candle-trocr"`); used only to attribute warnings.
    pub(crate) backend_name: &'static str,
    /// Whether the caller asked this backend for plain-prose OCR rather than a
    /// formula/table/chart task (GH#1676): a caller who explicitly requested LaTeX (a
    /// formula task) expects LaTeX back, so [`filter_implausible_lines`]'s bare-LaTeX
    /// rule only fires when this is `true`.
    pub(crate) plain_text_task: bool,
}

/// Build the full `ExtractedDocument` a candle OCR backend should return.
///
/// Populates `metadata` (`FormatMetadata::Ocr`, `ocr_used`, processed image
/// dimensions read from `image_bytes`), `tables` (parsed out of any GFM tables
/// present in `content`), `detected_languages` (the languages the caller
/// requested; these backends do not perform independent language detection),
/// and `processing_warnings` (see [`auto_rotate_unsupported_warning`]).
///
/// This is the single call site all four candle backends route their
/// `process_image` result through (issue #179's precedent for not pasting
/// shared OCR-result plumbing a fifth time), which makes it the one place
/// that needs to know about `auto_rotate` rather than four separate call
/// sites that could drift the way the sceptre/paddle indent check did (#861).
///
/// The tables and detected languages below are derived from the content
/// [`filter_implausible_lines`] filtered, not the raw model output, so a dropped line
/// cannot surface as a stray table row or corrupt an otherwise-clean GFM table (GH#1676).
pub(crate) fn build_ocr_document(
    content: String,
    formulas: Vec<Formula>,
    image_bytes: &[u8],
    config: &OcrConfig,
    context: OcrDocumentContext,
) -> ExtractedDocument {
    let OcrDocumentContext {
        mime_type,
        backend_name,
        plain_text_task,
    } = context;
    let (content, line_filter) = filter_implausible_lines(&content, config, plain_text_task);
    let tables = extract_gfm_tables(&content);
    let metadata = build_metadata(image_bytes, tables.len() as u32);

    let mut processing_warnings = Vec::new();
    if let Some(warning) = auto_rotate_unsupported_warning(config, backend_name) {
        processing_warnings.push(warning);
    }
    if let Some(warning) = line_filter.into_warning(backend_name) {
        processing_warnings.push(warning);
    }

    ExtractedDocument {
        content,
        formulas,
        mime_type,
        metadata,
        tables,
        detected_languages: Some(effective_languages(config)),
        processing_warnings,
        ..Default::default()
    }
}

/// Warn once per extraction when `auto_rotate` was requested of a candle
/// backend that has no orientation-detection step at all (#861).
///
/// Unlike Tesseract's `auto_rotate_unavailable` (gated on whether the
/// `auto-rotate` *build feature* is compiled in) and PaddleOCR's own
/// `config.auto_rotate` handling in `paddle_ocr::backend`, none of the four
/// candle backends (TrOCR, PaddleOCR-VL, GLM-OCR, DeepSeek-OCR) read
/// `OcrConfig::auto_rotate` at all, in any build — there is no feature flag
/// that would make them honour it. So this always fires when the flag is
/// set, rather than being conditioned on `cfg!(auto_rotate)` the way
/// Tesseract's is.
///
/// A warning rather than a hard error, matching the precedent at
/// `ocr::processor::execution::is_auto_rotate_requested_but_unavailable`
/// (#309): the caller asked for orientation correction that silently will
/// not happen, but the rest of the extraction is otherwise sound, so failing
/// the whole call would be a worse outcome than telling them.
fn auto_rotate_unsupported_warning(config: &OcrConfig, backend_name: &'static str) -> Option<ProcessingWarning> {
    if !config.auto_rotate {
        return None;
    }
    Some(crate::core::diagnostics::warning(
        backend_name,
        format!(
            "auto_rotate was requested but the `{backend_name}` backend has no orientation \
             detection or correction step; the image was OCR'd in its original orientation"
        ),
    ))
}

/// Build OCR metadata: `FormatMetadata::Ocr` with the table count, `ocr_used`,
/// and processed image dimensions (best-effort; a decode failure just omits
/// the dimensions rather than failing the whole OCR call, since dimensions
/// are diagnostic, not load-bearing).
fn build_metadata(image_bytes: &[u8], table_count: u32) -> Metadata {
    let mut metadata = Metadata {
        format: Some(FormatMetadata::Ocr(OcrMetadata {
            table_count,
            ..Default::default()
        })),
        ocr_used: true,
        ..Default::default()
    };

    if let Some((width, height)) = probe_image_dimensions(image_bytes) {
        metadata
            .additional
            .insert(Cow::Borrowed(PROCESSED_WIDTH_KEY), serde_json::json!(width));
        metadata
            .additional
            .insert(Cow::Borrowed(PROCESSED_HEIGHT_KEY), serde_json::json!(height));
    }

    metadata
}

/// Read image dimensions from the header without decoding pixel data.
fn probe_image_dimensions(image_bytes: &[u8]) -> Option<(u32, u32)> {
    crate::extraction::image_decode::probe_standard_image_with_default_security_limits(image_bytes)
        .ok()
        .map(|(width, height, _)| (width, height))
}

/// Parse every GFM table in `content` into structured [`Table`] entries.
///
/// GLM-OCR and PaddleOCR-VL emit GFM tables directly in their markdown output;
/// without this, `tables[]` stayed empty even though the table data was
/// present in `content`. The markdown rendering of each parsed table reuses
/// [`crate::rendering::common::render_table_markdown`] (the same renderer
/// `extractors/markdown.rs` uses for tables) rather than re-serializing ad hoc,
/// so the round-tripped markdown matches the rest of the codebase's table
/// formatting.
fn extract_gfm_tables(content: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut in_table = false;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut in_cell = false;

    for event in Parser::new_ext(content, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                rows.clear();
            }
            Event::End(TagEnd::Table) if in_table => {
                in_table = false;
                if !rows.is_empty() {
                    let cells = std::mem::take(&mut rows);
                    let markdown = crate::rendering::common::render_table_markdown(&cells);
                    tables.push(Table {
                        cells,
                        markdown,
                        page_number: 1,
                        ..Default::default()
                    });
                }
            }
            Event::Start(Tag::TableHead | Tag::TableRow) if in_table => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) if in_table && !current_row.is_empty() => {
                rows.push(std::mem::take(&mut current_row));
            }
            Event::Start(Tag::TableCell) if in_table => {
                in_cell = true;
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) if in_table => {
                in_cell = false;
                current_row.push(current_cell.trim().to_string());
                current_cell.clear();
            }
            Event::Text(text) | Event::Code(text) if in_table && in_cell => {
                current_cell.push_str(&text);
            }
            _ => {}
        }
    }

    tables
}

/// Minimum number of disallowed-script letters a line must contain before rule (a) in
/// [`filter_implausible_lines`] considers dropping it (GH#1676).
const IMPLAUSIBLE_SCRIPT_MIN_LETTERS: usize = 3;

/// Minimum fraction of a line's script-classified letters that must fall in a disallowed
/// script before rule (a) in [`filter_implausible_lines`] drops it (GH#1676).
const IMPLAUSIBLE_SCRIPT_MIN_FRACTION: f64 = 0.5;

/// Minimum number of `\command`-shaped tokens a line must contain, with no bare
/// alphabetic word present, before rule (b) in [`filter_implausible_lines`] drops it on a
/// plain-text OCR task (GH#1676).
const LATEX_LINE_MIN_COMMANDS: usize = 2;

/// Classify an already-known-alphabetic character into a [`Script`] by code-point range.
///
/// Returns `None` for a letter outside every range covered here (e.g. Armenian,
/// Ethiopic): such letters are neither counted as evidence of a disallowed script nor as
/// evidence of an allowed one, so they cannot influence rule (a) either way.
fn classify_char(ch: char) -> Option<Script> {
    match ch as u32 {
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F => Some(Script::Latin),
        0x0370..=0x03FF => Some(Script::Greek),
        0x0400..=0x04FF => Some(Script::Cyrillic),
        0x0590..=0x05FF => Some(Script::Hebrew),
        0x0600..=0x06FF | 0x0750..=0x077F => Some(Script::Arabic),
        0x0900..=0x097F => Some(Script::Devanagari),
        0x0E00..=0x0E7F => Some(Script::Thai),
        0x3040..=0x30FF => Some(Script::Kana),
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => Some(Script::Han),
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7A3 => Some(Script::Hangul),
        _ => None,
    }
}

/// Build the script allowlist for [`filter_implausible_lines`] from the configured OCR
/// language list, or `None` when any configured language is unrecognized (which disables
/// rule (a) entirely rather than risk false positives against a language it cannot map).
///
/// This deliberately reads [`effective_languages`] (the same resolution `detected_languages`
/// already uses in [`build_ocr_document`]) rather than `config.tesseract_config.language`:
/// none of the four candle backends consult `tesseract_config` for anything (it is a
/// Tesseract-only field), so the language list that actually reached the model is the one
/// `effective_languages` reports. Latin is always allowed regardless of the configured
/// languages: Latin loanwords, product names, and numerals routinely appear even in
/// non-Latin-script documents, and disallowing Latin would make rule (a) noisy rather than
/// conservative.
fn allowed_scripts(config: &OcrConfig) -> Option<Vec<Script>> {
    let mut scripts = vec![Script::Latin];
    for lang in effective_languages(config) {
        let scripts_for_lang = scripts_for_language(&lang)?;
        for script in scripts_for_lang {
            if !scripts.contains(script) {
                scripts.push(*script);
            }
        }
    }
    Some(scripts)
}

/// Count a line's script-classified letters, split into those in a disallowed script and
/// the total classified (returns `(disallowed_count, classified_count)`).
fn line_script_counts(line: &str, allowed: &[Script]) -> (usize, usize) {
    let mut disallowed = 0usize;
    let mut classified = 0usize;
    for ch in line.chars().filter(|ch| ch.is_alphabetic()) {
        if let Some(script) = classify_char(ch) {
            classified += 1;
            if !allowed.contains(&script) {
                disallowed += 1;
            }
        }
    }
    (disallowed, classified)
}

/// Rule (a): a line is an implausible script when at least
/// [`IMPLAUSIBLE_SCRIPT_MIN_LETTERS`] of its script-classified letters are in a script the
/// configured OCR language list does not allow, and those disallowed letters make up at
/// least [`IMPLAUSIBLE_SCRIPT_MIN_FRACTION`] of the line's classified letters.
fn line_has_implausible_script(line: &str, allowed: &[Script]) -> bool {
    let (disallowed, classified) = line_script_counts(line, allowed);
    if classified == 0 || disallowed < IMPLAUSIBLE_SCRIPT_MIN_LETTERS {
        return false;
    }
    (disallowed as f64 / classified as f64) >= IMPLAUSIBLE_SCRIPT_MIN_FRACTION
}

/// Matches a LaTeX-command-shaped token (`\` followed by one or more letters), e.g.
/// `\alpha` or `\frac`. Used only by rule (b) in [`filter_implausible_lines`]. ~keep
static LATEX_COMMAND_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[A-Za-z]+").expect("static LaTeX command pattern is valid"));

/// A token made only of alphabetic characters (a plain word, not a LaTeX command and not
/// punctuation/digits) — evidence that a line is prose rather than bare markup.
fn is_bare_alphabetic_word(word: &str) -> bool {
    !word.is_empty() && word.chars().all(char::is_alphabetic)
}

/// Rule (b): on a plain-text OCR task, a line with at least [`LATEX_LINE_MIN_COMMANDS`]
/// LaTeX-command-shaped tokens and no bare alphabetic word anywhere in it reads as raw
/// LaTeX markup rather than the prose the caller asked for.
///
/// The "no bare word" guard exists so a genuinely prose line that happens to mention a
/// command name in passing is not dropped: it only fires when the *entire* line is
/// markup, not merely contains some.
fn line_is_bare_latex(line: &str) -> bool {
    let command_count = LATEX_COMMAND_PATTERN.find_iter(line).count();
    if command_count < LATEX_LINE_MIN_COMMANDS {
        return false;
    }
    !line.split_whitespace().any(is_bare_alphabetic_word)
}

/// A line consisting only of a `$$` math-fence delimiter or a ``` code-fence delimiter.
/// Lines between two such delimiters are exempt from rule (b): a genuine LaTeX block the
/// model deliberately fenced off is not "bare" markup, it is the requested output.
fn is_fence_boundary(trimmed_line: &str) -> bool {
    trimmed_line == "$$" || trimmed_line.starts_with("```")
}

/// How many lines [`filter_implausible_lines`] dropped, by rule.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplausibleLineFilter {
    dropped_script_lines: usize,
    dropped_latex_lines: usize,
}

impl ImplausibleLineFilter {
    fn is_empty(&self) -> bool {
        self.dropped_script_lines == 0 && self.dropped_latex_lines == 0
    }

    /// One combined warning naming both counts, or `None` when nothing was dropped.
    fn into_warning(self, backend_name: &'static str) -> Option<ProcessingWarning> {
        if self.is_empty() {
            return None;
        }
        Some(crate::core::diagnostics::warning(
            backend_name,
            format!(
                "dropped {} line(s) with a script implausible for the configured OCR language(s) \
                 and {} line(s) of bare LaTeX markup on a plain-text OCR task",
                self.dropped_script_lines, self.dropped_latex_lines
            ),
        ))
    }
}

/// Drop OCR output lines that read as noise rather than recognized text (GH#1676): a CJK
/// run in an English-only document (rule (a), script implausibility) or bare LaTeX markup
/// on a task that asked for plain prose (rule (b)). Conservative by design — both rules
/// require multiple corroborating signals before dropping a line, and a `$$`/``` fenced
/// block is exempt from rule (b) so a legitimately requested formula is never touched.
///
/// Returns the filtered content (lines rejoined with `\n`) and the drop counts.
pub(crate) fn filter_implausible_lines(
    content: &str,
    config: &OcrConfig,
    plain_text_task: bool,
) -> (String, ImplausibleLineFilter) {
    let allowed_scripts = allowed_scripts(config);
    let mut outcome = ImplausibleLineFilter::default();
    let mut in_fence = false;
    let mut kept_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if is_fence_boundary(trimmed) {
            in_fence = !in_fence;
            kept_lines.push(line);
            continue;
        }

        if let Some(allowed) = &allowed_scripts
            && line_has_implausible_script(line, allowed)
        {
            outcome.dropped_script_lines += 1;
            continue;
        }

        if plain_text_task && !in_fence && line_is_bare_latex(line) {
            outcome.dropped_latex_lines += 1;
            continue;
        }

        kept_lines.push(line);
    }

    (kept_lines.join("\n"), outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_extract_no_tables_from_plain_text() {
        let tables = extract_gfm_tables("Just some OCR'd prose with no tables.");
        assert!(tables.is_empty(), "expected no tables; got: {tables:?}");
    }

    #[test]
    fn should_extract_single_gfm_table_from_content() {
        let content = "Some text before.\n\n\
            | Name | Age |\n\
            |------|-----|\n\
            | Alice | 30 |\n\
            | Bob | 25 |\n\n\
            Some text after.";

        let tables = extract_gfm_tables(content);

        assert_eq!(tables.len(), 1, "expected exactly one table; got: {tables:?}");
        assert_eq!(
            tables[0].cells,
            vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
                vec!["Bob".to_string(), "25".to_string()],
            ]
        );
        assert_eq!(tables[0].page_number, 1);
        assert!(tables[0].markdown.contains("Name"));
        assert!(tables[0].markdown.contains("Alice"));
    }

    #[test]
    fn should_extract_multiple_gfm_tables_from_content() {
        let content = "| A |\n|---|\n| 1 |\n\ntext between\n\n| B |\n|---|\n| 2 |";

        let tables = extract_gfm_tables(content);

        assert_eq!(tables.len(), 2, "expected two tables; got: {tables:?}");
        assert_eq!(tables[0].cells, vec![vec!["A".to_string()], vec!["1".to_string()]]);
        assert_eq!(tables[1].cells, vec![vec!["B".to_string()], vec!["2".to_string()]]);
    }

    #[test]
    fn should_build_ocr_document_with_metadata_tables_and_languages() {
        // 1x1 PNG (smallest valid PNG payload), used only to exercise the
        // dimension probe without depending on network or real OCR output.
        let png_1x1: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
            0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x18,
            0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let content = "| H |\n|---|\n| v |".to_string();
        let config = OcrConfig {
            language: vec!["deu".to_string()],
            ..Default::default()
        };

        let doc = build_ocr_document(
            content.clone(),
            Vec::new(),
            png_1x1,
            &config,
            OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-trocr",
                plain_text_task: true,
            },
        );

        assert_eq!(doc.content, content);
        assert!(doc.metadata.ocr_used);
        let Some(FormatMetadata::Ocr(ocr_metadata)) = &doc.metadata.format else {
            panic!("expected FormatMetadata::Ocr; got: {:?}", doc.metadata.format);
        };
        assert_eq!(ocr_metadata.table_count, 1);
        assert_eq!(
            doc.metadata.additional.get(PROCESSED_WIDTH_KEY),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            doc.metadata.additional.get(PROCESSED_HEIGHT_KEY),
            Some(&serde_json::json!(1))
        );
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.detected_languages, Some(vec!["deu".to_string()]));
        assert!(
            doc.processing_warnings.is_empty(),
            "auto_rotate defaults to false; no warning should be emitted, got: {:?}",
            doc.processing_warnings
        );
    }

    /// #861: none of the four candle OCR backends (TrOCR, PaddleOCR-VL,
    /// GLM-OCR, DeepSeek-OCR) implement orientation detection, so a caller
    /// who sets `auto_rotate: true` must be told the request was silently
    /// unactionable rather than getting no signal at all. Before this fix,
    /// `build_ocr_document` never inspected `config.auto_rotate` and always
    /// returned an empty `processing_warnings` (via `..Default::default()`),
    /// so this assertion fails against the unfixed code with:
    /// `assertion failed: !doc.processing_warnings.is_empty()`.
    #[test]
    fn should_warn_when_auto_rotate_requested_of_a_backend_with_no_rotation_support() {
        let config = OcrConfig {
            auto_rotate: true,
            ..Default::default()
        };

        let doc = build_ocr_document(
            "text".to_string(),
            Vec::new(),
            &[],
            &config,
            OcrDocumentContext {
                mime_type: Cow::Borrowed("text/plain"),
                backend_name: "candle-trocr",
                plain_text_task: true,
            },
        );

        assert_eq!(
            doc.processing_warnings.len(),
            1,
            "expected exactly one auto_rotate warning, got: {:?}",
            doc.processing_warnings
        );
        assert_eq!(doc.processing_warnings[0].source, "candle-trocr");
        assert_eq!(
            doc.processing_warnings[0].message,
            "auto_rotate was requested but the `candle-trocr` backend has no orientation \
             detection or correction step; the image was OCR'd in its original orientation"
        );
    }

    /// The warning must fire exactly once per extraction (not once per page,
    /// once per region, or duplicated), matching the single call site inside
    /// `build_ocr_document` this test exercises directly.
    #[test]
    fn should_not_warn_when_auto_rotate_is_not_requested() {
        let config = OcrConfig::default();
        assert!(
            !config.auto_rotate,
            "test assumes the OcrConfig default is auto_rotate: false"
        );

        let doc = build_ocr_document(
            "text".to_string(),
            Vec::new(),
            &[],
            &config,
            OcrDocumentContext {
                mime_type: Cow::Borrowed("text/plain"),
                backend_name: "candle-glm-ocr",
                plain_text_task: true,
            },
        );

        assert!(
            doc.processing_warnings.is_empty(),
            "auto_rotate: false must not produce a warning, got: {:?}",
            doc.processing_warnings
        );
    }

    /// GH#1676: a CJK run in an English-only document is implausible and is dropped, with
    /// a warning naming the count.
    #[test]
    fn should_drop_cjk_line_under_english_only_language_config() {
        let config = OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };
        let content = "Printed heading.\n这是一个测试\nMore printed prose.".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(
            filtered, "Printed heading.\nMore printed prose.",
            "expected the CJK line to be dropped, got: {filtered:?}"
        );
        assert_eq!(outcome.dropped_script_lines, 1);
        assert_eq!(outcome.dropped_latex_lines, 0);
        let warning = outcome
            .into_warning("candle-paddleocr-vl")
            .expect("a dropped line must produce a warning");
        assert_eq!(warning.source, "candle-paddleocr-vl");
        assert!(
            warning.message.contains('1'),
            "warning should name the dropped-line count, got: {}",
            warning.message
        );
    }

    /// GH#1676: the same CJK line is plausible once Japanese (which is written with Han
    /// characters as well as kana) is in the configured language list.
    #[test]
    fn should_keep_cjk_line_when_japanese_is_configured() {
        let config = OcrConfig {
            language: vec!["eng".to_string(), "jpn".to_string()],
            ..Default::default()
        };
        let content = "这是一个测试".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_script_lines, 0);
    }

    /// GH#1676: a `+`-joined Tesseract list (the CLI `--ocr-language` shape) is split before
    /// the script allowlist is built, so `eng+deu` still drops a CJK line instead of being
    /// treated as one unknown code that disables rule (a).
    #[test]
    fn should_split_plus_joined_language_list_before_building_the_allowlist() {
        let config = OcrConfig {
            language: vec!["eng+deu".to_string()],
            ..Default::default()
        };
        let content = "这是一个测试".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, "");
        assert_eq!(outcome.dropped_script_lines, 1);
        assert_eq!(effective_languages(&config), vec!["eng".to_string(), "deu".to_string()]);
    }

    /// GH#1676: a Cyrillic run is implausible under an English-only config.
    #[test]
    fn should_drop_cyrillic_line_under_english_only_language_config() {
        let config = OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };
        let content = "Привет мир".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, "");
        assert_eq!(outcome.dropped_script_lines, 1);
    }

    /// GH#1676: the same Cyrillic run is plausible once Russian is configured.
    #[test]
    fn should_keep_cyrillic_line_when_russian_is_configured() {
        let config = OcrConfig {
            language: vec!["rus".to_string()],
            ..Default::default()
        };
        let content = "Привет мир".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_script_lines, 0);
    }

    /// GH#1676: an unrecognized language code disables rule (a) entirely rather than
    /// risk a false positive against a language the classifier cannot map.
    #[test]
    fn should_keep_implausible_script_line_when_language_code_is_unrecognized() {
        let config = OcrConfig {
            language: vec!["xx-unrecognized".to_string()],
            ..Default::default()
        };
        let content = "这是一个测试".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content, "rule (a) must be disabled for an unmapped language");
        assert_eq!(outcome.dropped_script_lines, 0);
    }

    /// GH#1676: German umlauts are Latin-script and must never be flagged, regardless of
    /// the configured language.
    #[test]
    fn should_keep_line_with_umlauts() {
        let config = OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };
        let content = "Über die Brücke gehen wir jetzt äöüß".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_script_lines, 0);
    }

    /// GH#1676: bare LaTeX markup on a plain-text OCR task reads as noise, not
    /// recognized prose, and is dropped.
    #[test]
    fn should_drop_bare_latex_line_on_plain_text_task() {
        let config = OcrConfig::default();
        let content = "\\alpha \\beta \\gamma".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, "");
        assert_eq!(outcome.dropped_latex_lines, 1);
    }

    /// GH#1676: the same LaTeX line is expected, not noise, on a formula task.
    #[test]
    fn should_keep_bare_latex_line_on_formula_task() {
        let config = OcrConfig::default();
        let content = "\\alpha \\beta \\gamma".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, false);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_latex_lines, 0);
    }

    /// GH#1676: a `$$`-fenced math block is a deliberately requested formula, not noise,
    /// even on a plain-text task.
    #[test]
    fn should_keep_latex_line_inside_dollar_fence_on_plain_text_task() {
        let config = OcrConfig::default();
        let content = "$$\n\\alpha \\beta \\gamma\n$$".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_latex_lines, 0);
    }

    /// GH#1676: a currency amount has no backslash commands at all and must never be
    /// treated as LaTeX markup.
    #[test]
    fn should_keep_currency_amount_line() {
        let config = OcrConfig::default();
        let content = "Total due: $100.00".to_string();

        let (filtered, outcome) = filter_implausible_lines(&content, &config, true);

        assert_eq!(filtered, content);
        assert_eq!(outcome.dropped_latex_lines, 0);
    }

    /// GH#1676: `build_ocr_document` parses tables from the *filtered* content, so a
    /// script-implausible pseudo-table above a real one cannot inflate the table count or
    /// corrupt the real table's header.
    #[test]
    fn should_count_tables_from_filtered_content_not_raw_content() {
        let raw_content = "这是|一个测试\n----|----\n乱码|数据\n\nSome real prose.\n\n\
            | Name | Age |\n|------|-----|\n| Alice | 30 |"
            .to_string();

        // Control: without filtering, pulldown-cmark parses the CJK block as its own
        // (implausible) table in addition to the real one.
        let raw_tables = extract_gfm_tables(&raw_content);
        assert_eq!(
            raw_tables.len(),
            2,
            "fixture must contain two GFM tables before filtering, got: {raw_tables:?}"
        );

        let config = OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };
        let doc = build_ocr_document(
            raw_content,
            Vec::new(),
            &[],
            &config,
            OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-paddleocr-vl",
                plain_text_task: true,
            },
        );

        assert_eq!(doc.tables.len(), 1, "expected only the real table to survive filtering");
        assert_eq!(
            doc.tables[0].cells[0],
            vec!["Name".to_string(), "Age".to_string()],
            "the real table's header must be intact, got: {:?}",
            doc.tables[0].cells
        );
        let Some(FormatMetadata::Ocr(ocr_metadata)) = &doc.metadata.format else {
            panic!("expected FormatMetadata::Ocr; got: {:?}", doc.metadata.format);
        };
        assert_eq!(ocr_metadata.table_count, 1);
    }
}
