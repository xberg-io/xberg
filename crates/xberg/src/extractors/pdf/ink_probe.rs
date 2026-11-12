//! Check a page the script rule would keep native against the page's ink.
//!
//! A text layer in a script no configured OCR language reads is kept native, because OCR would
//! read that script as letters of another one. The layer says what script the page is in, but a
//! wrong `ToUnicode` map fills the layer of an English page with Arabic or Bengali code points just
//! the same. Where nothing but lost-glyph markers vouches for such a layer (no cluster structure
//! to check), the page is read once in the configured languages, and the layer is given up only
//! when the read is confident prose that spells none of the layer's script. A page whose layer is
//! drawn over a full-page picture is read with the Latin model whatever the configured languages,
//! so the verdict depends on the page alone. ~keep

use super::unreadable_script::{
    NativeLayer, first_stage, judgeable_boundaries, ocr_dropped_native_script, page_is_scan, page_reads_as_scan,
    page_text, script_letters, structured_scripts, uncovered_script_counts,
};
use crate::core::config::ExtractionConfig;
use crate::extractors::pdf::ocr::OcrFallbackDecision;
use crate::types::PageBoundary;

/// Pages read first. When none of them is contradicted the rest of the document is not read, so a
/// genuine page in an unreadable script costs at most this many extra OCR reads, not one per page.
const SENTINEL_PAGES: usize = 4;

/// Pages read in all, once a sentinel page was contradicted.
const MAX_PROBED_PAGES: usize = 256;

/// Share of a read's words that must be plain words for the read to be prose. Tesseract reads a
/// script its model does not cover as brackets, digits and runs of capitals; prose is almost
/// all words in lower case or with one leading capital.
const MIN_PLAIN_WORD_SHARE: f64 = 0.9;

/// Words a read needs before its shape says anything.
const MIN_PROSE_WORDS: usize = 8;

/// Least mean letters per word of a read that is words and not strewn single letters.
const MIN_MEAN_WORD_LETTERS: f64 = 3.0;

/// Letters from which a word counts as long.
const LONG_WORD_LETTERS: usize = 4;

/// Distinct long words a read needs: `ii ll ee oo` junk has none, and prose of eight words has more.
const MIN_DISTINCT_LONG_WORDS: usize = 6;

/// Distinct letters a long word needs, so `iiii` or `llll` is not a word.
const MIN_DISTINCT_LETTERS_IN_LONG_WORD: usize = 3;

/// What the ink check found.
#[derive(Default)]
pub(super) struct ProbeOutcome {
    /// Pages whose ink contradicts their layer; the script rule must leave them to OCR.
    pub(super) contradicted: Vec<u32>,
    pub(super) warnings: Vec<crate::types::ProcessingWarning>,
    pub(super) reads: Option<std::sync::Arc<super::ocr::read_reuse::ProbeReads>>,
}

/// Share of a layer's script letters, from which the letters in scripts the configured languages read
/// vouch for the layer's map. A wrong map sends every letter of its font to the other script, so
/// what stays readable is other fonts (a title); the wrong-script fixtures hold 2%, and the genuine
/// Arabic and Hebrew pages 3% and 4%. A correct layer with a minority in another script holds far
/// more: 45% on the minority fixtures, 28% on the mostly Arabic page.
const MIN_READABLE_LETTER_SHARE: f64 = 0.1;

/// Whether a layer already spells a sizable share of its letters (see [`MIN_READABLE_LETTER_SHARE`])
/// in scripts the configured languages read. A read in those languages cannot then contradict the
/// rest of the layer: it finds the prose that the layer spells correctly.
fn layer_spells_read_scripts(text: &str, languages: &[String]) -> bool {
    script_letters(text, languages).is_some_and(|letters| {
        let other: usize = letters.uncovered.iter().map(|(_, count)| count).sum();
        letters.covered as f64 >= MIN_READABLE_LETTER_SHARE * (letters.covered + other) as f64
    })
}

/// The failing pages the script rule would keep on lost-glyph markers alone: their layer is in a
/// script no configured language reads, nothing but the markers vouches for it (see
/// [`structured_scripts`]), and few of its letters are in a script the configured languages read
/// (see [`layer_spells_read_scripts`]).
pub(super) fn unstructured_script_pages(
    decision: &OcrFallbackDecision,
    layer: NativeLayer<'_>,
    config: &ExtractionConfig,
) -> Vec<u32> {
    let (Some(ocr), Some(boundaries)) = (config.ocr.as_ref(), judgeable_boundaries(decision, layer)) else {
        return Vec::new();
    };
    let Some(stage) = first_stage(ocr) else {
        return Vec::new();
    };
    decision
        .failing_pages
        .iter()
        .copied()
        .filter(|page| {
            boundaries
                .iter()
                .find(|boundary| boundary.page_number == *page)
                .and_then(|boundary| layer.text.get(boundary.byte_start..boundary.byte_end))
                .is_some_and(|text| {
                    text.contains(super::unmapped_glyphs::LOST_GLYPH_MARKER)
                        && uncovered_script_counts(text, &stage.languages)
                            .is_some_and(|counts| structured_scripts(text, counts).is_none())
                        && !layer_spells_read_scripts(text, &stage.languages)
                })
        })
        .collect()
}

/// Whether a read is prose: at least [`MIN_PROSE_WORDS`] words, [`MIN_PLAIN_WORD_SHARE`] of them
/// plain (letters only, lower case or one leading capital, edge punctuation aside), a mean of
/// [`MIN_MEAN_WORD_LETTERS`] letters per word, and [`MIN_DISTINCT_LONG_WORDS`] distinct long words
/// of varied letters. The language pipeline holds no dictionary for the stage language, so word
/// shape stands for it.
fn read_is_prose(read: &str) -> bool {
    let words: Vec<&str> = read
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    let is_plain = |word: &str| {
        let joiner = |c: char| matches!(c, '\'' | '\u{2019}' | '-');
        word.chars().all(|c| c.is_alphabetic() || joiner(c)) && !word.chars().skip(1).any(char::is_uppercase)
    };
    let plain = words.iter().copied().filter(|word| is_plain(word)).count();
    let letters: usize = words
        .iter()
        .map(|word| word.chars().filter(|c| c.is_alphabetic()).count())
        .sum();
    let long: std::collections::HashSet<String> = words
        .iter()
        .filter(|word| is_plain(word) && word.chars().filter(|c| c.is_alphabetic()).count() >= LONG_WORD_LETTERS)
        .filter(|word| {
            word.chars()
                .filter(|c| c.is_alphabetic())
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= MIN_DISTINCT_LETTERS_IN_LONG_WORD
        })
        .map(|word| word.to_lowercase())
        .collect();
    words.len() >= MIN_PROSE_WORDS
        && plain as f64 >= MIN_PLAIN_WORD_SHARE * words.len() as f64
        && letters as f64 >= MIN_MEAN_WORD_LETTERS * words.len() as f64
        && long.len() >= MIN_DISTINCT_LONG_WORDS
}

/// Whether an accepted OCR read of a page is that page's text in the script the layer cannot spell:
/// the read is prose, holds at least half as many letters as the layer claims and none of the
/// layer's scripts. A wrong map keeps the letter count of the text it garbles, while a page that
/// is really in the layer's script yields a read that is short, empty, noise, or letters of the
/// script.
pub(super) fn read_contradicts_layer(native_page: &str, read: &str) -> bool {
    let native_letters = native_page
        .chars()
        .filter(|c| c.is_alphabetic() && *c != super::unmapped_glyphs::LOST_GLYPH_MARKER)
        .count();
    let read_letters = read.chars().filter(|c| c.is_alphabetic()).count();
    native_letters > 0
        && read_letters * 2 >= native_letters
        && read_is_prose(read)
        && ocr_dropped_native_script(native_page, read)
}

/// Whether a page's mean OCR confidence (`0.0..=1.0`) clears the acceptance gate. A backend that
/// reports no calibrated score is not confident, so its read contradicts nothing.
fn read_is_confident(score: Option<f64>, thresholds: &crate::core::config::OcrQualityThresholds) -> bool {
    let default_gate = crate::core::config::OcrQualityThresholds::default().min_ocr_mean_confidence;
    let gate = thresholds.min_ocr_mean_confidence.max(default_gate) / 100.0;
    score.is_some_and(|score| score >= gate)
}

/// Up to [`SENTINEL_PAGES`] pages spread over the sorted eligible pages: the first, the last and
/// evenly spaced pages between, so a contradicted page late in the document is still probed.
fn sentinels(eligible: &[u32]) -> Vec<u32> {
    if eligible.len() <= SENTINEL_PAGES {
        return eligible.to_vec();
    }
    let last = eligible.len() - 1;
    let mut picked: Vec<u32> = (0..SENTINEL_PAGES)
        .map(|i| eligible[i * last / (SENTINEL_PAGES - 1)])
        .collect();
    picked.dedup();
    picked
}

/// The next pages to read: the sentinel pages first, then, once one of them was contradicted, the
/// rest of the first [`MAX_PROBED_PAGES`] eligible pages. Empty when the check is over.
fn next_batch(eligible: &[u32], probed: &[u32], any_contradicted: bool) -> Vec<u32> {
    if probed.is_empty() {
        sentinels(eligible)
    } else if any_contradicted {
        eligible
            .iter()
            .take(MAX_PROBED_PAGES)
            .copied()
            .filter(|page| !probed.contains(page))
            .collect()
    } else {
        Vec::new()
    }
}

/// The probed pages that have no score, split by cause: a backend that read words but reports no
/// calibrated score (uncalibrated), and a read with no scored words at all (a blank read after
/// noise discarding, for one), which says nothing about the backend.
fn unscored_pages(
    batch: &[u32],
    confidence: &ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
) -> (Vec<u32>, Vec<u32>) {
    let (mut uncalibrated, mut unscored) = (Vec::new(), Vec::new());
    for page in batch {
        match confidence.get(page) {
            Some(c) if c.score.is_none() && c.word_count > 0 => uncalibrated.push(*page),
            Some(c) if c.score.is_none() => unscored.push(*page),
            _ => {}
        }
    }
    (uncalibrated, unscored)
}

/// The pages of a batch the OCR run reported no confidence for: it produced no read of them.
fn unread_pages(batch: &[u32], confidence: &ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>) -> Vec<u32> {
    batch
        .iter()
        .copied()
        .filter(|page| !confidence.contains_key(page))
        .collect()
}

fn warning(message: String) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Owned(message),
    }
}

/// The failing pages whose layer only its lost-glyph markers vouch for (see
/// [`unstructured_script_pages`]) and whose ink an OCR read contradicts (see
/// [`read_contradicts_layer`]): the script rule must leave them to OCR.
///
/// The read goes through the ordinary page route with noise discarding forced on, and only a read
/// whose page confidence clears the acceptance gate counts. Scan pages (see [`page_reads_as_scan`])
/// are left out, since OCR reads them anyway. Each page whose layer is drawn over its full-page
/// picture is read from a render without its pictures (see [`read_without_pictures`]). A document
/// over the OCR preview size limit is not read, and a probe that fails leaves the layer native with
/// a warning. Cancellation is returned as the error.
///
/// Of the other pages, up to four spread across the candidates (first, last, evenly spaced) are
/// read first; the rest of the first [`MAX_PROBED_PAGES`] candidates only if a page is
/// contradicted. A sentinel outside those first candidates is read in addition to the cap.
pub(super) async fn contradicted_pages(
    decision: &OcrFallbackDecision,
    layer: NativeLayer<'_>,
    content: &[u8],
    config: &ExtractionConfig,
    layout_pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
) -> crate::Result<ProbeOutcome> {
    let mut outcome = ProbeOutcome::default();
    let candidates = unstructured_script_pages(decision, layer, config);
    let (Some(boundaries), false) = (layer.boundaries, candidates.is_empty()) else {
        return Ok(outcome);
    };
    if content.len() > super::ocr::preview_byte_limit(config) {
        outcome.warnings.push(warning(format!(
            "The ink of pages {candidates:?} was not checked against their text layer because the document \
             exceeds the OCR preview size limit; their native text is kept."
        )));
        return Ok(outcome);
    }
    let document = match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(document) => document,
        Err(error) => {
            tracing::debug!(%error, "script evidence skipped the ink probe: the PDF could not be opened");
            return Ok(outcome);
        }
    };
    let mut eligible: Vec<u32> = candidates
        .into_iter()
        .filter(|page| {
            *page > 0
                && !page_reads_as_scan(
                    &document,
                    *page as usize - 1,
                    page_text(layer.text, boundaries, *page).unwrap_or_default(),
                    config,
                )
        })
        .collect();
    eligible.sort_unstable();
    eligible.dedup();
    let (pictures, eligible): (Vec<u32>, Vec<u32>) = eligible
        .into_iter()
        .partition(|page| page_is_scan(&document, *page as usize - 1));
    let (probe_config, thresholds) = discarding_noise(config);
    let retention_limit = config.security_limits.clone().unwrap_or_default().max_content_size;
    outcome.reads = Some(std::sync::Arc::new(super::ocr::read_reuse::ProbeReads::new(
        retention_limit,
    )));
    let probe = Probe {
        layer: layer.text,
        boundaries,
        config: &probe_config,
        thresholds: &thresholds,
    };
    read_without_pictures(&probe, &document, &pictures, &mut outcome).await?;
    let mut probed: Vec<u32> = Vec::new();
    loop {
        let batch = next_batch(&eligible, &probed, !outcome.contradicted.is_empty());
        if batch.is_empty() {
            return Ok(outcome);
        }
        probed.extend_from_slice(&batch);
        if !read_through_page_route(&probe, &batch, content, layout_pages, &mut outcome).await? {
            return Ok(outcome);
        }
    }
}

/// What the probe judges a read against: the native layer, and the config and thresholds it reads
/// with (see [`discarding_noise`]).
struct Probe<'a> {
    layer: &'a str,
    boundaries: &'a [PageBoundary],
    config: &'a ExtractionConfig,
    thresholds: &'a crate::core::config::OcrQualityThresholds,
}

/// Reads `pages` through the ordinary page route and judges the reads (see [`judge_reads`]). False
/// when the run failed, after a warning that the layers are kept; an error when the extraction is
/// cancelled.
async fn read_through_page_route(
    probe: &Probe<'_>,
    pages: &[u32],
    content: &[u8],
    layout_pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
    outcome: &mut ProbeOutcome,
) -> crate::Result<bool> {
    let run = super::ocr::extract_mixed_ocr_native_with_probe_reads(
        probe.layer,
        probe.boundaries,
        (pages, &[]),
        content,
        probe.config,
        layout_pages,
        (&[], &[], outcome.reads.clone()),
    )
    .await;
    super::ensure_pdf_not_cancelled(probe.config)?;
    Ok(match run {
        Ok(run) => {
            judge_reads(probe, pages, (&run.output.1, &run.output.7), outcome);
            true
        }
        Err(error) => {
            tracing::debug!(%error, "script evidence kept the layer: the ink probe failed");
            outcome.warnings.push(warning(format!(
                "The ink of pages {pages:?} could not be checked against their text layer ({error}); \
                 their native text is kept."
            )));
            false
        }
    })
}

/// Reads each of `pages`, whose layer is drawn over a full-page picture, from a render without its
/// pictures (see [`super::picture_layer::page_without_pictures`]) with the Latin reference reader
/// (see [`reference_reader`]), and judges the reads (see [`judge_reads`]). The picture is left out
/// because a textured picture turns the read of the glyphs on it into noise. A page whose render or
/// read fails, as when the Latin model is missing and cannot be downloaded, has no verdict: it is
/// listed as contradicted, so it is read as the scan it was taken for. An error when the extraction
/// is cancelled. ~keep
async fn read_without_pictures(
    probe: &Probe<'_>,
    document: &xberg_native_pdf::PdfDocument,
    pages: &[u32],
    outcome: &mut ProbeOutcome,
) -> crate::Result<()> {
    let reader = reference_reader(probe);
    let (mut reads, mut confidence) = (ahash::AHashMap::new(), ahash::AHashMap::new());
    for page in pages {
        let Some(image) = super::picture_layer::page_without_pictures(document, *page as usize - 1, &reader) else {
            continue;
        };
        let read = super::ocr::read_rendered_page(image, &reader).await;
        super::ensure_pdf_not_cancelled(&reader)?;
        match read {
            Ok((read, page_confidence)) => {
                reads.insert(*page, read);
                confidence.extend(page_confidence.map(|page_confidence| (*page, page_confidence)));
            }
            Err(error) => tracing::debug!(%error, page, "script evidence could not read the page without pictures"),
        }
    }
    let unread = unread_pages(pages, &confidence);
    if !unread.is_empty() {
        outcome.warnings.push(warning(format!(
            "The ink of pages {unread:?} could not be read with the English OCR model to check their text \
             layer; they are read as scans."
        )));
        outcome.contradicted.extend_from_slice(&unread);
    }
    let read: Vec<u32> = pages.iter().copied().filter(|page| !unread.contains(page)).collect();
    judge_reads(probe, &read, (&reads, &confidence), outcome);
    Ok(())
}

/// The config of the read that judges a layer drawn over a picture: the probe's config with OCR
/// set to Tesseract's Latin model (`eng`, downloaded on first use like any language) and the
/// probe's thresholds, whatever backend and languages the caller configured. The pipeline holds
/// that one stage, so a weak read is judged as it is, never replaced by another backend's. The
/// caller's model directory and OCR limits are kept. ~keep
fn reference_reader(probe: &Probe<'_>) -> ExtractionConfig {
    let mut reader = probe.config.clone();
    let configured = reader.ocr.take().unwrap_or_default();
    let latin = crate::core::config::OcrConfig::default();
    let stage = crate::core::config::OcrPipelineStage {
        backend: latin.backend.clone(),
        priority: 100,
        language: Some(latin.language.clone()),
        tesseract_config: None,
        paddle_ocr_config: None,
        paddle_ocr_settings: None,
        vlm_config: None,
        backend_options: None,
    };
    reader.ocr = Some(crate::core::config::OcrConfig {
        quality_thresholds: Some(probe.thresholds.clone()),
        pipeline: Some(crate::core::config::OcrPipelineConfig {
            stages: vec![stage],
            quality_thresholds: probe.thresholds.clone(),
        }),
        tessdata_path: configured.tessdata_path,
        security_limits: configured.security_limits,
        ..latin
    });
    reader
}

/// Adds to `outcome` the pages of `pages` whose confident read contradicts their layer, and a
/// warning for each kind of page that has no usable read: no read at all, a read whose backend
/// reports no calibrated confidence, and a read with no scored words.
fn judge_reads(
    probe: &Probe<'_>,
    pages: &[u32],
    (reads, confidence): (
        &ahash::AHashMap<u32, String>,
        &ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    ),
    outcome: &mut ProbeOutcome,
) {
    let unread = unread_pages(pages, confidence);
    if !unread.is_empty() {
        outcome.warnings.push(warning(format!(
            "The ink of pages {unread:?} could not be checked against their text layer (the OCR read \
             failed); their native text is kept."
        )));
    }
    let (uncalibrated, unscored) = unscored_pages(pages, confidence);
    if !uncalibrated.is_empty() {
        outcome.warnings.push(warning(format!(
            "The ink of pages {uncalibrated:?} could not be checked against their text layer because the \
             OCR backend reports no calibrated confidence; their native text is kept."
        )));
    }
    if !unscored.is_empty() {
        outcome.warnings.push(warning(format!(
            "The ink of pages {unscored:?} could not be checked against their text layer (the OCR read \
             had no scored words); their native text is kept."
        )));
    }
    outcome.contradicted.extend(pages.iter().copied().filter(|page| {
        page_text(probe.layer, probe.boundaries, *page)
            .zip(reads.get(page))
            .is_some_and(|(native, read)| {
                read_is_confident(confidence.get(page).and_then(|c| c.score), probe.thresholds)
                    && read_contradicts_layer(native, read)
            })
    }));
}

/// The config the probe reads with: suspected recognition noise is discarded whatever the caller
/// set, so a noisy read comes back blank. Also returns the quality thresholds in effect.
fn discarding_noise(config: &ExtractionConfig) -> (ExtractionConfig, crate::core::config::OcrQualityThresholds) {
    let mut probe = config.clone();
    let mut thresholds = crate::core::config::OcrQualityThresholds::default();
    if let Some(ocr) = probe.ocr.as_mut() {
        thresholds = ocr.effective_thresholds();
        thresholds.discard_suspected_ocr_noise = true;
        ocr.quality_thresholds = Some(thresholds.clone());
    }
    (probe, thresholds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARABIC: &str = "ثخغ غػصبا تثرعا عظثبسثس ثخغ ظصج شازض ثخغ ؽثدؼثظ";
    const ENGLISH: &str = "the committee met on Tuesday to review the plan for the new library and the parking area";

    #[test]
    fn a_layer_with_a_tenth_of_its_letters_in_a_read_script_is_not_left_to_the_probe() {
        let eng = ["eng".to_string()];
        assert_eq!(ARABIC.chars().filter(|c| !c.is_whitespace()).count(), 39);
        assert!(!layer_spells_read_scripts(&format!("Note \u{FFFD}{ARABIC}"), &eng));
        assert!(layer_spells_read_scripts(&format!("Notes \u{FFFD}{ARABIC}"), &eng));
        assert!(!layer_spells_read_scripts(
            &format!("Notes \u{FFFD}{ARABIC}"),
            &["xyz".to_string()]
        ));
    }

    #[test]
    fn a_layer_with_exactly_a_tenth_of_its_letters_in_a_read_script_is_not_left_to_the_probe() {
        let eng = ["eng".to_string()];
        let arabic: String = ARABIC.chars().filter(|c| !c.is_whitespace()).take(36).collect();
        let at_a_tenth = format!("Note \u{FFFD}{arabic}");
        assert_eq!(at_a_tenth.chars().filter(|c| c.is_alphabetic()).count(), 40);
        assert!(layer_spells_read_scripts(&at_a_tenth, &eng));
        assert!(!layer_spells_read_scripts(&format!("Not \u{FFFD}{arabic}"), &eng));
    }

    #[test]
    fn a_confident_prose_read_in_another_script_contradicts_the_layer() {
        let layer = format!("\u{FFFD}{ARABIC}");
        assert!(read_contradicts_layer(&layer, ENGLISH));
    }

    #[test]
    fn a_short_or_empty_read_or_one_in_the_layers_script_contradicts_nothing() {
        let layer = format!("\u{FFFD}{ARABIC}");
        assert!(!read_contradicts_layer(&layer, "fox"));
        assert!(!read_contradicts_layer(&layer, ""));
        assert!(!read_contradicts_layer(&layer, &layer));
    }

    #[test]
    fn junk_a_latin_model_reads_from_another_script_is_not_a_contradiction() {
        let layer = format!("\u{FFFD}{ARABIC}");
        for junk in [
            "ii ll ee oo ae ss ii ll ee oo ae ss ii ll ee oo ae ss",
            "iiii llll eeee oooo ssss aaaa iiii llll eeee oooo ssss aaaa",
            "ill lee ooo aes sis ill lee ooo aes sis ill lee ooo aes sis",
            "8 [FU [ish AYE AYE Aya Otc [AN leu siAl ANAN HE Hinnn UAT",
            "Il Ill lI ll Il Ill lI ll Il Ill lI ll Il Ill lI ll",
        ] {
            assert!(!read_contradicts_layer(&layer, junk), "{junk}");
        }
    }

    #[test]
    fn a_read_needs_a_score_that_clears_the_acceptance_gate() {
        let thresholds = crate::core::config::OcrQualityThresholds::default();
        assert!(read_is_confident(Some(0.9), &thresholds));
        assert!(read_is_confident(Some(0.75), &thresholds));
        assert!(!read_is_confident(Some(0.74), &thresholds));
        assert!(
            !read_is_confident(None, &thresholds),
            "no calibrated score is not confidence"
        );
        let disabled = crate::core::config::OcrQualityThresholds {
            min_ocr_mean_confidence: 0.0,
            ..Default::default()
        };
        assert!(
            !read_is_confident(Some(0.4), &disabled),
            "a disabled gate does not lower the bar"
        );
    }

    #[test]
    fn a_score_less_page_is_uncalibrated_only_when_the_backend_read_words() {
        let mut confidence = ahash::AHashMap::new();
        let page = |score, word_count| crate::types::page::PageOcrConfidence {
            score,
            word_count,
            backend: "paddleocr".to_owned(),
        };
        confidence.insert(1, page(None, 40));
        confidence.insert(2, page(None, 0));
        confidence.insert(3, page(Some(0.9), 40));
        let (uncalibrated, unscored) = unscored_pages(&[1, 2, 3, 4], &confidence);
        assert_eq!(uncalibrated, vec![1]);
        assert_eq!(unscored, vec![2]);
    }

    #[test]
    fn a_page_the_ocr_run_reported_nothing_for_is_unread() {
        let mut confidence = ahash::AHashMap::new();
        confidence.insert(2, crate::types::page::PageOcrConfidence::default());
        assert_eq!(unread_pages(&[1, 2, 3], &confidence), vec![1, 3]);
        assert!(unread_pages(&[2], &confidence).is_empty());
    }

    #[test]
    fn the_probe_reads_sentinel_pages_and_goes_on_only_after_a_contradiction() {
        let pages: Vec<u32> = (1..=300).collect();
        let first = next_batch(&pages, &[], false);
        assert_eq!(first, vec![1, 100, 200, 300], "first, last and evenly spaced between");
        assert!(
            next_batch(&pages, &first, false).is_empty(),
            "no contradiction ends the check"
        );
        let rest = next_batch(&pages, &first, true);
        assert_eq!(
            rest.len(),
            MAX_PROBED_PAGES - 3,
            "the cap counts from the first eligible page"
        );
        assert!(rest.iter().all(|page| !first.contains(page)));
        assert_eq!(next_batch(&pages[..2], &[], false), vec![1, 2]);
        assert!(next_batch(&pages[..2], &[1, 2], true).is_empty());
    }

    #[test]
    fn a_contradicted_page_last_among_many_is_still_a_sentinel() {
        let pages: Vec<u32> = (10..=30).collect();
        assert_eq!(sentinels(&pages), vec![10, 16, 23, 30]);
        assert_eq!(sentinels(&[7]), vec![7]);
        assert_eq!(sentinels(&[7, 9]), vec![7, 9]);
        assert_eq!(sentinels(&[7, 9, 11, 13]), vec![7, 9, 11, 13]);
        assert_eq!(sentinels(&[7, 9, 11, 13, 15, 17]), vec![7, 9, 13, 17]);
        assert_eq!(sentinels(&[7, 9, 11, 13, 15]), vec![7, 9, 11, 15]);
    }

    #[test]
    fn a_short_english_page_with_few_long_words_and_capitals_is_not_prose() {
        let layer = format!("\u{FFFD}{ARABIC}");
        let short = "THE CAT SAT ON THE MAT AND ATE THE FISH";
        let few_long = "the cat sat on a mat and ate the fish in a pan";
        assert!(!read_is_prose(short), "all capitals are not plain words");
        assert!(!read_is_prose(few_long), "thirteen short words hold too few long ones");
        assert!(!read_contradicts_layer(&layer, short));
        assert!(!read_contradicts_layer(&layer, few_long));
    }

    /// #2826: a layer over a picture is judged by Tesseract's Latin model whatever the caller
    /// configured, in the caller's model directory and with the probe's thresholds.
    #[test]
    fn a_layer_over_a_picture_is_read_with_the_latin_model_whatever_the_configured_languages() {
        let configured = ExtractionConfig {
            ocr: Some(crate::OcrConfig {
                backend: "paddleocr".to_string(),
                language: vec!["khm".to_string()],
                tesseract_config: Some(crate::types::TesseractConfig {
                    language: vec!["khm".to_string()],
                    ..Default::default()
                }),
                tessdata_path: Some("/models".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let (config, thresholds) = discarding_noise(&configured);
        let probe = Probe {
            layer: "",
            boundaries: &[],
            config: &config,
            thresholds: &thresholds,
        };
        let reader = reference_reader(&probe);
        let ocr = reader.ocr.as_ref().unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.effective_tesseract_language(), ["eng"]);
        assert_eq!(ocr.tessdata_path.as_deref(), Some(std::path::Path::new("/models")));
        assert!(ocr.effective_thresholds().discard_suspected_ocr_noise);
        let pipeline = ocr.effective_pipeline().expect("one fixed stage");
        assert_eq!(pipeline.stages.len(), 1, "no other backend replaces a weak read");
        let stage = crate::extractors::pdf::ocr::stage_model_inputs::StageModelInputs::new(ocr, &pipeline.stages[0]);
        assert_eq!(stage.backend(), "tesseract");
        assert_eq!(stage.languages(), ["eng"]);
        assert!(pipeline.quality_thresholds.discard_suspected_ocr_noise);
    }
}
