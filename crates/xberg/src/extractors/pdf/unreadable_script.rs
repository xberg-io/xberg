//! Keep a native text layer whose script the configured OCR languages cannot read.
//!
//! OCR with a model that does not cover a script reads that script as letters of another one, so
//! replacing a native page with such a read loses its text. A page with lost-glyph markers is
//! judged on script evidence: the scripts of its native letters against the scripts the OCR
//! languages cover. A page without markers is also judged on whether its signs attach to letters as
//! the script's orthography requires, because other text decoded through a wrong map reaches the
//! same code points.

use crate::core::config::{ExtractionConfig, VlmFallbackPolicy};
use crate::extractors::pdf::ocr::OcrFallbackDecision;
use crate::extractors::pdf::ocr::stage_model_inputs::StageModelInputs;
use crate::types::PageBoundary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Script {
    Latin,
    Greek,
    Cyrillic,
    Armenian,
    Hebrew,
    Arabic,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
    Thai,
    Lao,
    Tibetan,
    Myanmar,
    Georgian,
    Hangul,
    Ethiopic,
    Khmer,
    Han,
    Kana,
}

/// The script of a character by Unicode block, marks and signs of the script included; `None`
/// for digits, punctuation, symbols, private-use characters and scripts this module does not
/// tell apart.
fn script_of(character: char) -> Option<Script> {
    let code = character as u32;
    let script = match code {
        0x0041..=0x005A
        | 0x0061..=0x007A
        | 0x00C0..=0x00D6
        | 0x00D8..=0x00F6
        | 0x00F8..=0x024F
        | 0x1E00..=0x1EFF
        | 0xFB00..=0xFB06
        | 0xFF21..=0xFF3A
        | 0xFF41..=0xFF5A => Script::Latin,
        0x0370..=0x03FF | 0x1F00..=0x1FFF => Script::Greek,
        0x0400..=0x052F | 0x1C80..=0x1C8F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => Script::Cyrillic,
        0x0530..=0x058F => Script::Armenian,
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => Script::Hebrew,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => Script::Arabic,
        0x0900..=0x097F => Script::Devanagari,
        0x0980..=0x09FF => Script::Bengali,
        0x0A00..=0x0A7F => Script::Gurmukhi,
        0x0A80..=0x0AFF => Script::Gujarati,
        0x0B00..=0x0B7F => Script::Oriya,
        0x0B80..=0x0BFF => Script::Tamil,
        0x0C00..=0x0C7F => Script::Telugu,
        0x0C80..=0x0CFF => Script::Kannada,
        0x0D00..=0x0D7F => Script::Malayalam,
        0x0D80..=0x0DFF => Script::Sinhala,
        0x0E00..=0x0E7F => Script::Thai,
        0x0E80..=0x0EFF => Script::Lao,
        0x0F00..=0x0FFF => Script::Tibetan,
        0x1000..=0x109F => Script::Myanmar,
        0x10A0..=0x10FF | 0x1C90..=0x1CBF => Script::Georgian,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => Script::Hangul,
        0x1200..=0x137F => Script::Ethiopic,
        0x1780..=0x17FF | 0x19E0..=0x19FF => Script::Khmer,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF => Script::Han,
        0x3040..=0x30FF => Script::Kana,
        _ => return None,
    };
    Some(script)
}

/// The scripts a Tesseract model covers; `None` for a model this module does not know, which
/// means no claim can be made about what it reads.
fn language_scripts(model: &str) -> Option<&'static [Script]> {
    use Script::*;
    let scripts: &'static [Script] = match model.to_ascii_lowercase().as_str() {
        "eng" | "deu" | "fra" | "spa" | "ita" | "por" | "nld" | "pol" | "ces" | "slk" | "slv" | "hrv" | "hun"
        | "ron" | "tur" | "swe" | "nor" | "dan" | "fin" | "est" | "lav" | "lit" | "isl" | "afr" | "cat" | "eus"
        | "glg" | "gle" | "mlt" | "sqi" | "bos" | "ind" | "msa" | "vie" | "fil" | "swa" | "lat" | "srp_latn"
        | "uzb" | "aze" | "cym" | "ltz" | "oci" | "epo" | "latin" | "script/latin" => &[Latin],
        "khm" | "khmer" | "script/khmer" => &[Khmer],
        "chi_sim" | "chi_tra" | "hans" | "hant" | "script/hans" | "script/hant" | "script/han" => &[Han],
        "jpn" | "japanese" | "script/japanese" => &[Han, Kana],
        "kor" | "korean" | "script/hangul" | "hangul" => &[Hangul, Han],
        "ara" | "fas" | "urd" | "pus" | "uig" | "snd" | "arabic" | "script/arabic" => &[Arabic],
        "heb" | "yid" | "hebrew" | "script/hebrew" => &[Hebrew],
        "rus" | "ukr" | "bul" | "bel" | "srp" | "mkd" | "mon" | "kaz" | "kir" | "tgk" | "cyrillic"
        | "script/cyrillic" => &[Cyrillic],
        "ell" | "grc" | "greek" | "script/greek" => &[Greek],
        "hin" | "mar" | "nep" | "san" | "devanagari" | "script/devanagari" => &[Devanagari],
        "ben" | "asm" | "bengali" | "script/bengali" => &[Bengali],
        "pan" | "gurmukhi" | "script/gurmukhi" => &[Gurmukhi],
        "guj" | "gujarati" | "script/gujarati" => &[Gujarati],
        "ori" | "oriya" | "script/oriya" => &[Oriya],
        "tam" | "tamil" | "script/tamil" => &[Tamil],
        "tel" | "telugu" | "script/telugu" => &[Telugu],
        "kan" | "kannada" | "script/kannada" => &[Kannada],
        "mal" | "malayalam" | "script/malayalam" => &[Malayalam],
        "sin" | "sinhala" | "script/sinhala" => &[Sinhala],
        "tha" | "thai" | "script/thai" => &[Thai],
        "lao" | "script/lao" => &[Lao],
        "bod" | "dzo" | "tibetan" | "script/tibetan" => &[Tibetan],
        "mya" | "myanmar" | "script/myanmar" => &[Myanmar],
        "kat" | "kat_old" | "georgian" | "script/georgian" => &[Georgian],
        "hye" | "armenian" | "script/armenian" => &[Armenian],
        "amh" | "tir" | "ethiopic" | "script/ethiopic" => &[Ethiopic],
        _ => return None,
    };
    Some(scripts)
}

/// Fewest uncovered-script characters that can establish a layer's script. A letter-poor layer,
/// such as a numeric table that repeats one unit symbol (`μ`, `Ω`), holds a handful; the Khmer
/// fixtures hold 222 and 28. A title or certificate page of a dozen characters is still a page.
const MIN_SCRIPT_CHARS: usize = 8;

/// Share of the characters that no configured language covers that must be in a known script.
/// The rest are private-use, dingbat and symbol characters, which a mis-decode or a symbol font
/// produces; a layer that is mostly those is not a page in any script. The Khmer fixtures have
/// none of them (100%); 3000 private-use characters with one letter give 0.03%.
const MIN_KNOWN_SHARE: f64 = 0.9;

/// The scripts a page is written in when its layer is nearly all in scripts none of `languages`
/// covers, largest first, or `None` when the layer does not establish that.
///
/// Characters a configured language covers (Latin always, and each configured model's scripts)
/// are not counted: a name or URL in a Khmer page neither helps nor hurts. Of the rest, the
/// whole-page text excluding whitespace, lost-glyph markers, digits and punctuation, at least
/// [`MIN_SCRIPT_CHARS`] must be in uncovered known scripts and they must make up
/// [`MIN_KNOWN_SHARE`] of it. They must also outnumber the letters of covered scripts: a mostly
/// covered page (an English page with a few Greek symbols) is one OCR can read. A configured language this module does not know means no claim.
#[cfg(test)]
fn unreadable_scripts(text: &str, languages: &[String]) -> Option<Vec<Script>> {
    Some(
        uncovered_script_counts(text, languages)?
            .into_iter()
            .map(|(script, _)| script)
            .collect(),
    )
}

/// [`unreadable_scripts`] with the number of letters each script holds, largest first.
fn uncovered_script_counts(text: &str, languages: &[String]) -> Option<Vec<(Script, usize)>> {
    if languages.is_empty() {
        return None;
    }
    let mut covered = vec![Script::Latin];
    for model in languages.iter().flat_map(|language| language.split('+')) {
        covered.extend_from_slice(language_scripts(model)?);
    }
    let mut counts: Vec<(Script, usize)> = Vec::new();
    let mut unattributed = 0usize;
    let mut covered_chars = 0usize;
    for character in text.chars().filter(|character| {
        !character.is_whitespace()
            && *character != super::unmapped_glyphs::LOST_GLYPH_MARKER
            && !character.is_numeric()
            && !character.is_ascii_punctuation()
            && !matches!(*character as u32, 0x00A0..=0x00BF | 0x2000..=0x206F | 0x3000..=0x3003)
    }) {
        match script_of(character) {
            Some(script) if covered.contains(&script) => covered_chars += 1,
            Some(script) => match counts.iter_mut().find(|(known, _)| *known == script) {
                Some((_, count)) => *count += 1,
                None => counts.push((script, 1)),
            },
            None => unattributed += 1,
        }
    }
    let known: usize = counts.iter().map(|(_, count)| count).sum();
    if known < MIN_SCRIPT_CHARS
        || (known as f64) < MIN_KNOWN_SHARE * (known + unattributed) as f64
        || known <= covered_chars
    {
        return None;
    }
    counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    Some(counts)
}

/// The tessdata model that reads `script`, for the user-facing hint.
fn model_for(script: Script) -> &'static str {
    use Script::*;
    match script {
        Khmer => "khm",
        Han => "chi_sim",
        Kana => "jpn",
        Hangul => "kor",
        Arabic => "ara",
        Hebrew => "heb",
        Cyrillic => "rus",
        Greek => "ell",
        Devanagari => "hin",
        Bengali => "ben",
        Gurmukhi => "pan",
        Gujarati => "guj",
        Oriya => "ori",
        Tamil => "tam",
        Telugu => "tel",
        Kannada => "kan",
        Malayalam => "mal",
        Sinhala => "sin",
        Thai => "tha",
        Lao => "lao",
        Tibetan => "bod",
        Myanmar => "mya",
        Georgian => "kat",
        Armenian => "hye",
        Ethiopic => "amh",
        Latin => "eng",
    }
}

/// Whether the page is a scan or a page that carries one, by the rule the scanned-page detector
/// uses: pictures covering at least 80% of it, or covering a quarter with a text layer too short to
/// be the page's body. A page with a figure beside a full text layer is not.
fn page_is_scan(document: &xberg_native_pdf::PdfDocument, page_index: usize) -> bool {
    crate::pdf::native::guard_native_panic(
        || Ok::<_, String>(crate::pdf::scan_detect::full_page_raster_density(document, page_index).is_some()),
        |message| message,
    )
    .unwrap_or_else(|error| {
        tracing::debug!(%error, page_index, "script evidence assumes a scan: the page raster check failed");
        true
    })
}

/// The setting the languages of the first stage were taken from, which is what a caller edits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LanguageSource {
    OcrLanguage,
    StageLanguage,
    TesseractConfig,
}

/// The stage whose result a Tesseract route returns first, and where its languages come from.
struct FirstStage {
    languages: Vec<String>,
    source: LanguageSource,
}

/// Where `effective_tesseract_language_for` takes the languages from: `tesseract_config.language`
/// only while the language it is given is the default `eng`, otherwise the language itself.
fn language_source(
    language: &[String],
    stage_sets_language: bool,
    tesseract: Option<&crate::types::TesseractConfig>,
) -> LanguageSource {
    let default = language.len() == 1 && language[0] == crate::core::config::ocr::DEFAULT_OCR_LANGUAGE;
    if default && tesseract.is_some_and(|tesseract| !tesseract.language.is_empty()) {
        LanguageSource::TesseractConfig
    } else if stage_sets_language {
        LanguageSource::StageLanguage
    } else {
        LanguageSource::OcrLanguage
    }
}

/// The stage that reads the page first, or `None` when the route makes no claim about what it reads.
/// Only Tesseract stages make a claim: PaddleOCR, Sceptre, VLM and custom backends, an empty pipeline,
/// and a `vlm_fallback` other than `Disabled` are "no claim", so a route that can reach one is never
/// judged. The executor (`run_ocr_pipeline_stages`) runs the stages by descending priority, the first
/// listed first among equals, and returns the first result that clears `pipeline_min_quality`. The
/// first stage therefore decides when it succeeds and clears the gate: if it reads the script, OCR is
/// the better reader; if it does not, it produces the page's text and replaces the native layer.
///
/// Residual: when the first stage fails (a missing model), is unavailable or scores below
/// `pipeline_min_quality`, the executor hands the page to the next stage and returns the best result
/// of any stage. With `[khm, eng]` a failing `khm` stage therefore lets the `eng` stage replace the
/// Khmer layer, and the native layer is not kept for that page. Keeping it whenever any later stage
/// cannot read the script would instead keep it for every healthy `khm`-first pipeline (#2738).
fn first_stage(ocr: &crate::OcrConfig) -> Option<FirstStage> {
    if !ocr.enabled || ocr.vlm_fallback != VlmFallbackPolicy::Disabled {
        return None;
    }
    let Some(pipeline) = &ocr.pipeline else {
        return (ocr.backend == "tesseract").then(|| FirstStage {
            languages: ocr.effective_tesseract_language(),
            source: language_source(&ocr.language, false, ocr.tesseract_config.as_ref()),
        });
    };
    if !pipeline
        .stages
        .iter()
        .all(|stage| stage.backend.eq_ignore_ascii_case("tesseract"))
    {
        return None;
    }
    let stage = pipeline
        .stages
        .iter()
        .min_by_key(|stage| std::cmp::Reverse(stage.priority))?;
    let inputs = StageModelInputs::new(ocr, stage);
    Some(FirstStage {
        languages: inputs.languages(),
        source: language_source(
            stage.language.as_deref().unwrap_or(&ocr.language),
            stage.language.is_some(),
            inputs.tesseract(),
        ),
    })
}

/// Tells a letter that starts a cluster from a sign that attaches to one, for the scripts whose
/// orthography fixes where signs may stand.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Base,
    Sign,
    Other,
}

/// Inclusive code point ranges of one kind of character.
type Ranges = &'static [(u32, u32)];

fn part_of(script: Script, character: char) -> Part {
    use Script::*;
    let code = character as u32;
    let (bases, signs): (Ranges, Ranges) = match script {
        Khmer => (&[(0x1780, 0x17B3)], &[(0x17B4, 0x17D3), (0x17DD, 0x17DD)]),
        Thai => (
            &[(0x0E01, 0x0E2E), (0x0E40, 0x0E46)],
            &[(0x0E30, 0x0E3A), (0x0E47, 0x0E4E)],
        ),
        Lao => (
            &[(0x0E81, 0x0EAE), (0x0EC0, 0x0EC4)],
            &[(0x0EB0, 0x0EBD), (0x0EC8, 0x0ECD)],
        ),
        Myanmar => (&[(0x1000, 0x102A)], &[(0x102B, 0x103E), (0x1056, 0x1059)]),
        Devanagari | Bengali | Gurmukhi | Gujarati | Oriya | Tamil | Telugu | Kannada | Malayalam => {
            let block = code & !0x7F;
            return match code - block {
                0x04..=0x39 | 0x58..=0x5F => Part::Base,
                0x00..=0x03 | 0x3C..=0x4D | 0x62..=0x63 => Part::Sign,
                _ => Part::Other,
            };
        }
        _ => return Part::Other,
    };
    if bases.iter().any(|(low, high)| (*low..=*high).contains(&code)) {
        Part::Base
    } else if signs.iter().any(|(low, high)| (*low..=*high).contains(&code)) {
        Part::Sign
    } else {
        Part::Other
    }
}

/// Fewest signs a layer needs for its cluster structure to be judged.
const MIN_SIGNS: usize = 4;
/// Most signs one cluster carries in these scripts: a stacked subscript, a vowel sign, a tone or
/// shift mark and a final mark.
const MAX_SIGNS_PER_CLUSTER: usize = 4;
/// Share of a script's signs that must stand in a cluster that starts with a letter of the script.
/// Text in the script has all of them there; random code points of its block have about half.
const MIN_VALID_SIGN_SHARE: f64 = 0.95;
/// Share of a script's characters that must be signs. These scripts write most syllables with a
/// sign (Thai text runs about a quarter), while another script's text shifted into the block puts
/// letters where the signs are only by chance.
const MIN_SIGN_DENSITY: f64 = 0.15;

/// Whether the signs of `script` in `text` attach to letters of the script the way its orthography
/// requires, so the layer is the script's text and not other text decoded to its code points. Only
/// scripts [`part_of`] knows can be judged; any other script returns `false`. Zero-width joiners
/// and non-joiners sit inside Indic clusters and do not end one.
fn clusters_are_valid(text: &str, script: Script) -> bool {
    let (mut characters, mut signs, mut valid) = (0usize, 0usize, 0usize);
    let mut run: Option<usize> = None;
    for character in text.chars() {
        if matches!(character, '\u{200C}' | '\u{200D}') {
            continue;
        }
        if script_of(character) == Some(script) {
            characters += 1;
        }
        match part_of(script, character) {
            Part::Base => run = Some(0),
            Part::Sign => {
                signs += 1;
                run = run.map(|count| count + 1);
                if run.is_some_and(|count| count <= MAX_SIGNS_PER_CLUSTER) {
                    valid += 1;
                }
            }
            Part::Other => run = None,
        }
    }
    signs >= MIN_SIGNS
        && valid as f64 >= MIN_VALID_SIGN_SHARE * signs as f64
        && signs as f64 >= MIN_SIGN_DENSITY * characters as f64
}

/// Share of a page's uncovered letters below which a script is a stray (a `μ` in a unit, a name in
/// another script) and does not decide whether the layer is structured text. A page with
/// at most one such letter in ten (a four-letter word in a fifty-letter page) is one the dominant
/// script describes; a script above
/// it, such as a Greek passage in a Khmer page, is a second language the check cannot vouch for.
const MINOR_SCRIPT_SHARE: f64 = 0.10;

/// The scripts of `text` that no language in `languages` reads, for the pages the guard may keep.
/// A layer with lost-glyph markers qualifies on script evidence alone. A layer without them also
/// qualifies when every script that carries weight in it (see [`MINOR_SCRIPT_SHARE`]) has checkable
/// cluster structure and the layer passes [`clusters_are_valid`] for it: other text decoded through
/// a wrong map reaches Greek, Han or Cyrillic code points without markers, and OCR is the only
/// reader of it. A substantial script without checkable structure leaves the page to OCR.
fn judged_scripts(text: &str, languages: &[String]) -> Option<Vec<Script>> {
    let counts = uncovered_script_counts(text, languages)?;
    if text.contains(super::unmapped_glyphs::LOST_GLYPH_MARKER) {
        return Some(counts.into_iter().map(|(script, _)| script).collect());
    }
    let total: usize = counts.iter().map(|(_, count)| count).sum();
    let weighty: Vec<Script> = counts
        .into_iter()
        .filter(|(_, count)| *count as f64 >= MINOR_SCRIPT_SHARE * total as f64)
        .map(|(script, _)| script)
        .collect();
    // Many scripts of a few letters each leave none above the share: nothing is established.
    (!weighty.is_empty() && weighty.iter().all(|script| clusters_are_valid(text, *script))).then_some(weighty)
}

/// A failing page, the unreadable scripts of its layer, and whether the layer holds lost-glyph markers.
type Candidate = (u32, Vec<Script>, bool);

fn describe(pages: &[(u32, Vec<Script>)]) -> String {
    pages
        .iter()
        .map(|(page, scripts)| {
            let names = scripts.iter().map(|script| format!("{script:?}")).collect::<Vec<_>>();
            format!(
                "page {page} ({}; for example {})",
                names.join(" and "),
                model_for(scripts[0])
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn warning(message: String) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Owned(message),
    }
}

/// Where the caller adds the model: the setting the shown languages were taken from.
fn where_to_add(stage: &FirstStage) -> &'static str {
    match stage.source {
        LanguageSource::StageLanguage => "the language of the first stage of ocr.pipeline",
        LanguageSource::TesseractConfig => "tesseract_config.language",
        LanguageSource::OcrLanguage => "ocr.language",
    }
}

/// The clause on glyphs with no Unicode value: all kept pages, or the marked ones by number.
fn lost_glyphs_clause(kept: &[(u32, Vec<Script>, bool)]) -> String {
    let marked: Vec<String> = kept
        .iter()
        .filter(|(.., marked)| *marked)
        .map(|(page, ..)| page.to_string())
        .collect();
    if marked.is_empty() {
        String::new()
    } else if marked.len() == kept.len() {
        ", and glyphs it has no Unicode value for stay lost".to_string()
    } else {
        format!(
            ", and glyphs it has no Unicode value for stay lost on page {}",
            marked.join(", ")
        )
    }
}

fn kept_pages_warning(kept: &[(u32, Vec<Script>, bool)], stage: &FirstStage) -> crate::types::ProcessingWarning {
    let lost = lost_glyphs_clause(kept);
    let kept: Vec<_> = kept.iter().map(|(page, scripts, _)| (*page, scripts.clone())).collect();
    let kept = kept.as_slice();
    warning(format!(
        "OCR was skipped for {}: the native text is in a script the configured OCR languages \
         ({}) cannot read, so it was kept as extracted{lost}. \
         Add a model for this script, for example the one named, to {} to OCR it.",
        describe(kept),
        stage.languages.join("+"),
        where_to_add(stage),
    ))
}

fn read_pages_warning(read: &[(u32, Vec<Script>)], stage: &FirstStage) -> crate::types::ProcessingWarning {
    warning(format!(
        "The native text of {} is in a script the configured OCR languages ({}) cannot read, and the \
         page is read by OCR because it carries a picture. Text that OCR reads in this script is \
         likely to be wrong. Add a model for this script, for example the one named, to {}.",
        describe(read),
        stage.languages.join("+"),
        where_to_add(stage),
    ))
}

/// Remove from the quality gate's failing pages every page that OCR with the configured
/// languages cannot improve: a page whose native text is written in a script none of the
/// languages covers (see [`unreadable_scripts`]). Pages that stay failing are unchanged, and the
/// document-level flags follow from what remains.
///
/// A page that is a scan or carries one (see [`page_is_scan`]) stays failing: its text layer says
/// nothing about the picture, which OCR is the only reader of. Returns `ocr` warnings: one for the
/// kept pages and one for the pages that stay failing although their layer is in an unreadable
/// script, so a caller whose scan is in Khmer is told to add `khm`. A page is in at most one.
///
/// The filter runs before the provenance and plausibility flags are merged into the decision, so
/// a page whose layer is flagged as a mis-decode (all-Greek English from a symbol font, for
/// example) is returned to OCR by them.
pub(super) fn retain_unreadable_script_pages(
    decision: &mut OcrFallbackDecision,
    native_text: &str,
    boundaries: Option<&[PageBoundary]>,
    page_count: Option<u32>,
    content: &[u8],
    config: &ExtractionConfig,
) -> Vec<crate::types::ProcessingWarning> {
    let Some(ocr) = config.ocr.as_ref() else {
        return Vec::new();
    };
    let Some(boundaries) = boundaries.filter(|boundaries| {
        !decision.failing_pages.is_empty()
            && page_count.is_some_and(|count| count as usize == boundaries.len())
            && boundaries
                .iter()
                .enumerate()
                .all(|(index, boundary)| boundary.page_number as usize == index + 1)
    }) else {
        return Vec::new();
    };
    let Some(stage) = first_stage(ocr) else {
        return Vec::new();
    };
    let candidates: Vec<Candidate> = decision
        .failing_pages
        .iter()
        .filter_map(|page| {
            let boundary = boundaries.iter().find(|boundary| boundary.page_number == *page)?;
            let text = native_text.get(boundary.byte_start..boundary.byte_end)?;
            let scripts = judged_scripts(text, &stage.languages)?;
            Some((*page, scripts, text.contains(super::unmapped_glyphs::LOST_GLYPH_MARKER)))
        })
        .collect();
    if candidates.is_empty() {
        return Vec::new();
    }
    let document = match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(document) => document,
        Err(error) => {
            tracing::debug!(%error, "script evidence skipped: the PDF could not be opened to check for pictures");
            return Vec::new();
        }
    };
    let (read, kept): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|(page, ..)| page_is_scan(&document, *page as usize - 1));
    let read: Vec<_> = read.into_iter().map(|(page, scripts, _)| (page, scripts)).collect();
    let mut warnings = Vec::new();
    if !kept.is_empty() {
        decision
            .failing_pages
            .retain(|page| !kept.iter().any(|(kept_page, ..)| kept_page == page));
        decision.fallback = !decision.failing_pages.is_empty();
        decision.whole_doc_failure = decision.failing_pages.len() == boundaries.len();
        warnings.push(kept_pages_warning(&kept, &stage));
    }
    if !read.is_empty() {
        warnings.push(read_pages_warning(&read, &stage));
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_PAGE: &[u8] =
        include_bytes!("../../../tests/fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
    const TWO_PAGES: &[u8] =
        include_bytes!("../../../tests/fixtures/pdf/khmer-cluster-2536/khmer-then-scanned-english.pdf");
    const SCAN_WITH_OVERLAY: &[u8] =
        include_bytes!("../../../tests/fixtures/pdf/khmer-cluster-2536/scanned-english-khmer-overlay.pdf");

    const TEXT_WITH_FIGURE: &[u8] =
        include_bytes!("../../../tests/fixtures/pdf/khmer-cluster-2536/khmer-text-with-figure.pdf");

    const KHMER_MARKED: &str = "ខ្ញុំ\u{FFFD}\u{FFFD} ស្រលាញ់\u{FFFD} ប្រទេស\u{FFFD}\u{FFFD} កម្ពុជា\u{FFFD} សួស្តី\u{FFFD} ភ្នំពេញ";

    fn langs(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|code| (*code).to_string()).collect()
    }

    fn page_text(pages: &[&str]) -> (String, Vec<PageBoundary>) {
        let mut text = String::new();
        let mut boundaries = Vec::new();
        for (index, page) in pages.iter().enumerate() {
            let byte_start = text.len();
            text.push_str(page);
            boundaries.push(PageBoundary {
                byte_start,
                byte_end: text.len(),
                page_number: index as u32 + 1,
            });
            text.push('\n');
        }
        (text, boundaries)
    }

    fn config_with(languages: &[&str]) -> ExtractionConfig {
        ExtractionConfig {
            ocr: Some(crate::OcrConfig {
                language: langs(languages),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn decision_for(text: &str, boundaries: &[PageBoundary]) -> OcrFallbackDecision {
        super::super::ocr::evaluate_per_page_ocr(
            text,
            Some(boundaries),
            Some(boundaries.len() as u32),
            &Default::default(),
        )
    }

    fn run(
        pages: &[&str],
        content: &[u8],
        config: &ExtractionConfig,
    ) -> (OcrFallbackDecision, Vec<crate::types::ProcessingWarning>) {
        let (text, boundaries) = page_text(pages);
        let mut decision = decision_for(&text, &boundaries);
        let warnings = retain_unreadable_script_pages(
            &mut decision,
            &text,
            Some(&boundaries),
            Some(boundaries.len() as u32),
            content,
            config,
        );
        (decision, warnings)
    }

    #[test]
    fn names_the_scripts_no_configured_language_reads() {
        assert_eq!(
            unreadable_scripts(KHMER_MARKED, &langs(&["eng"])),
            Some(vec![Script::Khmer])
        );
        assert_eq!(
            unreadable_scripts(KHMER_MARKED, &langs(&["ENG"])),
            Some(vec![Script::Khmer])
        );
        let russian = "Привет мир это длинная строка текста\u{FFFD}";
        assert_eq!(
            unreadable_scripts(russian, &langs(&["eng", "deu"])),
            Some(vec![Script::Cyrillic])
        );
        let devanagari = "यह एक लंबा वाक्य है जो देवनागरी में लिखा गया\u{FFFD}";
        assert_eq!(
            unreadable_scripts(devanagari, &langs(&["eng"])),
            Some(vec![Script::Devanagari])
        );
    }

    #[test]
    fn japanese_is_a_set_of_uncovered_scripts_under_english() {
        let japanese = "日本語のテキストはここにあります。これは長い文です\u{FFFD}";
        let found = unreadable_scripts(japanese, &langs(&["eng"])).expect("Han and Kana");
        assert!(found.contains(&Script::Han) && found.contains(&Script::Kana) && found.len() == 2);
        assert_eq!(unreadable_scripts(japanese, &langs(&["jpn"])), None);
    }

    #[test]
    fn latin_names_in_a_khmer_page_do_not_dilute_it() {
        let khmer = [KHMER_MARKED; 4].join(" ");
        let page = format!("{khmer} Mallori https://example.org/khmer Phnom Penh Invoice");
        assert_eq!(unreadable_scripts(&page, &langs(&["eng"])), Some(vec![Script::Khmer]));
    }

    #[test]
    fn a_mostly_covered_page_stays_with_ocr() {
        let latin_heavy = format!(
            "{KHMER_MARKED} {}",
            "Invoice number and total due on receipt of goods ".repeat(2)
        );
        assert_eq!(unreadable_scripts(&latin_heavy, &langs(&["eng"])), None);
        let english_with_greek = format!(
            "The measured resistance in the sample was {} with a tolerance of the stated value \u{FFFD}\u{FFFD} per cent \
             across every one of the trial runs reported in the table below and in the appendix",
            "αβγδ ".repeat(8)
        );
        let greek = english_with_greek
            .chars()
            .filter(|c| script_of(*c) == Some(Script::Greek))
            .count();
        assert_eq!(greek, 32);
        assert_eq!(unreadable_scripts(&english_with_greek, &langs(&["eng"])), None);
    }

    #[test]
    fn the_floor_separates_a_unit_symbol_from_a_short_title() {
        let omegas = |count: usize| format!("12.5 3.4 \u{FFFD} {} 9.1", "Ω ".repeat(count));
        assert_eq!(
            unreadable_scripts(&omegas(MIN_SCRIPT_CHARS - 1), &langs(&["eng"])),
            None
        );
        assert_eq!(
            unreadable_scripts(&omegas(MIN_SCRIPT_CHARS), &langs(&["eng"])),
            Some(vec![Script::Greek])
        );
        let title = "ខ្ញុំ\u{FFFD} សួស្តី";
        assert_eq!(unreadable_scripts(title, &langs(&["eng"])), Some(vec![Script::Khmer]));
    }

    #[test]
    fn a_covered_script_or_an_unknown_model_leaves_ocr_the_page() {
        let devanagari = "यह एक लंबा वाक्य है जो देवनागरी में लिखा गया\u{FFFD}";
        for (text, languages) in [
            (KHMER_MARKED, langs(&["eng", "khm"])),
            (KHMER_MARKED, langs(&["eng+khm"])),
            (KHMER_MARKED, langs(&["KHM"])),
            (KHMER_MARKED, langs(&["khmer"])),
            (KHMER_MARKED, langs(&["script/Khmer"])),
            (devanagari, langs(&["script/Devanagari"])),
            (devanagari, langs(&["hin"])),
            (KHMER_MARKED, langs(&["eng", "osd"])),
            (KHMER_MARKED, langs(&["zzz"])),
            (KHMER_MARKED, langs(&["eng", "zzz"])),
            (KHMER_MARKED, Vec::new()),
        ] {
            assert_eq!(unreadable_scripts(text, &languages), None, "{text} {languages:?}");
        }
    }

    #[test]
    fn latin_is_read_by_the_non_latin_models() {
        let english = "An English page with lost glyphs \u{FFFD}\u{FFFD} in a longer sentence of text";
        for languages in [&["rus"][..], &["chi_sim"], &["khm"], &["eng"]] {
            assert_eq!(unreadable_scripts(english, &langs(languages)), None, "{languages:?}");
        }
    }

    #[test]
    fn a_layer_that_is_not_mostly_a_known_unreadable_script_does_not_qualify() {
        let private_use = "\u{E000}".repeat(3000);
        let dingbats = "\u{2701}".repeat(3000);
        let cases = [
            ("private use", format!("{private_use}\u{FFFD}й")),
            ("dingbats", format!("{dingbats}\u{FFFD}й")),
            (
                "numeric table",
                "12.5 3.4 7.8 \u{FFFD} 9.1 2.2 μ 4.4 5.5 6.6".to_string(),
            ),
        ];
        for (name, text) in cases {
            assert_eq!(unreadable_scripts(&text, &langs(&["eng"])), None, "{name}");
        }
    }

    #[test]
    fn marked_page_in_an_unreadable_script_leaves_the_failing_pages_with_a_warning() {
        let (decision, warnings) = run(&[KHMER_MARKED, ""], TWO_PAGES, &config_with(&["eng"]));
        assert_eq!(decision.failing_pages, [2]);
        assert!(decision.fallback);
        assert!(!decision.whole_doc_failure);
        assert_eq!(warnings.len(), 1);
        let warning = &warnings[0];
        assert_eq!(warning.source, "ocr");
        for part in ["page 1", "Khmer", "for example khm", "(eng)", "lost"] {
            assert!(warning.message.contains(part), "{part}: {}", warning.message);
        }
        assert!(!warning.message.contains("page 2"));
    }

    #[test]
    fn single_marked_page_in_an_unreadable_script_needs_no_fallback() {
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
        assert!(!decision.fallback);
        assert!(!decision.whole_doc_failure);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn a_page_a_picture_covers_stays_failing_so_its_body_is_read() {
        let (decision, warnings) = run(&[KHMER_MARKED], SCAN_WITH_OVERLAY, &config_with(&["eng"]));
        assert_eq!(decision.failing_pages, [1]);
        assert!(decision.fallback);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        for part in ["page 1", "Khmer", "for example khm", "(eng)", "read by OCR"] {
            assert!(warnings[0].message.contains(part), "{part}: {}", warnings[0].message);
        }
        assert!(!warnings[0].message.contains("kept"));
    }

    #[test]
    fn a_text_heavy_page_with_a_figure_is_kept_although_a_picture_covers_a_quarter_of_it() {
        let document = xberg_native_pdf::PdfDocument::from_bytes(TEXT_WITH_FIGURE.to_vec()).unwrap();
        assert!(
            crate::pdf::scan_detect::carries_scan_raster(&document, 0),
            "a coverage-only check would send this page to OCR"
        );
        assert!(!page_is_scan(&document, 0));
        let (decision, warnings) = run(&[KHMER_MARKED], TEXT_WITH_FIGURE, &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn a_page_stays_failing_when_nothing_qualifies_it() {
        let latin_marked = "Invoice \u{FFFD}\u{FFFD}\u{FFFD} total \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
        for (page, languages) in [
            (KHMER_MARKED, &["eng", "khm"][..]),
            (KHMER_MARKED, &["eng", "zzz"][..]),
            (latin_marked, &["eng"][..]),
        ] {
            let (text, boundaries) = page_text(&[page]);
            let before = decision_for(&text, &boundaries);
            let (after, warning) = run(&[page], ONE_PAGE, &config_with(languages));
            assert_eq!(
                (after.fallback, after.failing_pages.clone()),
                (before.fallback, before.failing_pages.clone()),
                "{page:?} {languages:?}"
            );
            assert!(warning.is_empty());
        }
    }

    #[test]
    fn page_numbering_and_count_must_match_the_boundaries() {
        let (text, mut boundaries) = page_text(&[KHMER_MARKED]);
        let config = config_with(&["eng"]);
        let mut decision = decision_for(&text, &boundaries);
        let before = decision.failing_pages.clone();
        assert!(!before.is_empty());
        for (page_count, shift) in [(None, 0), (Some(2), 0), (Some(1), 1)] {
            boundaries[0].page_number += shift;
            let warning =
                retain_unreadable_script_pages(&mut decision, &text, Some(&boundaries), page_count, ONE_PAGE, &config);
            boundaries[0].page_number -= shift;
            assert!(warning.is_empty(), "{page_count:?} {shift}");
            assert_eq!(decision.failing_pages, before, "{page_count:?} {shift}");
        }
        let warning = retain_unreadable_script_pages(&mut decision, &text, None, Some(1), ONE_PAGE, &config);
        assert!(warning.is_empty());
        assert_eq!(decision.failing_pages, before);
    }

    #[test]
    fn other_backends_are_left_alone() {
        let mut paddle = config_with(&["eng"]);
        paddle.ocr.as_mut().unwrap().backend = "paddleocr".to_string();
        let mut vlm = config_with(&["eng"]);
        vlm.ocr.as_mut().unwrap().vlm_fallback = VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 };
        for config in [paddle, vlm] {
            let (decision, warning) = run(&[KHMER_MARKED], ONE_PAGE, &config);
            assert_eq!(decision.failing_pages, [1]);
            assert!(warning.is_empty());
        }
    }

    fn pipeline_config(pipeline: serde_json::Value) -> ExtractionConfig {
        let mut config = config_with(&["eng"]);
        config.ocr.as_mut().unwrap().pipeline = Some(serde_json::from_value(pipeline).unwrap());
        config
    }

    #[test]
    fn a_pipeline_keeps_the_native_page_when_its_first_stage_cannot_read_the_script() {
        for stages in [
            serde_json::json!([{"backend": "tesseract", "priority": 100}]),
            serde_json::json!([
                {"backend": "tesseract", "priority": 100},
                {"backend": "tesseract", "priority": 50, "language": ["eng", "deu"]},
            ]),
            serde_json::json!([
                {"backend": "tesseract", "priority": 100},
                {"backend": "tesseract", "priority": 50, "language": ["khm"]},
            ]),
            // The array order is not the run order: the priority is.
            serde_json::json!([
                {"backend": "tesseract", "priority": 50, "language": ["khm"]},
                {"backend": "tesseract", "priority": 100},
            ]),
            // Equal priorities run in the order listed.
            serde_json::json!([
                {"backend": "tesseract", "priority": 100},
                {"backend": "tesseract", "priority": 100, "language": ["khm"]},
            ]),
        ] {
            let config = pipeline_config(serde_json::json!({ "stages": stages }));
            let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &config);
            assert!(decision.failing_pages.is_empty(), "{stages}");
            assert_eq!(warnings.len(), 1, "{stages}");
            assert!(warnings[0].message.contains("(eng"), "{}", warnings[0].message);
        }
    }

    #[test]
    fn a_pipeline_is_left_to_ocr_when_its_first_stage_reads_the_script_or_one_makes_no_claim() {
        for pipeline in [
            serde_json::json!({"stages": [{"backend": "tesseract", "priority": 100, "language": ["eng", "khm"]}]}),
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 100, "language": ["khm"]},
                {"backend": "tesseract", "priority": 50, "language": ["eng", "khm"]},
            ]}),
            // The `khm` stage reads first and the English stage is only the fallback.
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 100, "language": ["khm"]},
                {"backend": "tesseract", "priority": 50},
            ]}),
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 50},
                {"backend": "tesseract", "priority": 100, "language": ["khm"]},
            ]}),
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 100, "language": ["khm"]},
                {"backend": "tesseract", "priority": 100},
            ]}),
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 100},
                {"backend": "paddleocr", "priority": 50},
            ]}),
            serde_json::json!({"stages": [{"backend": "vlm", "priority": 100}]}),
            serde_json::json!({"stages": []}),
        ] {
            let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &pipeline_config(pipeline.clone()));
            assert_eq!(decision.failing_pages, [1], "{pipeline}");
            assert!(warnings.is_empty(), "{pipeline}");
        }
    }

    fn with_tesseract_language(config: &mut ExtractionConfig, language: &str) {
        config.ocr.as_mut().unwrap().tesseract_config =
            Some(serde_json::from_value(serde_json::json!({"language": [language]})).unwrap());
    }

    #[test]
    fn the_stage_languages_follow_the_runner_through_the_tesseract_config() {
        // The parent `tesseract_config.language` names `khm` when the stage names none.
        let mut inherited = pipeline_config(serde_json::json!({"stages": [{"backend": "tesseract", "priority": 100}]}));
        with_tesseract_language(&mut inherited, "khm");
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &inherited);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());

        // The stage `tesseract_config.language` wins over a parent that names another model.
        let mut own = pipeline_config(serde_json::json!({"stages": [
            {"backend": "tesseract", "priority": 100, "tesseract_config": {"language": ["khm"]}},
        ]}));
        with_tesseract_language(&mut own, "deu");
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &own);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());

        // Without a pipeline the parent `tesseract_config.language` is read too.
        let mut plain = config_with(&["eng"]);
        with_tesseract_language(&mut plain, "khm");
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &plain);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());

        // A stage `language` is read before any `tesseract_config.language`, which then names the loser.
        let mut stage_language = pipeline_config(serde_json::json!({"stages": [
            {"backend": "tesseract", "priority": 100, "language": ["deu"]},
        ]}));
        with_tesseract_language(&mut stage_language, "khm");
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &stage_language);
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn a_page_of_many_small_scripts_establishes_nothing_and_does_not_panic() {
        let page = "αβγδεζ абвгде աբգդեզ אבגדהו ابتثجح აბგდევ 가나다라마바 天地玄黄宇宙 ሀለሐመሠረ กขคงจฉ កខគឃងច";
        assert_eq!(judged_scripts(page, &langs(&["eng"])), None);
        let (decision, warnings) = forced_failing(page, &["eng"]);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn the_kept_page_warning_says_glyphs_stay_lost_only_for_a_page_with_markers() {
        let unmarked = "ខ្ញុំ ស្រលាញ់ ប្រទេស";
        let (_, marked) = run(&[KHMER_MARKED], ONE_PAGE, &config_with(&["eng"]));
        let (_, clean) = run(&[unmarked], ONE_PAGE, &config_with(&["eng"]));
        assert!(marked[0].message.contains("stay lost"), "{}", marked[0].message);
        assert_eq!(clean.len(), 1);
        assert!(!clean[0].message.contains("stay lost"), "{}", clean[0].message);
    }

    #[test]
    fn the_warnings_name_the_setting_that_the_first_stage_reads() {
        let stages = |first: serde_json::Value, second: serde_json::Value| {
            pipeline_config(serde_json::json!({"stages": [first, second]}))
        };
        let own = serde_json::json!({"backend": "tesseract", "priority": 100, "language": ["eng"]});
        let inherits = serde_json::json!({"backend": "tesseract", "priority": 100});
        let other = serde_json::json!({"backend": "tesseract", "priority": 50, "language": ["eng"]});
        let after_inheriting = serde_json::json!({"backend": "tesseract", "priority": 50});
        for content in [ONE_PAGE, SCAN_WITH_OVERLAY] {
            let (_, named) = run(&[KHMER_MARKED], content, &stages(own.clone(), after_inheriting.clone()));
            assert_eq!(named.len(), 1);
            assert!(
                named[0].message.contains("first stage of ocr.pipeline"),
                "{}",
                named[0].message
            );
            assert!(!named[0].message.contains("to ocr.language"), "{}", named[0].message);
            // The first stage inherits `ocr.language`, so that is the setting to edit.
            let (_, inherited) = run(&[KHMER_MARKED], content, &stages(inherits.clone(), other.clone()));
            assert!(
                inherited[0].message.contains("to ocr.language"),
                "{}",
                inherited[0].message
            );
        }
    }

    #[test]
    fn the_warning_names_tesseract_config_when_the_shown_languages_came_from_it() {
        let mut inherited = pipeline_config(serde_json::json!({"stages": [{"backend": "tesseract", "priority": 100}]}));
        with_tesseract_language(&mut inherited, "deu");
        let mut plain = config_with(&["eng"]);
        with_tesseract_language(&mut plain, "deu");
        let mut stage_eng = pipeline_config(serde_json::json!({"stages": [
            {"backend": "tesseract", "priority": 100, "language": ["eng"]},
        ]}));
        with_tesseract_language(&mut stage_eng, "deu");
        let mut stage_language = pipeline_config(serde_json::json!({"stages": [
            {"backend": "tesseract", "priority": 100, "language": ["fra"]},
        ]}));
        with_tesseract_language(&mut stage_language, "deu");
        for content in [ONE_PAGE, SCAN_WITH_OVERLAY] {
            for config in [&inherited, &plain, &stage_eng] {
                let (_, warnings) = run(&[KHMER_MARKED], content, config);
                assert_eq!(warnings.len(), 1);
                assert!(warnings[0].message.contains("(deu"), "{}", warnings[0].message);
                assert!(
                    warnings[0].message.contains("tesseract_config.language"),
                    "{}",
                    warnings[0].message
                );
            }
            let (_, warnings) = run(&[KHMER_MARKED], content, &stage_language);
            assert!(
                warnings[0].message.contains("first stage of ocr.pipeline"),
                "{}",
                warnings[0].message
            );
        }
    }

    #[test]
    fn the_lost_glyph_clause_names_the_marked_pages_unless_every_kept_page_is_marked() {
        let stage = FirstStage {
            languages: langs(&["eng"]),
            source: LanguageSource::OcrLanguage,
        };
        let page = |number, marked| (number, vec![Script::Khmer], marked);
        let mixed = kept_pages_warning(&[page(1, true), page(2, false), page(3, true)], &stage).message;
        assert!(mixed.contains("stay lost on page 1, 3"), "{mixed}");
        let all = kept_pages_warning(&[page(1, true), page(2, true)], &stage).message;
        assert!(all.contains("stay lost.") || all.contains("stay lost,"), "{all}");
        assert!(!all.contains("stay lost on page"), "{all}");
        let none = kept_pages_warning(&[page(1, false), page(2, false)], &stage).message;
        assert!(!none.contains("stay lost"), "{none}");
    }

    #[test]
    fn a_vlm_fallback_on_the_pipeline_path_is_no_claim() {
        let mut config = pipeline_config(serde_json::json!({"stages": [{"backend": "tesseract", "priority": 100}]}));
        config.ocr.as_mut().unwrap().vlm_fallback = VlmFallbackPolicy::Always;
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &config);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());
    }

    fn forced_failing(page: &str, languages: &[&str]) -> (OcrFallbackDecision, Vec<crate::types::ProcessingWarning>) {
        let (text, boundaries) = page_text(&[page]);
        let mut decision = decision_for(&text, &boundaries);
        decision.failing_pages = vec![1];
        decision.fallback = true;
        decision.whole_doc_failure = true;
        let warnings = retain_unreadable_script_pages(
            &mut decision,
            &text,
            Some(&boundaries),
            Some(1),
            ONE_PAGE,
            &config_with(languages),
        );
        (decision, warnings)
    }

    #[test]
    fn unmarked_code_points_of_scripts_without_checkable_structure_stay_with_ocr() {
        let han = "今天天气很好我们去公园散步图书馆的新书放在二楼的左边".repeat(3);
        let greek = "ηελλο ωορλδ τηισ ισ α τεστ οφ γρεεκ λεττερσ ".repeat(4);
        let cyrillic = "привет мир это проверка русского текста ".repeat(4);
        for (name, page) in [("han", han), ("greek", greek), ("cyrillic", cyrillic)] {
            let (decision, warnings) = forced_failing(&page, &["eng"]);
            assert_eq!(decision.failing_pages, [1], "{name}");
            assert!(warnings.is_empty(), "{name}");
        }
    }

    #[test]
    fn marked_pages_in_scripts_without_checkable_structure_are_still_kept() {
        let marked = format!(
            "{}\u{FFFD}",
            "今天天气很好我们去公园散步图书馆的新书放在二楼的左边".repeat(3)
        );
        let (decision, warnings) = forced_failing(&marked, &["eng"]);
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn random_khmer_code_points_are_not_a_khmer_layer() {
        let mut state = 0x2545_F491_u32;
        let page: String = (0..400)
            .map(|index| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                if index % 7 == 6 {
                    ' '
                } else {
                    char::from_u32(0x1780 + (state >> 16) % 0x60).unwrap()
                }
            })
            .collect();
        assert!(
            unreadable_scripts(&page, &langs(&["eng"])).is_some(),
            "the script is detected"
        );
        assert!(judged_scripts(&page, &langs(&["eng"])).is_none());
        let (decision, warnings) = forced_failing(&page, &["eng"]);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn real_text_of_each_checkable_script_has_valid_clusters() {
        for (script, text) in [
            (Script::Khmer, "ខ្ញុំ ស្រលាញ់ ប្រទេស កម្ពុជា សួស្តី ភ្នំពេញ"),
            (Script::Thai, "สวัสดีครับ ผมชอบกินข้าวผัดกระเพรา ประเทศไทย"),
            (Script::Lao, "ສະບາຍດີ ຂ້ອຍມັກກິນເຂົ້າ ປະເທດລາວ"),
            (Script::Myanmar, "မင်္ဂလာပါ ကျွန်တော် မြန်မာနိုင်ငံ"),
            (Script::Devanagari, "नमस्ते मुझे हिन्दी भाषा पसंद है भारत देश"),
            (Script::Tamil, "வணக்கம் நான் தமிழ் மொழியை விரும்புகிறேன்"),
        ] {
            assert!(clusters_are_valid(text, script), "{script:?}");
        }
    }

    #[test]
    fn a_page_without_lost_glyph_markers_is_judged_by_its_script() {
        let unmarked = "ខ្ញុំ ស្រលាញ់ ប្រទេស";
        assert!(
            !decision_for(unmarked, &page_text(&[unmarked]).1)
                .failing_pages
                .is_empty()
        );
        let (decision, warnings) = run(&[unmarked], ONE_PAGE, &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
        let (decision, warnings) = run(&[unmarked], ONE_PAGE, &config_with(&["eng", "khm"]));
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_kept_page_and_a_read_page_each_get_one_warning() {
        let (decision, warnings) = run(&[KHMER_MARKED, KHMER_MARKED], TWO_PAGES, &config_with(&["eng"]));
        assert_eq!(decision.failing_pages, [2]);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].message.contains("page 1") && !warnings[0].message.contains("page 2"));
        assert!(warnings[1].message.contains("page 2") && !warnings[1].message.contains("page 1"));
    }

    #[test]
    fn a_flagged_mis_decode_returns_to_ocr_after_the_filter() {
        let (text, boundaries) = page_text(&[KHMER_MARKED]);
        let mut decision = decision_for(&text, &boundaries);
        retain_unreadable_script_pages(
            &mut decision,
            &text,
            Some(&boundaries),
            Some(1),
            ONE_PAGE,
            &config_with(&["eng"]),
        );
        assert!(decision.failing_pages.is_empty());
        super::super::ocr::apply_flagged_pages(&mut decision, &[1], true, Some(1));
        assert_eq!(decision.failing_pages, [1]);
        assert!(decision.fallback);
    }

    #[test]
    fn english_decoded_through_a_greek_symbol_font_is_kept_only_when_a_lost_glyph_marks_it() {
        let english = "The committee reviewed the annual report and approved the budget for the next fiscal year. \
                       Members discussed staffing, equipment purchases and the schedule of public meetings. ";
        let symbol_font: String = english
            .chars()
            .map(|c| match c {
                'a' => 'α',
                'b' => 'β',
                'c' => 'χ',
                'd' => 'δ',
                'e' => 'ε',
                'f' => 'φ',
                'g' => 'γ',
                'h' => 'η',
                'i' => 'ι',
                'k' => 'κ',
                'l' => 'λ',
                'm' => 'μ',
                'n' => 'ν',
                'o' => 'ο',
                'p' => 'π',
                'q' => 'θ',
                'r' => 'ρ',
                's' => 'σ',
                't' => 'τ',
                'u' => 'υ',
                'w' => 'ω',
                'x' => 'ξ',
                'y' => 'ψ',
                other => other,
            })
            .collect::<String>()
            .repeat(3);
        let (unmarked_decision, unmarked_warnings) = forced_failing(&symbol_font, &["eng"]);
        assert_eq!(unmarked_decision.failing_pages, [1]);
        assert!(unmarked_warnings.is_empty());
        let marked = format!("{symbol_font}\u{FFFD}");
        let (text, boundaries) = page_text(&[&marked]);
        // The language check abstains on Greek-letter gibberish, so nothing flags this layer, and
        // script evidence cannot tell it from real Greek. Without a lost-glyph marker the page stays
        // with OCR (above); with one it is kept, as on main, unless a flag returns it (see
        // `a_flagged_mis_decode_returns_to_ocr_after_the_filter`).
        let scan = super::super::ocr::scan_text_plausibility(&text, Some(&boundaries), Some(1), &Default::default());
        assert_eq!(
            (scan.judged, scan.implausible.clone(), scan.unjudged.clone()),
            (0, vec![], vec![1])
        );
        let (decision, _) = run(&[&marked], ONE_PAGE, &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
    }

    const KHMER_UNMARKED: &str = "ខ្ញុំ ស្រលាញ់ ប្រទេស កម្ពុជា សួស្តី ភ្នំពេញ ក្រុង សៀវភៅ សាលារៀន";

    #[test]
    fn a_stray_character_of_another_script_does_not_block_a_valid_layer() {
        for stray in ["μ", "я", "今", "שלום", "α β"] {
            let page = format!("{KHMER_UNMARKED} {stray}");
            assert!(judged_scripts(&page, &langs(&["eng"])).is_some(), "{stray}");
        }
        let (decision, warnings) = forced_failing(&format!("{KHMER_UNMARKED} 5 μm"), &["eng"]);
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn a_substantial_second_script_leaves_the_page_to_ocr() {
        let greek = "Η γρήγορη καφέ αλεπού πηδά πάνω από το τεμπέλικο σκυλί";
        let page = format!("{KHMER_UNMARKED} {greek}");
        assert!(judged_scripts(&page, &langs(&["eng"])).is_none());
        let (decision, warnings) = forced_failing(&page, &["eng"]);
        assert_eq!(decision.failing_pages, [1]);
        assert!(warnings.is_empty());
    }

    fn random_block(start: u32, span: u32) -> String {
        let mut state = 0x2545_F491_u32;
        (0..400)
            .map(|index| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                if index % 7 == 6 {
                    ' '
                } else {
                    char::from_u32(start + (state >> 16) % span).unwrap()
                }
            })
            .collect()
    }

    #[test]
    fn random_thai_and_myanmar_code_points_are_not_a_layer() {
        for (name, start, span) in [("thai", 0x0E01, 0x5B), ("myanmar", 0x1000, 0x5F)] {
            let page = random_block(start, span);
            assert!(unreadable_scripts(&page, &langs(&["eng"])).is_some(), "{name}");
            assert!(judged_scripts(&page, &langs(&["eng"])).is_none(), "{name}");
        }
    }

    #[test]
    fn english_shifted_into_the_thai_block_is_not_a_thai_layer() {
        let english = "the committee reviewed the annual report and approved the budget for the next fiscal year \
                       members discussed staffing equipment purchases and the schedule of public meetings";
        let shifted: String = english
            .chars()
            .map(|c| {
                if c == ' ' {
                    ' '
                } else {
                    char::from_u32(c as u32 - 0x61 + 0x0E01).unwrap()
                }
            })
            .collect();
        assert!(unreadable_scripts(&shifted, &langs(&["eng"])).is_some());
        assert!(judged_scripts(&shifted, &langs(&["eng"])).is_none());
    }

    #[test]
    fn a_layer_with_few_signs_is_not_judged() {
        let consonants = "កខគឃងចឆជឈញ".repeat(12);
        let page = format!("{consonants} ខ្ញុំ ស្រ ក្រ ប្រ");
        assert!(!clusters_are_valid(&page, Script::Khmer));
    }

    #[test]
    fn joiners_inside_an_indic_cluster_do_not_end_it() {
        let text = "क्\u{200D}ष र्\u{200D}क ज्\u{200C}ञ त्\u{200D}र नमस्ते हिन्दी";
        assert!(clusters_are_valid(text, Script::Devanagari));
    }
}
