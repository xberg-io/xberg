"""Type definitions for Kreuzberg extraction results.

These TypedDicts mirror the strongly-typed Rust metadata structures,
providing type hints for Python users while the actual data comes from
the Rust core via PyO3 bindings.
"""

# ruff: noqa: A005
from __future__ import annotations

import sys
from typing import Any, Literal, TypedDict

if sys.version_info >= (3, 11):
    from enum import StrEnum
else:
    from enum import Enum

    class StrEnum(str, Enum):
        """Backport of StrEnum for Python 3.10."""


class OutputFormat(StrEnum):
    """Output content format for extraction results.

    Controls the format of the extracted content in ExtractionConfig.

    Values:
        PLAIN: Plain text content only (default)
        MARKDOWN: Markdown format
        DJOT: Djot markup format
        HTML: HTML format
    """

    PLAIN = "plain"
    MARKDOWN = "markdown"
    DJOT = "djot"
    HTML = "html"


class ResultFormat(StrEnum):
    """Result structure format for extraction output.

    Controls whether results use unified format or element-based format.

    Values:
        UNIFIED: All content in a single content field (default)
        ELEMENT_BASED: Semantic elements for Unstructured-compatible output
    """

    UNIFIED = "unified"
    ELEMENT_BASED = "element_based"


class ExcelMetadata(TypedDict, total=False):
    """Excel/spreadsheet metadata."""

    sheet_count: int
    sheet_names: list[str]


class EmailMetadata(TypedDict, total=False):
    """Email metadata."""

    from_email: str | None
    from_name: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str | None
    attachments: list[str]


class ArchiveMetadata(TypedDict, total=False):
    """Archive (ZIP/TAR/7Z) metadata."""

    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None


class ImageMetadata(TypedDict, total=False):
    """Image metadata."""

    width: int
    height: int
    format: str
    exif: dict[str, str]


class XmlMetadata(TypedDict, total=False):
    """XML metadata."""

    element_count: int
    unique_elements: list[str]


class TextMetadata(TypedDict, total=False):
    """Text/Markdown metadata."""

    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None


class PdfMetadata(TypedDict, total=False):
    """PDF metadata."""

    title: str | None
    author: str | None
    subject: str | None
    keywords: str | None
    creator: str | None
    producer: str | None
    creation_date: str | None
    modification_date: str | None
    page_count: int


class HeaderMetadata(TypedDict):
    """HTML header/heading metadata."""

    level: int
    text: str
    id: str | None
    depth: int
    html_offset: int


class LinkMetadata(TypedDict):
    """HTML link metadata."""

    href: str
    text: str
    title: str | None
    link_type: Literal["anchor", "internal", "external", "email", "phone", "other"]
    rel: list[str]
    attributes: dict[str, str]


class HtmlImageMetadata(TypedDict):
    """HTML image metadata."""

    src: str
    alt: str | None
    title: str | None
    dimensions: tuple[int, int] | None
    image_type: Literal["data_uri", "inline_svg", "external", "relative"]
    attributes: dict[str, str]


class StructuredData(TypedDict):
    """Structured data (JSON-LD, microdata, RDFa) metadata."""

    data_type: Literal["json_ld", "microdata", "rdfa"]
    raw_json: str
    schema_type: str | None


class HtmlMetadata(TypedDict, total=False):
    """HTML metadata."""

    title: str | None
    description: str | None
    keywords: list[str]
    author: str | None
    canonical_url: str | None
    base_href: str | None
    language: str | None
    text_direction: Literal["ltr", "rtl", "auto"] | None
    open_graph: dict[str, str]
    twitter_card: dict[str, str]
    meta_tags: dict[str, str]
    headers: list[HeaderMetadata]
    links: list[LinkMetadata]
    images: list[HtmlImageMetadata]
    structured_data: list[StructuredData]


class PptxMetadata(TypedDict, total=False):
    """PowerPoint metadata."""

    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]


class OcrMetadata(TypedDict, total=False):
    """OCR processing metadata."""

    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None


class ImagePreprocessingMetadata(TypedDict, total=False):
    """Image preprocessing metadata."""

    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int] | None
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int | None
    skipped_resize: bool
    resize_error: str | None


class ErrorMetadata(TypedDict, total=False):
    """Error metadata for batch operations."""

    error_type: str
    message: str


class ChunkMetadata(TypedDict):
    """Chunk metadata describing offsets within the original document."""

    byte_start: int
    byte_end: int
    token_count: int | None
    chunk_index: int
    total_chunks: int
    first_page: int | None
    last_page: int | None


class PageBoundary(TypedDict):
    """Page boundaries in the document content."""

    byte_start: int
    byte_end: int
    page_number: int


class PageConfig(TypedDict, total=False):
    """Page extraction configuration."""

    extract_pages: bool
    insert_page_markers: bool
    marker_format: str


class PageInfo(TypedDict, total=False):
    """Metadata for an individual page/slide/sheet.

    Captures per-page information including dimensions, content counts,
    and visibility state (for presentations).
    """

    number: int
    title: str | None
    dimensions: tuple[float, float] | None
    image_count: int | None
    table_count: int | None
    hidden: bool | None


PageUnitType = Literal["page", "slide", "sheet"]
"""Type of paginated unit in a document.

Distinguishes between different types of "pages":
- "page": Standard document pages (PDF, DOCX, images)
- "slide": Presentation slides (PPTX, ODP)
- "sheet": Spreadsheet sheets (XLSX, ODS)
"""


class PageStructure(TypedDict, total=False):
    """Page structure metadata.

    Contains information about pages/slides/sheets in a document, including
    boundaries for mapping chunks to pages and detailed per-page metadata.
    """

    total_count: int
    unit_type: PageUnitType
    boundaries: list[PageBoundary] | None
    pages: list[PageInfo] | None


class HierarchicalBlock(TypedDict, total=False):
    """A text block with hierarchy level assignment."""

    text: str
    font_size: float
    level: str
    bbox: tuple[float, float, float, float] | None


class PageHierarchy(TypedDict, total=False):
    """Page hierarchy structure containing heading levels and block information."""

    block_count: int
    blocks: list[HierarchicalBlock]


class PageContent(TypedDict):
    """Content for a single page/slide.

    When page extraction is enabled, documents are split into per-page content
    with associated tables and images mapped to each page.
    """

    page_number: int
    content: str
    tables: list[Table]
    images: list[ExtractedImage]
    hierarchy: PageHierarchy | None


ElementType = Literal[
    "title",
    "narrative_text",
    "heading",
    "list_item",
    "table",
    "image",
    "page_break",
    "code_block",
    "block_quote",
    "footer",
    "header",
]
"""Semantic element type classification.

Classifies extracted document elements into semantic categories
compatible with Unstructured.io element types.
"""


class BoundingBox(TypedDict):
    """Bounding box coordinates for an element."""

    x0: float  # Left x-coordinate
    y0: float  # Bottom y-coordinate
    x1: float  # Right x-coordinate
    y1: float  # Top y-coordinate


class ElementMetadata(TypedDict, total=False):
    """Metadata for a semantic element.

    Attributes:
        page_number: Page number (1-indexed)
        filename: Source filename or document name
        coordinates: Bounding box coordinates if available
        element_index: Position index in the element sequence
        additional: Additional custom metadata fields
    """

    page_number: int | None
    filename: str | None
    coordinates: BoundingBox | None
    element_index: int | None
    additional: dict[str, str]


class Element(TypedDict):
    """Semantic element extracted from document.

    Represents a logical unit of content with semantic classification,
    unique identifier, and metadata for tracking origin and position.
    Compatible with Unstructured.io element format when output_format='element_based'.

    Attributes:
        element_id: Unique identifier (deterministic hash-based ID)
        element_type: Semantic type classification
        text: Content string
        metadata: Element metadata including page number, coordinates, etc.
    """

    element_id: str
    element_type: ElementType
    text: str
    metadata: ElementMetadata


class Chunk(TypedDict, total=False):
    """Text chunk with optional embedding vector."""

    content: str
    embedding: list[float] | None
    metadata: ChunkMetadata


class ExtractedImage(TypedDict, total=False):
    """Image artifact extracted from a document page."""

    data: bytes
    format: str
    image_index: int
    page_number: int | None
    width: int | None
    height: int | None
    colorspace: str | None
    bits_per_component: int | None
    is_mask: bool
    description: str | None
    ocr_result: ExtractionResult | None


class Metadata(TypedDict, total=False):
    """Strongly-typed metadata for extraction results.

    This TypedDict mirrors the Rust Metadata struct, providing type hints
    for the most common metadata fields. The actual data comes from the
    Rust core and may include additional custom fields from postprocessors.

    All fields are optional (total=False) since they depend on:
    - File format being extracted
    - Feature flags (e.g., PDF support)
    - Postprocessors enabled
    - Extraction configuration

    Format-specific fields are flattened at the root level. Use the format_type
    discriminator to determine which fields are present.

    Common fields:
        language: Document language (ISO 639-1 code)
        date: Document date (ISO 8601 format)
        subject: Document subject

    Discriminator:
        format_type: Format discriminator ("pdf", "excel", "email", etc.)

    Format-specific fields (flattened at root level):
        PDF fields (when format_type == "pdf"):
            title, authors, keywords, created_at, modified_at, created_by,
            producer, page_count, pdf_version, is_encrypted, width, height, summary

        Excel fields (when format_type == "excel"):
            sheet_count, sheet_names

        Email fields (when format_type == "email"):
            from_email, from_name, to_emails, cc_emails, bcc_emails,
            message_id, attachments

        PowerPoint fields (when format_type == "pptx"):
            author, description, fonts

        Archive fields (when format_type == "archive"):
            format, file_count, file_list, total_size, compressed_size

        Image fields (when format_type == "image"):
            exif

        XML fields (when format_type == "xml"):
            element_count, unique_elements

        Text/Markdown fields (when format_type == "text"):
            line_count, word_count, character_count, headers, links, code_blocks

        HTML fields (when format_type == "html"):
            canonical_url, base_href, language, text_direction, open_graph,
            twitter_card, meta_tags, html_headers, html_links, html_images,
            structured_data

        OCR fields (when format_type == "ocr"):
            psm, output_format, table_count, table_rows, table_cols

    Processing metadata:
        image_preprocessing: Image preprocessing metadata dict

    Structured data:
        json_schema: JSON schema dict for structured extraction

    Error handling:
        error: Error metadata dict for batch operations

    Custom fields:
        Any additional fields added by Python postprocessors (entity extraction,
        keyword extraction, etc.) will appear as top-level keys in the dict.

    Example:
        >>> result = extract_file("document.xml")
        >>> metadata: Metadata = result["metadata"]
        >>> if metadata.get("format_type") == "xml":
        ...     element_count = metadata["element_count"]
        ...     print(f"Elements: {element_count}")
        >>> if "entities" in metadata:  # Custom field from postprocessor
        ...     entities = metadata["entities"]
    """

    date: str
    subject: str

    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]

    title: str
    authors: list[str]
    keywords: list[str]
    created_at: str
    modified_at: str
    created_by: str
    producer: str
    page_count: int
    pdf_version: str
    is_encrypted: bool
    width: int
    height: int
    summary: str

    sheet_count: int
    sheet_names: list[str]

    from_email: str
    from_name: str
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str
    attachments: list[str]

    author: str
    description: str
    fonts: list[str]

    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int

    exif: dict[str, str]

    element_count: int
    unique_elements: list[str]

    line_count: int
    word_count: int
    character_count: int
    headers: list[str]
    links: list[tuple[str, str]]
    code_blocks: list[tuple[str, str]]

    canonical_url: str
    base_href: str
    language: str
    text_direction: str
    open_graph: dict[str, str]
    twitter_card: dict[str, str]
    meta_tags: dict[str, str]
    html_headers: list[HeaderMetadata]
    html_links: list[LinkMetadata]
    html_images: list[ImageMetadata]
    structured_data: list[StructuredData]

    psm: int
    output_format: str
    table_count: int
    table_rows: int
    table_cols: int

    image_preprocessing: ImagePreprocessingMetadata
    json_schema: dict[str, Any]
    error: ErrorMetadata


class Table(TypedDict):
    """Extracted table structure."""

    cells: list[list[str]]
    markdown: str
    page_number: int


class DjotContent(TypedDict, total=False):
    """Structured Djot document representation.

    Attributes:
        plain_text: Plain text representation for backward compatibility
        blocks: Structured block-level content (headings, paragraphs, lists, etc.)
        metadata: Metadata from YAML frontmatter
        tables: Extracted tables as structured data
        images: Extracted images with metadata
        links: Extracted links with URLs and titles
        footnotes: Footnote definitions
        attributes: Attributes mapped by element identifier
    """

    plain_text: str
    blocks: list[FormattedBlock]
    metadata: Metadata
    tables: list[Table]
    images: list[DjotImage]
    links: list[DjotLink]
    footnotes: list[Footnote]
    attributes: dict[str, Attributes]


class FormattedBlock(TypedDict, total=False):
    """Block-level element in a Djot document.

    Attributes:
        block_type: Type of block element
        level: Heading level (1-6) or nesting level for lists
        content: Text content for inline elements
        children: Child blocks for list items and containers
        attributes: HTML/CSS attributes (id, class, etc.)
    """

    block_type: str
    level: int | None
    content: str | None
    children: list[FormattedBlock] | None
    attributes: Attributes | None


class DjotImage(TypedDict, total=False):
    """Image with metadata in Djot document.

    Attributes:
        url: Image URL or reference
        alt: Alternative text
        title: Image title/caption
        attributes: Image attributes
    """

    url: str
    alt: str | None
    title: str | None
    attributes: Attributes | None


class DjotLink(TypedDict, total=False):
    """Link in Djot document.

    Attributes:
        url: Link URL
        text: Link text
        title: Link title
        link_type: Link type (internal, external, email, phone, footnote)
    """

    url: str
    text: str
    title: str | None
    link_type: str | None


class Footnote(TypedDict, total=False):
    """Footnote definition.

    Attributes:
        label: Footnote identifier
        content: Footnote content
    """

    label: str
    content: str


Attributes = dict[str, str | int | bool | list[str] | None]
"""HTML/CSS attributes for an element."""


class ExtractionResult(TypedDict):
    """Extraction result returned by all extraction functions.

    Attributes:
        content: Extracted text content
        mime_type: MIME type of the processed document
        metadata: Strongly-typed metadata (see Metadata TypedDict)
        tables: List of extracted tables
        detected_languages: List of detected language codes (ISO 639-1)
        chunks: Optional list of text chunks with embeddings and metadata
        images: Optional list of extracted images (with nested OCR results)
        pages: Optional list of per-page content when page extraction is enabled
        elements: Optional list of semantic elements when output_format='element_based'
        djot_content: Optional structured Djot content when output_format='djot'
    """

    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
    pages: list[PageContent] | None
    elements: list[Element] | None
    djot_content: DjotContent | None


__all__ = [
    "ArchiveMetadata",
    "Attributes",
    "BoundingBox",
    "Chunk",
    "ChunkMetadata",
    "DjotContent",
    "DjotImage",
    "DjotLink",
    "Element",
    "ElementMetadata",
    "ElementType",
    "EmailMetadata",
    "ErrorMetadata",
    "ExcelMetadata",
    "ExtractedImage",
    "ExtractionResult",
    "Footnote",
    "FormattedBlock",
    "HeaderMetadata",
    "HierarchicalBlock",
    "HtmlImageMetadata",
    "HtmlMetadata",
    "ImageMetadata",
    "ImagePreprocessingMetadata",
    "LinkMetadata",
    "Metadata",
    "OcrMetadata",
    "OutputFormat",
    "PageBoundary",
    "PageConfig",
    "PageContent",
    "PageHierarchy",
    "PageInfo",
    "PageStructure",
    "PageUnitType",
    "PdfMetadata",
    "PptxMetadata",
    "ResultFormat",
    "StructuredData",
    "Table",
    "TextMetadata",
    "XmlMetadata",
]
