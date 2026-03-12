# Type Reference

Complete type definitions and documentation for Kreuzberg across all language bindings.

## ExtractionResult

Primary extraction result containing document content, metadata, and structured data elements. All extraction operations return this unified type with format-agnostic content and format-specific metadata.

### Rust

```rust title="extraction_result.rs"
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: Cow<'static, str>,  // MIME type string (serializes as String)
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<Chunk>>,
    pub images: Option<Vec<ExtractedImage>>,
    pub pages: Option<Vec<PageContent>>,
    pub elements: Option<Vec<Element>>,
    pub djot_content: Option<DjotContent>,
    pub ocr_elements: Option<Vec<OcrElement>>,
    pub document: Option<DocumentStructure>,
}
```

### Python

```python title="extraction_result.py"
class ExtractionResult(TypedDict):
    """Main result containing extracted content, metadata, and structured data."""
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
    extracted_keywords: list[ExtractedKeyword] | None
    quality_score: float | None
    processing_warnings: list[ProcessingWarning]
    djot_content: DjotContent | None
    pages: list[PageContent] | None
    elements: list[Element] | None
    document: DocumentStructure | None
```

### TypeScript

```typescript title="extraction_result.ts"
export interface ExtractionResult {
  content: string;
  mimeType: string;
  metadata: Metadata;
  tables: Table[];
  detectedLanguages: string[] | null;
  chunks: Chunk[] | null;
  images: ExtractedImage[] | null;
  djotContent?: DjotContent;
  pages?: PageContent[];
  elements?: Element[];
  document?: DocumentStructure;
}
```

### Ruby

```ruby title="extraction_result.rb"
class Kreuzberg::Result
    attr_reader :content, :mime_type, :metadata, :tables
    attr_reader :detected_languages, :chunks, :images, :extracted_keywords, :quality_score, :processing_warnings
    attr_reader :djot_content, :pages, :elements, :document
end
```

### Java

```java title="ExtractionResult.java"
public record ExtractionResult(
    String content,
    String mimeType,
    Metadata metadata,
    List<Table> tables,
    List<String> detectedLanguages,
    List<Chunk> chunks,
    List<ExtractedImage> images,
    List<ExtractedKeyword> extractedKeywords,
    Double qualityScore,
    List<ProcessingWarning> processingWarnings,
    DjotContent djotContent,
    List<PageContent> pages,
    List<Element> elements,
    DocumentStructure document
) {}
```

### Go

```go title="extraction_result.go"
type ExtractionResult struct {
    Content             string              `json:"content"`
    MimeType            string              `json:"mime_type"`
    Metadata            Metadata            `json:"metadata"`
    Tables              []Table             `json:"tables"`
    DetectedLanguages   []string            `json:"detected_languages,omitempty"`
    Chunks              []Chunk             `json:"chunks,omitempty"`
    Images              []ExtractedImage    `json:"images,omitempty"`
    ExtractedKeywords   []ExtractedKeyword  `json:"extracted_keywords,omitempty"`
    QualityScore        *float64            `json:"quality_score,omitempty"`
    ProcessingWarnings  []ProcessingWarning `json:"processing_warnings"`
    DjotContent         *DjotContent        `json:"djot_content,omitempty"`
    Pages               []PageContent       `json:"pages,omitempty"`
    Elements            []Element           `json:"elements,omitempty"`
    Document            *DocumentStructure  `json:"document,omitempty"`
}
```

## Metadata

Document metadata with discriminated union pattern. The `format_type` field determines which format-specific fields are populated, enabling type-safe access to PDF, Excel, Email, and other format-specific metadata.

### Rust

```rust title="metadata.rs"
pub struct Metadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub authors: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub language: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub document_version: Option<String>,
    pub abstract_text: Option<String>,
    pub output_format: Option<String>,
    pub pages: Option<PageStructure>,
    pub format: Option<FormatMetadata>,
    pub image_preprocessing: Option<ImagePreprocessingMetadata>,
    pub json_schema: Option<serde_json::Value>,
    pub error: Option<ErrorMetadata>,
    pub extraction_duration_ms: Option<u64>,
    pub additional: HashMap<String, serde_json::Value>,
}

pub enum FormatMetadata {
    #[cfg(feature = "pdf")]
    Pdf(PdfMetadata),
    Excel(ExcelMetadata),
    Email(EmailMetadata),
    Pptx(PptxMetadata),
    Archive(ArchiveMetadata),
    Image(ImageMetadata),
    Xml(XmlMetadata),
    Text(TextMetadata),
    Html(Box<HtmlMetadata>),
    Ocr(OcrMetadata),
}
```

### Python

```python title="metadata.py"
class Metadata(TypedDict, total=False):
    """Document metadata with format-specific fields and processing info."""
    title: str | None
    subject: str | None
    authors: list[str] | None
    keywords: list[str] | None
    category: str | None
    tags: list[str] | None
    language: str | None
    created_at: str | None
    modified_at: str | None
    created_by: str | None
    modified_by: str | None
    document_version: str | None
    abstract_text: str | None
    output_format: str | None
    pages: PageStructure | None
    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]
    # Format-specific fields are included at root level based on format_type
    image_preprocessing: ImagePreprocessingMetadata | None
    json_schema: dict[str, Any] | None
    error: ErrorMetadata | None
```

### TypeScript

```typescript title="metadata.ts"
export interface Metadata {
  title?: string | null;
  subject?: string | null;
  authors?: string[] | null;
  keywords?: string[] | null;
  language?: string | null;
  createdAt?: string | null;
  modifiedAt?: string | null;
  createdBy?: string | null;
  modifiedBy?: string | null;
  pages?: PageStructure | null;
  format_type?:
    | "pdf"
    | "excel"
    | "email"
    | "pptx"
    | "archive"
    | "image"
    | "xml"
    | "text"
    | "html"
    | "ocr";
  // Format-specific fields are included at root level based on format_type
  image_preprocessing?: ImagePreprocessingMetadata | null;
  json_schema?: Record<string, unknown> | null;
  error?: ErrorMetadata | null;
  [key: string]: any;
}
```

### Ruby

```ruby title="metadata.rb"
# Metadata is returned as a Hash from the native extension
result.metadata  # Hash with string keys and mixed values
# Check format_type to determine which format-specific fields are available
result.metadata["format_type"]  # "pdf", "excel", "email", etc.
```

### Java

```java title="Metadata.java"
public final class Metadata {
    private final Optional<String> title;
    private final Optional<String> subject;
    private final Optional<List<String>> authors;
    private final Optional<List<String>> keywords;
    private final Optional<String> category;
    private final Optional<List<String>> tags;
    private final Optional<String> language;
    private final Optional<String> createdAt;
    private final Optional<String> modifiedAt;
    private final Optional<String> createdBy;
    private final Optional<String> modifiedBy;
    private final Optional<String> documentVersion;
    private final Optional<String> abstractText;
    private final Optional<String> outputFormat;
    private final Optional<PageStructure> pages;
    private final Optional<FormatMetadata> format;
    private final Optional<ImagePreprocessingMetadata> imagePreprocessing;
    private final Optional<Map<String, Object>> jsonSchema;
    private final Optional<ErrorMetadata> error;
}

public final class FormatMetadata {
    private final FormatType type;
    private final Optional<PdfMetadata> pdf;
    private final Optional<ExcelMetadata> excel;
    private final Optional<EmailMetadata> email;
    // Additional Optional fields for each supported format type
}
```

### Go

```go title="metadata.go"
type Metadata struct {
    Title              *string                     `json:"title,omitempty"`
    Subject            *string                     `json:"subject,omitempty"`
    Authors            []string                    `json:"authors,omitempty"`
    Keywords           []string                    `json:"keywords,omitempty"`
    Category           *string                     `json:"category,omitempty"`
    Tags               []string                    `json:"tags,omitempty"`
    Language           *string                     `json:"language,omitempty"`
    CreatedAt          *string                     `json:"created_at,omitempty"`
    ModifiedAt         *string                     `json:"modified_at,omitempty"`
    CreatedBy          *string                     `json:"created_by,omitempty"`
    ModifiedBy         *string                     `json:"modified_by,omitempty"`
    DocumentVersion    *string                     `json:"document_version,omitempty"`
    AbstractText       *string                     `json:"abstract_text,omitempty"`
    OutputFormat       *string                     `json:"output_format,omitempty"`
    Pages              *PageStructure              `json:"pages,omitempty"`
    Format             FormatMetadata              `json:"-"`
    ImagePreprocessing *ImagePreprocessingMetadata `json:"image_preprocessing,omitempty"`
    JSONSchema         json.RawMessage             `json:"json_schema,omitempty"`
    Error              *ErrorMetadata              `json:"error,omitempty"`
    Additional         map[string]json.RawMessage `json:"-"`
}

type FormatMetadata struct {
    Type    FormatType
    Pdf     *PdfMetadata
    Excel   *ExcelMetadata
    Email   *EmailMetadata
    // Additional pointer fields for each supported format type
}
```

## ProcessingWarning

Non-fatal warning that occurred during extraction pipeline processing.

### Rust

```rust title="processing_warning.rs"
pub struct ProcessingWarning {
    pub source: String,      // Component that generated the warning
    pub message: String,     // Human-readable warning message
}
```

### Python

```python title="processing_warning.py"
class ProcessingWarning(TypedDict):
    source: str      # Component that generated the warning
    message: str     # Human-readable warning message
```

### TypeScript

```typescript title="processing_warning.ts"
export interface ProcessingWarning {
    source: string;      // Component that generated the warning
    message: string;     // Human-readable warning message
}
```

### Java

```java title="ProcessingWarning.java"
public record ProcessingWarning(
    String source,       // Component that generated the warning
    String message       // Human-readable warning message
) {}
```

### Go

```go title="processing_warning.go"
type ProcessingWarning struct {
    Source  string `json:"source"`
    Message string `json:"message"`
}
```

## ExtractedKeyword

Extracted keyword with score, algorithm information, and positions.

### Rust

```rust title="extracted_keyword.rs"
pub struct ExtractedKeyword {
    pub text: String,                    // Keyword text
    pub score: f32,                      // Score from algorithm (0.0-1.0 range, normalize for comparing algorithms)
    pub algorithm: String,               // Algorithm used ("YAKE", "RAKE", etc.)
    pub positions: Option<Vec<usize>>,   // Character positions in content
}
```

### Python

```python title="extracted_keyword.py"
class ExtractedKeyword(TypedDict):
    text: str                        # Keyword text
    score: float                     # Score from algorithm
    algorithm: str                   # Algorithm used ("YAKE", "RAKE", etc.)
    positions: list[int] | None      # Character positions in content
```

### TypeScript

```typescript title="extracted_keyword.ts"
export interface ExtractedKeyword {
    text: string;                    // Keyword text
    score: number;                   // Score from algorithm
    algorithm: string;               // Algorithm used ("YAKE", "RAKE", etc.)
    positions?: number[];            // Character positions in content
}
```

### Java

```java title="ExtractedKeyword.java"
public record ExtractedKeyword(
    String text,                     // Keyword text
    float score,                     // Score from algorithm
    String algorithm,                // Algorithm used ("YAKE", "RAKE", etc.)
    List<Integer> positions          // Character positions in content
) {}
```

### Go

```go title="extracted_keyword.go"
type ExtractedKeyword struct {
    Text      string  `json:"text"`
    Score     float32 `json:"score"`
    Algorithm string  `json:"algorithm"`
    Positions []int64 `json:"positions,omitempty"`
}
```

### Metadata.pages Field

Contains page structure information when page tracking is available. This field provides detailed boundaries and metadata for individual pages/slides/sheets within multi-page documents.

**Type**: `Option<PageStructure>` (Rust), `PageStructure | None` (Python), `PageStructure | null` (TypeScript), `Optional<PageStructure>` (Java), `*PageStructure` (Go)

**When populated**: Only when the document format supports page tracking (PDF, PPTX, DOCX, XLSX) and extraction is successful.

**Available fields**:

- `total_count`: Total number of pages/slides/sheets in the document
- `unit_type`: Type of paginated unit ("page", "slide", or "sheet")
- `boundaries`: Byte offset boundaries for each page (enables O(1) lookups from byte positions to page numbers)
- `pages`: Detailed per-page metadata including dimensions, titles, and content counts

**Example usage**:

```rust title="Rust - Accessing Page Structure"
if let Some(page_structure) = metadata.pages {
    println!("Document has {} pages", page_structure.total_count);
    if let Some(boundaries) = page_structure.boundaries {
        for boundary in boundaries {
            println!("Page {}: bytes {} to {}", boundary.page_number, boundary.byte_start, boundary.byte_end);
        }
    }
}
```

```python title="Python - Accessing Page Structure"
if metadata.get("pages"):
    page_structure = metadata["pages"]
    print(f"Document has {page_structure['total_count']} pages")
    if page_structure.get("boundaries"):
        for boundary in page_structure["boundaries"]:
            print(f"Page {boundary['page_number']}: bytes {boundary['byte_start']}-{boundary['byte_end']}")
```

```typescript title="TypeScript - Accessing Page Structure"
if (metadata.pages) {
  console.log(`Document has ${metadata.pages.totalCount} pages`);
  if (metadata.pages.boundaries) {
    for (const boundary of metadata.pages.boundaries) {
      console.log(
        `Page ${boundary.pageNumber}: bytes ${boundary.byteStart}-${boundary.byteEnd}`,
      );
    }
  }
}
```

```java title="Java - Accessing Page Structure"
metadata.pages().ifPresent(pageStructure -> {
    System.out.println("Document has " + pageStructure.getTotalCount() + " pages");
    pageStructure.getBoundaries().ifPresent(boundaries -> {
        for (PageBoundary boundary : boundaries) {
            System.out.println("Page " + boundary.pageNumber() + ": bytes " +
                boundary.byteStart() + "-" + boundary.byteEnd());
        }
    });
});
```

```go title="Go - Accessing Page Structure"
if metadata.Pages != nil {
    fmt.Printf("Document has %d pages\n", metadata.Pages.TotalCount)
    if metadata.Pages.Boundaries != nil {
        for _, boundary := range metadata.Pages.Boundaries {
            fmt.Printf("Page %d: bytes %d-%d\n", boundary.PageNumber, boundary.ByteStart, boundary.ByteEnd)
        }
    }
}
```

## PageStructure

Unified representation of page/slide/sheet structure with byte-accurate boundaries. Tracks the logical structure of multi-page documents, enabling precise page-to-content mapping and efficient chunk-to-page lookups.

### Rust

```rust title="page_structure.rs"
pub struct PageStructure {
    pub total_count: usize,
    pub unit_type: PageUnitType,
    pub boundaries: Option<Vec<PageBoundary>>,
    pub pages: Option<Vec<PageInfo>>,
}
```

### Python

```python title="page_structure.py"
class PageStructure(TypedDict, total=False):
    total_count: int
    unit_type: str  # "page", "slide", "sheet"
    boundaries: list[PageBoundary] | None
    pages: list[PageInfo] | None
```

### TypeScript

```typescript title="page_structure.ts"
interface PageStructure {
  totalCount: number;
  unitType: "page" | "slide" | "sheet";
  boundaries?: PageBoundary[];
  pages?: PageInfo[];
}
```

### Ruby

```ruby title="page_structure.rb"
class PageStructure < Dry::Struct
  attribute :total_count, Types::Integer
  attribute :unit_type, Types::String.enum("page", "slide", "sheet")
  attribute :boundaries, Types::Array.of(PageBoundary).optional
  attribute :pages, Types::Array.of(PageInfo).optional
end
```

### Java

```java title="PageStructure.java"
public final class PageStructure {
    private final long totalCount;
    private final PageUnitType unitType;
    private final List<PageBoundary> boundaries;
    private final List<PageInfo> pages;

    public Optional<List<PageBoundary>> getBoundaries() { }
    public Optional<List<PageInfo>> getPages() { }
}
```

### Go

```go title="page_structure.go"
type PageStructure struct {
    TotalCount int              `json:"total_count"`
    UnitType   string           `json:"unit_type"`
    Boundaries []PageBoundary   `json:"boundaries,omitempty"`
    Pages      []PageInfo       `json:"pages,omitempty"`
}
```

### C#

```csharp title="PageStructure.cs"
public record PageStructure
{
    public required int TotalCount { get; init; }
    public required string UnitType { get; init; }
    public List<PageBoundary>? Boundaries { get; init; }
    public List<PageInfo>? Pages { get; init; }
}
```

**Fields:**

- `total_count`: Total number of pages/slides/sheets
- `unit_type`: Distinction between Page/Slide/Sheet
- `boundaries`: Byte offset ranges for each page (enables O(1) lookups)
- `pages`: Per-page metadata (dimensions, counts, visibility)

## PageBoundary

Byte offset range for a single page/slide/sheet. Enables O(1) page lookups and precise chunk-to-page mapping.

WARNING: Byte offsets are UTF-8 safe boundaries. Do not use them as character indices.

### Rust

```rust title="page_boundary.rs"
pub struct PageBoundary {
    pub byte_start: usize,
    pub byte_end: usize,
    pub page_number: usize,
}
```

### Python

```python title="page_boundary.py"
class PageBoundary(TypedDict):
    byte_start: int
    byte_end: int
    page_number: int
```

### TypeScript

```typescript title="page_boundary.ts"
interface PageBoundary {
  byteStart: number;
  byteEnd: number;
  pageNumber: number;
}
```

### Ruby

```ruby title="page_boundary.rb"
class PageBoundary < Dry::Struct
  attribute :byte_start, Types::Integer
  attribute :byte_end, Types::Integer
  attribute :page_number, Types::Integer
end
```

### Java

```java title="PageBoundary.java"
public record PageBoundary(
    long byteStart,
    long byteEnd,
    long pageNumber
) {}
```

### Go

```go title="page_boundary.go"
type PageBoundary struct {
    ByteStart  int `json:"byte_start"`
    ByteEnd    int `json:"byte_end"`
    PageNumber int `json:"page_number"`
}
```

### C#

```csharp title="PageBoundary.cs"
public record PageBoundary
{
    public required int ByteStart { get; init; }
    public required int ByteEnd { get; init; }
    public required int PageNumber { get; init; }
}
```

**Fields:**

- `byte_start`: UTF-8 byte offset (inclusive)
- `byte_end`: UTF-8 byte offset (exclusive)
- `page_number`: 1-indexed page number

## PageInfo

Detailed per-page metadata. Contains format-specific metadata for individual pages/slides/sheets.

### Rust

```rust title="page_info.rs"
pub struct PageInfo {
    pub number: usize,
    pub title: Option<String>,
    pub dimensions: Option<(f64, f64)>,
    pub image_count: Option<usize>,
    pub table_count: Option<usize>,
    pub hidden: Option<bool>,
    pub is_blank: Option<bool>,
}
```

### Python

```python title="page_info.py"
class PageInfo(TypedDict, total=False):
    number: int
    title: str | None
    dimensions: tuple[float, float] | None
    image_count: int | None
    table_count: int | None
    hidden: bool | None
    is_blank: bool | None
```

### TypeScript

```typescript title="page_info.ts"
interface PageInfo {
  number: number;
  title?: string;
  dimensions?: [number, number];
  imageCount?: number;
  tableCount?: number;
  hidden?: boolean;
  isBlank?: boolean;
}
```

### Ruby

```ruby title="page_info.rb"
class PageInfo < Dry::Struct
  attribute :number, Types::Integer
  attribute :title, Types::String.optional
  attribute :dimensions, Types::Array.of(Types::Float).optional
  attribute :image_count, Types::Integer.optional
  attribute :table_count, Types::Integer.optional
  attribute :hidden, Types::Bool.optional
  attribute :is_blank, Types::Bool.optional
end
```

### Java

```java title="PageInfo.java"
public record PageInfo(
    int number,
    Optional<String> title,
    Optional<double[]> dimensions,
    Optional<Integer> imageCount,
    Optional<Integer> tableCount,
    Optional<Boolean> hidden,
    Optional<Boolean> isBlank
) {}
```

### Go

```go title="page_info.go"
type PageInfo struct {
    Number      int       `json:"number"`
    Title       *string   `json:"title,omitempty"`
    Dimensions  []float64 `json:"dimensions,omitempty"`
    ImageCount  *int      `json:"image_count,omitempty"`
    TableCount  *int      `json:"table_count,omitempty"`
    Hidden      *bool     `json:"hidden,omitempty"`
    IsBlank     *bool     `json:"is_blank,omitempty"`
}
```

### C#

```csharp title="PageInfo.cs"
public record PageInfo
{
    public required int Number { get; init; }
    public string? Title { get; init; }
    public (double Width, double Height)? Dimensions { get; init; }
    public int? ImageCount { get; init; }
    public int? TableCount { get; init; }
    public bool? Hidden { get; init; }
    public bool? IsBlank { get; init; }
}
```

**Fields:**

- `number`: 1-indexed page number
- `title`: Page/slide title (PPTX)
- `dimensions`: Width and height in points (PDF, PPTX)
- `image_count`: Number of images on page
- `table_count`: Number of tables on page
- `hidden`: Whether page/slide is hidden (PPTX)
- `is_blank`: Whether the page contains no meaningful content (fewer than 3 non-whitespace characters and no tables or images)

## PageUnitType

Enum distinguishing page types across document formats.

### Rust

```rust title="page_unit_type.rs"
pub enum PageUnitType {
    Page,
    Slide,
    Sheet,
}
```

### Python

```python title="page_unit_type.py"
# String literal type
PageUnitType = Literal["page", "slide", "sheet"]
```

### TypeScript

```typescript title="page_unit_type.ts"
type PageUnitType = "page" | "slide" | "sheet";
```

### Ruby

```ruby title="page_unit_type.rb"
module PageUnitType
  PAGE = "page"
  SLIDE = "slide"
  SHEET = "sheet"
end
```

### Java

```java title="PageUnitType.java"
public enum PageUnitType {
    PAGE,
    SLIDE,
    SHEET
}
```

### Go

```go title="page_unit_type.go"
type PageUnitType string

const (
    PageUnitTypePage  PageUnitType = "page"
    PageUnitTypeSlide PageUnitType = "slide"
    PageUnitTypeSheet PageUnitType = "sheet"
)
```

### C#

```csharp title="PageUnitType.cs"
public enum PageUnitType
{
    Page,
    Slide,
    Sheet
}
```

**Values:**

- `Page`: Standard document pages (PDF, DOCX)
- `Slide`: Presentation slides (PPTX)
- `Sheet`: Spreadsheet sheets (XLSX)

## Format-Specific Metadata

### PDF Metadata

Document properties extracted from PDF files including title, author, creation dates, and page count. Available when `format_type == "pdf"`.

#### Rust

```rust title="pdf_metadata.rs"
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub page_count: Option<usize>,
}
```

#### Python

```python title="pdf_metadata.py"
class PdfMetadata(TypedDict, total=False):
    title: str | None
    author: str | None
    subject: str | None
    keywords: str | None
    creator: str | None
    producer: str | None
    creation_date: str | None
    modification_date: str | None
    page_count: int
```

#### TypeScript

```typescript title="pdf_metadata.ts"
export interface PdfMetadata {
  title?: string | null;
  author?: string | null;
  subject?: string | null;
  keywords?: string | null;
  creator?: string | null;
  producer?: string | null;
  creationDate?: string | null;
  modificationDate?: string | null;
  pageCount?: number;
}
```

#### Java

```java title="PdfMetadata.java"
public record PdfMetadata(
    Optional<String> title,
    Optional<String> author,
    Optional<String> subject,
    Optional<String> keywords,
    Optional<String> creator,
    Optional<String> producer,
    Optional<String> creationDate,
    Optional<String> modificationDate,
    Optional<Integer> pageCount
) {}
```

#### Go

```go title="pdf_metadata.go"
type PdfMetadata struct {
    Title            *string `json:"title,omitempty"`
    Author           *string `json:"author,omitempty"`
    Subject          *string `json:"subject,omitempty"`
    Keywords         []string `json:"keywords,omitempty"`
    Creator          *string `json:"creator,omitempty"`
    Producer         *string `json:"producer,omitempty"`
    CreatedAt        *string `json:"created_at,omitempty"`
    ModifiedAt       *string `json:"modified_at,omitempty"`
    PageCount        *int    `json:"page_count,omitempty"`
}
```

### Excel Metadata

Spreadsheet workbook information including sheet count and sheet names. Available when `format_type == "excel"`.

#### Rust

```rust title="excel_metadata.rs"
pub struct ExcelMetadata {
    pub sheet_count: usize,
    pub sheet_names: Vec<String>,
}
```

#### Python

```python title="excel_metadata.py"
class ExcelMetadata(TypedDict, total=False):
    sheet_count: int
    sheet_names: list[str]
```

#### TypeScript

```typescript title="excel_metadata.ts"
export interface ExcelMetadata {
  sheetCount?: number;
  sheetNames?: string[];
}
```

#### Java

```java title="ExcelMetadata.java"
public record ExcelMetadata(
    int sheetCount,
    List<String> sheetNames
) {}
```

#### Go

```go title="excel_metadata.go"
type ExcelMetadata struct {
    SheetCount int      `json:"sheet_count"`
    SheetNames []string `json:"sheet_names"`
}
```

### Email Metadata

Email message headers and recipient information including sender, recipients, message ID, and attachment lists. Available when `format_type == "email"`.

#### Rust

```rust title="email_metadata.rs"
pub struct EmailMetadata {
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub bcc_emails: Vec<String>,
    pub message_id: Option<String>,
    pub attachments: Vec<String>,
}
```

#### Python

```python title="email_metadata.py"
class EmailMetadata(TypedDict, total=False):
    from_email: str | None
    from_name: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str | None
    attachments: list[str]
```

#### TypeScript

```typescript title="email_metadata.ts"
export interface EmailMetadata {
  fromEmail?: string | null;
  fromName?: string | null;
  toEmails?: string[];
  ccEmails?: string[];
  bccEmails?: string[];
  messageId?: string | null;
  attachments?: string[];
}
```

#### Java

```java title="EmailMetadata.java"
public record EmailMetadata(
    Optional<String> fromEmail,
    Optional<String> fromName,
    List<String> toEmails,
    List<String> ccEmails,
    List<String> bccEmails,
    Optional<String> messageId,
    List<String> attachments
) {}
```

#### Go

```go title="email_metadata.go"
type EmailMetadata struct {
    FromEmail   *string  `json:"from_email,omitempty"`
    FromName    *string  `json:"from_name,omitempty"`
    ToEmails    []string `json:"to_emails"`
    CcEmails    []string `json:"cc_emails"`
    BccEmails   []string `json:"bcc_emails"`
    MessageID   *string  `json:"message_id,omitempty"`
    Attachments []string `json:"attachments"`
}
```

### Archive Metadata

Archive file properties including format type, file count, file list, and compression information. Available when `format_type == "archive"`.

#### Rust

```rust title="archive_metadata.rs"
pub struct ArchiveMetadata {
    pub format: String,
    pub file_count: usize,
    pub file_list: Vec<String>,
    pub total_size: usize,
    pub compressed_size: Option<usize>,
}
```

#### Python

```python title="archive_metadata.py"
class ArchiveMetadata(TypedDict, total=False):
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None
```

#### TypeScript

```typescript title="archive_metadata.ts"
export interface ArchiveMetadata {
  format?: string;
  fileCount?: number;
  fileList?: string[];
  totalSize?: number;
  compressedSize?: number | null;
}
```

#### Java

```java title="ArchiveMetadata.java"
public record ArchiveMetadata(
    String format,
    int fileCount,
    List<String> fileList,
    int totalSize,
    Optional<Integer> compressedSize
) {}
```

#### Go

```go title="archive_metadata.go"
type ArchiveMetadata struct {
    Format         string   `json:"format"`
    FileCount      int      `json:"file_count"`
    FileList       []string `json:"file_list"`
    TotalSize      int      `json:"total_size"`
    CompressedSize *int     `json:"compressed_size,omitempty"`
}
```

### Image Metadata

Image properties including dimensions, format type, and EXIF metadata extracted from image files. Available when `format_type == "image"`.

#### Rust

```rust title="image_metadata.rs"
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub exif: HashMap<String, String>,
}
```

#### Python

```python title="image_metadata.py"
class ImageMetadata(TypedDict, total=False):
    width: int
    height: int
    format: str
    exif: dict[str, str]
```

#### TypeScript

```typescript title="image_metadata.ts"
export interface ImageMetadata {
  width?: number;
  height?: number;
  format?: string;
  exif?: Record<string, string>;
}
```

#### Java

```java title="ImageMetadata.java"
public record ImageMetadata(
    int width,
    int height,
    String format,
    Map<String, String> exif
) {}
```

#### Go

```go title="image_metadata.go"
type ImageMetadata struct {
    Width  uint32            `json:"width"`
    Height uint32            `json:"height"`
    Format string            `json:"format"`
    EXIF   map[string]string `json:"exif"`
}
```

### HTML Metadata

Rich web page metadata including SEO tags, Open Graph fields, Twitter Card properties, structured data, and complex resource links. Available when `format_type == "html"`. Structured fields like headers, links, and images are represented as complex typed objects, not simple arrays of strings.

#### Rust

```rust title="html_metadata.rs"
pub struct HtmlMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub author: Option<String>,
    pub canonical_url: Option<String>,
    pub base_href: Option<String>,
    pub language: Option<String>,
    pub text_direction: Option<TextDirection>,
    pub open_graph: BTreeMap<String, String>,
    pub twitter_card: BTreeMap<String, String>,
    pub meta_tags: BTreeMap<String, String>,
    pub headers: Vec<HeaderMetadata>,
    pub links: Vec<LinkMetadata>,
    pub images: Vec<ImageMetadataType>,
    pub structured_data: Vec<StructuredData>,
}
```

#### Python

```python title="html_metadata.py"
class HtmlMetadata(TypedDict, total=False):
    title: str | None
    description: str | None
    keywords: list[str]
    author: str | None
    canonical_url: str | None
    base_href: str | None
    language: str | None
    text_direction: str | None
    open_graph: dict[str, str]
    twitter_card: dict[str, str]
    meta_tags: dict[str, str]
    headers: list[HeaderMetadata]
    links: list[LinkMetadata]
    images: list[ImageMetadataType]
    structured_data: list[StructuredData]
```

#### TypeScript

```typescript title="html_metadata.ts"
export interface HtmlMetadata {
  title?: string | null;
  description?: string | null;
  keywords: string[];
  author?: string | null;
  canonicalUrl?: string | null;
  baseHref?: string | null;
  language?: string | null;
  textDirection?: string | null;
  openGraph: Record<string, string>;
  twitterCard: Record<string, string>;
  metaTags: Record<string, string>;
  headers: HeaderMetadata[];
  links: LinkMetadata[];
  images: ImageMetadataType[];
  structuredData: StructuredData[];
}
```

#### Java

```java title="HtmlMetadata.java"
public record HtmlMetadata(
    Optional<String> title,
    Optional<String> description,
    List<String> keywords,
    Optional<String> author,
    Optional<String> canonicalUrl,
    Optional<String> baseHref,
    Optional<String> language,
    Optional<TextDirection> textDirection,
    Map<String, String> openGraph,
    Map<String, String> twitterCard,
    Map<String, String> metaTags,
    List<HeaderMetadata> headers,
    List<LinkMetadata> links,
    List<ImageMetadataType> images,
    List<StructuredData> structuredData
) {}
```

#### Go

```go title="html_metadata.go"
type HtmlMetadata struct {
    Title           *string                `json:"title,omitempty"`
    Description     *string                `json:"description,omitempty"`
    Keywords        []string               `json:"keywords"`
    Author          *string                `json:"author,omitempty"`
    CanonicalURL    *string                `json:"canonical_url,omitempty"`
    BaseHref        *string                `json:"base_href,omitempty"`
    Language        *string                `json:"language,omitempty"`
    TextDirection   *string                `json:"text_direction,omitempty"`
    OpenGraph       map[string]string      `json:"open_graph"`
    TwitterCard     map[string]string      `json:"twitter_card"`
    MetaTags        map[string]string      `json:"meta_tags"`
    Headers         []HeaderMetadata       `json:"headers"`
    Links           []LinkMetadata         `json:"links"`
    Images          []ImageMetadataType    `json:"images"`
    StructuredData  []StructuredData       `json:"structured_data"`
}
```

#### C#

```csharp title="HtmlMetadata.cs"
public record HtmlMetadata
{
    public string? Title { get; init; }
    public string? Description { get; init; }
    public List<string> Keywords { get; init; } = new();
    public string? Author { get; init; }
    public string? CanonicalUrl { get; init; }
    public string? BaseHref { get; init; }
    public string? Language { get; init; }
    public string? TextDirection { get; init; }
    public Dictionary<string, string> OpenGraph { get; init; } = new();
    public Dictionary<string, string> TwitterCard { get; init; } = new();
    public Dictionary<string, string> MetaTags { get; init; } = new();
    public List<HeaderMetadata> Headers { get; init; } = new();
    public List<LinkMetadata> Links { get; init; } = new();
    public List<ImageMetadataType> Images { get; init; } = new();
    public List<StructuredData> StructuredData { get; init; } = new();
}
```

## HeaderMetadata

Metadata for header elements (h1-h6) with hierarchy and positioning information.

### Rust

```rust title="header_metadata.rs"
pub struct HeaderMetadata {
    pub level: u8,
    pub text: String,
    pub id: Option<String>,
    pub depth: usize,
    pub html_offset: usize,
}
```

### Python

```python title="header_metadata.py"
class HeaderMetadata(TypedDict, total=False):
    level: int
    text: str
    id: str | None
    depth: int
    html_offset: int
```

### TypeScript

```typescript title="header_metadata.ts"
export interface HeaderMetadata {
  level: number;
  text: string;
  id?: string | null;
  depth: number;
  htmlOffset: number;
}
```

### Java

```java title="HeaderMetadata.java"
public record HeaderMetadata(
    int level,
    String text,
    Optional<String> id,
    int depth,
    int htmlOffset
) {}
```

### Go

```go title="header_metadata.go"
type HeaderMetadata struct {
    Level      int    `json:"level"`
    Text       string `json:"text"`
    ID         *string `json:"id,omitempty"`
    Depth      int    `json:"depth"`
    HtmlOffset int    `json:"html_offset"`
}
```

### C#

```csharp title="HeaderMetadata.cs"
public record HeaderMetadata
{
    public required int Level { get; init; }
    public required string Text { get; init; }
    public string? Id { get; init; }
    public required int Depth { get; init; }
    public required int HtmlOffset { get; init; }
}
```

**Fields:**

- `level`: Header level 1-6 (h1 through h6)
- `text`: Normalized text content of the header
- `id`: Optional HTML id attribute
- `depth`: Document tree depth at the header element
- `html_offset`: Byte offset in original HTML document

## LinkMetadata

Metadata for hyperlink elements with classification and relationship information.

### Rust

```rust title="link_metadata.rs"
pub struct LinkMetadata {
    pub href: String,
    pub text: String,
    pub title: Option<String>,
    pub link_type: LinkType,
    pub rel: Vec<String>,
    pub attributes: HashMap<String, String>,
}
```

### Python

```python title="link_metadata.py"
class LinkMetadata(TypedDict, total=False):
    href: str
    text: str
    title: str | None
    link_type: LinkType
    rel: list[str]
    attributes: dict[str, str]
```

### TypeScript

```typescript title="link_metadata.ts"
export interface LinkMetadata {
  href: string;
  text: string;
  title?: string | null;
  linkType: LinkType;
  rel: string[];
  attributes: Record<string, string>;
}
```

### Java

```java title="LinkMetadata.java"
public record LinkMetadata(
    String href,
    String text,
    Optional<String> title,
    LinkType linkType,
    List<String> rel,
    Map<String, String> attributes
) {}
```

### Go

```go title="link_metadata.go"
type LinkMetadata struct {
    Href       string            `json:"href"`
    Text       string            `json:"text"`
    Title      *string           `json:"title,omitempty"`
    LinkType   LinkType          `json:"link_type"`
    Rel        []string          `json:"rel"`
    Attributes map[string]string `json:"attributes"`
}
```

### C#

```csharp title="LinkMetadata.cs"
public record LinkMetadata
{
    public required string Href { get; init; }
    public required string Text { get; init; }
    public string? Title { get; init; }
    public required LinkType LinkType { get; init; }
    public required List<string> Rel { get; init; }
    public required Dictionary<string, string> Attributes { get; init; }
}
```

**Fields:**

- `href`: The href URL value
- `text`: Link text content (normalized)
- `title`: Optional title attribute
- `link_type`: Classification of link type
- `rel`: Values from rel attribute
- `attributes`: Additional attributes as key-value pairs

## LinkType

Link type classification enum.

### Rust

```rust title="link_type.rs"
pub enum LinkType {
    Anchor,
    Internal,
    External,
    Email,
    Phone,
    Other,
}
```

### Python

```python title="link_type.py"
LinkType = Literal["anchor", "internal", "external", "email", "phone", "other"]
```

### TypeScript

```typescript title="link_type.ts"
export type LinkType =
  | "anchor"
  | "internal"
  | "external"
  | "email"
  | "phone"
  | "other";
```

### Java

```java title="LinkType.java"
public enum LinkType {
    ANCHOR,
    INTERNAL,
    EXTERNAL,
    EMAIL,
    PHONE,
    OTHER
}
```

### Go

```go title="link_type.go"
type LinkType string

const (
    LinkTypeAnchor   LinkType = "anchor"
    LinkTypeInternal LinkType = "internal"
    LinkTypeExternal LinkType = "external"
    LinkTypeEmail    LinkType = "email"
    LinkTypePhone    LinkType = "phone"
    LinkTypeOther    LinkType = "other"
)
```

### C#

```csharp title="LinkType.cs"
public enum LinkType
{
    Anchor,
    Internal,
    External,
    Email,
    Phone,
    Other
}
```

**Values:**

- `Anchor`: Anchor link (#section)
- `Internal`: Internal link (same domain)
- `External`: External link (different domain)
- `Email`: Email link (mailto:)
- `Phone`: Phone link (tel:)
- `Other`: Other link type

## ImageMetadataType

Metadata for image elements with source, dimensions, and type classification.

### Rust

```rust title="image_metadata_type.rs"
pub struct ImageMetadataType {
    pub src: String,
    pub alt: Option<String>,
    pub title: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub image_type: ImageType,
    pub attributes: HashMap<String, String>,
}
```

### Python

```python title="image_metadata_type.py"
class ImageMetadataType(TypedDict, total=False):
    src: str
    alt: str | None
    title: str | None
    dimensions: tuple[int, int] | None
    image_type: ImageType
    attributes: dict[str, str]
```

### TypeScript

```typescript title="image_metadata_type.ts"
export interface ImageMetadataType {
  src: string;
  alt?: string | null;
  title?: string | null;
  dimensions?: [number, number] | null;
  imageType: ImageType;
  attributes: Record<string, string>;
}
```

### Java

```java title="ImageMetadataType.java"
public record ImageMetadataType(
    String src,
    Optional<String> alt,
    Optional<String> title,
    Optional<int[]> dimensions,
    ImageType imageType,
    Map<String, String> attributes
) {}
```

### Go

```go title="image_metadata_type.go"
type ImageMetadataType struct {
    Src        string            `json:"src"`
    Alt        *string           `json:"alt,omitempty"`
    Title      *string           `json:"title,omitempty"`
    Dimensions *[2]int           `json:"dimensions,omitempty"`
    ImageType  ImageType         `json:"image_type"`
    Attributes map[string]string `json:"attributes"`
}
```

### C#

```csharp title="ImageMetadataType.cs"
public record ImageMetadataType
{
    public required string Src { get; init; }
    public string? Alt { get; init; }
    public string? Title { get; init; }
    public (int Width, int Height)? Dimensions { get; init; }
    public required ImageType ImageType { get; init; }
    public required Dictionary<string, string> Attributes { get; init; }
}
```

**Fields:**

- `src`: Image source (URL, data URI, or SVG content)
- `alt`: Alternative text from alt attribute
- `title`: Title attribute
- `dimensions`: Image dimensions as (width, height) if available
- `image_type`: Classification of image source type
- `attributes`: Additional attributes as key-value pairs

## ImageType

Image type classification enum.

### Rust

```rust title="image_type.rs"
pub enum ImageType {
    DataUri,
    InlineSvg,
    External,
    Relative,
}
```

### Python

```python title="image_type.py"
ImageType = Literal["data-uri", "inline-svg", "external", "relative"]
```

### TypeScript

```typescript title="image_type.ts"
export type ImageType = "data-uri" | "inline-svg" | "external" | "relative";
```

### Java

```java title="ImageType.java"
public enum ImageType {
    DATA_URI,
    INLINE_SVG,
    EXTERNAL,
    RELATIVE
}
```

### Go

```go title="image_type.go"
type ImageType string

const (
    ImageTypeDataUri   ImageType = "data-uri"
    ImageTypeInlineSvg ImageType = "inline-svg"
    ImageTypeExternal  ImageType = "external"
    ImageTypeRelative  ImageType = "relative"
)
```

### C#

```csharp title="ImageType.cs"
public enum ImageType
{
    DataUri,
    InlineSvg,
    External,
    Relative
}
```

**Values:**

- `DataUri`: Data URI image
- `InlineSvg`: Inline SVG
- `External`: External image URL
- `Relative`: Relative path image

## StructuredData

Structured data block metadata (Schema.org, microdata, RDFa) with type classification.

### Rust

```rust title="structured_data.rs"
pub struct StructuredData {
    pub data_type: StructuredDataType,
    pub raw_json: String,
    pub schema_type: Option<String>,
}
```

### Python

```python title="structured_data.py"
class StructuredData(TypedDict, total=False):
    data_type: StructuredDataType
    raw_json: str
    schema_type: str | None
```

### TypeScript

```typescript title="structured_data.ts"
export interface StructuredData {
  dataType: StructuredDataType;
  rawJson: string;
  schemaType?: string | null;
}
```

### Java

```java title="StructuredData.java"
public record StructuredData(
    StructuredDataType dataType,
    String rawJson,
    Optional<String> schemaType
) {}
```

### Go

```go title="structured_data.go"
type StructuredData struct {
    DataType   StructuredDataType `json:"data_type"`
    RawJson    string             `json:"raw_json"`
    SchemaType *string            `json:"schema_type,omitempty"`
}
```

### C#

```csharp title="StructuredData.cs"
public record StructuredData
{
    public required StructuredDataType DataType { get; init; }
    public required string RawJson { get; init; }
    public string? SchemaType { get; init; }
}
```

**Fields:**

- `data_type`: Type of structured data (JSON-LD, Microdata, RDFa)
- `raw_json`: Raw JSON string representation
- `schema_type`: Schema type if detectable (e.g., "Article", "Event", "Product")

## StructuredDataType

Structured data type classification enum.

### Rust

```rust title="structured_data_type.rs"
pub enum StructuredDataType {
    JsonLd,
    Microdata,
    RDFa,
}
```

### Python

```python title="structured_data_type.py"
StructuredDataType = Literal["json-ld", "microdata", "rdfa"]
```

### TypeScript

```typescript title="structured_data_type.ts"
export type StructuredDataType = "json-ld" | "microdata" | "rdfa";
```

### Java

```java title="StructuredDataType.java"
public enum StructuredDataType {
    JSON_LD,
    MICRODATA,
    RDFA
}
```

### Go

```go title="structured_data_type.go"
type StructuredDataType string

const (
    StructuredDataTypeJsonLd   StructuredDataType = "json-ld"
    StructuredDataTypeMicrodata StructuredDataType = "microdata"
    StructuredDataTypeRDFa     StructuredDataType = "rdfa"
)
```

### C#

```csharp title="StructuredDataType.cs"
public enum StructuredDataType
{
    JsonLd,
    Microdata,
    RDFa
}
```

**Values:**

- `JsonLd`: JSON-LD structured data
- `Microdata`: Microdata structured data
- `RDFa`: RDFa structured data

### Text/Markdown Metadata

Text document statistics and structure including line/word/character counts, headers, links, and code blocks. Available when `format_type == "text"`.

#### Rust

```rust title="text_metadata.rs"
pub struct TextMetadata {
    pub line_count: usize,
    pub word_count: usize,
    pub character_count: usize,
    pub headers: Option<Vec<String>>,
    pub links: Option<Vec<(String, String)>>,
    pub code_blocks: Option<Vec<(String, String)>>,
}
```

#### Python

```python title="text_metadata.py"
class TextMetadata(TypedDict, total=False):
    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None
```

#### TypeScript

```typescript title="text_metadata.ts"
export interface TextMetadata {
  lineCount?: number;
  wordCount?: number;
  characterCount?: number;
  headers?: string[] | null;
  links?: [string, string][] | null;
  codeBlocks?: [string, string][] | null;
}
```

#### Java

```java title="TextMetadata.java"
public record TextMetadata(
    int lineCount,
    int wordCount,
    int characterCount,
    Optional<List<String>> headers,
    Optional<List<String[]>> links,
    Optional<List<String[]>> codeBlocks
) {}
```

#### Go

```go title="text_metadata.go"
type TextMetadata struct {
    LineCount      int         `json:"line_count"`
    WordCount      int         `json:"word_count"`
    CharacterCount int         `json:"character_count"`
    Headers        []string    `json:"headers,omitempty"`
    Links          [][2]string `json:"links,omitempty"`
    CodeBlocks     [][2]string `json:"code_blocks,omitempty"`
}
```

### PowerPoint Metadata

Presentation metadata including title, author, description, summary, and font information. Available when `format_type == "pptx"`.

#### Rust

```rust title="pptx_metadata.rs"
pub struct PptxMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub fonts: Vec<String>,
}
```

#### Python

```python title="pptx_metadata.py"
class PptxMetadata(TypedDict, total=False):
    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]
```

#### TypeScript

```typescript title="pptx_metadata.ts"
export interface PptxMetadata {
  title?: string | null;
  author?: string | null;
  description?: string | null;
  summary?: string | null;
  fonts?: string[];
}
```

#### Java

```java title="PptxMetadata.java"
public record PptxMetadata(
    Optional<String> title,
    Optional<String> author,
    Optional<String> description,
    Optional<String> summary,
    List<String> fonts
) {}
```

#### Go

```go title="pptx_metadata.go"
type PptxMetadata struct {
    Title       *string  `json:"title,omitempty"`
    Author      *string  `json:"author,omitempty"`
    Description *string  `json:"description,omitempty"`
    Summary     *string  `json:"summary,omitempty"`
    Fonts       []string `json:"fonts"`
}
```

### OCR Metadata

Optical Character Recognition processing metadata including language, page segmentation mode, output format, and table detection results. Available when `format_type == "ocr"`.

#### Rust

```rust title="ocr_metadata.rs"
pub struct OcrMetadata {
    pub language: String,
    pub psm: i32,
    pub output_format: String,
    pub table_count: usize,
    pub table_rows: Option<usize>,
    pub table_cols: Option<usize>,
}
```

#### Python

```python title="ocr_metadata.py"
class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None
```

#### TypeScript

```typescript title="ocr_metadata.ts"
export interface OcrMetadata {
  language?: string;
  psm?: number;
  outputFormat?: string;
  tableCount?: number;
  tableRows?: number | null;
  tableCols?: number | null;
}
```

#### Java

```java title="OcrMetadata.java"
public record OcrMetadata(
    String language,
    int psm,
    String outputFormat,
    int tableCount,
    Optional<Integer> tableRows,
    Optional<Integer> tableCols
) {}
```

#### Go

```go title="ocr_metadata.go"
type OcrMetadata struct {
    Language     string `json:"language"`
    PSM          int    `json:"psm"`
    OutputFormat string `json:"output_format"`
    TableCount   int    `json:"table_count"`
    TableRows    *int   `json:"table_rows,omitempty"`
    TableCols    *int   `json:"table_cols,omitempty"`
}
```

## Table

Structured table data extracted from documents with cell contents in 2D array format, markdown representation, and source page number.

### Rust

```rust title="table.rs"
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
    pub bounding_box: Option<BoundingBox>,
}
```

### Python

```python title="table.py"
class Table(TypedDict):
    cells: list[list[str]]
    markdown: str
    page_number: int
    bounding_box: BoundingBox | None
```

### TypeScript

```typescript title="table.ts"
export interface Table {
  cells: string[][];
  markdown: string;
  pageNumber: number;
}
```

### Ruby

```ruby title="table.rb"
Kreuzberg::Result::Table = Struct.new(:cells, :markdown, :page_number, :bounding_box, keyword_init: true)
```

### Java

```java title="Table.java"
public record Table(
    List<List<String>> cells,
    String markdown,
    int pageNumber,
    Optional<BoundingBox> boundingBox
) {}
```

### Go

```go title="table.go"
type Table struct {
    Cells        [][]string   `json:"cells"`
    Markdown     string       `json:"markdown"`
    PageNumber   int          `json:"page_number"`
    BoundingBox  *BoundingBox `json:"bounding_box,omitempty"`
}
```

## Chunk

Text chunk for RAG and vector search applications, containing content segment, optional embedding vector, and position metadata for precise document referencing.

### Rust

```rust title="chunk.rs"
pub struct Chunk {
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: ChunkMetadata,
}

pub struct ChunkMetadata {
    pub byte_start: usize,
    pub byte_end: usize,
    pub token_count: Option<usize>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub first_page: Option<usize>,
    pub last_page: Option<usize>,
    pub heading_context: Option<HeadingContext>,
}
```

### Python

```python title="chunk.py"
class ChunkMetadata(TypedDict):
    byte_start: int
    byte_end: int
    token_count: int | None
    chunk_index: int
    total_chunks: int
    first_page: int | None
    last_page: int | None
    heading_context: HeadingContext | None

class Chunk(TypedDict, total=False):
    content: str
    embedding: list[float] | None
    metadata: ChunkMetadata
```

### TypeScript

```typescript title="chunk.ts"
export interface ChunkMetadata {
  byteStart: number;
  byteEnd: number;
  tokenCount?: number | null;
  chunkIndex: number;
  totalChunks: number;
  firstPage?: number | null;
  lastPage?: number | null;
  headingContext?: HeadingContext | null;
}

export interface Chunk {
  content: string;
  embedding?: number[] | null;
  metadata: ChunkMetadata;
}
```

### Ruby

```ruby title="chunk.rb"
Kreuzberg::Result::Chunk = Struct.new(
    :content, :byte_start, :byte_end, :token_count,
    :chunk_index, :total_chunks, :first_page, :last_page, :embedding,
    :heading_context,
    keyword_init: true
)
```

### Java

```java title="Chunk.java"
public record ChunkMetadata(
    int byteStart,
    int byteEnd,
    Optional<Integer> tokenCount,
    int chunkIndex,
    int totalChunks,
    Optional<Integer> firstPage,
    Optional<Integer> lastPage,
    Optional<HeadingContext> headingContext
) {}

public record Chunk(
    String content,
    Optional<List<Float>> embedding,
    ChunkMetadata metadata
) {}
```

### Go

```go title="chunk.go"
type ChunkMetadata struct {
    ByteStart      int             `json:"byte_start"`
    ByteEnd        int             `json:"byte_end"`
    TokenCount     *int            `json:"token_count,omitempty"`
    ChunkIndex     int             `json:"chunk_index"`
    TotalChunks    int             `json:"total_chunks"`
    FirstPage      *int            `json:"first_page,omitempty"`
    LastPage       *int            `json:"last_page,omitempty"`
    HeadingContext *HeadingContext  `json:"heading_context,omitempty"`
}

type Chunk struct {
    Content   string        `json:"content"`
    Embedding []float32     `json:"embedding,omitempty"`
    Metadata  ChunkMetadata `json:"metadata"`
}
```

## ExtractedImage

Binary image data extracted from documents with format metadata, dimensions, colorspace information, and optional nested OCR extraction results.

### Rust

```rust title="extracted_image.rs"
use bytes::Bytes;
use std::borrow::Cow;

pub struct ExtractedImage {
    pub data: Bytes,
    pub format: Cow<'static, str>,
    pub image_index: usize,
    pub page_number: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    pub ocr_result: Option<Box<ExtractionResult>>,
    pub bounding_box: Option<BoundingBox>,
}
```

**Field notes:**

- `data`: Uses `Bytes` for cheap cloning of large image buffers
- `format`: Uses `Cow<'static, str>` to avoid allocation for static format literals (e.g., "jpeg", "png"). In serialized JSON, appears as a regular string.
- All other fields serialize as expected for their types

### Python

```python title="extracted_image.py"
class ExtractedImage(TypedDict, total=False):
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
    bounding_box: BoundingBox | None
```

### TypeScript

```typescript title="extracted_image.ts"
export interface ExtractedImage {
  data: Uint8Array;
  format: string;
  imageIndex: number;
  pageNumber?: number | null;
  width?: number | null;
  height?: number | null;
  colorspace?: string | null;
  bitsPerComponent?: number | null;
  isMask: boolean;
  description?: string | null;
  ocrResult?: ExtractionResult | null;
}
```

### Ruby

```ruby title="extracted_image.rb"
Kreuzberg::Result::Image = Struct.new(
    :data, :format, :image_index, :page_number, :width, :height,
    :colorspace, :bits_per_component, :is_mask, :description, :ocr_result, :bounding_box,
    keyword_init: true
)
```

### Java

```java title="ExtractedImage.java"
public record ExtractedImage(
    byte[] data,
    String format,
    int imageIndex,
    Optional<Integer> pageNumber,
    Optional<Integer> width,
    Optional<Integer> height,
    Optional<String> colorspace,
    Optional<Integer> bitsPerComponent,
    boolean isMask,
    Optional<String> description,
    Optional<ExtractionResult> ocrResult,
    Optional<BoundingBox> boundingBox
) {}
```

### Go

```go title="extracted_image.go"
type ExtractedImage struct {
    Data             []byte            `json:"data"`
    Format           string            `json:"format"`
    ImageIndex       int               `json:"image_index"`
    PageNumber       *int              `json:"page_number,omitempty"`
    Width            *uint32           `json:"width,omitempty"`
    Height           *uint32           `json:"height,omitempty"`
    Colorspace       *string           `json:"colorspace,omitempty"`
    BitsPerComponent *uint32           `json:"bits_per_component,omitempty"`
    IsMask           bool              `json:"is_mask"`
    Description      *string           `json:"description,omitempty"`
    OCRResult        *ExtractionResult `json:"ocr_result,omitempty"`
    BoundingBox      *BoundingBox      `json:"bounding_box,omitempty"`
}
```

## Configuration Types

### ExtractionConfig

Comprehensive extraction pipeline configuration controlling OCR, chunking, image processing, language detection, and all processing features.

#### Rust

```rust title="extraction_config.rs"
pub struct ExtractionConfig {
    pub use_cache: bool,
    pub enable_quality_processing: bool,
    pub ocr: Option<OcrConfig>,
    pub force_ocr: bool,
    pub chunking: Option<ChunkingConfig>,
    pub images: Option<ImageExtractionConfig>,
    pub pdf_options: Option<PdfConfig>,
    pub token_reduction: Option<TokenReductionConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub keywords: Option<KeywordConfig>,
    pub postprocessor: Option<PostProcessorConfig>,
    pub max_concurrent_extractions: Option<usize>,
}
```

#### Python

```python title="extraction_config.py"
@dataclass
class ExtractionConfig:
    use_cache: bool = True
    enable_quality_processing: bool = True
    ocr: OcrConfig | None = None
    force_ocr: bool = False
    chunking: ChunkingConfig | None = None
    images: ImageExtractionConfig | None = None
    pdf_options: PdfConfig | None = None
    token_reduction: TokenReductionConfig | None = None
    language_detection: LanguageDetectionConfig | None = None
    keywords: KeywordConfig | None = None
    postprocessor: PostProcessorConfig | None = None
    max_concurrent_extractions: int | None = None
```

#### TypeScript

```typescript title="extraction_config.ts"
export interface ExtractionConfig {
  useCache?: boolean;
  enableQualityProcessing?: boolean;
  ocr?: OcrConfig;
  forceOcr?: boolean;
  chunking?: ChunkingConfig;
  images?: ImageExtractionConfig;
  pdfOptions?: PdfConfig;
  tokenReduction?: TokenReductionConfig;
  languageDetection?: LanguageDetectionConfig;
  keywords?: KeywordConfig;
  postprocessor?: PostProcessorConfig;
  maxConcurrentExtractions?: number;
}
```

#### Java

```java title="ExtractionConfig.java"
public record ExtractionConfig(
    boolean useCache,
    boolean enableQualityProcessing,
    Optional<OcrConfig> ocr,
    boolean forceOcr,
    Optional<ChunkingConfig> chunking,
    Optional<ImageExtractionConfig> images,
    Optional<PdfConfig> pdfOptions,
    Optional<TokenReductionConfig> tokenReduction,
    Optional<LanguageDetectionConfig> languageDetection,
    Optional<KeywordConfig> keywords,
    Optional<PostProcessorConfig> postprocessor,
    Optional<Integer> maxConcurrentExtractions
) {}
```

#### Go

```go title="extraction_config.go"
type ExtractionConfig struct {
    UseCache                    bool
    EnableQualityProcessing     bool
    OCR                         *OcrConfig
    ForceOCR                    bool
    Chunking                    *ChunkingConfig
    Images                      *ImageExtractionConfig
    PDFOptions                  *PdfConfig
    TokenReduction              *TokenReductionConfig
    LanguageDetection           *LanguageDetectionConfig
    Keywords                    *KeywordConfig
    PostProcessor               *PostProcessorConfig
    MaxConcurrentExtractions    *int
}
```

### OcrConfig

OCR engine selection and language configuration for Tesseract, EasyOCR, and PaddleOCR backends.

#### Rust

```rust title="ocr_config.rs"
pub struct OcrConfig {
    pub backend: String,  // "tesseract", "easyocr", "paddleocr"
    pub language: String, // e.g., "eng", "deu", "fra"
    pub tesseract_config: Option<TesseractConfig>,
}
```

#### Python

```python title="ocr_config.py"
@dataclass
class OcrConfig:
    backend: str = "tesseract"
    language: str = "eng"
    tesseract_config: TesseractConfig | None = None
```

#### TypeScript

```typescript title="ocr_config.ts"
export interface OcrConfig {
  backend: string;
  language?: string;
  tesseractConfig?: TesseractConfig;
}
```

#### Java

```java title="OcrConfig.java"
public record OcrConfig(
    String backend,
    String language,
    Optional<TesseractConfig> tesseractConfig
) {}
```

#### Go

```go title="ocr_config.go"
type OcrConfig struct {
    Backend            string
    Language           string
    TesseractConfig    *TesseractConfig
}
```

### TesseractConfig

Advanced Tesseract OCR engine parameters including page segmentation mode, preprocessing, table detection, and character whitelisting/blacklisting.

#### Rust

```rust title="tesseract_config.rs"
pub struct TesseractConfig {
    pub language: String,
    pub psm: i32,  // Page Segmentation Mode (0-13)
    pub output_format: String,  // "text" or "markdown"
    pub oem: i32,  // OCR Engine Mode (0-3)
    pub min_confidence: f64,
    pub preprocessing: Option<ImagePreprocessingConfig>,
    pub enable_table_detection: bool,
    pub table_min_confidence: f64,
    pub table_column_threshold: i32,
    pub table_row_threshold_ratio: f64,
    pub use_cache: bool,
    pub classify_use_pre_adapted_templates: bool,
    pub language_model_ngram_on: bool,
    pub tessedit_dont_blkrej_good_wds: bool,
    pub tessedit_dont_rowrej_good_wds: bool,
    pub tessedit_enable_dict_correction: bool,
    pub tessedit_char_whitelist: String,
    pub tessedit_char_blacklist: String,
    pub tessedit_use_primary_params_model: bool,
    pub textord_space_size_is_variable: bool,
    pub thresholding_method: bool,
}
```

### ChunkingConfig

Text chunking configuration for RAG pipelines with character limits, overlap control, and optional embedding generation.

#### Rust

```rust title="chunking_config.rs"
pub struct ChunkingConfig {
    pub max_characters: usize,     // default: 1000, serde alias: "max_chars"
    pub overlap: usize,              // default: 200, serde alias: "max_overlap"
    pub trim: bool,                  // default: true
    pub chunker_type: ChunkerType,   // default: ChunkerType::Text
    pub embedding: Option<EmbeddingConfig>,
    pub preset: Option<String>,
    pub sizing: ChunkSizing,         // default: ChunkSizing::Characters
}
```

#### Python

```python title="chunking_config.py"
@dataclass
class ChunkingConfig:
    max_characters: int = 1000
    overlap: int = 200
    trim: bool = True
    chunker_type: ChunkerType = ChunkerType.TEXT
    embedding: EmbeddingConfig | None = None
    preset: str | None = None
    sizing: ChunkSizing = ChunkSizing.CHARACTERS
```

#### TypeScript

```typescript title="chunking_config.ts"
export interface ChunkingConfig {
  maxCharacters?: number;
  overlap?: number;
  trim?: boolean;
  chunkerType?: ChunkerType;
  embedding?: EmbeddingConfig;
  preset?: string;
  sizing?: ChunkSizing;
}
```

#### Java

```java title="ChunkingConfig.java"
public record ChunkingConfig(
    int maxCharacters,
    int overlap,
    boolean trim,
    ChunkerType chunkerType,
    Optional<EmbeddingConfig> embedding,
    Optional<String> preset,
    ChunkSizing sizing
) {}
```

#### Go

```go title="chunking_config.go"
type ChunkingConfig struct {
    MaxCharacters int              `json:"max_characters"`
    Overlap       int              `json:"overlap"`
    Trim          bool             `json:"trim"`
    ChunkerType   ChunkerType      `json:"chunker_type"`
    Embedding     *EmbeddingConfig `json:"embedding,omitempty"`
    Preset        *string          `json:"preset,omitempty"`
    Sizing        ChunkSizing      `json:"sizing"`
}
```

### ChunkerType

Chunking strategy type for text processing.

#### Rust

```rust title="chunker_type.rs"
#[derive(Default)]
pub enum ChunkerType {
    #[default]
    Text,
    Markdown,
}
```

#### Python

```python title="chunker_type.py"
from enum import Enum

class ChunkerType(str, Enum):
    TEXT = "text"
    MARKDOWN = "markdown"
```

#### TypeScript

```typescript title="chunker_type.ts"
export type ChunkerType = "text" | "markdown";
```

#### Java

```java title="ChunkerType.java"
public enum ChunkerType {
    TEXT("text"),
    MARKDOWN("markdown");

    private final String value;

    ChunkerType(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}
```

#### Go

```go title="chunker_type.go"
type ChunkerType string

const (
    ChunkerTypeText     ChunkerType = "text"
    ChunkerTypeMarkdown ChunkerType = "markdown"
)
```

### HeadingLevel

A single heading in the document hierarchy, representing one level of section nesting.

#### Rust

```rust title="heading_level.rs"
pub struct HeadingLevel {
    pub level: u8,      // Heading depth (1 = h1, 2 = h2, etc.)
    pub text: String,   // The text content of the heading
}
```

#### Python

```python title="heading_level.py"
@dataclass
class HeadingLevel:
    level: int
    text: str
```

#### TypeScript

```typescript title="heading_level.ts"
export interface HeadingLevel {
  level: number;
  text: string;
}
```

#### Java

```java title="HeadingLevel.java"
public record HeadingLevel(
    int level,
    String text
) {}
```

#### Go

```go title="heading_level.go"
type HeadingLevel struct {
    Level int    `json:"level"`
    Text  string `json:"text"`
}
```

### HeadingContext

Heading hierarchy from root to current section, providing structural context for a chunk within the document.

#### Rust

```rust title="heading_context.rs"
pub struct HeadingContext {
    pub headings: Vec<HeadingLevel>,  // Hierarchy from root to current section
}
```

#### Python

```python title="heading_context.py"
@dataclass
class HeadingContext:
    headings: list[HeadingLevel]
```

#### TypeScript

```typescript title="heading_context.ts"
export interface HeadingContext {
  headings: HeadingLevel[];
}
```

#### Java

```java title="HeadingContext.java"
public record HeadingContext(
    List<HeadingLevel> headings
) {}
```

#### Go

```go title="heading_context.go"
type HeadingContext struct {
    Headings []HeadingLevel `json:"headings"`
}
```

### ChunkSizing

Chunk size measurement strategy. Defaults to Unicode character count; optionally uses a HuggingFace tokenizer (requires `chunking-tokenizers` feature).

#### Rust

```rust title="chunk_sizing.rs"
pub enum ChunkSizing {
    Characters,           // Default: Unicode character count
    Tokenizer {           // Requires `chunking-tokenizers` feature
        model: String,    // HuggingFace model ID (e.g., "Xenova/gpt-4o")
        cache_dir: Option<PathBuf>,  // Optional cache directory
    },
}
```

#### Python

```python title="chunk_sizing.py"
@dataclass
class ChunkSizing:
    mode: str = "characters"          # "characters" or "tokenizer"
    model: str | None = None          # HuggingFace model ID
    cache_dir: str | None = None      # Optional cache directory
```

#### TypeScript

```typescript title="chunk_sizing.ts"
export type ChunkSizing =
  | { mode: "characters" }
  | { mode: "tokenizer"; model: string; cacheDir?: string };
```

#### Java

```java title="ChunkSizing.java"
public sealed interface ChunkSizing {
    record Characters() implements ChunkSizing {}
    record Tokenizer(
        String model,
        Optional<String> cacheDir
    ) implements ChunkSizing {}
}
```

#### Go

```go title="chunk_sizing.go"
type ChunkSizing struct {
    Mode     string  `json:"mode"`                // "characters" or "tokenizer"
    Model    *string `json:"model,omitempty"`      // HuggingFace model ID
    CacheDir *string `json:"cache_dir,omitempty"`  // Optional cache directory
}
```

### EmbeddingConfig

Vector embedding configuration supporting FastEmbed models with normalization, batch processing, and custom model selection.

#### Rust

```rust title="embedding_config.rs"
pub struct EmbeddingConfig {
    pub model: EmbeddingModelType,
    pub normalize: bool,
    pub batch_size: usize,
    pub show_download_progress: bool,
    pub cache_dir: Option<PathBuf>,
}

pub enum EmbeddingModelType {
    Preset { name: String },
    FastEmbed { model: String, dimensions: usize },
    Custom { model_id: String, dimensions: usize },
}
```

#### Python

```python title="embedding_config.py"
@dataclass
class EmbeddingConfig:
    model: EmbeddingModelType = field(default_factory=lambda: Preset("balanced"))
    normalize: bool = True
    batch_size: int = 32
    show_download_progress: bool = False
    cache_dir: Path | None = None

@dataclass
class EmbeddingModelType:
    # Discriminated union: Preset, FastEmbed, or Custom model types
    pass
```

#### TypeScript

```typescript title="embedding_config.ts"
export interface EmbeddingConfig {
  model: EmbeddingModelType;
  normalize?: boolean;
  batchSize?: number;
  showDownloadProgress?: boolean;
  cacheDir?: string;
}

export type EmbeddingModelType =
  | { type: "preset"; name: string }
  | { type: "fastembed"; model: string; dimensions: number }
  | { type: "custom"; modelId: string; dimensions: number };
```

### ImageExtractionConfig

Image extraction and preprocessing settings including DPI targeting, dimension limits, and automatic DPI adjustment for OCR quality.

#### Rust

```rust title="image_extraction_config.rs"
pub struct ImageExtractionConfig {
    pub extract_images: bool,
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub inject_placeholders: bool,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}
```

#### Python

```python title="image_extraction_config.py"
@dataclass
class ImageExtractionConfig:
    extract_images: bool = True
    target_dpi: int = 300
    max_image_dimension: int = 4096
    inject_placeholders: bool = True
    auto_adjust_dpi: bool = True
    min_dpi: int = 72
    max_dpi: int = 600
```

#### TypeScript

```typescript title="image_extraction_config.ts"
export interface ImageExtractionConfig {
  extractImages?: boolean;
  targetDpi?: number;
  maxImageDimension?: number;
  injectPlaceholders?: boolean;
  autoAdjustDpi?: boolean;
  minDpi?: number;
  maxDpi?: number;
}
```

#### Java

```java title="ImageExtractionConfig.java"
public record ImageExtractionConfig(
    boolean extractImages,
    int targetDpi,
    int maxImageDimension,
    boolean injectPlaceholders,
    boolean autoAdjustDpi,
    int minDpi,
    int maxDpi
) {}
```

#### Go

```go title="image_extraction_config.go"
type ImageExtractionConfig struct {
    ExtractImages        bool
    TargetDPI            int32
    MaxImageDimension    int32
    InjectPlaceholders   bool
    AutoAdjustDPI        bool
    MinDPI               int32
    MaxDPI               int32
}
```

### PdfConfig

PDF-specific extraction options including image extraction control, password support for encrypted PDFs, and metadata extraction flags.

#### Rust

```rust title="pdf_config.rs"
pub struct PdfConfig {
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
}
```

#### Python

```python title="pdf_config.py"
@dataclass
class PdfConfig:
    extract_images: bool = False
    passwords: list[str] | None = None
    extract_metadata: bool = True
```

#### TypeScript

```typescript title="pdf_config.ts"
export interface PdfConfig {
  extractImages?: boolean;
  passwords?: string[];
  extractMetadata?: boolean;
}
```

#### Ruby

```ruby title="pdf_config.rb"
class Kreuzberg::Config::PdfConfig
    attr_accessor :extract_images, :passwords, :extract_metadata
end
```

#### Java

```java title="PdfConfig.java"
public final class PdfConfig {
    private final boolean extractImages;
    private final List<String> passwords;
    private final boolean extractMetadata;

    public static Builder builder() { }
}
```

#### Go

```go title="pdf_config.go"
type PdfConfig struct {
    ExtractImages   *bool    `json:"extract_images,omitempty"`
    Passwords       []string `json:"passwords,omitempty"`
    ExtractMetadata *bool    `json:"extract_metadata,omitempty"`
}
```

### TokenReductionConfig

Token reduction settings for output optimization while preserving semantically important words and phrases.

#### Rust

```rust title="token_reduction_config.rs"
pub struct TokenReductionConfig {
    pub mode: String,
    pub preserve_important_words: bool,
}
```

#### Python

```python title="token_reduction_config.py"
@dataclass
class TokenReductionConfig:
    mode: str = "off"
    preserve_important_words: bool = True
```

#### TypeScript

```typescript title="token_reduction_config.ts"
export interface TokenReductionConfig {
  mode?: string;
  preserveImportantWords?: boolean;
}
```

#### Ruby

```ruby title="token_reduction_config.rb"
class Kreuzberg::Config::TokenReductionConfig
    attr_accessor :mode, :preserve_important_words
end
```

#### Java

```java title="TokenReductionConfig.java"
public final class TokenReductionConfig {
    private final String mode;
    private final boolean preserveImportantWords;

    public static Builder builder() { }
}
```

#### Go

```go title="token_reduction_config.go"
type TokenReductionConfig struct {
    Mode                   string `json:"mode,omitempty"`
    PreserveImportantWords *bool  `json:"preserve_important_words,omitempty"`
}
```

### LanguageDetectionConfig

Automatic language identification configuration with confidence thresholds and multi-language detection support.

#### Rust

```rust title="language_detection_config.rs"
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}
```

#### Python

```python title="language_detection_config.py"
@dataclass
class LanguageDetectionConfig:
    enabled: bool = True
    min_confidence: float = 0.8
    detect_multiple: bool = False
```

#### TypeScript

```typescript title="language_detection_config.ts"
export interface LanguageDetectionConfig {
  enabled?: boolean;
  minConfidence?: number;
  detectMultiple?: boolean;
}
```

#### Ruby

```ruby title="language_detection_config.rb"
class Kreuzberg::Config::LanguageDetectionConfig
    attr_accessor :enabled, :min_confidence, :detect_multiple
end
```

#### Java

```java title="LanguageDetectionConfig.java"
public final class LanguageDetectionConfig {
    private final boolean enabled;
    private final double minConfidence;
    private final boolean detectMultiple;

    public static Builder builder() { }
}
```

#### Go

```go title="language_detection_config.go"
type LanguageDetectionConfig struct {
    Enabled        *bool    `json:"enabled,omitempty"`
    MinConfidence  *float64 `json:"min_confidence,omitempty"`
    DetectMultiple *bool    `json:"detect_multiple,omitempty"`
}
```

### KeywordConfig

Automatic keyword and keyphrase extraction using YAKE or RAKE algorithms with configurable scoring, n-gram ranges, and language support.

#### Rust

```rust title="keyword_config.rs"
pub struct KeywordConfig {
    pub algorithm: KeywordAlgorithm,
    pub max_keywords: usize,
    pub min_score: f32,
    pub ngram_range: (usize, usize),
    pub language: Option<String>,
    pub yake_params: Option<YakeParams>,
    pub rake_params: Option<RakeParams>,
}
```

#### Python

```python title="keyword_config.py"
@dataclass
class YakeParams:
    window_size: int = 2

@dataclass
class RakeParams:
    min_word_length: int = 1
    max_words_per_phrase: int = 3

@dataclass
class KeywordConfig:
    algorithm: str = "yake"
    max_keywords: int = 10
    min_score: float = 0.0
    ngram_range: tuple[int, int] = (1, 3)
    language: str | None = "en"
    yake_params: YakeParams | None = None
    rake_params: RakeParams | None = None
```

#### TypeScript

```typescript title="keyword_config.ts"
export interface YakeParams {
  windowSize?: number;
}

export interface RakeParams {
  minWordLength?: number;
  maxWordsPerPhrase?: number;
}

export interface KeywordConfig {
  algorithm?: KeywordAlgorithm;
  maxKeywords?: number;
  minScore?: number;
  ngramRange?: [number, number];
  language?: string;
  yakeParams?: YakeParams;
  rakeParams?: RakeParams;
}
```

#### Ruby

```ruby title="keyword_config.rb"
class Kreuzberg::Config::KeywordConfig
    attr_accessor :algorithm, :max_keywords, :min_score,
                  :ngram_range, :language, :yake_params, :rake_params
end
```

#### Java

```java title="KeywordConfig.java"
public final class KeywordConfig {
    private final String algorithm;
    private final Integer maxKeywords;
    private final Double minScore;
    private final int[] ngramRange;
    private final String language;
    private final YakeParams yakeParams;
    private final RakeParams rakeParams;

    public static Builder builder() { }

    public static final class YakeParams { }
    public static final class RakeParams { }
}
```

#### Go

```go title="keyword_config.go"
type YakeParams struct {
    WindowSize *int `json:"window_size,omitempty"`
}

type RakeParams struct {
    MinWordLength     *int `json:"min_word_length,omitempty"`
    MaxWordsPerPhrase *int `json:"max_words_per_phrase,omitempty"`
}

type KeywordConfig struct {
    Algorithm   string      `json:"algorithm,omitempty"`
    MaxKeywords *int        `json:"max_keywords,omitempty"`
    MinScore    *float64    `json:"min_score,omitempty"`
    NgramRange  *[2]int     `json:"ngram_range,omitempty"`
    Language    *string     `json:"language,omitempty"`
    Yake        *YakeParams `json:"yake_params,omitempty"`
    Rake        *RakeParams `json:"rake_params,omitempty"`
}
```

### ImagePreprocessingMetadata

Image preprocessing transformation log tracking original and final DPI, scaling factors, dimension changes, and any processing errors.

#### Rust

```rust title="image_preprocessing_metadata.rs"
pub struct ImagePreprocessingMetadata {
    pub original_dimensions: (usize, usize),
    pub original_dpi: (f64, f64),
    pub target_dpi: i32,
    pub scale_factor: f64,
    pub auto_adjusted: bool,
    pub final_dpi: i32,
    pub new_dimensions: Option<(usize, usize)>,
    pub resample_method: String,
    pub dimension_clamped: bool,
    pub calculated_dpi: Option<i32>,
    pub skipped_resize: bool,
    pub resize_error: Option<String>,
}
```

#### Python

```python title="image_preprocessing_metadata.py"
class ImagePreprocessingMetadata(TypedDict, total=False):
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
```

#### TypeScript

```typescript title="image_preprocessing_metadata.ts"
export interface ImagePreprocessingMetadata {
  originalDimensions?: [number, number];
  originalDpi?: [number, number];
  targetDpi?: number;
  scaleFactor?: number;
  autoAdjusted?: boolean;
  finalDpi?: number;
  newDimensions?: [number, number] | null;
  resampleMethod?: string;
  dimensionClamped?: boolean;
  calculatedDpi?: number | null;
  skippedResize?: boolean;
  resizeError?: string | null;
}
```

#### Ruby

```ruby title="image_preprocessing_metadata.rb"
class Kreuzberg::Result::ImagePreprocessingMetadata
    attr_reader :original_dimensions, :original_dpi, :target_dpi, :scale_factor,
                :auto_adjusted, :final_dpi, :new_dimensions, :resample_method,
                :dimension_clamped, :calculated_dpi, :skipped_resize, :resize_error
end
```

#### Java

```java title="ImagePreprocessingMetadata.java"
public record ImagePreprocessingMetadata(
    int[] originalDimensions,
    double[] originalDpi,
    int targetDpi,
    double scaleFactor,
    boolean autoAdjusted,
    int finalDpi,
    Optional<int[]> newDimensions,
    String resampleMethod,
    boolean dimensionClamped,
    Optional<Integer> calculatedDpi,
    boolean skippedResize,
    Optional<String> resizeError
) {}
```

#### Go

```go title="image_preprocessing_metadata.go"
type ImagePreprocessingMetadata struct {
    OriginalDimensions [2]int    `json:"original_dimensions"`
    OriginalDPI        [2]float64 `json:"original_dpi"`
    TargetDPI          int        `json:"target_dpi"`
    ScaleFactor        float64    `json:"scale_factor"`
    AutoAdjusted       bool       `json:"auto_adjusted"`
    FinalDPI           int        `json:"final_dpi"`
    NewDimensions      *[2]int    `json:"new_dimensions,omitempty"`
    ResampleMethod     string     `json:"resample_method"`
    DimensionClamped   bool       `json:"dimension_clamped"`
    CalculatedDPI      *int       `json:"calculated_dpi,omitempty"`
    SkippedResize      bool       `json:"skipped_resize"`
    ResizeError        *string    `json:"resize_error,omitempty"`
}
```

### ImagePreprocessingConfig

Image preprocessing configuration for OCR quality enhancement including rotation, deskewing, denoising, contrast adjustment, and binarization methods.

#### Rust

```rust title="image_preprocessing_config.rs"
pub struct ImagePreprocessingConfig {
    pub target_dpi: i32,
    pub auto_rotate: bool,
    pub deskew: bool,
    pub denoise: bool,
    pub contrast_enhance: bool,
    pub binarization_method: String,
    pub invert_colors: bool,
}
```

#### Python

```python title="image_preprocessing_config.py"
@dataclass
class ImagePreprocessingConfig:
    target_dpi: int = 300
    auto_rotate: bool = True
    deskew: bool = True
    denoise: bool = False
    contrast_enhance: bool = False
    binarization_method: str = "otsu"
    invert_colors: bool = False
```

#### TypeScript

```typescript title="image_preprocessing_config.ts"
export interface ImagePreprocessingConfig {
  targetDpi?: number;
  autoRotate?: boolean;
  deskew?: boolean;
  denoise?: boolean;
  contrastEnhance?: boolean;
  binarizationMethod?: string;
  invertColors?: boolean;
}
```

#### Ruby

```ruby title="image_preprocessing_config.rb"
class Kreuzberg::Config::ImagePreprocessingConfig
    attr_accessor :target_dpi, :auto_rotate, :deskew, :denoise,
                  :contrast_enhance, :binarization_method, :invert_colors
end
```

#### Java

```java title="ImagePreprocessingConfig.java"
public final class ImagePreprocessingConfig {
    private final int targetDpi;
    private final boolean autoRotate;
    private final boolean deskew;
    private final boolean denoise;
    private final boolean contrastEnhance;
    private final String binarizationMethod;
    private final boolean invertColors;

    public static Builder builder() { }
}
```

#### Go

```go title="image_preprocessing_config.go"
type ImagePreprocessingConfig struct {
    TargetDPI        *int   `json:"target_dpi,omitempty"`
    AutoRotate       *bool  `json:"auto_rotate,omitempty"`
    Deskew           *bool  `json:"deskew,omitempty"`
    Denoise          *bool  `json:"denoise,omitempty"`
    ContrastEnhance  *bool  `json:"contrast_enhance,omitempty"`
    BinarizationMode string `json:"binarization_method,omitempty"`
    InvertColors     *bool  `json:"invert_colors,omitempty"`
}
```

### ErrorMetadata

Error information captured during batch operations providing error type classification and detailed error messages.

#### Rust

```rust title="error_metadata.rs"
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
}
```

#### Python

```python title="error_metadata.py"
class ErrorMetadata(TypedDict, total=False):
    error_type: str
    message: str
```

#### TypeScript

```typescript title="error_metadata.ts"
export interface ErrorMetadata {
  errorType?: string;
  message?: string;
}
```

#### Ruby

```ruby title="error_metadata.rb"
class Kreuzberg::Result::ErrorMetadata
    attr_reader :error_type, :message
end
```

#### Java

```java title="ErrorMetadata.java"
public record ErrorMetadata(
    String errorType,
    String message
) {}
```

#### Go

```go title="error_metadata.go"
type ErrorMetadata struct {
    ErrorType string `json:"error_type"`
    Message   string `json:"message"`
}
```

### XmlMetadata

XML document structure statistics including total element count and unique element type inventory.

#### Rust

```rust title="xml_metadata.rs"
pub struct XmlMetadata {
    pub element_count: usize,
    pub unique_elements: Vec<String>,
}
```

#### Python

```python title="xml_metadata.py"
class XmlMetadata(TypedDict, total=False):
    element_count: int
    unique_elements: list[str]
```

#### TypeScript

```typescript title="xml_metadata.ts"
export interface XmlMetadata {
  elementCount?: number;
  uniqueElements?: string[];
}
```

#### Ruby

```ruby title="xml_metadata.rb"
class Kreuzberg::Result::XmlMetadata
    attr_reader :element_count, :unique_elements
end
```

#### Java

```java title="XmlMetadata.java"
public record XmlMetadata(
    int elementCount,
    List<String> uniqueElements
) {}
```

#### Go

```go title="xml_metadata.go"
type XmlMetadata struct {
    ElementCount  int      `json:"element_count"`
    UniqueElements []string `json:"unique_elements"`
}
```

### PostProcessorConfig

Post-processing pipeline control allowing selective enabling or disabling of individual text processors.

#### Rust

```rust title="post_processor_config.rs"
pub struct PostProcessorConfig {
    pub enabled: bool,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}
```

#### Python

```python title="post_processor_config.py"
@dataclass
class PostProcessorConfig:
    enabled: bool = True
    enabled_processors: list[str] | None = None
    disabled_processors: list[str] | None = None
```

#### TypeScript

```typescript title="post_processor_config.ts"
export interface PostProcessorConfig {
  enabled?: boolean;
  enabledProcessors?: string[];
  disabledProcessors?: string[];
}
```

#### Ruby

```ruby title="post_processor_config.rb"
class Kreuzberg::Config::PostProcessorConfig
    attr_accessor :enabled, :enabled_processors, :disabled_processors
end
```

#### Java

```java title="PostProcessorConfig.java"
public final class PostProcessorConfig {
    private final boolean enabled;
    private final List<String> enabledProcessors;
    private final List<String> disabledProcessors;

    public static Builder builder() { }
}
```

#### Go

```go title="post_processor_config.go"
type PostProcessorConfig struct {
    Enabled            *bool    `json:"enabled,omitempty"`
    EnabledProcessors  []string `json:"enabled_processors,omitempty"`
    DisabledProcessors []string `json:"disabled_processors,omitempty"`
}
```

### HierarchyConfig

Document hierarchy detection configuration controlling font size clustering and hierarchy level assignment. Extracts document structure (H1-H6 headings and body text) by analyzing font sizes and spatial positioning of text blocks.

#### Rust

```rust title="hierarchy_config.rs"
pub struct HierarchyConfig {
    // Enable hierarchy extraction
    pub enabled: bool,

    /// Number of font size clusters to use for hierarchy levels (1-7)
    /// Default: 6 (provides H1-H6 heading levels with body text)
    pub k_clusters: usize,

    // Include bounding box information in hierarchy blocks
    pub include_bbox: bool,

    /// OCR coverage threshold for smart OCR triggering (0.0-1.0)
    /// Default: 0.5 (trigger OCR if less than 50% of page has text)
    pub ocr_coverage_threshold: Option<f32>,
}
```

#### Python

```python title="hierarchy_config.py"
class HierarchyConfig:
    """Hierarchy detection configuration for document structure analysis."""

    def __init__(
        self,
        enabled: bool = True,
        k_clusters: int = 6,
        include_bbox: bool = True,
        ocr_coverage_threshold: float | None = None
    ):
        self.enabled = enabled
        self.k_clusters = k_clusters
        self.include_bbox = include_bbox
        self.ocr_coverage_threshold = ocr_coverage_threshold
```

#### TypeScript

```typescript title="hierarchy_config.ts"
export interface HierarchyConfig {
  /** Enable hierarchy extraction. Default: true. */
  enabled?: boolean;

  /** Number of font size clusters (2-10). Default: 6. */
  kClusters?: number;

  /** Include bounding box information. Default: true. */
  includeBbox?: boolean;

  /** OCR coverage threshold (0.0-1.0). Default: null. */
  ocrCoverageThreshold?: number | null;
}
```

#### Ruby

```ruby title="hierarchy_config.rb"
class Kreuzberg::Config::Hierarchy
    attr_reader :enabled, :k_clusters, :include_bbox, :ocr_coverage_threshold

    def initialize(
        enabled: true,
        k_clusters: 6,
        include_bbox: true,
        ocr_coverage_threshold: nil
    )
        @enabled = enabled
        @k_clusters = k_clusters
        @include_bbox = include_bbox
        @ocr_coverage_threshold = ocr_coverage_threshold
    end
end
```

#### Java

```java title="HierarchyConfig.java"
public final class HierarchyConfig {
    private final boolean enabled;
    private final int kClusters;
    private final boolean includeBbox;
    private final Double ocrCoverageThreshold;

    public static Builder builder() {
        return new Builder();
    }

    public boolean isEnabled() { return enabled; }
    public int getKClusters() { return kClusters; }
    public boolean isIncludeBbox() { return includeBbox; }
    public Double getOcrCoverageThreshold() { return ocrCoverageThreshold; }

    public static final class Builder {
        private boolean enabled = true;
        private int kClusters = 6;
        private boolean includeBbox = true;
        private Double ocrCoverageThreshold;

        public Builder enabled(boolean enabled) { ... }
        public Builder kClusters(int kClusters) { ... }
        public Builder includeBbox(boolean includeBbox) { ... }
        public Builder ocrCoverageThreshold(Double threshold) { ... }
        public HierarchyConfig build() { ... }
    }
}
```

#### Go

```go title="hierarchy_config.go"
// HierarchyConfig controls PDF hierarchy extraction based on font sizes.
type HierarchyConfig struct {
    // Enable hierarchy extraction. Default: true.
    Enabled *bool `json:"enabled,omitempty"`

    // Number of font size clusters (2-10). Default: 6.
    KClusters *int `json:"k_clusters,omitempty"`

    // Include bounding box information. Default: true.
    IncludeBbox *bool `json:"include_bbox,omitempty"`

    // OCR coverage threshold (0.0-1.0). Default: null.
    OcrCoverageThreshold *float64 `json:"ocr_coverage_threshold,omitempty"`
}
```

#### C#

```csharp title="HierarchyConfig.cs"
public sealed class HierarchyConfig
{
    /// <summary>
    /// Whether hierarchy detection is enabled.
    /// </summary>
    [JsonPropertyName("enabled")]
    public bool? Enabled { get; set; }

    /// <summary>
    /// Number of k clusters for hierarchy detection.
    /// </summary>
    [JsonPropertyName("k_clusters")]
    public int? KClusters { get; set; }

    /// <summary>
    /// Whether to include bounding box information in hierarchy output.
    /// </summary>
    [JsonPropertyName("include_bbox")]
    public bool? IncludeBbox { get; set; }

    /// <summary>
    /// OCR coverage threshold for hierarchy detection (0.0-1.0).
    /// </summary>
    [JsonPropertyName("ocr_coverage_threshold")]
    public float? OcrCoverageThreshold { get; set; }
}
```

**Fields:**

- `enabled`: Enable or disable hierarchy extraction (Default: `true`)
- `k_clusters`: Number of font size clusters for hierarchy classification (Range: 2-10, Default: 6)
  - 6 clusters map to H1-H6 heading levels plus body text
  - Larger values create finer-grained hierarchy distinctions
  - Smaller values group more font sizes together
- `include_bbox`: Include bounding box coordinates in output (Default: `true`)
  - When true, each block includes left, top, right, bottom coordinates in PDF units
  - When false, reduces output size but loses spatial positioning information
- `ocr_coverage_threshold`: Trigger OCR when text coverage falls below threshold (Range: 0.0-1.0, Default: `null`)
  - `0.5` = OCR triggers if less than 50% of page has extractable text
  - `null` = OCR triggering controlled by other config settings
  - Useful for detecting scanned or image-heavy documents

**Example Usage:**

```rust title="Rust - HierarchyConfig Setup"
use kreuzberg::core::config::HierarchyConfig;

let hierarchy = HierarchyConfig {
    enabled: true,
    k_clusters: 6,
    include_bbox: true,
    ocr_coverage_threshold: Some(0.5),
};
```

```python title="Python - HierarchyConfig Setup"
from kreuzberg import HierarchyConfig, ExtractionConfig, PdfConfig

hierarchy = HierarchyConfig(
    enabled=True,
    k_clusters=6,
    include_bbox=True,
    ocr_coverage_threshold=0.5
)

pdf_config = PdfConfig(hierarchy=hierarchy)
config = ExtractionConfig(pdf_options=pdf_config)
```

```typescript title="TypeScript - HierarchyConfig Setup"
const hierarchyConfig: HierarchyConfig = {
  enabled: true,
  kClusters: 6,
  includeBbox: true,
  ocrCoverageThreshold: 0.5,
};

const pdfConfig: PdfConfig = {
  hierarchy: hierarchyConfig,
};
```

```java title="Java - HierarchyConfig Setup"
HierarchyConfig hierarchyConfig = HierarchyConfig.builder()
    .enabled(true)
    .kClusters(6)
    .includeBbox(true)
    .ocrCoverageThreshold(0.5)
    .build();

PdfConfig pdfConfig = PdfConfig.builder()
    .hierarchy(hierarchyConfig)
    .build();
```

```go title="Go - HierarchyConfig Setup"
hierarchyConfig := &kreuzberg.HierarchyConfig{
    Enabled:              kreuzberg.BoolPtr(true),
    KClusters:           kreuzberg.IntPtr(6),
    IncludeBbox:         kreuzberg.BoolPtr(true),
    OcrCoverageThreshold: kreuzberg.FloatPtr(0.5),
}

pdfConfig := &kreuzberg.PdfConfig{
    Hierarchy: hierarchyConfig,
}
```

## PageHierarchy

Output structure containing extracted document hierarchy with text blocks and their hierarchy levels. Returned in extraction results when hierarchy extraction is enabled.

### Rust

```rust title="page_hierarchy.rs"
pub struct PageHierarchy {
    // Total number of hierarchy blocks extracted from the page
    pub block_count: usize,

    // Array of hierarchical text blocks ordered by document position
    pub blocks: Vec<HierarchicalBlock>,
}
```

### Python

```python title="page_hierarchy.py"
class PageHierarchy(TypedDict):
    """Document hierarchy structure with text blocks and levels."""
    block_count: int
    blocks: list[HierarchicalBlock]
```

### TypeScript

```typescript title="page_hierarchy.ts"
export interface PageHierarchy {
  /** Total number of hierarchy blocks extracted from the page */
  blockCount: number;

  /** Array of hierarchical text blocks ordered by document position */
  blocks: HierarchicalBlock[];
}
```

### Ruby

```ruby title="page_hierarchy.rb"
class Kreuzberg::Result::PageHierarchy
    attr_reader :block_count, :blocks
end
```

### Java

```java title="PageHierarchy.java"
public record PageHierarchy(
    int blockCount,
    List<HierarchicalBlock> blocks
) {}
```

### Go

```go title="page_hierarchy.go"
type PageHierarchy struct {
    BlockCount int                   `json:"block_count"`
    Blocks     []HierarchicalBlock    `json:"blocks"`
}
```

### C#

```csharp title="PageHierarchy.cs"
public record PageHierarchy
{
    [JsonPropertyName("block_count")]
    public required int BlockCount { get; init; }

    [JsonPropertyName("blocks")]
    public required List<HierarchicalBlock> Blocks { get; init; }
}
```

**Fields:**

- `block_count`: Total number of text blocks in the hierarchy (useful for batch processing)
- `blocks`: Array of `HierarchicalBlock` objects in document order (top-to-bottom, left-to-right)

## HierarchicalBlock

A single text block with assigned hierarchy level and spatial information. Represents a unit of text (heading or body paragraph) within the document structure.

### Rust

```rust title="hierarchical_block.rs"
#[derive(Debug, Clone)]
pub struct HierarchicalBlock {
    // The text content of this block
    pub text: String,

    // Hierarchy level: "h1", "h2", "h3", "h4", "h5", "h6", or "body"
    pub level: HierarchyLevel,

    // Font size in points (derived from PDF or OCR)
    pub font_size: f32,

    /// Bounding box coordinates in PDF units (if include_bbox=true)
    /// Format: (left, top, right, bottom)
    pub bbox: Option<BoundingBox>,

    // Index position of this block in the blocks array
    pub block_index: usize,
}

pub enum HierarchyLevel {
    H1 = 1,
    H2 = 2,
    H3 = 3,
    H4 = 4,
    H5 = 5,
    H6 = 6,
    Body = 0,
}

pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}
```

### Python

```python title="hierarchical_block.py"
class HierarchicalBlock(TypedDict, total=False):
    """A text block with hierarchy level assignment."""
    text: str
    level: Literal["h1", "h2", "h3", "h4", "h5", "h6", "body"]
    font_size: float
    bbox: tuple[float, float, float, float] | None
    block_index: int
```

### TypeScript

```typescript title="hierarchical_block.ts"
export interface HierarchicalBlock {
  /** The text content of this block */
  text: string;

  /** Hierarchy level: "h1" through "h6" or "body" */
  level: "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "body";

  /** Font size in points */
  fontSize: number;

  /** Bounding box [left, top, right, bottom] in PDF coordinates, or null */
  bbox?: [number, number, number, number] | null;

  /** Index position of this block in the blocks array */
  blockIndex: number;
}
```

### Ruby

```ruby title="hierarchical_block.rb"
class Kreuzberg::Result::HierarchicalBlock
    attr_reader :text, :level, :font_size, :bbox, :block_index
end
```

### Java

```java title="HierarchicalBlock.java"
public record HierarchicalBlock(
    String text,
    String level,  // "h1", "h2", ..., "h6", "body"
    float fontSize,
    Optional<BoundingBox> bbox,
    int blockIndex
) {}

public record BoundingBox(
    float left,
    float top,
    float right,
    float bottom
) {}
```

### Go

```go title="hierarchical_block.go"
type HierarchicalBlock struct {
    Text       string      `json:"text"`
    Level      string      `json:"level"`  // "h1", "h2", ..., "h6", "body"
    FontSize   float32     `json:"font_size"`
    Bbox       *BoundingBox `json:"bbox,omitempty"`
    BlockIndex int         `json:"block_index"`
}

type BoundingBox struct {
    Left   float32 `json:"left"`
    Top    float32 `json:"top"`
    Right  float32 `json:"right"`
    Bottom float32 `json:"bottom"`
}
```

### C#

```csharp title="HierarchicalBlock.cs"
public record HierarchicalBlock
{
    [JsonPropertyName("text")]
    public required string Text { get; init; }

    [JsonPropertyName("level")]
    public required string Level { get; init; }  // "h1", "h2", ..., "h6", "body"

    [JsonPropertyName("font_size")]
    public required float FontSize { get; init; }

    [JsonPropertyName("bbox")]
    public BoundingBox? Bbox { get; init; }

    [JsonPropertyName("block_index")]
    public required int BlockIndex { get; init; }
}

public record BoundingBox
{
    [JsonPropertyName("left")]
    public required float Left { get; init; }

    [JsonPropertyName("top")]
    public required float Top { get; init; }

    [JsonPropertyName("right")]
    public required float Right { get; init; }

    [JsonPropertyName("bottom")]
    public required float Bottom { get; init; }
}
```

**Fields:**

- `text`: Complete text content of the block (normalized and trimmed)
  - Whitespace is collapsed for consistency
  - Preserves original character content
  - Empty strings are included in output

- `level`: Hierarchy level classification
  - `"h1"` through `"h6"`: Heading levels assigned by font size clustering
  - `"body"`: Body text or smaller headings (cluster 6+)
  - Assignment based on font size centroid similarity

- `font_size`: Average font size of text in this block (in points)
  - Derived from PDF font metrics or OCR confidence
  - Used internally for hierarchy level assignment
  - Useful for downstream styling or filtering

- `bbox`: Bounding box in PDF coordinate system (optional)
  - Format: `[left, top, right, bottom]` in PDF units
  - Top-left origin (0,0), Y increases downward
  - `null` when `include_bbox=false` in config
  - Enables precise text positioning, highlighting, or spatial queries
  - Coordinates are in points (1/72 inch)

- `block_index`: Zero-indexed position in the blocks array
  - Useful for document position tracking
  - Enables block-to-source mapping
  - Matches order in extraction output

**Hierarchy Level Assignment Algorithm:**

1. Extract font sizes from all text blocks
2. Apply K-means clustering with k=`k_clusters` parameter
3. Sort clusters by centroid size (descending)
4. Map clusters to hierarchy levels:
   - Cluster 0 (largest font) → H1
   - Cluster 1 → H2
   - Cluster 2 → H3
   - Cluster 3 → H4
   - Cluster 4 → H5
   - Cluster 5 → H6
   - Cluster 6+ (smallest font) → Body
5. Assign levels based on block's font size similarity to cluster centroids

**Example Usage:**

```rust title="Rust - Iterating PageHierarchy Blocks"
use kreuzberg::types::ExtractionResult;

if let Some(pages) = result.pages {
    for page in pages {
        if let Some(hierarchy) = &page.hierarchy {
            println!("Found {} blocks:", hierarchy.block_count);
            for block in &hierarchy.blocks {
                println!("  [{:?}] {}", block.level, block.text);
                if let Some(bbox) = &block.bbox {
                    println!("    Position: ({}, {}) to ({}, {})",
                        bbox.left, bbox.top, bbox.right, bbox.bottom);
                }
            }
        }
    }
}
```

```python title="Python - Iterating PageHierarchy Blocks"
from kreuzberg import extract_file

result = extract_file('document.pdf')

if result.get('pages'):
    for page in result['pages']:
        if 'hierarchy' in page:
            hierarchy = page['hierarchy']
            print(f"Found {hierarchy['block_count']} blocks:")
            for block in hierarchy['blocks']:
                print(f"  [{block['level']}] {block['text']}")
                if block.get('bbox'):
                    left, top, right, bottom = block['bbox']
                    print(f"    Position: ({left}, {top}) to ({right}, {bottom})")
```

```typescript title="TypeScript - Iterating PageHierarchy Blocks"
import { extract } from "kreuzberg";

const result = await extract("document.pdf");

if (result.pages) {
  for (const page of result.pages) {
    if (page.hierarchy) {
      const { blockCount, blocks } = page.hierarchy;
      console.log(`Found ${blockCount} blocks:`);
      for (const block of blocks) {
        console.log(`  [${block.level}] ${block.text}`);
        if (block.bbox) {
          const [left, top, right, bottom] = block.bbox;
          console.log(
            `    Position: (${left}, ${top}) to (${right}, ${bottom})`,
          );
        }
      }
    }
  }
}
```

```java title="Java - Iterating PageHierarchy Blocks"
ExtractionResult result = kreuzberg.extract(new File("document.pdf"));

if (result.pages() != null) {
    for (PageContent page : result.pages()) {
        page.hierarchy().ifPresent(hierarchy -> {
            System.out.println("Found " + hierarchy.blockCount() + " blocks:");
            for (HierarchicalBlock block : hierarchy.blocks()) {
                System.out.println("  [" + block.level() + "] " + block.text());
                block.bbox().ifPresent(bbox -> {
                    System.out.printf("    Position: (%.1f, %.1f) to (%.1f, %.1f)%n",
                        bbox.left(), bbox.top(), bbox.right(), bbox.bottom());
                });
            }
        });
    }
}
```

```go title="Go - Iterating PageHierarchy Blocks"
result, _ := kreuzberg.Extract("document.pdf", nil)

if result.Pages != nil {
    for _, page := range result.Pages {
        if page.Hierarchy != nil {
            hierarchy := page.Hierarchy
            fmt.Printf("Found %d blocks:\n", hierarchy.BlockCount)
            for _, block := range hierarchy.Blocks {
                fmt.Printf("  [%s] %s\n", block.Level, block.Text)
                if block.Bbox != nil {
                    bbox := block.Bbox
                    fmt.Printf("    Position: (%.1f, %.1f) to (%.1f, %.1f)\n",
                        bbox.Left, bbox.Top, bbox.Right, bbox.Bottom)
                }
            }
        }
    }
}
```

**Common Use Cases:**

1. **Document Structure Extraction**: Build table of contents from H1-H6 blocks
2. **Content Filtering**: Extract only body text or headings at specific levels
3. **Spatial Highlighting**: Use bbox coordinates for PDF annotation and visual markup
4. **Semantic Chunking**: Group blocks by hierarchy level for AI processing
5. **Accessibility**: Generate proper HTML semantic structure from hierarchy levels
6. **Document Analysis**: Calculate reading complexity and structure metrics

## Type Mappings

Cross-language type equivalents showing how Kreuzberg types map across Rust, Python, TypeScript, Ruby, Java, and Go:

| Purpose           | Rust                  | Python        | TypeScript     | Ruby              | Java              | Go                   |
| ----------------- | --------------------- | ------------- | -------------- | ----------------- | ----------------- | -------------------- |
| String            | `String`              | `str`         | `string`       | `String`          | `String`          | `string`             |
| Optional/Nullable | `Option<T>`           | `T \| None`   | `T \| null`    | `T or nil`        | `Optional<T>`     | `*T`                 |
| Array/List        | `Vec<T>`              | `list[T]`     | `T[]`          | `Array`           | `List<T>`         | `[]T`                |
| Tuple/Pair        | `(T, U)`              | `tuple[T, U]` | `[T, U]`       | `Array`           | `Pair<T,U>`       | `[2]T`               |
| Dictionary/Map    | `HashMap<K,V>`        | `dict[K, V]`  | `Record<K, V>` | `Hash`            | `Map<K, V>`       | `map[K]V`            |
| Integer           | `i32`, `i64`, `usize` | `int`         | `number`       | `Integer`         | `int`, `long`     | `int`, `int64`       |
| Float             | `f32`, `f64`          | `float`       | `number`       | `Float`           | `float`, `double` | `float32`, `float64` |
| Boolean           | `bool`                | `bool`        | `boolean`      | `Boolean`         | `boolean`         | `bool`               |
| Bytes             | `Vec<u8>`             | `bytes`       | `Uint8Array`   | `String` (binary) | `byte[]`          | `[]byte`             |
| Union/Enum        | `enum`                | `Literal`     | `union`        | `case` statement  | `sealed class`    | custom struct        |

## Nullability and Optionals

### Language-Specific Optional Field Handling

Each language binding uses its idiomatic approach for representing optional and nullable values:

**Rust**: Uses `Option<T>` explicitly. `None` represents absence. Mandatory at compile-time.

**Python**: Uses `T | None` type hints. Can be `None` at runtime. TypedDict with `total=False` makes all fields optional.

**TypeScript**: Uses `T | null` or `T | undefined`. Properties marked `?` are optional. Nullable with `null` literal.

**Ruby**: Everything is nullable. Use `nil` for absence. No type system enforcement.

**Java**: Uses `Optional<T>` for explicit optionality. Records with `Optional` fields. Checked at compile-time for clarity.

**Go**: Uses pointers (`*T`) for optional values. `nil` represents absence. Primitive types can't be nil (use pointers).

### Practical Examples: Accessing Optional Metadata Fields

Demonstrating idiomatic null-safe field access patterns across all supported languages:

```rust title="optional_field_access.rs"
// Rust: Pattern matching for safe optional field access
if let Some(title) = metadata.format.pdf.title {
    println!("Title: {}", title);
}
```

```python title="optional_field_access.py"
# Python: Dictionary-based metadata access with safe get method
if metadata.get("title"):
    print(f"Title: {metadata['title']}")
```

```typescript title="optional_field_access.ts"
// TypeScript: Nullish coalescing for default values
console.log(metadata.title ?? "No title");
```

```ruby title="optional_field_access.rb"
# Ruby: Conditional output with truthy check
puts "Title: #{result.metadata["title"]}" if result.metadata["title"]
```

```java title="OptionalFieldAccess.java"
// Java: Functional-style Optional handling with ifPresent
metadata.title()
    .ifPresent(title -> System.out.println("Title: " + title));
```

```go title="optional_field_access.go"
// Go: Nil-safe pointer dereferencing with nested checks
if metadata.Pdf != nil && metadata.Pdf.Title != nil {
    fmt.Println("Title:", *metadata.Pdf.Title)
}
```

## Element

Semantic element extracted from a document when using element-based output. Each element represents a specific document component (title, paragraph, list item, table, etc.) with associated metadata.

### Rust

```rust title="element.rs"
pub struct Element {
    pub element_id: ElementId,
    pub element_type: ElementType,
    pub text: String,
    pub metadata: ElementMetadata,
}
```

### Python

```python title="element.py"
class Element(TypedDict):
    """Semantic element from element-based extraction."""
    element_id: str
    element_type: ElementType
    text: str
    metadata: ElementMetadata
```

### TypeScript

```typescript title="element.ts"
export interface Element {
  elementId: string;
  elementType: ElementType;
  text: string;
  metadata: ElementMetadata;
}
```

### Ruby

```ruby title="element.rb"
class Kreuzberg::Element
    attr_reader :element_id, :element_type, :text, :metadata
end
```

### Java

```java title="Element.java"
public record Element(
    String elementId,
    ElementType elementType,
    String text,
    ElementMetadata metadata
) {}
```

### Go

```go title="element.go"
type Element struct {
    ElementID   string          `json:"element_id"`
    ElementType ElementType     `json:"element_type"`
    Text        string          `json:"text"`
    Metadata    ElementMetadata `json:"metadata"`
}
```

### C#

```csharp title="Element.cs"
public record Element(
    string ElementId,
    ElementType ElementType,
    string Text,
    ElementMetadata Metadata
);
```

### PHP

```php title="Element.php"
class Element {
    public string $elementId;
    public ElementType $elementType;
    public string $text;
    public ElementMetadata $metadata;
}
```

### Elixir

```elixir title="element.ex"
defmodule Kreuzberg.Element do
  @type t :: %__MODULE__{
    element_id: String.t(),
    element_type: atom(),
    text: String.t(),
    metadata: Kreuzberg.ElementMetadata.t()
  }

  defstruct [:element_id, :element_type, :text, :metadata]
end
```

### WASM

```typescript title="element.ts"
export interface Element {
  elementId: string;
  elementType: ElementType;
  text: string;
  metadata: ElementMetadata;
}
```

## ElementId

Unique identifier for semantic elements. A newtype wrapper around a string that provides deterministic, content-based identification for elements.

### Rust

```rust title="element_id.rs"
pub struct ElementId(String);
```

In Rust, ElementId is a newtype wrapper providing type safety. It is serialized/deserialized as a plain string in JSON.

### Python

```python title="element_id.py"
# ElementId is represented as a plain string in Python
element_id: str
```

### TypeScript

```typescript title="element_id.ts"
// ElementId is represented as a plain string in TypeScript
type ElementId = string;
```

### Ruby

```ruby title="element_id.rb"
# ElementId is represented as a plain string in Ruby
element_id # => String
```

### Java

```java title="ElementId.java"
// ElementId is represented as a String in Java
String elementId;
```

### Go

```go title="element_id.go"
// ElementId is represented as a string in Go
type ElementID string
```

### C#

```csharp title="ElementId.cs"
// ElementId is represented as a string in C#
string ElementId { get; }
```

ElementId values are deterministically generated from element type, content, and page number, ensuring stable identifiers across extraction runs.

## ElementType

Enumeration of semantic element types extracted from documents.

### Rust

```rust title="element_type.rs"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    Title,
    NarrativeText,
    ListItem,
    Table,
    Image,
    PageBreak,
    Heading,
    CodeBlock,
    BlockQuote,
    Header,
    Footer,
}
```

### Python

```python title="element_type.py"
from typing import Literal

ElementType = Literal[
    "title",
    "narrative_text",
    "list_item",
    "table",
    "image",
    "page_break",
    "heading",
    "code_block",
    "block_quote",
    "header",
    "footer"
]
```

### TypeScript

```typescript title="element_type.ts"
export type ElementType =
  | "title"
  | "narrative_text"
  | "list_item"
  | "table"
  | "image"
  | "page_break"
  | "heading"
  | "code_block"
  | "block_quote"
  | "header"
  | "footer";
```

### Ruby

```ruby title="element_type.rb"
module Kreuzberg
    module ElementType
        TITLE = 'title'
        NARRATIVE_TEXT = 'narrative_text'
        LIST_ITEM = 'list_item'
        TABLE = 'table'
        IMAGE = 'image'
        PAGE_BREAK = 'page_break'
        HEADING = 'heading'
        CODE_BLOCK = 'code_block'
        BLOCK_QUOTE = 'block_quote'
        HEADER = 'header'
        FOOTER = 'footer'
    end
end
```

### Java

```java title="ElementType.java"
public enum ElementType {
    TITLE("title"),
    NARRATIVE_TEXT("narrative_text"),
    LIST_ITEM("list_item"),
    TABLE("table"),
    IMAGE("image"),
    PAGE_BREAK("page_break"),
    HEADING("heading"),
    CODE_BLOCK("code_block"),
    BLOCK_QUOTE("block_quote"),
    HEADER("header"),
    FOOTER("footer");

    private final String value;

    ElementType(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}
```

### Go

```go title="element_type.go"
type ElementType string

const (
    ElementTypeTitle         ElementType = "title"
    ElementTypeNarrativeText ElementType = "narrative_text"
    ElementTypeListItem      ElementType = "list_item"
    ElementTypeTable         ElementType = "table"
    ElementTypeImage         ElementType = "image"
    ElementTypePageBreak     ElementType = "page_break"
    ElementTypeHeading       ElementType = "heading"
    ElementTypeCodeBlock     ElementType = "code_block"
    ElementTypeBlockQuote    ElementType = "block_quote"
    ElementTypeHeader        ElementType = "header"
    ElementTypeFooter        ElementType = "footer"
)
```

### C#

```csharp title="ElementType.cs"
public enum ElementType
{
    Title,
    NarrativeText,
    ListItem,
    Table,
    Image,
    PageBreak,
    Heading,
    CodeBlock,
    BlockQuote,
    Header,
    Footer
}
```

### PHP

```php title="ElementType.php"
enum ElementType: string {
    case TITLE = 'title';
    case NARRATIVE_TEXT = 'narrative_text';
    case LIST_ITEM = 'list_item';
    case TABLE = 'table';
    case IMAGE = 'image';
    case PAGE_BREAK = 'page_break';
    case HEADING = 'heading';
    case CODE_BLOCK = 'code_block';
    case BLOCK_QUOTE = 'block_quote';
    case HEADER = 'header';
    case FOOTER = 'footer';
}
```

### Elixir

```elixir title="element_type.ex"
defmodule Kreuzberg.ElementType do
  @type t :: :title
           | :narrative_text
           | :list_item
           | :table
           | :image
           | :page_break
           | :heading
           | :code_block
           | :block_quote
           | :header
           | :footer
end
```

### WASM

```typescript title="element_type.ts"
export type ElementType =
  | "title"
  | "narrative_text"
  | "list_item"
  | "table"
  | "image"
  | "page_break"
  | "heading"
  | "code_block"
  | "block_quote"
  | "header"
  | "footer";
```

## ElementMetadata

Metadata associated with each extracted element, including page numbers, coordinates, and element-type-specific information.

### Rust

```rust title="element_metadata.rs"
pub struct ElementMetadata {
    pub page_number: Option<usize>,
    pub filename: Option<String>,
    pub coordinates: Option<BoundingBox>,
    pub element_index: Option<usize>,
    pub additional: HashMap<String, String>,
}
```

### Python

```python title="element_metadata.py"
class ElementMetadata(TypedDict, total=False):
    """Metadata for extracted elements."""
    page_number: int | None
    filename: str | None
    coordinates: BoundingBox | None
    element_index: int | None
    additional: dict[str, str]
```

### TypeScript

```typescript title="element_metadata.ts"
export interface ElementMetadata {
  pageNumber?: number | null;
  filename?: string | null;
  coordinates?: BoundingBox | null;
  elementIndex?: number | null;
  additional?: Record<string, string>;
}
```

### Ruby

```ruby title="element_metadata.rb"
class Kreuzberg::ElementMetadata
    attr_reader :page_number, :filename, :coordinates
    attr_reader :element_index, :additional
end
```

### Java

```java title="ElementMetadata.java"
public record ElementMetadata(
    Integer pageNumber,
    String filename,
    BoundingBox coordinates,
    Integer elementIndex,
    Map<String, String> additional
) {}
```

### Go

```go title="element_metadata.go"
type ElementMetadata struct {
    PageNumber   *int               `json:"page_number,omitempty"`
    Filename     *string            `json:"filename,omitempty"`
    Coordinates  *BoundingBox       `json:"coordinates,omitempty"`
    ElementIndex *int               `json:"element_index,omitempty"`
    Additional   map[string]string  `json:"additional,omitempty"`
}
```

### C#

```csharp title="ElementMetadata.cs"
public record ElementMetadata(
    int? PageNumber,
    string? Filename,
    BoundingBox? Coordinates,
    int? ElementIndex,
    Dictionary<string, string>? Additional
);
```

### PHP

```php title="ElementMetadata.php"
class ElementMetadata {
    public ?int $pageNumber;
    public ?string $filename;
    public ?BoundingBox $coordinates;
    public ?int $elementIndex;
    public array $additional; // array<string, string>
}
```

### Elixir

```elixir title="element_metadata.ex"
defmodule Kreuzberg.ElementMetadata do
  @type t :: %__MODULE__{
    page_number: integer() | nil,
    filename: String.t() | nil,
    coordinates: Kreuzberg.BoundingBox.t() | nil,
    element_index: integer() | nil,
    additional: %{optional(String.t()) => String.t()}
  }

  defstruct [:page_number, :filename, :coordinates, :element_index, :additional]
end
```

### WASM

```typescript title="element_metadata.ts"
export interface ElementMetadata {
  pageNumber?: number | null;
  filename?: string | null;
  coordinates?: BoundingBox | null;
  elementIndex?: number | null;
  additional?: Record<string, string>;
}
```

## BoundingBox

Rectangular bounding box coordinates for element positioning. Coordinates are in PDF points (1/72 inch) with origin typically at bottom-left (PDF) or top-left (HTML).

### Rust

```rust title="bounding_box.rs"
pub struct BoundingBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
```

### Python

```python title="bounding_box.py"
class BoundingBox(TypedDict):
    """Rectangular bounding box coordinates."""
    x0: float
    y0: float
    x1: float
    y1: float
```

### TypeScript

```typescript title="bounding_box.ts"
export interface BoundingBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}
```

### Ruby

```ruby title="bounding_box.rb"
class Kreuzberg::BoundingBox
    attr_reader :x0, :y0, :x1, :y1
end
```

### Java

```java title="BoundingBox.java"
public record BoundingBox(
    double x0,
    double y0,
    double x1,
    double y1
) {}
```

### Go

```go title="bounding_box.go"
type BoundingBox struct {
    X0 float64 `json:"x0"`
    Y0 float64 `json:"y0"`
    X1 float64 `json:"x1"`
    Y1 float64 `json:"y1"`
}
```

### C#

```csharp title="BoundingBox.cs"
public record BoundingBox(
    double X0,
    double Y0,
    double X1,
    double Y1
);
```

### PHP

```php title="BoundingBox.php"
class BoundingBox {
    public float $x0;
    public float $y0;
    public float $x1;
    public float $y1;
}
```

### Elixir

```elixir title="bounding_box.ex"
defmodule Kreuzberg.BoundingBox do
  @type t :: %__MODULE__{
    x0: float(),
    y0: float(),
    x1: float(),
    y1: float()
  }

  defstruct [:x0, :y0, :x1, :y1]
end
```

### WASM

```typescript title="bounding_box.ts"
export interface BoundingBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}
```

## OutputFormat (Result Structure)

Output format selection for extraction results. Controls whether results are returned in unified format (default) or element-based format (Unstructured.io compatible).

**Note:** This is used by the `result_format` configuration field. For content format options (Plain/Markdown/Djot/Html), see [ContentFormat](#contentformat-output-format).

### Rust

```rust title="output_format.rs"
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Unified,
    ElementBased,
}
```

### Python

```python title="output_format.py"
from typing import Literal

OutputFormat = Literal["unified", "element_based", "elements"]
```

### TypeScript

```typescript title="output_format.ts"
export type OutputFormat = "unified" | "element_based" | "elements";
```

### Ruby

```ruby title="output_format.rb"
module Kreuzberg
    module OutputFormat
        UNIFIED = 'unified'
        ELEMENT_BASED = 'element_based'
        ELEMENTS = 'elements'  # Alias for element_based
    end
end
```

### Java

```java title="OutputFormat.java"
public enum OutputFormat {
    UNIFIED("unified"),
    ELEMENT_BASED("element_based");

    private final String value;

    OutputFormat(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}
```

### Go

```go title="output_format.go"
type OutputFormat string

const (
    OutputFormatUnified      OutputFormat = "unified"
    OutputFormatElementBased OutputFormat = "element_based"
    OutputFormatElements     OutputFormat = "elements"  // Alias
)
```

### C#

```csharp title="OutputFormat.cs"
public enum OutputFormat
{
    Unified,
    ElementBased
}
```

### PHP

```php title="OutputFormat.php"
enum OutputFormat: string {
    case UNIFIED = 'unified';
    case ELEMENT_BASED = 'element_based';
    case ELEMENTS = 'elements';  // Alias
}
```

### Elixir

```elixir title="output_format.ex"
defmodule Kreuzberg.OutputFormat do
  @type t :: :unified | :element_based | :elements
end
```

### WASM

```typescript title="output_format.ts"
export type OutputFormat = "unified" | "element_based" | "elements";
```

**Usage Notes:**

- `unified` (default): Returns complete document text in `content` field with metadata, tables, images, and optional pages
- `element_based` or `elements`: Returns array of semantic elements in `elements` field (Unstructured.io compatible)
- Both formats can coexist: enabling `element_based` populates `elements` while maintaining `content`, `tables`, etc.

## ContentFormat (Output Format)

Content format selection for extracted text. Controls whether extracted content is returned as plain text, Markdown, Djot, or HTML.

**Note:** This is used by the `output_format` configuration field. For result structure options (Unified/ElementBased), see [OutputFormat (Result Structure)](#outputformat-result-structure).

### Rust

```rust title="content_format.rs"
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Plain,
    Markdown,
    Djot,
    Html,
}
```

### Python

```python title="content_format.py"
from typing import Literal

ContentFormat = Literal["plain", "markdown", "djot", "html"]
```

### TypeScript

```typescript title="content_format.ts"
export type ContentFormat = "plain" | "markdown" | "djot" | "html";
```

### Java

```java title="ContentFormat.java"
public enum ContentFormat {
    PLAIN("plain"),
    MARKDOWN("markdown"),
    DJOT("djot"),
    HTML("html");

    private final String value;

    ContentFormat(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}
```

### Go

```go title="content_format.go"
type ContentFormat string

const (
    ContentFormatPlain    ContentFormat = "plain"
    ContentFormatMarkdown ContentFormat = "markdown"
    ContentFormatDjot     ContentFormat = "djot"
    ContentFormatHtml     ContentFormat = "html"
)
```

### Ruby

```ruby title="content_format.rb"
module Kreuzberg
    module ContentFormat
        PLAIN = 'plain'
        MARKDOWN = 'markdown'
        DJOT = 'djot'
        HTML = 'html'
    end
end
```

**Usage Notes:**

- `plain` (default): Returns plain text content
- `markdown`: Returns content formatted as Markdown
- `djot`: Returns content formatted as Djot (lightweight markup format)
- `html`: Returns content as HTML
- When `djot` is selected and the document is in Djot format, the `djot_content` field on `ExtractionResult` will be populated with structured Djot AST data

## DocumentStructure

Tree-based representation of document structure when `include_document_structure` is enabled in ExtractionConfig. Contains a hierarchical model of document nodes representing headings, paragraphs, lists, tables, and other semantic content.

### Rust

```rust title="document_structure.rs"
pub struct DocumentStructure {
    pub nodes: Vec<DocumentNode>,
}

pub struct DocumentNode {
    pub id: NodeId,
    pub content: NodeContent,
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,
    pub content_layer: ContentLayer,
    pub page: Option<u32>,
    pub page_end: Option<u32>,
    pub bbox: Option<BoundingBox>,
    pub annotations: Vec<TextAnnotation>,
}

pub struct NodeId(pub String);

pub type NodeIndex = u32;

#[serde(tag = "node_type")]
pub enum NodeContent {
    Title { text: String },
    Heading { level: u8, text: String },
    Paragraph { text: String },
    List { ordered: bool },
    ListItem { text: String },
    Table { grid: TableGrid },
    Image { description: Option<String>, image_index: Option<usize> },
    Code { text: String, language: Option<String> },
    Quote,
    Formula { text: String },
    Footnote { text: String },
    Group { label: Option<String>, heading_level: Option<u8>, heading_text: Option<String> },
    PageBreak,
}

pub enum ContentLayer {
    Body,
    Header,
    Footer,
    Footnote,
}

pub struct TableGrid {
    pub rows: u32,
    pub cols: u32,
    pub cells: Vec<GridCell>,
}

pub struct GridCell {
    pub content: String,
    pub row: u32,
    pub col: u32,
    pub row_span: u32,
    pub col_span: u32,
    pub is_header: bool,
    pub bbox: Option<BoundingBox>,
}

pub struct TextAnnotation {
    pub start: u32,
    pub end: u32,
    pub kind: AnnotationKind,
}

#[serde(tag = "annotation_type")]
pub enum AnnotationKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
    Subscript,
    Superscript,
    Link { url: String, title: Option<String> },
}
```

### Python

```python title="document_structure.py"
class DocumentStructure(TypedDict):
    nodes: list[DocumentNode]

class DocumentNode(TypedDict):
    id: str
    content: NodeContent
    parent: int | None
    children: list[int]
    content_layer: ContentLayer
    page: int | None
    page_end: int | None
    bbox: BoundingBox | None
    annotations: list[TextAnnotation]

ContentLayer = Literal["body", "header", "footer", "footnote"]

class NodeContent(TypedDict, total=False):
    node_type: str  # Discriminator: "title", "heading", "paragraph", etc.
    text: str
    level: int
    ordered: bool
    grid: TableGrid
    description: str
    image_index: int
    language: str
    label: str
    heading_level: int
    heading_text: str

class TableGrid(TypedDict):
    rows: int
    cols: int
    cells: list[GridCell]

class GridCell(TypedDict):
    content: str
    row: int
    col: int
    row_span: int
    col_span: int
    is_header: bool
    bbox: BoundingBox | None

class TextAnnotation(TypedDict):
    start: int
    end: int
    kind: AnnotationKind

class AnnotationKind(TypedDict, total=False):
    annotation_type: str  # Discriminator: "bold", "italic", etc.
    url: str
    title: str
```

### TypeScript

```typescript title="document_structure.ts"
export interface DocumentStructure {
  nodes: DocumentNode[];
}

export interface DocumentNode {
  id: string;
  content: NodeContent;
  parent: number | null;
  children: number[];
  contentLayer: ContentLayer;
  page: number | null;
  pageEnd: number | null;
  bbox: BoundingBox | null;
  annotations: TextAnnotation[];
}

export type ContentLayer = "body" | "header" | "footer" | "footnote";

export type NodeContent =
  | { nodeType: "title"; text: string }
  | { nodeType: "heading"; level: number; text: string }
  | { nodeType: "paragraph"; text: string }
  | { nodeType: "list"; ordered: boolean }
  | { nodeType: "listItem"; text: string }
  | { nodeType: "table"; grid: TableGrid }
  | { nodeType: "image"; description?: string; imageIndex?: number }
  | { nodeType: "code"; text: string; language?: string }
  | { nodeType: "quote" }
  | { nodeType: "formula"; text: string }
  | { nodeType: "footnote"; text: string }
  | {
      nodeType: "group";
      label?: string;
      headingLevel?: number;
      headingText?: string;
    }
  | { nodeType: "pageBreak" };

export interface TableGrid {
  rows: number;
  cols: number;
  cells: GridCell[];
}

export interface GridCell {
  content: string;
  row: number;
  col: number;
  rowSpan: number;
  colSpan: number;
  isHeader: boolean;
  bbox: BoundingBox | null;
}

export interface TextAnnotation {
  start: number;
  end: number;
  kind: AnnotationKind;
}

export type AnnotationKind =
  | { annotationType: "bold" }
  | { annotationType: "italic" }
  | { annotationType: "underline" }
  | { annotationType: "strikethrough" }
  | { annotationType: "code" }
  | { annotationType: "subscript" }
  | { annotationType: "superscript" }
  | { annotationType: "link"; url: string; title?: string };
```

### Ruby

```ruby title="document_structure.rb"
class Kreuzberg::DocumentStructure
    attr_reader :nodes

    def initialize(nodes:)
        @nodes = nodes
    end
end

class Kreuzberg::DocumentNode
    attr_reader :id, :content, :parent, :children, :content_layer, :page, :page_end, :bbox, :annotations

    def initialize(id:, content:, parent:, children:, content_layer:, page:, page_end:, bbox:, annotations:)
        @id = id
        @content = content
        @parent = parent
        @children = children
        @content_layer = content_layer
        @page = page
        @page_end = page_end
        @bbox = bbox
        @annotations = annotations
    end
end

module Kreuzberg::ContentLayer
    BODY = "body"
    HEADER = "header"
    FOOTER = "footer"
    FOOTNOTE = "footnote"
end

# NodeContent and AnnotationKind are returned as Hashes with node_type/annotation_type discriminators
```

### Java

```java title="DocumentStructure.java"
public record DocumentStructure(
    List<DocumentNode> nodes
) {}

public record DocumentNode(
    String id,
    NodeContent content,
    Integer parent,
    List<Integer> children,
    ContentLayer contentLayer,
    Integer page,
    Integer pageEnd,
    BoundingBox bbox,
    List<TextAnnotation> annotations
) {}

public enum ContentLayer {
    BODY("body"),
    HEADER("header"),
    FOOTER("footer"),
    FOOTNOTE("footnote");

    private final String value;

    ContentLayer(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}

public sealed interface NodeContent {
    record Title(String text) implements NodeContent {}
    record Heading(int level, String text) implements NodeContent {}
    record Paragraph(String text) implements NodeContent {}
    record List(boolean ordered) implements NodeContent {}
    record ListItem(String text) implements NodeContent {}
    record Table(TableGrid grid) implements NodeContent {}
    record Image(Optional<String> description, Optional<Integer> imageIndex) implements NodeContent {}
    record Code(String text, Optional<String> language) implements NodeContent {}
    record Quote() implements NodeContent {}
    record Formula(String text) implements NodeContent {}
    record Footnote(String text) implements NodeContent {}
    record Group(Optional<String> label, Optional<Integer> headingLevel, Optional<String> headingText) implements NodeContent {}
    record PageBreak() implements NodeContent {}
}

public record TableGrid(
    int rows,
    int cols,
    List<GridCell> cells
) {}

public record GridCell(
    String content,
    int row,
    int col,
    int rowSpan,
    int colSpan,
    boolean isHeader,
    BoundingBox bbox
) {}

public record TextAnnotation(
    int start,
    int end,
    AnnotationKind kind
) {}

public sealed interface AnnotationKind {
    record Bold() implements AnnotationKind {}
    record Italic() implements AnnotationKind {}
    record Underline() implements AnnotationKind {}
    record Strikethrough() implements AnnotationKind {}
    record Code() implements AnnotationKind {}
    record Subscript() implements AnnotationKind {}
    record Superscript() implements AnnotationKind {}
    record Link(String url, Optional<String> title) implements AnnotationKind {}
}
```

### Go

```go title="document_structure.go"
type DocumentStructure struct {
    Nodes []DocumentNode `json:"nodes"`
}

type DocumentNode struct {
    ID             string                 `json:"id"`
    Content        NodeContent            `json:"content"`
    Parent         *int                   `json:"parent,omitempty"`
    Children       []int                  `json:"children"`
    ContentLayer   ContentLayer           `json:"content_layer"`
    Page           *int                   `json:"page,omitempty"`
    PageEnd        *int                   `json:"page_end,omitempty"`
    BBox           *BoundingBox           `json:"bbox,omitempty"`
    Annotations    []TextAnnotation       `json:"annotations"`
}

type ContentLayer string

const (
    ContentLayerBody     ContentLayer = "body"
    ContentLayerHeader   ContentLayer = "header"
    ContentLayerFooter   ContentLayer = "footer"
    ContentLayerFootnote ContentLayer = "footnote"
)

type NodeContent struct {
    NodeType      string                 `json:"node_type"`
    Text          string                 `json:"text,omitempty"`
    Level         int                    `json:"level,omitempty"`
    Ordered       bool                   `json:"ordered,omitempty"`
    Grid          *TableGrid             `json:"grid,omitempty"`
    Description   string                 `json:"description,omitempty"`
    ImageIndex    *int                   `json:"image_index,omitempty"`
    Language      string                 `json:"language,omitempty"`
    Label         string                 `json:"label,omitempty"`
    HeadingLevel  *int                   `json:"heading_level,omitempty"`
    HeadingText   string                 `json:"heading_text,omitempty"`
}

type TableGrid struct {
    Rows  int32      `json:"rows"`
    Cols  int32      `json:"cols"`
    Cells []GridCell `json:"cells"`
}

type GridCell struct {
    Content   string       `json:"content"`
    Row       int32        `json:"row"`
    Col       int32        `json:"col"`
    RowSpan   int32        `json:"row_span"`
    ColSpan   int32        `json:"col_span"`
    IsHeader  bool         `json:"is_header"`
    BBox      *BoundingBox `json:"bbox,omitempty"`
}

type TextAnnotation struct {
    Start int                `json:"start"`
    End   int                `json:"end"`
    Kind  AnnotationKind     `json:"kind"`
}

type AnnotationKind struct {
    AnnotationType string `json:"annotation_type"`
    URL            string `json:"url,omitempty"`
    Title          string `json:"title,omitempty"`
}
```

### C#

```csharp title="DocumentStructure.cs"
public sealed class DocumentStructure
{
    public IReadOnlyList<DocumentNode> Nodes { get; init; }
}

public sealed class DocumentNode
{
    public string Id { get; init; }
    public NodeContent Content { get; init; }
    public int? Parent { get; init; }
    public IReadOnlyList<int> Children { get; init; }
    public ContentLayer ContentLayer { get; init; }
    public int? Page { get; init; }
    public int? PageEnd { get; init; }
    public BoundingBox? BBox { get; init; }
    public IReadOnlyList<TextAnnotation> Annotations { get; init; }
}

public enum ContentLayer
{
    Body,
    Header,
    Footer,
    Footnote,
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "nodeType")]
[JsonDerivedType(typeof(TitleNode), "title")]
[JsonDerivedType(typeof(HeadingNode), "heading")]
[JsonDerivedType(typeof(ParagraphNode), "paragraph")]
[JsonDerivedType(typeof(ListNode), "list")]
[JsonDerivedType(typeof(ListItemNode), "listItem")]
[JsonDerivedType(typeof(TableNode), "table")]
[JsonDerivedType(typeof(ImageNode), "image")]
[JsonDerivedType(typeof(CodeNode), "code")]
[JsonDerivedType(typeof(QuoteNode), "quote")]
[JsonDerivedType(typeof(FormulaNode), "formula")]
[JsonDerivedType(typeof(FootnoteNode), "footnote")]
[JsonDerivedType(typeof(GroupNode), "group")]
[JsonDerivedType(typeof(PageBreakNode), "pageBreak")]
public abstract record NodeContent;

public record TitleNode(string Text) : NodeContent;
public record HeadingNode(int Level, string Text) : NodeContent;
public record ParagraphNode(string Text) : NodeContent;
public record ListNode(bool Ordered) : NodeContent;
public record ListItemNode(string Text) : NodeContent;
public record TableNode(TableGrid Grid) : NodeContent;
public record ImageNode(string? Description = null, int? ImageIndex = null) : NodeContent;
public record CodeNode(string Text, string? Language = null) : NodeContent;
public record QuoteNode() : NodeContent;
public record FormulaNode(string Text) : NodeContent;
public record FootnoteNode(string Text) : NodeContent;
public record GroupNode(string? Label = null, int? HeadingLevel = null, string? HeadingText = null) : NodeContent;
public record PageBreakNode() : NodeContent;

public sealed class TableGrid
{
    public int Rows { get; init; }
    public int Cols { get; init; }
    public IReadOnlyList<GridCell> Cells { get; init; }
}

public sealed class GridCell
{
    public string Content { get; init; }
    public int Row { get; init; }
    public int Col { get; init; }
    public int RowSpan { get; init; }
    public int ColSpan { get; init; }
    public bool IsHeader { get; init; }
    public BoundingBox? BBox { get; init; }
}

public sealed class TextAnnotation
{
    public int Start { get; init; }
    public int End { get; init; }
    public AnnotationKind Kind { get; init; }
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "annotationType")]
[JsonDerivedType(typeof(BoldAnnotation), "bold")]
[JsonDerivedType(typeof(ItalicAnnotation), "italic")]
[JsonDerivedType(typeof(UnderlineAnnotation), "underline")]
[JsonDerivedType(typeof(StrikethroughAnnotation), "strikethrough")]
[JsonDerivedType(typeof(CodeAnnotation), "code")]
[JsonDerivedType(typeof(SubscriptAnnotation), "subscript")]
[JsonDerivedType(typeof(SuperscriptAnnotation), "superscript")]
[JsonDerivedType(typeof(LinkAnnotation), "link")]
public abstract record AnnotationKind;

public record BoldAnnotation() : AnnotationKind;
public record ItalicAnnotation() : AnnotationKind;
public record UnderlineAnnotation() : AnnotationKind;
public record StrikethroughAnnotation() : AnnotationKind;
public record CodeAnnotation() : AnnotationKind;
public record SubscriptAnnotation() : AnnotationKind;
public record SuperscriptAnnotation() : AnnotationKind;
public record LinkAnnotation(string Url, string? Title = null) : AnnotationKind;
```

### PHP

```php title="document_structure.php"
class DocumentStructure {
    public readonly array $nodes;

    public function __construct(array $nodes) {
        $this->nodes = $nodes;
    }
}

class DocumentNode {
    public readonly string $id;
    public readonly array $content;
    public readonly ?int $parent;
    public readonly array $children;
    public readonly string $contentLayer;
    public readonly ?int $page;
    public readonly ?int $pageEnd;
    public readonly ?BoundingBox $bbox;
    public readonly array $annotations;

    public function __construct(
        string $id,
        array $content,
        ?int $parent,
        array $children,
        string $contentLayer,
        ?int $page,
        ?int $pageEnd,
        ?BoundingBox $bbox,
        array $annotations
    ) {
        $this->id = $id;
        $this->content = $content;
        $this->parent = $parent;
        $this->children = $children;
        $this->contentLayer = $contentLayer;
        $this->page = $page;
        $this->pageEnd = $pageEnd;
        $this->bbox = $bbox;
        $this->annotations = $annotations;
    }
}

class ContentLayer {
    public const BODY = 'body';
    public const HEADER = 'header';
    public const FOOTER = 'footer';
    public const FOOTNOTE = 'footnote';
}

class TableGrid {
    public readonly int $rows;
    public readonly int $cols;
    public readonly array $cells;

    public function __construct(int $rows, int $cols, array $cells) {
        $this->rows = $rows;
        $this->cols = $cols;
        $this->cells = $cells;
    }
}

class GridCell {
    public readonly string $content;
    public readonly int $row;
    public readonly int $col;
    public readonly int $rowSpan;
    public readonly int $colSpan;
    public readonly bool $isHeader;
    public readonly ?BoundingBox $bbox;

    public function __construct(string $content, int $row, int $col, int $rowSpan, int $colSpan, bool $isHeader, ?BoundingBox $bbox) {
        $this->content = $content;
        $this->row = $row;
        $this->col = $col;
        $this->rowSpan = $rowSpan;
        $this->colSpan = $colSpan;
        $this->isHeader = $isHeader;
        $this->bbox = $bbox;
    }
}

class TextAnnotation {
    public readonly int $start;
    public readonly int $end;
    public readonly array $kind;

    public function __construct(int $start, int $end, array $kind) {
        $this->start = $start;
        $this->end = $end;
        $this->kind = $kind;
    }
}
```

### Elixir

```elixir title="document_structure.ex"
defmodule Kreuzberg.DocumentStructure do
  @type t :: %__MODULE__{
    nodes: list(DocumentNode.t())
  }

  defstruct [:nodes]
end

defmodule Kreuzberg.DocumentNode do
  @type t :: %__MODULE__{
    id: String.t(),
    content: Kreuzberg.NodeContent.t(),
    parent: non_neg_integer() | nil,
    children: list(non_neg_integer()),
    content_layer: atom(),
    page: non_neg_integer() | nil,
    page_end: non_neg_integer() | nil,
    bbox: Kreuzberg.BoundingBox.t() | nil,
    annotations: list(Kreuzberg.TextAnnotation.t())
  }

  defstruct [:id, :content, :parent, :children, :content_layer, :page, :page_end, :bbox, :annotations]
end

defmodule Kreuzberg.ContentLayer do
  @type t :: :body | :header | :footer | :footnote
end

defmodule Kreuzberg.NodeContent do
  @type t ::
    {:title, String.t()} |
    {:heading, pos_integer(), String.t()} |
    {:paragraph, String.t()} |
    {:list, boolean()} |
    {:list_item, String.t()} |
    {:table, Kreuzberg.TableGrid.t()} |
    {:image, String.t() | nil, non_neg_integer() | nil} |
    {:code, String.t(), String.t() | nil} |
    :quote |
    {:formula, String.t()} |
    {:footnote, String.t()} |
    {:group, String.t() | nil, pos_integer() | nil, String.t() | nil} |
    :page_break
end

defmodule Kreuzberg.TableGrid do
  @type t :: %__MODULE__{
    rows: pos_integer(),
    cols: pos_integer(),
    cells: list(Kreuzberg.GridCell.t())
  }

  defstruct [:rows, :cols, :cells]
end

defmodule Kreuzberg.GridCell do
  @type t :: %__MODULE__{
    content: String.t(),
    row: pos_integer(),
    col: pos_integer(),
    row_span: pos_integer(),
    col_span: pos_integer(),
    is_header: boolean(),
    bbox: Kreuzberg.BoundingBox.t() | nil
  }

  defstruct [:content, :row, :col, :row_span, :col_span, :is_header, :bbox]
end

defmodule Kreuzberg.TextAnnotation do
  @type t :: %__MODULE__{
    start: non_neg_integer(),
    end: non_neg_integer(),
    kind: Kreuzberg.AnnotationKind.t()
  }

  defstruct [:start, :end, :kind]
end

defmodule Kreuzberg.AnnotationKind do
  @type t ::
    :bold |
    :italic |
    :underline |
    :strikethrough |
    :code |
    :subscript |
    :superscript |
    {:link, String.t(), String.t() | nil}
end
```

### WebAssembly

```typescript title="document_structure_wasm.ts"
export interface DocumentStructure {
  nodes: DocumentNode[];
}

export interface DocumentNode {
  id: string;
  content: NodeContent;
  parent: number | null;
  children: number[];
  contentLayer: ContentLayer;
  page: number | null;
  pageEnd: number | null;
  bbox: BoundingBox | null;
  annotations: TextAnnotation[];
}

export type ContentLayer = "body" | "header" | "footer" | "footnote";

export type NodeContent =
  | { nodeType: "title"; text: string }
  | { nodeType: "heading"; level: number; text: string }
  | { nodeType: "paragraph"; text: string }
  | { nodeType: "list"; ordered: boolean }
  | { nodeType: "listItem"; text: string }
  | { nodeType: "table"; grid: TableGrid }
  | { nodeType: "image"; description?: string; imageIndex?: number }
  | { nodeType: "code"; text: string; language?: string }
  | { nodeType: "quote" }
  | { nodeType: "formula"; text: string }
  | { nodeType: "footnote"; text: string }
  | {
      nodeType: "group";
      label?: string;
      headingLevel?: number;
      headingText?: string;
    }
  | { nodeType: "pageBreak" };

export interface TableGrid {
  rows: number;
  cols: number;
  cells: GridCell[];
}

export interface GridCell {
  content: string;
  row: number;
  col: number;
  rowSpan: number;
  colSpan: number;
  isHeader: boolean;
  bbox: BoundingBox | null;
}

export interface TextAnnotation {
  start: number;
  end: number;
  kind: AnnotationKind;
}

export type AnnotationKind =
  | { annotationType: "bold" }
  | { annotationType: "italic" }
  | { annotationType: "underline" }
  | { annotationType: "strikethrough" }
  | { annotationType: "code" }
  | { annotationType: "subscript" }
  | { annotationType: "superscript" }
  | { annotationType: "link"; url: string; title?: string };
```
