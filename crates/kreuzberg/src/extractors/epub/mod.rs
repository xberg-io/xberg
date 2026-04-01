//! Native EPUB extractor using permissive-licensed dependencies.
//!
//! This extractor provides native Rust-based EPUB extraction without GPL-licensed
//! dependencies, extracting:
//! - Metadata from OPF (Open Packaging Format) using Dublin Core standards
//! - Content from XHTML files in spine order
//! - Proper handling of EPUB2 and EPUB3 formats
//!
//! Uses only permissive-licensed crates:
//! - `zip` (MIT/Apache) - for reading EPUB container
//! - `roxmltree` (MIT) - for parsing XML
//! - `html-to-markdown-rs` (MIT) - for converting XHTML to Markdown/Djot when requested

mod content;
mod metadata;
mod parsing;

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::Metadata;
use crate::types::ProcessingWarning;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::{EpubMetadata, FormatMetadata};
use crate::types::uri::{Uri, UriKind, classify_uri};
use ahash::{AHashMap, AHashSet};
use async_trait::async_trait;
use std::borrow::Cow;
use std::io::{Cursor, Read};
use zip::ZipArchive;

use content::{
    extract_text_from_xhtml, looks_like_navigation_document, strip_document_head, strip_specialized_navigation_sections,
};
use metadata::{build_additional_metadata, parse_opf};
use parsing::{parse_container_xml, read_file_from_zip, resolve_path};

/// EPUB format extractor using permissive-licensed dependencies.
///
/// Extracts content and metadata from EPUB files (both EPUB2 and EPUB3)
/// using native Rust parsing without GPL-licensed dependencies.
pub struct EpubExtractor;

impl EpubExtractor {
    /// Create a new EPUB extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EpubExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
#[allow(dead_code)]
struct RenderedSpineDocument {
    content_fragment: String,
    content_fully_converted: bool,
    document: Option<crate::types::document_structure::DocumentStructure>,
    warnings: Vec<ProcessingWarning>,
}

#[cfg(feature = "office")]
#[allow(dead_code)]
fn trim_trailing_newlines(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

#[cfg(feature = "office")]
#[allow(dead_code)]
impl EpubExtractor {
    fn build_fallback_document_structure(
        document: &content::EpubSpineDocument,
        index: usize,
    ) -> crate::types::document_structure::DocumentStructure {
        use crate::types::builder::DocumentStructureBuilder;

        let mut builder = DocumentStructureBuilder::new().source_format("epub");
        let chapter_title =
            extract_title_from_xhtml(&document.xhtml).unwrap_or_else(|| format!("Chapter {}", index + 1));
        builder.push_heading(1, &chapter_title, None, None);

        let text = extract_text_from_xhtml(&document.xhtml);
        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }

        builder.build()
    }

    /// Render a spine document once.
    fn render_spine_document(
        document: &content::EpubSpineDocument,
        index: usize,
        config: &ExtractionConfig,
    ) -> RenderedSpineDocument {
        let wants_markup = matches!(config.output_format, OutputFormat::Markdown | OutputFormat::Djot);
        let mut warnings = Vec::new();

        let (content_fragment, content_fully_converted) = if wants_markup {
            match crate::extraction::html::convert_html_to_markdown_with_metadata(
                &document.xhtml,
                config.html_options.clone(),
                Some(config.output_format.clone()),
            ) {
                Ok((converted, _)) => (trim_trailing_newlines(&converted).to_string(), true),
                Err(err) => {
                    warnings.push(ProcessingWarning {
                        source: std::borrow::Cow::Borrowed("epub"),
                        message: std::borrow::Cow::Owned(format!(
                            "XHTML conversion failed for spine item '{}'; falling back to plain text: {}",
                            document.file_path, err
                        )),
                    });
                    (extract_text_from_xhtml(&document.xhtml).trim_end().to_string(), false)
                }
            }
        } else {
            (extract_text_from_xhtml(&document.xhtml).trim_end().to_string(), true)
        };

        let document = if config.include_document_structure {
            let chapter_structure = crate::extraction::html::structure::build_document_structure(&document.xhtml);

            if chapter_structure.nodes.is_empty() {
                warnings.push(ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("epub"),
                    message: std::borrow::Cow::Owned(format!(
                        "Document structure extraction produced no nodes for spine item '{}'; falling back to plain-text structure",
                        document.file_path
                    )),
                });
                Some(Self::build_fallback_document_structure(document, index))
            } else {
                Some(chapter_structure)
            }
        } else {
            None
        };

        RenderedSpineDocument {
            content_fragment,
            content_fully_converted,
            document,
            warnings,
        }
    }

    fn build_document_structure(
        rendered_documents: &[RenderedSpineDocument],
    ) -> Option<crate::types::document_structure::DocumentStructure> {
        use crate::types::builder::DocumentStructureBuilder;

        let mut builder = DocumentStructureBuilder::new().source_format("epub");
        let mut has_nodes = false;

        for rendered in rendered_documents {
            let Some(chapter_structure) = &rendered.document else {
                continue;
            };

            for node in &chapter_structure.nodes {
                has_nodes = true;
                builder.push_raw(
                    node.content.clone(),
                    None,
                    None,
                    node.content_layer,
                    node.annotations.clone(),
                );
            }
        }

        if has_nodes { Some(builder.build()) } else { None }
    }

    /// Build an `InternalDocument` from the EPUB spine.
    ///
    /// Walks each chapter's XHTML content using the HTML structure walker,
    /// emitting elements via `InternalDocumentBuilder`. Captures TOC-to-chapter
    /// relationships where possible by linking chapter headings.
    ///
    /// If `cover_image_path` is provided, the cover image is extracted from the
    /// archive and emitted as the first Image element.
    fn build_internal_document(
        archive: &mut ZipArchive<Cursor<Vec<u8>>>,
        spine_hrefs: &[String],
        manifest_dir: &str,
        nav_hrefs: &AHashSet<String>,
        cover_image_path: Option<&str>,
    ) -> Option<InternalDocument> {
        use crate::types::internal::{ElementKind, InternalElement};

        let mut builder = InternalDocumentBuilder::new("epub");

        // Emit cover image as the first element if present
        if let Some(cover_path) = cover_image_path {
            let mut buf = Vec::new();
            if let Ok(mut entry) = archive.by_name(cover_path) {
                let _ = entry.read_to_end(&mut buf);
            }
            if !buf.is_empty() {
                let fmt = cover_path
                    .rsplit('.')
                    .next()
                    .map(|ext| match ext.to_lowercase().as_str() {
                        "jpg" | "jpeg" => "jpeg",
                        "png" => "png",
                        "gif" => "gif",
                        "webp" => "webp",
                        "svg" => "svg",
                        "bmp" => "bmp",
                        _ => "png",
                    })
                    .unwrap_or("png");
                let image = crate::types::ExtractedImage {
                    data: bytes::Bytes::from(buf),
                    format: Cow::Owned(fmt.to_string()),
                    image_index: 0,
                    page_number: Some(0),
                    width: None,
                    height: None,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: Some("Cover".to_string()),
                    ocr_result: None,
                    bounding_box: None,
                    source_path: None,
                };
                builder.push_image(Some("Cover"), image, None, None);
            }
        }

        for (index, href) in spine_hrefs.iter().enumerate() {
            let file_path = match resolve_path(manifest_dir, href) {
                Ok(canonical) => canonical.path,
                Err(_) => continue,
            };

            // Skip EPUB3 navigation documents (TOC, landmarks, page-list)
            if nav_hrefs.contains(&file_path) {
                continue;
            }

            let xhtml_content = match read_file_from_zip(archive, &file_path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let normalized = content::normalize_xhtml(&xhtml_content);
            let sanitized = strip_specialized_navigation_sections(&strip_document_head(&normalized));

            // Skip navigation documents (TOC pages, etc.)
            if looks_like_navigation_document(&sanitized) {
                continue;
            }

            // Skip empty chapters
            if extract_text_from_xhtml(&sanitized).is_empty() {
                continue;
            }

            let chapter_structure = crate::extraction::html::structure::build_document_structure(&sanitized);

            if chapter_structure.nodes.is_empty() {
                // Fallback: extract plain text
                let chapter_title =
                    extract_title_from_xhtml(&xhtml_content).unwrap_or_else(|| format!("Chapter {}", index + 1));
                builder.push_heading(1, &chapter_title, None, None);

                let text = extract_text_from_xhtml(&xhtml_content);
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        builder.push_paragraph(trimmed, vec![], None, None);
                    }
                }
            } else {
                // Convert DocumentStructure nodes to InternalDocument elements
                let mut first_heading_idx: Option<u32> = None;
                let mut in_list = false;
                let mut in_blockquote = false;

                // Build a set of node indices that are Quote containers so we
                // can detect when child nodes are inside a blockquote.
                let quote_indices: AHashSet<crate::types::document_structure::NodeIndex> = chapter_structure
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| matches!(n.content, crate::types::document_structure::NodeContent::Quote))
                    .map(|(i, _)| crate::types::document_structure::NodeIndex(i as u32))
                    .collect();

                // Debugging: print nodes around blockquotes
                for (ni, n) in chapter_structure.nodes.iter().enumerate() {
                    if matches!(n.content, crate::types::document_structure::NodeContent::Quote) {
                        eprintln!("DEBUG: Quote at idx={}, children={:?}", ni, n.children);
                        for ci in &n.children {
                            let cn = &chapter_structure.nodes[ci.0 as usize];
                            eprintln!("  child[{}] parent={:?} content={:?}", ci.0, cn.parent, &format!("{:?}", cn.content)[..80.min(format!("{:?}", cn.content).len())]);
                        }
                    }
                }

                for node in chapter_structure.nodes.iter() {
                    use crate::types::document_structure::NodeContent;

                    // Track blockquote entry/exit based on parent relationships
                    let is_inside_quote = node.parent.as_ref().is_some_and(|p| quote_indices.contains(p));

                    if is_inside_quote && !in_blockquote {
                        builder.push_quote_start();
                        in_blockquote = true;
                    } else if !is_inside_quote && in_blockquote {
                        builder.push_quote_end();
                        in_blockquote = false;
                    }

                    // Close an open list if the current node is not a ListItem
                    if in_list && !matches!(&node.content, NodeContent::ListItem { .. }) {
                        builder.end_list();
                        in_list = false;
                    }

                    match &node.content {
                        // Skip Quote container nodes — we handle them via parent tracking
                        NodeContent::Quote => continue,
                        NodeContent::Heading { level, text } => {
                            let idx = builder.push_heading(*level, text, None, None);
                            if first_heading_idx.is_none() {
                                first_heading_idx = Some(idx);
                            }
                            collect_annotation_uris(&node.annotations, text, &mut builder);
                        }
                        NodeContent::Paragraph { text } => {
                            builder.push_paragraph(text, node.annotations.clone(), None, None);
                            collect_annotation_uris(&node.annotations, text, &mut builder);
                        }
                        NodeContent::ListItem { text } => {
                            if !in_list {
                                builder.push_list(false);
                                in_list = true;
                            }
                            builder.push_list_item(text.as_str(), false, vec![], None, None);
                        }
                        NodeContent::Table { grid } => {
                            let cells: Vec<Vec<String>> = (0..grid.rows)
                                .map(|r| {
                                    grid.cells
                                        .iter()
                                        .filter(|c| c.row == r)
                                        .map(|c| c.content.clone())
                                        .collect()
                                })
                                .collect();
                            if !cells.is_empty() {
                                builder.push_table_from_cells(&cells, None, None);
                            }
                        }
                        NodeContent::Code { text, language } => {
                            builder.push_code(text, language.as_deref(), None, None);
                        }
                        NodeContent::Image { description, src, .. } => {
                            // Collect image URI
                            if let Some(img_src) = src
                                && !img_src.is_empty()
                            {
                                builder.push_uri(Uri {
                                    url: img_src.clone(),
                                    label: description.clone(),
                                    page: Some((index + 1) as u32),
                                    kind: UriKind::Image,
                                });
                            }
                            // Try to extract image binary from the EPUB ZIP.
                            // Image src is relative to the XHTML file, not the manifest dir.
                            let xhtml_dir = file_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                            let image_data = src.as_ref().and_then(|img_src| {
                                let resolved = resolve_path(xhtml_dir, img_src).ok()?;
                                let mut buf = Vec::new();
                                archive.by_name(&resolved.path).ok()?.read_to_end(&mut buf).ok()?;
                                if buf.is_empty() {
                                    return None;
                                }
                                let fmt = img_src
                                    .rsplit('.')
                                    .next()
                                    .map(|ext| match ext.to_lowercase().as_str() {
                                        "jpg" | "jpeg" => "jpeg",
                                        "png" => "png",
                                        "gif" => "gif",
                                        "webp" => "webp",
                                        "svg" => "svg",
                                        "bmp" => "bmp",
                                        _ => "png",
                                    })
                                    .unwrap_or("png");
                                Some((buf, fmt.to_string()))
                            });

                            if let Some((data, format)) = image_data {
                                let image = crate::types::ExtractedImage {
                                    data: bytes::Bytes::from(data),
                                    format: Cow::Owned(format),
                                    image_index: 0,
                                    page_number: Some(index + 1),
                                    width: None,
                                    height: None,
                                    colorspace: None,
                                    bits_per_component: None,
                                    is_mask: false,
                                    description: description.clone(),
                                    ocr_result: None,
                                    bounding_box: None,
                                    source_path: None,
                                };
                                builder.push_image(description.as_deref(), image, None, None);
                            } else {
                                // No image data — emit placeholder
                                let text_val = description.as_deref().unwrap_or("");
                                let elem =
                                    InternalElement::text(ElementKind::Image { image_index: u32::MAX }, text_val, 0);
                                builder.push_element(elem);
                            }
                        }
                        NodeContent::Group {
                            heading_text: Some(_), ..
                        } => {
                            // Skip: the heading text is already emitted by the
                            // Heading node that follows this Group wrapper.
                        }
                        _ => {
                            // Other node types: skip or handle generically
                        }
                    }
                }

                // Close any trailing open list or blockquote
                if in_list {
                    builder.end_list();
                }
                if in_blockquote {
                    builder.push_quote_end();
                }

                // Chapter headings get automatic slug-based anchors from push_heading,
                // enabling TOC entry resolution in the derivation step.
                let _ = first_heading_idx;
            }
        }

        // TOC→chapter relationships: each chapter heading is a TOC target.
        // These anchors are set automatically by push_heading (slug-based),
        // so the derivation step can resolve TOC entries to headings by key.

        Some(builder.build())
    }
}

/// Extract URIs from document structure annotations (link annotations).
#[cfg(feature = "office")]
fn collect_annotation_uris(
    annotations: &[crate::types::document_structure::TextAnnotation],
    text: &str,
    builder: &mut InternalDocumentBuilder,
) {
    use crate::types::document_structure::AnnotationKind;

    for ann in annotations {
        if let AnnotationKind::Link { url, .. } = &ann.kind
            && !url.is_empty()
        {
            let label = if ann.start < ann.end && (ann.end as usize) <= text.len() {
                let slice = &text[ann.start as usize..ann.end as usize];
                if slice.is_empty() {
                    None
                } else {
                    Some(slice.to_string())
                }
            } else {
                None
            };
            builder.push_uri(Uri {
                url: url.clone(),
                label,
                page: None,
                kind: classify_uri(url),
            });
        }
    }
}

/// Extract the first heading text from XHTML content.
#[cfg(feature = "office")]
fn extract_title_from_xhtml(xhtml: &str) -> Option<String> {
    let sanitized = content::normalize_xhtml(xhtml);
    let doc = roxmltree::Document::parse(&sanitized).ok()?;

    for node in doc.root().descendants() {
        if node.is_element() {
            let tag = node.tag_name().name().to_ascii_lowercase();
            if matches!(tag.as_str(), "h1" | "h2" | "h3") {
                let text: String = node
                    .descendants()
                    .filter(|n| n.is_text())
                    .filter_map(|n| n.text())
                    .collect::<Vec<_>>()
                    .join("");
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    None
}

impl Plugin for EpubExtractor {
    fn name(&self) -> &str {
        "epub-extractor"
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
        "Extracts content and metadata from EPUB documents (native Rust implementation with permissive licenses)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for EpubExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "epub", size_bytes = content.len(), "extraction starting");
        let _ = config; // conditionally used by ocr feature
        let cursor = Cursor::new(content.to_vec());

        let mut archive = ZipArchive::new(cursor).map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("Failed to open EPUB as ZIP: {}", e),
            source: None,
        })?;

        let container_xml = read_file_from_zip(&mut archive, "META-INF/container.xml")?;
        let opf_path = parse_container_xml(&container_xml)?;

        let manifest_dir = if let Some(last_slash) = opf_path.rfind('/') {
            opf_path[..last_slash].to_string()
        } else {
            String::new()
        };

        let opf_xml = read_file_from_zip(&mut archive, &opf_path)?;
        let (package, _processing_warnings) = parse_opf(&opf_xml, &manifest_dir)?;
        let additional_metadata = build_additional_metadata(&package.metadata);
        let epub_format_metadata = FormatMetadata::Epub(EpubMetadata {
            coverage: package.metadata.coverage.clone(),
            dc_format: package.metadata.format.clone(),
            relation: package.metadata.relation.clone(),
            source: package.metadata.source.clone(),
            dc_type: package.metadata.dc_type.clone(),
            cover_image: package.metadata.cover_image_href.clone(),
        });

        // Collect nav document hrefs so we can skip them in content extraction
        let nav_hrefs: AHashSet<String> = package
            .manifest
            .values()
            .filter(|item| item.is_nav())
            .filter_map(|item| item.path.clone())
            .collect();

        // Extract spine hrefs for internal document building
        let spine_hrefs: Vec<String> = package
            .spine_items
            .iter()
            .filter_map(|spine_item| {
                package
                    .manifest
                    .get(&spine_item.idref)
                    .map(|item| item.raw_href.clone())
            })
            .collect();

        let metadata_map: AHashMap<Cow<'static, str>, serde_json::Value> = additional_metadata
            .into_iter()
            .map(|(k, v)| (Cow::Owned(k), v))
            .collect();

        // Build InternalDocument from spine chapters
        let cover_image_path = package.metadata.cover_image_href.as_deref();
        let mut doc =
            Self::build_internal_document(&mut archive, &spine_hrefs, &manifest_dir, &nav_hrefs, cover_image_path)
                .unwrap_or_else(|| InternalDocumentBuilder::new("epub").build());
        doc.mime_type = Cow::Owned(mime_type.to_string());

        doc.metadata = Metadata {
            title: package.metadata.title,
            authors: package.metadata.creator.map(|c| vec![c]),
            language: package.metadata.language,
            created_at: package.metadata.date,
            format: Some(epub_format_metadata),
            additional: metadata_map,
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "epub",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/epub+zip",
            "application/x-epub+zip",
            "application/vnd.epub+zip",
        ]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_epub_extractor_plugin_interface() {
        let extractor = EpubExtractor::new();
        assert_eq!(extractor.name(), "epub-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_epub_extractor_default() {
        let extractor = EpubExtractor;
        assert_eq!(extractor.name(), "epub-extractor");
    }

    #[tokio::test]
    async fn test_epub_extractor_initialize_shutdown() {
        let extractor = EpubExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_epub_extractor_supported_mime_types() {
        let extractor = EpubExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"application/epub+zip"));
        assert!(supported.contains(&"application/x-epub+zip"));
        assert!(supported.contains(&"application/vnd.epub+zip"));
    }

    #[test]
    fn test_epub_full_dublin_core_metadata() {
        let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:language>en</dc:language>
    <dc:coverage>Worldwide</dc:coverage>
    <dc:format>application/epub+zip</dc:format>
    <dc:relation>http://example.com/related</dc:relation>
    <dc:source>Original Manuscript</dc:source>
    <dc:type>Text</dc:type>
    <dc:publisher>Test Publisher</dc:publisher>
    <dc:description>A test book</dc:description>
    <dc:rights>CC BY 4.0</dc:rights>
    <meta name="cover" content="cover-img"/>
  </metadata>
  <manifest>
    <item id="cover-img" href="images/cover.jpg" media-type="image/jpeg"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

        let (package, _warnings) = metadata::parse_opf(opf, "").expect("Metadata parse failed");
        let epub_meta = &package.metadata;
        assert_eq!(epub_meta.title, Some("Test Book".to_string()));
        assert_eq!(epub_meta.coverage, Some("Worldwide".to_string()));
        assert_eq!(epub_meta.format, Some("application/epub+zip".to_string()));
        assert_eq!(epub_meta.relation, Some("http://example.com/related".to_string()));
        assert_eq!(epub_meta.source, Some("Original Manuscript".to_string()));
        assert_eq!(epub_meta.dc_type, Some("Text".to_string()));
        assert_eq!(epub_meta.cover_image_href, Some("images/cover.jpg".to_string()));

        // Verify Dublin Core extension fields go into FormatMetadata::Epub
        let format_meta = FormatMetadata::Epub(EpubMetadata {
            coverage: epub_meta.coverage.clone(),
            dc_format: epub_meta.format.clone(),
            relation: epub_meta.relation.clone(),
            source: epub_meta.source.clone(),
            dc_type: epub_meta.dc_type.clone(),
            cover_image: epub_meta.cover_image_href.clone(),
        });
        match &format_meta {
            FormatMetadata::Epub(em) => {
                assert_eq!(em.coverage.as_deref(), Some("Worldwide"));
                assert_eq!(em.dc_format.as_deref(), Some("application/epub+zip"));
                assert_eq!(em.relation.as_deref(), Some("http://example.com/related"));
                assert_eq!(em.source.as_deref(), Some("Original Manuscript"));
                assert_eq!(em.dc_type.as_deref(), Some("Text"));
                assert_eq!(em.cover_image.as_deref(), Some("images/cover.jpg"));
            }
            _ => panic!("Expected FormatMetadata::Epub variant"),
        }

        // Standard Dublin Core fields still go into additional
        let additional = metadata::build_additional_metadata(epub_meta);
        assert!(additional.contains_key("publisher"));
        assert!(additional.contains_key("description"));
        assert!(additional.contains_key("rights"));
        // These should NOT be in additional anymore
        assert!(!additional.contains_key("coverage"));
        assert!(!additional.contains_key("format"));
        assert!(!additional.contains_key("relation"));
        assert!(!additional.contains_key("source"));
        assert!(!additional.contains_key("type"));
        assert!(!additional.contains_key("cover_image"));
    }
}
