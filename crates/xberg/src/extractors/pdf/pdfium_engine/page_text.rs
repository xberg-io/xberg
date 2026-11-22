//! Per-page text handling for the pdfium engine: margin filtering and text clean-up.
//!
//! The pure parts live here so they are testable without a pdfium binary; the pdfium-facing
//! adapter [`page_text_within_margins`] only gathers character boxes and feeds them to those.

use pdfium_render::prelude::*;
use std::borrow::Cow;

use crate::pdf::native::text::PageMarginFractions;

/// Whether both margins are zero, i.e. the whole page is content and no filtering applies.
pub(super) fn margins_are_zero(margins: PageMarginFractions) -> bool {
    margins.top <= 0.0 && margins.bottom <= 0.0
}

/// Distance of a point above the visual bottom edge of the page, as a fraction of the visual
/// height. `u` and `v` are the point's position inside the page's unrotated box (`0.0` at the
/// left/bottom edge, `1.0` at the right/top edge); `quarter_turns` is the page's clockwise
/// `/Rotate` in units of 90 degrees.
///
/// Character boxes are reported in unrotated user space, while the margins are defined on the
/// page as displayed, so a rotated page's visual vertical axis is a different user-space axis.
pub(super) fn visual_bottom_fraction(quarter_turns: u8, u: f32, v: f32) -> f32 {
    match quarter_turns % 4 {
        0 => v,
        1 => 1.0 - u,
        2 => 1.0 - v,
        _ => u,
    }
}

/// Whether a point at `bottom_fraction` above the visual bottom edge lies in the content band
/// left by the margins.
pub(super) fn in_content_band(bottom_fraction: f32, margins: PageMarginFractions) -> bool {
    bottom_fraction >= margins.bottom && bottom_fraction <= 1.0 - margins.top
}

/// Keeps the characters inside the content band. Each item is a character and, when pdfium gave
/// it a box, its [`visual_bottom_fraction`]. A character without a box (a generated space or a
/// line break) follows the decision made for the character before it, so a kept line keeps its
/// breaks and a dropped line leaves none behind.
pub(super) fn filter_chars_by_band(
    chars: impl IntoIterator<Item = (char, Option<f32>)>,
    margins: PageMarginFractions,
) -> String {
    let mut kept = String::new();
    let mut keep_previous = false;
    for (ch, fraction) in chars {
        let keep = fraction.map_or(keep_previous, |fraction| in_content_band(fraction, margins));
        if keep {
            kept.push(ch);
        }
        keep_previous = keep;
    }
    kept
}

/// Backend-agnostic clean-up shared with the native engine: control-character ligature repair,
/// and conversion of embedded HTML markup when the `html` feature is built.
pub(super) fn clean_page_text(raw: String) -> String {
    let repaired = match crate::pdf::text::fix_pdf_control_chars(&raw) {
        Cow::Owned(repaired) => repaired,
        Cow::Borrowed(_) => raw,
    };

    #[cfg(feature = "html")]
    if crate::pdf::text::contains_html_markup(&repaired) {
        return crate::pdf::text::convert_html_page_text(&repaired);
    }

    repaired
}

fn page_quarter_turns(page: &PdfPage<'_>) -> u8 {
    match page.rotation() {
        Ok(PdfPageRenderRotation::Degrees90) => 1,
        Ok(PdfPageRenderRotation::Degrees180) => 2,
        Ok(PdfPageRenderRotation::Degrees270) => 3,
        Ok(PdfPageRenderRotation::None) | Err(_) => 0,
    }
}

/// Text of the characters inside the content band, or `None` when the page box or its size is
/// unavailable and the band cannot be placed.
pub(super) fn page_text_within_margins(
    page: &PdfPage<'_>,
    text: &PdfPageText<'_>,
    margins: PageMarginFractions,
) -> Option<String> {
    let bounds = page.boundaries().bounding().ok()?.bounds;
    let (left, bottom) = (bounds.left().value, bounds.bottom().value);
    let (width, height) = (bounds.width().value, bounds.height().value);
    if !(width > 0.0 && height > 0.0) {
        return None;
    }
    let quarter_turns = page_quarter_turns(page);

    let page_chars = text.chars();
    let chars = page_chars.iter().filter_map(|ch| {
        let unicode = ch.unicode_char()?;
        let fraction = ch.loose_bounds().ok().map(|rect| {
            let center_x = (rect.left().value + rect.right().value) / 2.0;
            let center_y = (rect.bottom().value + rect.top().value) / 2.0;
            visual_bottom_fraction(quarter_turns, (center_x - left) / width, (center_y - bottom) / height)
        });
        Some((unicode, fraction))
    });
    Some(filter_chars_by_band(chars, margins))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn margins(top: f32, bottom: f32) -> PageMarginFractions {
        PageMarginFractions { top, bottom }
    }

    #[test]
    fn should_treat_only_two_zero_margins_as_whole_page() {
        let cases = [
            (0.0, 0.0, true),
            (0.1, 0.0, false),
            (0.0, 0.1, false),
            (0.1, 0.1, false),
        ];
        for (top, bottom, expected) in cases {
            assert_eq!(
                margins_are_zero(margins(top, bottom)),
                expected,
                "top={top} bottom={bottom}"
            );
        }
    }

    #[test]
    fn should_map_user_space_position_to_visual_height_for_each_rotation() {
        // A point near the unrotated left edge (u=0.1) and the unrotated bottom (v=0.2).
        let cases = [(0, 0.2), (1, 0.9), (2, 0.8), (3, 0.1), (4, 0.2), (5, 0.9)];
        for (quarter_turns, expected) in cases {
            let got = visual_bottom_fraction(quarter_turns, 0.1, 0.2);
            assert!(
                (got - expected).abs() < 1e-6,
                "rotation {quarter_turns}: expected {expected}, got {got}"
            );
        }
    }

    #[test]
    fn should_decide_content_band_inclusively_at_the_margin_edges() {
        let m = margins(0.1, 0.2);
        let cases = [(0.19, false), (0.2, true), (0.5, true), (0.9, true), (0.91, false)];
        for (fraction, expected) in cases {
            assert_eq!(in_content_band(fraction, m), expected, "fraction={fraction}");
        }
    }

    #[test]
    fn should_drop_chars_outside_the_band_and_keep_breaks_of_kept_lines() {
        let m = margins(0.1, 0.1);
        let chars = vec![
            ('H', Some(0.95)),
            ('d', Some(0.95)),
            ('\n', None),
            ('B', Some(0.5)),
            ('o', Some(0.5)),
            ('\r', None),
            ('\n', None),
            ('F', Some(0.02)),
            ('t', Some(0.02)),
        ];
        assert_eq!(filter_chars_by_band(chars, m), "Bo\r\n");
    }

    #[test]
    fn should_keep_everything_inside_the_band_including_boxless_chars() {
        let chars = vec![('a', Some(0.5)), (' ', None), ('b', Some(0.5))];
        assert_eq!(filter_chars_by_band(chars, margins(0.1, 0.1)), "a b");
    }

    #[test]
    fn should_drop_a_leading_boxless_char_because_nothing_before_it_was_kept() {
        assert_eq!(
            filter_chars_by_band(vec![('\n', None), ('a', Some(0.5))], margins(0.1, 0.1)),
            "a"
        );
    }

    #[test]
    fn should_repair_etx_ligature_and_drop_other_control_characters() {
        assert_eq!(clean_page_text("So\u{0003}ware\u{0001}".to_string()), "Software");
    }

    #[test]
    fn should_leave_clean_text_untouched() {
        assert_eq!(
            clean_page_text("plain text\nline two".to_string()),
            "plain text\nline two"
        );
    }

    #[cfg(feature = "html")]
    #[test]
    fn should_convert_embedded_html_markup_to_markdown() {
        let cleaned = clean_page_text("<p>Hello <b>world</b></p>".to_string());
        assert!(
            !cleaned.contains("<p>"),
            "html markup should be converted, got {cleaned:?}"
        );
        assert!(cleaned.contains("Hello"));
    }
}
