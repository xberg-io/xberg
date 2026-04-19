//! ExtractionResult conversion to Ruby values
//!
//! Handles conversion of Kreuzberg ExtractionResult to Ruby Hash,
//! including complex nested structures like chunks, images, tables, and elements.

use crate::error_handling::runtime_error;
use crate::helpers::{json_value_to_ruby, set_hash_entry};

use kreuzberg::ExtractionResult as RustExtractionResult;
use magnus::{Error, RHash, Ruby, IntoValue};
use magnus::value::ReprValue;

/// Convert Kreuzberg ExtractionResult to Ruby Hash
///
/// Converts the Rust extraction result into a Ruby hash with all fields including:
/// - content, mime_type, metadata
/// - tables (with cells and markdown)
/// - detected_languages
/// - chunks (with embeddings)
/// - images (including OCR results)
/// - pages (with per-page content)
/// - elements (for element-based format)
pub fn extraction_result_to_ruby(ruby: &Ruby, result: RustExtractionResult) -> Result<RHash, Error> {
    let hash = ruby.hash_new();

    // Set content and MIME type
    let content_value = ruby.str_new(result.content.as_str()).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "content", content_value)?;

    let mime_value = ruby.str_new(result.mime_type.as_ref()).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "mime_type", mime_value)?;

    // Set metadata both as JSON string and parsed hash
    let metadata_json = serde_json::to_string(&result.metadata)
        .map_err(|e| runtime_error(format!("Failed to serialize metadata: {}", e)))?;
    let metadata_json_value = ruby.str_new(&metadata_json).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "metadata_json", metadata_json_value)?;
    let metadata_value = serde_json::to_value(&result.metadata)
        .map_err(|e| runtime_error(format!("Failed to serialize metadata: {}", e)))?;
    let metadata_hash = json_value_to_ruby(ruby, &metadata_value)?;
    set_hash_entry(ruby, &hash, "metadata", metadata_hash)?;

    // Convert tables
    let tables_array = ruby.ary_new();
    for table in result.tables {
        let table_hash = ruby.hash_new();

        let cells_array = ruby.ary_new();
        for row in table.cells {
            let row_array = ruby.ary_from_vec(row);
            cells_array.push(row_array)?;
        }
        table_hash.aset("cells", cells_array)?;
        table_hash.aset("markdown", table.markdown)?;
        table_hash.aset("page_number", table.page_number)?;
        if let Some(bbox) = table.bounding_box {
            let bbox_hash = ruby.hash_new();
            bbox_hash.aset("x0", bbox.x0)?;
            bbox_hash.aset("y0", bbox.y0)?;
            bbox_hash.aset("x1", bbox.x1)?;
            bbox_hash.aset("y1", bbox.y1)?;
            table_hash.aset("bounding_box", bbox_hash)?;
        } else {
            table_hash.aset("bounding_box", ruby.qnil().as_value())?;
        }

        tables_array.push(table_hash)?;
    }
    let tables_value = tables_array.into_value_with(ruby);
    set_hash_entry(ruby, &hash, "tables", tables_value)?;

    // Convert detected languages
    if let Some(langs) = result.detected_languages {
        let langs_array = ruby.ary_from_vec(langs);
        let langs_value = langs_array.into_value_with(ruby);
        set_hash_entry(ruby, &hash, "detected_languages", langs_value)?;
    } else {
        set_hash_entry(ruby, &hash, "detected_languages", ruby.qnil().as_value())?;
    }

    // Convert chunks
    if let Some(chunks) = result.chunks {
        let chunks_array = ruby.ary_new();
        for chunk in chunks {
            let chunk_hash = ruby.hash_new();
            chunk_hash.aset("content", chunk.content)?;
            chunk_hash.aset("byte_start", chunk.metadata.byte_start)?;
            chunk_hash.aset("byte_end", chunk.metadata.byte_end)?;
            if let Some(token_count) = chunk.metadata.token_count {
                chunk_hash.aset("token_count", token_count)?;
            } else {
                chunk_hash.aset("token_count", ruby.qnil().as_value())?;
            }
            chunk_hash.aset("chunk_index", chunk.metadata.chunk_index)?;
            chunk_hash.aset("total_chunks", chunk.metadata.total_chunks)?;
            if let Some(first_page) = chunk.metadata.first_page {
                chunk_hash.aset("first_page", first_page as i64)?;
            } else {
                chunk_hash.aset("first_page", ruby.qnil().as_value())?;
            }
            if let Some(last_page) = chunk.metadata.last_page {
                chunk_hash.aset("last_page", last_page as i64)?;
            } else {
                chunk_hash.aset("last_page", ruby.qnil().as_value())?;
            }
            if let Some(embedding) = chunk.embedding {
                let embedding_array = ruby.ary_new();
                for value in embedding {
                    embedding_array.push(ruby.float_from_f64(value as f64).into_value_with(ruby))?;
                }
                chunk_hash.aset("embedding", embedding_array)?;
            } else {
                chunk_hash.aset("embedding", ruby.qnil().as_value())?;
            }
            chunks_array.push(chunk_hash)?;
        }
        let chunks_value = chunks_array.into_value_with(ruby);
        set_hash_entry(ruby, &hash, "chunks", chunks_value)?;
    } else {
        set_hash_entry(ruby, &hash, "chunks", ruby.qnil().as_value())?;
    }

    // Convert images
    if let Some(images) = result.images {
        let images_array = ruby.ary_new();
        for image in images {
            let image_hash = ruby.hash_new();
            let data_value = ruby.str_from_slice(&image.data).into_value_with(ruby);
            image_hash.aset("data", data_value)?;
            let format_value = ruby.str_new(image.format.as_ref()).into_value_with(ruby);
            image_hash.aset("format", format_value)?;
            image_hash.aset("image_index", image.image_index as i64)?;
            if let Some(page) = image.page_number {
                image_hash.aset("page_number", page as i64)?;
            } else {
                image_hash.aset("page_number", ruby.qnil().as_value())?;
            }
            if let Some(width) = image.width {
                image_hash.aset("width", width as i64)?;
            } else {
                image_hash.aset("width", ruby.qnil().as_value())?;
            }
            if let Some(height) = image.height {
                image_hash.aset("height", height as i64)?;
            } else {
                image_hash.aset("height", ruby.qnil().as_value())?;
            }
            if let Some(colorspace) = image.colorspace {
                image_hash.aset("colorspace", colorspace)?;
            } else {
                image_hash.aset("colorspace", ruby.qnil().as_value())?;
            }
            if let Some(bits) = image.bits_per_component {
                image_hash.aset("bits_per_component", bits as i64)?;
            } else {
                image_hash.aset("bits_per_component", ruby.qnil().as_value())?;
            }
            image_hash.aset(
                "is_mask",
                if image.is_mask {
                    ruby.qtrue().as_value()
                } else {
                    ruby.qfalse().as_value()
                },
            )?;
            if let Some(description) = image.description {
                image_hash.aset("description", description)?;
            } else {
                image_hash.aset("description", ruby.qnil().as_value())?;
            }
            if let Some(ocr_result) = image.ocr_result {
                let nested = extraction_result_to_ruby(ruby, *ocr_result)?;
                image_hash.aset("ocr_result", nested.into_value_with(ruby))?;
            } else {
                image_hash.aset("ocr_result", ruby.qnil().as_value())?;
            }
            if let Some(bbox) = image.bounding_box {
                let bbox_hash = ruby.hash_new();
                bbox_hash.aset("x0", bbox.x0)?;
                bbox_hash.aset("y0", bbox.y0)?;
                bbox_hash.aset("x1", bbox.x1)?;
                bbox_hash.aset("y1", bbox.y1)?;
                image_hash.aset("bounding_box", bbox_hash)?;
            } else {
                image_hash.aset("bounding_box", ruby.qnil().as_value())?;
            }
            images_array.push(image_hash)?;
        }
        set_hash_entry(ruby, &hash, "images", images_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "images", ruby.qnil().as_value())?;
    }

    // Convert pages
    if let Some(page_content_list) = result.pages {
        let pages_array = ruby.ary_new();
        for page_content in page_content_list {
            let page_hash = ruby.hash_new();
            page_hash.aset("page_number", page_content.page_number as i64)?;
            page_hash.aset("content", page_content.content)?;

            let tables_array = ruby.ary_new();
            for table in page_content.tables {
                let table_hash = ruby.hash_new();

                let cells_array = ruby.ary_new();
                for row in table.cells.clone() {
                    let row_array = ruby.ary_from_vec(row);
                    cells_array.push(row_array)?;
                }
                table_hash.aset("cells", cells_array)?;
                table_hash.aset("markdown", table.markdown.clone())?;
                table_hash.aset("page_number", table.page_number as i64)?;
                if let Some(ref bbox) = table.bounding_box {
                    let bbox_hash = ruby.hash_new();
                    bbox_hash.aset("x0", bbox.x0)?;
                    bbox_hash.aset("y0", bbox.y0)?;
                    bbox_hash.aset("x1", bbox.x1)?;
                    bbox_hash.aset("y1", bbox.y1)?;
                    table_hash.aset("bounding_box", bbox_hash)?;
                } else {
                    table_hash.aset("bounding_box", ruby.qnil().as_value())?;
                }

                tables_array.push(table_hash)?;
            }
            page_hash.aset("tables", tables_array)?;

            let images_array = ruby.ary_new();
            for image in page_content.images {
                let image_hash = ruby.hash_new();
                let data_value = ruby.str_from_slice(&image.data).into_value_with(ruby);
                image_hash.aset("data", data_value)?;
                let format_value = ruby.str_new(image.format.as_ref()).into_value_with(ruby);
                image_hash.aset("format", format_value)?;
                image_hash.aset("image_index", image.image_index as i64)?;
                if let Some(page) = image.page_number {
                    image_hash.aset("page_number", page as i64)?;
                } else {
                    image_hash.aset("page_number", ruby.qnil().as_value())?;
                }
                if let Some(width) = image.width {
                    image_hash.aset("width", width as i64)?;
                } else {
                    image_hash.aset("width", ruby.qnil().as_value())?;
                }
                if let Some(height) = image.height {
                    image_hash.aset("height", height as i64)?;
                } else {
                    image_hash.aset("height", ruby.qnil().as_value())?;
                }
                if let Some(colorspace) = &image.colorspace {
                    image_hash.aset("colorspace", colorspace.clone())?;
                } else {
                    image_hash.aset("colorspace", ruby.qnil().as_value())?;
                }
                if let Some(bits) = image.bits_per_component {
                    image_hash.aset("bits_per_component", bits as i64)?;
                } else {
                    image_hash.aset("bits_per_component", ruby.qnil().as_value())?;
                }
                image_hash.aset(
                    "is_mask",
                    if image.is_mask {
                        ruby.qtrue().as_value()
                    } else {
                        ruby.qfalse().as_value()
                    },
                )?;
                if let Some(description) = &image.description {
                    image_hash.aset("description", description.clone())?;
                } else {
                    image_hash.aset("description", ruby.qnil().as_value())?;
                }
                if let Some(ocr_result) = &image.ocr_result {
                    let nested = extraction_result_to_ruby(ruby, (**ocr_result).clone())?;
                    image_hash.aset("ocr_result", nested.into_value_with(ruby))?;
                } else {
                    image_hash.aset("ocr_result", ruby.qnil().as_value())?;
                }
                if let Some(ref bbox) = image.bounding_box {
                    let bbox_hash = ruby.hash_new();
                    bbox_hash.aset("x0", bbox.x0)?;
                    bbox_hash.aset("y0", bbox.y0)?;
                    bbox_hash.aset("x1", bbox.x1)?;
                    bbox_hash.aset("y1", bbox.y1)?;
                    image_hash.aset("bounding_box", bbox_hash)?;
                } else {
                    image_hash.aset("bounding_box", ruby.qnil().as_value())?;
                }
                images_array.push(image_hash)?;
            }
            page_hash.aset("images", images_array)?;

            pages_array.push(page_hash)?;
        }
        set_hash_entry(ruby, &hash, "pages", pages_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "pages", ruby.qnil().as_value())?;
    }

    // Convert elements (element-based format)
    if let Some(elements_list) = result.elements {
        let elements_array = ruby.ary_new();
        for element in elements_list {
            let element_hash = ruby.hash_new();
            element_hash.aset("element_id", element.element_id.as_ref())?;

            // Convert ElementType to snake_case string
            use kreuzberg::types::ElementType as ET;
            let element_type_str = match element.element_type {
                ET::Title => "title",
                ET::NarrativeText => "narrative_text",
                ET::Heading => "heading",
                ET::ListItem => "list_item",
                ET::Table => "table",
                ET::Image => "image",
                ET::PageBreak => "page_break",
                ET::CodeBlock => "code_block",
                ET::BlockQuote => "block_quote",
                ET::Footer => "footer",
                ET::Header => "header",
            };
            element_hash.aset("element_type", element_type_str)?;
            element_hash.aset("text", element.text)?;

            let metadata_hash = ruby.hash_new();
            if let Some(page_num) = element.metadata.page_number {
                metadata_hash.aset("page_number", page_num as i64)?;
            } else {
                metadata_hash.aset("page_number", ruby.qnil().as_value())?;
            }
            if let Some(filename) = &element.metadata.filename {
                metadata_hash.aset("filename", filename.as_str())?;
            } else {
                metadata_hash.aset("filename", ruby.qnil().as_value())?;
            }
            if let Some(coords) = element.metadata.coordinates {
                let coords_hash = ruby.hash_new();
                coords_hash.aset("x0", coords.x0)?;
                coords_hash.aset("y0", coords.y0)?;
                coords_hash.aset("x1", coords.x1)?;
                coords_hash.aset("y1", coords.y1)?;
                metadata_hash.aset("coordinates", coords_hash)?;
            } else {
                metadata_hash.aset("coordinates", ruby.qnil().as_value())?;
            }
            if let Some(elem_idx) = element.metadata.element_index {
                metadata_hash.aset("element_index", elem_idx as i64)?;
            } else {
                metadata_hash.aset("element_index", ruby.qnil().as_value())?;
            }
            let additional_hash = ruby.hash_new();
            for (key, value) in &element.metadata.additional {
                additional_hash.aset(key.as_str(), value.as_str())?;
            }
            metadata_hash.aset("additional", additional_hash)?;

            element_hash.aset("metadata", metadata_hash)?;
            elements_array.push(element_hash)?;
        }
        set_hash_entry(ruby, &hash, "elements", elements_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "elements", ruby.qnil().as_value())?;
    }

    // Convert ocr_elements
    if let Some(ocr_elements) = result.ocr_elements {
        let elements_array = ruby.ary_new();
        for elem in ocr_elements {
            let elem_json = serde_json::to_value(&elem)
                .map_err(|e| runtime_error(format!("Failed to serialize ocr_element: {}", e)))?;
            let elem_ruby = json_value_to_ruby(ruby, &elem_json)?;
            elements_array.push(elem_ruby)?;
        }
        set_hash_entry(ruby, &hash, "ocr_elements", elements_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "ocr_elements", ruby.qnil().as_value())?;
    }

    // Convert document structure
    if let Some(doc_structure) = result.document {
        let document_hash = ruby.hash_new();
        let nodes_array = ruby.ary_new();

        for node in doc_structure.nodes {
            let node_hash = ruby.hash_new();
            node_hash.aset("id", node.id.as_ref())?;

            // Convert NodeContent to hash
            let content_hash = ruby.hash_new();
            use kreuzberg::types::NodeContent;
            match node.content {
                NodeContent::Title { text } => {
                    content_hash.aset("node_type", "title")?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Heading { level, text } => {
                    content_hash.aset("node_type", "heading")?;
                    content_hash.aset("level", level as i64)?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Paragraph { text } => {
                    content_hash.aset("node_type", "paragraph")?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::List { ordered } => {
                    content_hash.aset("node_type", "list")?;
                    content_hash.aset("ordered", if ordered { ruby.qtrue().as_value() } else { ruby.qfalse().as_value() })?;
                }
                NodeContent::ListItem { text } => {
                    content_hash.aset("node_type", "list_item")?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Table { grid } => {
                    content_hash.aset("node_type", "table")?;
                    let grid_hash = ruby.hash_new();
                    grid_hash.aset("rows", grid.rows as i64)?;
                    grid_hash.aset("cols", grid.cols as i64)?;
                    let cells_array = ruby.ary_new();
                    for cell in grid.cells {
                        let cell_hash = ruby.hash_new();
                        cell_hash.aset("content", cell.content)?;
                        cell_hash.aset("row", cell.row as i64)?;
                        cell_hash.aset("col", cell.col as i64)?;
                        cell_hash.aset("row_span", cell.row_span as i64)?;
                        cell_hash.aset("col_span", cell.col_span as i64)?;
                        cell_hash.aset("is_header", if cell.is_header { ruby.qtrue().as_value() } else { ruby.qfalse().as_value() })?;
                        if let Some(bbox) = cell.bbox {
                            let bbox_hash = ruby.hash_new();
                            bbox_hash.aset("x0", bbox.x0)?;
                            bbox_hash.aset("y0", bbox.y0)?;
                            bbox_hash.aset("x1", bbox.x1)?;
                            bbox_hash.aset("y1", bbox.y1)?;
                            cell_hash.aset("bbox", bbox_hash)?;
                        } else {
                            cell_hash.aset("bbox", ruby.qnil().as_value())?;
                        }
                        cells_array.push(cell_hash)?;
                    }
                    grid_hash.aset("cells", cells_array)?;
                    content_hash.aset("grid", grid_hash)?;
                }
                NodeContent::Image { description, image_index, src } => {
                    content_hash.aset("node_type", "image")?;
                    if let Some(desc) = description {
                        content_hash.aset("description", desc)?;
                    } else {
                        content_hash.aset("description", ruby.qnil().as_value())?;
                    }
                    if let Some(idx) = image_index {
                        content_hash.aset("image_index", idx as i64)?;
                    } else {
                        content_hash.aset("image_index", ruby.qnil().as_value())?;
                    }
                    if let Some(s) = src {
                        content_hash.aset("src", s)?;
                    } else {
                        content_hash.aset("src", ruby.qnil().as_value())?;
                    }
                }
                NodeContent::Code { text, language } => {
                    content_hash.aset("node_type", "code")?;
                    content_hash.aset("text", text)?;
                    if let Some(lang) = language {
                        content_hash.aset("language", lang)?;
                    } else {
                        content_hash.aset("language", ruby.qnil().as_value())?;
                    }
                }
                NodeContent::Quote => {
                    content_hash.aset("node_type", "quote")?;
                }
                NodeContent::Formula { text } => {
                    content_hash.aset("node_type", "formula")?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Footnote { text } => {
                    content_hash.aset("node_type", "footnote")?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Group { label, heading_level, heading_text } => {
                    content_hash.aset("node_type", "group")?;
                    if let Some(lbl) = label {
                        content_hash.aset("label", lbl)?;
                    } else {
                        content_hash.aset("label", ruby.qnil().as_value())?;
                    }
                    if let Some(level) = heading_level {
                        content_hash.aset("heading_level", level as i64)?;
                    } else {
                        content_hash.aset("heading_level", ruby.qnil().as_value())?;
                    }
                    if let Some(text) = heading_text {
                        content_hash.aset("heading_text", text)?;
                    } else {
                        content_hash.aset("heading_text", ruby.qnil().as_value())?;
                    }
                }
                NodeContent::PageBreak => {
                    content_hash.aset("node_type", "page_break")?;
                }
                NodeContent::Slide { number, title } => {
                    content_hash.aset("node_type", "slide")?;
                    content_hash.aset("number", number as i64)?;
                    if let Some(t) = title {
                        content_hash.aset("title", t)?;
                    } else {
                        content_hash.aset("title", ruby.qnil().as_value())?;
                    }
                }
                NodeContent::DefinitionList => {
                    content_hash.aset("node_type", "definition_list")?;
                }
                NodeContent::DefinitionItem { term, definition } => {
                    content_hash.aset("node_type", "definition_item")?;
                    content_hash.aset("term", term)?;
                    content_hash.aset("definition", definition)?;
                }
                NodeContent::Citation { key, text } => {
                    content_hash.aset("node_type", "citation")?;
                    content_hash.aset("key", key)?;
                    content_hash.aset("text", text)?;
                }
                NodeContent::Admonition { kind, title } => {
                    content_hash.aset("node_type", "admonition")?;
                    content_hash.aset("kind", kind)?;
                    if let Some(t) = title {
                        content_hash.aset("title", t)?;
                    } else {
                        content_hash.aset("title", ruby.qnil().as_value())?;
                    }
                }
                NodeContent::RawBlock { format, content } => {
                    content_hash.aset("node_type", "raw_block")?;
                    content_hash.aset("format", format)?;
                    content_hash.aset("content", content)?;
                }
                NodeContent::MetadataBlock { entries } => {
                    content_hash.aset("node_type", "metadata_block")?;
                    let entries_array = ruby.ary_new();
                    for (key, value) in entries {
                        let entry_array = ruby.ary_new();
                        entry_array.push(ruby.str_new(&key))?;
                        entry_array.push(ruby.str_new(&value))?;
                        entries_array.push(entry_array)?;
                    }
                    content_hash.aset("entries", entries_array)?;
                }
            }
            node_hash.aset("content", content_hash)?;

            if let Some(parent_idx) = node.parent {
                node_hash.aset("parent", parent_idx.0 as i64)?;
            } else {
                node_hash.aset("parent", ruby.qnil().as_value())?;
            }

            let children_array = ruby.ary_new();
            for child_idx in node.children {
                children_array.push(child_idx.0 as i64)?;
            }
            node_hash.aset("children", children_array)?;

            let layer_str = match node.content_layer {
                kreuzberg::types::ContentLayer::Body => "body",
                kreuzberg::types::ContentLayer::Header => "header",
                kreuzberg::types::ContentLayer::Footer => "footer",
                kreuzberg::types::ContentLayer::Footnote => "footnote",
            };
            node_hash.aset("content_layer", layer_str)?;

            if let Some(page) = node.page {
                node_hash.aset("page", page as i64)?;
            } else {
                node_hash.aset("page", ruby.qnil().as_value())?;
            }

            if let Some(page_end) = node.page_end {
                node_hash.aset("page_end", page_end as i64)?;
            } else {
                node_hash.aset("page_end", ruby.qnil().as_value())?;
            }

            if let Some(bbox) = node.bbox {
                let bbox_hash = ruby.hash_new();
                bbox_hash.aset("x0", bbox.x0)?;
                bbox_hash.aset("y0", bbox.y0)?;
                bbox_hash.aset("x1", bbox.x1)?;
                bbox_hash.aset("y1", bbox.y1)?;
                node_hash.aset("bbox", bbox_hash)?;
            } else {
                node_hash.aset("bbox", ruby.qnil().as_value())?;
            }

            let annotations_array = ruby.ary_new();
            for annotation in node.annotations {
                let ann_hash = ruby.hash_new();
                ann_hash.aset("start", annotation.start as i64)?;
                ann_hash.aset("end", annotation.end as i64)?;

                // Convert AnnotationKind to hash
                let kind_hash = ruby.hash_new();
                use kreuzberg::types::AnnotationKind;
                match annotation.kind {
                    AnnotationKind::Bold => {
                        kind_hash.aset("annotation_type", "bold")?;
                    }
                    AnnotationKind::Italic => {
                        kind_hash.aset("annotation_type", "italic")?;
                    }
                    AnnotationKind::Underline => {
                        kind_hash.aset("annotation_type", "underline")?;
                    }
                    AnnotationKind::Strikethrough => {
                        kind_hash.aset("annotation_type", "strikethrough")?;
                    }
                    AnnotationKind::Code => {
                        kind_hash.aset("annotation_type", "code")?;
                    }
                    AnnotationKind::Subscript => {
                        kind_hash.aset("annotation_type", "subscript")?;
                    }
                    AnnotationKind::Superscript => {
                        kind_hash.aset("annotation_type", "superscript")?;
                    }
                    AnnotationKind::Link { url, title } => {
                        kind_hash.aset("annotation_type", "link")?;
                        kind_hash.aset("url", url)?;
                        if let Some(t) = title {
                            kind_hash.aset("title", t)?;
                        } else {
                            kind_hash.aset("title", ruby.qnil().as_value())?;
                        }
                    }
                    AnnotationKind::Highlight => {
                        kind_hash.aset("annotation_type", "highlight")?;
                    }
                    AnnotationKind::Color { value } => {
                        kind_hash.aset("annotation_type", "color")?;
                        kind_hash.aset("value", value)?;
                    }
                    AnnotationKind::FontSize { value } => {
                        kind_hash.aset("annotation_type", "font_size")?;
                        kind_hash.aset("value", value)?;
                    }
                    AnnotationKind::Custom { name, value } => {
                        kind_hash.aset("annotation_type", "custom")?;
                        kind_hash.aset("name", name)?;
                        if let Some(v) = value {
                            kind_hash.aset("value", v)?;
                        } else {
                            kind_hash.aset("value", ruby.qnil().as_value())?;
                        }
                    }
                }
                ann_hash.aset("kind", kind_hash)?;
                annotations_array.push(ann_hash)?;
            }
            node_hash.aset("annotations", annotations_array)?;

            nodes_array.push(node_hash)?;
        }
        document_hash.aset("nodes", nodes_array)?;
        set_hash_entry(ruby, &hash, "document", document_hash.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "document", ruby.qnil().as_value())?;
    }

    // Convert extracted keywords
    if let Some(keywords) = result.extracted_keywords {
        let keywords_array = ruby.ary_new();
        for kw in keywords {
            let kw_hash = ruby.hash_new();
            kw_hash.aset("text", kw.text.as_str())?;
            kw_hash.aset("score", ruby.float_from_f64(kw.score as f64).into_value_with(ruby))?;
            let algo_str = serde_json::to_value(&kw.algorithm)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            kw_hash.aset("algorithm", algo_str.as_str())?;
            if let Some(positions) = kw.positions {
                let positions_array = ruby.ary_new();
                for pos in positions {
                    positions_array.push(pos as i64)?;
                }
                kw_hash.aset("positions", positions_array)?;
            } else {
                kw_hash.aset("positions", ruby.qnil().as_value())?;
            }
            keywords_array.push(kw_hash)?;
        }
        set_hash_entry(ruby, &hash, "extracted_keywords", keywords_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "extracted_keywords", ruby.qnil().as_value())?;
    }

    // Convert quality score
    if let Some(score) = result.quality_score {
        set_hash_entry(ruby, &hash, "quality_score", ruby.float_from_f64(score).into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "quality_score", ruby.qnil().as_value())?;
    }

    // Convert processing warnings
    let warnings_array = ruby.ary_new();
    for warning in result.processing_warnings {
        let w_hash = ruby.hash_new();
        w_hash.aset("source", warning.source.as_ref())?;
        w_hash.aset("message", warning.message.as_ref())?;
        warnings_array.push(w_hash)?;
    }
    set_hash_entry(ruby, &hash, "processing_warnings", warnings_array.into_value_with(ruby))?;

    // Convert LLM usage
    if let Some(usages) = result.llm_usage {
        let usage_array = ruby.ary_new();
        for usage in usages {
            let usage_hash = ruby.hash_new();
            usage_hash.aset("model", usage.model.as_str())?;
            usage_hash.aset("source", usage.source.as_str())?;
            if let Some(input_tokens) = usage.input_tokens {
                usage_hash.aset("input_tokens", input_tokens as i64)?;
            } else {
                usage_hash.aset("input_tokens", ruby.qnil().as_value())?;
            }
            if let Some(output_tokens) = usage.output_tokens {
                usage_hash.aset("output_tokens", output_tokens as i64)?;
            } else {
                usage_hash.aset("output_tokens", ruby.qnil().as_value())?;
            }
            if let Some(total_tokens) = usage.total_tokens {
                usage_hash.aset("total_tokens", total_tokens as i64)?;
            } else {
                usage_hash.aset("total_tokens", ruby.qnil().as_value())?;
            }
            if let Some(cost) = usage.estimated_cost {
                usage_hash.aset("estimated_cost", ruby.float_from_f64(cost).into_value_with(ruby))?;
            } else {
                usage_hash.aset("estimated_cost", ruby.qnil().as_value())?;
            }
            if let Some(reason) = usage.finish_reason {
                usage_hash.aset("finish_reason", reason.as_str())?;
            } else {
                usage_hash.aset("finish_reason", ruby.qnil().as_value())?;
            }
            usage_array.push(usage_hash)?;
        }
        set_hash_entry(ruby, &hash, "llm_usage", usage_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "llm_usage", ruby.qnil().as_value())?;
    }

    // Convert structured output (Value::Null maps to qnil via json_value_to_ruby)
    let structured_ruby = match &result.structured_output {
        Some(val) => json_value_to_ruby(ruby, val)?,
        None => ruby.qnil().as_value(),
    };
    set_hash_entry(ruby, &hash, "structured_output", structured_ruby)?;

    // Convert annotations
    if let Some(annotations) = result.annotations {
        let annotations_array = ruby.ary_new();
        for annot in annotations {
            let annot_hash = ruby.hash_new();
            let type_str = serde_json::to_value(&annot.annotation_type)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            annot_hash.aset("annotation_type", type_str.as_str())?;
            if let Some(content) = annot.content {
                annot_hash.aset("content", content.as_str())?;
            } else {
                annot_hash.aset("content", ruby.qnil().as_value())?;
            }
            annot_hash.aset("page_number", annot.page_number as i64)?;
            if let Some(bbox) = annot.bounding_box {
                let bbox_hash = ruby.hash_new();
                bbox_hash.aset("x0", bbox.x0)?;
                bbox_hash.aset("y0", bbox.y0)?;
                bbox_hash.aset("x1", bbox.x1)?;
                bbox_hash.aset("y1", bbox.y1)?;
                annot_hash.aset("bounding_box", bbox_hash)?;
            } else {
                annot_hash.aset("bounding_box", ruby.qnil().as_value())?;
            }
            annotations_array.push(annot_hash)?;
        }
        set_hash_entry(ruby, &hash, "annotations", annotations_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "annotations", ruby.qnil().as_value())?;
    }

    // Convert uris
    if let Some(uris) = result.uris {
        let uris_array = ruby.ary_new();
        for uri in uris {
            let uri_hash = ruby.hash_new();
            uri_hash.aset("url", uri.url.as_str())?;
            if let Some(label) = &uri.label {
                uri_hash.aset("label", label.as_str())?;
            } else {
                uri_hash.aset("label", ruby.qnil().as_value())?;
            }
            if let Some(page) = uri.page {
                uri_hash.aset("page", page as i64)?;
            } else {
                uri_hash.aset("page", ruby.qnil().as_value())?;
            }
            let kind_str = serde_json::to_value(&uri.kind)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            uri_hash.aset("kind", kind_str.as_str())?;
            uris_array.push(uri_hash)?;
        }
        set_hash_entry(ruby, &hash, "uris", uris_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "uris", ruby.qnil().as_value())?;
    }

    // Convert children (archive entries)
    if let Some(children) = result.children {
        let children_array = ruby.ary_new();
        for entry in children {
            let entry_hash = ruby.hash_new();
            entry_hash.aset("path", entry.path.as_str())?;
            entry_hash.aset("mime_type", entry.mime_type.as_str())?;
            let nested = extraction_result_to_ruby(ruby, *entry.result)?;
            entry_hash.aset("result", nested.into_value_with(ruby))?;
            children_array.push(entry_hash)?;
        }
        set_hash_entry(ruby, &hash, "children", children_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "children", ruby.qnil().as_value())?;
    }

    Ok(hash)
}
