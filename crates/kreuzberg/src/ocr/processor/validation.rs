//! Image and configuration validation logic.
//!
//! This module handles validation of images, language files, and Tesseract configuration
//! before OCR processing begins.

use crate::ocr::error::OcrError;
use crate::ocr::validation::TESSERACT_SUPPORTED_LANGUAGE_CODES;
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};

/// Validate language configuration and check for traineddata files.
///
/// This function validates that:
/// 1. Language string is not empty
/// 2. Traineddata files exist for all specified languages
///
/// # Arguments
///
/// * `language` - Language code(s) to validate (can be "eng" or "eng+fra" etc.)
/// * `tessdata_path` - Path to tessdata directory
///
/// # Returns
///
/// `Ok(())` if validation passes, otherwise returns an error
pub(super) fn validate_language_and_traineddata(language: &str, tessdata_path: &str) -> Result<(), OcrError> {
    // Validate language before initializing to prevent segfault ~keep
    if language.trim().is_empty() {
        return Err(OcrError::TesseractInitializationFailed(
            "Language cannot be empty. Please specify a valid language code (e.g., 'eng')".to_string(),
        ));
    }

    // Validate language file exists before initializing to prevent segfault ~keep
    if !tessdata_path.is_empty() {
        let languages: Vec<&str> = language.split('+').collect();
        for lang in languages {
            let lang = lang.trim();
            if lang.is_empty() {
                continue;
            }
            let traineddata_path = Path::new(tessdata_path).join(format!("{}.traineddata", lang));
            if !traineddata_path.exists() {
                return Err(OcrError::TesseractInitializationFailed(format!(
                    "Language '{}' not found. Traineddata file does not exist: {}",
                    lang,
                    traineddata_path.display()
                )));
            }
        }
    }

    Ok(())
}

/// Resolve tessdata path from environment or fallback locations.
///
/// Checks TESSDATA_PREFIX environment variable first, then tries common
/// installation paths for macOS, Linux, and Windows.
///
/// # Returns
///
/// Path to tessdata directory if found, otherwise empty string
pub(super) fn resolve_tessdata_path() -> String {
    // 1. TESSDATA_PREFIX env var (explicit override)
    if let Ok(path) = env::var("TESSDATA_PREFIX")
        && !path.is_empty()
    {
        return path;
    }

    // 2. KREUZBERG_CACHE_DIR/tessdata (downloaded by `cache warm` command)
    if let Ok(cache_dir) = env::var("KREUZBERG_CACHE_DIR") {
        let tessdata = PathBuf::from(cache_dir).join("tessdata");
        if tessdata.exists() {
            return tessdata.to_string_lossy().into_owned();
        }
    }

    // 3. Bundled tessdata (compiled-in path from build.rs)
    if let Some(bundled) = option_env!("TESSDATA_PREFIX_BUNDLED") {
        let tessdata = PathBuf::from(bundled).join("tessdata");
        if tessdata.exists() {
            return tessdata.to_string_lossy().into_owned();
        }
    }

    // 4. System fallback paths
    let fallback_paths = [
        "/opt/homebrew/share/tessdata",
        "/opt/homebrew/opt/tesseract/share/tessdata",
        "/usr/local/opt/tesseract/share/tessdata",
        "/usr/share/tesseract-ocr/5/tessdata",
        "/usr/share/tesseract-ocr/4/tessdata",
        "/usr/share/tessdata",
        "/usr/local/share/tessdata",
        r#"C:\Program Files\Tesseract-OCR\tessdata"#,
        r#"C:\ProgramData\Tesseract-OCR\tessdata"#,
    ];

    fallback_paths
        .iter()
        .find(|p| Path::new(p).exists())
        .map(|p| (*p).to_string())
        .unwrap_or_default()
}

/// Resolve all installed Tesseract languages from the tessdata directory.
///
/// Scans the tessdata directory for `*.traineddata` files, filters against
/// known Tesseract language codes (excluding non-language files like `osd`),
/// and returns a `+`-separated language string (e.g., `"eng+fra+deu"`).
///
/// # Arguments
///
/// * `tessdata_path` - Path to the tessdata directory
///
/// # Returns
///
/// A `+`-separated string of installed language codes, or an error if no languages are found.
pub(super) fn resolve_all_installed_languages(tessdata_path: &str) -> Result<String, OcrError> {
    if tessdata_path.is_empty() {
        return Err(OcrError::TesseractInitializationFailed(
            "Cannot resolve installed languages: tessdata path is empty. \
             Set TESSDATA_PREFIX or install Tesseract with language data."
                .to_string(),
        ));
    }

    let tessdata_dir = Path::new(tessdata_path);
    if !tessdata_dir.exists() {
        return Err(OcrError::TesseractInitializationFailed(format!(
            "Tessdata directory does not exist: {}",
            tessdata_path
        )));
    }

    let entries = std::fs::read_dir(tessdata_dir).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!("Failed to read tessdata directory '{}': {}", tessdata_path, e))
    })?;

    // Non-language traineddata files to exclude (special-purpose data, not OCR languages)
    const EXCLUDED: &[&str] = &["osd", "equ"];

    let mut languages: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let lang = file_name.strip_suffix(".traineddata")?;
            if EXCLUDED.contains(&lang) {
                return None;
            }
            if TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(lang) {
                Some(lang.to_string())
            } else {
                None
            }
        })
        .collect();

    if languages.is_empty() {
        return Err(OcrError::TesseractInitializationFailed(format!(
            "No installed Tesseract languages found in '{}'",
            tessdata_path
        )));
    }

    languages.sort();
    Ok(languages.join("+"))
}

/// Strip control characters from text, preserving whitespace.
///
/// Removes control characters (0x00-0x1F, 0x7F) except for newlines, carriage returns, and tabs.
///
/// # Arguments
///
/// * `text` - Text to clean
///
/// # Returns
///
/// Cleaned text with control characters removed
pub(super) fn strip_control_characters(text: &str) -> Cow<'_, str> {
    if text
        .chars()
        .any(|c| matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') && c != '\n' && c != '\r' && c != '\t')
    {
        Cow::Owned(
            text.chars()
                .filter(|c| !matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') || matches!(c, '\n' | '\r' | '\t'))
                .collect(),
        )
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_all_installed_languages_success() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        // Create mock traineddata files
        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("fra.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("deu.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "deu+eng+fra");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_osd() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("osd.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_equ() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("equ.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("notareal.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_all_installed_languages(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_all_installed_languages_empty_path() {
        let result = resolve_all_installed_languages("");
        assert!(result.is_err());
    }

    #[test]
    fn test_strip_control_characters() {
        let input = "Hello\x00World\x01Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorldTest");

        let input_with_newlines = "Hello\nWorld\rTest\t!";
        let output = strip_control_characters(input_with_newlines);
        assert_eq!(output, "Hello\nWorld\rTest\t!");
    }

    #[test]
    fn test_strip_control_characters_all_control() {
        let input = "\x00\x01\x02\x03";
        let output = strip_control_characters(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_control_characters_no_control() {
        let input = "Hello World Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "Hello World Test");
    }

    #[test]
    fn test_strip_control_characters_delete_char() {
        let input = "Hello\x7FWorld";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorld");
    }
}
