use crate::KNOWN_FORMAT_FIELDS;
use crate::config::{JsExtractionConfig, JsFileExtractionConfig};
use kreuzberg::{
    Chunk as RustChunk, ChunkMetadata as RustChunkMetadata, ExtractionConfig, ExtractionResult as RustExtractionResult,
    FileExtractionConfig, ProcessingWarning as RustProcessingWarning, utils::snake_to_camel,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsHierarchicalBlock {
    pub text: String,
    pub font_size: f64,
    pub level: String,
    #[napi(ts_type = "[number, number, number, number] | undefined")]
    pub bbox: Option<Vec<f64>>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsPageHierarchy {
    pub block_count: u32,
    pub blocks: Vec<JsHierarchicalBlock>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsPageContent {
    pub page_number: u32,
    pub content: String,
    #[serde(skip)]
    pub tables: Vec<JsTable>,
    #[serde(skip)]
    pub images: Vec<JsExtractedImage>,
    pub hierarchy: Option<JsPageHierarchy>,
    pub is_blank: Option<bool>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
    pub bounding_box: Option<JsBoundingBox>,
}

#[napi(object)]
pub struct JsExtractedImage {
    pub data: Buffer,
    pub format: String,
    pub image_index: u32,
    pub page_number: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    #[napi(ts_type = "JsExtractionResult | undefined")]
    pub ocr_result: Option<serde_json::Value>,
    pub bounding_box: Option<JsBoundingBox>,
    pub source_path: Option<String>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsHeadingLevel {
    pub level: u32,
    pub text: String,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsHeadingContext {
    pub headings: Vec<JsHeadingLevel>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunkMetadata {
    pub byte_start: u32,
    pub byte_end: u32,
    pub token_count: Option<u32>,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub first_page: Option<u32>,
    pub last_page: Option<u32>,
    pub heading_context: Option<JsHeadingContext>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunk {
    pub content: String,
    #[napi(ts_type = "number[] | undefined")]
    pub embedding: Option<Vec<f64>>,
    pub metadata: JsChunkMetadata,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsBoundingBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractedKeyword {
    pub text: String,
    pub score: f64,
    pub algorithm: String,
    pub positions: Option<Vec<u32>>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsProcessingWarning {
    pub source: String,
    pub message: String,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsPdfAnnotation {
    #[napi(js_name = "annotationType")]
    pub annotation_type: String,
    pub content: Option<String>,
    #[napi(js_name = "pageNumber")]
    pub page_number: u32,
    #[napi(js_name = "boundingBox")]
    pub bounding_box: Option<JsBoundingBox>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsElementMetadata {
    pub page_number: Option<u32>,
    pub filename: Option<String>,
    pub coordinates: Option<JsBoundingBox>,
    pub element_index: Option<u32>,
    #[napi(ts_type = "Record<string, string> | undefined")]
    pub additional: Option<serde_json::Value>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsElement {
    pub element_id: String,
    pub element_type: String,
    pub text: String,
    pub metadata: JsElementMetadata,
}

fn usize_to_u32(value: usize, field: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("{} exceeds supported range (must fit in u32)", field),
        )
    })
}

pub fn resolve_config(config: Option<JsExtractionConfig>) -> Result<ExtractionConfig> {
    match config {
        Some(cfg) => ExtractionConfig::try_from(cfg),
        None => Ok(ExtractionConfig::default()),
    }
}

pub fn resolve_file_config(config: Option<JsFileExtractionConfig>) -> Result<Option<FileExtractionConfig>> {
    match config {
        Some(cfg) => Ok(Some(FileExtractionConfig::try_from(cfg)?)),
        None => Ok(None),
    }
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsUri {
    pub url: String,
    pub label: Option<String>,
    pub page: Option<u32>,
    pub kind: String,
}

#[napi(object)]
pub struct JsArchiveEntry {
    pub path: String,
    #[napi(js_name = "mimeType")]
    pub mime_type: String,
    #[napi(ts_type = "JsExtractionResult")]
    pub result: serde_json::Value,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Vec<String>,
    pub chunks: Vec<JsChunk>,
    #[serde(skip)]
    pub images: Vec<JsExtractedImage>,
    #[serde(skip)]
    pub pages: Vec<JsPageContent>,
    pub elements: Vec<JsElement>,
    #[napi(ts_type = "DocumentStructure | null")]
    pub document: Option<serde_json::Value>,
    #[napi(ts_type = "DjotContent | null", js_name = "djotContent")]
    pub djot_content: Option<serde_json::Value>,
    #[napi(ts_type = "OcrElement[] | null")]
    pub ocr_elements: Option<serde_json::Value>,
    #[napi(js_name = "extractedKeywords")]
    pub extracted_keywords: Vec<JsExtractedKeyword>,
    #[napi(js_name = "qualityScore")]
    pub quality_score: Option<f64>,
    #[napi(js_name = "processingWarnings")]
    pub processing_warnings: Vec<JsProcessingWarning>,
    pub annotations: Vec<JsPdfAnnotation>,
    #[serde(skip)]
    pub children: Vec<JsArchiveEntry>,
    pub uris: Vec<JsUri>,
}

impl TryFrom<RustExtractionResult> for JsExtractionResult {
    type Error = napi::Error;

    fn try_from(val: RustExtractionResult) -> Result<Self> {
        let metadata = serde_json::to_value(&val.metadata)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize metadata: {}", e)))?;

        // Transform snake_case keys from Rust core to camelCase for TypeScript/Node.js
        let metadata = snake_to_camel(metadata);

        let images = if let Some(imgs) = val.images {
            let mut js_images = Vec::with_capacity(imgs.len());
            for img in imgs {
                let ocr_result = if let Some(ocr) = img.ocr_result {
                    Some(JsExtractionResult::try_from(*ocr).and_then(|js_res| {
                        serde_json::to_value(js_res).map_err(|e| {
                            Error::new(
                                Status::GenericFailure,
                                format!("Failed to serialize OCR result metadata: {}", e),
                            )
                        })
                    })?)
                } else {
                    None
                };

                js_images.push(JsExtractedImage {
                    data: img.data.to_vec().into(),
                    format: img.format.into_owned(),
                    image_index: img.image_index as u32,
                    page_number: img.page_number.map(|p| p as u32),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                    bounding_box: img.bounding_box.map(|bb| JsBoundingBox {
                        x0: bb.x0,
                        y0: bb.y0,
                        x1: bb.x1,
                        y1: bb.y1,
                    }),
                    source_path: img.source_path,
                });
            }
            js_images
        } else {
            Vec::new()
        };

        let pages = if let Some(pages_vec) = val.pages {
            let mut js_pages = Vec::with_capacity(pages_vec.len());
            for page in pages_vec {
                let page_tables: Vec<JsTable> = page
                    .tables
                    .iter()
                    .map(|t| JsTable {
                        cells: t.cells.clone(),
                        markdown: t.markdown.clone(),
                        page_number: t.page_number as u32,
                        bounding_box: t.bounding_box.map(|bb| JsBoundingBox {
                            x0: bb.x0,
                            y0: bb.y0,
                            x1: bb.x1,
                            y1: bb.y1,
                        }),
                    })
                    .collect();

                let page_images: Vec<JsExtractedImage> = page
                    .images
                    .iter()
                    .map(|img| {
                        let ocr_result = if let Some(ocr) = &img.ocr_result {
                            JsExtractionResult::try_from((**ocr).clone())
                                .and_then(|js_res| {
                                    serde_json::to_value(js_res).map_err(|e| {
                                        Error::new(
                                            Status::GenericFailure,
                                            format!("Failed to serialize OCR result in page image: {}", e),
                                        )
                                    })
                                })
                                .ok()
                        } else {
                            None
                        };

                        JsExtractedImage {
                            data: img.data.to_vec().into(),
                            format: img.format.to_string(),
                            image_index: img.image_index as u32,
                            page_number: img.page_number.map(|p| p as u32),
                            width: img.width,
                            height: img.height,
                            colorspace: img.colorspace.clone(),
                            bits_per_component: img.bits_per_component,
                            is_mask: img.is_mask,
                            description: img.description.clone(),
                            ocr_result,
                            bounding_box: img.bounding_box.map(|bb| JsBoundingBox {
                                x0: bb.x0,
                                y0: bb.y0,
                                x1: bb.x1,
                                y1: bb.y1,
                            }),
                            source_path: img.source_path.clone(),
                        }
                    })
                    .collect();

                let hierarchy = page.hierarchy.map(|h| {
                    let blocks: Vec<JsHierarchicalBlock> = h
                        .blocks
                        .into_iter()
                        .map(|block| JsHierarchicalBlock {
                            text: block.text,
                            font_size: block.font_size as f64,
                            level: block.level,
                            bbox: block
                                .bbox
                                .map(|(l, t, r, b)| vec![l as f64, t as f64, r as f64, b as f64]),
                        })
                        .collect();

                    JsPageHierarchy {
                        block_count: h.block_count as u32,
                        blocks,
                    }
                });

                js_pages.push(JsPageContent {
                    page_number: page.page_number as u32,
                    content: page.content,
                    tables: page_tables,
                    images: page_images,
                    hierarchy,
                    is_blank: page.is_blank,
                });
            }
            js_pages
        } else {
            Vec::new()
        };

        let elements = val
            .elements
            .map(|elems| {
                elems
                    .into_iter()
                    .map(|e| {
                        let additional = if e.metadata.additional.is_empty() {
                            None
                        } else {
                            serde_json::to_value(&e.metadata.additional).ok()
                        };
                        JsElement {
                            element_id: e.element_id.to_string(),
                            element_type: serde_json::to_value(e.element_type)
                                .ok()
                                .and_then(|v| v.as_str().map(String::from))
                                .unwrap_or_default(),
                            text: e.text,
                            metadata: JsElementMetadata {
                                page_number: e.metadata.page_number.map(|p| p as u32),
                                filename: e.metadata.filename,
                                coordinates: e.metadata.coordinates.map(|c| JsBoundingBox {
                                    x0: c.x0,
                                    y0: c.y0,
                                    x1: c.x1,
                                    y1: c.y1,
                                }),
                                element_index: e.metadata.element_index.map(|i| i as u32),
                                additional,
                            },
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let document = val
            .document
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to serialize document structure: {}", e),
                )
            })?;

        let djot_content = val
            .djot_content
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to serialize djot_content: {}", e),
                )
            })?;

        let ocr_elements = val
            .ocr_elements
            .map(|elems| serde_json::to_value(&elems))
            .transpose()
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to serialize ocr_elements: {}", e),
                )
            })?;

        let extracted_keywords = val
            .extracted_keywords
            .map(|keywords| {
                keywords
                    .into_iter()
                    .map(|kw| JsExtractedKeyword {
                        text: kw.text,
                        score: kw.score as f64,
                        algorithm: serde_json::to_value(kw.algorithm)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_default(),
                        positions: kw.positions.map(|p| p.into_iter().map(|v| v as u32).collect()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let processing_warnings = val
            .processing_warnings
            .into_iter()
            .map(|w| JsProcessingWarning {
                source: w.source.into_owned(),
                message: w.message.into_owned(),
            })
            .collect();

        let annotations = val
            .annotations
            .map(|annots| {
                annots
                    .into_iter()
                    .map(|a| {
                        let type_str = serde_json::to_value(a.annotation_type)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_default();
                        JsPdfAnnotation {
                            annotation_type: type_str,
                            content: a.content,
                            page_number: a.page_number as u32,
                            bounding_box: a.bounding_box.map(|bb| JsBoundingBox {
                                x0: bb.x0,
                                y0: bb.y0,
                                x1: bb.x1,
                                y1: bb.y1,
                            }),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let children = val
            .children
            .map(|entries| {
                entries
                    .into_iter()
                    .filter_map(|entry| {
                        let result = JsExtractionResult::try_from(*entry.result).ok()?;
                        let result_json = serde_json::to_value(result).ok()?;
                        Some(JsArchiveEntry {
                            path: entry.path,
                            mime_type: entry.mime_type,
                            result: result_json,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let uris = val
            .uris
            .map(|uri_vec| {
                uri_vec
                    .into_iter()
                    .map(|u| {
                        let kind_str = serde_json::to_value(u.kind)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_default();
                        JsUri {
                            url: u.url,
                            label: u.label,
                            page: u.page,
                            kind: kind_str,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(JsExtractionResult {
            content: val.content,
            mime_type: val.mime_type.to_string(),
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| JsTable {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as u32,
                    bounding_box: t.bounding_box.map(|bb| JsBoundingBox {
                        x0: bb.x0,
                        y0: bb.y0,
                        x1: bb.x1,
                        y1: bb.y1,
                    }),
                })
                .collect(),
            detected_languages: val.detected_languages.unwrap_or_default(),
            chunks: if let Some(chunks) = val.chunks {
                let mut js_chunks = Vec::with_capacity(chunks.len());
                for chunk in chunks {
                    let metadata = JsChunkMetadata {
                        byte_start: usize_to_u32(chunk.metadata.byte_start, "chunks[].metadata.byte_start")?,
                        byte_end: usize_to_u32(chunk.metadata.byte_end, "chunks[].metadata.byte_end")?,
                        token_count: match chunk.metadata.token_count {
                            Some(tokens) => Some(usize_to_u32(tokens, "chunks[].metadata.token_count")?),
                            None => None,
                        },
                        chunk_index: usize_to_u32(chunk.metadata.chunk_index, "chunks[].metadata.chunk_index")?,
                        total_chunks: usize_to_u32(chunk.metadata.total_chunks, "chunks[].metadata.total_chunks")?,
                        first_page: chunk.metadata.first_page.map(|p| p as u32),
                        last_page: chunk.metadata.last_page.map(|p| p as u32),
                        heading_context: chunk.metadata.heading_context.map(|hc| JsHeadingContext {
                            headings: hc
                                .headings
                                .into_iter()
                                .map(|h| JsHeadingLevel {
                                    level: u32::from(h.level),
                                    text: h.text,
                                })
                                .collect(),
                        }),
                    };

                    let embedding = chunk
                        .embedding
                        .map(|values| values.into_iter().map(f64::from).collect());

                    js_chunks.push(JsChunk {
                        content: chunk.content,
                        embedding,
                        metadata,
                    });
                }
                js_chunks
            } else {
                Vec::new()
            },
            images,
            pages,
            elements,
            document,
            djot_content,
            ocr_elements,
            extracted_keywords,
            quality_score: val.quality_score,
            processing_warnings,
            annotations,
            children,
            uris,
        })
    }
}

impl TryFrom<JsExtractionResult> for RustExtractionResult {
    type Error = napi::Error;

    fn try_from(val: JsExtractionResult) -> Result<Self> {
        let metadata = {
            let mut metadata_map: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_value(val.metadata.clone()).map_err(|e| {
                    Error::new(
                        Status::GenericFailure,
                        format!("Failed to parse metadata as map: {}", e),
                    )
                })?;

            let language = metadata_map
                .remove("language")
                .and_then(|v| serde_json::from_value(v).ok());
            let subject = metadata_map
                .remove("subject")
                .and_then(|v| serde_json::from_value(v).ok());
            let image_preprocessing = metadata_map
                .remove("image_preprocessing")
                .and_then(|v| serde_json::from_value(v).ok());
            let json_schema = metadata_map.remove("json_schema");
            let error = metadata_map
                .remove("error")
                .and_then(|v| serde_json::from_value(v).ok());
            let category = metadata_map
                .remove("category")
                .and_then(|v| serde_json::from_value(v).ok());
            let tags = metadata_map.remove("tags").and_then(|v| serde_json::from_value(v).ok());
            let document_version = metadata_map
                .remove("document_version")
                .and_then(|v| serde_json::from_value(v).ok());
            let abstract_text = metadata_map
                .remove("abstract_text")
                .and_then(|v| serde_json::from_value(v).ok());
            let output_format = metadata_map
                .remove("output_format")
                .and_then(|v| serde_json::from_value(v).ok());

            let mut format_fields = serde_json::Map::new();
            for key in KNOWN_FORMAT_FIELDS.iter() {
                if let Some(value) = metadata_map.remove(*key) {
                    format_fields.insert(key.to_string(), value);
                }
            }

            let format = if !format_fields.is_empty() {
                serde_json::from_value(serde_json::Value::Object(format_fields)).ok()
            } else {
                None
            };

            let additional = metadata_map
                .into_iter()
                .map(|(k, v)| (std::borrow::Cow::Owned(k), v))
                .collect();

            kreuzberg::Metadata {
                language,
                subject,
                format,
                image_preprocessing,
                json_schema,
                error,
                category,
                tags,
                document_version,
                abstract_text,
                output_format,
                additional,
                ..Default::default()
            }
        };

        let images = if val.images.is_empty() {
            None
        } else {
            let mut rust_images = Vec::with_capacity(val.images.len());
            for img in val.images {
                let ocr_result = if let Some(json) = img.ocr_result {
                    Some(Box::new(
                        serde_json::from_value::<JsExtractionResult>(json)
                            .map_err(|e| {
                                Error::new(
                                    Status::GenericFailure,
                                    format!("Failed to deserialize OCR result: {}", e),
                                )
                            })
                            .and_then(RustExtractionResult::try_from)?,
                    ))
                } else {
                    None
                };

                rust_images.push(kreuzberg::ExtractedImage {
                    data: bytes::Bytes::from(img.data.to_vec()),
                    format: std::borrow::Cow::Owned(img.format),
                    image_index: img.image_index as usize,
                    page_number: img.page_number.map(|p| p as usize),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                    bounding_box: img.bounding_box.map(|bb| kreuzberg::types::BoundingBox {
                        x0: bb.x0,
                        y0: bb.y0,
                        x1: bb.x1,
                        y1: bb.y1,
                    }),
                    source_path: img.source_path,
                });
            }
            Some(rust_images)
        };

        let chunks = if val.chunks.is_empty() {
            None
        } else {
            let mut rust_chunks = Vec::with_capacity(val.chunks.len());
            for chunk in val.chunks {
                let embedding = if let Some(values) = chunk.embedding {
                    let mut normalized = Vec::with_capacity(values.len());
                    for (idx, value) in values.into_iter().enumerate() {
                        if !value.is_finite() {
                            return Err(Error::new(
                                Status::InvalidArg,
                                format!("chunks[].embedding[{}] must be a finite number", idx),
                            ));
                        }
                        if value > f32::MAX as f64 || value < -(f32::MAX as f64) {
                            return Err(Error::new(
                                Status::InvalidArg,
                                format!("chunks[].embedding[{}] value {} exceeds f32 range", idx, value),
                            ));
                        }
                        normalized.push(value as f32);
                    }
                    Some(normalized)
                } else {
                    None
                };

                rust_chunks.push(RustChunk {
                    content: chunk.content,
                    embedding,
                    metadata: RustChunkMetadata {
                        byte_start: chunk.metadata.byte_start as usize,
                        byte_end: chunk.metadata.byte_end as usize,
                        token_count: chunk.metadata.token_count.map(|v| v as usize),
                        chunk_index: chunk.metadata.chunk_index as usize,
                        total_chunks: chunk.metadata.total_chunks as usize,
                        first_page: chunk.metadata.first_page.map(|v| v as usize),
                        last_page: chunk.metadata.last_page.map(|v| v as usize),
                        heading_context: chunk.metadata.heading_context.map(|hc| kreuzberg::HeadingContext {
                            headings: hc
                                .headings
                                .into_iter()
                                .map(|h| kreuzberg::HeadingLevel {
                                    level: h.level as u8,
                                    text: h.text,
                                })
                                .collect(),
                        }),
                    },
                });
            }
            Some(rust_chunks)
        };

        let document = val.document.and_then(|v| serde_json::from_value(v).ok());

        Ok(RustExtractionResult {
            content: val.content,
            mime_type: std::borrow::Cow::Owned(val.mime_type),
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| kreuzberg::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as usize,
                    bounding_box: t.bounding_box.map(|bb| kreuzberg::types::BoundingBox {
                        x0: bb.x0,
                        y0: bb.y0,
                        x1: bb.x1,
                        y1: bb.y1,
                    }),
                })
                .collect(),
            detected_languages: if val.detected_languages.is_empty() {
                None
            } else {
                Some(val.detected_languages)
            },
            chunks,
            images,
            pages: None,
            elements: if val.elements.is_empty() {
                None
            } else {
                Some(
                    val.elements
                        .into_iter()
                        .filter_map(|e| {
                            let element_id = kreuzberg::types::ElementId::new(e.element_id).ok()?;
                            let element_type: kreuzberg::types::ElementType =
                                serde_json::from_value(serde_json::Value::String(e.element_type)).ok()?;
                            let additional = e
                                .metadata
                                .additional
                                .and_then(|v| serde_json::from_value(v).ok())
                                .unwrap_or_default();
                            Some(kreuzberg::types::Element {
                                element_id,
                                element_type,
                                text: e.text,
                                metadata: kreuzberg::types::ElementMetadata {
                                    page_number: e.metadata.page_number.map(|p| p as usize),
                                    filename: e.metadata.filename,
                                    coordinates: e.metadata.coordinates.map(|c| kreuzberg::types::BoundingBox {
                                        x0: c.x0,
                                        y0: c.y0,
                                        x1: c.x1,
                                        y1: c.y1,
                                    }),
                                    element_index: e.metadata.element_index.map(|i| i as usize),
                                    additional,
                                },
                            })
                        })
                        .collect(),
                )
            },
            document,
            djot_content: val.djot_content.and_then(|v| serde_json::from_value(v).ok()),
            ocr_elements: val.ocr_elements.and_then(|v| serde_json::from_value(v).ok()),
            extracted_keywords: if val.extracted_keywords.is_empty() {
                None
            } else {
                Some(
                    val.extracted_keywords
                        .into_iter()
                        .filter_map(|kw| {
                            let algorithm: kreuzberg::keywords::KeywordAlgorithm =
                                serde_json::from_value(serde_json::Value::String(kw.algorithm)).ok()?;
                            Some(kreuzberg::keywords::Keyword {
                                text: kw.text,
                                score: kw.score as f32,
                                algorithm,
                                positions: kw.positions.map(|p| p.into_iter().map(|v| v as usize).collect()),
                            })
                        })
                        .collect(),
                )
            },
            quality_score: val.quality_score,
            processing_warnings: val
                .processing_warnings
                .into_iter()
                .map(|w| RustProcessingWarning {
                    source: std::borrow::Cow::Owned(w.source),
                    message: std::borrow::Cow::Owned(w.message),
                })
                .collect(),
            annotations: if val.annotations.is_empty() {
                None
            } else {
                Some(
                    val.annotations
                        .into_iter()
                        .filter_map(|a| {
                            let annotation_type: kreuzberg::types::PdfAnnotationType =
                                serde_json::from_value(serde_json::Value::String(a.annotation_type)).ok()?;
                            Some(kreuzberg::types::PdfAnnotation {
                                annotation_type,
                                content: a.content,
                                page_number: a.page_number as usize,
                                bounding_box: a.bounding_box.map(|bb| kreuzberg::types::BoundingBox {
                                    x0: bb.x0,
                                    y0: bb.y0,
                                    x1: bb.x1,
                                    y1: bb.y1,
                                }),
                            })
                        })
                        .collect(),
                )
            },
            children: if val.children.is_empty() {
                None
            } else {
                Some(
                    val.children
                        .into_iter()
                        .filter_map(|entry| {
                            let js_result: JsExtractionResult = serde_json::from_value(entry.result).ok()?;
                            let result = RustExtractionResult::try_from(js_result).ok()?;
                            Some(kreuzberg::ArchiveEntry {
                                path: entry.path,
                                mime_type: entry.mime_type,
                                result: Box::new(result),
                            })
                        })
                        .collect(),
                )
            },
            uris: if val.uris.is_empty() {
                None
            } else {
                Some(
                    val.uris
                        .into_iter()
                        .filter_map(|u| {
                            let kind: kreuzberg::UriKind =
                                serde_json::from_value(serde_json::Value::String(u.kind)).ok()?;
                            Some(kreuzberg::Uri {
                                url: u.url,
                                label: u.label,
                                page: u.page,
                                kind,
                            })
                        })
                        .collect(),
                )
            },
            formatted_content: None,
        })
    }
}
