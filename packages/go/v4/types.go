package kreuzberg

import "encoding/json"

// ExtractionResult mirrors the Rust ExtractionResult struct returned by the core API.
type ExtractionResult struct {
	Content           string           `json:"content"`
	MimeType          string           `json:"mime_type"`
	Metadata          Metadata         `json:"metadata"`
	Tables            []Table          `json:"tables"`
	DetectedLanguages []string         `json:"detected_languages,omitempty"`
	Chunks            []Chunk          `json:"chunks,omitempty"`
	Images            []ExtractedImage `json:"images,omitempty"`
	Pages             []PageContent    `json:"pages,omitempty"`
	Elements          []Element        `json:"elements,omitempty"`
	DjotContent       *DjotContent     `json:"djot_content,omitempty"`
}

// Table represents a detected table in the source document.
type Table struct {
	Cells      [][]string `json:"cells"`
	Markdown   string     `json:"markdown"`
	PageNumber int        `json:"page_number"`
}

// Chunk contains chunked content plus optional embeddings and metadata.
type Chunk struct {
	Content   string        `json:"content"`
	Embedding []float32     `json:"embedding,omitempty"`
	Metadata  ChunkMetadata `json:"metadata"`
}

// ChunkMetadata provides positional information for a chunk.
type ChunkMetadata struct {
	ByteStart   uint64   `json:"byte_start"`
	ByteEnd     uint64   `json:"byte_end"`
	TokenCount  *uint64  `json:"token_count,omitempty"`
	ChunkIndex  uint64   `json:"chunk_index"`
	TotalChunks uint64   `json:"total_chunks"`
	FirstPage   *uint64  `json:"first_page,omitempty"`
	LastPage    *uint64  `json:"last_page,omitempty"`
}

// ExtractedImage represents an extracted image, optionally with nested OCR results.
type ExtractedImage struct {
	Data             []byte            `json:"data"`
	Format           string            `json:"format"`
	ImageIndex       uint64            `json:"image_index"`
	PageNumber       *uint64           `json:"page_number,omitempty"`
	Width            *uint32           `json:"width,omitempty"`
	Height           *uint32           `json:"height,omitempty"`
	Colorspace       *string           `json:"colorspace,omitempty"`
	BitsPerComponent *uint32           `json:"bits_per_component,omitempty"`
	IsMask           bool              `json:"is_mask"`
	Description      *string           `json:"description,omitempty"`
	OCRResult        *ExtractionResult `json:"ocr_result,omitempty"`
}

// Metadata aggregates document metadata and format-specific payloads.
type Metadata struct {
	Title              *string                     `json:"title,omitempty"`
	Subject            *string                     `json:"subject,omitempty"`
	Authors            []string                    `json:"authors,omitempty"`
	Keywords           []string                    `json:"keywords,omitempty"`
	Language           *string                     `json:"language,omitempty"`
	CreatedAt          *string                     `json:"created_at,omitempty"`
	ModifiedAt         *string                     `json:"modified_at,omitempty"`
	CreatedBy          *string                     `json:"created_by,omitempty"`
	ModifiedBy         *string                     `json:"modified_by,omitempty"`
	Pages              *PageStructure              `json:"pages,omitempty"`
	Format             FormatMetadata              `json:"-"`
	ImagePreprocessing *ImagePreprocessingMetadata `json:"image_preprocessing,omitempty"`
	JSONSchema         json.RawMessage             `json:"json_schema,omitempty"`
	Error              *ErrorMetadata              `json:"error,omitempty"`
	Additional         map[string]json.RawMessage  `json:"-"`
}

// FormatMetadata represents the discriminated union of metadata formats.
type FormatMetadata struct {
	Type    FormatType
	Pdf     *PdfMetadata
	Excel   *ExcelMetadata
	Email   *EmailMetadata
	Pptx    *PptxMetadata
	Archive *ArchiveMetadata
	Image   *ImageMetadata
	XML     *XMLMetadata
	Text    *TextMetadata
	HTML    *HtmlMetadata
	OCR     *OcrMetadata
}

// FormatType enumerates supported metadata discriminators.
type FormatType string

const (
	FormatUnknown FormatType = ""
	FormatPDF     FormatType = "pdf"
	FormatExcel   FormatType = "excel"
	FormatEmail   FormatType = "email"
	FormatPPTX    FormatType = "pptx"
	FormatArchive FormatType = "archive"
	FormatImage   FormatType = "image"
	FormatXML     FormatType = "xml"
	FormatText    FormatType = "text"
	FormatHTML    FormatType = "html"
	FormatOCR     FormatType = "ocr"
)

// FormatType returns the discriminated format string.
func (m Metadata) FormatType() FormatType {
	return m.Format.Type
}

// PdfMetadata returns the PDF metadata if present.
func (m Metadata) PdfMetadata() (*PdfMetadata, bool) {
	return m.Format.Pdf, m.Format.Type == FormatPDF && m.Format.Pdf != nil
}

// ExcelMetadata returns the Excel metadata if present.
func (m Metadata) ExcelMetadata() (*ExcelMetadata, bool) {
	return m.Format.Excel, m.Format.Type == FormatExcel && m.Format.Excel != nil
}

// EmailMetadata returns the Email metadata if present.
func (m Metadata) EmailMetadata() (*EmailMetadata, bool) {
	return m.Format.Email, m.Format.Type == FormatEmail && m.Format.Email != nil
}

// PptxMetadata returns the PPTX metadata if present.
func (m Metadata) PptxMetadata() (*PptxMetadata, bool) {
	return m.Format.Pptx, m.Format.Type == FormatPPTX && m.Format.Pptx != nil
}

// ArchiveMetadata returns the archive metadata if present.
func (m Metadata) ArchiveMetadata() (*ArchiveMetadata, bool) {
	return m.Format.Archive, m.Format.Type == FormatArchive && m.Format.Archive != nil
}

// ImageMetadata returns the image metadata if present.
func (m Metadata) ImageMetadata() (*ImageMetadata, bool) {
	return m.Format.Image, m.Format.Type == FormatImage && m.Format.Image != nil
}

// XMLMetadata returns the XML metadata if present.
func (m Metadata) XMLMetadata() (*XMLMetadata, bool) {
	return m.Format.XML, m.Format.Type == FormatXML && m.Format.XML != nil
}

// TextMetadata returns the text metadata if present.
func (m Metadata) TextMetadata() (*TextMetadata, bool) {
	return m.Format.Text, m.Format.Type == FormatText && m.Format.Text != nil
}

// HTMLMetadata returns the HTML metadata if present.
func (m Metadata) HTMLMetadata() (*HtmlMetadata, bool) {
	return m.Format.HTML, m.Format.Type == FormatHTML && m.Format.HTML != nil
}

// OcrMetadata returns the OCR metadata if present.
func (m Metadata) OcrMetadata() (*OcrMetadata, bool) {
	return m.Format.OCR, m.Format.Type == FormatOCR && m.Format.OCR != nil
}

// PdfMetadata contains metadata extracted from PDF documents.
type PdfMetadata struct {
	Title       *string  `json:"title,omitempty"`
	Subject     *string  `json:"subject,omitempty"`
	Authors     []string `json:"authors,omitempty"`
	Keywords    []string `json:"keywords,omitempty"`
	CreatedAt   *string  `json:"created_at,omitempty"`
	ModifiedAt  *string  `json:"modified_at,omitempty"`
	CreatedBy   *string  `json:"created_by,omitempty"`
	Producer    *string  `json:"producer,omitempty"`
	PageCount   *int     `json:"page_count,omitempty"`
	PDFVersion  *string  `json:"pdf_version,omitempty"`
	IsEncrypted *bool    `json:"is_encrypted,omitempty"`
	Width       *int64   `json:"width,omitempty"`
	Height      *int64   `json:"height,omitempty"`
	Summary     *string  `json:"summary,omitempty"`
}

// ExcelMetadata lists sheets inside spreadsheet documents.
type ExcelMetadata struct {
	SheetCount uint64   `json:"sheet_count"`
	SheetNames []string `json:"sheet_names"`
}

// EmailMetadata captures envelope data for EML/MSG messages.
type EmailMetadata struct {
	FromEmail   *string  `json:"from_email,omitempty"`
	FromName    *string  `json:"from_name,omitempty"`
	ToEmails    []string `json:"to_emails"`
	CcEmails    []string `json:"cc_emails"`
	BccEmails   []string `json:"bcc_emails"`
	MessageID   *string  `json:"message_id,omitempty"`
	Attachments []string `json:"attachments"`
}

// ArchiveMetadata summarizes archive contents.
type ArchiveMetadata struct {
	Format         string    `json:"format"`
	FileCount      uint64    `json:"file_count"`
	FileList       []string  `json:"file_list"`
	TotalSize      uint64    `json:"total_size"`
	CompressedSize *uint64   `json:"compressed_size,omitempty"`
}

// ImageMetadata describes standalone image documents.
type ImageMetadata struct {
	Width  uint32            `json:"width"`
	Height uint32            `json:"height"`
	Format string            `json:"format"`
	EXIF   map[string]string `json:"exif"`
}

// XMLMetadata provides statistics for XML documents.
type XMLMetadata struct {
	ElementCount   uint64   `json:"element_count"`
	UniqueElements []string `json:"unique_elements"`
}

// TextMetadata contains counts for plain text and Markdown documents.
type TextMetadata struct {
	LineCount      uint64      `json:"line_count"`
	WordCount      uint64      `json:"word_count"`
	CharacterCount uint64      `json:"character_count"`
	Headers        []string    `json:"headers,omitempty"`
	Links          [][2]string `json:"links,omitempty"`
	CodeBlocks     [][2]string `json:"code_blocks,omitempty"`
}

// LinkType enumerates link classification types.
type LinkType string

const (
	LinkTypeAnchor   LinkType = "anchor"
	LinkTypeInternal LinkType = "internal"
	LinkTypeExternal LinkType = "external"
	LinkTypeEmail    LinkType = "email"
	LinkTypePhone    LinkType = "phone"
	LinkTypeOther    LinkType = "other"
)

// ImageType enumerates image source classification types.
type ImageType string

const (
	ImageTypeDataUri  ImageType = "data-uri"
	ImageTypeInlineSvg ImageType = "inline-svg"
	ImageTypeExternal ImageType = "external"
	ImageTypeRelative ImageType = "relative"
)

// TextDirection enumerates text direction types.
type TextDirection string

const (
	TextDirectionLTR  TextDirection = "ltr"
	TextDirectionRTL  TextDirection = "rtl"
	TextDirectionAuto TextDirection = "auto"
)

// StructuredDataType enumerates structured data format types.
type StructuredDataType string

const (
	StructuredDataTypeJSONLD  StructuredDataType = "json-ld"
	StructuredDataTypeMicrodata StructuredDataType = "microdata"
	StructuredDataTypeRDFa   StructuredDataType = "rdfa"
)

//revive:disable-next-line var-naming
type HtmlMetadata struct {
	Title          *string             `json:"title,omitempty"`
	Description    *string             `json:"description,omitempty"`
	Keywords       []string            `json:"keywords,omitempty"`
	Author         *string             `json:"author,omitempty"`
	CanonicalURL   *string             `json:"canonical_url,omitempty"`
	BaseHref       *string             `json:"base_href,omitempty"`
	Language       *string             `json:"language,omitempty"`
	TextDirection  *TextDirection      `json:"text_direction,omitempty"`
	OpenGraph      map[string]string   `json:"open_graph,omitempty"`
	TwitterCard    map[string]string   `json:"twitter_card,omitempty"`
	MetaTags       map[string]string   `json:"meta_tags,omitempty"`
	Headers        []HeaderMetadata    `json:"headers,omitempty"`
	Links          []LinkMetadata      `json:"links,omitempty"`
	Images         []HTMLImageMetadata `json:"images,omitempty"`
	StructuredData []StructuredData    `json:"structured_data,omitempty"`
}

// HeaderMetadata represents a heading element in HTML.
type HeaderMetadata struct {
	Level      uint8   `json:"level"`
	Text       string  `json:"text"`
	ID         *string `json:"id,omitempty"`
	Depth      uint64  `json:"depth"`
	HTMLOffset uint64  `json:"html_offset"`
}

// LinkMetadata represents a hyperlink in HTML.
type LinkMetadata struct {
	Href       string      `json:"href"`
	Text       string      `json:"text"`
	Title      *string     `json:"title,omitempty"`
	LinkType   LinkType    `json:"link_type"`
	Rel        []string    `json:"rel,omitempty"`
	Attributes [][2]string `json:"attributes,omitempty"`
}

// HTMLImageMetadata represents an image element in HTML.
type HTMLImageMetadata struct {
	Src        string       `json:"src"`
	Alt        *string      `json:"alt,omitempty"`
	Title      *string      `json:"title,omitempty"`
	Dimensions *[2]uint32   `json:"dimensions,omitempty"`
	ImageType  ImageType    `json:"image_type"`
	Attributes [][2]string  `json:"attributes,omitempty"`
}

// StructuredData represents structured data (JSON-LD, microdata, etc.) in HTML.
type StructuredData struct {
	DataType   StructuredDataType `json:"data_type"`
	RawJSON    string             `json:"raw_json"`
	SchemaType *string            `json:"schema_type,omitempty"`
}

// PptxMetadata summarizes slide decks.
type PptxMetadata struct {
	SlideCount uint64   `json:"slide_count"`
	SlideNames []string `json:"slide_names"`
}

// OcrMetadata records OCR settings/results associated with an extraction.
type OcrMetadata struct {
	Language     string `json:"language"`
	PSM          int    `json:"psm"`
	OutputFormat string `json:"output_format"`
	TableCount   int    `json:"table_count"`
	TableRows    *int   `json:"table_rows,omitempty"`
	TableCols    *int   `json:"table_cols,omitempty"`
}

// ImagePreprocessingMetadata tracks OCR preprocessing steps.
type ImagePreprocessingMetadata struct {
	OriginalDimensions [2]uint64     `json:"original_dimensions"`
	OriginalDPI        [2]float64    `json:"original_dpi"`
	TargetDPI          int32         `json:"target_dpi"`
	ScaleFactor        float64       `json:"scale_factor"`
	AutoAdjusted       bool          `json:"auto_adjusted"`
	FinalDPI           int32         `json:"final_dpi"`
	NewDimensions      *[2]uint64    `json:"new_dimensions,omitempty"`
	ResampleMethod     string        `json:"resample_method"`
	DimensionClamped   bool          `json:"dimension_clamped"`
	CalculatedDPI      *int32        `json:"calculated_dpi,omitempty"`
	SkippedResize      bool          `json:"skipped_resize"`
	ResizeError        *string       `json:"resize_error,omitempty"`
}

// ErrorMetadata describes failures in batch operations.
type ErrorMetadata struct {
	ErrorType string `json:"error_type"`
	Message   string `json:"message"`
}

// PageUnitType enumerates the types of paginated units in documents.
type PageUnitType string

const (
	PageUnitTypePage  PageUnitType = "page"
	PageUnitTypeSlide PageUnitType = "slide"
	PageUnitTypeSheet PageUnitType = "sheet"
)

// PageBoundary marks byte offset boundaries for a page in the extracted content.
type PageBoundary struct {
	ByteStart  uint64 `json:"byte_start"`
	ByteEnd    uint64 `json:"byte_end"`
	PageNumber uint64 `json:"page_number"`
}

// PageInfo provides metadata about an individual page/slide/sheet.
type PageInfo struct {
	Number     uint64      `json:"number"`
	Title      *string     `json:"title,omitempty"`
	Dimensions *[2]float64 `json:"dimensions,omitempty"`
	ImageCount *uint64     `json:"image_count,omitempty"`
	TableCount *uint64     `json:"table_count,omitempty"`
	Hidden     *bool       `json:"hidden,omitempty"`
}

// PageStructure describes the page/slide/sheet structure of a document.
type PageStructure struct {
	TotalCount uint64         `json:"total_count"`
	UnitType   PageUnitType   `json:"unit_type"`
	Boundaries []PageBoundary `json:"boundaries,omitempty"`
	Pages      []PageInfo     `json:"pages,omitempty"`
}

// HierarchicalBlock represents a text block with hierarchy level assignment.
type HierarchicalBlock struct {
	Text     string      `json:"text"`
	FontSize float32     `json:"font_size"`
	Level    string      `json:"level"`
	Bbox     *[4]float32 `json:"bbox,omitempty"`
}

// PageHierarchy contains heading levels and block information for a page.
type PageHierarchy struct {
	BlockCount uint64              `json:"block_count"`
	Blocks     []HierarchicalBlock `json:"blocks,omitempty"`
}

// PageContent represents extracted content for a single page.
type PageContent struct {
	PageNumber uint64           `json:"page_number"`
	Content    string           `json:"content"`
	Tables     []Table          `json:"tables,omitempty"`
	Images     []ExtractedImage `json:"images,omitempty"`
	Hierarchy  *PageHierarchy   `json:"hierarchy,omitempty"`
}

// ElementType defines semantic classification for extracted elements.
type ElementType string

const (
	// ElementTypeTitle marks a document title element.
	ElementTypeTitle ElementType = "title"
	// ElementTypeNarrativeText marks main narrative text body.
	ElementTypeNarrativeText ElementType = "narrative_text"
	// ElementTypeHeading marks a section heading.
	ElementTypeHeading ElementType = "heading"
	// ElementTypeListItem marks a list item (bullet, numbered, etc.).
	ElementTypeListItem ElementType = "list_item"
	// ElementTypeTable marks a table element.
	ElementTypeTable ElementType = "table"
	// ElementTypeImage marks an image element.
	ElementTypeImage ElementType = "image"
	// ElementTypePageBreak marks a page break marker.
	ElementTypePageBreak ElementType = "page_break"
	// ElementTypeCodeBlock marks a code block.
	ElementTypeCodeBlock ElementType = "code_block"
	// ElementTypeBlockQuote marks a block quote.
	ElementTypeBlockQuote ElementType = "block_quote"
	// ElementTypeFooter marks footer text.
	ElementTypeFooter ElementType = "footer"
	// ElementTypeHeader marks header text.
	ElementTypeHeader ElementType = "header"
)

// BoundingBox represents bounding box coordinates for element positioning.
type BoundingBox struct {
	// X0 is the left x-coordinate.
	X0 float64 `json:"x0"`
	// Y0 is the bottom y-coordinate.
	Y0 float64 `json:"y0"`
	// X1 is the right x-coordinate.
	X1 float64 `json:"x1"`
	// Y1 is the top y-coordinate.
	Y1 float64 `json:"y1"`
}

// ElementMetadata contains metadata for a semantic element.
type ElementMetadata struct {
	// PageNumber is the 1-indexed page number.
	PageNumber *uint64 `json:"page_number,omitempty"`
	// Filename is the source filename or document name.
	Filename *string `json:"filename,omitempty"`
	// Coordinates contains bounding box coordinates if available.
	Coordinates *BoundingBox `json:"coordinates,omitempty"`
	// ElementIndex is the position index in the element sequence.
	ElementIndex *uint64 `json:"element_index,omitempty"`
	// Additional contains custom metadata fields.
	Additional map[string]string `json:"additional,omitempty"`
}

// Element represents a semantic element extracted from a document.
//
// It combines semantic classification, unique identification, and metadata
// for tracking origin and position within the source document.
// This type supports Unstructured.io element format when output_format='element_based'.
type Element struct {
	// ElementID is a unique element identifier (deterministic hash-based ID).
	ElementID string `json:"element_id"`
	// ElementType is the semantic type classification of the element.
	ElementType ElementType `json:"element_type"`
	// Text is the content string of the element.
	Text string `json:"text"`
	// Metadata contains element metadata including page number, coordinates, etc.
	Metadata ElementMetadata `json:"metadata"`
}

// DjotAttributeEntry represents a single element identifier to attributes mapping.
// Mirrors Rust's (String, Attributes) tuple in Vec<(String, Attributes)>.
type DjotAttributeEntry struct {
	// Name is the element identifier.
	Name string `json:"name"`
	// Attrs contains the element attributes.
	Attrs Attributes `json:"attrs"`
}

// DjotContent represents a comprehensive Djot document structure with semantic preservation.
// This type captures the full richness of Djot markup, including block-level structures,
// inline formatting, attributes, links, images, footnotes, and math expressions.
type DjotContent struct {
	// PlainText is plain text representation for backwards compatibility.
	PlainText string `json:"plain_text"`
	// Blocks contains structured block-level content.
	Blocks []FormattedBlock `json:"blocks"`
	// Metadata contains metadata from YAML frontmatter (required, non-optional).
	Metadata Metadata `json:"metadata"`
	// Tables contains extracted tables as structured data.
	Tables []Table `json:"tables,omitempty"`
	// Images contains extracted images with metadata.
	Images []DjotImage `json:"images,omitempty"`
	// Links contains extracted links with URLs.
	Links []DjotLink `json:"links,omitempty"`
	// Footnotes contains footnote definitions.
	Footnotes []Footnote `json:"footnotes,omitempty"`
	// Attributes maps element identifiers to their attribute sets.
	Attributes []DjotAttributeEntry `json:"attributes,omitempty"`
}

// FormattedBlock represents a block-level element in a Djot document.
// It represents structural elements like headings, paragraphs, lists, code blocks, etc.
type FormattedBlock struct {
	// BlockType is the type of block element.
	BlockType BlockType `json:"block_type"`
	// Level is the heading level (1-6) for headings, or nesting level for lists (optional).
	Level *uint64 `json:"level,omitempty"`
	// InlineContent contains inline content within the block.
	InlineContent []InlineElement `json:"inline_content"`
	// Attributes contains element attributes (classes, IDs, key-value pairs).
	Attributes *Attributes `json:"attributes,omitempty"`
	// Language is the language identifier for code blocks.
	Language *string `json:"language,omitempty"`
	// Code is the raw code content for code blocks.
	Code *string `json:"code,omitempty"`
	// Children contains nested blocks for containers (blockquotes, list items, divs).
	Children []FormattedBlock `json:"children,omitempty"`
}

// BlockType represents the types of block-level elements in Djot.
type BlockType string

const (
	BlockTypeParagraph          BlockType = "paragraph"
	BlockTypeHeading            BlockType = "heading"
	BlockTypeBlockquote         BlockType = "blockquote"
	BlockTypeCodeBlock          BlockType = "code_block"
	BlockTypeListItem           BlockType = "list_item"
	BlockTypeOrderedList        BlockType = "ordered_list"
	BlockTypeBulletList         BlockType = "bullet_list"
	BlockTypeTaskList           BlockType = "task_list"
	BlockTypeDefinitionList     BlockType = "definition_list"
	BlockTypeDefinitionTerm     BlockType = "definition_term"
	BlockTypeDefinitionDesc     BlockType = "definition_description"
	BlockTypeDiv                BlockType = "div"
	BlockTypeSection            BlockType = "section"
	BlockTypeThematicBreak      BlockType = "thematic_break"
	BlockTypeRawBlock           BlockType = "raw_block"
	BlockTypeMathDisplay        BlockType = "math_display"
)

// InlineElement represents an inline element within a block.
// It represents text with formatting, links, images, etc.
type InlineElement struct {
	// ElementType is the type of inline element.
	ElementType InlineType `json:"element_type"`
	// Content is the text content.
	Content string `json:"content"`
	// Attributes contains element attributes.
	Attributes *Attributes `json:"attributes,omitempty"`
	// Metadata contains additional metadata (e.g., href for links, src/alt for images).
	Metadata map[string]string `json:"metadata,omitempty"`
}

// InlineType represents the types of inline elements in Djot.
type InlineType string

const (
	InlineTypeText         InlineType = "text"
	InlineTypeStrong       InlineType = "strong"
	InlineTypeEmphasis     InlineType = "emphasis"
	InlineTypeHighlight    InlineType = "highlight"
	InlineTypeSubscript    InlineType = "subscript"
	InlineTypeSuperscript  InlineType = "superscript"
	InlineTypeInsert       InlineType = "insert"
	InlineTypeDelete       InlineType = "delete"
	InlineTypeCode         InlineType = "code"
	InlineTypeLink         InlineType = "link"
	InlineTypeImage        InlineType = "image"
	InlineTypeSpan         InlineType = "span"
	InlineTypeMath         InlineType = "math"
	InlineTypeRawInline    InlineType = "raw_inline"
	InlineTypeFootnoteRef  InlineType = "footnote_ref"
	InlineTypeSymbol       InlineType = "symbol"
)

// Attributes represents element attributes in Djot.
// It represents the attributes attached to elements using {.class #id key="value"} syntax.
type Attributes struct {
	// ID is the element ID (#identifier).
	ID *string `json:"id,omitempty"`
	// Classes contains CSS classes (.class1 .class2).
	Classes []string `json:"classes,omitempty"`
	// KeyValues contains key-value pairs (key="value").
	KeyValues [][2]string `json:"key_values,omitempty"`
}

// DjotImage represents an image element in Djot.
type DjotImage struct {
	// Src is the image source URL or path.
	Src string `json:"src"`
	// Alt is the alternative text.
	Alt string `json:"alt"`
	// Title is the optional title.
	Title *string `json:"title,omitempty"`
	// Attributes contains element attributes.
	Attributes *Attributes `json:"attributes,omitempty"`
}

// DjotLink represents a link element in Djot.
type DjotLink struct {
	// URL is the link URL.
	URL string `json:"url"`
	// Text is the link text content.
	Text string `json:"text"`
	// Title is the optional title.
	Title *string `json:"title,omitempty"`
	// Attributes contains element attributes.
	Attributes *Attributes `json:"attributes,omitempty"`
}

// Footnote represents a footnote in Djot.
type Footnote struct {
	// Label is the footnote label.
	Label string `json:"label"`
	// Content contains footnote content blocks.
	Content []FormattedBlock `json:"content"`
}
