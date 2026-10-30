//! The OCR language of a page chosen from the script of its document's own text layer (GH#2300).
//!
//! OCR reads a page with the configured languages only. A document whose text layer is Khmer or
//! Chinese, and that goes to OCR with the default `eng`, returns Latin letters. This module adds
//! the language of the document's script to the languages the caller set. It never removes or
//! reorders a configured language, and it never changes which of `OcrConfig.language` and
//! `tesseract_config.language` wins (#1644, #1572).

use crate::core::config::{ExtractionConfig, OcrConfig};
#[cfg(feature = "ocr")]
use crate::ocr::processor::validation::read_again_languages_are_on_disk;
use crate::ocr_script::{Script, languages_are_wildcard, languages_cover_script, tesseract_language_for_script};

/// Fewest non-ASCII letters the text layer must hold before its script counts as evidence.
/// A few stray symbols in an English document must not add a language.
const MIN_SCRIPT_LETTERS: usize = 20;

/// Most non-ASCII letters read to decide the script. Detection cost stays flat on long documents.
const MAX_SCRIPT_LETTERS: usize = 8192;

/// Map the script `whatlang` reports to the coarse script the language table uses.
fn script_from_detected(script: whatlang::Script) -> Option<Script> {
    use whatlang::Script as Detected;
    match script {
        Detected::Latin => Some(Script::Latin),
        Detected::Cyrillic => Some(Script::Cyrillic),
        Detected::Greek => Some(Script::Greek),
        Detected::Arabic => Some(Script::Arabic),
        Detected::Hebrew => Some(Script::Hebrew),
        Detected::Devanagari => Some(Script::Devanagari),
        Detected::Thai => Some(Script::Thai),
        Detected::Khmer => Some(Script::Khmer),
        Detected::Mandarin => Some(Script::Han),
        Detected::Hiragana | Detected::Katakana => Some(Script::Kana),
        Detected::Hangul => Some(Script::Hangul),
        _ => None,
    }
}

/// The script of the non-ASCII letters in `text`, or `None` when there are too few of them.
///
/// ASCII is left out on purpose. Every document holds digits, URLs and Latin names, and they
/// would drown a page-sized run of Khmer in a mixed document.
pub(super) fn text_layer_script(text: &str) -> Option<Script> {
    let sample: String = text
        .chars()
        .filter(|c| !c.is_ascii() && c.is_alphabetic())
        .take(MAX_SCRIPT_LETTERS)
        .collect();
    if sample.chars().count() < MIN_SCRIPT_LETTERS {
        return None;
    }
    whatlang::detect_script(&sample).and_then(script_from_detected)
}

#[cfg(not(feature = "ocr"))]
fn read_again_languages_are_on_disk(_ocr: &OcrConfig, _language: &str) -> bool {
    true
}

/// The language to add for a document whose text layer is `native_text`, or `None` when the
/// configured languages already read its script, the backend is not Tesseract, or the text layer
/// gives no evidence of a non-Latin script.
pub(super) fn language_for_text_layer(ocr: &OcrConfig, native_text: &str) -> Option<&'static str> {
    if ocr.backend != "tesseract" {
        return None;
    }
    let languages = ocr.effective_tesseract_language();
    if languages_are_wildcard(&languages) {
        return None;
    }
    let script = text_layer_script(native_text)?;
    if languages_cover_script(&languages, script) {
        return None;
    }
    tesseract_language_for_script(script)
}

/// A copy of `config` whose OCR languages also read the script of `native_text`, or `None` when
/// the configured languages need no help. Callers pass the copy only to the routes that choose
/// to OCR a page on their own; an explicit `force_ocr` request is not re-decided. A config with
/// no `ocr` block is left alone: giving it one would turn on the gates that an explicit block
/// switches on.
pub(crate) fn config_with_text_layer_language(
    config: &ExtractionConfig,
    native_text: &str,
) -> Option<ExtractionConfig> {
    config_with_language_where(config, native_text, read_again_languages_are_on_disk)
}

/// [`config_with_text_layer_language`] with the disk check given as an argument.
fn config_with_language_where(
    config: &ExtractionConfig,
    native_text: &str,
    on_disk: impl Fn(&OcrConfig, &str) -> bool,
) -> Option<ExtractionConfig> {
    let ocr = config.ocr.as_ref()?;
    let language = language_for_text_layer(ocr, native_text)?;
    if !on_disk(ocr, language) {
        return None;
    }
    let mut with_language = config.clone();
    let mut ocr = ocr.clone();
    crate::ocr_script::push_language(&mut ocr, language);
    with_language.ocr = Some(ocr);
    Some(with_language)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KHMER: &str = "ភាសាខ្មែរគឺជាភាសាផ្លូវការរបស់ប្រទេសកម្ពុជា ហើយត្រូវបានប្រើប្រាស់ជាភាសាសំខាន់។";
    const CHINESE: &str = "今天天气很好我们一起去公园散步然后在湖边吃午饭大家都很高兴";
    const ENGLISH: &str = "The quick brown fox jumps over the lazy dog near the river bank.";

    fn with_language(config: &ExtractionConfig, text: &str) -> Option<ExtractionConfig> {
        config_with_language_where(config, text, |_, _| true)
    }

    #[test]
    fn a_language_that_is_not_on_disk_is_not_added() {
        let config = config_with(Some(OcrConfig::default()));
        assert!(config_with_language_where(&config, KHMER, |_, _| false).is_none());
    }

    /// A tessdata directory that holds the named models as empty files, and an OCR config that
    /// reads `languages` from it.
    #[cfg(feature = "ocr")]
    fn ocr_over(models: &[&str], languages: &[&str]) -> (tempfile::TempDir, OcrConfig) {
        let dir = tempfile::tempdir().unwrap();
        for model in models {
            std::fs::write(dir.path().join(format!("{model}.traineddata")), b"").unwrap();
        }
        let ocr = OcrConfig {
            language: languages.iter().map(|l| l.to_string()).collect(),
            tessdata_path: Some(dir.path().to_path_buf()),
            ..Default::default()
        };
        (dir, ocr)
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn the_added_language_is_on_disk_when_the_directory_holds_the_whole_list() {
        let (_dir, ocr) = ocr_over(&["eng", "added-test-lang"], &["eng"]);
        assert!(read_again_languages_are_on_disk(&ocr, "added-test-lang"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn an_added_language_that_is_not_in_any_directory_is_not_on_disk() {
        let (_dir, ocr) = ocr_over(&["eng"], &["eng"]);
        assert!(!read_again_languages_are_on_disk(&ocr, "absent-test-lang"));
    }

    /// The whole list is checked: the added model is present, a configured one is not.
    #[cfg(feature = "ocr")]
    #[test]
    fn a_configured_language_that_is_missing_keeps_the_added_language_off() {
        let (_dir, ocr) = ocr_over(&["added-test-lang"], &["absent-configured-lang"]);
        assert!(!read_again_languages_are_on_disk(&ocr, "added-test-lang"));
    }

    /// The route the product calls: the disk check is the real one, not a stub.
    #[cfg(feature = "ocr")]
    #[test]
    fn the_text_layer_language_joins_only_when_every_model_is_on_disk() {
        let (_dir, ocr) = ocr_over(&["eng", "khm"], &["eng"]);
        let out = config_with_text_layer_language(&config_with(Some(ocr)), KHMER).expect("khmer is added");
        assert_eq!(out.ocr.expect("ocr block").language, ["eng", "khm"]);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn the_text_layer_language_is_not_added_when_a_configured_model_is_missing() {
        let (_dir, ocr) = ocr_over(&["khm"], &["absent-configured-lang"]);
        assert!(config_with_text_layer_language(&config_with(Some(ocr)), KHMER).is_none());
    }

    fn config_with(ocr: Option<OcrConfig>) -> ExtractionConfig {
        ExtractionConfig {
            ocr,
            ..Default::default()
        }
    }

    #[test]
    fn a_khmer_text_layer_is_khmer() {
        assert_eq!(text_layer_script(KHMER), Some(Script::Khmer));
    }

    #[test]
    fn a_chinese_text_layer_is_han() {
        assert_eq!(text_layer_script(CHINESE), Some(Script::Han));
    }

    #[test]
    fn an_english_text_layer_gives_no_evidence() {
        assert_eq!(text_layer_script(ENGLISH), None);
    }

    #[test]
    fn a_few_stray_letters_give_no_evidence() {
        let text = format!("{ENGLISH} ភាសា");
        assert_eq!(text_layer_script(&text), None);
    }

    #[test]
    fn khmer_pages_in_an_english_document_still_give_evidence() {
        let text = format!("{ENGLISH} 1234 http://example.org {KHMER}");
        assert_eq!(text_layer_script(&text), Some(Script::Khmer));
    }

    #[test]
    fn the_script_language_joins_the_default_language() {
        let config = config_with(Some(OcrConfig::default()));
        let out = with_language(&config, KHMER).expect("khmer is added");
        assert_eq!(out.ocr.as_ref().unwrap().language, ["eng", "khm"]);
        assert_eq!(
            config.ocr.as_ref().unwrap().language,
            ["eng"],
            "the input stays as it was"
        );
    }

    #[test]
    fn a_config_with_no_ocr_block_stays_without_one() {
        assert!(with_language(&config_with(None), CHINESE).is_none());
    }

    #[test]
    fn the_chinese_language_joins_the_default_language() {
        let config = config_with(Some(OcrConfig::default()));
        let out = with_language(&config, CHINESE).expect("han is added");
        assert_eq!(out.ocr.as_ref().unwrap().language, ["eng", "chi_sim"]);
    }

    #[test]
    fn a_caller_language_that_does_not_cover_the_script_keeps_its_place() {
        let ocr = OcrConfig {
            language: vec!["deu".to_string()],
            ..Default::default()
        };
        let out = with_language(&config_with(Some(ocr)), KHMER).expect("khmer is added");
        assert_eq!(out.ocr.as_ref().unwrap().language, ["deu", "khm"]);
    }

    #[test]
    fn a_caller_language_that_covers_the_script_adds_nothing() {
        let ocr = OcrConfig {
            language: vec!["khm".to_string(), "eng".to_string()],
            ..Default::default()
        };
        assert!(with_language(&config_with(Some(ocr)), KHMER).is_none());
    }

    #[test]
    fn a_greek_or_cyrillic_text_layer_adds_no_language() {
        let greek = "Αλφα βήτα γάμμα δέλτα έψιλον ζήτα ήτα θήτα ιώτα κάππα λάμδα";
        let cyrillic = "Съешь же ещё этих мягких французских булок да выпей чаю";
        for text in [greek, cyrillic] {
            let config = config_with(Some(OcrConfig::default()));
            assert!(with_language(&config, text).is_none(), "{text}");
        }
    }

    #[test]
    fn the_wildcard_language_adds_nothing() {
        let ocr = OcrConfig {
            language: vec!["all".to_string()],
            ..Default::default()
        };
        assert!(with_language(&config_with(Some(ocr)), KHMER).is_none());
    }

    #[test]
    fn another_backend_is_left_alone() {
        let ocr = OcrConfig {
            backend: "paddle-ocr".to_string(),
            ..Default::default()
        };
        assert!(with_language(&config_with(Some(ocr)), KHMER).is_none());
    }

    /// The losing field: the tesseract language wins while `OcrConfig.language` is the default,
    /// and the script language must join the winner without making the other one win.
    #[test]
    fn the_tesseract_config_language_wins_when_the_ocr_language_is_the_default() {
        let ocr = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                language: vec!["fra".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(ocr.effective_tesseract_language(), ["fra"]);
        let out = with_language(&config_with(Some(ocr)), KHMER).expect("khmer is added");
        let ocr = out.ocr.as_ref().unwrap();
        assert_eq!(ocr.effective_tesseract_language(), ["fra", "khm"]);
        assert_eq!(ocr.language, ["eng"], "the losing field is not touched");
    }

    #[test]
    fn a_non_default_ocr_language_still_wins_over_the_tesseract_config_language() {
        let ocr = OcrConfig {
            language: vec!["deu".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig {
                language: vec!["fra".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let out = with_language(&config_with(Some(ocr)), KHMER).expect("khmer is added");
        let ocr = out.ocr.as_ref().unwrap();
        assert_eq!(ocr.effective_tesseract_language(), ["deu", "khm"]);
        assert_eq!(ocr.tesseract_config.as_ref().unwrap().language, ["fra"]);
    }
}
