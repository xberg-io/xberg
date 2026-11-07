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

/// The script of an alphabetic character; `None` when the character is not alphabetic or its
/// script is not one this module tells apart.
fn script_of(character: char) -> Option<Script> {
    if !character.is_alphabetic() {
        return None;
    }
    let code = character as u32;
    let script = match code {
        0x0041..=0x024F | 0x1E00..=0x1EFF | 0xFB00..=0xFB06 | 0xFF21..=0xFF5A => Script::Latin,
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

/// The scripts of `text`'s letters when every letter is in a script that none of `languages`
/// covers. `None` when any letter is in a covered or unrecognised script, when `text` has no
/// letters, or when a configured language is unknown: each leaves OCR a legitimate claim on part
/// of the page.
fn unreadable_scripts(text: &str, languages: &[String]) -> Option<Vec<Script>> {
    let mut covered = Vec::new();
    for model in languages.iter().flat_map(|language| language.split('+')) {
        covered.extend_from_slice(language_scripts(model)?);
    }
    let mut found: Vec<Script> = Vec::new();
    for character in text.chars().filter(|character| character.is_alphabetic()) {
        let script = script_of(character)?;
        if covered.contains(&script) {
            return None;
        }
        if !found.contains(&script) {
            found.push(script);
        }
    }
    (!found.is_empty()).then_some(found)
}

/// Remove from the quality gate's failing pages every page that OCR with the configured
/// languages cannot improve: a page whose native text carries lost-glyph markers and whose
/// letters are all in scripts none of the languages covers. Pages that stay failing are
/// unchanged, and the document-level flags follow from what remains.
pub(super) fn retain_unreadable_script_pages(
    decision: &mut OcrFallbackDecision,
    native_text: &str,
    boundaries: Option<&[PageBoundary]>,
    page_count: Option<u32>,
    config: &ExtractionConfig,
) {
    let Some(ocr) = config.ocr.as_ref() else {
        return;
    };
    let Some(boundaries) = boundaries.filter(|boundaries| {
        !decision.failing_pages.is_empty()
            && page_count.is_some_and(|count| count as usize == boundaries.len())
            && boundaries
                .iter()
                .enumerate()
                .all(|(index, boundary)| boundary.page_number as usize == index + 1)
    }) else {
        return;
    };
    if !(ocr.enabled
        && ocr.backend == "tesseract"
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled)
    {
        return;
    }
    let languages = ocr.effective_tesseract_language();
    let unreadable = |page: u32| {
        boundaries
            .iter()
            .find(|boundary| boundary.page_number == page)
            .and_then(|boundary| native_text.get(boundary.byte_start..boundary.byte_end))
            .is_some_and(|text| {
                text.contains(super::unmapped_glyphs::LOST_GLYPH_MARKER)
                    && unreadable_scripts(text, &languages).is_some()
            })
    };
    let before = decision.failing_pages.len();
    decision.failing_pages.retain(|page| !unreadable(*page));
    if decision.failing_pages.len() != before {
        decision.fallback = !decision.failing_pages.is_empty();
        decision.whole_doc_failure = decision.failing_pages.len() == boundaries.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn langs(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|code| (*code).to_string()).collect()
    }

    #[test]
    fn reports_scripts_that_no_configured_language_reads() {
        assert_eq!(
            unreadable_scripts("ខ្ញុំ� កម្ពុជា", &langs(&["eng"])),
            Some(vec![Script::Khmer])
        );
        assert_eq!(
            unreadable_scripts("Привет мир", &langs(&["eng", "deu"])),
            Some(vec![Script::Cyrillic])
        );
        assert_eq!(
            unreadable_scripts("English text", &langs(&["khm"])),
            Some(vec![Script::Latin])
        );
        assert_eq!(
            unreadable_scripts("ខ្ញុំ� α", &langs(&["eng"])),
            Some(vec![Script::Khmer, Script::Greek])
        );
        assert_eq!(
            unreadable_scripts("日本語のテキスト", &langs(&["eng"])),
            Some(vec![Script::Han, Script::Kana])
        );
    }

    #[test]
    fn claims_nothing_when_any_letter_or_language_leaves_ocr_a_claim() {
        let khmer = "ខ្ញុំ� កម្ពុជា";
        for (text, languages) in [
            (khmer, langs(&["eng", "khm"])),
            (khmer, langs(&["eng+khm"])),
            (khmer, langs(&["khmer"])),
            ("ខ្ញុំ� A", langs(&["eng"])),
            ("English� prose", langs(&["eng"])),
            ("��", langs(&["eng"])),
            (khmer, langs(&["zzz"])),
            (khmer, langs(&["eng", "zzz"])),
            ("日本語", langs(&["jpn"])),
        ] {
            assert_eq!(unreadable_scripts(text, &languages), None, "{text} {languages:?}");
        }
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

    const KHMER_MARKED: &str = "ខ្ញុំ\u{FFFD}\u{FFFD} ស្រលាញ់\u{FFFD} ប្រទេស\u{FFFD}\u{FFFD} កម្ពុជា\u{FFFD}";

    #[test]
    fn marked_page_in_an_unreadable_script_leaves_the_failing_pages() {
        let (text, boundaries) = page_text(&[KHMER_MARKED, ""]);
        let mut decision = decision_for(&text, &boundaries);
        assert_eq!(
            decision.failing_pages,
            [1, 2],
            "the gate fails both pages before the filter"
        );
        assert!(decision.whole_doc_failure);
        retain_unreadable_script_pages(&mut decision, &text, Some(&boundaries), Some(2), &config_with(&["eng"]));
        assert_eq!(decision.failing_pages, [2]);
        assert!(decision.fallback);
        assert!(!decision.whole_doc_failure);
    }

    #[test]
    fn single_marked_page_in_an_unreadable_script_needs_no_fallback() {
        let (text, boundaries) = page_text(&[KHMER_MARKED]);
        let mut decision = decision_for(&text, &boundaries);
        assert!(decision.fallback);
        retain_unreadable_script_pages(&mut decision, &text, Some(&boundaries), Some(1), &config_with(&["eng"]));
        assert!(decision.failing_pages.is_empty());
        assert!(!decision.fallback);
        assert!(!decision.whole_doc_failure);
    }

    #[test]
    fn page_stays_failing_when_a_language_reads_its_script_or_nothing_marks_it() {
        let unmarked = "ខ្ញុំ ស្រលាញ់ ប្រទេស កម្ពុជា";
        let latin_marked = "Invoice \u{FFFD}\u{FFFD}\u{FFFD} total \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
        for (page, languages) in [
            (KHMER_MARKED, &["eng", "khm"][..]),
            (KHMER_MARKED, &["eng", "zzz"][..]),
            (latin_marked, &["eng"][..]),
            (unmarked, &["eng"][..]),
        ] {
            let (text, boundaries) = page_text(&[page]);
            let mut decision = decision_for(&text, &boundaries);
            let before = (decision.fallback, decision.failing_pages.clone());
            retain_unreadable_script_pages(
                &mut decision,
                &text,
                Some(&boundaries),
                Some(1),
                &config_with(languages),
            );
            assert_eq!(
                (decision.fallback, decision.failing_pages.clone()),
                before,
                "{page:?} {languages:?}"
            );
        }
    }

    #[test]
    fn explicit_pipeline_and_other_backends_are_left_alone() {
        let (text, boundaries) = page_text(&[KHMER_MARKED]);
        let mut paddle = config_with(&["eng"]);
        paddle.ocr.as_mut().unwrap().backend = "paddleocr".to_string();
        let mut vlm = config_with(&["eng"]);
        vlm.ocr.as_mut().unwrap().vlm_fallback = VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 };
        for config in [paddle, vlm] {
            let mut decision = decision_for(&text, &boundaries);
            retain_unreadable_script_pages(&mut decision, &text, Some(&boundaries), Some(1), &config);
            assert_eq!(decision.failing_pages, [1]);
        }
    }
}
