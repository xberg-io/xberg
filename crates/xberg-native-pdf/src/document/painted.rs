//! Painted page content that text can sit on.
//!
//! White text is still the page's content when something painted lies under it: labels on a dark
//! box or a photo. Over nothing it cannot be seen, and extraction drops it
//! (`TextExtractor::invisible_text_page`).

use super::PdfDocument;
use crate::content::Operator;
use crate::geometry::Rect;
use crate::layout::Color;

/// Operators read when auditing a page stream for paint the path walker cannot colour.
const MAX_AUDIT_OPERATORS: usize = 1_000_000;
/// Smallest channel value of a white fill or stroke.
const WHITE_PAINT_MIN: f32 = 0.999;

impl PdfDocument {
    /// The regions of `page_index` covered by painted content other than white: filled or stroked
    /// paths of any other colour and every image. `None` when the page paints content whose colour
    /// the path walker does not resolve (a shading, or a fill colour space other than DeviceGray,
    /// DeviceRGB or DeviceCMYK) or the page cannot be read, so the caller keeps all text.
    pub(crate) fn painted_regions(&self, page_index: usize) -> Option<Vec<Rect>> {
        let content = self.cached_page_content(page_index).ok()?;
        let resolvable =
            crate::content::parser::visit_complete_content_stream(&content, MAX_AUDIT_OPERATORS, |op| match op {
                Operator::PaintShading { .. } => false,
                Operator::SetFillColorSpace { name } => is_device_space(&name),
                _ => true,
            })
            .ok()?;
        if !resolvable {
            return None;
        }
        let mut regions: Vec<Rect> = self
            .extract_paths(page_index)
            .ok()?
            .iter()
            .filter(|path| {
                path.fill_color.is_some_and(|color| !is_white(color))
                    || (path.has_stroke() && path.stroke_color.is_some_and(|color| !is_white(color)))
            })
            .map(crate::elements::PathContent::rendered_bbox)
            .collect();
        regions.extend(self.page_image_handles(page_index).ok()?.iter().map(|image| image.bbox));
        Some(regions)
    }
}

fn is_device_space(name: &str) -> bool {
    matches!(name, "DeviceGray" | "DeviceRGB" | "DeviceCMYK")
}

fn is_white(color: Color) -> bool {
    [color.r, color.g, color.b]
        .iter()
        .all(|channel| *channel >= WHITE_PAINT_MIN)
}

/// Whether `region` and `bbox` share an area.
pub(crate) fn overlaps(region: &Rect, bbox: &Rect) -> bool {
    region.x < bbox.x + bbox.width
        && bbox.x < region.x + region.width
        && region.y < bbox.y + bbox.height
        && bbox.y < region.y + region.height
}
