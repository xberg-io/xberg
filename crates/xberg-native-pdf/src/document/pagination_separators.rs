//! Strict page-local evidence for explicit bottom pagination paint.

use super::paths::FormPathBudget;
use super::*;
use crate::content::{Operator, parser::visit_strict_content_stream};
use crate::elements::{PathContent, PathOperation};

const MAX_SEPARATOR_OBJECT_WORK: usize = 16 * 1024 * 1024;
const MAX_SEPARATOR_CONTENT_BYTES: usize = 1024 * 1024;
// Bound the whole-stream proof, including text and marked content. ~keep
const MAX_SEPARATOR_OPERATORS: usize = 8192;
// Exclude short decorative marks and thicker filled boxes, in page points. ~keep
const MIN_SEPARATOR_WIDTH_PT: f32 = 24.0;
const MAX_SEPARATOR_HEIGHT_PT: f32 = 2.0;

#[derive(Clone, Copy, Default)]
struct SeparatorMarkedContent {
    bottom_attached: bool,
    optional: bool,
}

#[derive(Default)]
pub(super) struct BottomPaginationSeparators {
    marked_content: Vec<SeparatorMarkedContent>,
    clip_stack: Vec<bool>,
    clipped: bool,
    invalid: bool,
    attached_rectangle: bool,
    filled_paths: Vec<usize>,
}

impl BottomPaginationSeparators {
    pub(super) fn observe(
        &mut self,
        operator: &crate::content::Operator,
        extractor: &crate::extractors::paths::PathExtractor,
    ) {
        use crate::content::Operator;

        match operator {
            Operator::BeginMarkedContent { tag } => self.enter_content(tag, false),
            Operator::BeginMarkedContentDict { tag, properties } => {
                self.enter_content(tag, has_explicit_bottom_pagination_attachment(properties));
            }
            Operator::EndMarkedContent => {
                self.invalid |= self.marked_content.pop().is_none();
                self.attached_rectangle = false;
            }
            Operator::SaveState => self.clip_stack.push(self.clipped),
            Operator::RestoreState => {
                if let Some(clipped) = self.clip_stack.pop() {
                    self.clipped = clipped;
                } else {
                    self.invalid = true;
                }
            }
            Operator::ClipNonZero | Operator::ClipEvenOdd => self.clipped = true,
            Operator::Tr { render: 4..=7 } => self.invalid = true,
            Operator::Rectangle { .. } => {
                self.attached_rectangle = !extractor.has_current_path()
                    && self
                        .marked_content
                        .last()
                        .is_some_and(|scope| scope.bottom_attached && !scope.optional);
            }
            Operator::Fill | Operator::FillEvenOdd => {
                if std::mem::take(&mut self.attached_rectangle) && !self.clipped && extractor.has_current_path() {
                    self.filled_paths.push(extractor.path_count());
                }
            }
            Operator::EndPath => self.attached_rectangle = false,
            _ => {}
        }
    }

    fn enter_content(&mut self, tag: &str, bottom_pagination: bool) {
        let parent = self.marked_content.last().copied().unwrap_or_default();
        self.marked_content.push(SeparatorMarkedContent {
            bottom_attached: if tag == "Artifact" {
                bottom_pagination
            } else {
                parent.bottom_attached
            },
            optional: parent.optional || tag == "OC",
        });
        self.attached_rectangle = false;
    }

    fn finish(self, paths: &[crate::elements::PathContent]) -> Vec<crate::geometry::Rect> {
        if self.invalid || !self.marked_content.is_empty() || !self.clip_stack.is_empty() {
            return Vec::new();
        }
        self.filled_paths
            .into_iter()
            .filter_map(|index| paths.get(index))
            .filter(|path| is_separator_path(path))
            .map(|path| path.bbox)
            .collect()
    }
}

fn has_explicit_bottom_pagination_attachment(properties: &Object) -> bool {
    let Some(properties) = properties.as_dict() else {
        return false;
    };
    properties.get("Type").and_then(Object::as_name) == Some("Pagination")
        && properties
            .get("Attached")
            .and_then(Object::as_array)
            .is_some_and(|edges| matches!(edges.as_slice(), [edge] if edge.as_name() == Some("Bottom")))
}

fn is_separator_path(path: &PathContent) -> bool {
    let [PathOperation::Rectangle(x, y, width, height)] = path.operations.as_slice() else {
        return false;
    };
    [x, y, width, height].into_iter().all(|value| value.is_finite())
        && path.layer.is_none()
        && path.has_fill()
        && *width >= MIN_SEPARATOR_WIDTH_PT
        && *height > 0.0
        && *height <= MAX_SEPARATOR_HEIGHT_PT
}

impl PdfDocument {
    /// Return bounds of thin page-local filled rectangles inside an inline
    /// /Artifact dictionary with /Type /Pagination and /Attached [/Bottom].
    /// The tag classifies the paint as bottom-attached pagination, so a footer
    /// decoration can qualify without any relationship to footnote text.
    ///
    /// Requires a complete, bounded single stream parsed without recovery. The
    /// strict scan declines multipart streams and unsupported operators such as
    /// custom color-space operators, shading, and inline images. It does not
    /// descend into Form XObjects, and it requires positive transformed rectangle
    /// dimensions, excluding flipped rectangles and negative transformed sizes.
    #[doc(hidden)]
    pub fn bottom_pagination_separator_bounds(&self, page_index: usize) -> Result<Vec<crate::geometry::Rect>> {
        let mut evidence = BottomPaginationSeparators::default();
        let paths = self.walk_page_paths(page_index, FormPathBudget::new(0, 0, 0), Some(&mut evidence))?;
        Ok(evidence.finish(&paths))
    }

    fn complete_separator_content(&self, page: &HashMap<String, Object>) -> Option<Vec<u8>> {
        let contents = page.get("Contents")?;
        let reference = contents.as_reference()?;
        // The ordinary cache can recover partial streams and omit corrupt array members. ~keep
        let mut remaining = MAX_SEPARATOR_OBJECT_WORK;
        let stream = self.resolve_density_object(contents, &mut remaining)?;
        let Object::Stream { .. } = stream else {
            return None;
        };
        self.decode_form_stream_with_limit(&stream, reference, MAX_SEPARATOR_CONTENT_BYTES)
            .ok()
    }

    pub(super) fn separator_operators(&self, page: &HashMap<String, Object>) -> Option<Vec<Operator>> {
        let content = self.complete_separator_content(page)?;
        let mut operators = Vec::new();
        let complete = visit_strict_content_stream(&content, MAX_SEPARATOR_OPERATORS, |operator| {
            operators.push(operator);
            true
        });
        matches!(complete, Ok(true)).then_some(operators)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::Color;

    #[test]
    fn bottom_pagination_query_requires_rectangle_to_start_a_path() {
        use crate::extractors::paths::PathExtractor;

        let mut evidence = BottomPaginationSeparators::default();
        evidence.enter_content("Artifact", true);
        let rectangle = Operator::Rectangle {
            x: 70.0,
            y: 90.0,
            width: 144.0,
            height: 0.5,
        };
        let mut extractor = PathExtractor::new();
        evidence.observe(&rectangle, &extractor);
        assert!(evidence.attached_rectangle);

        extractor.move_to(70.0, 89.0);
        evidence.observe(&rectangle, &extractor);
        assert!(!evidence.attached_rectangle);
    }

    #[test]
    fn bottom_pagination_query_rejects_ineligible_path_metadata() {
        let rect = |x, y, width, height| {
            let mut path = PathContent::new(Rect::new(70.0, 90.0, 144.0, 0.5)).with_fill(Color::black());
            path.push(PathOperation::Rectangle(x, y, width, height));
            path
        };
        let eligible = rect(70.0, 90.0, 144.0, 0.5);
        assert!(is_separator_path(&eligible));
        assert!(!is_separator_path(&eligible.clone().with_layer("Footers")));
        let mut stroke_only = eligible;
        stroke_only.fill_color = None;
        assert!(!is_separator_path(&stroke_only));

        for path in [
            rect(f32::INFINITY, 90.0, 144.0, 0.5),
            rect(70.0, f32::NAN, 144.0, 0.5),
            rect(70.0, 90.0, f32::INFINITY, 0.5),
            rect(70.0, 90.0, 144.0, f32::NAN),
        ] {
            assert!(!is_separator_path(&path), "{path:?}");
        }
    }
}
