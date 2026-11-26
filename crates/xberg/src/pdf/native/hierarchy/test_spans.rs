//! Span builders the hierarchy pass tests share.

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

/// An upright span of `size`-point glyphs whose box is one glyph tall.
pub(super) fn span(text: &str, x: f32, y: f32, width: f32, size: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, y, width, size),
        font_size: size,
        ..TextSpan::default()
    }
}

/// Top to bottom, then left to right: the order the extractor hands over.
pub(super) fn rows(mut spans: Vec<TextSpan>) -> Vec<TextSpan> {
    spans.sort_by(|left, right| {
        right
            .bbox
            .y
            .total_cmp(&left.bbox.y)
            .then(left.bbox.x.total_cmp(&right.bbox.x))
    });
    spans
}

pub(super) fn texts(spans: &[TextSpan]) -> Vec<&str> {
    spans.iter().map(|span| span.text.as_str()).collect()
}
