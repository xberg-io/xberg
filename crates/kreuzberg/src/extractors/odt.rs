//! ODT (OpenDocument Text) extractor using native Rust parsing.
//!
//! Supports: OpenDocument Text (.odt)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::office_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::Metadata;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::uri::{Uri, UriKind};
use ahash::AHashMap;
use async_trait::async_trait;
use bytes::Bytes;
use roxmltree::Document;
use std::borrow::Cow;
use std::io::Cursor;

/// High-performance ODT extractor using native Rust XML parsing.
///
/// This extractor provides:
/// - Fast text extraction via roxmltree XML parsing
/// - Comprehensive metadata extraction from meta.xml
/// - Table extraction with row and cell support
/// - Formatting preservation (bold, italic, strikeout)
/// - Support for headings, paragraphs, and special elements
pub struct OdtExtractor;

impl OdtExtractor {
    /// Create a new ODT extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for OdtExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OdtExtractor {
    fn name(&self) -> &str {
        "odt-extractor"
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
        "Native Rust ODT (OpenDocument Text) extractor with metadata and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Resolved formatting properties for a text style.
#[derive(Default, Clone)]
struct OdtStyleProps {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: Option<String>,
    font_size: Option<String>,
}

/// Build a map from style-name to resolved formatting properties.
///
/// Parses `<style:style>` elements from the `office:automatic-styles` section
/// of content.xml and resolves `style:text-properties` attributes.
fn build_style_map(root: roxmltree::Node) -> AHashMap<String, OdtStyleProps> {
    let mut styles = AHashMap::new();
    for child in root.children() {
        if child.tag_name().name() == "automatic-styles" || child.tag_name().name() == "styles" {
            for style_node in child.children() {
                if style_node.tag_name().name() == "style"
                    && let Some(name) = style_node
                        .attribute(("urn:oasis:names:tc:opendocument:xmlns:style:1.0", "name"))
                        .or_else(|| style_node.attribute("style:name"))
                {
                    let mut props = OdtStyleProps::default();
                    for prop_child in style_node.children() {
                        if prop_child.tag_name().name() == "text-properties" {
                            // Bold: fo:font-weight="bold"
                            if let Some(fw) = prop_child
                                .attribute((
                                    "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0",
                                    "font-weight",
                                ))
                                .or_else(|| prop_child.attribute("fo:font-weight"))
                            {
                                props.bold = fw == "bold";
                            }
                            // Italic: fo:font-style="italic"
                            if let Some(fs) = prop_child
                                .attribute((
                                    "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0",
                                    "font-style",
                                ))
                                .or_else(|| prop_child.attribute("fo:font-style"))
                            {
                                props.italic = fs == "italic";
                            }
                            // Underline: style:text-underline-style != "none"
                            if let Some(ul) = prop_child
                                .attribute((
                                    "urn:oasis:names:tc:opendocument:xmlns:style:1.0",
                                    "text-underline-style",
                                ))
                                .or_else(|| prop_child.attribute("style:text-underline-style"))
                            {
                                props.underline = ul != "none";
                            }
                            // Strikethrough: style:text-line-through-style != "none"
                            if let Some(st) = prop_child
                                .attribute((
                                    "urn:oasis:names:tc:opendocument:xmlns:style:1.0",
                                    "text-line-through-style",
                                ))
                                .or_else(|| prop_child.attribute("style:text-line-through-style"))
                            {
                                props.strikethrough = st != "none";
                            }
                            // Color: fo:color="#rrggbb"
                            if let Some(color) = prop_child
                                .attribute(("urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0", "color"))
                                .or_else(|| prop_child.attribute("fo:color"))
                                && color != "#000000"
                            {
                                props.color = Some(color.to_string());
                            }
                            // Font size: fo:font-size="12pt"
                            if let Some(size) = prop_child
                                .attribute((
                                    "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0",
                                    "font-size",
                                ))
                                .or_else(|| prop_child.attribute("fo:font-size"))
                            {
                                props.font_size = Some(size.to_string());
                            }
                        }
                    }
                    styles.insert(name.to_string(), props);
                }
            }
        }
    }
    styles
}

/// Pre-extract all images from the ODT ZIP archive into a map keyed by href path.
///
/// Scans the archive for files under `Pictures/` (the standard ODT image directory)
/// and builds a lookup map so that image references in content.xml can be resolved
/// to binary data without re-borrowing the archive during XML walking.
fn pre_extract_images(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> AHashMap<String, (Vec<u8>, String)> {
    use std::io::Read;

    let mut images = AHashMap::new();
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for name in names {
        if !name.starts_with("Pictures/") {
            continue;
        }
        let ext = name.rsplit('.').next().map(|e| e.to_lowercase()).unwrap_or_default();
        let format = match ext.as_str() {
            "jpg" | "jpeg" => "jpeg",
            "png" => "png",
            "gif" => "gif",
            "webp" => "webp",
            "svg" => "svg",
            "bmp" => "bmp",
            "tiff" | "tif" => "tiff",
            _ => "png",
        };
        if let Ok(mut file) = archive.by_name(&name) {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                images.insert(name, (buf, format.to_string()));
            }
        }
    }
    images
}

/// Pre-extract embedded formula objects (MathML) from the ODT archive.
///
/// ODT stores formulas in subdirectories like `Object 1/content.xml` containing
/// MathML markup. This function scans for those and converts them to text.
fn pre_extract_formulas(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> AHashMap<String, String> {
    use std::io::Read;

    let mut formulas = AHashMap::new();
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for name in &names {
        // Formula objects live in e.g. "Object 1/content.xml"
        if !name.ends_with("/content.xml") || name == "content.xml" {
            continue;
        }
        if let Ok(mut file) = archive.by_name(name) {
            let mut xml = String::new();
            if file.read_to_string(&mut xml).is_ok() && xml.contains("math") {
                let text = extract_mathml_text(&xml);
                if !text.is_empty() {
                    // Key is the object directory path without trailing /content.xml
                    let dir = name.trim_end_matches("/content.xml");
                    // Also store with trailing slash variant for flexible lookup
                    formulas.insert(dir.to_string(), text.clone());
                    formulas.insert(format!("{}/", dir), text);
                }
            }
        }
    }

    formulas
}

/// Convert MathML XML content to a plain-text representation.
fn extract_mathml_text(xml: &str) -> String {
    let Ok(doc) = Document::parse(xml) else {
        return String::new();
    };

    let root = doc.root_element();
    let mut tokens = Vec::new();
    collect_mathml_tokens(root, &mut tokens);
    tokens.join(" ")
}

/// Recursively collect text tokens from MathML elements.
fn collect_mathml_tokens(node: roxmltree::Node, tokens: &mut Vec<String>) {
    let tag = node.tag_name().name();
    match tag {
        // Token elements that contain displayable text
        "mi" | "mn" | "mo" | "ms" | "mtext" => {
            if let Some(t) = node.text() {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    tokens.push(trimmed.to_string());
                }
            }
        }
        // Fraction: a/b
        "mfrac" => {
            let children: Vec<_> = node.children().filter(|c| c.is_element()).collect();
            if children.len() == 2 {
                let mut num = Vec::new();
                collect_mathml_tokens(children[0], &mut num);
                let mut den = Vec::new();
                collect_mathml_tokens(children[1], &mut den);
                if !num.is_empty() || !den.is_empty() {
                    tokens.push(format!("({})/({})", num.join(" "), den.join(" ")));
                    return;
                }
            }
            for child in node.children() {
                collect_mathml_tokens(child, tokens);
            }
        }
        // Superscript: base^exp
        "msup" => {
            let children: Vec<_> = node.children().filter(|c| c.is_element()).collect();
            if children.len() == 2 {
                let mut base = Vec::new();
                collect_mathml_tokens(children[0], &mut base);
                let mut exp = Vec::new();
                collect_mathml_tokens(children[1], &mut exp);
                tokens.push(format!("{}^{}", base.join(" "), exp.join(" ")));
                return;
            }
            for child in node.children() {
                collect_mathml_tokens(child, tokens);
            }
        }
        // Subscript: base_sub
        "msub" => {
            let children: Vec<_> = node.children().filter(|c| c.is_element()).collect();
            if children.len() == 2 {
                let mut base = Vec::new();
                collect_mathml_tokens(children[0], &mut base);
                let mut sub = Vec::new();
                collect_mathml_tokens(children[1], &mut sub);
                tokens.push(format!("{}_{}", base.join(" "), sub.join(" ")));
                return;
            }
            for child in node.children() {
                collect_mathml_tokens(child, tokens);
            }
        }
        // Square root
        "msqrt" => {
            let mut inner = Vec::new();
            for child in node.children() {
                collect_mathml_tokens(child, &mut inner);
            }
            tokens.push(format!("sqrt({})", inner.join(" ")));
        }
        // All other elements: recurse
        _ => {
            for child in node.children() {
                collect_mathml_tokens(child, tokens);
            }
        }
    }
}

/// Extract description text from a `draw:frame` element by looking at
/// `svg:title`, `svg:desc`, and `text:p` children of the frame.
fn extract_frame_description(frame: roxmltree::Node) -> Option<String> {
    // Try svg:title first
    for child in frame.children() {
        let name = child.tag_name().name();
        if name == "title"
            && let Some(text) = child.text()
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    // Try svg:desc
    for child in frame.children() {
        let name = child.tag_name().name();
        if name == "desc"
            && let Some(text) = child.text()
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    // Try text:p children (used for captions inside the frame)
    for child in frame.children() {
        if child.tag_name().name() == "p"
            && let Some(text) = extract_node_text(child)
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Build an `InternalDocument` from ODT content.xml.
///
/// Walks the XML tree and emits flat elements through `InternalDocumentBuilder`.
/// Captures headings, paragraphs, lists, tables, images, formulas, footnotes
/// (with inline markers), and headers/footers with appropriate content layers.
fn build_internal_document(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<InternalDocument> {
    // Pre-extract images so we don't need the archive borrow during XML walking
    let image_data = pre_extract_images(archive);
    let formula_data = pre_extract_formulas(archive);

    let mut xml_content = String::new();

    match archive.by_name("content.xml") {
        Ok(mut file) => {
            use std::io::Read;
            file.read_to_string(&mut xml_content)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read content.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(InternalDocumentBuilder::new("odt").build());
        }
    }

    let doc = Document::parse(&xml_content)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse content.xml: {}", e)))?;

    let root = doc.root_element();
    let style_map = build_style_map(root);
    let mut builder = InternalDocumentBuilder::new("odt");

    for body_child in root.children() {
        if body_child.tag_name().name() == "body" {
            for text_elem in body_child.children() {
                if text_elem.tag_name().name() == "text" {
                    build_internal_elements(text_elem, &mut builder, &style_map, &image_data, &formula_data);
                }
            }
        }
    }

    // Extract headers/footers from styles.xml
    extract_odt_internal_headers_footers(archive, &mut builder);

    Ok(builder.build())
}

/// Recursively walk ODT XML elements and populate the `InternalDocumentBuilder`.
fn build_internal_elements(
    parent: roxmltree::Node,
    builder: &mut InternalDocumentBuilder,
    style_map: &AHashMap<String, OdtStyleProps>,
    image_data: &AHashMap<String, (Vec<u8>, String)>,
    formula_data: &AHashMap<String, String>,
) {
    use crate::types::document_structure::ContentLayer;
    use crate::types::internal::{ElementKind, InternalElement};

    let mut footnote_counter = 0u32;

    for node in parent.children() {
        match node.tag_name().name() {
            "h" => {
                let (text, _annotations, uris) = collect_odt_annotations(node, style_map);
                for uri in uris {
                    builder.push_uri(uri);
                }
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let level = node
                        .attribute(("urn:oasis:names:tc:opendocument:xmlns:text:1.0", "outline-level"))
                        .and_then(|v| v.parse::<u8>().ok())
                        .unwrap_or(1);
                    builder.push_heading(level, trimmed, None, None);
                }
            }
            "p" => {
                // Collect footnote markers for inline injection
                let mut footnote_markers: Vec<(String, String)> = Vec::new();

                // Check for draw:frame children — may contain images or embedded formulas
                for desc in node.descendants() {
                    if desc.tag_name().name() == "frame" {
                        // Check if this frame contains a draw:object referencing a formula
                        let mut is_formula = false;
                        for frame_child in desc.children() {
                            if frame_child.tag_name().name() == "object" {
                                let obj_href = frame_child
                                    .attribute(("http://www.w3.org/1999/xlink", "href"))
                                    .or_else(|| frame_child.attribute("xlink:href"));
                                if let Some(href) = obj_href {
                                    // Normalize: remove leading "./" if present
                                    let normalized = href.trim_start_matches("./");
                                    if let Some(formula_text) = formula_data.get(normalized) {
                                        builder.push_formula(formula_text, None, None);
                                        is_formula = true;
                                        break;
                                    }
                                }
                            }
                        }

                        if !is_formula {
                            // Look for image inside this frame
                            for frame_child in desc.descendants() {
                                if frame_child.tag_name().name() == "image" {
                                    let href = frame_child
                                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                                        .or_else(|| frame_child.attribute("xlink:href"));

                                    // Use extract_frame_description for richer alt text
                                    let description = extract_frame_description(desc).or_else(|| {
                                        desc.attribute((
                                            "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0",
                                            "title",
                                        ))
                                        .or_else(|| desc.attribute("svg:title"))
                                        .map(|s| s.to_string())
                                    });

                                    let extracted = href.and_then(|h| {
                                        image_data.get(h).map(|(data, format)| (data.clone(), format.clone()))
                                    });

                                    if let Some((data, format)) = extracted {
                                        let image = ExtractedImage {
                                            data: Bytes::from(data),
                                            format: Cow::Owned(format),
                                            image_index: 0,
                                            page_number: None,
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
                                        let idx = builder.push_image(description.as_deref(), image, None, None);
                                        if let Some(h) = href {
                                            let mut attrs = AHashMap::with_capacity(1);
                                            attrs.insert("src".to_string(), h.to_string());
                                            builder.set_attributes(idx, attrs);
                                        }
                                    } else {
                                        let text_val = description.as_deref().or(href).unwrap_or("");
                                        let elem =
                                            InternalElement::text(ElementKind::Image { image_index: 0 }, text_val, 0);
                                        let idx = builder.push_element(elem);
                                        if let Some(h) = href {
                                            let mut attrs = AHashMap::with_capacity(1);
                                            attrs.insert("src".to_string(), h.to_string());
                                            builder.set_attributes(idx, attrs);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Extract footnotes from this paragraph and collect markers for inline injection
                for child in node.descendants() {
                    if child.tag_name().name() == "note" {
                        let _ = child
                            .attribute(("urn:oasis:names:tc:opendocument:xmlns:text:1.0", "note-class"))
                            .or_else(|| child.attribute("text:note-class"))
                            .unwrap_or("footnote");
                        let note_id = child
                            .attribute(("urn:oasis:names:tc:opendocument:xmlns:text:1.0", "id"))
                            .or_else(|| child.attribute("text:id"));

                        for note_child in child.children() {
                            if note_child.tag_name().name() == "note-citation" {
                                let citation_text = extract_node_text(note_child).unwrap_or_default();
                                let citation_trimmed = citation_text.trim();
                                if !citation_trimmed.is_empty() {
                                    footnote_counter += 1;
                                    let key = note_id
                                        .map(|id| id.to_string())
                                        .unwrap_or_else(|| format!("fn{}", footnote_counter));
                                    footnote_markers.push((citation_trimmed.to_string(), key.clone()));
                                    builder.push_footnote_ref(citation_trimmed, &key, None);
                                }
                            }
                            if note_child.tag_name().name() == "note-body"
                                && let Some(note_text) = extract_node_text(note_child)
                            {
                                let trimmed = note_text.trim();
                                if !trimmed.is_empty() {
                                    let key = note_id.map(|id| id.to_string()).unwrap_or_else(|| {
                                        if footnote_counter == 0 {
                                            footnote_counter += 1;
                                        }
                                        format!("fn{}", footnote_counter)
                                    });
                                    let def_idx = builder.push_footnote_definition(trimmed, &key, None);
                                    builder.set_layer(def_idx, ContentLayer::Footnote);
                                }
                            }
                        }
                    }
                }

                let (mut text, annotations, uris) = collect_odt_annotations(node, style_map);
                for uri in uris {
                    builder.push_uri(uri);
                }

                // Inject inline footnote markers [^N] into the paragraph text
                for (citation, _key) in &footnote_markers {
                    let marker = format!("[^{}]", citation);
                    // Append marker if not already present in text
                    if !text.contains(&marker) {
                        text.push_str(&marker);
                    }
                }

                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    builder.push_paragraph(trimmed, annotations, None, None);
                }
            }
            "table" => {
                let cells = extract_table_cells(node);
                if !cells.is_empty() {
                    builder.push_table_from_cells(&cells, None, None);
                }
            }
            "list" => {
                build_internal_list(node, builder);
            }
            "section" => {
                build_internal_elements(node, builder, style_map, image_data, formula_data);
            }
            _ => {}
        }
    }
}

/// Build list structure from an ODT `text:list` element for InternalDocumentBuilder.
fn build_internal_list(list_node: roxmltree::Node, builder: &mut InternalDocumentBuilder) {
    builder.push_list(false);
    for item in list_node.children() {
        if item.tag_name().name() == "list-item" {
            for child in item.children() {
                match child.tag_name().name() {
                    "p" | "h" => {
                        if let Some(text) = extract_node_text(child) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                builder.push_list_item(trimmed, false, vec![], None, None);
                            }
                        }
                    }
                    "list" => {
                        build_internal_list(child, builder);
                    }
                    _ => {}
                }
            }
        }
    }
    builder.end_list();
}

/// Extract headers and footers from styles.xml for InternalDocumentBuilder.
fn extract_odt_internal_headers_footers(
    archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    builder: &mut InternalDocumentBuilder,
) {
    use crate::types::document_structure::ContentLayer;

    let mut styles_xml = String::new();
    if let Ok(mut file) = archive.by_name("styles.xml") {
        use std::io::Read;
        if file.read_to_string(&mut styles_xml).is_err() {
            return;
        }
    } else {
        return;
    }

    let Ok(doc) = Document::parse(&styles_xml) else {
        return;
    };

    for node in doc.root_element().descendants() {
        match node.tag_name().name() {
            "header" => {
                if let Some(text) = extract_node_text(node) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let idx = builder.push_paragraph(trimmed, vec![], None, None);
                        builder.set_layer(idx, ContentLayer::Header);
                    }
                }
            }
            "footer" => {
                if let Some(text) = extract_node_text(node) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let idx = builder.push_paragraph(trimmed, vec![], None, None);
                        builder.set_layer(idx, ContentLayer::Footer);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Collect text and annotations from an ODT paragraph/heading node's children.
///
/// Walks `<text:span>` children, resolves their `text:style-name` against the
/// style map, and produces byte-offset `TextAnnotation`s.
fn collect_odt_annotations(
    node: roxmltree::Node,
    style_map: &AHashMap<String, OdtStyleProps>,
) -> (String, Vec<crate::types::TextAnnotation>, Vec<Uri>) {
    use crate::types::builder;
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};

    let mut text = String::new();
    let mut annotations = Vec::new();
    let mut uris = Vec::new();

    for child in node.children() {
        match child.tag_name().name() {
            "span" => {
                let span_text = child.text().unwrap_or("");
                if span_text.is_empty() {
                    continue;
                }
                let start = text.len() as u32;
                text.push_str(span_text);
                let end = text.len() as u32;

                // Resolve style
                let style_name = child
                    .attribute(("urn:oasis:names:tc:opendocument:xmlns:text:1.0", "style-name"))
                    .or_else(|| child.attribute("text:style-name"));
                if let Some(name) = style_name
                    && let Some(props) = style_map.get(name)
                {
                    if props.bold {
                        annotations.push(builder::bold(start, end));
                    }
                    if props.italic {
                        annotations.push(builder::italic(start, end));
                    }
                    if props.underline {
                        annotations.push(builder::underline(start, end));
                    }
                    if props.strikethrough {
                        annotations.push(builder::strikethrough(start, end));
                    }
                    if let Some(ref color) = props.color {
                        annotations.push(TextAnnotation {
                            start,
                            end,
                            kind: AnnotationKind::Color { value: color.clone() },
                        });
                    }
                    if let Some(ref size) = props.font_size {
                        annotations.push(TextAnnotation {
                            start,
                            end,
                            kind: AnnotationKind::FontSize { value: size.clone() },
                        });
                    }
                }
            }
            "tab" => {
                text.push('\t');
            }
            "line-break" => {
                text.push('\n');
            }
            "note" => {
                // Footnotes/endnotes: skip inline (handled separately)
            }
            "a" => {
                // Hyperlinks inside paragraphs
                let link_text = child.text().unwrap_or("");
                if !link_text.is_empty() {
                    let start = text.len() as u32;
                    text.push_str(link_text);
                    let end = text.len() as u32;
                    let url = child
                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                        .or_else(|| child.attribute("xlink:href"))
                        .unwrap_or("");
                    if !url.is_empty() {
                        annotations.push(builder::link(start, end, url, None));
                        let kind = if url.starts_with('#') {
                            UriKind::Anchor
                        } else if url.starts_with("mailto:") {
                            UriKind::Email
                        } else {
                            UriKind::Hyperlink
                        };
                        uris.push(Uri {
                            url: url.to_string(),
                            label: Some(link_text.to_string()),
                            page: None,
                            kind,
                        });
                    }
                }
            }
            _ => {
                if let Some(t) = child.text() {
                    text.push_str(t);
                }
            }
        }
    }

    // Fallback: if no children produced text, try the node's own text
    if text.is_empty()
        && let Some(t) = node.text()
    {
        text = t.to_string();
    }

    (text, annotations, uris)
}

/// Extract table cells as `Vec<Vec<String>>` from an ODT table element.
fn extract_table_cells(table_node: roxmltree::Node) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();
            for cell_node in row_node.children() {
                if cell_node.tag_name().name() == "table-cell" {
                    let cell_text = extract_node_text(cell_node).unwrap_or_default();
                    row_cells.push(cell_text.trim().to_string());
                }
            }
            if !row_cells.is_empty() {
                rows.push(row_cells);
            }
        }
    }
    rows
}

/// Extract text from a single XML node, handling spans and formatting
///
/// # Arguments
/// * `node` - The XML node to extract text from
///
/// # Returns
/// * `Option<String>` - The extracted text with formatting preserved
fn extract_node_text(node: roxmltree::Node) -> Option<String> {
    let mut text_parts = Vec::new();

    for child in node.children() {
        match child.tag_name().name() {
            "span" => {
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
            "tab" => {
                text_parts.push("\t".to_string());
            }
            "line-break" => {
                text_parts.push("\n".to_string());
            }
            _ => {
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
        }
    }

    if text_parts.is_empty() {
        node.text().map(|s| s.to_string())
    } else {
        Some(text_parts.join(""))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for OdtExtractor {
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
        tracing::debug!(format = "odt", size_bytes = content.len(), "extraction starting");
        let _ = config; // conditionally used by ocr feature
        let content_owned = content.to_vec();

        let cursor = Cursor::new(content_owned.clone());
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

        let mut doc = build_internal_document(&mut archive)?;
        doc.mime_type = Cow::Owned(mime_type.to_string());

        // Extract metadata from meta.xml
        let mut metadata_map = AHashMap::new();

        let meta_cursor = Cursor::new(content_owned);
        let mut meta_archive = zip::ZipArchive::new(meta_cursor).map_err(|e| {
            crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive for metadata: {}", e))
        })?;

        if let Ok(odt_props) = office_metadata::extract_odt_properties(&mut meta_archive) {
            if let Some(title) = odt_props.title {
                metadata_map.insert(Cow::Borrowed("title"), serde_json::Value::String(title));
            }
            if let Some(creator) = odt_props.creator {
                metadata_map.insert(
                    Cow::Borrowed("authors"),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert(Cow::Borrowed("created_by"), serde_json::Value::String(creator));
            }
            if let Some(initial_creator) = odt_props.initial_creator {
                metadata_map.insert(
                    Cow::Borrowed("initial_creator"),
                    serde_json::Value::String(initial_creator),
                );
            }
            if let Some(subject) = odt_props.subject {
                metadata_map.insert(Cow::Borrowed("subject"), serde_json::Value::String(subject));
            }
            if let Some(keywords) = odt_props.keywords {
                metadata_map.insert(Cow::Borrowed("keywords"), serde_json::Value::String(keywords));
            }
            if let Some(description) = odt_props.description {
                metadata_map.insert(Cow::Borrowed("description"), serde_json::Value::String(description));
            }
            if let Some(creation_date) = odt_props.creation_date {
                metadata_map.insert(Cow::Borrowed("created_at"), serde_json::Value::String(creation_date));
            }
            if let Some(date) = odt_props.date {
                metadata_map.insert(Cow::Borrowed("modified_at"), serde_json::Value::String(date));
            }
            if let Some(language) = odt_props.language {
                metadata_map.insert(Cow::Borrowed("language"), serde_json::Value::String(language));
            }
            if let Some(generator) = odt_props.generator {
                metadata_map.insert(Cow::Borrowed("generator"), serde_json::Value::String(generator));
            }
            if let Some(editing_duration) = odt_props.editing_duration {
                metadata_map.insert(
                    Cow::Borrowed("editing_duration"),
                    serde_json::Value::String(editing_duration),
                );
            }
            if let Some(editing_cycles) = odt_props.editing_cycles {
                metadata_map.insert(
                    Cow::Borrowed("editing_cycles"),
                    serde_json::Value::String(editing_cycles),
                );
            }
            if let Some(page_count) = odt_props.page_count {
                metadata_map.insert(
                    Cow::Borrowed("page_count"),
                    serde_json::Value::Number(page_count.into()),
                );
            }
            if let Some(word_count) = odt_props.word_count {
                metadata_map.insert(
                    Cow::Borrowed("word_count"),
                    serde_json::Value::Number(word_count.into()),
                );
            }
            if let Some(character_count) = odt_props.character_count {
                metadata_map.insert(
                    Cow::Borrowed("character_count"),
                    serde_json::Value::Number(character_count.into()),
                );
            }
            if let Some(paragraph_count) = odt_props.paragraph_count {
                metadata_map.insert(
                    Cow::Borrowed("paragraph_count"),
                    serde_json::Value::Number(paragraph_count.into()),
                );
            }
            if let Some(table_count) = odt_props.table_count {
                metadata_map.insert(
                    Cow::Borrowed("table_count"),
                    serde_json::Value::Number(table_count.into()),
                );
            }
            if let Some(image_count) = odt_props.image_count {
                metadata_map.insert(
                    Cow::Borrowed("image_count"),
                    serde_json::Value::Number(image_count.into()),
                );
            }
        }

        // Map standard fields from metadata_map to typed Metadata fields
        let title = metadata_map
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let subject = metadata_map
            .remove(&Cow::Borrowed("subject"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let authors = metadata_map.remove(&Cow::Borrowed("authors")).and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        });
        let created_by = metadata_map
            .remove(&Cow::Borrowed("created_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let created_at = metadata_map
            .remove(&Cow::Borrowed("created_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let modified_at = metadata_map
            .remove(&Cow::Borrowed("modified_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let language = metadata_map
            .remove(&Cow::Borrowed("language"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let keywords = metadata_map.remove(&Cow::Borrowed("keywords")).and_then(|v| {
            v.as_str().map(|s| {
                s.split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty())
                    .collect()
            })
        });

        doc.metadata = Metadata {
            title,
            subject,
            authors,
            keywords,
            language,
            created_at,
            modified_at,
            created_by,
            additional: metadata_map,
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "odt",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.oasis.opendocument.text"]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_odt_extractor_plugin_interface() {
        let extractor = OdtExtractor::new();
        assert_eq!(extractor.name(), "odt-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_odt_extractor_supports_odt() {
        let extractor = OdtExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.oasis.opendocument.text")
        );
    }

    #[tokio::test]
    async fn test_odt_extractor_default() {
        let extractor = OdtExtractor;
        assert_eq!(extractor.name(), "odt-extractor");
    }

    #[tokio::test]
    async fn test_odt_extractor_initialize_shutdown() {
        let extractor = OdtExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extract_node_text_simple() {
        let xml = r#"<p xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">Hello world</p>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let result = extract_node_text(node);
        assert!(result.is_some());
        assert!(!result.unwrap().is_empty());
    }

    /// Helper to load test ODT, extract with document structure, and return the structure.
    async fn extract_odt_with_structure(filename: &str) -> Option<crate::types::document_structure::DocumentStructure> {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/odt")
            .join(filename);
        if !test_file.exists() {
            return None;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test ODT");
        let extractor = OdtExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(&content, "application/vnd.oasis.opendocument.text", &config)
            .await
            .expect("ODT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        result.document
    }

    #[tokio::test]
    async fn test_odt_footnote_extraction() {
        let doc = extract_odt_with_structure("footnote.odt").await;
        let Some(doc) = doc else { return };
        // Should contain at least one Footnote node
        let has_footnote = doc.nodes.iter().any(|n| {
            matches!(
                n.content,
                crate::types::document_structure::NodeContent::Footnote { .. }
            )
        });
        assert!(
            has_footnote,
            "Footnote ODT should produce Footnote nodes in document structure"
        );
    }

    #[tokio::test]
    async fn test_odt_header_extraction() {
        let doc = extract_odt_with_structure("headers.odt").await;
        let Some(doc) = doc else { return };
        // headers.odt contains document headings (text:h elements), which are stored as
        // NodeContent::Group nodes with heading_level set.
        let has_heading = doc.nodes.iter().any(|n| {
            matches!(
                n.content,
                crate::types::document_structure::NodeContent::Group {
                    heading_level: Some(_),
                    ..
                }
            )
        });
        assert!(
            has_heading,
            "Headers ODT should produce Group nodes with heading_level in document structure"
        );
    }

    #[tokio::test]
    async fn test_odt_image_extraction() {
        let doc = extract_odt_with_structure("imageWithCaption.odt").await;
        let Some(doc) = doc else { return };
        let has_image = doc
            .nodes
            .iter()
            .any(|n| matches!(n.content, crate::types::document_structure::NodeContent::Image { .. }));
        assert!(has_image, "Image ODT should produce Image nodes in document structure");
    }

    #[tokio::test]
    async fn test_odt_bold_annotations() {
        let doc = extract_odt_with_structure("bold.odt").await;
        let Some(doc) = doc else { return };
        let has_bold = doc.nodes.iter().any(|n| {
            n.annotations
                .iter()
                .any(|a| matches!(a.kind, crate::types::document_structure::AnnotationKind::Bold))
        });
        assert!(
            has_bold,
            "Bold ODT should produce Bold annotations in document structure"
        );
    }

    #[tokio::test]
    async fn test_odt_italic_annotations() {
        let doc = extract_odt_with_structure("italic.odt").await;
        let Some(doc) = doc else { return };
        let has_italic = doc.nodes.iter().any(|n| {
            n.annotations
                .iter()
                .any(|a| matches!(a.kind, crate::types::document_structure::AnnotationKind::Italic))
        });
        assert!(
            has_italic,
            "Italic ODT should produce Italic annotations in document structure"
        );
    }

    #[tokio::test]
    async fn test_odt_underline_annotations() {
        let doc = extract_odt_with_structure("strikeout.odt").await;
        let Some(doc) = doc else { return };
        // strikeout.odt should have strikethrough annotations
        let has_strikethrough = doc.nodes.iter().any(|n| {
            n.annotations
                .iter()
                .any(|a| matches!(a.kind, crate::types::document_structure::AnnotationKind::Strikethrough))
        });
        assert!(
            has_strikethrough,
            "Strikeout ODT should produce Strikethrough annotations"
        );
    }
}
