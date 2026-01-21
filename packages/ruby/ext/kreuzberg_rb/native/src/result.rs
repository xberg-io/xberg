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

    let mime_value = ruby.str_new(result.mime_type.as_str()).into_value_with(ruby);
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
            image_hash.aset("format", image.format)?;
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

                tables_array.push(table_hash)?;
            }
            page_hash.aset("tables", tables_array)?;

            let images_array = ruby.ary_new();
            for image in page_content.images {
                let image_hash = ruby.hash_new();
                let data_value = ruby.str_from_slice(&image.data).into_value_with(ruby);
                image_hash.aset("data", data_value)?;
                image_hash.aset("format", image.format.clone())?;
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

    Ok(hash)
}
