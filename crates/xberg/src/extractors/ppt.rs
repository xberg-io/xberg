//! Native PPT extractor for PowerPoint 97-2003 binary format.
//!
//! Extracts text directly from OLE/CFB compound documents without LibreOffice.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::LEGACY_POWERPOINT_MIME_TYPE;
use crate::extraction::ppt::{PptEmbeddedObject, PptSlideText};
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::{ArchiveEntry, ExtractedImage, ProcessingWarning};
use crate::types::{Metadata, PageInfo, PageStructure, PageUnitType};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg_attr(alef, alef(skip))]
/// Native PPT extractor using OLE/CFB parsing.
///
/// This extractor handles PowerPoint 97-2003 binary (.ppt) files without
/// requiring LibreOffice, providing ~50x faster extraction.
pub struct PptExtractor;

impl PptExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for PptExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Warning source tag for embedded-object failures, distinct from the `ppt` tag the
/// record parser uses so a caller can tell a malformed deck from an unreadable object.
const PPT_EMBEDDED_WARNING_SOURCE: &str = "ppt_embedded_objects";

/// Join a title's outline paragraphs -- kept as `\n` inside `PptSlideText::title`, one
/// `\r` paragraph mark per break -- into the single line a `Slide` node's title is
/// displayed as (xberg-io/xberg#1635): `"Special Databases:\nREACTIONS"` becomes
/// `"Special Databases: REACTIONS"`, not two lines that read as two different slides.
fn join_title_paragraphs(title: &str) -> String {
    title
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// How many of `body`'s leading lines, joined by a single space and trimmed, equal
/// `title`. The outline merge always prepends a recovered title at the very front of the
/// slide's text, and a title the drawing already carried is normally its first shape too --
/// so this looks from the front, and `0` means the title's lines were not found there.
fn leading_lines_matching(body: &str, title: &str) -> usize {
    if title.is_empty() {
        return 0;
    }
    let mut joined = String::new();
    for (index, line) in body.lines().enumerate() {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(line.trim());
        if joined == title {
            return index + 1;
        }
    }
    0
}

impl PptExtractor {
    /// Build an `InternalDocument` from PPT extracted slides and embedded images.
    ///
    /// `slides` carries the deck's real per-slide structure (persist order and numbering,
    /// from `extraction::ppt::extract_texts_from_records`) -- slide numbers are read from
    /// that structure, never re-derived by splitting rendered text (#1418), and each
    /// slide's speaker notes come from `PptSlideText::notes`, resolved by `slideIdRef`
    /// rather than by position among the deck's non-empty notes pages (#1640).
    fn build_internal_document(slides: &[PptSlideText], images: &[ExtractedImage]) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("ppt");

        for slide in slides.iter() {
            let trimmed = slide.text.trim();
            // The file's own outline title (#1635) wins when it states one; the first-line
            // guess below is the fallback for a deck whose title is drawn on the canvas and
            // never entered in the outline view, which has no outline title to read at all.
            let (title, header_lines): (Option<String>, usize) = match slide.title.as_deref() {
                Some(file_title) => {
                    let joined = join_title_paragraphs(file_title);
                    let matched = leading_lines_matching(trimmed, &joined);
                    (Some(joined), matched)
                }
                None => {
                    let mut lines = trimmed.lines();
                    let first_line = lines.next().unwrap_or("");
                    if !first_line.is_empty() && first_line.len() <= 80 && lines.clone().next().is_some() {
                        (Some(first_line.to_string()), 1)
                    } else {
                        (None, 0)
                    }
                }
            };
            builder.push_slide(slide.number, title.as_deref(), None);

            if !trimmed.is_empty() {
                if title.is_some() && header_lines > 0 {
                    // Skip the lines the title already accounts for -- whether the outline
                    // merge prepended them or the drawing already carried them -- so the
                    // title text is never also emitted as a body paragraph.
                    for line in trimmed.lines().skip(header_lines) {
                        let lt = line.trim();
                        if !lt.is_empty() {
                            builder.push_paragraph(lt, vec![], None, None);
                        }
                    }
                } else {
                    builder.push_paragraph(trimmed, vec![], None, None);
                }
            }

            // Inside the slide loop, so an image node sits on the slide that displays it
            // rather than behind the last one -- which is what made a picture-only slide
            // read as an empty slide (#1620). ~keep
            for image in images.iter().filter(|image| image.page_number == Some(slide.number)) {
                builder.push_image(None, image.clone(), image.page_number, None);
            }

            if let Some(notes) = slide.notes.as_deref()
                && !notes.is_empty()
            {
                let key = format!("slide-{}-notes", slide.number);
                builder.push_footnote_definition(notes, &key, None);
            }
        }

        // A blip no live shape referenced resolves to no slide (#1620); it is still
        // extracted, appended here as it always was, rather than dropped. ~keep
        for image in images.iter().filter(|image| image.page_number.is_none()) {
            builder.push_image(None, image.clone(), None, None);
        }

        builder.build()
    }

    /// Identify each recovered OLE object and extract it recursively, as
    /// `extractors::pptx` does for the members of `ppt/embeddings/`.
    ///
    /// A legacy deck names its embedded objects nowhere, so each is pathed by the
    /// position the deck's external-object list declares it at. That number is fixed by
    /// the file, not by how many objects happened to be readable, so one object failing
    /// does not rename the rest.
    ///
    /// An object that cannot be identified or extracted is reported and skipped: one
    /// unreadable object must not cost the caller the deck's text or the other objects.
    async fn extract_embedded_objects(
        objects: &[PptEmbeddedObject],
        config: &ExtractionConfig,
    ) -> (Vec<ArchiveEntry>, Vec<ProcessingWarning>) {
        let mut children = Vec::new();
        let mut warnings = Vec::new();

        let mut child_config = config.clone();
        child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

        for object in objects {
            let path = format!("embedded-object-{}", object.index);
            let Some((inner_bytes, inner_mime)) =
                crate::extraction::ooxml_embedded::extract_ole_embedded_object(&object.data)
            else {
                crate::core::diagnostics::push_warning(
                    &mut warnings,
                    PPT_EMBEDDED_WARNING_SOURCE,
                    format!("Skipped embedded object '{path}': format identification not supported"),
                );
                continue;
            };

            match crate::core::extractor::extract_bytes(&inner_bytes, &inner_mime, &child_config).await {
                Ok(result) => children.push(ArchiveEntry {
                    path,
                    mime_type: inner_mime,
                    result: Box::new(result),
                }),
                Err(e) => crate::core::diagnostics::push_warning(
                    &mut warnings,
                    PPT_EMBEDDED_WARNING_SOURCE,
                    format!("Failed to extract embedded object '{path}': {e}"),
                ),
            }
        }

        (children, warnings)
    }
}

impl Plugin for PptExtractor {
    fn name(&self) -> &str {
        "ppt-extractor"
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
        "Native PPT text extraction via OLE/CFB parsing"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for PptExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let include_master_slides = config.content_filter.as_ref().is_some_and(|f| f.include_headers);
        let extract_images = config.needs_image_data();
        // Recovering an embedded object only to refuse to descend into it would cost the
        // zlib inflate for nothing, so the depth budget is checked before parsing rather
        // than after -- the same budget `extractors::pptx` spends on `ppt/embeddings/`.
        let max_embedded_object_bytes = (config.max_archive_depth > 0).then(|| {
            let security_limits = config.security_limits.clone().unwrap_or_default();
            config
                .max_embedded_file_bytes
                .unwrap_or(security_limits.max_archive_size as u64)
        });

        let result = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::XbergError::Cancelled);
                }
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                    let _guard = span.entered();
                    crate::extraction::ppt::extract_ppt_text_with_options(
                        &content_owned,
                        include_master_slides,
                        extract_images,
                        max_embedded_object_bytes,
                    )
                })
                .await
                .map_err(|e| crate::error::XbergError::parsing(format!("PPT extraction task failed: {e}")))?
            } else {
                crate::extraction::ppt::extract_ppt_text_with_options(
                    content,
                    include_master_slides,
                    extract_images,
                    max_embedded_object_bytes,
                )
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::XbergError::Cancelled);
                }
                crate::extraction::ppt::extract_ppt_text_with_options(
                    content,
                    include_master_slides,
                    extract_images,
                    max_embedded_object_bytes,
                )
            }
        }?;

        let mut metadata_map = AHashMap::new();

        let meta_title = result.metadata.title;
        let meta_subject = result.metadata.subject;

        let (meta_authors, meta_created_by) = if let Some(author) = result.metadata.author {
            (Some(vec![author.clone()]), Some(author))
        } else {
            (None, None)
        };

        let meta_modified_by = result.metadata.last_author;

        metadata_map.insert(
            Cow::Borrowed("slide_count"),
            serde_json::Value::Number(result.slide_count.into()),
        );
        metadata_map.insert(
            Cow::Borrowed("extraction_method"),
            serde_json::Value::String("native_ole".to_string()),
        );

        if !result.speaker_notes.is_empty() {
            metadata_map.insert(
                Cow::Borrowed("speaker_notes"),
                serde_json::Value::Array(
                    result
                        .speaker_notes
                        .iter()
                        .map(|n| serde_json::Value::String(n.clone()))
                        .collect(),
                ),
            );
        }

        let page_structure = if result.slide_count > 0 {
            Some(PageStructure {
                total_count: result.slide_count as u32,
                unit_type: PageUnitType::Slide,
                boundaries: None,
                pages: Some(
                    (1..=result.slide_count)
                        .map(|num| PageInfo {
                            number: num as u32,
                            title: None,
                            dimensions: None,
                            image_count: None,
                            table_count: None,
                            hidden: None,
                            is_blank: None,
                            has_vector_graphics: false,
                        })
                        .collect(),
                ),
            })
        } else {
            None
        };

        let mut doc = Self::build_internal_document(&result.slides, &result.images);
        doc.mime_type = mime_type.to_string();
        doc.processing_warnings.extend(result.processing_warnings);

        if !result.embedded_objects.is_empty() {
            let (children, embed_warnings) = Self::extract_embedded_objects(&result.embedded_objects, config).await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(embed_warnings);
        }

        doc.metadata = Metadata {
            title: meta_title,
            subject: meta_subject,
            authors: meta_authors,
            created_by: meta_created_by,
            modified_by: meta_modified_by,
            pages: page_structure,
            additional: metadata_map,
            ..Default::default()
        };

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[LEGACY_POWERPOINT_MIME_TYPE]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;

    fn image_on(page: Option<u32>, index: u32) -> ExtractedImage {
        ExtractedImage {
            data: bytes::Bytes::from_static(b"png"),
            format: std::borrow::Cow::Borrowed("png"),
            image_index: index,
            page_number: page,
            ..Default::default()
        }
    }

    /// #1620: an image node must sit on the slide that displays it. Emitting every image
    /// after the slide loop put each one behind the last slide, which made a slide whose
    /// only content is a picture read as an empty slide.
    #[test]
    fn should_emit_each_image_node_on_its_own_slide() {
        let slides = vec![
            PptSlideText {
                number: 1,
                text: "First".to_string(),
                title: None,
                notes: None,
            },
            // Picture-only: no text at all, so before the fix this slide had no content.
            PptSlideText {
                number: 2,
                text: String::new(),
                title: None,
                notes: None,
            },
            PptSlideText {
                number: 3,
                text: "Third".to_string(),
                title: None,
                notes: None,
            },
        ];
        let images = vec![image_on(Some(2), 0), image_on(None, 1)];

        let document = PptExtractor::build_internal_document(&slides, &images);

        let order: Vec<String> = document
            .elements
            .iter()
            .filter_map(|element| match &element.kind {
                ElementKind::Slide { number } => Some(format!("slide{number}")),
                ElementKind::Image { .. } => Some("image".to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(
            order,
            vec!["slide1", "slide2", "image", "slide3", "image"],
            "the referenced image belongs to slide 2; the unreferenced one still trails the deck"
        );
    }

    #[tokio::test]
    async fn test_ppt_extractor_plugin_interface() {
        let extractor = PptExtractor::new();
        assert_eq!(extractor.name(), "ppt-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types(), &["application/vnd.ms-powerpoint"]);
    }

    #[tokio::test]
    async fn test_ppt_extractor_initialize_shutdown() {
        let extractor = PptExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_ppt_extractor_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(!result.content.is_empty(), "Should extract text from PPT");
        assert_eq!(&*result.mime_type, "application/vnd.ms-powerpoint");
    }

    #[tokio::test]
    async fn test_ppt_document_structure_slides() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some(), "Should produce document structure for PPT");
        let doc = result.document.unwrap();
        let has_slide = doc
            .nodes
            .iter()
            .any(|n| matches!(n.content, crate::types::document_structure::NodeContent::Slide { .. }));
        assert!(has_slide, "PPT should produce Slide nodes in document structure");
    }

    #[tokio::test]
    async fn test_ppt_slide_count_metadata() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(
            result.metadata.additional.contains_key("slide_count"),
            "Should have slide_count metadata"
        );
        let slide_count = result.metadata.additional.get("slide_count").unwrap();
        assert!(slide_count.as_u64().unwrap_or(0) > 0, "Slide count should be > 0");
    }

    /// PPT speaker notes go to `metadata.additional["speaker_notes"]` as a JSON array,
    /// NOT to `PageContent.speaker_notes`.  The legacy binary format does not support
    /// per-slide `PageContent` objects, so `page_contents` is always `None` for PPT
    /// regardless of whether `page_config` is set.
    #[tokio::test]
    async fn test_ppt_speaker_notes_in_metadata_not_page_contents() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig {
            pages: Some(crate::core::config::PageConfig::default()),
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(
            result.pages.is_none(),
            "PPT should not produce pages; speaker notes are in metadata.additional"
        );
        if let Some(notes) = result.metadata.additional.get("speaker_notes") {
            assert!(notes.is_array(), "PPT speaker_notes in metadata should be a JSON array");
        }
    }

    /// Slide-element `text` (the title, for `ElementKind::Slide`) for slide `number`, or
    /// `None` if no such slide element exists.
    fn slide_element_title(doc: &crate::types::internal::InternalDocument, number: u32) -> Option<String> {
        doc.elements.iter().find_map(|e| match &e.kind {
            ElementKind::Slide { number: n } if *n == number => Some(e.text.clone()),
            _ => None,
        })
    }

    /// Paragraph texts, in document order.
    fn paragraph_texts(doc: &crate::types::internal::InternalDocument) -> Vec<String> {
        doc.elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Paragraph))
            .map(|e| e.text.clone())
            .collect()
    }

    /// xberg-io/xberg#1635: a slide whose only text is its title -- a picture, a diagram, a
    /// section divider -- previously guessed `title: None` because the first-line heuristic
    /// required a *second* line to trust the first as a title. The file's own outline title
    /// carries no such requirement.
    #[test]
    fn should_use_the_outline_title_when_the_slide_has_no_other_text() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Section Divider".to_string(),
            title: Some("Section Divider".to_string()),
            notes: None,
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some("Section Divider".to_string()));
        assert!(
            paragraph_texts(&doc).is_empty(),
            "a title-only slide must not also get a body paragraph repeating the title"
        );
    }

    /// xberg-io/xberg#1635: a title stored as two outline paragraphs (one PowerPoint `\r`
    /// break) must reach the node whole, space-joined -- not truncated to its first line
    /// the way the old first-line heuristic cut `"Special Databases:\nREACTIONS"` down to
    /// `"Special Databases:"`.
    #[test]
    fn should_join_a_two_paragraph_outline_title_instead_of_truncating_it() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Special Databases:\nREACTIONS\nBody bullet one".to_string(),
            title: Some("Special Databases:\nREACTIONS".to_string()),
            notes: None,
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(
            slide_element_title(&doc, 1),
            Some("Special Databases: REACTIONS".to_string())
        );
        assert_eq!(
            paragraph_texts(&doc),
            vec!["Body bullet one".to_string()],
            "only the body bullet remains a paragraph; the title's two lines must not appear there too"
        );
    }

    /// xberg-io/xberg#1635: the old heuristic dropped a title longer than 80 characters
    /// outright (`title: None`). An outline title has no such length cap.
    #[test]
    fn should_use_an_outline_title_longer_than_eighty_characters() {
        let long_title = "A section title that runs well past the eighty character heuristic cutoff used before";
        assert!(long_title.len() > 80, "fixture must exercise the old length cutoff");
        let slides = vec![PptSlideText {
            number: 1,
            text: format!("{long_title}\nBody text"),
            title: Some(long_title.to_string()),
            notes: None,
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some(long_title.to_string()));
        assert_eq!(paragraph_texts(&doc), vec!["Body text".to_string()]);
    }

    /// A deck whose title is drawn on the canvas and never entered in the outline view has
    /// no outline title to read (`PptSlideText::title` is `None`); the first-line heuristic
    /// must still apply exactly as before (xberg-io/xberg#1635 fallback contract).
    #[test]
    fn should_fall_back_to_the_first_line_heuristic_without_an_outline_title() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Drawn Title\nDrawn body".to_string(),
            title: None,
            notes: None,
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some("Drawn Title".to_string()));
        assert_eq!(paragraph_texts(&doc), vec!["Drawn body".to_string()]);
    }

    /// #1418 root-cause regression at the consumer side: `build_internal_document`
    /// must trust the structured `slides` list, never re-split a slide's own
    /// text on `"\n\n"`. A single slide whose text happens to contain an
    /// internal blank line must still produce exactly one `Slide` element.
    #[test]
    fn should_produce_one_slide_element_when_slide_text_contains_internal_blank_line() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Title\n\nBody".to_string(),
            title: None,
            notes: None,
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(
            slide_numbers,
            vec![1],
            "one Slide entry must produce exactly one Slide element, however its text is shaped"
        );
    }

    /// #1418: a slide with no text atoms must still get a `Slide` element
    /// carrying its real persist-order number, not be dropped (which would
    /// shift every later slide's number down).
    #[test]
    fn should_number_slide_elements_by_persist_order_including_an_empty_middle_slide() {
        let slides = vec![
            PptSlideText {
                number: 1,
                text: "Slide One".to_string(),
                title: None,
                notes: None,
            },
            PptSlideText {
                number: 2,
                text: String::new(),
                title: None,
                notes: None,
            },
            PptSlideText {
                number: 3,
                text: "Slide Three".to_string(),
                title: None,
                notes: None,
            },
        ];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(slide_numbers, vec![1, 2, 3]);
    }

    /// #1417: images recovered from the `Pictures` stream must be attached
    /// to the document (`InternalDocument::images`), not silently discarded.
    #[test]
    fn should_attach_images_to_internal_document_when_images_are_present() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Slide One".to_string(),
            title: None,
            notes: None,
        }];
        let image = ExtractedImage {
            data: bytes::Bytes::from_static(b"\xFF\xD8\xFFfake-jpeg"),
            format: Cow::Borrowed("jpeg"),
            image_index: 0,
            page_number: None,
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: None,
            kind_confidence: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        };
        let doc = PptExtractor::build_internal_document(&slides, std::slice::from_ref(&image));

        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].format, "jpeg");
        assert_eq!(&doc.images[0].data[..], b"\xFF\xD8\xFFfake-jpeg");
        let has_image_element = doc
            .elements
            .iter()
            .any(|e| matches!(&e.kind, crate::types::internal::ElementKind::Image { image_index: 0 }));
        assert!(has_image_element, "an Image element must reference the pushed image");
    }

    /// #87/#1418 end-to-end: `simple.ppt` has exactly two `Slide` (0x03EE)
    /// containers (see `extraction::ppt::tests::test_extract_ppt_real_file_reports_two_slides`).
    /// The document structure produced through the real extraction pipeline
    /// must report exactly slide numbers `[1, 2]`, not ordinals derived from
    /// re-splitting joined text.
    #[tokio::test]
    async fn should_report_exact_contiguous_slide_numbers_for_real_ppt_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let doc = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(
            slide_numbers,
            vec![1, 2],
            "simple.ppt has exactly two Slide containers, numbered 1 and 2"
        );
    }
}
