//! Simple top-to-bottom, left-to-right reading order strategy.

use crate::error::Result;
use crate::layout::TextSpan;
use crate::pipeline::{OrderedTextSpan, ReadingOrderInfo};

use super::{ReadingOrderContext, ReadingOrderStrategy};

/// Simple top-to-bottom, left-to-right reading order.
///
/// This strategy sorts spans by Y coordinate (descending, so top comes first)
/// then by X coordinate (ascending, so left comes first).
///
/// This is the simplest strategy and works well for single-column documents.
pub struct SimpleStrategy;

impl ReadingOrderStrategy for SimpleStrategy {
    fn apply(&self, spans: Vec<TextSpan>, _context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>> {
        let mut spans_with_index: Vec<_> = spans.into_iter().enumerate().collect();

        spans_with_index.sort_by(|(_, a), (_, b)| {
            let y_cmp = crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y);
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x)
        });

        Ok(spans_with_index
            .into_iter()
            .enumerate()
            .map(|(order, (_, span))| OrderedTextSpan::with_info(span, order, ReadingOrderInfo::simple()))
            .collect())
    }

    fn name(&self) -> &'static str {
        "SimpleStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight};

    fn make_span(text: &str, x: f32, y: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, 50.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            offset_semantic: false,
            split_boundary_before: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn test_simple_ordering() {
        let spans = vec![
            make_span("Bottom", 0.0, 50.0),
            make_span("Top", 0.0, 100.0),
            make_span("Middle", 0.0, 75.0),
        ];

        let strategy = SimpleStrategy;
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Top");
        assert_eq!(ordered[1].span.text, "Middle");
        assert_eq!(ordered[2].span.text, "Bottom");
    }

    #[test]
    fn test_left_to_right_on_same_line() {
        let spans = vec![
            make_span("Right", 100.0, 100.0),
            make_span("Left", 0.0, 100.0),
            make_span("Center", 50.0, 100.0),
        ];

        let strategy = SimpleStrategy;
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Left");
        assert_eq!(ordered[1].span.text, "Center");
        assert_eq!(ordered[2].span.text, "Right");
    }
}
