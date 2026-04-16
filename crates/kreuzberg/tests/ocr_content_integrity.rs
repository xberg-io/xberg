//! Regression test for https://github.com/kreuzberg-dev/kreuzberg/issues/706
//!
//! Tesseract OCR was producing corrupted page content: the top-level `content`
//! field contained the coherent HOCR-rendered text followed by a word-by-word
//! dump of every OcrText element, effectively doubling the output.
//!
//! Root cause: `inject_ocr_elements_from_vec` pushed each OcrElement into
//! `InternalDocument::elements` as an `ElementKind::OcrText`. The rendering
//! pipeline (`render_plain`) iterated those elements and appended every word
//! token back into `content`, on top of the already-rendered HOCR string.
//!
//! Fix: OCR elements are now stored directly in `InternalDocument::prebuilt_ocr_elements`
//! (bypassing the rendering pipeline) and page content is set via
//! `InternalDocument::prebuilt_pages` (bypassing the word-grouped fallback in
//! `build_pages`).

#![cfg(feature = "ocr")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use kreuzberg::extract_file_sync;

/// Content must not be doubled when OCR is enabled.
///
/// Before the fix, `content` contained the HOCR-rendered paragraph text
/// immediately followed by a word-by-word dump of every OcrText element,
/// roughly doubling the word count.  After the fix the two representations
/// must be absent: `content` should equal approximately what is in `pages[0].content`.
#[test]
fn test_ocr_content_not_doubled() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config).expect("OCR extraction must succeed");

    let content_words: Vec<&str> = result.content.split_whitespace().collect();

    // If content is empty there is no text to double — skip the assertion.
    if content_words.is_empty() {
        return;
    }

    // The pages array must be populated.
    let pages = result
        .pages
        .as_ref()
        .expect("pages must be populated when extract_pages=true");
    assert!(!pages.is_empty(), "at least one page must be present");

    let page_content = &pages[0].content;
    let page_words: Vec<&str> = page_content.split_whitespace().collect();

    // Core invariant: top-level content word count must be close to page content
    // word count.  Before the fix, content was roughly 2× the page content because
    // the word-element dump was appended after the HOCR text.
    //
    // Allow a 30 % margin to absorb minor whitespace / formatting differences.
    if !page_words.is_empty() {
        let ratio = content_words.len() as f64 / page_words.len() as f64;
        assert!(
            ratio <= 1.3,
            "content word count ({}) is more than 30% larger than pages[0].content word count ({}). \
             This indicates doubled output — word-token dump appended after HOCR text (issue #706). \
             ratio = {:.2}",
            content_words.len(),
            page_words.len(),
            ratio,
        );
    }

    // Secondary invariant: the content string must NOT contain the page content
    // verbatim twice in a row (i.e. the string is not literally concatenated with
    // itself).
    if page_content.trim().len() > 4 {
        let trimmed = page_content.trim();
        let doubled = format!("{trimmed}{trimmed}");
        assert!(
            !result.content.contains(doubled.as_str()),
            "content appears to contain page text concatenated with itself — doubled output (issue #706)",
        );
    }
}

/// Page content must match the top-level content (after trimming) when there
/// is only one page, for any image that produces non-empty OCR output.
#[test]
fn test_ocr_page_content_matches_top_level_content() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config).expect("OCR extraction must succeed");

    if result.content.trim().is_empty() {
        // No text detected — nothing to assert.
        return;
    }

    let pages = result
        .pages
        .as_ref()
        .expect("pages must be populated when extract_pages=true");
    assert!(!pages.is_empty(), "at least one page must be present");

    // For a single-page image the page content must not be dramatically shorter
    // than the top-level content. Before the fix, top-level content was bloated
    // with the word dump while page content was absent or minimal.
    let top_words = result.content.split_whitespace().count();
    let page_words = pages[0].content.split_whitespace().count();

    if top_words > 0 && page_words > 0 {
        let ratio = top_words as f64 / page_words.max(1) as f64;
        assert!(
            ratio <= 1.3,
            "top-level content ({} words) is more than 30% larger than pages[0].content ({} words). \
             Indicates word-dump appended to top-level content but missing from page — issue #706.",
            top_words,
            page_words,
        );
    }
}
