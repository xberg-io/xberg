//! Result type bindings
//!
//! Provides PHP-friendly wrappers around extraction result types.

use ext_php_rs::prelude::*;

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
#[derive(Clone)]
pub struct ExtractionResult {
    /// Extracted text content
    pub content: String,

    /// MIME type of the document
    pub mime_type: String,

    /// Document metadata
    pub metadata: Vec<(String, String)>,

    /// Extracted tables
    pub tables: Vec<ExtractedTable>,

    /// Detected languages
    pub detected_languages: Option<Vec<String>>,

    /// Extracted images
    pub images: Option<Vec<ExtractedImage>>,

    /// Text chunks
    pub chunks: Option<Vec<TextChunk>>,

    /// Per-page results
    pub pages: Option<Vec<PageResult>>,
}

#[php_impl]
impl ExtractionResult {
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
}

impl ExtractionResult {
    /// Convert from Rust ExtractionResult to PHP ExtractionResult.
    pub fn from_rust(result: kreuzberg::ExtractionResult) -> PhpResult<Self> {
        let mut metadata = Vec::new();

        if let Some(title) = &result.metadata.title {
            metadata.push(("title".to_string(), title.clone()));
        }
        if let Some(subject) = &result.metadata.subject {
            metadata.push(("subject".to_string(), subject.clone()));
        }
        if let Some(authors) = &result.metadata.authors {
            metadata.push(("authors".to_string(), authors.join(", ")));
        }
        if let Some(keywords) = &result.metadata.keywords {
            metadata.push(("keywords".to_string(), keywords.join(", ")));
        }
        if let Some(language) = &result.metadata.language {
            metadata.push(("language".to_string(), language.clone()));
        }
        if let Some(created_at) = &result.metadata.created_at {
            metadata.push(("created_at".to_string(), created_at.clone()));
        }
        if let Some(modified_at) = &result.metadata.modified_at {
            metadata.push(("modified_at".to_string(), modified_at.clone()));
        }
        if let Some(created_by) = &result.metadata.created_by {
            metadata.push(("created_by".to_string(), created_by.clone()));
        }
        if let Some(modified_by) = &result.metadata.modified_by {
            metadata.push(("modified_by".to_string(), modified_by.clone()));
        }

        let tables = result
            .tables
            .into_iter()
            .map(ExtractedTable::from_rust)
            .collect::<PhpResult<Vec<_>>>()?;

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

        Ok(Self {
            content: result.content,
            mime_type: result.mime_type,
            metadata,
            tables,
            detected_languages: result.detected_languages,
            images,
            chunks,
            pages,
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
#[derive(Clone)]
pub struct ExtractedTable {
    /// Table cells as nested arrays
    pub cells: Vec<Vec<String>>,

    /// Markdown representation
    pub markdown: String,

    /// Page number
    pub page_number: usize,
}

#[php_impl]
impl ExtractedTable {}

impl ExtractedTable {
    /// Convert from Rust Table to PHP ExtractedTable.
    pub fn from_rust(table: kreuzberg::Table) -> PhpResult<Self> {
        Ok(Self {
            cells: table.cells,
            markdown: table.markdown,
            page_number: table.page_number,
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
#[derive(Clone)]
pub struct ExtractedImage {
    pub data: Vec<u8>,
    pub format: String,
    pub image_index: usize,
    pub page_number: Option<usize>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<i32>,
    pub description: Option<String>,
    pub is_mask: bool,
}

#[php_impl]
impl ExtractedImage {}

impl ExtractedImage {
    pub fn from_rust(img: kreuzberg::ExtractedImage) -> PhpResult<Self> {
        Ok(Self {
            data: img.data,
            format: img.format,
            image_index: img.image_index,
            page_number: img.page_number,
            width: img.width.map(|w| w as i32),
            height: img.height.map(|h| h as i32),
            colorspace: img.colorspace,
            bits_per_component: img.bits_per_component.map(|b| b as i32),
            description: img.description,
            is_mask: img.is_mask,
        })
    }
}

/// Text chunk with optional embedding.
///
/// # Properties
///
/// - `content` (string): Chunk text content
/// - `embedding` (array|null): Embedding vector (if enabled)
/// - `byte_start` (int): Start byte offset
/// - `byte_end` (int): End byte offset
/// - `chunk_index` (int): Index of this chunk
/// - `total_chunks` (int): Total number of chunks
#[php_class]
#[derive(Clone)]
pub struct TextChunk {
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub token_count: usize,
    pub first_page: Option<usize>,
    pub last_page: Option<usize>,
}

#[php_impl]
impl TextChunk {}

impl TextChunk {
    pub fn from_rust(chunk: kreuzberg::Chunk) -> PhpResult<Self> {
        Ok(Self {
            content: chunk.content,
            embedding: chunk.embedding,
            byte_start: chunk.metadata.byte_start,
            byte_end: chunk.metadata.byte_end,
            chunk_index: chunk.metadata.chunk_index,
            total_chunks: chunk.metadata.total_chunks,
            token_count: chunk.metadata.token_count.unwrap_or(0),
            first_page: chunk.metadata.first_page,
            last_page: chunk.metadata.last_page,
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
#[derive(Clone)]
pub struct PageResult {
    pub page_number: usize,
    pub content: String,
    pub tables: Vec<ExtractedTable>,
    pub images: Vec<ExtractedImage>,
}

#[php_impl]
impl PageResult {}

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
