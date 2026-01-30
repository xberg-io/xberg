use crate::KNOWN_FORMAT_FIELDS;
use crate::config::JsExtractionConfig;
use kreuzberg::{
    Chunk as RustChunk, ChunkMetadata as RustChunkMetadata, ExtractionConfig, ExtractionResult as RustExtractionResult,
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
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
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

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<JsChunk>>,
    #[serde(skip)]
    pub images: Option<Vec<JsExtractedImage>>,
    #[serde(skip)]
    pub pages: Option<Vec<JsPageContent>>,
    pub elements: Option<Vec<JsElement>>,
}

impl TryFrom<RustExtractionResult> for JsExtractionResult {
    type Error = napi::Error;

    fn try_from(val: RustExtractionResult) -> Result<Self> {
        let metadata = serde_json::to_value(&val.metadata)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize metadata: {}", e)))?;

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
                    data: img.data.into(),
                    format: img.format,
                    image_index: img.image_index as u32,
                    page_number: img.page_number.map(|p| p as u32),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(js_images)
        } else {
            None
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
                            data: img.data.clone().into(),
                            format: img.format.clone(),
                            image_index: img.image_index as u32,
                            page_number: img.page_number.map(|p| p as u32),
                            width: img.width,
                            height: img.height,
                            colorspace: img.colorspace.clone(),
                            bits_per_component: img.bits_per_component,
                            is_mask: img.is_mask,
                            description: img.description.clone(),
                            ocr_result,
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
                });
            }
            Some(js_pages)
        } else {
            None
        };

        let elements = val.elements.map(|elems| {
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
        });

        Ok(JsExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| JsTable {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as u32,
                })
                .collect(),
            detected_languages: val.detected_languages,
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
                Some(js_chunks)
            } else {
                None
            },
            images,
            pages,
            elements,
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

            let additional = metadata_map;

            kreuzberg::Metadata {
                language,
                subject,
                format,
                image_preprocessing,
                json_schema,
                error,
                additional,
                ..Default::default()
            }
        };

        let images = if let Some(imgs) = val.images {
            let mut rust_images = Vec::with_capacity(imgs.len());
            for img in imgs {
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
                    data: img.data.to_vec(),
                    format: img.format,
                    image_index: img.image_index as usize,
                    page_number: img.page_number.map(|p| p as usize),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(rust_images)
        } else {
            None
        };

        let chunks = if let Some(chunks) = val.chunks {
            let mut rust_chunks = Vec::with_capacity(chunks.len());
            for chunk in chunks {
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
                    },
                });
            }
            Some(rust_chunks)
        } else {
            None
        };

        Ok(RustExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| kreuzberg::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as usize,
                })
                .collect(),
            detected_languages: val.detected_languages,
            chunks,
            images,
            pages: None,
            elements: val.elements.map(|elems| {
                elems
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
                    .collect()
            }),
            djot_content: None,
        })
    }
}
