//! Which characters of a native page count as lost glyphs when the page is routed to OCR or kept.

use super::scoring::{NativeTextStats, evaluate_native_text_for_ocr, evaluate_per_page_ocr};
use crate::core::config::OcrQualityThresholds;
use crate::extractors::pdf::unmapped_glyphs::LOST_SYMBOL_GLYPH_MARKER;
use crate::types::PageBoundary;

/// A font the decoder cannot name a glyph of writes U+FFFF for that glyph (public docling.pdf
/// does for its ligatures). That is text the page keeps, not a lost-glyph marker: the garbage
/// count stays 0 and a readable page stays on the native route. ~keep
#[test]
fn should_keep_the_native_route_for_a_page_whose_decode_writes_u_ffff_for_an_unmapped_ligature() {
    let thresholds = OcrQualityThresholds::default();
    let sentence = "The committee reviewed the con\u{FFFF}gurations of the e\u{FFFF}ort and the \u{FFFF}nal \
                    \u{FFFF}ag that \u{FFFF}attens the work the sta\u{FFFF} introduced, which signi\u{FFFF}cantly \
                    changed the plan. ";
    let prose = "Each section of the report describes the measured values and the method in plain words. ";
    let text = format!("{sentence}{}", prose.repeat(120));
    assert!(
        text.chars().filter(|c| !c.is_whitespace()).count() > 8000,
        "a long readable page"
    );
    assert_eq!(text.matches('\u{FFFF}').count(), 7, "seven words hold the glyph");
    let stats = NativeTextStats::compute(&text, &thresholds);
    assert_eq!(stats.garbage_char_count, 0, "U+FFFF is no lost-glyph marker");
    assert_eq!(
        NativeTextStats::compute(&text.replace('\u{FFFF}', "\u{FFFD}"), &thresholds).garbage_char_count,
        7,
        "control: the same page with lost-glyph markers counts them"
    );
    let decision = evaluate_native_text_for_ocr(&text, Some(1), &thresholds);
    assert!(
        !decision.fallback,
        "the page stays on the native route: failing pages {:?}, garbage {}",
        decision.failing_pages, decision.stats.garbage_char_count
    );
}

/// Lost symbol glyphs outside every page keep the aggregate check, as U+FFFD markers do. ~keep
#[test]
fn should_keep_the_aggregate_fallback_for_lost_symbol_glyphs_outside_the_page_boundaries() {
    let thresholds = OcrQualityThresholds::default();
    let first_page = "This first page contains reliable searchable measurements and explanatory text.";
    let garbage = LOST_SYMBOL_GLYPH_MARKER
        .to_string()
        .repeat(thresholds.min_garbage_chars);
    let second_page = "This second page also contains reliable searchable measurements and explanatory text.";
    let text = format!("{first_page}{garbage}{second_page}");
    let boundaries = vec![
        PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: first_page.len(),
        },
        PageBoundary {
            page_number: 2,
            byte_start: first_page.len() + garbage.len(),
            byte_end: text.len(),
        },
    ];

    let decision = evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &thresholds);

    assert!(decision.fallback);
    assert!(decision.whole_doc_failure);
    assert!(decision.failing_pages.is_empty());
}
