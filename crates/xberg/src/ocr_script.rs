//! Writing systems and the OCR languages that read them.
//!
//! One table maps a configured language code to the scripts it is written in. The candle
//! script-plausibility filter (GH#1676) and the Tesseract language choice (GH#2300) both read it,
//! so the two cannot disagree about which language covers which script.

/// A writing system, coarse-grained: enough to tell which OCR language reads a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Script {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Devanagari,
    Thai,
    Han,
    Kana,
    Hangul,
    Khmer,
}

/// Map a configured OCR language code (ISO 639-1, ISO 639-3, or a Tesseract code) to the
/// script(s) it is written in.
///
/// Returns `None` for a code this table does not recognize. The candle filter reads that as
/// "cannot judge script plausibility for this language list" and disables its script rule
/// rather than risk false positives against a language it cannot map. The Tesseract language
/// choice reads it as "this language does not cover the script".
pub(crate) fn scripts_for_language(lang: &str) -> Option<&'static [Script]> {
    match lang.trim().to_ascii_lowercase().as_str() {
        "en" | "eng" | "de" | "deu" | "ger" | "fr" | "fra" | "fre" | "es" | "spa" | "it" | "ita" | "pt" | "por"
        | "nl" | "nld" | "dut" | "pl" | "pol" | "sv" | "swe" | "tr" | "tur" | "vi" | "vie" | "id" | "ind" => {
            Some(&[Script::Latin])
        }
        "ru" | "rus" | "uk" | "ukr" | "bg" | "bul" | "sr" | "srp" => Some(&[Script::Cyrillic]),
        "el" | "ell" | "gre" => Some(&[Script::Greek]),
        "ar" | "ara" | "fa" | "fas" | "per" | "ur" | "urd" => Some(&[Script::Arabic]),
        "he" | "heb" | "yi" | "yid" => Some(&[Script::Hebrew]),
        "hi" | "hin" | "mr" | "mar" | "ne" | "nep" => Some(&[Script::Devanagari]),
        "th" | "tha" => Some(&[Script::Thai]),
        "km" | "khm" => Some(&[Script::Khmer]),
        "zh" | "zho" | "chi" | "chi_sim" | "chi_tra" => Some(&[Script::Han]),
        "ja" | "jpn" => Some(&[Script::Han, Script::Kana]),
        "ko" | "kor" => Some(&[Script::Hangul, Script::Han]),
        _ => None,
    }
}

/// Classify an already-known-alphabetic character into a [`Script`] by code-point range.
///
/// Returns `None` for a letter outside every range covered here (e.g. Armenian, Ethiopic, Khmer):
/// callers count such a letter as evidence for no script.
#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "ocr"),
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    all(
        not(target_arch = "wasm32"),
        any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
    )
))]
pub(crate) fn script_of_letter(ch: char) -> Option<Script> {
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

/// The Tesseract language that reads a script, for a script no configured language covers.
///
/// Only Khmer and Han have an entry: the two scripts of GH#2300, whose pages come back as Latin
/// letters when they are read with `eng`. Every other script returns `None`, so a document
/// that only holds a few Greek or Cyrillic letters (maths, names) does not read its Latin pages
/// with a second alphabet that has look-alike letters. A script is added here with its own
/// measurement on the harness corpora, not by extending the table. ~keep
#[cfg(all(feature = "pdf", feature = "ocr-pipeline"))]
pub(crate) fn tesseract_language_for_script(script: Script) -> Option<&'static str> {
    match script {
        Script::Khmer => Some("khm"),
        Script::Han => Some("chi_sim"),
        _ => None,
    }
}

/// `true` when one of `languages` reads `script`.
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn languages_cover_script<S: AsRef<str>>(languages: &[S], script: Script) -> bool {
    languages
        .iter()
        .any(|lang| scripts_for_language(lang.as_ref()).is_some_and(|scripts| scripts.contains(&script)))
}

/// `true` when the language list is the `all` wildcard, which already reads every script.
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn languages_are_wildcard<S: AsRef<str>>(languages: &[S]) -> bool {
    languages
        .iter()
        .any(|lang| matches!(lang.as_ref().trim().to_ascii_lowercase().as_str(), "all" | "*"))
}

/// Add `language` to whichever language field Tesseract reads, leaving the other field and the
/// precedence between the two as they are.
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn push_language(ocr: &mut crate::core::config::OcrConfig, language: &str) {
    let default = [crate::core::config::ocr::DEFAULT_OCR_LANGUAGE.to_string()];
    if ocr.language != default {
        ocr.language.push(language.to_string());
        return;
    }
    if let Some(tess) = ocr.tesseract_config.as_mut()
        && !tess.language.is_empty()
    {
        tess.language.push(language.to_string());
        return;
    }
    ocr.language.push(language.to_string());
}

#[cfg(all(test, feature = "ocr-pipeline"))]
mod tests {
    use super::*;

    #[test]
    fn a_language_covers_its_own_script_only() {
        assert!(languages_cover_script(&["khm"], Script::Khmer));
        assert!(languages_cover_script(&["eng", "chi_sim"], Script::Han));
        assert!(!languages_cover_script(&["eng"], Script::Khmer));
        assert!(!languages_cover_script(&["deu"], Script::Han));
    }

    #[test]
    fn an_unrecognized_language_covers_nothing() {
        assert!(!languages_cover_script(&["xyz"], Script::Khmer));
        assert!(scripts_for_language("xyz").is_none());
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn khmer_and_han_have_an_ocr_language_that_covers_them() {
        for script in [Script::Khmer, Script::Han] {
            let lang = tesseract_language_for_script(script).expect("a language for the script");
            assert!(languages_cover_script(&[lang], script), "{lang} must cover {script:?}");
        }
    }

    /// Greek and Cyrillic letters have Latin look-alikes: adding their language to a Latin
    /// document turns Latin letters into look-alikes (measured on the harness corpora).
    #[cfg(feature = "pdf")]
    #[test]
    fn a_script_with_latin_look_alikes_adds_no_language() {
        for script in [
            Script::Latin,
            Script::Greek,
            Script::Cyrillic,
            Script::Thai,
            Script::Hangul,
        ] {
            assert_eq!(tesseract_language_for_script(script), None, "{script:?}");
        }
    }

    #[test]
    fn the_wildcard_is_recognised_in_either_spelling() {
        assert!(languages_are_wildcard(&["eng", "ALL"]));
        assert!(languages_are_wildcard(&["*"]));
        assert!(!languages_are_wildcard(&["eng"]));
    }
}
