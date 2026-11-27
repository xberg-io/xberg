//! Language detection using the whatlang Rust crate.
//!
//! Provides fast language detection for extracted text content.

use crate::Result;
use crate::core::config::LanguageDetectionConfig;
use crate::types::LanguageConfidence;
use whatlang::{Lang, detect};

pub mod processor;
pub use processor::LanguageDetector;

/// Confidence threshold above which an aggregated (multi-chunk) per-language
/// detection is considered reliable.
///
/// Mirrors whatlang's own `Info::is_reliable()` threshold (not exposed as a public
/// constant by whatlang), applied here to the chunk-averaged confidence for a
/// language rather than to a single detection instance (#261).
const AGGREGATE_RELIABLE_THRESHOLD: f64 = 0.9;

/// Number of characters per chunk when detecting multiple languages.
///
/// `pub(crate)` so the PDF OCR plausibility detector's prose-chunking (issue #1696, in
/// `extractors::pdf::ocr::plausibility`) chunks on the same boundary this module's own
/// multi-language detection uses, rather than picking an independent chunk size that would
/// need its own calibration. ~keep
pub(crate) const CHUNK_SIZE: usize = 200;

/// Per-chunk language-detection reliability, aggregated over a caller-supplied set of text
/// chunks (issue #1696's plausibility signal).
///
/// Distinct from [`LangAggregate`]: that type aggregates *per detected language* for the
/// public multi-language API; this aggregates *across all chunks regardless of language* for
/// a single plausibility verdict — a wrong-mapped page has no one "detected language" to
/// aggregate toward, only a chunk-by-chunk reliability record. ~keep
// ~keep: the only consumer is `extractors::pdf::ocr::plausibility`, whose module is gated on
// `pdf` + (`ocr` | `ocr-pipeline`); under CI's `ocr,auto-rotate-tract` leg `language-detection`
// is on but `pdf` is off, so an ungated seam is dead code and fails `-D warnings`.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ChunkReliability {
    /// Total chunks evaluated.
    pub(crate) chunks: usize,
    /// Chunks whatlang classified as reliable (`Info::is_reliable()`, a margin between the
    /// best and second-best language guess — not the same axis as raw confidence).
    pub(crate) reliable_chunks: usize,
    /// Sum of whatlang's per-chunk confidence, `0.0` for a chunk whatlang could not classify
    /// at all (`detect` returning `None`), so the mean below is never inflated by treating an
    /// unclassifiable chunk as absent rather than as evidence of implausibility. ~keep
    pub(crate) confidence_sum: f64,
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
impl ChunkReliability {
    /// Fraction of chunks whatlang classified as reliable, in `[0.0, 1.0]`. `0.0` when no
    /// chunks were evaluated.
    pub(crate) fn reliable_ratio(&self) -> f64 {
        if self.chunks == 0 {
            0.0
        } else {
            self.reliable_chunks as f64 / self.chunks as f64
        }
    }

    /// Mean whatlang confidence across every evaluated chunk, in `[0.0, 1.0]`. `0.0` when no
    /// chunks were evaluated.
    pub(crate) fn mean_confidence(&self) -> f64 {
        if self.chunks == 0 {
            0.0
        } else {
            self.confidence_sum / self.chunks as f64
        }
    }
}

/// Run whatlang over each of `chunks` and aggregate reliability/confidence.
///
/// The only site outside this module's own multi-language detection that touches whatlang
/// directly — kept here so every whatlang call in the crate goes through one seam. A chunk
/// whatlang cannot classify at all (`detect` returns `None`, e.g. no alphabetic content)
/// counts as confidence `0.0` and not reliable, rather than being skipped: skipping it would
/// undercount `chunks` and let a page of entirely unclassifiable text score an artificially
/// high `reliable_ratio` off a near-empty denominator. ~keep
///
/// Chunks are read in order and the read stops once `stop_at_reliable` of them have been
/// classified reliable, returning `None`: the reliable count only grows, so a caller whose
/// verdict is settled by that count need not pay whatlang for the rest. `usize::MAX` reads
/// every chunk. ~keep
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(crate) fn chunk_reliability(chunks: &[&str], stop_at_reliable: usize) -> Option<ChunkReliability> {
    if stop_at_reliable == 0 {
        return None;
    }
    let mut reliable_chunks = 0usize;
    let mut confidence_sum = 0.0f64;

    for chunk in chunks {
        if let Some(info) = detect(chunk) {
            confidence_sum += info.confidence();
            if info.is_reliable() {
                reliable_chunks += 1;
                if reliable_chunks >= stop_at_reliable {
                    return None;
                }
            }
        }
    }

    Some(ChunkReliability {
        chunks: chunks.len(),
        reliable_chunks,
        confidence_sum,
    })
}

/// Detect languages in text using whatlang.
///
/// Returns a list of detected language codes (ISO 639-3 format).
/// Returns `None` if no languages could be detected with sufficient confidence.
///
/// In `config.detect_multiple` mode, languages are ordered by descending
/// chunk-share — the first entry is the language whatlang detected most
/// often across the document's 200-character chunks, ties broken by ISO
/// 639-3 code. This is a thin wrapper over [`detect_language_details`] that
/// keeps `ExtractedDocument::detected_languages` backward compatible; callers
/// that also need confidence, document share, script, and reliability should
/// use [`detect_language_details`] / `ExtractedDocument::detected_language_confidences`
/// instead (#261).
///
/// # Arguments
///
/// * `text` - The text to analyze for language detection
/// * `config` - Optional configuration for language detection
///
/// # Example
///
/// Not run as a doctest: this function is `pub(crate)`. Downstream crates reach it by
/// setting [`crate::core::config::LanguageDetectionConfig`] on the extraction config and
/// reading `ExtractedDocument::detected_languages`.
///
/// ```ignore
/// use xberg::language_detection::detect_languages;
/// use xberg::core::config::LanguageDetectionConfig;
///
/// let text = "Hello world! This is English text.";
/// let config = LanguageDetectionConfig {
///     enabled: true,
///     min_confidence: 0.8,
///     detect_multiple: false,
/// };
/// let languages = detect_languages(text, &config).expect("language detection succeeded");
/// println!("Detected languages: {:?}", languages);
/// ```
pub(crate) fn detect_languages(text: &str, config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    Ok(detect_language_details(text, config)?
        .map(|details| details.into_iter().map(|detail| detail.language).collect()))
}

/// Detect languages in text using whatlang, returning structured per-language details.
///
/// Unlike [`detect_languages`], this carries the confidence, document-share proportion,
/// script, and reliability for every detected language, in the same order `detect_languages`
/// would return their codes (#261). Returns `None` under the same conditions as
/// `detect_languages`: detection disabled, empty input text, or no language meeting
/// `config.min_confidence`.
pub(crate) fn detect_language_details(
    text: &str,
    config: &LanguageDetectionConfig,
) -> Result<Option<Vec<LanguageConfidence>>> {
    if !config.enabled {
        return Ok(None);
    }

    if text.trim().is_empty() {
        return Ok(None);
    }

    if !config.detect_multiple {
        return detect_single_language_details(text, config);
    }

    detect_multiple_languages_details(text, config)
}

/// Detect a single primary language in the text, with structured details.
fn detect_single_language_details(
    text: &str,
    config: &LanguageDetectionConfig,
) -> Result<Option<Vec<LanguageConfidence>>> {
    match detect(text) {
        Some(info) => {
            if info.confidence() >= config.min_confidence {
                Ok(Some(vec![LanguageConfidence {
                    language: lang_to_iso639_3(info.lang()),
                    confidence: info.confidence(),
                    proportion: 1.0,
                    script: info.script().name().to_string(),
                    reliable: info.is_reliable(),
                }]))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Per-language running totals accumulated while scanning chunks in
/// [`detect_multiple_languages_details`].
struct LangAggregate {
    /// Number of chunks classified as this language above `min_confidence`.
    count: usize,
    /// Sum of whatlang's per-chunk confidence for this language, used to compute
    /// the chunk-averaged confidence once scanning completes.
    confidence_sum: f64,
    /// Script from the most recently scanned chunk classified as this language.
    script: whatlang::Script,
}

/// Detect multiple languages in the text by analyzing chunks, with structured details.
///
/// This splits the text into chunks and detects the language of each chunk, then
/// returns per-language confidence, proportion, script, and reliability for the
/// most common languages found, ordered by descending chunk-share (ties broken by
/// ISO 639-3 code).
fn detect_multiple_languages_details(
    text: &str,
    config: &LanguageDetectionConfig,
) -> Result<Option<Vec<LanguageConfidence>>> {
    let char_vec: Vec<char> = text.chars().collect();
    let chunk_strings: Vec<String> = char_vec
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect();

    if chunk_strings.is_empty() {
        return Ok(None);
    }

    let mut lang_aggregates: ahash::AHashMap<Lang, LangAggregate> = ahash::AHashMap::new();
    let threshold = config.min_confidence;

    for chunk in &chunk_strings {
        if let Some(info) = detect(chunk)
            && info.confidence() >= threshold
        {
            let aggregate = lang_aggregates.entry(info.lang()).or_insert(LangAggregate {
                count: 0,
                confidence_sum: 0.0,
                script: info.script(),
            });
            aggregate.count += 1;
            aggregate.confidence_sum += info.confidence();
            aggregate.script = info.script();
        }
    }

    if lang_aggregates.is_empty() {
        return detect_single_language_details(text, config);
    }

    let total_chunks = chunk_strings.len() as f64;
    let mut lang_vec: Vec<(Lang, LangAggregate)> = lang_aggregates.into_iter().collect();
    lang_vec.sort_by(|a, b| {
        b.1.count
            .cmp(&a.1.count)
            .then_with(|| lang_to_iso639_3(a.0).cmp(&lang_to_iso639_3(b.0)))
    });

    let details = lang_vec
        .into_iter()
        .map(|(lang, aggregate)| {
            let confidence = aggregate.confidence_sum / aggregate.count as f64;
            LanguageConfidence {
                language: lang_to_iso639_3(lang),
                confidence,
                proportion: aggregate.count as f64 / total_chunks,
                script: aggregate.script.name().to_string(),
                reliable: confidence > AGGREGATE_RELIABLE_THRESHOLD,
            }
        })
        .collect();

    Ok(Some(details))
}

/// Convert whatlang Lang enum to ISO 639-3 language code.
///
/// Maps whatlang's language codes to standardized ISO 639-3 codes.
fn lang_to_iso639_3(lang: Lang) -> String {
    match lang {
        Lang::Eng => "eng",
        Lang::Rus => "rus",
        Lang::Cmn => "cmn",
        Lang::Spa => "spa",
        Lang::Por => "por",
        Lang::Ita => "ita",
        Lang::Fra => "fra",
        Lang::Deu => "deu",
        Lang::Ukr => "ukr",
        Lang::Kat => "kat",
        Lang::Ara => "ara",
        Lang::Hin => "hin",
        Lang::Jpn => "jpn",
        Lang::Heb => "heb",
        Lang::Yid => "yid",
        Lang::Pol => "pol",
        Lang::Amh => "amh",
        Lang::Jav => "jav",
        Lang::Kor => "kor",
        Lang::Nob => "nob",
        Lang::Dan => "dan",
        Lang::Swe => "swe",
        Lang::Fin => "fin",
        Lang::Tur => "tur",
        Lang::Nld => "nld",
        Lang::Hun => "hun",
        Lang::Ces => "ces",
        Lang::Ell => "ell",
        Lang::Bul => "bul",
        Lang::Bel => "bel",
        Lang::Mar => "mar",
        Lang::Kan => "kan",
        Lang::Ron => "ron",
        Lang::Slv => "slv",
        Lang::Hrv => "hrv",
        Lang::Srp => "srp",
        Lang::Mkd => "mkd",
        Lang::Lit => "lit",
        Lang::Lav => "lav",
        Lang::Est => "est",
        Lang::Tam => "tam",
        Lang::Vie => "vie",
        Lang::Urd => "urd",
        Lang::Tha => "tha",
        Lang::Guj => "guj",
        Lang::Uzb => "uzb",
        Lang::Pan => "pan",
        Lang::Aze => "aze",
        Lang::Ind => "ind",
        Lang::Tel => "tel",
        Lang::Pes => "pes",
        Lang::Mal => "mal",
        Lang::Ori => "ori",
        Lang::Mya => "mya",
        Lang::Nep => "nep",
        Lang::Sin => "sin",
        Lang::Khm => "khm",
        Lang::Tuk => "tuk",
        Lang::Aka => "aka",
        Lang::Zul => "zul",
        Lang::Sna => "sna",
        Lang::Afr => "afr",
        Lang::Lat => "lat",
        Lang::Slk => "slk",
        Lang::Cat => "cat",
        Lang::Tgl => "tgl",
        Lang::Hye => "hye",
        Lang::Epo => "epo",
        Lang::Ben => "ben",
        Lang::Cym => "cym",
    }
    .to_string()
}

#[cfg(test)]
mod tests;
