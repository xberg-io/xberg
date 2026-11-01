//! Second read for a scanned page whose text runs sideways in the raster (GH#2314).
//!
//! A page that is one full-page raster is read with sparse-text segmentation (psm 11), which
//! keeps the table values that automatic layout loses on upright scans (GH#1786). Sparse-text
//! mode also drops most words of a page whose lines run sideways. Such a page carries the
//! low-confidence noise verdict, so that verdict triggers one more read with automatic layout.
//! In a pipeline the second read runs after the last stage, on a page whose returned read still
//! carries the verdict: a later stage that reads the page clean keeps it whole. The second read
//! replaces the returned read only when it scores strictly higher, reads at least as many words,
//! and its word boxes are mostly taller than wide. The word floor keeps a page whose returned read
//! holds real words; the box shape keeps upright pages. Every paragraph of the returned read that
//! the second read leaves blank joins the kept read, and so does each dictionary word or number
//! of its paragraphs that the kept read does not hold. The tables of the returned read stay, and
//! each table its text prints joins the text of the kept read.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ahash::AHashMap;
use parking_lot::Mutex;

use super::scoring::{OcrPageNoiseVerdict, compute_quality_score, mean_text_conf_of, page_carries_noise_verdict};
use crate::core::config::{OcrConfig, OcrQualityThresholds};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::geometry_bounds;
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElementLevel};

/// A word of the first read with no digit joins the kept read from this many characters.
const MIN_CARRIED_WORD_CHARS: usize = 3;

/// What the second read of one page needs after the stages have run.
pub(crate) struct SecondRead {
    backend: Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: Arc<Vec<u8>>,
    retry_config: OcrConfig,
    element_selection: Option<crate::types::OcrElementConfig>,
}

/// The pages of a pipeline run that can take a second read, by document page.
pub(crate) type SecondReads = Arc<Mutex<AHashMap<usize, SecondRead>>>;

/// The reads of one stage of a pipeline run, by document page. The stage runs with no second read
/// and records each read. A kept second read replaces the read of its page, and the stage then
/// runs once more over the recorded reads, so that the kept read passes through the page assembly.
pub(crate) struct StageReads {
    reads: Mutex<AHashMap<usize, ExtractedDocument>>,
    second_reads: SecondReads,
    replaying: AtomicBool,
}

impl StageReads {
    pub(crate) fn new(second_reads: SecondReads) -> Self {
        Self {
            reads: Mutex::new(AHashMap::new()),
            second_reads,
            replaying: AtomicBool::new(false),
        }
    }

    fn recorded_read(&self, page: usize) -> Option<ExtractedDocument> {
        self.replaying
            .load(Ordering::Acquire)
            .then(|| self.reads.lock().remove(&page))
            .flatten()
    }
}

/// Read one whole-page raster: the base read of the image extractor, then the sideways retry. With
/// `stage_page` the page belongs to a stage of a pipeline run, and the retry waits for the run.
// ~keep Each parameter is a separate input of the read, and all callers name them individually.
#[allow(clippy::too_many_arguments)]
pub(super) async fn read_whole_page_raster(
    backend: &Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: Arc<Vec<u8>>,
    original_config: &OcrConfig,
    effective_config: &OcrConfig,
    whole_page_raster: bool,
    thresholds: &OcrQualityThresholds,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
    stage_page: Option<(&StageReads, usize)>,
) -> crate::Result<ExtractedDocument> {
    if let Some(read) = stage_page.and_then(|(reads, page)| reads.recorded_read(page)) {
        return Ok(read);
    }
    let primary = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
        backend,
        Arc::clone(&image_bytes),
        original_config,
        effective_config,
        crate::extractors::image::SparseRetryPolicy {
            whole_page_raster,
            allow_standard_retry: false,
        },
        cancel_token,
    )
    .await?;
    // ~keep The carry needs the paragraphs of both reads. Only the markdown read builds them; with
    // any other output the first read stays, so no text of it can be lost.
    let second_read = (whole_page_raster
        && crate::extractors::image::is_implicit_horizontal_tesseract(original_config)
        && !thresholds.discard_suspected_ocr_noise
        && primary.ocr_internal_document.is_some()
        && carries_noise_verdict(&primary, backend, thresholds))
    .then(|| SecondRead {
        backend: Arc::clone(backend),
        image_bytes,
        retry_config: automatic_layout_config(effective_config),
        element_selection: original_config.element_config.clone(),
    });
    if let Some((reads, page)) = stage_page {
        reads.reads.lock().insert(page, primary.clone());
        if let Some(second_read) = second_read {
            reads.second_reads.lock().insert(page, second_read);
        }
        return Ok(primary);
    }
    let Some(second_read) = second_read else {
        return Ok(primary);
    };
    Ok(second_read
        .read_against(&primary, thresholds, cancel_token)
        .await?
        .unwrap_or(primary))
}

impl SecondRead {
    /// Read the page again. Returns the new read if it replaces `returned`, the read the run returned.
    async fn read_against(
        self,
        returned: &ExtractedDocument,
        thresholds: &OcrQualityThresholds,
        cancel_token: Option<&crate::cancellation::CancellationToken>,
    ) -> crate::Result<Option<ExtractedDocument>> {
        let Some(first) = returned.ocr_internal_document.as_ref() else {
            return Ok(None);
        };
        if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            return Err(crate::XbergError::Cancelled);
        }
        // ~keep The backend never sees the caller's token, so a cancel that arrives during the retry
        // is seen here, before the result is selected or an ordinary error is turned into the first read.
        let retry_result = self
            .backend
            .process_image_owned(self.image_bytes, &self.retry_config)
            .await;
        if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            return Err(crate::XbergError::Cancelled);
        }
        let mut retry = match retry_result {
            Ok(retry) => retry,
            Err(error @ crate::XbergError::Cancelled) => return Err(error),
            Err(error) => {
                tracing::warn!(%error, "sideways-page OCR retry failed; keeping the returned read");
                return Ok(None);
            }
        };
        if compute_quality_score(&retry.content, thresholds) <= compute_quality_score(&returned.content, thresholds)
            || retry.ocr_internal_document.is_none()
            || count_words(&retry.content) < count_words(&returned.content)
            || carries_noise_verdict(&retry, &self.backend, thresholds)
            || !words_run_sideways(&retry)
        {
            return Ok(None);
        }
        carry_text_the_retry_missed(first, &mut retry);
        let words = dictionary_words_the_retry_missed(first, &retry, &self.retry_config).await;
        carry_words(first, &mut retry, words);
        carry_tables(returned, &mut retry);
        retry.ocr_elements = match (self.element_selection.as_ref(), retry.ocr_elements.take()) {
            (Some(selection), Some(elements)) if selection.include_elements => {
                Some(selection.select_elements(&elements))
            }
            _ => None,
        };
        crate::extractors::image::merge_primary_warnings_into_selected(returned, &mut retry);
        Ok(Some(retry))
    }
}

/// Run the second read of each page of the returned stage that still carries the noise verdict.
/// True when a second read replaced a read: the stage must then run over its recorded reads.
pub(super) async fn run_second_reads(
    reads: &StageReads,
    verdicts: &[OcrPageNoiseVerdict],
    thresholds: &OcrQualityThresholds,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<bool> {
    let mut pending = Vec::new();
    for verdict in verdicts {
        let second_read = reads.second_reads.lock().remove(&verdict.page_index);
        let returned = reads.reads.lock().get(&verdict.page_index).cloned();
        if let (Some(second_read), Some(returned)) = (second_read, returned) {
            pending.push((verdict.page_index, second_read, returned));
        }
    }
    let mut kept = Vec::new();
    #[cfg(feature = "tokio-runtime")]
    {
        let mut tasks = tokio::task::JoinSet::new();
        for (page, second_read, returned) in pending {
            let thresholds = thresholds.clone();
            let cancel_token = cancel_token.cloned();
            tasks.spawn(crate::engine::seams::inherit_progress(async move {
                let read = second_read
                    .read_against(&returned, &thresholds, cancel_token.as_ref())
                    .await;
                (page, read)
            }));
        }
        while let Some(joined) = tasks.join_next().await {
            let (page, read) = joined.map_err(|error| crate::XbergError::Plugin {
                message: format!("OCR task panicked: {error}"),
                plugin_name: "ocr".to_string(),
            })?;
            kept.push((page, read?));
        }
    }
    #[cfg(not(feature = "tokio-runtime"))]
    for (page, second_read, returned) in pending {
        kept.push((
            page,
            second_read.read_against(&returned, thresholds, cancel_token).await?,
        ));
    }
    let mut replaced = false;
    for (page, read) in kept {
        if let Some(read) = read {
            reads.reads.lock().insert(page, read);
            replaced = true;
        }
    }
    reads.replaying.store(replaced, Ordering::Release);
    Ok(replaced)
}

/// The configuration of the second read.
fn automatic_layout_config(effective_config: &OcrConfig) -> OcrConfig {
    // ~keep The decision reads private, unfiltered word geometry whatever the caller selected;
    // the caller's own element selection is restored on the kept read.
    let mut retry_config = crate::extractors::image::sparse_image_ocr_fallback_config(effective_config);
    // ~keep A caller-set preprocessing stays, so both reads share one pixel frame and the boxes the
    // carry compares and keeps mean the same place.
    if let Some(preprocessing) = effective_config
        .tesseract_config
        .as_ref()
        .and_then(|tesseract| tesseract.preprocessing.clone())
    {
        retry_config.tesseract_config.get_or_insert_default().preprocessing = Some(preprocessing);
    }
    retry_config.element_config = Some(crate::types::OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        min_confidence: 0.0,
        build_hierarchy: false,
    });
    retry_config
}

/// ~keep Automatic layout can miss a short text that sparse-text mode reads, such as a page
/// number or a heading set in another direction. A first-read paragraph with text and no word of
/// the kept read on its area joins the kept read and its content. Only `carry_words` takes text
/// from under a kept word. The carried paragraph adds no element to the OCR elements of the kept
/// read. Its block id is dropped, because the block ids of the two reads are unrelated.
fn carry_text_the_retry_missed(first: &InternalDocument, retry: &mut ExtractedDocument) {
    let retry_words: Vec<_> = retry
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word)
        .filter_map(|element| geometry_bounds(&element.geometry))
        .collect();
    for paragraph in first.elements.iter().filter(|element| holds_read_text(element)) {
        let Some(area) = paragraph.ocr_geometry.as_ref().and_then(geometry_bounds) else {
            continue;
        };
        if !retry_words.iter().any(|word| overlaps(area, *word)) {
            push_carried(retry, paragraph, paragraph.text.trim());
        }
    }
}

fn holds_read_text(element: &InternalElement) -> bool {
    matches!(element.kind, ElementKind::OcrText { .. }) && !element.text.trim().is_empty()
}

/// Add `text` of the first-read `paragraph` to the kept read and its content.
fn push_carried(retry: &mut ExtractedDocument, paragraph: &InternalElement, text: &str) {
    let Some(kept) = retry.ocr_internal_document.as_mut() else {
        return;
    };
    let mut carried = paragraph.clone();
    carried.text = text.to_string();
    #[cfg(feature = "ocr")]
    if let Some(attributes) = carried.attributes.as_mut() {
        attributes.remove(crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE);
    }
    let index = kept.elements.len() as u32;
    kept.push_element(carried.with_index(index));
    push_block(&mut retry.content, text);
}

fn push_block(content: &mut String, block: &str) {
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    content.push_str(block);
}

/// ~keep The kept read holds each table of the returned read. Page text comes from the text of a
/// read, not its paragraphs, so a table that text prints joins the kept text too. Tables join after
/// the words, because the count of a word in the kept text decides whether a paragraph's copy joins,
/// and the first read holds that copy beside the table.
fn carry_tables(returned: &ExtractedDocument, retry: &mut ExtractedDocument) {
    for table in &returned.tables {
        let markdown = table.markdown.trim();
        if !markdown.is_empty() && returned.content.contains(markdown) {
            push_block(&mut retry.content, markdown);
        }
    }
    retry.tables.extend(returned.tables.iter().cloned());
}

/// True when a token holds a numeric character of any script.
fn holds_digit(token: &str) -> bool {
    token.chars().any(char::is_numeric)
}

/// The words of a text, without the marks at their ends. A token between white space that holds a
/// digit is one word, so a date, a time or a reference stays whole. Any other token gives its runs
/// of letters, with the marks a name holds inside.
fn words_of(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .flat_map(|token| {
            let whole = holds_digit(token);
            token.split(move |character: char| !whole && !character.is_alphanumeric() && !"$,.%'-".contains(character))
        })
        .map(|token| token.trim_matches(|character: char| !character.is_alphanumeric()))
        .filter(|word| !word.is_empty())
}

/// A word of the first read that the kept read does not hold: the index of its paragraph, the word,
/// and whether it holds a digit.
type MissedWord<'a> = (usize, &'a str, bool);

/// The numbers, and the words of `MIN_CARRIED_WORD_CHARS` or more, that a paragraph of the first read
/// holds and the kept read does not. A word counts once. A number counts each time, because a column
/// of equal totals holds the same number on each row.
fn words_the_retry_missed<'a>(first: &'a InternalDocument, retry: &ExtractedDocument) -> Vec<MissedWord<'a>> {
    let mut held: AHashMap<String, usize> = AHashMap::new();
    for word in words_of(&retry.content) {
        *held.entry(word.to_lowercase()).or_default() += 1;
    }
    let mut missed = Vec::new();
    for (index, paragraph) in first.elements.iter().enumerate() {
        if !holds_read_text(paragraph) {
            continue;
        }
        for word in words_of(&paragraph.text) {
            let number = holds_digit(word);
            let count = held.entry(word.to_lowercase()).or_default();
            if *count > 0 {
                *count -= usize::from(number);
            } else if number || word.chars().count() >= MIN_CARRIED_WORD_CHARS {
                *count = usize::from(!number);
                missed.push((index, word, number));
            }
        }
    }
    missed
}

/// The missed words that join the kept read: each word with a digit, and each word the Tesseract
/// dictionary of the read does not reject. A word with no verdict joins: a lost word is worse.
async fn dictionary_words_the_retry_missed<'a>(
    first: &'a InternalDocument,
    retry: &ExtractedDocument,
    config: &OcrConfig,
) -> Vec<(usize, &'a str)> {
    let missed = words_the_retry_missed(first, retry);
    let queries: Vec<String> = missed
        .iter()
        .filter(|(_, _, has_digit)| !has_digit)
        .map(|(_, word, _)| (*word).to_string())
        .collect();
    let verdicts = dictionary_verdicts(config, queries).await;
    join_verdicts(missed, verdicts)
}

/// `verdicts` holds one entry for each word of `missed` with no digit, in order.
pub(super) fn join_verdicts<'a>(missed: Vec<MissedWord<'a>>, verdicts: Vec<Option<bool>>) -> Vec<(usize, &'a str)> {
    let mut verdicts = verdicts.into_iter();
    missed
        .into_iter()
        .filter(|(_, _, has_digit)| *has_digit || verdicts.next().flatten() != Some(false))
        .map(|(index, word, _)| (index, word))
        .collect()
}

#[cfg(feature = "ocr")]
async fn dictionary_verdicts(config: &OcrConfig, words: Vec<String>) -> Vec<Option<bool>> {
    if words.is_empty() {
        return Vec::new();
    }
    let unanswered = words.len();
    let config = config.clone();
    let lookup = move || crate::ocr::processor::words_in_dictionary(&config, &words);
    #[cfg(feature = "tokio-runtime")]
    let verdicts = tokio::task::spawn_blocking(lookup)
        .await
        .map_err(|error| crate::ocr::error::OcrError::ProcessingFailed(error.to_string()))
        .and_then(|verdicts| verdicts);
    #[cfg(not(feature = "tokio-runtime"))]
    let verdicts = lookup();
    verdicts.unwrap_or_else(|error| {
        tracing::warn!(%error, "dictionary lookup for the sideways-page retry failed; keeping every word");
        vec![None; unanswered]
    })
}

#[cfg(not(feature = "ocr"))]
async fn dictionary_verdicts(_: &OcrConfig, words: Vec<String>) -> Vec<Option<bool>> {
    vec![None; words.len()]
}

/// Add each paragraph's joined words to the kept read, as one paragraph at the place of that
/// first-read paragraph.
fn carry_words(first: &InternalDocument, retry: &mut ExtractedDocument, words: Vec<(usize, &str)>) {
    for chunk in words.chunk_by(|left, right| left.0 == right.0) {
        let text = chunk.iter().map(|(_, word)| *word).collect::<Vec<_>>().join(" ");
        push_carried(retry, &first.elements[chunk[0].0], &text);
    }
}

/// True when two boxes given as left, top, width and height share an area.
fn overlaps(a: (u64, u64, u64, u64), b: (u64, u64, u64, u64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// True when the read carries the page verdict that triggers the retry. A retry must clear the
/// same verdict: a read that is still noise is not a better read of the page.
fn carries_noise_verdict(
    document: &ExtractedDocument,
    backend: &Arc<dyn crate::plugins::OcrBackend>,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let dict_ratio = document
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
        .and_then(serde_json::Value::as_f64);
    page_carries_noise_verdict(
        &document.content,
        thresholds,
        backend.confidence_semantics(),
        mean_text_conf_of(&document.metadata.additional),
        dict_ratio,
    )
}

/// Words of three or more letters or digits. The word floor counts them: a retry that reads fewer
/// of them than the first read has lost words, whatever its quality score says.
pub(super) fn count_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| token.chars().filter(|character| character.is_alphanumeric()).count() >= 3)
        .count()
}

/// True when more than half of the read's word boxes are taller than wide. Lines, blocks and the
/// page box are not words: a tall upright paragraph says nothing about the direction of the text.
fn words_run_sideways(document: &ExtractedDocument) -> bool {
    let (mut tall, mut total) = (0usize, 0usize);
    for element in document.ocr_elements.iter().flatten() {
        if element.level != OcrElementLevel::Word {
            continue;
        }
        if let OcrBoundingGeometry::Rectangle { width, height, .. } = element.geometry {
            total += 1;
            if height > width {
                tall += 1;
            }
        }
    }
    tall * 2 > total
}

/// The noise thresholds the page verdict uses, from the caller's OCR configuration.
pub(super) fn noise_thresholds(config: &crate::core::config::ExtractionConfig) -> OcrQualityThresholds {
    config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default()
}
