//! Hangul Word Processor (.hwp) extractor.
//!
//! Extracts text content from HWP 5.0 documents using the vendored HWP parser
//! in `crate::extraction::hwp`.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::hwp::model::{CharShape, HwpDocument};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;

/// Extractor for Hangul Word Processor (.hwp) files.
///
/// Supports HWP 5.0 format, the standard document format in South Korea.
pub struct HwpExtractor;

impl HwpExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for HwpExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HwpExtractor {
    fn name(&self) -> &str {
        "hwp-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Hangul Word Processor (.hwp) text extraction"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

fn extract_hwp_content(content: &[u8]) -> Result<HwpDocument> {
    crate::extraction::hwp::extract_hwp_document(content)
        .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to read HWP file: {e}")))
}

/// Build an `InternalDocument` from HWP structured model.
fn build_hwp_internal_document(hwp_doc: &HwpDocument) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("hwp");
    for section in &hwp_doc.sections {
        for para in &section.paragraphs {
            if let Some(ref t) = para.text
                && !t.content.is_empty()
            {
                let annotations = apply_char_shapes(&t.content, &para.char_shape_runs, &hwp_doc.char_shapes);
                if para.outline_level > 0 {
                    // outline_level 1 maps to Heading 1, etc.
                    let idx = builder.push_heading(para.outline_level, &t.content, None, None);
                    if !annotations.is_empty() {
                        builder.set_annotations(idx, annotations);
                    }
                } else {
                    builder.push_paragraph(&t.content, annotations, None, None);
                }
            }
        }
    }

    for (idx, image) in hwp_doc.images.iter().enumerate() {
        let format = match infer::get(&image.data) {
            Some(info) => Cow::Owned(info.mime_type().to_string()),
            None => Cow::Borrowed("application/octet-stream"),
        };

        let extracted = ExtractedImage {
            data: Bytes::from(image.data.clone()),
            format,
            image_index: idx as u32,
            page_number: None,
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: Some(image.name.clone()),
            image_kind: None,
            kind_confidence: None,
            cluster_id: None,
        };
        builder.push_image(None, extracted, None, None);
    }

    builder.build()
}

fn apply_char_shapes(text: &str, runs: &[(u32, u16)], char_shapes: &[CharShape]) -> Vec<TextAnnotation> {
    let mut annotations = Vec::new();
    if runs.is_empty() || char_shapes.is_empty() {
        return annotations;
    }

    let char_indices: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let total_chars = char_indices.len();
    let total_bytes = text.len();

    let mut sorted_runs = runs.to_vec();
    sorted_runs.sort_by_key(|r| r.0);

    for i in 0..sorted_runs.len() {
        let (start_pos, shape_idx) = sorted_runs[i];
        let end_pos = if i + 1 < sorted_runs.len() {
            sorted_runs[i + 1].0
        } else {
            total_chars as u32
        };

        if let Some(shape) = char_shapes.get(shape_idx as usize) {
            let start_byte = char_indices.get(start_pos as usize).cloned().unwrap_or(total_bytes);
            let end_byte = char_indices.get(end_pos as usize).cloned().unwrap_or(total_bytes);

            if start_byte < end_byte {
                if shape.bold {
                    annotations.push(TextAnnotation {
                        start: start_byte as u32,
                        end: end_byte as u32,
                        kind: AnnotationKind::Bold,
                    });
                }
                if shape.italic {
                    annotations.push(TextAnnotation {
                        start: start_byte as u32,
                        end: end_byte as u32,
                        kind: AnnotationKind::Italic,
                    });
                }
                if shape.underline {
                    annotations.push(TextAnnotation {
                        start: start_byte as u32,
                        end: end_byte as u32,
                        kind: AnnotationKind::Underline,
                    });
                }
            }
        }
    }
    annotations
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for HwpExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let hwp_doc = extract_hwp_content(content)?;
        if hwp_doc.sections.is_empty() {
            return Err(crate::KreuzbergError::parsing(
                "no BodyText sections found in HWP document".to_string(),
            ));
        }
        let mut doc = build_hwp_internal_document(&hwp_doc);
        if doc.elements.is_empty() {
            return Err(crate::KreuzbergError::parsing(
                "no BodyText sections found in HWP document".to_string(),
            ));
        }
        doc.mime_type = mime_type.to_string();
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-hwp"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hwp_extractor_plugin_interface() {
        let extractor = HwpExtractor::new();
        assert_eq!(extractor.name(), "hwp-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types(), &["application/x-hwp"]);
    }

    #[test]
    fn test_hwp_extractor_initialize_shutdown() {
        let extractor = HwpExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_apply_char_shapes() {
        let text = "Hello world";
        // Let's say "Hello" is bold, " world" is italic.
        // Hello: 5 chars. " world": 6 chars.
        // runs: start pos to shape_idx.
        // 0 -> 0 (bold)
        // 5 -> 1 (italic)
        let runs = vec![(0, 0), (5, 1)];
        let shape1 = CharShape {
            bold: true,
            ..Default::default()
        };
        let shape2 = CharShape {
            italic: true,
            ..Default::default()
        };
        let char_shapes = vec![shape1, shape2];

        let annotations = apply_char_shapes(text, &runs, &char_shapes);
        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].kind, AnnotationKind::Bold);
        assert_eq!(annotations[0].start, 0);
        assert_eq!(annotations[0].end, 5); // "Hello" is 5 bytes

        assert_eq!(annotations[1].kind, AnnotationKind::Italic);
        assert_eq!(annotations[1].start, 5);
        assert_eq!(annotations[1].end, 11); // " world" is 6 bytes
    }

    #[test]
    fn test_build_hwp_internal_document() {
        use crate::extraction::hwp::model::{HwpImage, ParaText, Paragraph, Section};

        let shape1 = CharShape {
            bold: true,
            ..Default::default()
        };

        let para = Paragraph {
            outline_level: 1, // Heading 1
            text: Some(ParaText {
                content: "My Heading".to_string(),
            }),
            char_shape_runs: vec![(0, 0)],
        };

        let section = Section { paragraphs: vec![para] };

        let image = HwpImage {
            name: "image1.png".to_string(),
            // Fake PNG magic bytes
            data: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        };

        let hwp_doc = HwpDocument {
            char_shapes: vec![shape1],
            sections: vec![section],
            images: vec![image],
        };

        let internal_doc = build_hwp_internal_document(&hwp_doc);

        assert_eq!(internal_doc.elements.len(), 2); // 1 heading + 1 image

        let first_elem = &internal_doc.elements[0];
        match first_elem.kind {
            crate::types::internal::ElementKind::Heading { level } => {
                assert_eq!(level, 1);
                assert_eq!(first_elem.text, "My Heading");
                assert_eq!(first_elem.annotations.len(), 1);
                assert_eq!(first_elem.annotations[0].kind, AnnotationKind::Bold);
            }
            _ => panic!("Expected Heading"),
        }

        let second_elem = &internal_doc.elements[1];
        match second_elem.kind {
            crate::types::internal::ElementKind::Image { image_index } => {
                let i = &internal_doc.images[image_index as usize];
                assert_eq!(i.source_path.as_deref(), Some("image1.png"));
                // infer crate should detect PNG from magic bytes
                assert_eq!(i.format, Cow::Borrowed("image/png"));
            }
            _ => panic!("Expected Image"),
        }
    }

    #[test]
    fn test_hwpx_mime_not_routed_to_hwp_extractor() {
        use crate::KreuzbergError;
        use crate::plugins::registry::DocumentExtractorRegistry;
        use std::sync::Arc;

        let mut registry = DocumentExtractorRegistry::new();
        registry.register(Arc::new(HwpExtractor::new())).unwrap();

        let result = registry.get("application/haansofthwpx");
        assert!(
            matches!(result, Err(KreuzbergError::UnsupportedFormat(_))),
            "application/haansofthwpx must not be routed to HwpExtractor"
        );
    }
}
