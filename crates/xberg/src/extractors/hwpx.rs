//! Hangul Word Processor XML (.hwpx) extractor.
//!
//! Extracts text, headings, tables, and images from HWPX documents using the `unhwp` crate.

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek};

use ahash::AHashMap;
use async_trait::async_trait;
use bytes::Bytes;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::ZipBombValidator;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::builder::trim_text_and_annotations;
use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;

/// `ProcessingWarning::source` for every warning this extractor emits (#171).
const HWPX_WARNING_SOURCE: &str = "hwpx";

/// Maximum bytes read from a single HWPX zip member's section XML.
///
/// `ZipBombValidator::validate` (called before any member is read) only checks
/// the *declared* compressed/uncompressed sizes and ratio from the zip central
/// directory -- it never decompresses. A crafted entry can under-declare its
/// size while its deflate stream expands far past that declaration, so the
/// entry's declared size cannot be trusted as an upper bound on what reading it
/// actually produces. Bounding the read itself with `Read::take` (the same
/// pattern as `odt::MAX_ODT_MEMBER_SIZE`) is what actually caps memory.
const MAX_HWPX_MEMBER_SIZE: u64 = 100 * 1024 * 1024;

/// Extractor for Hangul Word Processor XML (.hwpx) files.
///
/// Supports HWPX (Open HWPML), the ZIP-based XML successor to the binary HWP 5.0 format.
#[cfg_attr(alef, alef(skip))]
pub struct HwpxExtractor;

impl HwpxExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for HwpxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HwpxExtractor {
    fn name(&self) -> &str {
        "hwpx-extractor"
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
        "Hangul Word Processor XML (.hwpx) text extraction"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

fn mime_to_format(mime: &str) -> Cow<'static, str> {
    match mime {
        "image/png" => Cow::Borrowed("png"),
        "image/jpeg" | "image/jpg" => Cow::Borrowed("jpeg"),
        "image/gif" => Cow::Borrowed("gif"),
        "image/bmp" => Cow::Borrowed("bmp"),
        "image/webp" => Cow::Borrowed("webp"),
        // HWPX embeds vector/legacy Windows metafiles (WMF/EMF) and SVG far more often
        // than other office formats — `unhwp`'s resource guesser (hwpx/mod.rs
        // `guess_mime_type`) returns these three MIME strings, and previously every one
        // of them fell through to the generic "bin" bucket, discarding the real format
        // (#120).
        "image/svg+xml" => Cow::Borrowed("svg"),
        "image/x-wmf" => Cow::Borrowed("wmf"),
        "image/x-emf" => Cow::Borrowed("emf"),
        _ => Cow::Borrowed("bin"),
    }
}

/// Collect the LaTeX of every equation, one `Vec` per section, in document order.
///
/// `unhwp` reads an equation only when it sits in a paragraph-level `<hp:ctrl>`;
/// its run reader drops `<hp:equation>`. Hangul writes the equation into the run,
/// so a real document loses every formula. Reading the section XML through the
/// crate's own container keeps the archive, the section order and the script
/// conversion with `unhwp`, and adds only the search for one element.
/// Collect the LaTeX of every equation, keyed by section index and by the
/// ordinal of the paragraph that holds it.
///
/// `unhwp` reads an equation only when it sits in a paragraph-level `<hp:ctrl>`;
/// its run reader drops `<hp:equation>`. Hangul writes the equation into the run,
/// so a real document loses every formula. This walks the section parts of the
/// archive the caller already opened, so the file is neither copied nor
/// decompressed a second time, and `unhwp` keeps the script conversion.
///
/// A paragraph inside a table is not counted: `unhwp` keeps those in the cell
/// rather than in the section's block list, so counting them would shift every
/// later ordinal.
fn collect_section_formulas<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> AHashMap<usize, Vec<(usize, String)>> {
    use crate::utils::xml_utils::EntityReader;
    use quick_xml::events::Event;

    let mut per_section: AHashMap<usize, Vec<(usize, String)>> = AHashMap::new();
    for (section_index, name) in section_parts(archive) {
        let Some(xml) = read_section_xml(archive, &name) else {
            continue;
        };

        let mut formulas: Vec<(usize, String)> = Vec::new();
        // `EntityReader` resolves `&amp;` and friends into the text it returns. A
        // bare reader emits them as separate events, and an equation script uses
        // `&` as its matrix column separator.
        let mut reader = EntityReader::from_str(&xml);
        let mut paragraph_ordinal = 0usize;
        let mut paragraph_depth = 0usize;
        let mut table_depth = 0usize;
        let mut in_equation = false;
        let mut in_script = false;
        let mut script = String::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                    "tbl" => table_depth += 1,
                    "p" if table_depth == 0 => paragraph_depth += 1,
                    "equation" | "eqEdit" => in_equation = true,
                    "script" if in_equation => in_script = true,
                    _ => {}
                },
                Ok(Event::Text(t)) if in_script => {
                    script.push_str(&std::borrow::Cow::Borrowed(t.as_ref()));
                }
                Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                    "tbl" => table_depth = table_depth.saturating_sub(1),
                    "p" if table_depth == 0 => {
                        paragraph_depth = paragraph_depth.saturating_sub(1);
                        if paragraph_depth == 0 {
                            paragraph_ordinal += 1;
                        }
                    }
                    "script" => in_script = false,
                    "equation" | "eqEdit" => {
                        let latex = unhwp::equation::to_latex(std::mem::take(&mut script).trim());
                        if !latex.trim().is_empty() {
                            formulas.push((paragraph_ordinal, latex.trim().to_string()));
                        }
                        in_equation = false;
                    }
                    _ => {}
                },
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
        if !formulas.is_empty() {
            per_section.insert(section_index, formulas);
        }
    }
    per_section
}

/// The section parts of the archive in order, each with its position.
///
/// `unhwp` numbers a section by its position in the list it reads, not by the
/// digits in its name, so the key is the position here too. A package whose
/// parts are `section0` and `section2` gives positions 0 and 1 on both sides.
fn section_parts<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Vec<(usize, String)> {
    let mut parts: Vec<(usize, String)> = archive
        .file_names()
        .filter_map(|name| {
            let name = name.ok()?;
            section_index_of(&name).map(|digits| (digits, name.into_owned()))
        })
        .collect();
    parts.sort();
    parts
        .into_iter()
        .enumerate()
        .map(|(position, (_, name))| (position, name))
        .collect()
}

fn read_section_xml<R: Read + Seek>(archive: &mut zip::ZipArchive<R>, name: &str) -> Option<String> {
    let mut xml = String::new();
    archive
        .by_name(name)
        .ok()
        .and_then(|part| part.take(MAX_HWPX_MEMBER_SIZE).read_to_string(&mut xml).ok())?;
    Some(xml)
}

/// The paragraphs of every `<hp:header>` and `<hp:footer>`, one entry each, in document order.
///
/// `unhwp` reads a header or footer only into `Section::header`/`footer`, which its
/// HWPX reader never fills: the `<hp:ctrl>` that holds them is skipped. The text is
/// read from the section XML here instead.
fn collect_section_furniture<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Vec<(ContentLayer, Vec<String>)> {
    use crate::utils::xml_utils::EntityReader;
    use quick_xml::events::Event;

    let mut found = Vec::new();
    for (_, name) in section_parts(archive) {
        let Some(xml) = read_section_xml(archive, &name) else {
            continue;
        };
        let mut reader = EntityReader::from_str(&xml);
        let mut layer: Option<ContentLayer> = None;
        let mut in_text = false;
        // One buffer per open paragraph, so a paragraph nested in a table cell does not run
        // into the text around it.
        let mut open_paragraphs: Vec<String> = Vec::new();
        let mut paragraphs: Vec<String> = Vec::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                    "header" => layer = Some(ContentLayer::Header),
                    "footer" => layer = Some(ContentLayer::Footer),
                    "p" if layer.is_some() => {
                        // The text before a nested paragraph is complete; what follows it is new.
                        if let Some(outer) = open_paragraphs.last_mut() {
                            if !outer.trim().is_empty() {
                                paragraphs.push(outer.trim().to_string());
                            }
                            outer.clear();
                        }
                        open_paragraphs.push(String::new());
                    }
                    "t" if layer.is_some() => in_text = true,
                    "tab" if in_text => open_paragraphs.last_mut().into_iter().for_each(|p| p.push(' ')),
                    _ => {}
                },
                Ok(Event::Empty(e)) if in_text && local_name(e.name().as_ref()) == "tab" => {
                    open_paragraphs.last_mut().into_iter().for_each(|p| p.push(' '));
                }
                Ok(Event::Text(t)) if in_text => {
                    if let Some(paragraph) = open_paragraphs.last_mut() {
                        paragraph.push_str(&std::borrow::Cow::Borrowed(t.as_ref()));
                    }
                }
                Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                    "t" => in_text = false,
                    "p" if layer.is_some() => {
                        let text = open_paragraphs.pop().unwrap_or_default();
                        if !text.trim().is_empty() {
                            paragraphs.push(text.trim().to_string());
                        }
                    }
                    "header" | "footer" => {
                        if let Some(layer) = layer
                            && !paragraphs.is_empty()
                        {
                            found.push((layer, std::mem::take(&mut paragraphs)));
                        }
                        paragraphs.clear();
                        layer = None;
                        open_paragraphs.clear();
                    }
                    _ => {}
                },
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
    }
    found
}

/// Return the number in a section part's name, so `Contents/section3.xml`
/// gives `3`.
///
/// The number orders the parts. It is not the key: `unhwp` numbers a section by
/// its position in the list it reads, and a package may skip a number.
fn section_index_of(name: &str) -> Option<usize> {
    let file = name.rsplit('/').next()?;
    let digits = file.strip_prefix("section")?.strip_suffix(".xml")?;
    digits.parse().ok()
}

/// Return the local part of a possibly prefixed XML qualified name.
fn local_name(qname: &str) -> &str {
    match qname.rsplit_once(':') {
        Some((_, local)) => local,
        None => qname,
    }
}

#[cfg(test)]
fn build_hwpx_internal_document(
    doc: unhwp::model::Document,
    mime_type: &str,
    section_formulas: &AHashMap<usize, Vec<(usize, String)>>,
) -> InternalDocument {
    build_hwpx_internal_document_with_furniture(doc, mime_type, section_formulas, &[], None)
}

/// Whether the content filter lets the header or footer layer through. Without a
/// filter the layers are kept, as they are not rendered then.
fn furniture_layer_kept(layer: ContentLayer, filter: Option<&crate::core::config::ContentFilterConfig>) -> bool {
    filter.is_none_or(|filter| match layer {
        ContentLayer::Header => filter.include_headers,
        ContentLayer::Footer => filter.include_footers,
        _ => true,
    })
}

/// Build the document. `scanned_furniture` adds header and footer paragraphs read from the
/// section XML to those in the parsed model. Each distinct header or footer, as a whole, is
/// pushed once, so one repeated per section appears once while a repeated paragraph inside
/// it stays, and a layer the filter drops is not built at
/// all, which keeps the footnote definitions inside it out of the document.
fn build_hwpx_internal_document_with_furniture(
    doc: unhwp::model::Document,
    mime_type: &str,
    section_formulas: &AHashMap<usize, Vec<(usize, String)>>,
    scanned_furniture: &[(ContentLayer, Vec<String>)],
    filter: Option<&crate::core::config::ContentFilterConfig>,
) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("hwpx");
    builder.set_mime_type(Cow::Owned(mime_type.to_string()));

    let mut metadata = crate::types::metadata::Metadata::default();
    if let Some(title) = &doc.metadata.title {
        metadata.title = Some(title.clone());
    }
    if let Some(author) = &doc.metadata.author {
        metadata.authors = Some(vec![author.clone()]);
    }
    if let Some(subject) = &doc.metadata.subject {
        metadata.subject = Some(subject.clone());
    }
    if !doc.metadata.keywords.is_empty() {
        metadata.keywords = Some(doc.metadata.keywords.clone());
    }
    if let Some(created) = &doc.metadata.created {
        metadata.created_at = Some(created.clone());
    }
    if let Some(modified) = &doc.metadata.modified {
        metadata.modified_at = Some(modified.clone());
    }
    if let Some(creator_app) = &doc.metadata.creator_app {
        metadata.additional.insert(
            Cow::Borrowed("creator_app"),
            serde_json::Value::String(creator_app.clone()),
        );
    }
    if let Some(version) = &doc.metadata.format_version {
        metadata.document_version = Some(version.clone());
    }
    if !metadata.is_empty() {
        builder.set_metadata(metadata);
    }

    let mut image_index: usize = 0;
    let mut footnote_counter: u32 = 0;
    let image_resources = ImageResourceLookup::new(&doc.resources);
    let mut furniture: Vec<(&[unhwp::model::Paragraph], ContentLayer)> = Vec::new();

    for section in &doc.sections {
        // Key on the section's own index. `unhwp` drops a section it cannot
        // read, so a position in the list does not identify the part.
        let scanned = section_formulas.get(&section.index);
        let mut next_formula = 0usize;
        let mut paragraph_ordinal = 0usize;
        // Section headers/footers (`unhwp::model::Section::header`/`footer`) previously
        // went unread entirely — only `section.content` was visited (#96). They are
        // pushed after the last section so the text follows the body. ~keep
        if let Some(header_paragraphs) = &section.header
            && furniture_layer_kept(ContentLayer::Header, filter)
        {
            furniture.push((header_paragraphs, ContentLayer::Header));
        }
        if let Some(footer_paragraphs) = &section.footer
            && furniture_layer_kept(ContentLayer::Footer, filter)
        {
            furniture.push((footer_paragraphs, ContentLayer::Footer));
        }

        for block in &section.content {
            match block {
                unhwp::model::Block::Paragraph(p) => {
                    // `Paragraph::has_text_content()` does not count `InlineContent::Equation`
                    // (see `unhwp::model::paragraph`), so an equation-only paragraph (no
                    // surrounding prose) would otherwise be dropped here before its LaTeX
                    // ever reached `build_paragraph_content` (#98). ~keep
                    let has_equation = p
                        .content
                        .iter()
                        .any(|c| matches!(c, unhwp::model::InlineContent::Equation(_)));
                    let (text, annotations) = build_paragraph_content(&mut builder, p, &mut footnote_counter);
                    let (trimmed, adjusted) = trim_text_and_annotations(&text, annotations);
                    if p.style.is_heading() && (p.has_text_content() || has_equation) {
                        if !trimmed.is_empty() {
                            let idx = builder.push_heading(p.style.heading_level, trimmed, None, None);
                            if !adjusted.is_empty() {
                                builder.set_annotations(idx, adjusted);
                            }
                        }
                    } else if (p.has_text_content() || has_equation) && !trimmed.is_empty() {
                        builder.push_paragraph(trimmed, adjusted, None, None);
                    }

                    for inline in &p.content {
                        if let unhwp::model::InlineContent::Image(img_ref) = inline {
                            let resource = match image_resources.get(&img_ref.id) {
                                ImageResourceMatch::Found(resource) => resource,
                                ImageResourceMatch::Missing => {
                                    // `img_ref.id` names a resource that `doc.resources` never
                                    // received a binary payload for (a missing/unpacked media
                                    // part). A well-formed HWPX package always has a resource
                                    // entry for every image reference it emits, so this only
                                    // fires on a genuinely incomplete document -- and unlike the
                                    // resolved case above, nothing is pushed for it at all: the
                                    // image is silently absent from the output (#171). ~keep
                                    builder.add_warning(crate::core::diagnostics::warning(
                                        HWPX_WARNING_SOURCE,
                                        format!(
                                            "Image reference '{}' has no corresponding entry in the document's \
                                             resources; the image could not be extracted",
                                            img_ref.id
                                        ),
                                    ));
                                    continue;
                                }
                                ImageResourceMatch::Ambiguous => {
                                    builder.add_warning(crate::core::diagnostics::warning(
                                        HWPX_WARNING_SOURCE,
                                        format!(
                                            "Image reference '{}' matches multiple resource filenames by stem; \
                                             image extraction is ambiguous and was skipped",
                                            img_ref.id
                                        ),
                                    ));
                                    continue;
                                }
                            };
                            let image = ExtractedImage {
                                data: Bytes::from(resource.data.clone()),
                                format: mime_to_format(resource.mime_type.as_deref().unwrap_or("")),
                                image_index: image_index as u32,
                                page_number: None,
                                width: img_ref.width,
                                height: img_ref.height,
                                colorspace: None,
                                bits_per_component: None,
                                is_mask: false,
                                description: img_ref.alt_text.clone(),
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
                            builder.push_image(img_ref.alt_text.as_deref(), image, None, None);
                            image_index += 1;
                        }
                    }
                }
                unhwp::model::Block::Table(t) => {
                    if !t.rows.is_empty() {
                        let mut cells: Vec<Vec<String>> = Vec::with_capacity(t.rows.len());
                        for row in &t.rows {
                            let mut row_cells = Vec::with_capacity(row.cells.len());
                            for cell in &row.cells {
                                row_cells.push(cell_plain_text(&mut builder, cell, &mut footnote_counter));
                            }
                            cells.push(row_cells);
                        }
                        push_table(&mut builder, cells, t.has_header);
                    }
                }
            }

            if matches!(block, unhwp::model::Block::Paragraph(_)) {
                // The scan is the only source of formula elements, because it
                // sees an equation wherever it sits. Emitting here keeps the
                // equation next to the paragraph that introduces it.
                // The two sides count paragraphs independently, and a shape
                // that holds its own paragraphs makes the crate emit more
                // blocks than the scan saw. Emitting everything up to the
                // current ordinal places a formula late when the counts drift,
                // rather than stranding every formula after it.
                while let Some((ordinal, latex)) = scanned.and_then(|list| list.get(next_formula)) {
                    if *ordinal > paragraph_ordinal {
                        break;
                    }
                    builder.push_formula(latex, None, None);
                    next_formula += 1;
                }
                paragraph_ordinal += 1;
            }
        }

        // An equation the block walk never reached still belongs to the output.
        for (_, latex) in scanned.into_iter().flatten().skip(next_formula) {
            builder.push_formula(latex, None, None);
        }
    }

    // Checked before a header is built, since building records its footnotes.
    let mut seen: ahash::AHashSet<(bool, Vec<String>)> = ahash::AHashSet::new();
    for (paragraphs, layer) in furniture {
        let key: Vec<String> = paragraphs
            .iter()
            .map(|p| p.plain_text().trim().to_string())
            .filter(|text| !text.is_empty())
            .collect();
        if seen.insert((layer == ContentLayer::Header, key)) {
            push_header_footer_paragraphs(&mut builder, paragraphs, layer, &mut footnote_counter);
        }
    }
    for (layer, paragraphs) in scanned_furniture {
        if furniture_layer_kept(*layer, filter) && seen.insert((*layer == ContentLayer::Header, paragraphs.clone())) {
            for text in paragraphs {
                let idx = builder.push_paragraph(text, vec![], None, None);
                builder.set_layer(idx, *layer);
            }
        }
    }

    builder.build()
}

#[derive(Clone, Copy)]
enum ImageResourceMatch<'a> {
    Found(&'a unhwp::model::Resource),
    Missing,
    Ambiguous,
}

struct ImageResourceLookup<'a> {
    resources: &'a std::collections::BTreeMap<String, unhwp::model::Resource>,
    stems: AHashMap<&'a str, ImageResourceMatch<'a>>,
}

impl<'a> ImageResourceLookup<'a> {
    fn new(resources: &'a std::collections::BTreeMap<String, unhwp::model::Resource>) -> Self {
        let mut stems = AHashMap::with_capacity(resources.len());
        for (name, resource) in resources {
            let Some(stem) = std::path::Path::new(name).file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            stems
                .entry(stem)
                .and_modify(|entry| *entry = ImageResourceMatch::Ambiguous)
                .or_insert(ImageResourceMatch::Found(resource));
        }
        Self { resources, stems }
    }

    fn get(&self, image_id: &str) -> ImageResourceMatch<'a> {
        self.resources
            .get(image_id)
            .map(ImageResourceMatch::Found)
            .or_else(|| self.stems.get(image_id).copied())
            .unwrap_or(ImageResourceMatch::Missing)
    }
}

/// Push a header's or footer's paragraphs, tagging every element with the given
/// `ContentLayer` so downstream filters (`include_headers`/`include_footers`) can find
/// them (#96 — these were previously not read from `unhwp`'s model at all).
fn push_header_footer_paragraphs(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[unhwp::model::Paragraph],
    layer: ContentLayer,
    footnote_counter: &mut u32,
) {
    for p in paragraphs {
        let (text, annotations) = build_paragraph_content(builder, p, footnote_counter);
        let (trimmed, adjusted) = trim_text_and_annotations(&text, annotations);
        if trimmed.is_empty() {
            continue;
        }
        let idx = builder.push_paragraph(trimmed, adjusted, None, None);
        builder.set_layer(idx, layer);
    }
}

/// Builds a paragraph's flattened text plus byte-offset annotations from its inline
/// content, and records footnote definitions as a side effect.
///
/// This replaces the previous reliance on `Paragraph::plain_text()`, which silently
/// dropped footnote text (`InlineContent::Footnote` fell through `plain_text()`'s
/// catch-all arm) and hyperlink targets (only the anchor text survived) (#97), and
/// rendered equations as their raw HWP script instead of the LaTeX `unhwp` already
/// computes (#98).
fn build_paragraph_content(
    builder: &mut InternalDocumentBuilder,
    p: &unhwp::model::Paragraph,
    footnote_counter: &mut u32,
) -> (String, Vec<TextAnnotation>) {
    let mut text = String::new();
    let mut annotations = Vec::new();
    // An equation leaves the text, so the spaces that surrounded it would
    // otherwise meet and read as a gap.
    let mut after_equation = false;

    for item in &p.content {
        match item {
            unhwp::model::InlineContent::Text(run) => {
                let value = if after_equation && text.ends_with(char::is_whitespace) {
                    run.text.trim_start()
                } else {
                    run.text.as_str()
                };
                text.push_str(value);
                after_equation = false;
            }
            unhwp::model::InlineContent::LineBreak => text.push('\n'),
            unhwp::model::InlineContent::Link { text: link_text, url } => {
                let start = text.len() as u32;
                text.push_str(link_text);
                let end = text.len() as u32;
                if start < end {
                    annotations.push(TextAnnotation {
                        start,
                        end,
                        kind: AnnotationKind::Link {
                            url: url.clone(),
                            title: None,
                        },
                    });
                }
            }
            unhwp::model::InlineContent::Equation(eq) => {
                // `unhwp::equation::to_latex` converts HWP's own EQEdit script DSL
                // (not MathML/OMML — HWP equations use a distinct script format) to
                // LaTeX. The parser leaves `Equation.latex` unset (only `unhwp`'s own
                // Markdown renderer calls `to_latex`), so it is invoked directly here
                // instead of duplicating a second HWP-script-to-LaTeX converter (#98). ~keep
                let latex = eq
                    .latex
                    .clone()
                    .unwrap_or_else(|| unhwp::equation::to_latex(&eq.script));
                // An HWP equation is an object rather than inline notation, so it
                // leaves the paragraph text, the shape DOCX and PPTX use for a math
                // run. `collect_section_formulas` emits the element for it.
                if !latex.trim().is_empty() {
                    after_equation = true;
                }
            }
            unhwp::model::InlineContent::Footnote(note_text) => {
                *footnote_counter += 1;
                let key = format!("hwpx-fn{footnote_counter}");
                text.push_str(&format!("[^{footnote_counter}]"));
                if !note_text.trim().is_empty() {
                    let idx = builder.push_footnote_definition(note_text.trim(), &key, None);
                    builder.set_layer(idx, ContentLayer::Footnote);
                }
            }
            unhwp::model::InlineContent::Image(_) => {
                // Images are extracted separately by the caller, which has access to
                // `doc.resources` for the binary payload.
            }
        }
    }

    (text, annotations)
}

/// Flattens a table cell's paragraphs to a single string for the cell grid, still
/// routing through `build_paragraph_content` so footnotes/links/equations inside a
/// cell are captured rather than silently dropped by `TableCell::plain_text()` (#120).
fn cell_plain_text(
    builder: &mut InternalDocumentBuilder,
    cell: &unhwp::model::TableCell,
    footnote_counter: &mut u32,
) -> String {
    let mut lines = Vec::with_capacity(cell.content.len());
    for p in &cell.content {
        // `build_paragraph_content`'s `InlineContent::Image` arm is a no-op: it relies
        // on a second pass over `p.content` that only the top-level section/paragraph
        // loop performs (to reach `doc.resources` for the binary payload). Table cells
        // never get that second pass, so an image inside a cell is dropped outright --
        // not even as a placeholder -- unlike a resolvable top-level image (#171). ~keep
        if p.content
            .iter()
            .any(|c| matches!(c, unhwp::model::InlineContent::Image(_)))
        {
            builder.add_warning(crate::core::diagnostics::warning(
                HWPX_WARNING_SOURCE,
                "A table cell contains an image; images inside table cells are not \
                 extracted and were omitted from the output",
            ));
        }
        let (text, _annotations) = build_paragraph_content(builder, p, footnote_counter);
        lines.push(text.trim().to_string());
    }
    lines.join("\n")
}

/// Pushes a table, setting `Table::columns` from the first row when `unhwp` reports a
/// header row — previously always `None` regardless of `has_header` (#120). Note:
/// `unhwp`'s HWPX parser currently sets `has_header` on every non-empty table
/// (`hwpx/section.rs::parse_table`), so this is best-effort until that upstream signal
/// is more precise; it is still strictly more informative than never populating
/// `columns` at all.
fn push_table(builder: &mut InternalDocumentBuilder, cells: Vec<Vec<String>>, has_header: bool) -> u32 {
    let markdown = crate::rendering::common::render_table_markdown(&cells);
    let columns = if has_header { cells.first().cloned() } else { None };
    let table = crate::types::Table {
        cells,
        markdown,
        columns,
        ..Default::default()
    };
    builder.push_table(table, None, None)
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for HwpxExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();

        if content.len() as u64 > limits.max_archive_size as u64 {
            return Err(crate::XbergError::validation(format!(
                "HWPX file exceeds size limit ({} > {} bytes)",
                content.len(),
                limits.max_archive_size
            )));
        }

        let cursor = Cursor::new(content);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| crate::XbergError::parsing(format!("invalid HWPX zip: {e}")))?;
        ZipBombValidator::new(limits)
            .validate(&mut archive)
            .map_err(|e| crate::XbergError::validation(e.to_string()))?;

        let section_formulas = collect_section_formulas(&mut archive);
        let furniture = collect_section_furniture(&mut archive);

        let doc = unhwp::parse_bytes(content)
            .map_err(|e| crate::XbergError::parsing(format!("Failed to parse HWPX: {e}")))?;
        let mut internal = build_hwpx_internal_document_with_furniture(
            doc,
            mime_type,
            &section_formulas,
            &furniture,
            config.content_filter.as_ref(),
        );
        if let Some(filter) = &config.content_filter {
            internal.apply_furniture_filter(filter);
        }
        Ok(internal)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/haansofthwpx", "application/hwp+zip"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;
    use unhwp::model::{
        Block, Document, Equation, InlineContent, Paragraph, Section, Table, TableCell, TableRow, TextRun,
    };

    /// Build a HWPX package in memory holding one section.
    fn hwpx_package(sections: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buffer));
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("mimetype", stored).unwrap();
            std::io::Write::write_all(&mut writer, b"application/hwp+zip").unwrap();
            for (name, xml) in sections {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, xml.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer
    }

    fn scan(package: &[u8]) -> AHashMap<usize, Vec<(usize, String)>> {
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();
        collect_section_formulas(&mut archive)
    }

    /// Hangul writes the equation into the run, which `unhwp` skips, so the scan
    /// is the only thing that finds it.
    /// A formula is placed after the paragraph that holds it, and a section's
    /// map is read by the section's own number.
    #[test]
    fn test_a_section_formula_follows_its_paragraph() {
        use unhwp::model::{Block, Document, Paragraph, Section, TextRun};

        let mut doc = Document::new();
        let mut section = Section::new(2);
        for text in ["First.", "Second.", "Third."] {
            let mut p = Paragraph::new();
            p.push_text(TextRun::new(text));
            section.content.push(Block::Paragraph(p));
        }
        doc.sections.push(section);

        // The equation sits in the third paragraph, which is ordinal 2.
        let scanned: AHashMap<usize, Vec<(usize, String)>> = [(2usize, vec![(2usize, "\\frac{a}{b}".to_string())])]
            .into_iter()
            .collect();
        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &scanned);

        let kinds: Vec<&ElementKind> = internal.elements.iter().map(|e| &e.kind).collect();
        let formula_at = kinds
            .iter()
            .position(|k| matches!(k, ElementKind::Formula))
            .expect("the equation reaches the document");
        assert_eq!(
            formula_at,
            kinds.len() - 1,
            "the equation follows the third paragraph, got {kinds:?}"
        );
    }

    #[test]
    fn test_scan_reads_an_equation_inside_a_run() {
        let xml = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p><hp:run><hp:t>Prose.</hp:t></hp:run></hp:p>
  <hp:p><hp:run><hp:equation><hp:script>a OVER b</hp:script></hp:equation></hp:run></hp:p>
</hs:sec>"#;

        let found = scan(&hwpx_package(&[("Contents/section0.xml", xml)]));

        assert_eq!(
            found.get(&0).map(Vec::as_slice),
            Some(&[(1usize, "\\frac{a}{b}".to_string())][..])
        );
    }

    /// `&amp;` separates the columns of a matrix, so a reader that drops entity
    /// references merges them silently.
    #[test]
    fn test_scan_resolves_entity_references_in_a_script() {
        let xml = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p><hp:run><hp:equation><hp:script>bmatrix { 1 &amp; 2 # 3 &amp; 4 }</hp:script></hp:equation></hp:run></hp:p>
</hs:sec>"#;

        let found = scan(&hwpx_package(&[("Contents/section0.xml", xml)]));

        let latex = &found.get(&0).expect("section 0 has an equation")[0].1;
        // Compare against the script with its references already resolved. A
        // reader that drops them converts `1 &amp; 2` as `1 2`, which differs.
        let expected = unhwp::equation::to_latex("bmatrix { 1 & 2 # 3 & 4 }");
        assert_eq!(
            latex,
            expected.trim(),
            "the scan must resolve `&amp;` before conversion"
        );
        assert_ne!(
            latex,
            unhwp::equation::to_latex("bmatrix { 1  2 # 3  4 }").trim(),
            "a dropped reference must not produce the same LaTeX"
        );
    }

    /// `ZipBombValidator::validate` only checks the *declared* central-directory
    /// sizes; it never decompresses. `collect_section_formulas` must bound its own
    /// read of a member's content with `Read::take(MAX_HWPX_MEMBER_SIZE)` rather
    /// than trusting the declared size, or a section whose entry decompresses to
    /// far more than declared can still exhaust memory. An XML comment (ignored by
    /// the parser, so it does not disturb element nesting) padded past the cap
    /// proves the bound: an equation entirely before the cap is found, one that
    /// only starts after it is not, and the scan completes without hanging or
    /// panicking on the oversized member.
    #[test]
    fn test_scan_bounds_the_read_of_an_oversized_section_member() {
        let before = r#"<hp:p><hp:run><hp:equation><hp:script>a OVER b</hp:script></hp:equation></hp:run></hp:p>"#;
        let padding = "x".repeat(MAX_HWPX_MEMBER_SIZE as usize + 4096);
        let after = r#"<hp:p><hp:run><hp:equation><hp:script>p OVER q</hp:script></hp:equation></hp:run></hp:p>"#;
        let xml = format!(
            "<hs:sec xmlns:hp=\"http://www.hancom.co.kr/hwpml/2011/paragraph\">{before}<!--{padding}-->{after}</hs:sec>"
        );

        let found = scan(&hwpx_package(&[("Contents/section0.xml", &xml)]));

        let formulas = found.get(&0).expect("the equation before the cap is found");
        assert_eq!(
            formulas.as_slice(),
            &[(0usize, "\\frac{a}{b}".to_string())][..],
            "only the equation entirely within the first MAX_HWPX_MEMBER_SIZE bytes must be found; \
             finding the second equation would mean the read was not actually bounded"
        );
    }

    /// A section keeps its own index, so a section the crate cannot read does
    /// not shift the equations of the sections after it.
    #[test]
    fn test_scan_keys_each_section_by_its_own_index() {
        let one = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p><hp:run><hp:equation><hp:script>x OVER y</hp:script></hp:equation></hp:run></hp:p>
</hs:sec>"#;
        let two = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p><hp:run><hp:equation><hp:script>p OVER q</hp:script></hp:equation></hp:run></hp:p>
</hs:sec>"#;

        let found = scan(&hwpx_package(&[
            ("Contents/section0.xml", one),
            ("Contents/section2.xml", two),
        ]));

        // The parts are keyed by position, which is how the crate numbers a
        // section. A package that skips a number still lines the two sides up.
        assert_eq!(found.get(&0).map(|f| f[0].1.as_str()), Some("\\frac{x}{y}"));
        assert_eq!(found.get(&1).map(|f| f[0].1.as_str()), Some("\\frac{p}{q}"));
        assert!(found.get(&2).is_none(), "only two parts exist");
    }

    /// A paragraph inside a table stays in the cell for `unhwp`, so counting it
    /// would shift the ordinal of every paragraph after the table.
    #[test]
    fn test_scan_does_not_count_a_paragraph_inside_a_table() {
        let xml = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p><hp:run><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>Cell.</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl></hp:run></hp:p>
  <hp:p><hp:run><hp:equation><hp:script>a OVER b</hp:script></hp:equation></hp:run></hp:p>
</hs:sec>"#;

        let found = scan(&hwpx_package(&[("Contents/section0.xml", xml)]));

        assert_eq!(
            found.get(&0).map(|f| f[0].0),
            Some(1),
            "the equation is in the second paragraph"
        );
    }

    #[test]
    fn test_mime_to_format_maps_svg_wmf_emf() {
        assert_eq!(mime_to_format("image/svg+xml"), Cow::Borrowed("svg"));
        assert_eq!(mime_to_format("image/x-wmf"), Cow::Borrowed("wmf"));
        assert_eq!(mime_to_format("image/x-emf"), Cow::Borrowed("emf"));
        assert_eq!(mime_to_format("image/png"), Cow::Borrowed("png"));
        assert_eq!(mime_to_format("application/octet-stream"), Cow::Borrowed("bin"));
    }

    #[test]
    fn test_section_header_and_footer_are_extracted() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        section.header = Some(vec![Paragraph::text("Confidential Draft")]);
        section.footer = Some(vec![Paragraph::text("Page footer text")]);
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let header = internal
            .elements
            .iter()
            .find(|e| e.layer == ContentLayer::Header)
            .expect("header element must be present");
        assert_eq!(header.text, "Confidential Draft");

        let footer = internal
            .elements
            .iter()
            .find(|e| e.layer == ContentLayer::Footer)
            .expect("footer element must be present");
        assert_eq!(footer.text, "Page footer text");
    }

    fn hwpx_document_with_furniture() -> Document {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        section.header = Some(vec![Paragraph::text("Running header")]);
        section.footer = Some(vec![Paragraph::text("Running footer")]);
        section.content.push(Block::Paragraph(Paragraph::text("Body text")));
        doc.sections.push(section);
        doc
    }

    #[test]
    fn hwpx_header_and_footer_text_renders_only_when_the_content_filter_includes_it() {
        use crate::core::config::{ContentFilterConfig, OutputFormat};
        for format in [OutputFormat::Markdown, OutputFormat::Plain] {
            for (headers, footers) in [(true, true), (true, false), (false, true), (false, false)] {
                let mut internal = build_hwpx_internal_document(
                    hwpx_document_with_furniture(),
                    "application/haansofthwpx",
                    &AHashMap::new(),
                );
                internal.apply_furniture_filter(&ContentFilterConfig {
                    include_headers: headers,
                    include_footers: footers,
                    ..Default::default()
                });
                let content =
                    crate::extraction::derive::derive_extraction_result(internal, false, format.clone()).content;
                assert!(content.contains("Body text"), "{content}");
                assert_eq!(content.contains("Running header"), headers, "{format:?} {content}");
                assert_eq!(content.contains("Running footer"), footers, "{format:?} {content}");
                if let Some(at) = content.find("Running header") {
                    assert!(at > content.find("Body text").unwrap(), "{content}");
                }
            }
        }
    }

    #[test]
    fn hwpx_header_and_footer_text_is_not_rendered_by_default() {
        let internal = build_hwpx_internal_document(
            hwpx_document_with_furniture(),
            "application/haansofthwpx",
            &AHashMap::new(),
        );
        let content = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        )
        .content;
        assert_eq!(content.trim(), "Body text");
    }

    fn hwpx_manifest(sections: usize) -> String {
        let items: String = (0..sections)
            .map(|n| {
                format!(r#"<opf:item id="section{n}" href="Contents/section{n}.xml" media-type="application/xml"/>"#)
            })
            .collect();
        let spine: String = (0..sections)
            .map(|n| format!(r#"<opf:itemref idref="section{n}"/>"#))
            .collect();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><opf:package xmlns:opf="http://www.idpf.org/2007/opf"><opf:manifest>{items}</opf:manifest><opf:spine>{spine}</opf:spine></opf:package>"#
        )
    }

    fn hwpx_section(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">
  <hp:p><hp:run><hp:t>{body}</hp:t></hp:run></hp:p>
  <hp:p><hp:run><hp:ctrl><hp:header id="1"><hp:subList><hp:p><hp:run><hp:t>Running header</hp:t></hp:run></hp:p></hp:subList></hp:header></hp:ctrl></hp:run></hp:p>
  <hp:p><hp:run><hp:ctrl><hp:footer id="2"><hp:subList><hp:p><hp:run><hp:t>Running footer</hp:t></hp:run></hp:p></hp:subList></hp:footer></hp:ctrl></hp:run></hp:p>
</hs:sec>"#
        )
    }

    async fn extract_hwpx_text(sections: &[&str], headers: bool, footers: bool, filter: bool) -> String {
        use crate::core::config::{ContentFilterConfig, OutputFormat};
        let parts: Vec<String> = sections.iter().map(|body| hwpx_section(body)).collect();
        let manifest = hwpx_manifest(parts.len());
        let names: Vec<String> = (0..parts.len()).map(|n| format!("Contents/section{n}.xml")).collect();
        let mut files = vec![("Contents/content.hpf", manifest.as_str())];
        files.extend(
            names
                .iter()
                .zip(&parts)
                .map(|(name, xml)| (name.as_str(), xml.as_str())),
        );
        let config = ExtractionConfig {
            content_filter: filter.then(|| ContentFilterConfig {
                include_headers: headers,
                include_footers: footers,
                ..Default::default()
            }),
            ..Default::default()
        };
        let doc = HwpxExtractor::new()
            .extract_content(&hwpx_package(&files), "application/haansofthwpx", &config)
            .await
            .unwrap();
        crate::extraction::derive::derive_extraction_result(doc, false, OutputFormat::Plain).content
    }

    #[tokio::test]
    async fn hwpx_extractor_renders_header_and_footer_only_when_the_content_filter_includes_them() {
        for (headers, footers) in [(true, true), (true, false), (false, true), (false, false)] {
            let content = extract_hwpx_text(&["Body one"], headers, footers, true).await;
            assert!(content.contains("Body one"), "{content}");
            assert_eq!(content.contains("Running header"), headers, "{content}");
            assert_eq!(content.contains("Running footer"), footers, "{content}");
            if headers {
                assert!(content.find("Running header").unwrap() > content.find("Body one").unwrap());
            }
        }
        let content = extract_hwpx_text(&["Body one"], true, true, false).await;
        assert!(!content.contains("Running"), "no filter keeps the default: {content}");
    }

    #[tokio::test]
    async fn hwpx_extractor_emits_a_header_repeated_across_sections_once() {
        let content = extract_hwpx_text(&["Body one", "Body two"], true, true, true).await;
        assert!(
            content.contains("Body one") && content.contains("Body two"),
            "{content}"
        );
        assert_eq!(content.matches("Running header").count(), 1, "{content}");
        assert_eq!(content.matches("Running footer").count(), 1, "{content}");
    }

    #[tokio::test]
    async fn hwpx_extractor_keeps_a_paragraph_repeated_inside_one_header() {
        use crate::core::config::ContentFilterConfig;
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">
  <hp:p><hp:run><hp:t>Body</hp:t></hp:run></hp:p>
  <hp:p><hp:run><hp:ctrl><hp:header id="1"><hp:subList><hp:p><hp:run><hp:t>Alpha</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>Beta</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>Alpha</hp:t></hp:run></hp:p></hp:subList></hp:header></hp:ctrl></hp:run></hp:p>
</hs:sec>"#;
        let manifest = hwpx_manifest(1);
        let package = hwpx_package(&[
            ("Contents/content.hpf", manifest.as_str()),
            ("Contents/section0.xml", xml),
        ]);
        let config = ExtractionConfig {
            content_filter: Some(ContentFilterConfig {
                include_headers: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let doc = HwpxExtractor::new()
            .extract_content(&package, "application/haansofthwpx", &config)
            .await
            .unwrap();
        let header: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| e.layer == ContentLayer::Header)
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(header, ["Alpha", "Beta", "Alpha"]);
    }

    #[test]
    fn hwpx_furniture_scan_turns_a_tab_into_a_space_and_splits_nested_paragraphs() {
        let xml = r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">
  <hp:p><hp:run><hp:ctrl><hp:header id="1"><hp:subList>
    <hp:p><hp:run><hp:t>Company<hp:tab/>Page 1</hp:t></hp:run></hp:p>
    <hp:p><hp:run><hp:t>Title</hp:t><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>Cell</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl></hp:run></hp:p>
  </hp:subList></hp:header></hp:ctrl></hp:run></hp:p>
</hs:sec>"#;
        let package = hwpx_package(&[("Contents/section0.xml", xml)]);
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();
        let found = collect_section_furniture(&mut archive);
        assert_eq!(
            found,
            vec![(
                ContentLayer::Header,
                vec!["Company Page 1".to_string(), "Title".to_string(), "Cell".to_string()]
            )]
        );
    }

    #[test]
    fn hwpx_footnote_inside_a_dropped_header_does_not_leak() {
        use crate::core::config::ContentFilterConfig;
        let mut header = Paragraph::text("Header note ");
        header
            .content
            .push(InlineContent::Footnote("Secret header footnote".to_string()));
        let mut doc = Document::new();
        let mut section = Section::new(0);
        section.header = Some(vec![header]);
        section.content.push(Block::Paragraph(Paragraph::text("Body text")));
        doc.sections.push(section);
        let filter = ContentFilterConfig {
            include_headers: false,
            include_footers: true,
            ..Default::default()
        };
        let internal = build_hwpx_internal_document_with_furniture(
            doc,
            "application/haansofthwpx",
            &AHashMap::new(),
            &[],
            Some(&filter),
        );
        assert!(
            internal.elements.iter().all(|e| !e.text.contains("Secret")),
            "{:?}",
            internal.elements.iter().map(|e| &e.text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_footnote_produces_marker_and_definition() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.push_text(TextRun::new("See "));
        p.content.push(InlineContent::Footnote("The note body.".to_string()));
        p.push_text(TextRun::new(" done"));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let body = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::Paragraph)
            .expect("body paragraph must be present");
        assert_eq!(body.text, "See [^1] done");

        let definition = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::FootnoteDefinition)
            .expect("footnote definition element must be present");
        assert_eq!(definition.text, "The note body.");
        assert_eq!(definition.layer, ContentLayer::Footnote);
    }

    #[test]
    fn test_link_produces_link_annotation_with_correct_offsets() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.push_text(TextRun::new("Go to "));
        p.content.push(InlineContent::Link {
            text: "our site".to_string(),
            url: "https://example.com".to_string(),
        });
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let elem = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::Paragraph)
            .expect("paragraph must be present");
        assert_eq!(elem.text, "Go to our site");
        assert_eq!(elem.annotations.len(), 1);
        let annotation = &elem.annotations[0];
        assert_eq!(annotation.start, 6);
        assert_eq!(annotation.end, 14);
        assert_eq!(
            annotation.kind,
            AnnotationKind::Link {
                url: "https://example.com".to_string(),
                title: None,
            }
        );
    }

    /// An equation is an object, so its LaTeX becomes a formula element and
    /// leaves the paragraph's own text.
    #[test]
    fn test_equation_becomes_a_formula_element() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.push_text(TextRun::new("Result: "));
        p.content.push(InlineContent::Equation(Equation::new("FRAC{a}{b}")));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let section_formulas: AHashMap<usize, Vec<(usize, String)>> =
            [(0usize, vec![(0usize, "\\frac{a}{b}".to_string())])]
                .into_iter()
                .collect();
        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &section_formulas);

        let formulas: Vec<&str> = internal
            .elements
            .iter()
            .filter(|e| e.kind == ElementKind::Formula)
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(formulas, vec!["\\frac{a}{b}"]);

        let elem = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::Paragraph)
            .expect("paragraph must be present");
        assert_eq!(elem.text, "Result:");
    }

    /// The spaces that surrounded an equation would meet once it leaves the
    /// text, so the sentence keeps exactly one.
    #[test]
    fn test_removing_an_equation_leaves_one_space() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.push_text(TextRun::new("The ratio is "));
        p.content.push(InlineContent::Equation(Equation::new("x OVER y")));
        p.push_text(TextRun::new(" per unit."));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let para = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::Paragraph)
            .expect("paragraph must be present");
        assert_eq!(para.text, "The ratio is per unit.");
    }

    /// A paragraph holding nothing but an equation still carries its math: the
    /// formula element stands in for the paragraph that no longer has text.
    #[test]
    fn test_equation_only_paragraph_keeps_its_math() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.content.push(InlineContent::Equation(Equation::new("FRAC{a}{b}")));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let section_formulas: AHashMap<usize, Vec<(usize, String)>> =
            [(0usize, vec![(0usize, "\\frac{a}{b}".to_string())])]
                .into_iter()
                .collect();
        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &section_formulas);

        let formulas: Vec<&str> = internal
            .elements
            .iter()
            .filter(|e| e.kind == ElementKind::Formula)
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(formulas, vec!["\\frac{a}{b}"], "the equation survives (#98)");
    }

    #[test]
    fn test_table_header_row_populates_columns_and_cell_content() {
        let mut doc = Document::new();
        let mut section = Section::new(0);

        let mut table = Table::new();
        table.has_header = true;
        let mut header_row = TableRow::new();
        header_row.cells.push(TableCell::text("Name"));
        header_row.cells.push(TableCell::text("Age"));
        table.rows.push(header_row);
        let mut data_row = TableRow::new();
        data_row.cells.push(TableCell::text("Alice"));
        data_row.cells.push(TableCell::text("30"));
        table.rows.push(data_row);

        section.content.push(Block::Table(table));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let table_elem = internal
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Table { .. }))
            .expect("table element must be present");
        let ElementKind::Table { table_index } = table_elem.kind else {
            unreachable!()
        };
        let extracted_table = &internal.tables[table_index as usize];
        assert_eq!(
            extracted_table.columns,
            Some(vec!["Name".to_string(), "Age".to_string()])
        );
        assert_eq!(
            extracted_table.cells,
            vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ]
        );
    }

    #[test]
    fn test_table_without_header_leaves_columns_unset() {
        let mut doc = Document::new();
        let mut section = Section::new(0);

        let mut table = Table::new();
        table.has_header = false;
        let mut row = TableRow::new();
        row.cells.push(TableCell::text("A"));
        table.rows.push(row);

        section.content.push(Block::Table(table));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let table_elem = internal
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Table { .. }))
            .expect("table element must be present");
        let ElementKind::Table { table_index } = table_elem.kind else {
            unreachable!()
        };
        assert_eq!(internal.tables[table_index as usize].columns, None);
    }

    #[test]
    fn test_table_cell_footnote_is_extracted_not_dropped() {
        let mut doc = Document::new();
        let mut section = Section::new(0);

        let mut table = Table::new();
        let mut row = TableRow::new();
        let mut cell = TableCell::new();
        let mut cell_para = Paragraph::new();
        cell_para.content.push(InlineContent::Footnote("cell note".to_string()));
        cell_para.push_text(TextRun::new("cell body"));
        cell.content.push(cell_para);
        row.cells.push(cell);
        table.rows.push(row);

        section.content.push(Block::Table(table));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let definition = internal
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::FootnoteDefinition)
            .expect("footnote inside a table cell must still produce a definition");
        assert_eq!(definition.text, "cell note");
    }

    fn hwpx_warnings(doc: &InternalDocument) -> Vec<String> {
        doc.processing_warnings
            .iter()
            .filter(|w| w.source == HWPX_WARNING_SOURCE)
            .map(|w| w.message.to_string())
            .collect()
    }

    /// #171: an `InlineContent::Image` whose `id` has no entry in
    /// `doc.resources` cannot be resolved to binary data and is dropped
    /// entirely (no image element, no placeholder).
    #[test]
    fn should_warn_when_image_resource_id_is_missing_from_resources() {
        let mut doc = Document::new();
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("bin0")));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);
        // Deliberately leave `doc.resources` empty: "bin0" is never registered.

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let warnings = hwpx_warnings(&internal);
        assert_eq!(warnings.len(), 1, "expected exactly one hwpx warning, got {warnings:?}");
        assert!(
            warnings[0].contains("bin0") && warnings[0].contains("could not be extracted"),
            "warning must name the unresolved image id, got {warnings:?}"
        );
        assert!(
            internal.images.is_empty(),
            "an unresolved image must not produce an image element"
        );
    }

    /// A resolvable image (its id present in `doc.resources`) must not warn.
    #[test]
    fn should_not_warn_when_image_resource_resolves() {
        let mut doc = Document::new();
        doc.resources.insert(
            "bin0".to_string(),
            unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![0x89, 0x50, 0x4E, 0x47]),
        );
        let mut section = Section::new(0);
        let mut p = Paragraph::new();
        p.content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("bin0")));
        section.content.push(Block::Paragraph(p));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        assert!(
            hwpx_warnings(&internal).is_empty(),
            "a resolvable image must not warn, got {:?}",
            hwpx_warnings(&internal)
        );
    }

    #[test]
    fn should_resolve_image_resource_when_filename_has_an_extension() {
        let mut doc = Document::new();
        doc.resources.insert(
            "BIN0001.png".to_string(),
            unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![0x89, 0x50, 0x4E, 0x47]),
        );
        let mut section = Section::new(0);
        let mut paragraph = Paragraph::new();
        paragraph
            .content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("BIN0001")));
        section.content.push(Block::Paragraph(paragraph));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        assert_eq!(internal.images.len(), 1);
        assert!(hwpx_warnings(&internal).is_empty());
    }

    #[test]
    fn should_warn_instead_of_choosing_an_ambiguous_image_filename_stem() {
        let mut doc = Document::new();
        for filename in ["BIN0001.png", "BIN0001.jpg"] {
            doc.resources.insert(
                filename.to_string(),
                unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![0x89, 0x50, 0x4E, 0x47]),
            );
        }
        let mut section = Section::new(0);
        let mut paragraph = Paragraph::new();
        paragraph
            .content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("BIN0001")));
        section.content.push(Block::Paragraph(paragraph));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        assert!(internal.images.is_empty());
        assert_eq!(hwpx_warnings(&internal).len(), 1);
        assert!(hwpx_warnings(&internal)[0].contains("ambiguous"));
    }

    #[test]
    fn should_prefer_an_exact_image_resource_key_over_filename_stem_fallbacks() {
        let mut doc = Document::new();
        doc.resources.insert(
            "BIN0001".to_string(),
            unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![1]),
        );
        for filename in ["BIN0001.png", "BIN0001.jpg"] {
            doc.resources.insert(
                filename.to_string(),
                unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![2]),
            );
        }
        let mut section = Section::new(0);
        let mut paragraph = Paragraph::new();
        paragraph
            .content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("BIN0001")));
        section.content.push(Block::Paragraph(paragraph));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        assert_eq!(internal.images.len(), 1);
        assert_eq!(internal.images[0].data.as_ref(), &[1]);
        assert!(hwpx_warnings(&internal).is_empty());
    }

    /// #171: `build_paragraph_content`'s `InlineContent::Image` arm is a no-op
    /// everywhere; only the top-level section/paragraph loop performs the
    /// second pass that actually resolves and pushes an image element. A
    /// table cell never gets that second pass, so an image inside a cell is
    /// dropped outright.
    #[test]
    fn should_warn_when_table_cell_contains_an_image() {
        let mut doc = Document::new();
        doc.resources.insert(
            "bin0".to_string(),
            unhwp::model::Resource::new(unhwp::model::ResourceType::Image, vec![0x89, 0x50, 0x4E, 0x47]),
        );
        let mut section = Section::new(0);

        let mut table = Table::new();
        let mut row = TableRow::new();
        let mut cell = TableCell::new();
        let mut cell_para = Paragraph::new();
        cell_para
            .content
            .push(InlineContent::Image(unhwp::model::ImageRef::new("bin0")));
        cell.content.push(cell_para);
        row.cells.push(cell);
        table.rows.push(row);

        section.content.push(Block::Table(table));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        let warnings = hwpx_warnings(&internal);
        assert_eq!(warnings.len(), 1, "expected exactly one hwpx warning, got {warnings:?}");
        assert!(
            warnings[0].contains("table cell") && warnings[0].contains("not extracted"),
            "warning must describe the dropped table-cell image, got {warnings:?}"
        );
        assert!(
            internal.images.is_empty(),
            "an image inside a table cell must not produce an image element"
        );
    }

    /// An ordinary table with only text cells must not warn.
    #[test]
    fn should_not_warn_for_table_with_only_text_cells() {
        let mut doc = Document::new();
        let mut section = Section::new(0);

        let mut table = Table::new();
        let mut row = TableRow::new();
        row.cells.push(TableCell::text("Alice"));
        row.cells.push(TableCell::text("30"));
        table.rows.push(row);

        section.content.push(Block::Table(table));
        doc.sections.push(section);

        let internal = build_hwpx_internal_document(doc, "application/haansofthwpx", &AHashMap::new());

        assert!(
            hwpx_warnings(&internal).is_empty(),
            "a table with only text cells must not warn, got {:?}",
            hwpx_warnings(&internal)
        );
    }

    #[test]
    fn test_hwpx_extractor_plugin_interface() {
        let extractor = HwpxExtractor::new();
        assert_eq!(extractor.name(), "hwpx-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(
            extractor.supported_mime_types(),
            &["application/haansofthwpx", "application/hwp+zip"]
        );
    }

    #[test]
    fn test_hwpx_extractor_initialize_shutdown() {
        let extractor = HwpxExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_hwpx_extract_real_document() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/hwpx/simple.hwpx");
        let content = std::fs::read(path).expect("test_documents/hwpx/simple.hwpx must exist");
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&content, "application/haansofthwpx", &ExtractionConfig::default())
            .await
            .expect("extraction of simple.hwpx must succeed");

        let text = result.content();
        assert!(
            text.contains("Hello from HWPX document"),
            "expected body text not found; got: {text}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_extract_corrupted_returns_err() {
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(b"not a zip", "application/haansofthwpx", &ExtractionConfig::default())
            .await;
        assert!(result.is_err(), "corrupted input must return Err, not panic");
    }

    fn make_zip_with_ratio(uncompressed_len: usize) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("content.hml", opts).unwrap();
        zw.write_all(&vec![0u8; uncompressed_len]).unwrap();
        zw.finish().unwrap();
        buf.into_inner()
    }

    fn make_zip_with_n_files(n: usize) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        for i in 0..n {
            zw.start_file(format!("f{i}.bin"), opts).unwrap();
            zw.write_all(b"x").unwrap();
        }
        zw.finish().unwrap();
        buf.into_inner()
    }

    #[tokio::test]
    async fn test_hwpx_rejects_zip_bomb_default_limits() {
        let zip_bytes = make_zip_with_ratio(32 * 1024 * 1024);
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&zip_bytes, "application/haansofthwpx", &ExtractionConfig::default())
            .await;
        assert!(result.is_err(), "default limits must block a >100:1 zip bomb");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ZIP bomb") || err.contains("ratio") || err.contains("validation"),
            "error should mention bomb/ratio/validation, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_zip_bomb() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_ratio(32 * 1024 * 1024);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_compression_ratio: 1,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "zip bomb must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ZIP bomb") || err.contains("ratio") || err.contains("validation"),
            "error should mention bomb/ratio/validation, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_oversized_file() {
        use crate::extractors::security::SecurityLimits;
        let limits = SecurityLimits {
            max_archive_size: 10,
            ..SecurityLimits::default()
        };
        let config = ExtractionConfig {
            security_limits: Some(limits),
            ..ExtractionConfig::default()
        };
        let oversized = vec![0u8; 11];
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&oversized, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "oversized file must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("size limit") || err.contains("validation"),
            "error should mention size limit, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_too_many_files() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_n_files(3);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_files_in_archive: 2,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "archive exceeding file-count limit must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("files") || err.contains("count") || err.contains("validation"),
            "error should mention file count, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_valid_zip_passes_security_check() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_ratio(1024);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_compression_ratio: 10_000,
                max_archive_size: 10 * 1024 * 1024,
                max_files_in_archive: 1_000,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_content(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        let is_parse_err = match &result {
            Err(e) => {
                let msg = e.to_string();
                !msg.contains("ZIP bomb")
                    && !msg.contains("ratio")
                    && !msg.contains("size limit")
                    && !msg.contains("too many files")
            }
            Ok(_) => true,
        };
        assert!(
            is_parse_err,
            "security validator must not reject a safe ZIP; got: {result:?}"
        );
    }
}
