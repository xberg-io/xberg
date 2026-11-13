//! Second Tesseract read of a page whose first read is noise in a script the configured list
//! cannot read (GH#2300), merged per line so every word the first read got right stays.

use std::collections::HashMap;
use std::path::Path;

use xberg_tesseract::{TessPageSegMode, TesseractAPI};

use super::api_pool::{RetryApiPools, RetryStage};
use super::validation::{TessdataEnv, first_directory_holding, tessdata_search_dirs};
use crate::core::extractor::cacheability;
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::RetainedWordConfidenceStats;

/// The margin by which the mean word confidence of a retry line must exceed the first read's on
/// the same line. ~keep
pub(super) const RETRY_MIN_CONF_GAIN: f64 = 20.0;

/// A page takes the retry only when at least this share of its first-read lines is admitted: one
/// line in `PAGE_FLOOR_DENOMINATOR`. Fixed, never tuned per page. ~keep
const PAGE_FLOOR_DENOMINATOR: usize = 3;

/// The traineddata model used when OSD identifies Han.
const HAN_MODEL: &str = "chi_sim";
pub(super) const VERTICAL_JAPANESE_MODEL: &str = "jpn_vert";
const VERTICAL_PSM: u8 = 5;

fn supports_vertical_japanese(language: &str, psm: u8) -> bool {
    language == "eng" && matches!(psm, 3 | 11)
}
pub(super) const KHMER_MODEL: &str = "khm";
pub(crate) const KHMER_SUPPLEMENT_KEY: &str = "xberg_mixed_pdf_khmer_supplement";

tokio::task_local! {
    pub(crate) static MIXED_PDF_KHMER_RETRY: bool;
}
pub(crate) fn mixed_pdf_khmer_retry_requested() -> bool {
    MIXED_PDF_KHMER_RETRY.try_with(|enabled| *enabled).unwrap_or(false)
}

pub(super) const METADATA_KEY: &str = "script_retry";

/// Tesseract OSD script names the retry knows a model for.
fn model_for_script(script: &str) -> Option<&'static str> {
    (script == "Han").then_some(HAN_MODEL)
}

pub(super) fn supports_retry(config: &crate::ocr::types::TesseractConfig) -> bool {
    supports_retry_format(&config.output_format)
        && (supports_retry_language(&config.language)
            || (mixed_pdf_khmer_retry_requested() && may_retry_khmer(&config.language)))
}

pub(crate) fn supports_retry_format(format: &str) -> bool {
    matches!(format, "text" | "markdown")
}

fn supports_retry_language(language: &str) -> bool {
    !list_reads_script_of(language, HAN_MODEL)
}

pub(crate) fn may_retry_language(language: &str) -> bool {
    super::execution::is_all_languages(language) || supports_retry_language(language)
}

pub(crate) fn requires_outer_cache_bypass(
    language: &str,
    vertical_japanese_retry: bool,
    override_path: Option<&Path>,
    env: &TessdataEnv,
) -> bool {
    // A wildcard is expanded by the processor after resolving the primary tessdata directory. ~keep
    if super::execution::is_all_languages(language) {
        return true;
    }
    if !supports_retry_language(language) {
        return false;
    }
    let directories = tessdata_search_dirs(override_path, env);
    retry_directories(&directories, &retry_list(language, HAN_MODEL)).is_some()
        || (vertical_japanese_retry && vertical_japanese_directory(&directories, language).is_some())
}

pub(crate) fn may_retry_khmer(language: &str) -> bool {
    !list_reads_script_of(language, KHMER_MODEL)
}

pub(crate) fn requires_khmer_outer_cache_bypass(
    language: &str,
    override_path: Option<&Path>,
    env: &TessdataEnv,
) -> bool {
    may_retry_khmer(language)
        && first_directory_holding(
            &tessdata_search_dirs(override_path, env),
            &retry_list(language, KHMER_MODEL),
        )
        .is_some()
}

/// Whether the configured `+`-joined list already reads the script of `model`.
fn list_reads_script_of(language: &str, model: &str) -> bool {
    let readers: &[&str] = match model {
        HAN_MODEL => &[
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
        KHMER_MODEL => &["khm", "script/khmer"],
        _ => &[],
    };
    language
        .split('+')
        .map(|item| item.trim().to_ascii_lowercase())
        .any(|item| item == "*" || readers.contains(&item.as_str()))
}

mod model_files;
mod vertical_japanese;
pub(super) use crate::core::extractor::cacheability::request_model_digest as hash_model_file;
pub(super) use model_files::RetryModelFiles;

#[derive(Clone, Debug)]
pub(super) struct ModelSet {
    pub(super) directory: String,
    pub(super) languages: String,
    revision: blake3::Hash,
    implicit_osd: bool,
}

impl PartialEq for ModelSet {
    fn eq(&self, other: &Self) -> bool {
        self.directory == other.directory
            && self.languages == other.languages
            && self.revision == other.revision
            && self.implicit_osd == other.implicit_osd
    }
}

impl Eq for ModelSet {}

impl ModelSet {
    pub(super) fn snapshot(directory: String, languages: &[String], implicit_osd: bool) -> Result<Self, OcrError> {
        let revision = Self::revision(&directory, languages.iter().map(String::as_str), implicit_osd)?;
        Ok(Self {
            revision,
            directory,
            languages: languages.join("+"),
            implicit_osd,
        })
    }

    pub(super) fn from_revision(
        directory: String,
        languages: String,
        revision: blake3::Hash,
        implicit_osd: bool,
    ) -> Self {
        Self {
            directory,
            languages,
            revision,
            implicit_osd,
        }
    }

    pub(super) fn validate(&self) -> Result<(), OcrError> {
        let current = Self::revision(&self.directory, self.languages.split('+'), self.implicit_osd)?;
        if current != self.revision {
            return Err(OcrError::ProcessingFailed(
                "Tesseract script retry models changed during OCR".into(),
            ));
        }
        Ok(())
    }

    fn revision<'a>(
        directory: &str,
        languages: impl IntoIterator<Item = &'a str>,
        implicit_osd: bool,
    ) -> Result<blake3::Hash, OcrError> {
        let root = Path::new(directory);
        let mut hash = hash_language_models(root, languages, hash_model_file)?;
        if implicit_osd {
            let revision = crate::core::extractor::cacheability::osd_revision_with(root, hash_model_file)
                .map_err(|error| engine_error("fingerprint implicit OSD model", error))?;
            hash.update(revision.as_bytes());
        }
        Ok(hash.finalize())
    }

    fn hash_into(&self, hash: &mut blake3::Hasher) {
        for value in [&self.directory, &self.languages] {
            hash.update(&(value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
        hash.update(self.revision.as_bytes());
    }
}

pub(super) fn hash_language_models<'a>(
    root: &Path,
    languages: impl IntoIterator<Item = &'a str>,
    read_hash: impl Fn(&Path) -> crate::Result<String>,
) -> Result<blake3::Hasher, OcrError> {
    let mut hash = blake3::Hasher::new();
    for language in languages {
        let path = root.join(format!("{language}.traineddata"));
        let revision = read_hash(&path)
            .map_err(|error| engine_error(&format!("fingerprint model '{}'", path.display()), error))?;
        hash.update(revision.as_bytes());
    }
    Ok(hash)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RetryModels {
    pub(super) han: Option<HanModels>,
    pub(super) khmer: Option<ModelSet>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HanModels {
    horizontal: Option<(ModelSet, ModelSet)>,
    vertical_japanese: Option<ModelSet>,
}

fn retry_directories(directories: &[String], han: &[String]) -> Option<(String, String)> {
    let osd_directory = first_directory_holding(directories, &["osd".to_string()])?;
    let han_directory = first_directory_holding(directories, han)?;
    Some((osd_directory, han_directory))
}

fn vertical_japanese_directory(directories: &[String], language: &str) -> Option<String> {
    if language != "eng" {
        return None;
    }
    first_directory_holding(directories, &[VERTICAL_JAPANESE_MODEL.to_owned()])
}

impl RetryModels {
    pub(super) fn from_snapshots(han: Option<HanModels>, khmer: Option<ModelSet>) -> Option<Self> {
        (han.is_some() || khmer.is_some()).then_some(Self { han, khmer })
    }

    #[cfg(test)]
    fn resolve(directories: &[String], language: &str, psm: u8) -> Result<Option<Self>, OcrError> {
        Ok(Self::from_snapshots(
            Self::resolve_han(directories, language, psm)?,
            Self::resolve_requested_khmer(directories, language, psm)?,
        ))
    }

    pub(super) fn resolve_requested_khmer(
        directories: &[String],
        language: &str,
        psm: u8,
    ) -> Result<Option<ModelSet>, OcrError> {
        if mixed_pdf_khmer_retry_requested() && may_retry_khmer(language) {
            Self::resolve_khmer(directories, language, psm)
        } else {
            Ok(None)
        }
    }

    pub(super) fn resolve_han(directories: &[String], language: &str, psm: u8) -> Result<Option<HanModels>, OcrError> {
        if !supports_retry_language(language) {
            return Ok(None);
        }
        let han = retry_list(language, HAN_MODEL);
        let horizontal = match retry_directories(directories, &han) {
            Some((osd, recognition)) => Some((
                ModelSet::snapshot(osd, &["osd".into()], false)?,
                ModelSet::snapshot(recognition, &han, matches!(psm, 0 | 1 | 12))?,
            )),
            None => None,
        };
        let vertical_japanese = match vertical_japanese_directory(directories, language)
            .filter(|_| supports_vertical_japanese(language, psm))
        {
            Some(directory) => Some(ModelSet::snapshot(directory, &[VERTICAL_JAPANESE_MODEL.into()], false)?),
            None => None,
        };
        Ok(
            (horizontal.is_some() || vertical_japanese.is_some()).then_some(HanModels {
                horizontal,
                vertical_japanese,
            }),
        )
    }

    fn resolve_khmer(directories: &[String], language: &str, psm: u8) -> Result<Option<ModelSet>, OcrError> {
        let languages = retry_list(language, KHMER_MODEL);
        let implicit_osd = matches!(psm, 0 | 1 | 12);
        let mut required = languages.clone();
        if implicit_osd {
            required.push("osd".to_owned());
        }
        let Some(directory) = first_directory_holding(directories, &required) else {
            return Ok(None);
        };
        ModelSet::snapshot(directory, &languages, implicit_osd).map(Some)
    }

    pub(super) fn hash_into(&self, hash: &mut blake3::Hasher) {
        if let Some(han) = &self.han {
            if let Some((osd, recognition)) = &han.horizontal {
                osd.hash_into(hash);
                recognition.hash_into(hash);
            }
            if let Some(vertical) = &han.vertical_japanese {
                hash.update(b"vertical-japanese-v1");
                vertical.hash_into(hash);
            }
        }
        if let Some(khmer) = &self.khmer {
            hash.update(b"khm");
            khmer.hash_into(hash);
        }
    }
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

pub(super) fn first_read_suggests_vertical_japanese(
    api: &TesseractAPI,
    tsv: Option<&str>,
    config: &crate::ocr::types::TesseractConfig,
) -> bool {
    if !supports_vertical_japanese(&config.language, config.psm) {
        return false;
    }
    let check = |tsv: &str| vertical_japanese::first_read_candidate(&parse_words(tsv), config.psm);
    tsv.map_or_else(|| api.get_tsv_text(0).is_ok_and(|tsv| check(&tsv)), check)
}

pub(super) fn first_read_suggests_han(api: &TesseractAPI, tsv: Option<&str>) -> bool {
    if let Some(tsv) = tsv {
        return plausible_han(
            parse_words(tsv)
                .iter()
                .map(|word| (&*word.text, word.conf, word.width, word.height)),
        );
    }
    match api.get_iterator().and_then(|it| it.extract_all_words()) {
        Ok(read) => plausible_han(read.words.iter().map(|word| {
            (
                word.text.as_str(),
                f64::from(word.confidence),
                i64::from(word.right) - i64::from(word.left),
                i64::from(word.bottom) - i64::from(word.top),
            )
        })),
        Err(error) => {
            tracing::debug!(%error, "Could not inspect first-pass words for Han script");
            false
        }
    }
}

fn plausible_han<'a>(words: impl IntoIterator<Item = (&'a str, f64, i64, i64)>) -> bool {
    const MAX_MISREAD_CONFIDENCE: f64 = 45.0;
    const MIN_WIDE_MISREADS: usize = 3;
    const MIN_WIDTH_TO_HEIGHT_PER_LETTER: f64 = 0.8;
    let mut wide_misreads = 0;
    for (text, confidence, width, height) in words {
        if text.chars().any(|c| script_letter("Han", c)) {
            return true;
        }
        if confidence >= MAX_MISREAD_CONFIDENCE || !confidence.is_finite() || height <= 0 {
            continue;
        }
        // Low-confidence marks can be failed Han reads even when their boxes are narrow. ~keep
        if width > 0 && text.chars().any(|c| !c.is_whitespace()) && !text.chars().any(char::is_alphanumeric) {
            return true;
        }
        // Han glyphs often get read as Latin noise with close to a square of ink per letter. ~keep
        let letters = text.chars().filter(|c| c.is_alphanumeric()).count() as f64;
        if letters >= 2.0
            && text.chars().any(is_latin)
            && width as f64 >= MIN_WIDTH_TO_HEIGHT_PER_LETTER * height as f64 * letters
        {
            wide_misreads += 1;
            if wide_misreads >= MIN_WIDE_MISREADS {
                return true;
            }
        }
    }
    false
}

/// The script OSD names on the first read's pixels.
fn osd_script(image: &xberg_tesseract::Pix, pass: &Pass<'_>, models: &ModelSet) -> Result<Option<String>, OcrError> {
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    pass.pool.with_engine(RetryStage::Osd, models, 3, |osd| {
        osd.set_page_seg_mode(TessPageSegMode::PSM_OSD_ONLY)
            .map_err(|e| engine_error("OSD mode", e))?;
        osd.set_image_2(image.as_ptr())
            .map_err(|e| engine_error("OSD image", e))?;
        osd.set_source_resolution(pass.source_dpi)
            .map_err(|e| engine_error("OSD resolution", e))?;
        retry_stage(|| {
            osd.detect_os()
                .map(|result| result.2)
                .map_err(|e| engine_error("OSD", e))
        })
    })
}

/// The second read used for a primary merge or an additive Khmer supplement.
pub(super) struct Outcome {
    script: String,
    model: &'static str,
    pub(super) merged: Merged,
    khmer_supplement: Option<Merged>,
}

impl Outcome {
    pub(super) fn replaces_primary(&self) -> bool {
        self.model != KHMER_MODEL && self.khmer_supplement.is_none()
    }
}

/// The models and settings of the page's first read, which the second read repeats.
pub(super) struct Pass<'a> {
    pub(super) config: &'a crate::ocr::types::TesseractConfig,
    pub(super) source_dpi: i32,
    pub(super) pool: &'a RetryApiPools,
}

fn engine_error(step: &str, error: impl std::fmt::Display) -> OcrError {
    OcrError::ProcessingFailed(format!("script retry: {step}: {error}"))
}

fn retry_stage<T>(action: impl FnOnce() -> Result<T, OcrError>) -> Result<Option<T>, OcrError> {
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    let result = action()?;
    Ok((!cacheability::ocr_stop_requested()).then_some(result))
}

/// Retry Han when same-page OSD identifies it or Japanese when the first read shows noisy
/// vertical columns. For a mixed PDF's Khmer candidate, corroborate
/// the supplemental read on the same pixels using a second segmentation mode.
/// Models come from the installed tessdata. If a model is absent, keep the first read. ~keep
pub(super) fn run(
    first: &TesseractAPI,
    first_tsv: &str,
    pass: &Pass<'_>,
    models: &RetryModels,
    validate_khmer: impl FnOnce() -> bool,
    validate_han: impl FnOnce() -> bool,
    on_error: impl Fn(&str, OcrError),
) -> Result<Option<Outcome>, OcrError> {
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    let started = std::time::Instant::now();
    let image = first
        .thresholded_image()
        .map_err(|e| engine_error("thresholded image", e))?;
    let Some(outcome) = run_on_image(&image, first_tsv, pass, models, validate_khmer, validate_han, on_error)? else {
        return Ok(None);
    };
    let Outcome {
        script, model, merged, ..
    } = &outcome;
    tracing::debug!(
        script,
        model,
        elapsed_ms = started.elapsed().as_millis() as u64,
        lines_admitted = merged.admitted,
        lines_refused = merged.refused,
        words_added = merged.added,
        "script retry read the page again"
    );
    Ok(Some(outcome))
}

fn run_on_image(
    image: &xberg_tesseract::Pix,
    first_tsv: &str,
    pass: &Pass<'_>,
    models: &RetryModels,
    validate_khmer: impl FnOnce() -> bool,
    validate_han: impl FnOnce() -> bool,
    on_error: impl Fn(&str, OcrError),
) -> Result<Option<Outcome>, OcrError> {
    read_script_recoveries(
        || match &models.khmer {
            Some(khmer) => Ok(read_khmer_supplement(image, pass, khmer)?
                .and_then(|supplement| new_khmer_occurrences(supplement, &parse_words(first_tsv)))),
            None => Ok(None),
        },
        validate_khmer,
        &on_error,
        || {
            let han = read_han(image, first_tsv, pass, models, &on_error)?;
            Ok(han.filter(|_| validate_han()))
        },
    )
}

fn read_script_recoveries(
    read_khmer: impl FnOnce() -> Result<Option<Merged>, OcrError>,
    validate_khmer: impl FnOnce() -> bool,
    on_error: impl Fn(&str, OcrError),
    read_han: impl FnOnce() -> Result<Option<Outcome>, OcrError>,
) -> Result<Option<Outcome>, OcrError> {
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    let khmer = match read_khmer() {
        Ok(supplement) => supplement,
        Err(error) => {
            on_error(KHMER_MODEL, error);
            None
        }
    };
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    let han = match read_han() {
        Ok(han) => han,
        Err(error) => {
            on_error(HAN_MODEL, error);
            None
        }
    };
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    // Han runs after Khmer and may outlive the model revision used by the earlier read. ~keep
    let khmer = khmer.filter(|_| validate_khmer());
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    if let Some(mut outcome) = han {
        outcome.khmer_supplement = khmer;
        return Ok(Some(outcome));
    }
    Ok(khmer.map(|merged| Outcome {
        script: "Khmer".into(),
        model: KHMER_MODEL,
        merged,
        khmer_supplement: None,
    }))
}

fn read_han(
    image: &xberg_tesseract::Pix,
    first_tsv: &str,
    pass: &Pass<'_>,
    models: &RetryModels,
    on_error: impl Fn(&str, OcrError),
) -> Result<Option<Outcome>, OcrError> {
    let Some(han) = &models.han else {
        return Ok(None);
    };
    let first = parse_words(first_tsv);
    if let Some(vertical) = &han.vertical_japanese {
        match vertical_japanese::read(image, pass, vertical, &first) {
            Ok(Some(outcome)) => return Ok(Some(outcome)),
            Ok(None) => {}
            Err(error) => on_error(VERTICAL_JAPANESE_MODEL, error),
        }
    }
    read_horizontal_han(image, pass, han.horizontal.as_ref(), first)
}

fn read_horizontal_han(
    image: &xberg_tesseract::Pix,
    pass: &Pass<'_>,
    horizontal: Option<&(ModelSet, ModelSet)>,
    first: Vec<TsvWord>,
) -> Result<Option<Outcome>, OcrError> {
    let Some((osd, recognition)) = horizontal else {
        return Ok(None);
    };
    let Some(script) = osd_script(image, pass, osd)? else {
        return Ok(None);
    };
    let Some(model) = model_for_script(&script) else {
        return Ok(None);
    };
    let Some(retry_words) = read_retry_words(image, pass, recognition, RetryStage::Han, pass.config.psm)? else {
        return Ok(None);
    };
    let merged = merge(first, retry_words, &script, RETRY_MIN_CONF_GAIN);
    Ok(Some(Outcome {
        script,
        model,
        merged,
        khmer_supplement: None,
    }))
}

fn read_khmer_supplement(
    image: &xberg_tesseract::Pix,
    pass: &Pass<'_>,
    models: &ModelSet,
) -> Result<Option<Merged>, OcrError> {
    let Some(retry) = read_retry_words(image, pass, models, RetryStage::Khmer, pass.config.psm)? else {
        return Ok(None);
    };
    let alternate_psm = if pass.config.psm == 6 { 11 } else { 6 };
    let Some(alternate) = read_retry_words(image, pass, models, RetryStage::Khmer, alternate_psm)? else {
        return Ok(None);
    };
    Ok(corroborated_khmer(retry, &alternate))
}

fn read_retry_words(
    image: &xberg_tesseract::Pix,
    pass: &Pass<'_>,
    models: &ModelSet,
    stage: RetryStage,
    psm: u8,
) -> Result<Option<Vec<TsvWord>>, OcrError> {
    if cacheability::ocr_stop_requested() {
        return Ok(None);
    }
    pass.pool.with_engine(stage, models, pass.config.oem, |retry| {
        retry
            .set_page_seg_mode(TessPageSegMode::from_int(i32::from(psm)))
            .map_err(|e| engine_error("retry mode", e))?;
        super::config::apply_tesseract_variables(retry, pass.config)?;
        retry
            .set_image_2(image.as_ptr())
            .map_err(|e| engine_error("retry image", e))?;
        retry
            .set_source_resolution(pass.source_dpi)
            .map_err(|e| engine_error("retry resolution", e))?;
        if retry_stage(|| retry.recognize().map_err(|e| engine_error("retry recognize", e)))?.is_none() {
            return Ok(None);
        }
        let tsv = retry.get_tsv_text(0).map_err(|e| engine_error("retry word table", e))?;
        Ok(Some(parse_words(&tsv)))
    })
}

/// Record the outcome in the result metadata: the merge counts and, for a page that took the
/// retry, the merged confidence.
pub(super) fn record(metadata: &mut HashMap<String, serde_json::Value>, outcome: &Outcome) {
    let merged = &outcome.merged;
    metadata.insert(
        METADATA_KEY.to_string(),
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
    if let Some(khmer) = &outcome.khmer_supplement {
        let han = merged.han_supplement();
        let khmer = khmer.text();
        let text = if han.is_empty() {
            khmer
        } else {
            format!("{han}\n\n{khmer}")
        };
        metadata.insert(KHMER_SUPPLEMENT_KEY.to_string(), text.into());
    } else if outcome.model == KHMER_MODEL {
        metadata.insert(KHMER_SUPPLEMENT_KEY.to_string(), merged.text().into());
    }
    if outcome.replaces_primary() && merged.changed() {
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

/// A protected box can be replaced only when it contains a letter of the target script
/// and no competing letters or digits. Punctuation may accompany the target letters. ~keep
fn replaced_by_script_letters(word: &TsvWord, retry: &[TsvWord], script: &str) -> bool {
    let mut target_letter = false;
    for character in retry
        .iter()
        .filter(|candidate| x_overlap(word, candidate) > 0 && y_overlap(word.top, word.bottom(), candidate) > 0)
        .flat_map(|candidate| candidate.text.chars())
    {
        if script_letter(script, character) {
            target_letter = true;
        } else if character.is_alphanumeric() {
            return false;
        }
    }
    target_letter
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
                && y_overlap(word.top, word.bottom(), candidate) > 0
        });
        match found {
            Some((index, _)) => used[index] = true,
            None if replaced_by_script_letters(word, retry, script) => {}
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
    pub(super) admitted_script_lines: Vec<String>,
}

impl Merged {
    fn han_supplement(&self) -> String {
        self.admitted_script_lines.join("\n")
    }

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

// Pair equal text at overlapping positions. Maximize the number of pairs, using
// overlap and then source order to make contested assignments deterministic. ~keep
fn matched_primary_occurrences<'a>(
    primary: impl IntoIterator<Item = &'a TsvWord>,
    retry: impl IntoIterator<Item = &'a TsvWord>,
) -> Vec<bool> {
    let mut by_text: HashMap<&str, Vec<(usize, &TsvWord)>> = HashMap::new();
    let mut primary_count = 0;
    for word in primary {
        by_text
            .entry(occurrence_text(&word.text))
            .or_default()
            .push((primary_count, word));
        primary_count += 1;
    }
    let mut edges = Vec::new();
    let mut matched = Vec::new();
    for (index, word) in retry.into_iter().enumerate() {
        matched.push(false);
        if let Some(originals) = by_text.get(occurrence_text(&word.text)) {
            for &(original_index, original) in originals {
                let width = x_overlap(word, original);
                let height = y_overlap(word.top, word.bottom(), original);
                if width > 0 && height > 0 {
                    edges.push((std::cmp::Reverse(width.saturating_mul(height)), index, original_index));
                }
            }
        }
    }
    edges.sort_unstable();
    let mut choices = vec![Vec::new(); matched.len()];
    let mut owners = vec![None; primary_count];
    for (_, retry, primary) in edges {
        choices[retry].push(primary);
        if !matched[retry] && owners[primary].is_none() {
            matched[retry] = true;
            owners[primary] = Some(retry);
        }
    }
    for (retry, matched) in matched.iter_mut().enumerate() {
        if !*matched && !choices[retry].is_empty() {
            *matched = augment_occurrence_pairing(retry, &choices, &mut owners);
        }
    }
    matched
}

// Visit each vertex at most once per search. The queue and predecessors bound
// working space by the number of occurrences and avoid recursive reassignment. ~keep
fn augment_occurrence_pairing(start: usize, choices: &[Vec<usize>], owners: &mut [Option<usize>]) -> bool {
    let mut queue = vec![(start, None)];
    let mut visited = vec![false; choices.len()];
    let mut previous = vec![None; owners.len()];
    visited[start] = true;
    let mut cursor = 0;
    while let Some(&(retry, incoming)) = queue.get(cursor) {
        cursor += 1;
        for &primary in &choices[retry] {
            if previous[primary].is_some() {
                continue;
            }
            previous[primary] = Some((retry, incoming));
            if let Some(owner) = owners[primary] {
                if !visited[owner] {
                    visited[owner] = true;
                    queue.push((owner, Some(primary)));
                }
                continue;
            }
            let mut free = Some(primary);
            while let Some(slot) = free {
                let (retry, incoming) = previous[slot].expect("augmenting path has a predecessor");
                owners[slot] = Some(retry);
                free = incoming;
            }
            return true;
        }
    }
    false
}

fn occurrence_text(text: &str) -> &str {
    // Khmer vowel signs are combining marks and can be the final code point. ~keep
    text.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && !('\u{1780}'..='\u{17ff}').contains(&c))
}

fn newly_admitted_script_lines(
    primary: &[((i64, i64), Vec<TsvWord>)],
    admitted: &[((i64, i64), Vec<TsvWord>)],
    script: &str,
) -> Vec<String> {
    let matches = matched_primary_occurrences(
        primary.iter().flat_map(|(_, words)| words),
        admitted.iter().flat_map(|(_, words)| words),
    );
    let mut matches = matches.into_iter();
    admitted
        .iter()
        .filter_map(|(_, words)| {
            let text = words
                .iter()
                .zip(matches.by_ref())
                .filter(|(_, matched)| !matched)
                .map(|(word, _)| {
                    word.text
                        .chars()
                        .filter(|&c| script_letter(script, c))
                        .collect::<String>()
                })
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            (!text.is_empty()).then_some(text)
        })
        .collect()
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
    let mut admitted_script = Vec::new();
    for (line, mut candidates) in first_lines.into_iter().zip(by_line) {
        let position = (line.top, line.left);
        match admit_line(&line.words, &candidates, script, margin) {
            Ok(gain) => {
                tracing::debug!(gain, "script retry: line takes the retry's read");
                merged.admitted += 1;
                reading_order(&mut candidates);
                admitted_script.push((position, candidates.clone()));
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
                admitted_script.push(((line.top, line.left), line.words.clone()));
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
        admitted_script.sort_by_key(|(position, _)| *position);
        merged.admitted_script_lines = newly_admitted_script_lines(&kept, &admitted_script, script);
    }
    lines.sort_by_key(|(position, _)| *position);
    merged.lines = lines.into_iter().map(|(_, words)| words).collect();
    merged
}

fn is_khmer_letter(character: char) -> bool {
    ('\u{1780}'..='\u{17a2}').contains(&character)
}

fn eligible_khmer_word(word: &TsvWord) -> bool {
    const MIN_CONFIDENCE: f64 = 60.0;
    word.conf >= MIN_CONFIDENCE
        && word.width > 0
        && word.height > 0
        && !is_protected_text(&word.text)
        && word.text.chars().any(is_khmer_letter)
}

fn khmer_boxes_agree(first: &TsvWord, second: &TsvWord) -> bool {
    x_overlap(first, second) > first.width.max(second.width) / 2
        && y_overlap(first.top, first.bottom(), second) > first.height.max(second.height) / 2
}

fn unique_khmer_matches(retry: &[TsvWord], alternate: &[TsvWord]) -> Vec<bool> {
    let mut alternate_by_text: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, word) in alternate
        .iter()
        .enumerate()
        .filter(|(_, word)| eligible_khmer_word(word))
    {
        alternate_by_text.entry(&word.text).or_default().push(index);
    }
    let mut matched_by = vec![0usize; alternate.len()];
    let unique_candidates: Vec<_> = retry
        .iter()
        .map(|word| {
            let mut candidates = alternate_by_text
                .get(word.text.as_str())?
                .iter()
                .copied()
                .filter(|&index| khmer_boxes_agree(word, &alternate[index]));
            let first = candidates.next()?;
            matched_by[first] += 1;
            let mut unique = true;
            for index in candidates {
                matched_by[index] += 1;
                unique = false;
            }
            unique.then_some(first)
        })
        .collect();
    unique_candidates
        .into_iter()
        .map(|index| index.is_some_and(|index| matched_by[index] == 1))
        .collect()
}

fn new_khmer_occurrences(mut supplement: Merged, primary: &[TsvWord]) -> Option<Merged> {
    let mut matches = matched_primary_occurrences(primary, supplement.words()).into_iter();
    for line in &mut supplement.lines {
        line.retain(|_| matches.next() == Some(false));
    }
    supplement.lines.retain(|line| !line.is_empty());
    supplement.added = supplement.lines.iter().map(Vec::len).sum();
    (supplement.added > 0).then_some(supplement)
}

fn corroborated_khmer(mut retry: Vec<TsvWord>, alternate: &[TsvWord]) -> Option<Merged> {
    const MIN_MATCHED_LINES: usize = 2;
    const MIN_MATCHED_WORDS_PER_LINE: usize = 2;

    retry.retain(eligible_khmer_word);
    let matched = unique_khmer_matches(&retry, alternate);
    let words = retry
        .into_iter()
        .zip(matched)
        .filter_map(|(word, matched)| matched.then_some(word));
    let lines: Vec<_> = lines_of(words.collect()).into_iter().map(|line| line.words).collect();
    let matched_lines = lines
        .iter()
        .filter(|line| line.len() >= MIN_MATCHED_WORDS_PER_LINE)
        .count();
    (matched_lines >= MIN_MATCHED_LINES).then(|| Merged {
        added: lines.iter().map(Vec::len).sum(),
        lines,
        ..Merged::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAN_LINE: &str = "今天天气很好我们去公园散步";

    #[test]
    fn han_plausibility_requires_script_evidence() {
        let read = |words: &[(&str, f64, i64, i64)]| {
            plausible_han(
                line(words, 1, 0)
                    .iter()
                    .map(|word| (&*word.text, word.conf, word.width, word.height)),
            )
        };
        assert!(!read(&[]));
        assert!(!read(&[
            ("2026", 10.0, 160, 40),
            ("Latin", 10.0, 105, 40),
            ("noise", 12.0, 95, 40)
        ]));
        assert!(!read(&[("POS", 20.0, 130, 40), ("HEADS", 28.0, 175, 40)]));
        assert!(read(&[
            ("POS", 20.0, 130, 40),
            ("HEADS", 28.0, 175, 40),
            ("MHS", 0.0, 140, 40)
        ]));
        assert!(read(&[("中文", 70.0, 80, 40)]));
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    fn assert_retry_worker_stops(token: crate::cancellation::CancellationToken, soft_stop: bool, stop_after: usize) {
        let stop = || {
            if soft_stop {
                token.request_stop();
            } else {
                token.cancel();
            }
        };
        let mut stages_run = 0;
        if stop_after == 0 {
            stop();
        }
        let mut native_stage = || {
            stages_run += 1;
            if stages_run == stop_after {
                stop();
            }
            Ok(())
        };
        let result = retry_stage(&mut native_stage)
            .expect("OSD stage")
            .and_then(|()| retry_stage(native_stage).expect("recognition stage"));
        assert_eq!(result, None, "a stopped worker must discard the retry");
        assert_eq!(stages_run, stop_after);
        assert!(!cacheability::can_publish_outer_result());

        let directory = tempfile::tempdir().expect("OCR cache directory");
        let cache = crate::ocr::cache::OcrCache::new(Some(directory.path().into())).expect("OCR cache");
        cache
            .set_cached_result("page", "tesseract", "retry", None, &Default::default())
            .expect("cancellation is a cache bypass");
        assert_eq!(
            directory.path().read_dir().expect("cache files").count(),
            0,
            "cancellation must prevent publication and leave no temporary entry"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn retry_worker_skips_stages_after_cancellation() {
        use crate::cancellation::CancellationToken;

        for soft_stop in [false, true] {
            for stop_after in 0..=2 {
                let token = CancellationToken::new();
                let worker_token = token.clone();
                cacheability::scope(async {
                    cacheability::with_ocr_cancellation(Some(&token), async move {
                        let worker = cacheability::blocking(move || {
                            assert_retry_worker_stops(worker_token, soft_stop, stop_after);
                        });
                        tokio::task::spawn_blocking(worker).await.expect("OCR worker");
                    })
                    .await;
                    assert!(
                        !cacheability::can_publish_outer_result(),
                        "the engine's enclosing scope must stay blocked"
                    );
                })
                .await;
                assert!(cacheability::can_publish_outer_result());
            }
        }
    }

    #[test]
    fn retry_requires_han_in_the_protected_word_box() {
        let first = line(&[("Ref", 20.0, 0, 25), ("漢字", 20.0, 30, 50)], 0, 0);
        for replacement in ["!", "Ж", "漢Ж"] {
            let retry = line(&[(replacement, 95.0, 0, 25), ("漢字測試", 95.0, 30, 50)], 0, 0);
            assert!(
                matches!(admit_line(&first, &retry, HAN, RETRY_MIN_CONF_GAIN), Err(Refusal::Lost(word)) if word == "Ref"),
                "{replacement} must not displace the protected token"
            );
        }
        let retry = line(&[("漢。", 95.0, 0, 25), ("漢字測試", 95.0, 30, 50)], 0, 0);
        assert!(admit_line(&first, &retry, HAN, RETRY_MIN_CONF_GAIN).is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn retry_worker_skips_stages_when_deadline_has_passed_before_stop_signal() {
        let token = crate::cancellation::CancellationToken::new();
        token.set_deadline(std::time::Instant::now());
        assert!(!token.is_cancelled());
        assert!(!token.stop_requested());
        cacheability::with_ocr_cancellation(Some(&token), async {
            tokio::task::spawn_blocking(cacheability::blocking(|| {
                let stage = retry_stage(|| Ok("Han")).expect("OSD stage");
                assert_eq!(stage, None, "the expired worker must skip OSD");
                assert!(!cacheability::can_publish_outer_result());
            }))
            .await
            .expect("OCR worker");
        })
        .await;
    }

    #[test]
    fn retry_rejects_han_in_a_vertically_disjoint_protected_word_box() {
        let first = line(&[("Ref", 20.0, 0, 25), ("漢字", 20.0, 30, 50)], 0, 0);
        let mut retry = line(&[("漢。", 95.0, 0, 25), ("漢字測試", 95.0, 30, 50)], 0, 0);
        assert!(admit_line(&first, &retry, HAN, RETRY_MIN_CONF_GAIN).is_ok());

        retry[0].top = first[0].bottom();
        assert!(
            matches!(admit_line(&first, &retry, HAN, RETRY_MIN_CONF_GAIN), Err(Refusal::Lost(word)) if word == "Ref"),
            "Han from a disjoint row cannot replace the protected token"
        );
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
    fn a_line_of_marks_passes_the_gate_and_is_replaced_by_the_han_read() {
        let han = line(&[("今天天", 92.0, 0, 120), ("气", 92.0, 140, 160)], 1, 100);
        for (first_width, second_width) in [(120, 160), (3, 8)] {
            let marks = line(&[("|", 20.0, 0, first_width), ("~", 25.0, 140, second_width)], 1, 100);
            assert!(plausible_han(marks.iter().map(|word| (
                &*word.text,
                word.conf,
                word.width,
                word.height
            ))));
            let merged = merge_han(marks, han.clone());
            assert_eq!(joined(&merged), "今天天 气");
            assert_eq!((merged.admitted, merged.refused, merged.added), (1, 0, 0));
        }
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
            khmer_supplement: None,
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
        use crate::ocr::types::TesseractConfig;
        assert!(TesseractConfig::default().enable_table_detection);
        for table_detection in [true, false] {
            for (format, supported) in [("text", true), ("markdown", true), ("hocr", false), ("tsv", false)] {
                assert_eq!(
                    supports_retry(&TesseractConfig {
                        enable_table_detection: table_detection,
                        output_format: format.into(),
                        ..Default::default()
                    }),
                    supported
                );
            }
        }
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
    fn retry_resolves_osd_and_the_complete_language_set_in_priority_order() {
        let root = tempfile::tempdir().unwrap();
        let absent = root.path().join("absent");
        let first = root.path().join("first");
        let second = root.path().join("second");
        let last = root.path().join("last");
        for dir in [&first, &second, &last] {
            std::fs::create_dir(dir).unwrap();
        }
        let write = |dir: &Path, lang: &str| {
            std::fs::write(dir.join(format!("{lang}.traineddata")), lang.as_bytes()).unwrap();
        };
        write(&first, "eng");
        write(&first, "osd");
        write(&second, "chi_sim");
        for language in ["eng", "deu", "chi_sim", "osd"] {
            write(&last, language);
        }
        let directories = [&absent, &first, &second, &last].map(|dir| dir.to_str().unwrap().to_owned());
        assert!(RetryModels::resolve(&directories[..3], "eng+deu", 3).unwrap().is_none());
        let models = RetryModels::resolve(&directories, "eng+deu", 3)
            .unwrap()
            .unwrap()
            .han
            .unwrap();
        let (osd, recognition) = models.horizontal.unwrap();
        assert_eq!(osd.directory, directories[1]);
        assert_eq!(osd.languages, "osd");
        assert_eq!(recognition.directory, directories[3]);
        assert_eq!(recognition.languages, "eng+deu+chi_sim");
        assert!(!absent.exists());
    }

    #[test]
    fn broken_vertical_japanese_engine_falls_through_to_working_horizontal_han() {
        let fixture = crate::utils::read_test_fixture("images/chi_sim_image.jpeg");
        let directories = tessdata_search_dirs(None, &TessdataEnv::from_process());
        let horizontal = RetryModels::resolve_han(&directories, "eng", 6)
            .unwrap()
            .and_then(|models| models.horizontal);
        if std::env::var_os("XBERG_REQUIRE_VERTICAL_JAPANESE_FIXTURES").is_some() {
            assert!(
                horizontal.is_some() && fixture.is_some(),
                "qualification requires eng, osd, chi_sim and the Han image"
            );
        }
        let (Some(bytes), Some(horizontal)) = (fixture, horizontal) else {
            return;
        };
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("jpn_vert.traineddata"), b"invalid model").unwrap();
        let vertical_japanese =
            ModelSet::snapshot(root.path().to_str().unwrap().into(), &["jpn_vert".into()], false).unwrap();
        let models = RetryModels {
            han: Some(HanModels {
                horizontal: Some(horizontal),
                vertical_japanese: Some(vertical_japanese),
            }),
            khmer: None,
        };
        let pixels = image::load_from_memory(&bytes).unwrap().to_rgb8();
        let pix = xberg_tesseract::Pix::from_raw_rgb(pixels.as_raw(), pixels.width(), pixels.height()).unwrap();
        let config = crate::ocr::types::TesseractConfig {
            psm: 11,
            ..Default::default()
        };
        let pool = super::super::api_pool::TesseractApiPool::new();
        let pass = Pass {
            config: &config,
            source_dpi: 300,
            pool: pool.retry(),
        };
        let mut tsv = String::from("header");
        for index in 1..=30 {
            tsv.push_str(&format!("\n5\t1\t1\t1\t{index}\t1\t0\t0\t20\t20\t10\tt?"));
        }
        let failed = std::cell::Cell::new(0);
        let result = read_han(&pix, &tsv, &pass, &models, |model, _| {
            assert_eq!(model, VERTICAL_JAPANESE_MODEL);
            failed.set(failed.get() + 1);
        })
        .unwrap()
        .expect("horizontal Han fallback succeeds");
        assert_eq!(failed.get(), 1);
        assert_eq!(result.model, HAN_MODEL);
    }

    #[test]
    fn vertical_retry_models_are_independent_of_horizontal_han_and_track_revision() {
        let root = tempfile::tempdir().unwrap();
        let directories = [root.path().to_str().unwrap().to_owned()];
        let model = root.path().join("jpn_vert.traineddata");
        assert!(RetryModels::resolve_han(&directories, "eng", 11).unwrap().is_none());
        std::fs::write(&model, b"japanese-v1").unwrap();
        let first = RetryModels::resolve_han(&directories, "eng", 11).unwrap().unwrap();
        assert!(first.horizontal.is_none());
        assert_eq!(first.vertical_japanese.as_ref().unwrap().languages, "jpn_vert");
        assert!(RetryModels::resolve_han(&directories, "eng", 6).unwrap().is_none());
        assert!(RetryModels::resolve_han(&directories, "jpn_vert", 5).unwrap().is_none());
        std::fs::write(model, b"japanese-v2").unwrap();
        let second = RetryModels::resolve_han(&directories, "eng", 11).unwrap().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn khmer_retry_requires_an_installed_complete_model_set() {
        let root = tempfile::tempdir().unwrap();
        let directories = [root.path().to_str().unwrap().to_owned()];
        for name in ["eng", "deu"] {
            std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
        }
        assert!(
            RetryModels::resolve_khmer(&directories, "deu+eng", 3)
                .unwrap()
                .is_none()
        );
        std::fs::write(root.path().join("khm.traineddata"), b"khm").unwrap();
        let models = RetryModels::resolve_khmer(&directories, "deu+eng", 3).unwrap().unwrap();
        assert_eq!(models.languages, "deu+eng+khm");
        assert!(
            RetryModels::resolve_khmer(&directories, "deu+eng", 1)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn khmer_supplement_requires_matching_words_on_multiple_same_page_lines() {
        let first_line = line(
            &[("កម្ពុជា", 92.0, 10, 30), ("សួស្តី", 90.0, 50, 30), ("Ref3124", 99.0, 90, 30)],
            1,
            10,
        );
        let second_line = line(
            &[("អរគុណ", 92.0, 10, 30), ("ភ្នំពេញ", 90.0, 50, 30), ("3124", 99.0, 90, 30)],
            2,
            60,
        );
        let retry = [first_line.clone(), second_line].concat();
        assert!(corroborated_khmer(retry.clone(), &first_line).is_none());
        let mut displaced = retry.clone();
        for word in &mut displaced {
            word.left += 150;
        }
        assert!(corroborated_khmer(retry.clone(), &displaced).is_none());
        let mut noisy_retry = retry.clone();
        noisy_retry.extend(line(&[("មិនស្គាល់", 95.0, 130, 30)], 1, 10));
        noisy_retry.extend(line(&[("រ", 90.0, 10, 30)], 3, 110));
        let supplement = corroborated_khmer(noisy_retry, &retry).unwrap();
        assert_eq!(supplement.text(), "កម្ពុជា សួស្តី\nអរគុណ ភ្នំពេញ");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn khmer_error_reports_failure_and_only_runs_han_before_stop() {
        use crate::cancellation::CancellationToken;

        for stop in [None, Some("cancel"), Some("soft stop"), Some("deadline")] {
            let token = CancellationToken::new();
            cacheability::with_ocr_cancellation(Some(&token), async {
                let error = std::cell::RefCell::new(None);
                let result = read_script_recoveries(
                    || {
                        match stop {
                            Some("cancel") => token.cancel(),
                            Some("soft stop") => token.request_stop(),
                            Some("deadline") => token.set_deadline(std::time::Instant::now()),
                            _ => {}
                        }
                        Err(engine_error("retry recognize", "injected Khmer error"))
                    },
                    || true,
                    |model, failure| {
                        assert_eq!(model, KHMER_MODEL);
                        *error.borrow_mut() = Some(failure);
                    },
                    || {
                        assert!(stop.is_none(), "Han ran after the operation stopped");
                        Ok(Some(Outcome {
                            script: "Han".into(),
                            model: HAN_MODEL,
                            merged: Merged::default(),
                            khmer_supplement: None,
                        }))
                    },
                )
                .unwrap();
                assert_eq!(result.map(|outcome| outcome.model), stop.is_none().then_some(HAN_MODEL));
                assert!(error.into_inner().unwrap().to_string().contains("injected Khmer error"));
            })
            .await;
        }
    }

    #[test]
    fn successful_khmer_does_not_skip_han_or_replace_its_merge() {
        let result = read_script_recoveries(
            || {
                Ok(Some(Merged {
                    lines: vec![line(&[("កម្ពុជា", 95.0, 0, 50)], 1, 0)],
                    ..Merged::default()
                }))
            },
            || true,
            |_, error| panic!("{error}"),
            || {
                Ok(Some(Outcome {
                    script: "Han".into(),
                    model: HAN_MODEL,
                    merged: Merged {
                        lines: vec![line(&[("漢字", 95.0, 0, 50), ("Ref3124", 95.0, 60, 50)], 1, 0)],
                        admitted: 1,
                        admitted_script_lines: vec!["漢字".into()],
                        ..Merged::default()
                    },
                    khmer_supplement: None,
                }))
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.merged.text(), "漢字 Ref3124");
        let mut metadata = HashMap::new();
        record(&mut metadata, &result);
        assert!(!result.replaces_primary());
        assert_eq!(metadata[KHMER_SUPPLEMENT_KEY], "漢字\n\nកម្ពុជា");
        assert_eq!(metadata[METADATA_KEY]["model"], HAN_MODEL);
    }

    #[test]
    fn khmer_revalidation_runs_after_han_and_discards_only_stale_khmer() {
        use std::cell::Cell;
        for drift in [false, true] {
            let han_ran = Cell::new(false);
            let result = read_script_recoveries(
                || Ok(Some(Merged::default())),
                || {
                    assert!(han_ran.get(), "Khmer revision must be checked after Han");
                    !drift
                },
                |_, error| panic!("{error}"),
                || {
                    han_ran.set(true);
                    Ok(Some(Outcome {
                        script: "Han".into(),
                        model: HAN_MODEL,
                        merged: Merged::default(),
                        khmer_supplement: None,
                    }))
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(result.model, HAN_MODEL);
            assert_eq!(result.khmer_supplement.is_some(), !drift);
        }
    }

    #[test]
    fn han_supplement_reassigns_contested_overlap() {
        let primary = line(&[("漢字", 20.0, 0, 50), ("漢字", 20.0, 45, 35)], 1, 0);
        let retry = line(&[("漢字", 95.0, 0, 65), ("漢字", 95.0, 0, 30)], 1, 0);
        assert_eq!(merge_han(primary, retry).han_supplement(), "");
    }

    #[test]
    fn khmer_supplement_reassigns_contested_overlap() {
        let primary = line(&[("កម្ពុជា", 20.0, 0, 50), ("កម្ពុជា", 20.0, 45, 35)], 1, 0);
        let retry = line(&[("កម្ពុជា", 95.0, 0, 55), ("កម្ពុជា", 95.0, 0, 30)], 1, 0);
        let supplement = Merged {
            lines: vec![retry],
            added: 2,
            ..Merged::default()
        };
        assert!(new_khmer_occurrences(supplement, &primary).is_none());
    }

    #[test]
    fn han_supplement_consumes_one_primary_for_two_overlapping_retry_occurrences() {
        let first = line(&[("漢字", 20.0, 0, 70)], 1, 0);
        let retry = line(&[("漢字", 95.0, 0, 40), ("漢字", 95.0, 35, 40)], 1, 0);
        assert_eq!(merge_han(first, retry).han_supplement(), "漢字");
    }

    #[test]
    fn khmer_supplement_excludes_existing_primary_occurrences() {
        let retry = [
            line(&[("កម្ពុជា", 95.0, 0, 50), ("សួស្តី", 95.0, 70, 50)], 1, 0),
            line(&[("កម្ពុជា", 95.0, 0, 50), ("អរគុណ", 95.0, 70, 50)], 2, 60),
        ]
        .concat();
        let primary = line(&[("កម្ពុជា", 20.0, 0, 50), ("សួស្តី", 30.0, 70, 50)], 1, 0);
        let result = new_khmer_occurrences(corroborated_khmer(retry.clone(), &retry).unwrap(), &primary).unwrap();
        assert_eq!(result.text(), "កម្ពុជា អរគុណ");
        assert!(new_khmer_occurrences(corroborated_khmer(retry.clone(), &retry).unwrap(), &retry).is_none());
    }

    #[test]
    fn han_supplement_omits_primary_han_but_keeps_new_occurrences() {
        let first = [
            line(&[("漢字", 20.0, 0, 35), ("|", 20.0, 50, 35)], 1, 0),
            line(&[("留存", 95.0, 0, 35)], 2, 60),
        ]
        .concat();
        let retry = line(&[("漢字", 95.0, 0, 35), ("漢字", 95.0, 50, 35)], 1, 0);
        let result = merge_han(first.clone(), retry);
        assert_eq!(result.han_supplement(), "漢字");
        assert_eq!(result.text(), "漢字 漢字\n留存");
        let only_retained = line(&[("漢字", 95.0, 0, 35)], 1, 0);
        assert!(merge_han(first, only_retained).han_supplement().is_empty());
    }

    #[test]
    fn khmer_corroboration_rejects_ambiguous_spatial_matches_in_both_directions() {
        let words = line(&[("កម្ពុជា", 92.0, 10, 30), ("កម្ពុជា", 92.0, 65, 30)], 1, 10);
        assert_eq!(unique_khmer_matches(&words, &words), [true, true]);
        let mut duplicate = words.clone();
        duplicate.push(words[0].clone());
        assert_eq!(unique_khmer_matches(&duplicate, &words), [false, true, false]);
        assert_eq!(unique_khmer_matches(&words, &duplicate), [false, true]);
    }

    #[test]
    fn khmer_hint_keeps_han_models_with_or_without_khmer_installed() {
        let root = tempfile::tempdir().unwrap();
        let directories = [root.path().to_str().unwrap().to_owned()];
        for name in ["eng", "osd", "chi_sim"] {
            std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
        }
        MIXED_PDF_KHMER_RETRY.sync_scope(true, || {
            let missing = RetryModels::resolve(&directories, "eng", 3).unwrap().unwrap();
            assert!(missing.khmer.is_none());
            assert_eq!(
                missing.han.as_ref().unwrap().horizontal.as_ref().unwrap().1.languages,
                "eng+chi_sim"
            );
            std::fs::write(root.path().join("khm.traineddata"), b"khm").unwrap();
            let installed = RetryModels::resolve(&directories, "eng", 3).unwrap().unwrap();
            assert_eq!(installed.khmer.unwrap().languages, "eng+khm");
            assert_eq!(installed.han, missing.han);
        });
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
