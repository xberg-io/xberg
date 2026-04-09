//! PowerPoint presentation extraction functions.
//!
//! This module provides PowerPoint (PPTX) file parsing by directly reading the Office Open XML
//! format. It extracts text content, slide structure, images, and presentation metadata.
//!
//! # Attribution
//!
//! This code is based on the [pptx-to-md](https://github.com/nilskruthoff/pptx-parser) library
//! by Nils Kruthoff, licensed under MIT OR Apache-2.0. The original code has been vendored and
//! adapted to integrate with Kreuzberg's architecture. See ATTRIBUTIONS.md for full license text.
//!
//! # Features
//!
//! - **Slide extraction**: Reads all slides from presentation
//! - **Text formatting**: Preserves bold, italic, underline formatting as Markdown
//! - **Image extraction**: Optionally extracts embedded images with metadata
//! - **Office metadata**: Extracts core properties, custom properties (when `office` feature enabled)
//! - **Structure preservation**: Maintains heading hierarchy and list structure
//!
//! # Supported Formats
//!
//! - `.pptx` - PowerPoint Presentation
//! - `.pptm` - PowerPoint Macro-Enabled Presentation
//! - `.ppsx` - PowerPoint Slide Show
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::pptx::extract_pptx_from_path;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let result = extract_pptx_from_path("presentation.pptx", true, None, false, false, true)?;
//!
//! println!("Slide count: {}", result.slide_count);
//! println!("Image count: {}", result.image_count);
//! println!("Content:\n{}", result.content);
//! # Ok(())
//! # }
//! ```

mod container;
mod content_builder;
mod elements;
mod image_handling;
mod metadata;
mod parser;

use ahash::AHashMap;
use bytes::Bytes;

use crate::error::Result;
use crate::types::builder::{self, DocumentStructureBuilder};
use crate::types::document_structure::TextAnnotation;
use crate::types::extraction::BoundingBox;
use crate::types::{ExtractedImage, PptxExtractionResult};

use container::{PptxContainer, SlideIterator};
use content_builder::ContentBuilder;
use elements::{ParserConfig, Run, SlideElement};
use image_handling::detect_image_format;
use metadata::{extract_all_notes, extract_metadata};

/// Join text runs with smart spacing: inserts a space between adjacent runs
/// only when the previous run doesn't end with whitespace and the next run
/// doesn't start with whitespace.
fn join_runs_with_spacing(runs: &[Run], extract: impl Fn(&Run) -> String) -> String {
    let mut result = String::new();
    for run in runs {
        let text = extract(run);
        if !result.is_empty() && !text.is_empty() {
            let ends_ws = result.ends_with(|c: char| c.is_whitespace());
            let starts_ws = text.starts_with(|c: char| c.is_whitespace());
            if !ends_ws && !starts_ws {
                result.push(' ');
            }
        }
        result.push_str(&text);
    }
    result
}

/// Extract PPTX content from a file path.
///
/// # Arguments
///
/// * `path` - Path to the PPTX file
/// * `extract_images` - Whether to extract embedded images
/// * `page_config` - Optional page configuration for boundary tracking
/// * `plain` - Whether to output plain text (no markdown)
/// * `include_structure` - Whether to build the `DocumentStructure` tree
/// * `inject_placeholders` - Whether to emit `![alt](target)` references in markdown output
///
/// # Returns
///
/// A `PptxExtractionResult` containing extracted content, metadata, and images.
pub fn extract_pptx_from_path(
    path: &str,
    extract_images: bool,
    page_config: Option<&crate::core::config::PageConfig>,
    plain: bool,
    include_structure: bool,
    inject_placeholders: bool,
) -> Result<PptxExtractionResult> {
    let container = PptxContainer::open(path)?;
    extract_pptx_from_container(
        container,
        extract_images,
        page_config,
        plain,
        include_structure,
        inject_placeholders,
    )
}

/// Extract PPTX content from a byte buffer.
///
/// # Arguments
///
/// * `data` - Raw PPTX file bytes
/// * `extract_images` - Whether to extract embedded images
/// * `page_config` - Optional page configuration for boundary tracking
/// * `plain` - Whether to output plain text (no markdown)
/// * `include_structure` - Whether to build the `DocumentStructure` tree
/// * `inject_placeholders` - Whether to emit `![alt](target)` references in markdown output
///
/// # Returns
///
/// A `PptxExtractionResult` containing extracted content, metadata, and images.
pub fn extract_pptx_from_bytes(
    data: &[u8],
    extract_images: bool,
    page_config: Option<&crate::core::config::PageConfig>,
    plain: bool,
    include_structure: bool,
    inject_placeholders: bool,
) -> Result<PptxExtractionResult> {
    let container = PptxContainer::from_bytes(data)?;
    extract_pptx_from_container(
        container,
        extract_images,
        page_config,
        plain,
        include_structure,
        inject_placeholders,
    )
}

fn extract_pptx_from_container<R: std::io::Read + std::io::Seek>(
    mut container: PptxContainer<R>,
    extract_images: bool,
    page_config: Option<&crate::core::config::PageConfig>,
    plain: bool,
    include_structure: bool,
    inject_placeholders: bool,
) -> Result<PptxExtractionResult> {
    let config = ParserConfig {
        extract_images,
        plain,
        inject_placeholders,
        ..Default::default()
    };

    let (metadata, office_metadata) = extract_metadata(&mut container.archive);

    let notes = extract_all_notes(&mut container)?;

    let mut iterator = SlideIterator::new(container);
    let slide_count = iterator.slide_count();

    let estimated_capacity = slide_count.saturating_mul(1000).max(8192);
    let mut content_builder = ContentBuilder::with_page_config(estimated_capacity, page_config.cloned(), plain);

    let mut total_image_count = 0;
    let mut total_table_count = 0;
    let mut extracted_images = Vec::new();
    let mut collected_hyperlinks: Vec<(String, Option<String>)> = Vec::new();
    let mut doc_builder = if include_structure {
        Some(DocumentStructureBuilder::new().source_format("pptx"))
    } else {
        None
    };
    let mut image_index_counter: u32 = 0;

    while let Some(slide) = iterator.next_slide()? {
        let byte_start = if page_config.is_some() {
            content_builder.start_slide(slide.slide_number)
        } else {
            0
        };

        let slide_content = slide.to_markdown(&config);
        content_builder.add_text(&slide_content);

        if let Some(slide_notes) = notes.get(&slide.slide_number) {
            content_builder.add_notes(slide_notes);
        }

        if page_config.is_some() {
            content_builder.end_slide(slide.slide_number, byte_start, slide_content.clone());
        }

        // Build document structure for this slide (only when requested)
        if let Some(ref mut builder) = doc_builder {
            build_slide_structure(&slide, builder, &mut image_index_counter);
        }

        // Collect hyperlinks from runs that have hlinkClick references
        collect_slide_hyperlinks(&slide, &mut collected_hyperlinks);

        if config.extract_images
            && let Ok(image_data) = iterator.get_slide_images(&slide)
        {
            // Collect image element metadata for matching
            let image_elements: Vec<_> = slide
                .elements
                .iter()
                .filter_map(|e| {
                    if let SlideElement::Image(img_ref, pos) = e {
                        Some((img_ref, pos))
                    } else {
                        None
                    }
                })
                .collect();

            for (img_idx_in_slide, (_, data)) in image_data.iter().enumerate() {
                let format = detect_image_format(data);
                let image_index = extracted_images.len();

                // Try to get dimensions and description from corresponding element
                // EMU to pixels at 96 DPI: 1 pixel = 9525 EMU
                let (width, height, description, bbox) =
                    if let Some((img_ref, pos)) = image_elements.get(img_idx_in_slide) {
                        let w = if pos.cx > 0 { Some((pos.cx / 9525) as u32) } else { None };
                        let h = if pos.cy > 0 { Some((pos.cy / 9525) as u32) } else { None };
                        (w, h, img_ref.description.clone(), position_to_bbox(pos))
                    } else {
                        (None, None, None, None)
                    };

                extracted_images.push(ExtractedImage {
                    data: Bytes::from(data.clone()),
                    format,
                    image_index,
                    page_number: Some(slide.slide_number as usize),
                    width,
                    height,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description,
                    ocr_result: None,
                    bounding_box: bbox,
                    source_path: None,
                });
            }
        }

        total_image_count += slide.image_count();
        total_table_count += slide.table_count();
    }

    let (content, boundaries, mut page_contents) = content_builder.build();

    // Refine is_blank: slides that have images are not blank
    if let Some(ref mut pcs) = page_contents {
        for pc in pcs.iter_mut() {
            if extracted_images
                .iter()
                .any(|img| img.page_number == Some(pc.page_number))
            {
                pc.is_blank = Some(false);
            }
        }
    }

    let page_structure = boundaries.as_ref().map(|bounds| crate::types::PageStructure {
        total_count: slide_count,
        unit_type: crate::types::PageUnitType::Slide,
        boundaries: Some(bounds.clone()),
        pages: page_contents.as_ref().map(|pcs| {
            pcs.iter()
                .map(|pc| crate::types::PageInfo {
                    number: pc.page_number,
                    title: None,
                    dimensions: None,
                    image_count: None,
                    table_count: None,
                    hidden: None,
                    is_blank: pc.is_blank,
                })
                .collect()
        }),
    });

    let document = doc_builder
        .map(|b| b.build())
        .and_then(|d| if d.is_empty() { None } else { Some(d) });

    Ok(PptxExtractionResult {
        content,
        metadata,
        slide_count,
        image_count: total_image_count,
        table_count: total_table_count,
        images: extracted_images,
        page_structure,
        page_contents,
        document,
        hyperlinks: collected_hyperlinks,
        office_metadata,
    })
}

/// Build annotations from a sequence of text runs, tracking byte offsets.
///
/// Returns the concatenated plain text and the corresponding annotations.
fn runs_to_text_and_annotations(runs: &[Run]) -> (String, Vec<TextAnnotation>) {
    let mut text = String::new();
    let mut annotations = Vec::new();

    for run in runs {
        let run_text = &run.text;
        if run_text.is_empty() {
            continue;
        }

        // Insert a space between runs when needed
        if !text.is_empty() {
            let ends_ws = text.ends_with(|c: char| c.is_whitespace());
            let starts_ws = run_text.starts_with(|c: char| c.is_whitespace());
            if !ends_ws && !starts_ws {
                text.push(' ');
            }
        }

        let start = text.len() as u32;
        text.push_str(run_text);
        let end = text.len() as u32;

        if run.formatting.bold {
            annotations.push(builder::bold(start, end));
        }
        if run.formatting.italic {
            annotations.push(builder::italic(start, end));
        }
        if run.formatting.underlined {
            annotations.push(builder::underline(start, end));
        }
        if run.formatting.strikethrough {
            annotations.push(builder::strikethrough(start, end));
        }
        if let Some(sz) = run.formatting.font_size {
            // sz is in hundredths of a point; convert to "Xpt" string
            let pts = sz as f64 / 100.0;
            let value = if pts.fract() == 0.0 {
                format!("{}pt", pts as u32)
            } else {
                format!("{:.1}pt", pts)
            };
            annotations.push(builder::font_size(start, end, &value));
        }
    }

    (text, annotations)
}

/// Convert an `ElementPosition` with dimensions to a `BoundingBox`.
///
/// EMU coordinates are converted to points (1 inch = 914400 EMU = 72 pt).
fn position_to_bbox(pos: &elements::ElementPosition) -> Option<BoundingBox> {
    if pos.x == 0 && pos.y == 0 && pos.cx == 0 && pos.cy == 0 {
        return None;
    }
    const EMU_PER_PT: f64 = 914_400.0 / 72.0;
    Some(BoundingBox {
        x0: pos.x as f64 / EMU_PER_PT,
        y0: pos.y as f64 / EMU_PER_PT,
        x1: (pos.x + pos.cx) as f64 / EMU_PER_PT,
        y1: (pos.y + pos.cy) as f64 / EMU_PER_PT,
    })
}

/// Populate the document structure builder for a single slide.
fn build_slide_structure(
    slide: &elements::Slide,
    doc_builder: &mut DocumentStructureBuilder,
    image_index_counter: &mut u32,
) {
    // Determine slide title: first short text element
    let mut sorted_indices: Vec<usize> = (0..slide.elements.len()).collect();
    sorted_indices.sort_by_key(|&i| {
        let pos = slide.elements[i].position();
        (pos.y, pos.x)
    });

    // Find the slide title: prefer explicit title placeholder, fall back to
    // first short text element.
    let slide_title = sorted_indices
        .iter()
        .find_map(|&idx| {
            if let SlideElement::Text(text, _) = &slide.elements[idx]
                && text.is_title
            {
                let plain = join_runs_with_spacing(&text.runs, Run::extract);
                if !plain.trim().is_empty() {
                    return Some(plain.trim().to_string());
                }
            }
            None
        })
        .or_else(|| {
            sorted_indices.iter().find_map(|&idx| {
                if let SlideElement::Text(text, _) = &slide.elements[idx] {
                    let plain = join_runs_with_spacing(&text.runs, Run::extract);
                    let normalized = plain.replace('\n', " ");
                    if normalized.len() < 100 && !normalized.trim().is_empty() {
                        return Some(normalized.trim().to_string());
                    }
                }
                None
            })
        });

    doc_builder.push_slide(slide.slide_number, slide_title.as_deref());

    let mut first_title_seen = false;

    for &idx in &sorted_indices {
        let elem = &slide.elements[idx];
        let pos = elem.position();
        let bbox = position_to_bbox(&pos);

        match elem {
            SlideElement::Text(text, _) => {
                let (plain_text, annotations) = runs_to_text_and_annotations(&text.runs);
                let normalized = plain_text.replace('\n', " ");
                let is_title_elem = text.is_title || (normalized.len() < 100 && !normalized.trim().is_empty());

                if is_title_elem && !first_title_seen {
                    first_title_seen = true;
                    doc_builder.push_heading(1, normalized.trim(), None, bbox);
                } else if !plain_text.trim().is_empty() {
                    let node_idx = doc_builder.push_paragraph(&plain_text, annotations, None, bbox);

                    // Store language from first run with a non-empty lang
                    if let Some(lang) = text.runs.iter().find_map(|r| {
                        let l = &r.formatting.lang;
                        if l.is_empty() { None } else { Some(l.clone()) }
                    }) {
                        let mut attrs = AHashMap::new();
                        attrs.insert("lang".to_string(), lang);
                        doc_builder.set_attributes(node_idx, attrs);
                    }
                }
            }
            SlideElement::Table(table, _) => {
                let cells: Vec<Vec<String>> = table
                    .rows
                    .iter()
                    .map(|row| {
                        row.cells
                            .iter()
                            .map(|cell| join_runs_with_spacing(&cell.runs, Run::extract))
                            .collect()
                    })
                    .collect();
                if !cells.is_empty() {
                    doc_builder.push_table_from_cells(&cells, None);
                }
            }
            SlideElement::List(list, _) => {
                if !list.items.is_empty() {
                    let is_ordered = list.items.first().is_some_and(|item| item.is_ordered);
                    let list_node = doc_builder.push_list(is_ordered, None);
                    for item in &list.items {
                        let item_text = join_runs_with_spacing(&item.runs, Run::extract);
                        if !item_text.trim().is_empty() {
                            doc_builder.push_list_item(list_node, item_text.trim(), None);
                        }
                    }
                }
            }
            SlideElement::Image(img_ref, _) => {
                // Prefer alt text description, fall back to target path
                let desc = img_ref.description.as_deref().or({
                    if img_ref.target.is_empty() {
                        None
                    } else {
                        Some(img_ref.target.as_str())
                    }
                });
                doc_builder.push_image(desc, Some(*image_index_counter), None, bbox);
                *image_index_counter += 1;
            }
            SlideElement::Unknown => {}
        }
    }

    doc_builder.exit_container();
}

/// Collect hyperlinks from all runs in a slide by resolving `hlinkClick` rIds
/// against the slide's hyperlink relationships.
fn collect_slide_hyperlinks(slide: &elements::Slide, out: &mut Vec<(String, Option<String>)>) {
    // Helper: iterate all runs from all elements
    let mut visit_runs = |runs: &[Run]| {
        for run in runs {
            if let Some(ref hlink_id) = run.hyperlink_id
                && let Some(href) = slide.hyperlinks.iter().find(|h| h.id == *hlink_id)
            {
                let label = if run.text.trim().is_empty() {
                    None
                } else {
                    Some(run.text.trim().to_string())
                };
                out.push((href.url.clone(), label));
            }
        }
    };

    for elem in &slide.elements {
        match elem {
            SlideElement::Text(text, _) => visit_runs(&text.runs),
            SlideElement::List(list, _) => {
                for item in &list.items {
                    visit_runs(&item.runs);
                }
            }
            SlideElement::Table(table, _) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        visit_runs(&cell.runs);
                    }
                }
            }
            _ => {}
        }
    }
}

// Re-export Slide implementation methods for internal use
impl elements::Slide {
    fn from_xml(slide_number: u32, xml_data: &[u8], rels_data: Option<&[u8]>) -> Result<Self> {
        let elements = parser::parse_slide_xml(xml_data)?;

        let (images, hyperlinks) = if let Some(rels) = rels_data {
            let slide_rels = parser::parse_slide_rels(rels)?;
            (slide_rels.images, slide_rels.hyperlinks)
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(Self {
            slide_number,
            elements,
            images,
            hyperlinks,
        })
    }

    fn to_markdown(&self, config: &ParserConfig) -> String {
        let mut builder = ContentBuilder::new(config.plain);

        if config.include_slide_comment {
            builder.add_slide_header(self.slide_number);
        }

        let mut element_indices: Vec<usize> = (0..self.elements.len()).collect();
        element_indices.sort_by_key(|&i| {
            let pos = self.elements[i].position();
            (pos.y, pos.x)
        });

        // Find the slide title: prefer a shape with an explicit title placeholder
        // type (type="title" or type="ctrTitle"). Fall back to the first short text
        // element in y-sorted order when no placeholder is marked.
        let title_idx = element_indices
            .iter()
            .find_map(|&idx| {
                if let SlideElement::Text(text, _) = &self.elements[idx]
                    && text.is_title
                {
                    let plain = join_runs_with_spacing(&text.runs, Run::extract);
                    if !plain.trim().is_empty() {
                        return Some(idx);
                    }
                }
                None
            })
            .or_else(|| {
                // Fallback: first short text element
                element_indices.iter().find_map(|&idx| {
                    if let SlideElement::Text(text, _) = &self.elements[idx] {
                        let plain = join_runs_with_spacing(&text.runs, Run::extract);
                        let normalized = plain.replace('\n', " ");
                        if normalized.len() < 100 && !normalized.trim().is_empty() {
                            return Some(idx);
                        }
                    }
                    None
                })
            });

        // Emit title first as a heading
        if let Some(tidx) = title_idx
            && let SlideElement::Text(text, _) = &self.elements[tidx]
        {
            let text_content: String = if config.plain {
                join_runs_with_spacing(&text.runs, Run::extract)
            } else {
                join_runs_with_spacing(&text.runs, Run::render_as_md)
            };
            let normalized = text_content.replace('\n', " ");
            builder.add_title(normalized.trim());
        }

        for &idx in &element_indices {
            // Skip the title element we already emitted
            if Some(idx) == title_idx {
                continue;
            }

            match &self.elements[idx] {
                SlideElement::Text(text, _) => {
                    let text_content: String = if config.plain {
                        join_runs_with_spacing(&text.runs, Run::extract)
                    } else {
                        join_runs_with_spacing(&text.runs, Run::render_as_md)
                    };

                    // All remaining text elements are body text, not titles
                    builder.add_text(&text_content);
                }
                SlideElement::Table(table, _) => {
                    let extract_fn: fn(&Run) -> String = if config.plain { Run::extract } else { Run::render_as_md };
                    let table_rows: Vec<Vec<String>> = table
                        .rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .iter()
                                .map(|cell| join_runs_with_spacing(&cell.runs, extract_fn))
                                .collect()
                        })
                        .collect();
                    builder.add_table(&table_rows);
                }
                SlideElement::List(list, _) => {
                    let extract_fn: fn(&Run) -> String = if config.plain { Run::extract } else { Run::render_as_md };
                    for item in &list.items {
                        let item_text = join_runs_with_spacing(&item.runs, extract_fn);
                        if item.has_bullet {
                            builder.add_list_item(item.level, item.is_ordered, &item_text);
                        } else {
                            // Non-bulleted preamble paragraph within a list shape.
                            // Render as plain text, not a list item.
                            builder.add_text(&item_text);
                        }
                    }
                }
                SlideElement::Image(img_ref, _) => {
                    // Only emit the markdown image reference when inject_placeholders
                    // is enabled (default true). When false, image data may still be
                    // extracted separately but the reference is not injected into text.
                    if config.inject_placeholders {
                        // Resolve image target from rels
                        let target = self
                            .images
                            .iter()
                            .find(|rel| rel.id == img_ref.id)
                            .map(|rel| rel.target.as_str())
                            .unwrap_or("");
                        builder.add_image_with_desc(&img_ref.id, img_ref.description.as_deref(), target);
                    }
                }
                SlideElement::Unknown => {}
            }
        }

        builder.build().0
    }

    fn image_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Image(_, _)))
            .count()
    }

    fn table_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Table(_, _)))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pptx_bytes(slides: Vec<&str>) -> Vec<u8> {
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("ppt/presentation.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

            let mut rels_xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            );
            for (i, _) in slides.iter().enumerate() {
                use std::fmt::Write;
                let _ = write!(
                    rels_xml,
                    r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                    i + 1,
                    i + 1
                );
            }
            rels_xml.push_str("</Relationships>");
            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(rels_xml.as_bytes()).unwrap();

            for (i, text) in slides.iter().enumerate() {
                let slide_xml = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>{}</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#,
                    text
                );
                zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), options)
                    .unwrap();
                zip.write_all(slide_xml.as_bytes()).unwrap();
            }

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
    <dc:title>Test Presentation</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:description>Test Description</dc:description>
    <dc:subject>Test Subject</dc:subject>
</cp:coreProperties>"#,
            )
            .unwrap();

            // Add app.xml with slide count
            let app_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Slides>{}</Slides>
    <Application>Microsoft Office PowerPoint</Application>
</Properties>"#,
                slides.len()
            );
            zip.start_file("docProps/app.xml", options).unwrap();
            zip.write_all(app_xml.as_bytes()).unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_extract_pptx_from_bytes_single_slide() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Hello World"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false, None, false, false, true).unwrap();

        assert_eq!(result.slide_count, 1);
        assert!(
            result.content.contains("Hello World"),
            "Content was: {}",
            result.content
        );
        assert_eq!(result.image_count, 0);
        assert_eq!(result.table_count, 0);
    }

    #[test]
    fn test_extract_pptx_from_bytes_multiple_slides() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false, None, false, false, true).unwrap();

        assert_eq!(result.slide_count, 3);
        assert!(result.content.contains("Slide 1"));
        assert!(result.content.contains("Slide 2"));
        assert!(result.content.contains("Slide 3"));
    }

    #[test]
    fn test_extract_pptx_metadata() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Content"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false, None, false, false, true).unwrap();

        // Metadata should be populated (slide_count should be 1 for the test content)
        assert_eq!(result.metadata.slide_count, 1);
    }

    #[test]
    fn test_extract_pptx_empty_slides() {
        let pptx_bytes = create_test_pptx_bytes(vec!["", "", ""]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false, None, false, false, true).unwrap();

        assert_eq!(result.slide_count, 3);
    }

    #[test]
    fn test_extract_pptx_from_bytes_invalid_data() {
        use crate::error::KreuzbergError;

        let invalid_bytes = b"not a valid pptx file";
        let result = extract_pptx_from_bytes(invalid_bytes, false, None, false, false, true);

        assert!(result.is_err());
        if let Err(KreuzbergError::Parsing { message: msg, .. }) = result {
            assert!(msg.contains("Failed to read PPTX archive") || msg.contains("Failed to write temp PPTX file"));
        } else {
            panic!("Expected ParsingError");
        }
    }

    #[test]
    fn test_extract_pptx_from_bytes_empty_data() {
        let empty_bytes: &[u8] = &[];
        let result = extract_pptx_from_bytes(empty_bytes, false, None, false, false, true);

        assert!(result.is_err());
    }

    #[test]
    fn test_detect_image_format_jpeg() {
        let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_format(&jpeg_header), "jpeg");
    }

    #[test]
    fn test_detect_image_format_png() {
        let png_header = vec![0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_image_format(&png_header), "png");
    }

    #[test]
    fn test_detect_image_format_gif() {
        let gif_header = b"GIF89a";
        assert_eq!(detect_image_format(gif_header), "gif");
    }

    #[test]
    fn test_detect_image_format_bmp() {
        let bmp_header = b"BM";
        assert_eq!(detect_image_format(bmp_header), "bmp");
    }

    #[test]
    fn test_detect_image_format_svg() {
        let svg_header = b"<svg xmlns=\"http://www.w3.org/2000/svg\">";
        assert_eq!(detect_image_format(svg_header), "svg");
    }

    #[test]
    fn test_detect_image_format_tiff_little_endian() {
        let tiff_header = vec![0x49, 0x49, 0x2A, 0x00];
        assert_eq!(detect_image_format(&tiff_header), "tiff");
    }

    #[test]
    fn test_detect_image_format_tiff_big_endian() {
        let tiff_header = vec![0x4D, 0x4D, 0x00, 0x2A];
        assert_eq!(detect_image_format(&tiff_header), "tiff");
    }

    #[test]
    fn test_detect_image_format_unknown() {
        let unknown_data = b"unknown format";
        assert_eq!(detect_image_format(unknown_data), "unknown");
    }

    #[test]
    fn test_get_slide_rels_path() {
        assert_eq!(
            image_handling::get_slide_rels_path("ppt/slides/slide1.xml"),
            "ppt/slides/_rels/slide1.xml.rels"
        );
        assert_eq!(
            image_handling::get_slide_rels_path("ppt/slides/slide10.xml"),
            "ppt/slides/_rels/slide10.xml.rels"
        );
    }

    #[test]
    fn test_get_full_image_path_relative() {
        assert_eq!(
            image_handling::get_full_image_path("ppt/slides/slide1.xml", "../media/image1.png"),
            "ppt/media/image1.png"
        );
    }

    #[test]
    fn test_get_full_image_path_direct() {
        assert_eq!(
            image_handling::get_full_image_path("ppt/slides/slide1.xml", "image1.png"),
            "ppt/slides/image1.png"
        );
    }

    // ── inject_placeholders tests (issue #671) ──

    /// Build a minimal PPTX ZIP with one slide that contains an image (<p:pic>) element.
    ///
    /// The slide XML includes a picture shape referencing rel id "rId2", and the
    /// slide rels file maps that id to "media/image1.png".
    fn create_pptx_with_image_slide(slide_text: &str) -> Vec<u8> {
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let opts = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="png" ContentType="image/png"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("_rels/.rels", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

            zip.start_file("ppt/presentation.xml", opts).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#).unwrap();

            // Slide with both a text run and a picture element
            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p><a:r><a:t>{slide_text}</a:t></a:r></a:p>
                </p:txBody>
            </p:sp>
            <p:pic>
                <p:nvPicPr>
                    <p:nvPr><p:ph type="pic"/></p:nvPr>
                    <p:cNvPicPr/>
                    <p:nvPr><a:hlinkClick r:id=""/></p:nvPr>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId2" descr="Test chart"/>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
                </p:spPr>
            </p:pic>
        </p:spTree>
    </p:cSld>
</p:sld>"#
            );
            zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            // Rels for the slide: rId2 → media/image1.png
            zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#).unwrap();

            // A tiny placeholder PNG (1×1 transparent)
            zip.start_file("ppt/media/image1.png", opts).unwrap();
            zip.write_all(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82").unwrap();

            zip.start_file("docProps/core.xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Image Test</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            zip.start_file("docProps/app.xml", opts).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Slides>1</Slides></Properties>").unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_inject_placeholders_true_emits_image_reference() {
        let pptx = create_pptx_with_image_slide("Hello");
        let result = extract_pptx_from_bytes(&pptx, false, None, false, false, true).unwrap();
        assert!(
            result.content.contains("!["),
            "inject_placeholders=true must emit image reference, got: {:?}",
            result.content
        );
    }

    #[test]
    fn test_inject_placeholders_false_suppresses_image_reference() {
        let pptx = create_pptx_with_image_slide("Hello");
        let result = extract_pptx_from_bytes(&pptx, false, None, false, false, false).unwrap();
        assert!(
            !result.content.contains("!["),
            "inject_placeholders=false must NOT emit image reference, got: {:?}",
            result.content
        );
    }

    #[test]
    fn test_inject_placeholders_false_preserves_text_content() {
        // Suppressing image references must not remove surrounding slide text.
        let pptx = create_pptx_with_image_slide("Quarterly Review");
        let result = extract_pptx_from_bytes(&pptx, false, None, false, false, false).unwrap();
        assert!(
            result.content.contains("Quarterly Review"),
            "Text content must survive when inject_placeholders=false, got: {:?}",
            result.content
        );
    }

    #[test]
    fn test_default_inject_placeholders_preserves_existing_behaviour() {
        // inject_placeholders defaults to true — existing call sites without the
        // new arg (via the public API passing true) must continue to emit refs.
        let pptx = create_pptx_with_image_slide("Slide Title");
        let result_true = extract_pptx_from_bytes(&pptx, false, None, false, false, true).unwrap();
        let result_false = extract_pptx_from_bytes(&pptx, false, None, false, false, false).unwrap();
        // With placeholders: content is longer (image refs add characters)
        assert!(
            result_true.content.len() >= result_false.content.len(),
            "inject_placeholders=true should produce >= content length vs false"
        );
    }
}
