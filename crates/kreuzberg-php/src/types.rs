//! Result type bindings
//!
//! Provides PHP-friendly wrappers around extraction result types.

use ext_php_rs::convert::IntoZval;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// PHP Enums (backed enums via #[php_enum])
// ---------------------------------------------------------------------------

/// Content layer classification.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\ContentLayer")]
pub enum ContentLayer {
    #[php(name = "body")]
    Body,
    #[php(name = "header")]
    Header,
    #[php(name = "footer")]
    Footer,
    #[php(name = "footnote")]
    Footnote,
}

/// Element type classification for structured extraction.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\ElementType")]
pub enum ElementType {
    #[php(name = "title")]
    Title,
    #[php(name = "narrativeText")]
    NarrativeText,
    #[php(name = "heading")]
    Heading,
    #[php(name = "listItem")]
    ListItem,
    #[php(name = "table")]
    Table,
    #[php(name = "image")]
    Image,
    #[php(name = "pageBreak")]
    PageBreak,
    #[php(name = "codeBlock")]
    CodeBlock,
    #[php(name = "blockQuote")]
    BlockQuote,
    #[php(name = "footer")]
    Footer,
    #[php(name = "header")]
    Header,
}

/// Keyword extraction algorithm.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\KeywordAlgorithm")]
pub enum KeywordAlgorithm {
    #[php(name = "yake")]
    Yake,
    #[php(name = "rake")]
    Rake,
}

/// OCR element granularity level.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\OcrElementLevel")]
pub enum OcrElementLevel {
    #[php(name = "word")]
    Word,
    #[php(name = "line")]
    Line,
    #[php(name = "block")]
    Block,
    #[php(name = "page")]
    Page,
}

/// Output format for extraction results.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\OutputFormat")]
pub enum OutputFormat {
    #[php(name = "plain")]
    Plain,
    #[php(name = "markdown")]
    Markdown,
    #[php(name = "djot")]
    Djot,
    #[php(name = "html")]
    Html,
    #[php(name = "json")]
    Json,
    #[php(name = "structured")]
    Structured,
}

/// Page unit type classification.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\PageUnitType")]
pub enum PageUnitType {
    #[php(name = "page")]
    Page,
    #[php(name = "slide")]
    Slide,
    #[php(name = "sheet")]
    Sheet,
}

/// Relationship kind between document elements.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\RelationshipKind")]
pub enum RelationshipKind {
    #[php(name = "footnoteReference")]
    FootnoteReference,
    #[php(name = "citationReference")]
    CitationReference,
    #[php(name = "internalLink")]
    InternalLink,
    #[php(name = "caption")]
    Caption,
    #[php(name = "label")]
    Label,
    #[php(name = "tocEntry")]
    TocEntry,
    #[php(name = "crossReference")]
    CrossReference,
}

/// Result format for extraction output.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\ResultFormat")]
pub enum ResultFormat {
    #[php(name = "unified")]
    Unified,
    #[php(name = "elementBased")]
    ElementBased,
}

/// URI kind classification.
#[php_enum]
#[php(name = "Kreuzberg\\Types\\UriKind")]
pub enum UriKind {
    #[php(name = "hyperlink")]
    Hyperlink,
    #[php(name = "image")]
    Image,
    #[php(name = "anchor")]
    Anchor,
    #[php(name = "citation")]
    Citation,
    #[php(name = "reference")]
    Reference,
    #[php(name = "email")]
    Email,
}

// ---------------------------------------------------------------------------
// Additional struct types required by parity tests
// ---------------------------------------------------------------------------

/// Archive entry containing extraction result for a file within an archive.
#[php_class]
#[php(name = "Kreuzberg\\Types\\ArchiveEntry")]
#[derive(Clone)]
pub struct ArchiveEntry {
    /// MIME type of the archived file
    #[php(prop)]
    #[php(name = "mimeType")]
    pub mime_type: String,

    /// Path within the archive
    #[php(prop)]
    pub path: String,

    /// Extraction result for this entry (accessed via getter property)
    pub result: ExtractionResult,
}

#[php_impl]
impl ArchiveEntry {
    /// Result property getter.
    #[php(getter)]
    pub fn get_result(&self) -> ExtractionResult {
        self.result.clone()
    }
}

/// Axis-aligned bounding box with float coordinates.
#[php_class]
#[php(name = "Kreuzberg\\Types\\BoundingBox")]
#[derive(Clone)]
pub struct BoundingBoxType {
    /// Left x coordinate
    #[php(prop)]
    pub x0: f64,

    /// Right x coordinate
    #[php(prop)]
    pub x1: f64,

    /// Top y coordinate
    #[php(prop)]
    pub y0: f64,

    /// Bottom y coordinate
    #[php(prop)]
    pub y1: f64,
}

/// URI/link discovered during extraction.
#[php_class]
#[php(name = "Kreuzberg\\Types\\Uri")]
#[derive(Clone)]
pub struct UriType {
    /// The kind of URI (e.g., hyperlink, image, anchor)
    #[php(prop)]
    pub kind: String,

    /// Optional label text for the URI
    #[php(prop)]
    pub label: Option<String>,

    /// Optional page number reference
    #[php(prop)]
    pub page: Option<i64>,

    /// The URL string
    #[php(prop)]
    pub url: String,
}

/// Processing warning from extraction.
#[php_class]
#[php(name = "Kreuzberg\\Types\\ProcessingWarning")]
#[derive(Clone)]
pub struct ProcessingWarning {
    /// Warning message
    #[php(prop)]
    pub message: String,

    /// Source of the warning
    #[php(prop)]
    pub source: String,
}

/// Extraction configuration (parity type).
///
/// This is a native Rust-side class under `Kreuzberg\Types\` namespace
/// that mirrors the PHP-side `Kreuzberg\Config\ExtractionConfig` for
/// parity test compatibility.
#[php_class]
#[php(name = "Kreuzberg\\Types\\ExtractionConfig")]
#[derive(Clone)]
pub struct ExtractionConfigType {
    #[php(prop)]
    pub acceleration: Option<String>,
    #[php(prop)]
    #[php(name = "cacheNamespace")]
    pub cache_namespace: Option<String>,
    #[php(prop)]
    #[php(name = "cacheTtlSecs")]
    pub cache_ttl_secs: Option<i64>,
    #[php(prop)]
    pub chunking: Option<String>,
    #[php(prop)]
    pub concurrency: Option<String>,
    #[php(prop)]
    #[php(name = "disableOcr")]
    pub disable_ocr: bool,
    #[php(prop)]
    pub email: Option<String>,
    #[php(prop)]
    #[php(name = "enableQualityProcessing")]
    pub enable_quality_processing: bool,
    #[php(prop)]
    #[php(name = "extractionTimeoutSecs")]
    pub extraction_timeout_secs: Option<i64>,
    #[php(prop)]
    #[php(name = "forceOcr")]
    pub force_ocr: bool,
    #[php(prop)]
    #[php(name = "forceOcrPages")]
    pub force_ocr_pages: Option<Vec<i64>>,
    #[php(prop)]
    #[php(name = "htmlOptions")]
    pub html_options: Option<String>,
    #[php(prop)]
    pub images: Option<String>,
    #[php(prop)]
    #[php(name = "includeDocumentStructure")]
    pub include_document_structure: bool,
    #[php(prop)]
    pub keywords: Option<String>,
    #[php(prop)]
    #[php(name = "languageDetection")]
    pub language_detection: Option<String>,
    #[php(prop)]
    pub layout: Option<String>,
    #[php(prop)]
    #[php(name = "maxArchiveDepth")]
    pub max_archive_depth: Option<i64>,
    #[php(prop)]
    #[php(name = "maxConcurrentExtractions")]
    pub max_concurrent_extractions: Option<i64>,
    #[php(prop)]
    pub ocr: Option<String>,
    #[php(prop)]
    #[php(name = "outputFormat")]
    pub output_format: Option<String>,
    #[php(prop)]
    pub pages: Option<String>,
    #[php(prop)]
    #[php(name = "pdfOptions")]
    pub pdf_options: Option<String>,
    #[php(prop)]
    pub postprocessor: Option<String>,
    #[php(prop)]
    #[php(name = "resultFormat")]
    pub result_format: Option<String>,
    #[php(prop)]
    #[php(name = "securityLimits")]
    pub security_limits: Option<String>,
    #[php(prop)]
    #[php(name = "tokenReduction")]
    pub token_reduction: Option<String>,
    #[php(prop)]
    #[php(name = "treeSitter")]
    pub tree_sitter: Option<String>,
    #[php(prop)]
    #[php(name = "useCache")]
    pub use_cache: bool,
}

/// Table structure for parity (separate from ExtractedTable to match canonical naming).
#[php_class]
#[php(name = "Kreuzberg\\Types\\Table")]
#[derive(Clone)]
pub struct TableType {
    /// Table cells as nested arrays
    #[php(prop)]
    pub cells: Vec<Vec<String>>,

    /// Markdown representation
    #[php(prop)]
    pub markdown: String,

    /// Page number where table was found
    #[php(prop)]
    #[php(name = "pageNumber")]
    pub page_number: usize,

    /// Bounding box
    #[php(prop)]
    #[php(name = "boundingBox")]
    pub bounding_box: Option<HashMap<String, f64>>,
}

/// Convert serde_json::Value to PHP Zval (array/string/number/bool/null).
///
/// Recursively converts JSON structures to PHP arrays, preserving nested objects.
pub(crate) fn json_value_to_php(value: &serde_json::Value) -> PhpResult<Zval> {
    match value {
        serde_json::Value::Null => Ok(Zval::new()),
        serde_json::Value::Bool(b) => Ok(b.into_zval(false)?),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_zval(false)?)
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_zval(false)?)
            } else {
                Err("Invalid number in JSON".into())
            }
        }
        serde_json::Value::String(s) => Ok(s.as_str().into_zval(false)?),
        serde_json::Value::Array(arr) => {
            let mut php_arr = Vec::new();
            for item in arr {
                php_arr.push(json_value_to_php(item)?);
            }
            Ok(php_arr.into_zval(false)?)
        }
        serde_json::Value::Object(obj) => {
            // Create a proper PHP object (stdClass) for JSON objects
            use ext_php_rs::boxed::ZBox;
            use ext_php_rs::types::ZendObject;

            let mut std_obj = ZendObject::new_stdclass();

            // Set properties on the stdClass object
            for (k, v) in obj {
                std_obj.set_property(k.as_str(), json_value_to_php(v)?)?;
            }

            Ok(std_obj.into_zval(false)?)
        }
    }
}

/// Extraction result containing content, metadata, and tables.
///
/// This is the primary return type for all extraction operations.
///
/// # Properties
///
/// - `content` (string): Extracted text content
/// - `mime_type` (string): MIME type of the extracted document
/// - `metadata` (array): Document metadata as key-value pairs
/// - `tables` (array): Array of ExtractedTable objects
/// - `detected_languages` (array|null): Detected languages with confidence scores
/// - `images` (array|null): Extracted images with their data
/// - `chunks` (array|null): Text chunks with optional embeddings
/// - `pages` (array|null): Per-page extraction results
/// - `annotations` (array|null): PDF annotations (text, highlight, link, etc.)
///
/// # Example
///
/// ```php
/// $result = kreuzberg_extract_file("document.pdf");
/// echo $result->content;
/// print_r($result->metadata);
/// foreach ($result->tables as $table) {
///     echo $table->markdown;
/// }
/// ```
#[php_class]
#[php(name = "Kreuzberg\\Types\\ExtractionResult")]
#[derive(Clone)]
pub struct ExtractionResult {
    /// Extracted text content
    #[php(prop)]
    pub content: String,

    /// MIME type of the document
    #[php(prop)]
    #[php(name = "mimeType")]
    pub mime_type: String,

    /// Document metadata (stored as serialized JSON, accessed via __get getter)
    metadata_json: String,

    /// Extracted tables (accessed via getter property)
    pub tables: Vec<ExtractedTable>,

    /// Detected languages
    #[php(prop)]
    #[php(name = "detectedLanguages")]
    pub detected_languages: Option<Vec<String>>,

    /// Extracted images (accessed via getter property)
    pub images: Option<Vec<ExtractedImage>>,

    /// Text chunks (accessed via getter property)
    pub chunks: Option<Vec<TextChunk>>,

    /// Per-page results (accessed via getter property)
    pub pages: Option<Vec<PageResult>>,

    /// Extracted keywords
    pub keywords: Option<Vec<Keyword>>,

    /// Extracted keywords with algorithm metadata (accessed via getter property)
    pub extracted_keywords: Option<Vec<Keyword>>,

    /// Quality score
    #[php(prop)]
    #[php(name = "qualityScore")]
    pub quality_score: Option<f64>,

    /// Structured Djot content (when output_format='djot')
    /// Deserialized via __get magic method
    djot_content_json: Option<String>,

    /// Semantic elements (when output_format='element_based')
    /// Deserialized via __get magic method
    elements_json: Option<String>,

    /// Document structure (when include_document_structure=true)
    /// Deserialized via __get magic method
    document_json: Option<String>,

    /// OCR elements with spatial/confidence metadata
    /// Deserialized via __get magic method
    ocr_elements_json: Option<String>,

    /// PDF annotations (accessed via getter property)
    pub annotations: Option<Vec<PdfAnnotation>>,

    /// Nested extraction results from archive contents
    /// Deserialized via __get magic method
    children_json: Option<String>,

    /// URIs/links discovered during extraction
    /// Deserialized via __get magic method
    uris_json: Option<String>,

    /// Processing warnings (accessed via getter property)
    pub processing_warnings: Vec<ProcessingWarning>,

    /// Full serialized JSON of the original ExtractionResult (for serialize_to_toon/json)
    pub(crate) result_json: String,
}

#[php_impl]
#[allow(non_snake_case)]
impl ExtractionResult {
    // -----------------------------------------------------------------------
    // Getter-backed properties (creates PHP properties visible to reflection)
    // -----------------------------------------------------------------------

    /// Tables property getter.
    #[php(getter)]
    pub fn get_tables(&self) -> Vec<ExtractedTable> {
        self.tables.clone()
    }

    /// Images property getter.
    #[php(getter)]
    pub fn get_images(&self) -> Option<Vec<ExtractedImage>> {
        self.images.clone()
    }

    /// Chunks property getter.
    #[php(getter)]
    pub fn get_chunks(&self) -> Option<Vec<TextChunk>> {
        self.chunks.clone()
    }

    /// Pages property getter.
    #[php(getter)]
    pub fn get_pages(&self) -> Option<Vec<PageResult>> {
        self.pages.clone()
    }

    /// Extracted keywords property getter.
    #[php(getter)]
    pub fn get_extractedKeywords(&self) -> Option<Vec<Keyword>> {
        self.extracted_keywords.clone()
    }

    /// Annotations property getter.
    #[php(getter)]
    pub fn get_annotations(&self) -> Option<Vec<PdfAnnotation>> {
        self.annotations.clone()
    }

    /// Processing warnings property getter.
    #[php(getter)]
    pub fn get_processingWarnings(&self) -> Vec<ProcessingWarning> {
        self.processing_warnings.clone()
    }

    // -----------------------------------------------------------------------
    // Regular methods
    // -----------------------------------------------------------------------

    /// Get metadata as a Metadata object.
    ///
    /// Returns a Metadata object with common fields accessible as properties.
    pub fn get_metadata(&self) -> PhpResult<Metadata> {
        Metadata::from_json(&self.metadata_json)
    }

    /// Magic getter for accessing properties that are not directly exposed.
    ///
    /// Allows access like $result->metadata, $result->chunks, $result->images, $result->pages, $result->tables.
    pub fn __get(&self, name: &str) -> PhpResult<Option<Zval>> {
        match name {
            "metadata" => {
                let metadata = self.get_metadata()?;
                Ok(Some(metadata.into_zval(false)?))
            }
            "chunks" => {
                if let Some(chunks) = &self.chunks {
                    Ok(Some(chunks.clone().into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "embeddings" => {
                // Extract embeddings from chunks if available
                if let Some(chunks) = &self.chunks {
                    use ext_php_rs::boxed::ZBox;
                    use ext_php_rs::types::ZendObject;

                    let mut php_embeddings = Vec::new();
                    for chunk in chunks {
                        if let Some(embedding) = &chunk.embedding {
                            // Create a proper PHP object (stdClass) with vector property
                            let mut embedding_obj = ZendObject::new_stdclass();
                            embedding_obj.set_property("vector", embedding.clone().into_zval(false)?)?;

                            php_embeddings.push(embedding_obj.into_zval(false)?);
                        }
                    }

                    if !php_embeddings.is_empty() {
                        Ok(Some(php_embeddings.into_zval(false)?))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            "images" => {
                if let Some(images) = &self.images {
                    Ok(Some(images.clone().into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "pages" => {
                if let Some(pages) = &self.pages {
                    Ok(Some(pages.clone().into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "keywords" => {
                if let Some(keywords) = &self.keywords {
                    use ext_php_rs::boxed::ZBox;
                    use ext_php_rs::types::ZendObject;

                    // Convert keywords to PHP array of objects (stdClass)
                    let mut php_keywords = Vec::new();
                    for kw in keywords {
                        // Create a proper PHP object with text and score properties
                        let mut kw_obj = ZendObject::new_stdclass();
                        kw_obj.set_property("text", kw.text.as_str().into_zval(false)?)?;
                        kw_obj.set_property("score", kw.score.into_zval(false)?)?;

                        php_keywords.push(kw_obj.into_zval(false)?);
                    }
                    Ok(Some(php_keywords.into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "tables" => Ok(Some(self.tables.clone().into_zval(false)?)),
            "djot_content" | "djotContent" => {
                if let Some(json) = &self.djot_content_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse djot_content: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            "elements" => {
                if let Some(json) = &self.elements_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse elements: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            "document" => {
                if let Some(json) = &self.document_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse document: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            "ocrElements" | "ocr_elements" => {
                if let Some(json) = &self.ocr_elements_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse ocr_elements: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            "extractedKeywords" | "extracted_keywords" => {
                if let Some(keywords) = &self.extracted_keywords {
                    use ext_php_rs::boxed::ZBox;
                    use ext_php_rs::types::ZendObject;

                    // Convert keywords to PHP array of objects (stdClass)
                    let mut php_keywords = Vec::new();
                    for kw in keywords {
                        // Create a proper PHP object with text, score, and algorithm properties
                        let mut kw_obj = ZendObject::new_stdclass();
                        kw_obj.set_property("text", kw.text.as_str().into_zval(false)?)?;
                        kw_obj.set_property("score", kw.score.into_zval(false)?)?;
                        if let Some(algo) = &kw.algorithm {
                            kw_obj.set_property("algorithm", algo.as_str().into_zval(false)?)?;
                        }
                        php_keywords.push(kw_obj.into_zval(false)?);
                    }
                    Ok(Some(php_keywords.into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "qualityScore" | "quality_score" => {
                if let Some(score) = self.quality_score {
                    Ok(Some(score.into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "annotations" => {
                if let Some(annotations) = &self.annotations {
                    Ok(Some(annotations.clone().into_zval(false)?))
                } else {
                    Ok(None)
                }
            }
            "children" => {
                if let Some(json) = &self.children_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse children: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            "uris" => {
                if let Some(json) = &self.uris_json {
                    let value: serde_json::Value =
                        serde_json::from_str(json).map_err(|e| format!("Failed to parse uris: {}", e))?;
                    Ok(Some(json_value_to_php(&value)?))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Get the total number of pages in the document.
    pub fn get_page_count(&self) -> usize {
        self.pages.as_ref().map(|p| p.len()).unwrap_or(0)
    }

    /// Get the total number of chunks in the document.
    pub fn get_chunk_count(&self) -> usize {
        self.chunks.as_ref().map(|c| c.len()).unwrap_or(0)
    }

    /// Get the primary detected language.
    pub fn get_detected_language(&self) -> Option<String> {
        self.detected_languages
            .as_ref()
            .and_then(|langs| langs.first().cloned())
    }

    /// Get all extracted keywords.
    pub fn get_keywords(&self) -> Option<Vec<Keyword>> {
        self.keywords.clone()
    }

    /// Get all embeddings extracted from chunks.
    ///
    /// Extracts embeddings from chunks if available.
    pub fn get_embeddings(&self) -> Option<Vec<Vec<f32>>> {
        self.chunks
            .as_ref()
            .map(|chunks| chunks.iter().filter_map(|chunk| chunk.embedding.clone()).collect())
    }
}

impl ExtractionResult {
    /// Convert from Rust ExtractionResult to PHP ExtractionResult.
    pub fn from_rust(result: kreuzberg::ExtractionResult) -> PhpResult<Self> {
        Self::from_rust_with_config(result, true)
    }

    /// Convert from Rust ExtractionResult to PHP ExtractionResult with optional table filtering.
    pub fn from_rust_with_config(result: kreuzberg::ExtractionResult, extract_tables: bool) -> PhpResult<Self> {
        use serde_json::json;

        // Serialize the full result to JSON before destructuring for serialize_to_toon/json
        let result_json = serde_json::to_string(&result).map_err(|e| format!("Failed to serialize result: {}", e))?;

        let mut metadata_obj = serde_json::Map::new();

        // Add common metadata fields
        if let Some(title) = &result.metadata.title {
            metadata_obj.insert("title".to_string(), json!(title));
        }
        if let Some(subject) = &result.metadata.subject {
            metadata_obj.insert("subject".to_string(), json!(subject));
        }
        if let Some(authors) = &result.metadata.authors {
            metadata_obj.insert("authors".to_string(), json!(authors));
        }
        if let Some(keywords) = &result.metadata.keywords {
            metadata_obj.insert("keywords".to_string(), json!(keywords));
        }
        if let Some(language) = &result.metadata.language {
            metadata_obj.insert("language".to_string(), json!(language));
        }
        if let Some(created_at) = &result.metadata.created_at {
            metadata_obj.insert("created_at".to_string(), json!(created_at));
        }
        if let Some(modified_at) = &result.metadata.modified_at {
            metadata_obj.insert("modified_at".to_string(), json!(modified_at));
        }
        if let Some(created_by) = &result.metadata.created_by {
            metadata_obj.insert("created_by".to_string(), json!(created_by));
        }
        if let Some(modified_by) = &result.metadata.modified_by {
            metadata_obj.insert("modified_by".to_string(), json!(modified_by));
        }

        // Add page count - try multiple sources
        let page_count = if let Some(pages_meta) = &result.metadata.pages {
            // Prefer page structure metadata if available
            Some(pages_meta.total_count)
        } else {
            // Fallback to counting pages array
            result.pages.as_ref().map(|pages_array| pages_array.len())
        };

        if let Some(count) = page_count {
            metadata_obj.insert("page_count".to_string(), json!(count));
        }

        // Add pages metadata structure if available
        if let Some(pages) = &result.metadata.pages {
            let pages_json = serde_json::to_value(pages).map_err(|e| format!("Failed to serialize pages: {}", e))?;
            metadata_obj.insert("pages".to_string(), pages_json);
        }

        // Add format metadata (both nested and flattened for compatibility)
        if let Some(format) = &result.metadata.format {
            let format_json = serde_json::to_value(format).map_err(|e| format!("Failed to serialize format: {}", e))?;

            // The format_json will have a "format_type" field due to the #[serde(tag = "format_type")] attribute
            // We flatten it into the root metadata object for fields like format_type, sheet_count, etc.
            if let serde_json::Value::Object(ref format_obj) = format_json {
                for (key, value) in format_obj {
                    metadata_obj.insert(key.clone(), value.clone());
                }
            }

            // Also add the full format object as a nested "format_nested" key for paths like "format_nested.format"
            // (renamed from "format" to avoid conflicts with ImageMetadata.format)
            metadata_obj.insert("format_nested".to_string(), format_json);
        }

        // Extract keywords from additional metadata before adding all additional fields
        let keywords = if let Some(keywords_value) = result.metadata.additional.get("keywords") {
            if let Some(arr) = keywords_value.as_array() {
                let mut keywords_vec = Vec::new();
                for keyword_value in arr {
                    match Keyword::from_json(keyword_value) {
                        Ok(kw) => keywords_vec.push(kw),
                        Err(_e) => {
                            continue;
                        }
                    }
                }
                if !keywords_vec.is_empty() {
                    Some(keywords_vec)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let extracted_keywords = result.extracted_keywords.as_ref().map(|kws| {
            kws.iter()
                .map(|kw| Keyword {
                    text: kw.text.clone(),
                    score: kw.score,
                    algorithm: Some(format!("{:?}", kw.algorithm).to_lowercase()),
                    positions: None,
                })
                .collect::<Vec<_>>()
        });

        let quality_score = result.quality_score;

        // Add error metadata if present (for batch operations)
        if let Some(error) = &result.metadata.error {
            let error_json = serde_json::to_value(error).map_err(|e| format!("Failed to serialize error: {}", e))?;
            metadata_obj.insert("error".to_string(), error_json);
        }

        // Add additional metadata fields (from postprocessors)
        for (key, value) in &result.metadata.additional {
            metadata_obj.insert(key.to_string(), value.clone());
        }

        // Serialize the metadata to JSON string
        let metadata_json =
            serde_json::to_string(&metadata_obj).map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        let tables = if extract_tables {
            result
                .tables
                .into_iter()
                .map(ExtractedTable::from_rust)
                .collect::<PhpResult<Vec<_>>>()?
        } else {
            vec![]
        };

        let images = result
            .images
            .map(|imgs| {
                imgs.into_iter()
                    .map(ExtractedImage::from_rust)
                    .collect::<PhpResult<Vec<_>>>()
            })
            .transpose()?;

        let chunks = result
            .chunks
            .map(|chnks| {
                chnks
                    .into_iter()
                    .map(TextChunk::from_rust)
                    .collect::<PhpResult<Vec<_>>>()
            })
            .transpose()?;

        let pages = result
            .pages
            .map(|pgs| {
                pgs.into_iter()
                    .map(PageResult::from_rust)
                    .collect::<PhpResult<Vec<_>>>()
            })
            .transpose()?;

        // Serialize djot_content to JSON if present
        let djot_content_json = result
            .djot_content
            .map(|djot| serde_json::to_string(&djot))
            .transpose()
            .map_err(|e| format!("Failed to serialize djot_content: {}", e))?;

        // Serialize elements to JSON if present
        let elements_json = result
            .elements
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize elements: {}", e))?;

        // Serialize document structure to JSON if present
        let document_json = result
            .document
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize document: {}", e))?;

        // Serialize OCR elements to JSON if present
        let ocr_elements_json = result
            .ocr_elements
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize ocr_elements: {}", e))?;

        let children_json = result
            .children
            .map(|c| serde_json::to_string(&c))
            .transpose()
            .map_err(|e| format!("Failed to serialize children: {}", e))?;

        let uris_json = result
            .uris
            .map(|u| serde_json::to_string(&u))
            .transpose()
            .map_err(|e| format!("Failed to serialize uris: {}", e))?;

        // Convert annotations if present
        let annotations = result
            .annotations
            .map(|anns| {
                anns.into_iter()
                    .map(PdfAnnotation::from_rust)
                    .collect::<PhpResult<Vec<_>>>()
            })
            .transpose()?;

        let processing_warnings = result
            .processing_warnings
            .iter()
            .map(|w| ProcessingWarning {
                message: w.message.to_string(),
                source: w.source.to_string(),
            })
            .collect();

        Ok(Self {
            content: result.content,
            mime_type: result.mime_type.to_string(),
            metadata_json,
            tables,
            detected_languages: result.detected_languages,
            images,
            chunks,
            pages,
            keywords,
            extracted_keywords,
            quality_score,
            djot_content_json,
            elements_json,
            document_json,
            ocr_elements_json,
            annotations,
            children_json,
            uris_json,
            processing_warnings,
            result_json,
        })
    }
}

/// Extracted table with cells and markdown representation.
///
/// # Properties
///
/// - `cells` (array): Table data as nested arrays (rows of columns)
/// - `markdown` (string): Markdown representation of the table
/// - `page_number` (int): Page number where table was found
///
/// # Example
///
/// ```php
/// foreach ($result->tables as $table) {
///     echo "Table on page {$table->page_number}:\n";
///     echo $table->markdown . "\n";
///     echo "Dimensions: " . count($table->cells) . " rows\n";
/// }
/// ```
#[php_class]
#[php(name = "Kreuzberg\\Types\\ExtractedTable")]
#[derive(Clone)]
pub struct ExtractedTable {
    /// Table cells as nested arrays
    #[php(prop)]
    pub cells: Vec<Vec<String>>,

    /// Markdown representation
    #[php(prop)]
    pub markdown: String,

    /// Page number
    #[php(prop)]
    pub page_number: usize,

    /// Bounding box as associative array {x0, y0, x1, y1} or null
    #[php(prop)]
    pub bounding_box: Option<HashMap<String, f64>>,
}

#[php_impl]
impl ExtractedTable {
    /// Get the table cells as nested arrays.
    #[php(name = "getCells")]
    pub fn get_cells(&self) -> Vec<Vec<String>> {
        self.cells.clone()
    }
}

impl ExtractedTable {
    /// Convert from Rust Table to PHP ExtractedTable.
    pub fn from_rust(table: kreuzberg::Table) -> PhpResult<Self> {
        let bounding_box = table.bounding_box.map(|bb| {
            let mut map = HashMap::new();
            map.insert("x0".to_string(), bb.x0);
            map.insert("y0".to_string(), bb.y0);
            map.insert("x1".to_string(), bb.x1);
            map.insert("y1".to_string(), bb.y1);
            map
        });
        Ok(Self {
            cells: table.cells,
            markdown: table.markdown,
            page_number: table.page_number,
            bounding_box,
        })
    }
}

/// Extracted image with data and metadata.
///
/// # Properties
///
/// - `data` (string): Binary image data
/// - `format` (string): Image format (e.g., "png", "jpeg")
/// - `image_index` (int): Index of image in document
/// - `page_number` (int|null): Page number where image was found
/// - `width` (int|null): Image width in pixels
/// - `height` (int|null): Image height in pixels
#[php_class]
#[php(name = "Kreuzberg\\Types\\ExtractedImage")]
#[derive(Clone)]
pub struct ExtractedImage {
    #[php(prop)]
    pub data: Vec<u8>,
    #[php(prop)]
    pub format: String,
    #[php(prop)]
    pub image_index: usize,
    #[php(prop)]
    pub page_number: Option<usize>,
    #[php(prop)]
    pub width: Option<i32>,
    #[php(prop)]
    pub height: Option<i32>,
    #[php(prop)]
    pub colorspace: Option<String>,
    #[php(prop)]
    pub bits_per_component: Option<i32>,
    #[php(prop)]
    pub description: Option<String>,
    #[php(prop)]
    pub is_mask: bool,
    /// Bounding box as associative array {x0, y0, x1, y1} or null
    #[php(prop)]
    pub bounding_box: Option<HashMap<String, f64>>,
    #[php(prop)]
    pub source_path: Option<String>,
}

#[php_impl]
impl ExtractedImage {
    /// Get the binary image data.
    #[php(name = "getData")]
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl ExtractedImage {
    pub fn from_rust(img: kreuzberg::ExtractedImage) -> PhpResult<Self> {
        let bounding_box = img.bounding_box.map(|bb| {
            let mut map = HashMap::new();
            map.insert("x0".to_string(), bb.x0);
            map.insert("y0".to_string(), bb.y0);
            map.insert("x1".to_string(), bb.x1);
            map.insert("y1".to_string(), bb.y1);
            map
        });
        Ok(Self {
            data: img.data.to_vec(),
            format: img.format.into_owned(),
            image_index: img.image_index,
            page_number: img.page_number,
            width: img.width.map(|w| w as i32),
            height: img.height.map(|h| h as i32),
            colorspace: img.colorspace,
            bits_per_component: img.bits_per_component.map(|b| b as i32),
            description: img.description,
            is_mask: img.is_mask,
            bounding_box,
            source_path: img.source_path,
        })
    }
}

/// Chunk metadata describing offsets within the original document.
///
/// # Properties
///
/// - `byte_start` (int): Starting byte offset
/// - `byte_end` (int): Ending byte offset
/// - `token_count` (int|null): Number of tokens in chunk
/// - `chunk_index` (int): Chunk index (0-based)
/// - `total_chunks` (int): Total number of chunks
/// - `first_page` (int|null): First page number in chunk
/// - `last_page` (int|null): Last page number in chunk
#[php_class]
#[php(name = "Kreuzberg\\Types\\ChunkMetadata")]
#[derive(Clone)]
pub struct ChunkMetadata {
    #[php(prop)]
    pub byte_start: usize,
    #[php(prop)]
    pub byte_end: usize,
    #[php(prop)]
    pub token_count: Option<usize>,
    #[php(prop)]
    pub chunk_index: usize,
    #[php(prop)]
    pub total_chunks: usize,
    #[php(prop)]
    pub first_page: Option<usize>,
    #[php(prop)]
    pub last_page: Option<usize>,
}

#[php_impl]
impl ChunkMetadata {}

impl ChunkMetadata {
    pub fn from_rust(metadata: kreuzberg::ChunkMetadata) -> PhpResult<Self> {
        Ok(Self {
            byte_start: metadata.byte_start,
            byte_end: metadata.byte_end,
            token_count: metadata.token_count,
            chunk_index: metadata.chunk_index,
            total_chunks: metadata.total_chunks,
            first_page: metadata.first_page,
            last_page: metadata.last_page,
        })
    }
}

/// Text chunk with optional embedding.
///
/// # Properties
///
/// - `content` (string): Chunk text content
/// - `embedding` (array|null): Embedding vector (if enabled)
/// - `metadata` (ChunkMetadata): Chunk metadata describing offsets and properties
#[php_class]
#[php(name = "Kreuzberg\\Types\\TextChunk")]
#[derive(Clone)]
pub struct TextChunk {
    #[php(prop)]
    pub content: String,
    #[php(prop)]
    pub chunk_type: String,
    #[php(prop)]
    pub embedding: Option<Vec<f32>>,
    pub metadata: ChunkMetadata,
}

#[php_impl]
impl TextChunk {
    /// Get chunk metadata.
    #[php(name = "getMetadata")]
    pub fn get_metadata(&self) -> ChunkMetadata {
        self.metadata.clone()
    }
}

impl TextChunk {
    pub fn from_rust(chunk: kreuzberg::Chunk) -> PhpResult<Self> {
        Ok(Self {
            content: chunk.content,
            chunk_type: serde_json::to_value(chunk.chunk_type)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "unknown".to_string()),
            embedding: chunk.embedding,
            metadata: ChunkMetadata::from_rust(chunk.metadata)?,
        })
    }
}

/// Per-page extraction result.
///
/// # Properties
///
/// - `page_number` (int): Page number (1-indexed)
/// - `content` (string): Extracted text for this page
/// - `tables` (array): Tables found on this page
/// - `images` (array): Images found on this page
#[php_class]
#[php(name = "Kreuzberg\\Types\\PageResult")]
#[derive(Clone)]
pub struct PageResult {
    #[php(prop)]
    pub page_number: usize,
    #[php(prop)]
    pub content: String,
    pub tables: Vec<ExtractedTable>,
    pub images: Vec<ExtractedImage>,
}

#[php_impl]
impl PageResult {
    /// Get all tables on this page.
    #[php(name = "getTables")]
    pub fn get_tables(&self) -> Vec<ExtractedTable> {
        self.tables.clone()
    }

    /// Get all images on this page.
    #[php(name = "getImages")]
    pub fn get_images(&self) -> Vec<ExtractedImage> {
        self.images.clone()
    }
}

impl PageResult {
    pub fn from_rust(page: kreuzberg::PageContent) -> PhpResult<Self> {
        let tables = page
            .tables
            .into_iter()
            .map(|arc_t| {
                let table: kreuzberg::Table = (*arc_t).clone();
                ExtractedTable::from_rust(table)
            })
            .collect::<PhpResult<Vec<_>>>()?;

        let images = page
            .images
            .into_iter()
            .map(|arc_img| {
                let img: kreuzberg::ExtractedImage = (*arc_img).clone();
                ExtractedImage::from_rust(img)
            })
            .collect::<PhpResult<Vec<_>>>()?;

        Ok(Self {
            page_number: page.page_number,
            content: page.content,
            tables,
            images,
        })
    }
}

/// Document metadata object.
///
/// Provides access to common metadata fields as properties.
/// Additional fields are accessible via the additional property.
#[php_class]
#[php(name = "Kreuzberg\\Types\\Metadata")]
#[derive(Clone)]
pub struct Metadata {
    /// Document title
    #[php(prop)]
    pub title: Option<String>,

    /// Document subject
    #[php(prop)]
    pub subject: Option<String>,

    /// Authors list
    #[php(prop)]
    pub authors: Option<Vec<String>>,

    /// Keywords list
    #[php(prop)]
    pub keywords: Option<Vec<String>>,

    /// Language code
    #[php(prop)]
    pub language: Option<String>,

    /// Creation timestamp
    #[php(prop)]
    pub created_at: Option<String>,

    /// Modification timestamp
    #[php(prop)]
    pub modified_at: Option<String>,

    /// Creator name
    #[php(prop)]
    pub created_by: Option<String>,

    /// Modifier name
    #[php(prop)]
    pub modified_by: Option<String>,

    /// Page count (convenience field)
    #[php(prop)]
    pub page_count: Option<usize>,

    /// Format type discriminator (e.g., "pdf", "excel", "image")
    #[php(prop)]
    pub format_type: Option<String>,

    /// Sheet count for Excel/spreadsheet documents
    #[php(prop)]
    pub sheet_count: Option<usize>,

    /// Image format (e.g., "PNG", "JPEG") for image documents
    #[php(prop)]
    pub format: Option<String>,

    /// Additional metadata fields (stored as JSON for now)
    additional_json: String,
}

#[php_impl]
impl Metadata {
    /// Create Metadata from JSON string.
    pub fn from_json(json: &str) -> PhpResult<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;

        let obj = value.as_object().ok_or("Metadata must be a JSON object")?;

        Ok(Self {
            title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
            subject: obj.get("subject").and_then(|v| v.as_str()).map(String::from),
            authors: obj.get("authors").and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
            }),
            keywords: obj.get("keywords").and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
            }),
            language: obj.get("language").and_then(|v| v.as_str()).map(String::from),
            created_at: obj.get("created_at").and_then(|v| v.as_str()).map(String::from),
            modified_at: obj.get("modified_at").and_then(|v| v.as_str()).map(String::from),
            created_by: obj.get("created_by").and_then(|v| v.as_str()).map(String::from),
            modified_by: obj.get("modified_by").and_then(|v| v.as_str()).map(String::from),
            page_count: obj.get("page_count").and_then(|v| v.as_u64()).map(|n| n as usize),
            format_type: obj.get("format_type").and_then(|v| v.as_str()).map(String::from),
            sheet_count: obj.get("sheet_count").and_then(|v| v.as_u64()).map(|n| n as usize),
            format: obj.get("format").and_then(|v| v.as_str()).map(String::from),
            additional_json: json.to_string(),
        })
    }

    /// Get additional metadata fields as associative array.
    pub fn get_additional(&self) -> PhpResult<HashMap<String, Zval>> {
        let value: serde_json::Value = serde_json::from_str(&self.additional_json)
            .map_err(|e| format!("Failed to parse additional JSON: {}", e))?;

        if let serde_json::Value::Object(obj) = value {
            let mut result = HashMap::new();
            // Exclude the common fields we already exposed
            let excluded = [
                "title",
                "subject",
                "authors",
                "keywords",
                "language",
                "created_at",
                "modified_at",
                "created_by",
                "modified_by",
                "page_count",
                "format_type",
                "sheet_count",
                "format",
            ];
            for (k, v) in obj {
                if !excluded.contains(&k.as_str()) {
                    result.insert(k, json_value_to_php(&v)?);
                }
            }
            Ok(result)
        } else {
            Ok(HashMap::new())
        }
    }

    /// Get all metadata as associative array (for backwards compatibility).
    #[php(name = "to_array")]
    pub fn to_array(&self) -> PhpResult<HashMap<String, Zval>> {
        let value: serde_json::Value =
            serde_json::from_str(&self.additional_json).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        if let serde_json::Value::Object(obj) = value {
            let mut result = HashMap::new();
            for (k, v) in obj {
                result.insert(k, json_value_to_php(&v)?);
            }
            Ok(result)
        } else {
            Ok(HashMap::new())
        }
    }

    /// Check if a custom metadata field exists.
    pub fn has_custom(&self, key: String) -> PhpResult<bool> {
        let additional = self.get_additional()?;
        Ok(additional.contains_key(&key))
    }

    /// Get a custom metadata field value.
    pub fn get_custom(&self, key: String) -> PhpResult<Zval> {
        let additional = self.get_additional()?;
        Ok(additional
            .get(&key)
            .map(|v| v.shallow_clone())
            .unwrap_or_else(Zval::new))
    }
}

/// PDF annotation type classification (PHP enum).
#[php_enum]
#[php(name = "Kreuzberg\\Types\\PdfAnnotationType")]
pub enum PdfAnnotationType {
    #[php(name = "text")]
    Text,
    #[php(name = "highlight")]
    Highlight,
    #[php(name = "link")]
    Link,
    #[php(name = "stamp")]
    Stamp,
    #[php(name = "underline")]
    Underline,
    #[php(name = "strikeOut")]
    StrikeOut,
    #[php(name = "other")]
    Other,
}

impl PdfAnnotationType {
    /// Convert from Rust PdfAnnotationType to PHP PdfAnnotationType.
    pub fn from_rust(annotation_type: kreuzberg::PdfAnnotationType) -> Self {
        match annotation_type {
            kreuzberg::PdfAnnotationType::Text => Self::Text,
            kreuzberg::PdfAnnotationType::Highlight => Self::Highlight,
            kreuzberg::PdfAnnotationType::Link => Self::Link,
            kreuzberg::PdfAnnotationType::Stamp => Self::Stamp,
            kreuzberg::PdfAnnotationType::Underline => Self::Underline,
            kreuzberg::PdfAnnotationType::StrikeOut => Self::StrikeOut,
            kreuzberg::PdfAnnotationType::Other => Self::Other,
        }
    }

    /// Get the string value of this annotation type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Highlight => "highlight",
            Self::Link => "link",
            Self::Stamp => "stamp",
            Self::Underline => "underline",
            Self::StrikeOut => "strike_out",
            Self::Other => "other",
        }
    }
}

/// PDF annotation extracted from a document page.
///
/// # Properties
///
/// - `annotationType` (string): The type of annotation as string
/// - `content` (string|null): Text content of the annotation
/// - `pageNumber` (int): Page number where the annotation appears (1-indexed)
/// - `boundingBox` (array|null): Bounding box as {x0, y0, x1, y1} or null
#[php_class]
#[php(name = "Kreuzberg\\Types\\PdfAnnotation")]
#[derive(Clone, Debug)]
pub struct PdfAnnotation {
    /// The type of annotation as string (exposed as property)
    #[php(prop)]
    #[php(name = "annotationType")]
    pub annotation_type: String,

    /// Text content of the annotation (e.g., comment text, link URL)
    #[php(prop)]
    pub content: Option<String>,

    /// Page number where the annotation appears (1-indexed)
    #[php(prop)]
    #[php(name = "pageNumber")]
    pub page_number: usize,

    /// Bounding box as associative array {x0, y0, x1, y1} or null
    #[php(prop)]
    #[php(name = "boundingBox")]
    pub bounding_box: Option<HashMap<String, f64>>,
}

#[php_impl]
impl PdfAnnotation {
    /// Get the annotation type as a string.
    #[php(name = "getAnnotationType")]
    pub fn get_annotation_type(&self) -> String {
        self.annotation_type.clone()
    }
}

impl PdfAnnotation {
    /// Convert from Rust PdfAnnotation to PHP PdfAnnotation.
    pub fn from_rust(annotation: kreuzberg::PdfAnnotation) -> PhpResult<Self> {
        let bounding_box = annotation.bounding_box.map(|bb| {
            let mut map = HashMap::new();
            map.insert("x0".to_string(), bb.x0);
            map.insert("y0".to_string(), bb.y0);
            map.insert("x1".to_string(), bb.x1);
            map.insert("y1".to_string(), bb.y1);
            map
        });
        let annotation_type = PdfAnnotationType::from_rust(annotation.annotation_type)
            .as_str()
            .to_string();
        Ok(Self {
            annotation_type,
            content: annotation.content,
            page_number: annotation.page_number,
            bounding_box,
        })
    }
}

/// Extracted keyword with score and metadata.
///
/// # Properties
///
/// - `text` (string): The keyword text
/// - `score` (float): Keyword relevance score (0-1 range typically)
/// - `algorithm` (string|null): The algorithm used to extract this keyword
#[php_class]
#[php(name = "Kreuzberg\\Types\\Keyword")]
#[derive(Clone, Debug)]
pub struct Keyword {
    /// Keyword text
    #[php(prop)]
    pub text: String,

    /// Keyword score
    #[php(prop)]
    pub score: f32,

    /// Algorithm used
    #[php(prop)]
    pub algorithm: Option<String>,

    /// Positions where the keyword appears
    #[php(prop)]
    pub positions: Option<Vec<i64>>,
}

impl Keyword {
    /// Convert from JSON value to Keyword.
    pub fn from_json(value: &serde_json::Value) -> PhpResult<Self> {
        let obj = value.as_object().ok_or("Keyword must be a JSON object")?;

        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Keyword must have a 'text' field")?
            .to_string();

        let score = obj
            .get("score")
            .and_then(|v| v.as_f64())
            .ok_or("Keyword must have a 'score' field")? as f32;

        let algorithm = obj.get("algorithm").and_then(|v| v.as_str()).map(|s| s.to_string());

        let positions = obj.get("positions").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|item| item.as_i64()).collect())
        });

        Ok(Self {
            text,
            score,
            algorithm,
            positions,
        })
    }
}

/// Convert PHP Zval to serde_json::Value recursively.
pub(crate) fn php_zval_to_json_value(zval: &Zval) -> PhpResult<serde_json::Value> {
    use serde_json::json;

    if zval.is_null() {
        return Ok(serde_json::Value::Null);
    }
    if let Some(b) = zval.bool() {
        return Ok(json!(b));
    }
    if let Some(i) = zval.long() {
        return Ok(json!(i));
    }
    if let Some(f) = zval.double() {
        return Ok(json!(f));
    }
    if let Some(s) = zval.str() {
        return Ok(json!(s.to_string()));
    }
    if let Some(arr) = zval.array() {
        let mut map = serde_json::Map::new();
        for (key, val) in arr.iter() {
            // Convert key to string, handling both numeric and string keys
            let key_str = format!("{}", key);
            map.insert(key_str, php_zval_to_json_value(val)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    Ok(serde_json::Value::Null)
}

/// Convert PHP array to kreuzberg::types::Table.
pub(crate) fn php_array_to_table(arr: &ext_php_rs::types::ZendHashTable) -> PhpResult<kreuzberg::types::Table> {
    // Extract cells as 2D array
    let cells = if let Some(cells_val) = arr.get("cells") {
        if let Some(cells_arr) = cells_val.array() {
            let mut rows = Vec::new();
            for (_, row_val) in cells_arr.iter() {
                if let Some(row_arr) = row_val.array() {
                    let mut cols = Vec::new();
                    for (_, cell_val) in row_arr.iter() {
                        cols.push(cell_val.str().unwrap_or("").to_string());
                    }
                    rows.push(cols);
                }
            }
            rows
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let markdown = arr.get("markdown").and_then(|v| v.str()).unwrap_or("").to_string();

    let page_number = arr
        .get("page_number")
        .and_then(|v| v.long())
        .map(|v| v as usize)
        .unwrap_or(1);

    let bounding_box = arr
        .get("bounding_box")
        .and_then(|v| v.array())
        .map(|bb_arr| kreuzberg::types::BoundingBox {
            x0: bb_arr.get("x0").and_then(|v| v.double()).unwrap_or(0.0),
            y0: bb_arr.get("y0").and_then(|v| v.double()).unwrap_or(0.0),
            x1: bb_arr.get("x1").and_then(|v| v.double()).unwrap_or(0.0),
            y1: bb_arr.get("y1").and_then(|v| v.double()).unwrap_or(0.0),
        });

    Ok(kreuzberg::types::Table {
        cells,
        markdown,
        page_number,
        bounding_box,
    })
}
