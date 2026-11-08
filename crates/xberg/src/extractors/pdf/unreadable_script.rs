//! Keep a native text layer whose script the configured OCR languages cannot read.
//!
//! OCR with a model that does not cover a script reads that script as letters of another one, so
//! replacing a marked native page with such a read loses its text. The decision rests on script
//! evidence only: the scripts of the page's native letters against the scripts the OCR languages
//! cover.

use crate::core::config::{ExtractionConfig, VlmFallbackPolicy};
use crate::extractors::pdf::ocr::OcrFallbackDecision;
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
fn unreadable_scripts(text: &str, languages: &[String]) -> Option<Vec<Script>> {
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
    Some(counts.into_iter().map(|(script, _)| script).collect())
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

/// The languages every stage of the OCR route reads with, or `None` when some stage's reading
/// cannot be described by a Tesseract model list. Only Tesseract stages make a language claim:
/// PaddleOCR, Sceptre, VLM and custom backends are "no claim", so a route that can reach one is
/// never judged. A pipeline's stages are pooled, because a stage that reads the script is enough
/// for OCR to be worth running.
fn route_languages(ocr: &crate::OcrConfig) -> Option<Vec<String>> {
    if !ocr.enabled {
        return None;
    }
    let Some(pipeline) = &ocr.pipeline else {
        return (ocr.backend == "tesseract" && ocr.vlm_fallback == VlmFallbackPolicy::Disabled)
            .then(|| ocr.effective_tesseract_language());
    };
    if pipeline.stages.is_empty() {
        return None;
    }
    let mut languages = Vec::new();
    for stage in &pipeline.stages {
        if !stage.backend.eq_ignore_ascii_case("tesseract") {
            return None;
        }
        let parent = &ocr.language;
        let own = stage.language.as_ref().unwrap_or(parent);
        for language in crate::core::config::ocr::effective_tesseract_language_for(
            own,
            stage.tesseract_config.as_ref().or(ocr.tesseract_config.as_ref()),
        ) {
            if !languages.contains(&language) {
                languages.push(language);
            }
        }
    }
    Some(languages)
}

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

fn kept_pages_warning(kept: &[(u32, Vec<Script>)], languages: &[String]) -> crate::types::ProcessingWarning {
    warning(format!(
        "OCR was skipped for {}: the native text is in a script the configured OCR languages \
         ({}) cannot read, so it was kept as extracted, and glyphs it has no Unicode value for stay lost. \
         Add a model for this script, for example the one named, to ocr.language to OCR it.",
        describe(kept),
        languages.join("+")
    ))
}

fn read_pages_warning(read: &[(u32, Vec<Script>)], languages: &[String]) -> crate::types::ProcessingWarning {
    warning(format!(
        "The native text of {} is in a script the configured OCR languages ({}) cannot read, and the \
         page is read by OCR because it carries a picture. Text that OCR reads in this script will be \
         wrong. Add a model for this script, for example the one named, to ocr.language.",
        describe(read),
        languages.join("+")
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
    let Some(languages) = route_languages(ocr) else {
        return Vec::new();
    };
    let candidates: Vec<(u32, Vec<Script>)> = decision
        .failing_pages
        .iter()
        .filter_map(|page| {
            let boundary = boundaries.iter().find(|boundary| boundary.page_number == *page)?;
            let text = native_text.get(boundary.byte_start..boundary.byte_end)?;
            Some((*page, unreadable_scripts(text, &languages)?))
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
        .partition(|(page, _)| page_is_scan(&document, *page as usize - 1));
    let mut warnings = Vec::new();
    if !kept.is_empty() {
        decision
            .failing_pages
            .retain(|page| !kept.iter().any(|(kept_page, _)| kept_page == page));
        decision.fallback = !decision.failing_pages.is_empty();
        decision.whole_doc_failure = decision.failing_pages.len() == boundaries.len();
        warnings.push(kept_pages_warning(&kept, &languages));
    }
    if !read.is_empty() {
        warnings.push(read_pages_warning(&read, &languages));
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
    fn a_pipeline_of_stages_that_cannot_read_the_script_keeps_the_native_page() {
        let config = pipeline_config(serde_json::json!({"stages": [
            {"backend": "tesseract", "priority": 100},
            {"backend": "tesseract", "priority": 50, "language": ["eng", "deu"]},
        ]}));
        let (decision, warnings) = run(&[KHMER_MARKED], ONE_PAGE, &config);
        assert!(decision.failing_pages.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("(eng+deu)"), "{}", warnings[0].message);
    }

    #[test]
    fn a_pipeline_stage_that_reads_the_script_or_makes_no_claim_leaves_the_page_to_ocr() {
        for pipeline in [
            serde_json::json!({"stages": [
                {"backend": "tesseract", "priority": 100},
                {"backend": "tesseract", "priority": 50, "language": ["khm"]},
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
    fn english_decoded_through_a_greek_symbol_font_is_outside_what_script_evidence_can_tell() {
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
        let marked = format!("{symbol_font}\u{FFFD}");
        let (text, boundaries) = page_text(&[&marked]);
        // The language check abstains on Greek-letter gibberish, so nothing flags this layer. Script
        // evidence cannot tell it from real Greek text: the filter keeps the page, as it would any
        // layer that reads as one unconfigured script. A wrongly decoded layer in a script the user
        // did not configure is therefore out of scope; a flagged one is returned to OCR (see
        // `a_flagged_mis_decode_returns_to_ocr_after_the_filter`).
        let scan = super::super::ocr::scan_text_plausibility(&text, Some(&boundaries), Some(1), &Default::default());
        assert_eq!(
            (scan.judged, scan.implausible.clone(), scan.unjudged.clone()),
            (0, vec![], vec![1])
        );
        let (decision, _) = run(&[&marked], ONE_PAGE, &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
    }
}
