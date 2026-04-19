# Go API Reference <span class="version-badge">v4.9.1</span>

Complete reference for the Kreuzberg Go bindings using cgo to access the Rust-powered extraction pipeline.

The Go binding exposes the same extraction capabilities as the other languages through C FFI bindings to `kreuzberg-ffi`. You get identical metadata extraction, OCR processing, chunking, embeddings, and plugin support—with synchronous and context-aware async APIs.

## Requirements

- **go 1.26+** (with cgo support)
- **C compiler** (gcc/clang for cgo compilation)
- **libkreuzberg_ffi.a** static library (at build time only)
- **Tesseract/EasyOCR/PaddleOCR** (optional, for OCR functionality)

## Installation

Kreuzberg Go binaries are **statically linked** — once built, they are self-contained and require no runtime library dependencies. Only the static library is needed at build time.

### Add the package to your `go.mod`

```bash title="Terminal"
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest
```

### Monorepo Development

For development in the Kreuzberg monorepo:

```bash title="Terminal"
# Build the FFI crate (produces static library)
cargo build -p kreuzberg-ffi --release

# Go will automatically link against target/release/libkreuzberg_ffi.a
cd packages/go/v4
go build -v

# Run your binary - no library paths needed, it's statically linked!
./v4
```

### External Projects

When building outside the monorepo, provide the static library via `CGO_LDFLAGS`:

```bash title="Terminal"
# Option 1: Download pre-built from GitHub Releases
curl -LO https://github.com/kreuzberg-dev/kreuzberg/releases/download/v4.9.1/go-ffi-linux-x86_64.tar.gz
tar -xzf go-ffi-linux-x86_64.tar.gz
mkdir -p ~/kreuzberg/lib
cp kreuzberg-ffi/lib/libkreuzberg_ffi.a ~/kreuzberg/lib/

# Option 2: Build static library yourself
git clone https://github.com/kreuzberg-dev/kreuzberg.git
cd kreuzberg && cargo build -p kreuzberg-ffi --release
cp target/release/libkreuzberg_ffi.a ~/kreuzberg/lib/

# Build your Go project with static linking
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build

# Run - no library paths needed!
./myapp
```

## Quickstart

### Basic file extraction (synchronous)

```go title="main.go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Format: %s\n", result.MimeType)
	fmt.Printf("Content length: %d\n", len(result.Content))
	fmt.Printf("Success: %v\n", result.Success)
}
```

### Async extraction with timeout

```go title="async_extraction.go"
package main

import (
	"context"
	"errors"
	"log"
	"time"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	result, err := kreuzberg.ExtractFile(ctx, "large-document.pdf", nil)
	if errors.Is(err, context.DeadlineExceeded) {
		log.Println("extraction timed out")
		return
	}
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	log.Printf("Extracted %d characters\n", len(result.Content))
}
```

---

## Core Functions

### BatchExtractBytes

Batch extract in-memory documents asynchronously.

**Signature:**

```go title="Go"
func BatchExtractBytes(ctx context.Context, items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation
- `items` ([]BytesWithMime): Document slice
- `config` (*ExtractionConfig): Configuration

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Context or setup errors

---

### BatchExtractBytesSync

Extract multiple in-memory documents in a single batch operation.

**Signature:**

```go title="Go"
func BatchExtractBytesSync(items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `items` ([]BytesWithMime): Slice of {Data, MimeType} pairs
- `config` (*ExtractionConfig): Configuration applied to all items

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Setup error or validation error

**BytesWithMime structure:**

```go title="Go"
type BytesWithMime struct {
	Data     []byte
	MimeType string
}
```

**Example - Batch extract multiple formats:**

```go title="batch_extract_bytes.go"
items := []kreuzberg.BytesWithMime{
	{Data: pdfData, MimeType: "application/pdf"},
	{Data: docxData, MimeType: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"},
	{Data: htmlData, MimeType: "text/html"},
}

results, err := kreuzberg.BatchExtractBytesSync(items, nil)
if err != nil {
	log.Fatalf("batch extraction failed: %v", err)
}

for i, result := range results {
	if result == nil || !result.Success {
		log.Printf("Item %d extraction failed\n", i)
		continue
	}
	log.Printf("Item %d: %s format\n", i, result.MimeType)
}
```

```text

---

### BatchExtractFiles

Batch extract multiple files asynchronously.

**Signature:**

```go title="Go"
func BatchExtractFiles(ctx context.Context, paths []string, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation
- `paths` ([]string): File paths
- `config` (*ExtractionConfig): Configuration for all files

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Context or setup errors

---

### BatchExtractFilesSync

Extract multiple files sequentially using the optimized batch pipeline.

**Signature:**

```go title="Go"
func BatchExtractFilesSync(paths []string, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `paths` ([]string): Slice of file paths
- `config` (*ExtractionConfig): Configuration applied to all files

**Returns:**

- `[]*ExtractionResult`: Slice of results (one per input file; may contain nils for failed extractions)
- `error`: Returned only if batch setup fails; individual file errors are captured in ErrorMetadata

**Example - Batch extract multiple PDFs:**

```go title="batch_extract_pdfs.go"
files := []string{"doc1.pdf", "doc2.pdf", "doc3.pdf"}

results, err := kreuzberg.BatchExtractFilesSync(files, nil)
if err != nil {
	log.Fatalf("batch extraction setup failed: %v", err)
}

for i, result := range results {
	if result == nil {
		fmt.Printf("File %d: extraction failed\n", i)
		continue
	}

	if result.Metadata.Error != nil {
		fmt.Printf("File %d: %s (%s)\n", i, result.Metadata.Error.ErrorType, result.Metadata.Error.Message)
		continue
	}

	fmt.Printf("File %d: extracted %d chars\n", i, len(result.Content))
}
```


```text

---

### BatchExtractFilesWithConfigs <span class="version-badge">v4.9.1</span>

Batch extract multiple files with per-file configuration overrides (asynchronous).

**Signature:**

```go title="Go"
func BatchExtractFilesWithConfigs(ctx context.Context, items []FileWithConfig, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation
- `items` ([]FileWithConfig): Slice of file path + per-file config pairs
- `config` (*ExtractionConfig): Batch-level configuration

---

### BatchExtractFilesWithConfigsSync <span class="version-badge">v4.9.1</span>

Synchronous variant of `BatchExtractFilesWithConfigs`.

**Signature:**

```go title="Go"
func BatchExtractFilesWithConfigsSync(items []FileWithConfig, config *ExtractionConfig) ([]*ExtractionResult, error)
```

---

### BatchExtractBytesWithConfigs / BatchExtractBytesWithConfigsSync <span class="version-badge">v4.9.1</span>

Batch extract multiple byte arrays with per-file configuration overrides. Async and sync variants follow the same pattern.

---

### FileExtractionConfig <span class="version-badge">v4.9.1</span>

Per-file extraction configuration overrides for batch operations. All fields are pointers — `nil` means "use the batch-level default."

```go title="Go"
type FileExtractionConfig struct {
	EnableQualityProcessing  *bool
	OCR                      *OCRConfig
	ForceOCR                 *bool
	Chunking                 *ChunkingConfig
	Images                   *ImageExtractionConfig
	PDFOptions               *PDFConfig
	TokenReduction           *TokenReductionConfig
	LanguageDetection        *LanguageDetectionConfig
	Pages                    *PageConfig
	Keywords                 *KeywordConfig
	PostProcessor            *PostProcessorConfig
	OutputFormat             *string
	ResultFormat             *string
	IncludeDocumentStructure *bool
}
```

Batch-level fields (`MaxConcurrentExtractions`, `UseCache`, `Acceleration`, `SecurityLimits`) cannot be overridden per file. See [Configuration Reference](configuration.md#fileextractionconfig) for details.

---

### ExtractBytes

Extract content from in-memory bytes asynchronously.

**Signature:**

```go title="Go"
func ExtractBytes(ctx context.Context, data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation and timeout
- `data` ([]byte): Document bytes
- `mimeType` (string): MIME type
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: KreuzbergError or context error

**Example:**

```go title="extract_bytes_async.go"
ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
defer cancel()

result, err := kreuzberg.ExtractBytes(ctx, data, "text/html", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

---

### ExtractBytesSync

Extract content from an in-memory byte slice with specified MIME type.

**Signature:**

```go title="Go"
func ExtractBytesSync(data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `data` ([]byte): Document bytes
- `mimeType` (string): MIME type (for example, "application/pdf", "text/plain")
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: KreuzbergError on extraction failure

**Example - Extract from downloaded PDF:**

```go title="extract_from_http.go"
httpResp, err := http.Get("https://example.com/document.pdf")
if err != nil {
	log.Fatal(err)
}
defer httpResp.Body.Close()

data, err := io.ReadAll(httpResp.Body)
if err != nil {
	log.Fatal(err)
}

result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

fmt.Printf("Extracted %d words\n", len(strings.Fields(result.Content)))

---

### ExtractFile

Extract content from a file asynchronously with context support.

**Signature:**

```go title="Go"
func ExtractFile(ctx context.Context, path string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation and timeout
- `path` (string): Path to the file
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: May include context errors (context.DeadlineExceeded, context.Canceled)

**Note:** Context cancellation is best-effort. The underlying C call cannot be interrupted, but the function returns immediately with ctx.Err() when the context deadline is exceeded or cancelled.

**Example - With deadline:**

```go title="extract_with_deadline.go"
ctx, cancel := context.WithDeadline(context.Background(), time.Now().Add(30*time.Second))
defer cancel()

result, err := kreuzberg.ExtractFile(ctx, "large.docx", nil)
if errors.Is(err, context.DeadlineExceeded) {
	log.Println("extraction took too long")
	return
}
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

---

### ExtractFileSync

Extract content and metadata from a file synchronously.

**Signature:**

```go title="Go"
func ExtractFileSync(path string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `path` (string): Path to the file to extract (absolute or relative)
- `config` (*ExtractionConfig): Optional extraction configuration; uses defaults if nil

**Returns:**

- `*ExtractionResult`: Populated result containing content, metadata, tables, chunks, and images
- `error`: KreuzbergError or standard Go error (see Error Handling section)

**Error Handling:**

- `ValidationError`: If path is empty
- `IOError`: If file not found or not readable
- `ParsingError`: If document parsing fails
- `MissingDependencyError`: If required OCR/processing library is missing
- `UnsupportedFormatError`: If MIME type is not supported

**Example - Extract PDF:**

```go title="extract_pdf.go"
result, err := kreuzberg.ExtractFileSync("report.pdf", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

fmt.Printf("Title: %s\n", *result.Metadata.PdfMetadata().Title)
fmt.Printf("Page count: %d\n", *result.Metadata.PdfMetadata().PageCount)
fmt.Printf("Content preview: %s...\n", result.Content[:100])
```

**Example - Extract with configuration:**

```go title="extract_with_config.go"
cfg := &kreuzberg.ExtractionConfig{
	UseCache: boolPtr(true),
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

---

### LibraryVersion

Get the version of the underlying Rust library.

**Signature:**

```go title="Go"
func LibraryVersion() string
```

**Returns:**

- `string`: Version string (for example, "4.9.1")

**Example:**

```go title="check_version.go"
fmt.Printf("Kreuzberg version: %s\n", kreuzberg.LibraryVersion())
```

---

## Configuration

### ExtractionConfig

Root configuration struct for all extraction operations. All fields are optional (pointers); omitted fields use Kreuzberg defaults.

**Signature:**

```go title="Go"
type ExtractionConfig struct {
	Chunking                 *ChunkingConfig          // Text chunking and embeddings
	Concurrency              *ConcurrencyConfig       // Concurrency control settings
	EnableQualityProcessing  *bool                    // Run quality improvements
	ForceOCR                 *bool                    // Force OCR even for text-extractable docs
	HTMLOptions              *HTMLConversionOptions   // HTML-to-Markdown conversion
	Images                   *ImageExtractionConfig   // Image extraction from docs
	IncludeDocumentStructure *bool                    // Include hierarchical document tree
	Keywords                 *KeywordConfig           // Keyword extraction settings
	LanguageDetection        *LanguageDetectionConfig // Language detection settings
	MaxConcurrentExtractions *int                     // Batch concurrency limit
	Layout                   *LayoutDetectionConfig   // Layout detection settings
	OCR                      *OCRConfig               // OCR backend and settings
	OutputFormat             string                   // Applied output format ("markdown", "djot", etc.)
	Pages                    *PageConfig              // Page extraction settings
	PdfOptions               *PdfConfig               // PDF-specific options
	Postprocessor            *PostProcessorConfig     // Post-processor selection
	ResultFormat             string                   // Result structure ("unified", "element_based")
	SecurityLimits           *SecurityLimitsConfig    // Security thresholds for archives/XML
	TokenReduction           *TokenReductionConfig    // Token pruning settings
	UseCache                 *bool                    // Enable result caching
}
```

---

### OCRConfig

Configure OCR backend selection and language.

**Signature:**

```go title="Go"
type OCRConfig struct {
	Backend       string            // OCR backend name: "tesseract", "easyocr", "paddle", etc.
	ElementConfig *OcrElementConfig // OCR element extraction fine-tuning
	Language      *string           // Language code (e.g., "eng", "deu", "fra")
	PaddleOcr     *PaddleOcrConfig  // PaddleOCR-specific configuration
	Tesseract     *TesseractConfig  // Tesseract-specific fine-tuning
}
```

**Example:**

```go title="ocr_config.go"
cfg := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
		Tesseract: &kreuzberg.TesseractConfig{
			PSM:           intPtr(3),
			MinConfidence: float64Ptr(0.5),
		},
	},
}
```

---

### TesseractConfig

Fine-grained Tesseract OCR tuning.

**Signature:**

```go title="Go"
type TesseractConfig struct {
	ClassifyUsePreAdaptedTemplates *bool                     // Use pre-adapted templates
	EnableTableDetection           *bool                     // Detect and extract tables
	Language                       string                    // Language code
	LanguageModelNgramOn           *bool                     // Enable N-gram language model
	MinConfidence                  *float64                  // Confidence threshold (0.0-1.0)
	OEM                            *int                      // Engine mode (0-3)
	OutputFormat                   string                    // Output format: "text", "pdf", "hocr"
	Preprocessing                  *ImagePreprocessingConfig // Image preprocessing
	PSM                            *int                      // Page segmentation mode (0-13)
	TableColumnThreshold           *int                      // Column separation threshold
	TableMinConfidence             *float64                  // Table detection confidence
	TableRowThresholdRatio         *float64                  // Row separation ratio
	TesseditCharBlacklist          string                    // Character blacklist
	TesseditCharWhitelist          string                    // Character whitelist
	TesseditDontBlkrejGoodWds      *bool                     // Don't reject good words
	TesseditDontRowrejGoodWds      *bool                     // Don't reject good rows
	TesseditEnableDictCorrection   *bool                     // Enable dictionary correction
	TesseditUsePrimaryParamsModel  *bool                     // Use primary parameters model
	TextordSpaceSizeIsVariable     *bool                     // Variable space size
	ThresholdingMethod             *bool                     // Thresholding method
	UseCache                       *bool                     // Cache OCR results
}
```

---

### ImagePreprocessingConfig

Configure OCR image preprocessing (DPI normalization, rotation, denoising, etc.).

**Signature:**

```go title="Go"
type ImagePreprocessingConfig struct {
	TargetDPI        *int   // Target DPI for OCR (typically 300)
	AutoRotate       *bool  // Auto-detect and correct image rotation
	Deskew           *bool  // Correct skewed text
	Denoise          *bool  // Remove noise
	ContrastEnhance  *bool  // Enhance contrast
	BinarizationMode string // Binarization method: "otsu", "adaptive"
	InvertColors     *bool  // Invert black/white
}
```

---

### ChunkingConfig

Configure text chunking for RAG and retrieval workloads.

**Signature:**

```go title="Go"
type ChunkingConfig struct {
	ChunkOverlap *int             // Deprecated: use MaxOverlap instead
	ChunkSize    *int             // Deprecated: use MaxChars instead
	Embedding    *EmbeddingConfig // Nested embedding configuration
	Enabled      *bool            // Enable chunking (default: true)
	MaxChars     *int             // Maximum characters per chunk
	MaxOverlap   *int             // Overlap between chunks in characters
	Preset       *string          // Chunking preset name
	ChunkerType  *string          // "text" (default), "markdown", or "yaml"
	Sizing       *ChunkSizingConfig // Chunk size measurement configuration
	PrependHeadingContext *bool   // Prepend heading hierarchy to chunks
}

type ChunkSizingConfig struct {
	SizingType string  // "characters" (default) or "tokenizer"
	Model      string  // HuggingFace model ID (required when SizingType is "tokenizer")
	CacheDir   *string // Optional directory to cache downloaded tokenizer files
}
```

**Note:** The Go binding maintains both `MaxChars`/`MaxOverlap` (recommended) and `ChunkSize`/`ChunkOverlap` (deprecated) for backward compatibility. New code should use `MaxChars` and `MaxOverlap`.

The `Sizing` field controls how chunk size is measured. By default, `MaxChars` counts characters. Set `SizingType` to `"tokenizer"` with a HuggingFace `Model` ID (for example `"bert-base-uncased"`) to measure by token count instead.

---

### ImageExtractionConfig

Configure image extraction from documents.

**Signature:**

```go title="Go"
type ImageExtractionConfig struct {
	ExtractImages     *bool // Extract embedded images
	TargetDPI         *int  // Target DPI for extraction
	MaxImageDimension *int  // Maximum dimension (width/height)
	AutoAdjustDPI     *bool // Auto-adjust DPI for small images
	MinDPI            *int  // Minimum DPI threshold
	MaxDPI            *int  // Maximum DPI threshold
}
```

---

### PdfConfig

PDF-specific extraction options.

**Signature:**

```go title="Go"
type PdfConfig struct {
	AllowSingleColumnTables *bool       // <span class="version-badge">v4.9.1</span> Allow extraction of single-column tables
	BottomMarginFraction    *float64    // Bottom margin to ignore during extraction
	ExtractAnnotations      *bool       // Extract PDF annotations
	ExtractImages           *bool       // Extract embedded images
	ExtractMetadata         *bool       // Extract document metadata
	FontConfig              *FontConfig // Font provider configuration
	Passwords               []string    // List of passwords for encrypted PDFs
	TopMarginFraction       *float64    // Top margin to ignore during extraction
}
```

---

### ConcurrencyConfig <span class="version-badge">v4.9.1</span>

Concurrency configuration for controlling parallel extraction.

**Signature:**

```go title="Go"
type ConcurrencyConfig struct {
	MaxThreads *int // Maximum number of concurrent threads
}
```

**Example:**

```go title="concurrency_config.go"
cfg := &kreuzberg.ExtractionConfig{
	Concurrency: &kreuzberg.ConcurrencyConfig{
		MaxThreads: intPtr(4),
	},
}
```

---

### EmbeddingConfig

Configure embedding generation for chunks.

**Signature:**

```go title="Go"
type EmbeddingConfig struct {
	Model                *EmbeddingModelType // Model selection
	Normalize            *bool               // L2 normalization
	BatchSize            *int                // Batch size for inference
	ShowDownloadProgress *bool               // Show download progress
	CacheDir             *string             // Cache directory
}

type EmbeddingModelType struct {
	Type       string // "preset" or "custom"
	Name       string // For preset models
	Model      string // For custom models
	ModelID    string // Alias for custom
	Dimensions *int   // Embedding dimensions
}
```

---

### KeywordConfig

Configure keyword extraction.

**Signature:**

```go title="Go"
type KeywordConfig struct {
	Algorithm   string      // "yake" or "rake"
	MaxKeywords *int        // Maximum keywords to extract
	MinScore    *float64    // Minimum keyword score
	NgramRange  *[2]int     // N-gram range: [min, max]
	Language    *string     // Language code
	Yake        *YakeParams // YAKE-specific tuning
	Rake        *RakeParams // RAKE-specific tuning
}

type YakeParams struct {
	WindowSize *int
}

type RakeParams struct {
	MinWordLength     *int
	MaxWordsPerPhrase *int
}
```

---

### LayoutDetectionConfig <span class="version-badge">v4.9.1</span>

Configure ONNX-based document layout detection.

**Signature:**

```go title="Go"
type LayoutDetectionConfig struct {
	ApplyHeuristics     *bool    // Whether to apply heuristic post-processing
	ConfidenceThreshold *float64 // Minimum confidence threshold (0.0-1.0)
	TableModel          *string  // Table structure model: "tatr", "slanet_wired", etc.
}
```

---

### PostProcessorConfig

Configure post-processing steps.

**Signature:**

```go title="Go"
type PostProcessorConfig struct {
	DisabledProcessors []string // Processors to skip
	Enabled            *bool    // Enable post-processing
	EnabledProcessors  []string // Specific processors to run
}
```

### OcrElementConfig

Fine-grained controls for OCR element extraction.

**Signature:**

```go title="Go"
type OcrElementConfig struct {
	BuildHierarchy  bool    // Build hierarchical relationship between elements
	IncludeElements bool    // Whether to extract spatial elements
	MinConfidence   float64 // Minimum confidence threshold (0.0-1.0)
	MinLevel        string  // Minimum level: "word", "line", "block", "page"
}
```

### PaddleOcrConfig

Specific configuration for the PaddleOCR backend.

**Signature:**

```go title="Go"
type PaddleOcrConfig struct {
	CacheDir             string   // Cache directory for model files
	DetDbBoxThresh       *float64 // Detection box threshold
	DetDbThresh          *float64 // Detection threshold
	DetDbUnclipRatio     *float64 // Detection unclip ratio
	DetLimitSideLen      *int     // Detection side length limit
	EnableTableDetection *bool    // Detect tables in images
	Language             string   // Language code
	ModelTier            string   // (v4.9.1) Model tier: "mobile" (default, ~21MB total, fast) or "server" (~172MB, best with GPU)
	Padding              *int     // (v4.9.1) Padding in pixels (0-100) around image before detection. Default: 10
	RecBatchNum          *int     // Recognition batch size
	UseAngleCls          *bool    // Use angle classification
}
```

### SecurityLimitsConfig

Security thresholds for archive and XML processing.

**Signature:**

```go title="Go"
type SecurityLimitsConfig struct {
	MaxArchiveSize      *int // Maximum archive size in bytes
	MaxCompressionRatio *int // Maximum allowed compression ratio
	MaxContentSize      *int // Maximum extracted content size
	MaxEntityLength     *int // Maximum XML entity expansion length
	MaxFilesInArchive   *int // Maximum number of files in archive
	MaxIterations       *int // Maximum processing iterations
	MaxNestingDepth     *int // Maximum recursion depth
	MaxTableCells       *int // Maximum table cells per document
	MaxXMLDepth         *int // Maximum XML structure depth
}
```

---

## Results & Types

### ExtractionResult

The main result struct containing all extracted data.

**Signature:**

```go title="Go"
type ExtractionResult struct {
	Annotations        []PdfAnnotation    // PDF annotations extracted from the document
	Chunks             []Chunk            // Text chunks (if enabled)
	Content            string             // Extracted text content
	DetectedLanguages  []string           // Detected languages (ISO codes)
	DjotContent        *DjotContent       // Rich Djot content structure
	Document           *DocumentStructure // Structured document tree
	Elements           []Element          // Semantic elements (ElementBased format)
	ExtractedKeywords  []ExtractedKeyword // Keywords from RAKE/YAKE extraction
	Images             []ExtractedImage   // Embedded images (if enabled)
	Metadata           Metadata           // Aggregated document metadata
	MimeType           string             // Detected or hint MIME type
	OcrElements        []OcrElement       // OCR elements with spatial metadata
	Pages              []PageContent      // Per-page content (if enabled)
	ProcessingWarnings []ProcessingWarning // Non-fatal pipeline warnings
	QualityScore       *float64           // Document quality score (0.0-1.0)
	Tables             []Table            // Extracted tables as markdown and grids
}
```

**Example - Accessing results:**

```go title="inspect_extraction_result.go"
result, err := kreuzberg.ExtractFileSync("report.pdf", nil)
if err != nil || !result.Success {
	log.Fatal("extraction failed")
}

fmt.Printf("Detected MIME type: %s\n", result.MimeType)
fmt.Printf("Content length: %d\n", len(result.Content))
fmt.Printf("Detected languages: %v\n", result.DetectedLanguages)
fmt.Printf("Number of tables: %d\n", len(result.Tables))
fmt.Printf("Number of chunks: %d\n", len(result.Chunks))
fmt.Printf("Number of images: %d\n", len(result.Images))
if result.QualityScore != nil {
	fmt.Printf("Quality score: %.2f\n", *result.QualityScore)
}
```

#### Pages

**Type**: `[]PageContent`

Per-page extracted content when page extraction is enabled via `PageConfig.ExtractPages = true`.

Each page contains:

- Page number (1-indexed)
- Text content for that page
- Tables on that page
- Images on that page
- Layout regions when layout detection is enabled, each with `Class` (string), `Confidence` (float64, 0–1), `BoundingBox`, and `AreaFraction` (float64, 0–1)

**Example:**

```go title="page_extraction.go"
config := &kreuzberg.ExtractionConfig{
	Pages: &kreuzberg.PageConfig{
		ExtractPages: boolPtr(true),
	},
}

result, err := kreuzberg.ExtractFileSync("document.pdf", config)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

if result.Pages != nil {
	for _, page := range result.Pages {
		fmt.Printf("Page %d:\n", page.PageNumber)
		fmt.Printf("  Content: %d chars\n", len(page.Content))
		fmt.Printf("  Tables: %d\n", len(page.Tables))
		fmt.Printf("  Images: %d\n", len(page.Images))
	}
}
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```go title="iterate_pages.go"
config := &kreuzberg.ExtractionConfig{
	Pages: &kreuzberg.PageConfig{
		ExtractPages:      boolPtr(true),
		InsertPageMarkers: boolPtr(true),
		MarkerFormat:      stringPtr("\n\n--- Page {page_num} ---\n\n"),
	},
}

result, err := kreuzberg.ExtractFileSync("document.pdf", config)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

// Access combined content with page markers
fmt.Println("Combined content with markers:")
if len(result.Content) > 500 {
	fmt.Println(result.Content[:500])
} else {
	fmt.Println(result.Content)
}
fmt.Println()

// Access per-page content
if result.Pages != nil {
	for _, page := range result.Pages {
		fmt.Printf("Page %d:\n", page.PageNumber)
		preview := page.Content
		if len(preview) > 100 {
			preview = preview[:100]
		}
		fmt.Printf("  %s...\n", preview)
		if len(page.Tables) > 0 {
			fmt.Printf("  Found %d table(s)\n", len(page.Tables))
		}
		if len(page.Images) > 0 {
			fmt.Printf("  Found %d image(s)\n", len(page.Images))
		}
	}
}
```

---

### DjotContent

Comprehensive Djot document structure with semantic preservation.

**Signature:**

```go title="Go"
type DjotContent struct {
	Attributes []DjotAttributeEntry // Element attributes mapping
	Blocks     []FormattedBlock     // Structured block-level content
	Footnotes  []Footnote           // Footnote definitions
	Images     []DjotImage           // Extracted images with metadata
	Links      []DjotLink            // Extracted links with URLs
	Metadata   Metadata             // YAML frontmatter metadata
	PlainText  string               // Plain text representation
	Tables     []Table              // Extracted tables as structured data
}
```

---

### DocumentStructure

Hierarchical tree representation of the document.

**Signature:**

```go title="Go"
type DocumentStructure struct {
	Nodes []DocumentNode // Flat list of nodes with parent/child references
}
```

#### DocumentNode

A single node in the document hierarchy.

**Signature:**

```go title="Go"
type DocumentNode struct {
	Annotations  []TextAnnotation // Inline formatting and links
	Bbox         *BoundingBox     // Spatial position
	Children     []uint32         // Child node indices
	Content      NodeContent      // Typed node content
	ContentLayer ContentLayer     // Layer: body, header, footer, etc.
	ID           string           // Unique node ID
	Page         *uint32          // Start page number
	PageEnd      *uint32          // End page number
	Parent       *uint32          // Parent node index
}
```

---

### Element

Semantic element extracted from a document (ElementBased format).

**Signature:**

```go title="Go"
type Element struct {
	ElementID   string          // Unique deterministic element ID
	ElementType ElementType     // Semantic type: "title", "heading", "table", etc.
	Metadata    ElementMetadata // Element-specific metadata (page, bbox, etc.)
	Text        string          // Text content of the element
}
```

#### ElementMetadata

Metadata for a semantic element.

**Signature:**

```go title="Go"
type ElementMetadata struct {
	Additional   map[string]string // Custom metadata fields
	Coordinates  *BoundingBox      // Bounding box coordinates
	ElementIndex *uint64           // Zero-based index in element sequence
	Filename     *string           // Source filename
	PageNumber   *uint64           // 1-indexed page number
}
```

---

### OcrElement

Spatial text element from OCR processing.

**Signature:**

```go title="Go"
type OcrElement struct {
	BackendMetadata map[string]interface{} // Raw backend-specific metadata
	Confidence      *OcrConfidence         // Confidence scores
	Geometry        *OcrBoundingGeometry   // Spatial geometry (box/points)
	Level           string                 // Element level (word/line)
	PageNumber      *int                   // 1-indexed page number
	ParentID        string                 // ID of parent element
	Rotation        *OcrRotation           // Rotation info
	Text            string                 // Recognized text
}
```

---

### Metadata

Aggregated document metadata with format-specific fields.

**Signature:**

```go title="Go"
type Metadata struct {
	AbstractText         *string                     // Abstract or summary text (from frontmatter)
	Additional           map[string]json.RawMessage  // Custom/additional fields (deprecated)
	Authors              []string                    // Primary author(s)
	Category             *string                     // Document category (classification/frontmatter)
	CreatedAt            *string                     // Creation timestamp (ISO 8601)
	CreatedBy            *string                     // User who created the document
	Date                 *string                     // Document date
	DocumentVersion      *string                     // Document version string (from frontmatter)
	Error                *ErrorMetadata              // Error info for batch operations
	ExtractionDurationMs *uint64                     // Extraction duration in milliseconds
	Format               FormatMetadata              // Format-specific metadata
	ImagePreprocessing   *ImagePreprocessingMetadata // OCR preprocessing info
	JSONSchema           json.RawMessage             // JSON Schema for structured data
	Keywords             []string                    // Keywords/tags
	Language             *string                     // Primary language (ISO 639 code)
	ModifiedAt           *string                     // Last modification timestamp (ISO 8601)
	ModifiedBy           *string                     // User who last modified the document
	OutputFormat         *string                     // Applied output format (e.g., "markdown")
	PageCount            *int                        // Total page count
	Pages                *PageStructure              // Page/slide/sheet structure
	Producer             *string                     // Document producer
	Subject              *string                     // Document subject or description
	Tags                 []string                    // Document tags (from frontmatter)
	Title                *string                     // Document title
}
```

**Access format-specific metadata:**

```go title="inspect_format_metadata.go"
fmt.Println("Format type:", result.Metadata.FormatType())

if pdfMeta, ok := result.Metadata.PdfMetadata(); ok {
	fmt.Printf("Title: %s\n", *pdfMeta.Title)
	fmt.Printf("Pages: %d\n", *pdfMeta.PageCount)
	fmt.Printf("Author: %s\n", *pdfMeta.Authors[0])
}

if excelMeta, ok := result.Metadata.ExcelMetadata(); ok {
	fmt.Printf("Sheets: %d\n", excelMeta.SheetCount)
	fmt.Printf("Sheet names: %v\n", excelMeta.SheetNames)
}

if htmlMeta, ok := result.Metadata.HTMLMetadata(); ok {
	fmt.Printf("Page title: %s\n", *htmlMeta.Title)
	fmt.Printf("OG image: %s\n", *htmlMeta.OGImage)
}
```

---

### Table

Extracted table structure.

**Signature:**

```go title="Go"
type Table struct {
	BoundingBox *BoundingBox // Spatial location on page
	Cells       [][]string   // 2D cell array [row][col]
	Markdown    string       // Markdown representation
	PageNumber  int          // Page number (1-indexed)
}
```

**Example:**

```go title="extract_tables.go"
for tableIdx, table := range result.Tables {
	fmt.Printf("Table %d (page %d):\n", tableIdx, table.PageNumber)
	for _, row := range table.Cells {
		fmt.Println(strings.Join(row, " | "))
	}
	fmt.Println("Markdown:", table.Markdown)
}
```

---

### Chunk

Text chunk with optional embeddings and metadata.

**Signature:**

```go title="Go"
type Chunk struct {
	Content   string        // Chunk text
	Embedding []float32     // Embedding vector (if enabled)
	Metadata  ChunkMetadata // Chunk positioning
}

type ChunkMetadata struct {
	ByteEnd        uint64           // UTF-8 byte offset (exclusive)
	ByteStart      uint64           // UTF-8 byte offset (inclusive)
	CharCount      int              // Number of characters in chunk
	ChunkIndex     uint64           // Index in chunk sequence
	FirstPage      *uint64          // First page this chunk appears on
	LastPage       *uint64          // Last page this chunk appears on
	TokenCount     *uint64          // Token count (if available)
	TotalChunks    uint64           // Total number of chunks in document
	HeadingContext *HeadingContext   // Heading hierarchy (markdown chunker only)
}
```

**Fields:**

- `ByteStart` (int): UTF-8 byte offset in content (inclusive)
- `ByteEnd` (int): UTF-8 byte offset in content (exclusive)
- `CharCount` (int): Number of characters in chunk
- `TokenCount` (\*int): Estimated token count (if configured)
- `FirstPage` (\*int): First page this chunk appears on (1-indexed, only when page boundaries available)
- `LastPage` (\*int): Last page this chunk appears on (1-indexed, only when page boundaries available)
- `HeadingContext` (\*HeadingContext): Heading hierarchy when using Markdown chunker. Only populated when chunker_type is set to markdown.

**Page tracking:** When `PageStructure.Boundaries` is available and chunking is enabled, `FirstPage` and `LastPage` are automatically calculated based on byte offsets.

**Example:**

```go title="inspect_chunks.go"
for _, chunk := range result.Chunks {
	fmt.Printf("Chunk %d/%d\n", chunk.Metadata.ChunkIndex, chunk.Metadata.TotalChunks)
	fmt.Printf("Content: %s...\n", chunk.Content[:min(50, len(chunk.Content))])
	fmt.Printf("Bytes: [%d:%d], %d chars\n", chunk.Metadata.ByteStart, chunk.Metadata.ByteEnd, chunk.Metadata.CharCount)
	if chunk.Metadata.TokenCount != nil {
		fmt.Printf("Tokens: %d\n", *chunk.Metadata.TokenCount)
	}

	// Show page information if available
	if chunk.Metadata.FirstPage != nil {
		first := *chunk.Metadata.FirstPage
		last := *chunk.Metadata.LastPage
		if first == last {
			fmt.Printf("Page: %d\n", first)
		} else {
			fmt.Printf("Pages: %d-%d\n", first, last)
		}
	}

	if len(chunk.Embedding) > 0 {
		fmt.Printf("Embedding dim: %d\n", len(chunk.Embedding))
		fmt.Printf("First 5 values: %v\n", chunk.Embedding[:5])
	}
}
```

---

### ExtractedImage

Image extracted from document with optional OCR results.

**Signature:**

```go title="Go"
type ExtractedImage struct {
	BitsPerComponent *uint32           // Bits per color component
	BoundingBox      *BoundingBox      // Spatial location on page
	Colorspace       *string           // Colorspace info (RGB, CMYK, etc.)
	Data             []byte            // Raw image bytes
	Description      *string           // Image description or alt-text
	Format           string            // Image format: "jpeg", "png", "webp"
	Height           *uint32           // Image height in pixels
	ImageIndex       uint64            // Zero-based index in results
	IsMask           bool              // Whether image is a transparency mask
	OCRResult        *ExtractionResult // Nested OCR result if processed
	PageNumber       *uint64           // 1-indexed page number
	Width            *uint32           // Image width in pixels
}
```

**Example:**

```go title="extract_images.go"
for imgIdx, img := range result.Images {
	fmt.Printf("Image %d: %s, %dx%d\n", imgIdx, img.Format, *img.Width, *img.Height)

	filename := fmt.Sprintf("image_%d.%s", imgIdx, img.Format)
	os.WriteFile(filename, img.Data, 0644)

	if img.OCRResult != nil {
		fmt.Printf("Image %d OCR: %s\n", imgIdx, img.OCRResult.Content)
	}
}
```

---

## Embeddings

### EmbedTexts()

Generate embeddings for a list of texts synchronously.

**Signature:**

```go
func EmbedTexts(texts []string, config *EmbeddingConfig) ([][]float32, error)
```

**Parameters:**

- `texts` (`[]string`): List of strings to embed.
- `config` (`*EmbeddingConfig`): Embedding configuration. Pass `nil` for defaults.

**Returns:** `[][]float32` — one embedding vector per input text.

**Example:**

--8<-- "snippets/go/utils/standalone_embed.md"

---

### EmbedTextsAsync()

Context-aware async variant of `EmbedTexts()`. Runs the embedding in a goroutine and respects context cancellation.

**Signature:**

```go
func EmbedTextsAsync(ctx context.Context, texts []string, config *EmbeddingConfig) ([][]float32, error)
```

Same parameters as `EmbedTexts()`, plus a `context.Context` as the first argument.

---

## PDF Rendering

!!! Info "Added in v4.9.1"

### RenderPdfPage

Render a single page of a PDF as a PNG image.

**Signature:**

```go title="Go"
func RenderPdfPage(path string, pageIndex int, dpi int) ([]byte, error)
```

**Parameters:**

- `path` (string): Path to the PDF file
- `pageIndex` (int): Zero-based page index to render
- `dpi` (int): Resolution for rendering (for example 150)

**Returns:**

- `[]byte`: PNG-encoded bytes for the requested page
- `error`: Error if file cannot be read, rendered, or page index is out of bounds

**Example:**

```go title="render_single_page.go"
png, err := kreuzberg.RenderPdfPage("document.pdf", 0, 150)
if err != nil {
	log.Fatalf("render failed: %v", err)
}
os.WriteFile("first_page.png", png, 0644)
```

---

### PdfPageIterator

A more memory-efficient alternative to rendering all pages at once when memory is a concern or when pages should be processed as they are rendered (for example, sending each page to a vision model for OCR). Renders one page at a time, so only one raw image is in memory at a time.

**Type:**

```go title="Go"
type PdfPageIterator struct { ... }

func NewPdfPageIterator(path string, dpi int) (*PdfPageIterator, error)
func (it *PdfPageIterator) Next() (pageIndex int, png []byte, ok bool, err error)
func (it *PdfPageIterator) PageCount() int
func (it *PdfPageIterator) Close()
```

**Example:**

```go title="iterate_pages.go"
iter, err := kreuzberg.NewPdfPageIterator("document.pdf", 150)
if err != nil {
	log.Fatalf("failed to create iterator: %v", err)
}
defer iter.Close()

for {
	pageIndex, png, ok, err := iter.Next()
	if err != nil {
		log.Fatalf("render error: %v", err)
	}
	if !ok {
		break
	}
	os.WriteFile(fmt.Sprintf("page_%d.png", pageIndex), png, 0644)
}
```

---

## Error Handling

### Error Types

Kreuzberg defines a type hierarchy of errors via the `KreuzbergError` interface:

```go title="Go"
type KreuzbergError interface {
	error
	Kind() ErrorKind
}

type ErrorKind string

const (
	ErrorKindUnknown           ErrorKind = "unknown"
	ErrorKindIO                ErrorKind = "io"
	ErrorKindValidation        ErrorKind = "validation"
	ErrorKindParsing           ErrorKind = "parsing"
	ErrorKindOCR               ErrorKind = "ocr"
	ErrorKindCache             ErrorKind = "cache"
	ErrorKindImageProcessing   ErrorKind = "image_processing"
	ErrorKindSerialization     ErrorKind = "serialization"
	ErrorKindMissingDependency ErrorKind = "missing_dependency"
	ErrorKindPlugin            ErrorKind = "plugin"
	ErrorKindUnsupportedFormat ErrorKind = "unsupported_format"
	ErrorKindRuntime           ErrorKind = "runtime"
)
```

**Error type classes:**

- `ValidationError`: Input validation failed (empty paths, missing MIME types)
- `ParsingError`: Document parsing failed (malformed file, unsupported format)
- `OCRError`: OCR backend failure (library missing, invalid language)
- `CacheError`: Cache operation failed
- `ImageProcessingError`: Image manipulation failed
- `SerializationError`: JSON encoding/decoding failed
- `MissingDependencyError`: Required library not found (Tesseract, EasyOCR, etc.)
- `PluginError`: Plugin registration or execution failed
- `UnsupportedFormatError`: MIME type not supported
- `IOError`: File I/O failure
- `RuntimeError`: Unexpected runtime failure (lock poisoning, etc.)

---

### Error Classification

Errors are automatically classified based on native error messages. Use `errors.As()` and `errors.Is()` to handle specific error types:

```go title="error_classification.go"
import (
	"errors"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
if err != nil {
	var parsingErr *kreuzberg.ParsingError
	if errors.As(err, &parsingErr) {
		log.Printf("Parsing failed: %v\n", parsingErr)
		return
	}

	var missingDep *kreuzberg.MissingDependencyError
	if errors.As(err, &missingDep) {
		log.Printf("Missing dependency: %s\n", missingDep.Dependency)
		return
	}

	log.Printf("Extraction failed: %v\n", err)
}
```

---

### Error Unwrapping

All Kreuzberg errors support error unwrapping via `errors.Unwrap()`:

```go title="error_unwrapping.go"
result, err := kreuzberg.ExtractFileSync("doc.pdf", nil)
if err != nil {
	rootErr := errors.Unwrap(err)
	if rootErr != nil {
		log.Printf("Root cause: %v\n", rootErr)
	}

	if krErr, ok := err.(kreuzberg.KreuzbergError); ok {
		log.Printf("Error kind: %v\n", krErr.Kind())
	}
}
```

---

### Error Handling Examples

**Handle file not found:**

```go title="handle_file_not_found.go"
result, err := kreuzberg.ExtractFileSync("missing.pdf", nil)
if err != nil {
	var ioErr *kreuzberg.IOError
	if errors.As(err, &ioErr) {
		log.Println("File not found or unreadable")
		return
	}
	log.Fatalf("unexpected error: %v\n", err)
}
```

**Handle missing OCR dependency:**

```go title="handle_missing_ocr.go"
cfg := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
if err != nil {
	var missingDep *kreuzberg.MissingDependencyError
	if errors.As(err, &missingDep) {
		log.Printf("Install %s to use OCR\n", missingDep.Dependency)
		return
	}
	log.Fatalf("extraction failed: %v\n", err)
}
```

**Batch error handling:**

```go title="batch_error_handling.go"
results, err := kreuzberg.BatchExtractFilesSync(files, nil)
if err != nil {
	log.Fatalf("batch setup failed: %v\n", err)
}

for i, result := range results {
	if result == nil {
		log.Printf("File %d: extraction failed (nil result)\n", i)
		continue
	}

	if result.Metadata.Error != nil {
		log.Printf("File %d: %s - %s\n", i, result.Metadata.Error.ErrorType, result.Metadata.Error.Message)
		continue
	}

	if !result.Success {
		log.Printf("File %d: extraction unsuccessful\n", i)
		continue
	}

	log.Printf("File %d: success (%d chars)\n", i, len(result.Content))
}
```

---

## Advanced Usage

### MIME Type Detection

Detect MIME type from file extension or content:

```go title="mime_detection.go"
mimeType := "application/pdf"
```

---

### CGO-Specific Patterns

#### Memory Management

Go's cgo automatically manages C memory for simple types. Kreuzberg handles C pointer cleanup internally via `defer` statements:

```go title="memory_safety.go"
result, err := kreuzberg.ExtractFileSync("doc.pdf", nil)

result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
```

#### Static Linking Configuration

Go binaries are statically linked against the FFI library, so **no runtime library paths are needed**. Configuration is done at build time:

**Monorepo Development:**

```bash title="Terminal"
# Build FFI library first
cargo build -p kreuzberg-ffi --release

# Go automatically finds target/release/libkreuzberg_ffi.a
go build -v ./...

# Run directly - no environment variables needed
./myapp
```

**External Projects:**

```bash title="Terminal"
# Set CGO_LDFLAGS to point to the static library
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build

# Run directly - no runtime dependencies
./myapp
```

#### Configuration as JSON

Internally, ExtractionConfig is serialized to JSON and passed to the C FFI:

```go title="json_serialization.go"
cfg := &kreuzberg.ExtractionConfig{
	UseCache: boolPtr(true),
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("doc.pdf", cfg)
```

---

### Custom Post-Processors

Register custom post-processing logic in Go:

```go title="custom_post_processor.go"
package main

import (
	"C"
	"encoding/json"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

//export myCustomProcessor
func myCustomProcessor(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result kreuzberg.ExtractionResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		errMsg := C.CString("failed to parse JSON")
		return errMsg
	}

	result.Content = strings.ToUpper(result.Content)

	modified, _ := json.Marshal(result)
	return C.CString(string(modified))
}

func init() {
	err := kreuzberg.RegisterPostProcessor(
		"go-uppercase",
		100, // priority
		(C.PostProcessorCallback)(C.myCustomProcessor),
	)
	if err != nil {
		log.Fatalf("failed to register post-processor: %v\n", err)
	}
}

func main() {
	cfg := &kreuzberg.ExtractionConfig{
		Postprocessor: &kreuzberg.PostProcessorConfig{
			EnabledProcessors: []string{"go-uppercase"},
		},
	}

	result, _ := kreuzberg.ExtractFileSync("doc.pdf", cfg)
}
```

---

### Custom Validators

Validate extraction results:

```go title="custom_validator.go"
//export myValidator
func myValidator(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result kreuzberg.ExtractionResult
	json.Unmarshal([]byte(jsonStr), &result)

	if len(result.Content) == 0 {
		errMsg := C.CString("content is empty")
		return errMsg
	}

	return nil
}

func init() {
	kreuzberg.RegisterValidator(
		"content-not-empty",
		50,
		(C.ValidatorCallback)(C.myValidator),
	)
}
```

---

### Custom OCR Backends

Register a custom OCR backend:

```go title="custom_ocr_backend.go"
//export customOCR
func customOCR(imageData *C.uint8_t, width C.uint32_t, height C.uint32_t, lang *C.char) *C.char {
	result := kreuzberg.ExtractionResult{
		Content:  "extracted text from custom OCR",
		MimeType: "text/plain",
		Success:  true,
	}
	data, _ := json.Marshal(result)
	return C.CString(string(data))
}

func init() {
	kreuzberg.RegisterOCRBackend(
		"custom-ocr",
		(C.OcrBackendCallback)(C.customOCR),
	)
}
```

---

### Plugin Management

List and manage registered plugins:

```go title="plugin_management.go"
validators, err := kreuzberg.ListValidators()
if err == nil {
	fmt.Printf("Validators: %v\n", validators)
}

processors, err := kreuzberg.ListPostProcessors()
if err == nil {
	fmt.Printf("Post-processors: %v\n", processors)
}

backends, err := kreuzberg.ListOCRBackends()
if err == nil {
	fmt.Printf("OCR backends: %v\n", backends)
}

if err := kreuzberg.ClearValidators(); err != nil {
	log.Fatalf("failed to clear validators: %v\n", err)
}

if err := kreuzberg.UnregisterValidator("my-validator"); err != nil {
	log.Fatalf("failed to unregister: %v\n", err)
}
```

---

### Performance Tips

1. **Batch Processing**: Use `BatchExtractFilesSync()` for multiple files to leverage internal optimizations
2. **Context Timeouts**: Set realistic timeouts; OCR can be slow on large documents
3. **Caching**: Enable `UseCache: boolPtr(true)` to cache often extracted documents
4. **Static Linking**: Binaries are self-contained after build; no runtime library paths needed
5. **Configuration Reuse**: Create and reuse ExtractionConfig objects across multiple calls
6. **Goroutines**: Use `ExtractFile()` / `ExtractBytes()` variants in goroutines for concurrency

---

## Troubleshooting

### Static Library Not Found

**Error:** `cannot find -lkreuzberg_ffi` or `undefined reference to 'kreuzberg_...'`

**Solution:**

```bash title="Terminal"
# Verify static library exists
ls -la target/release/libkreuzberg_ffi.a

# For monorepo development, just build the FFI crate:
cargo build -p kreuzberg-ffi --release

# For external projects, provide the path via CGO_LDFLAGS:
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build
```

The binary will be statically linked and have no runtime dependencies on Kreuzberg libraries.

---

### CGO Compilation Errors

**Error:** `error: kreuzberg.h: No such file or directory`

**Solution:**

Ensure kreuzberg-ffi is built before building your Go module:

```bash title="Terminal"
cargo build -p kreuzberg-ffi --release
go build ./...
```

---

### Missing OCR Library

**Error:** `MissingDependencyError: Missing dependency: tesseract`

**Solution:**

Install Tesseract or use a different OCR backend:

```bash title="Terminal"
# macOS
brew install tesseract

# Debian/Ubuntu
apt-get install tesseract-ocr

# Or use PaddleOCR (native) or EasyOCR (Python package)
```

---

### Context Timeout on Large Documents

**Issue:** Extraction times out before completion

**Solution:**

Increase timeout or disable OCR for large documents:

```go title="handle_large_documents.go"
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
defer cancel()

cfg := &kreuzberg.ExtractionConfig{
	ForceOCR: boolPtr(false),
}

result, err := kreuzberg.ExtractFile(ctx, "large.pdf", cfg)
```

---

## Testing

Run the test suite:

```bash title="Terminal"
# Unit tests (from packages/go)
task go:test

# Lint (gofmt + golangci-lint)
task go:lint

# E2E tests (from e2e/go, auto-generated from fixtures)
task e2e:go:verify

# Manual test (build FFI library first)
cargo build -p kreuzberg-ffi --release
go test -v ./packages/go/v4
```

---

## Helper Functions

Add these utility functions to your code:

```go title="Go"
func stringPtr(s string) *string {
	return &s
}

func boolPtr(b bool) *bool {
	return &b
}

func intPtr(i int) *int {
	return &i
}

func float64Ptr(f float64) *float64 {
	return &f
}

func uint32Ptr(u uint32) *uint32 {
	return &u
}
```

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. The Go binding passes LLM configuration as JSON through the FFI layer. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Pass `StructuredExtraction` config to extract structured data from documents using an LLM:

```go title="structured_extraction.go"
config := &kreuzberg.ExtractionConfig{
	StructuredExtraction: &kreuzberg.StructuredExtractionConfig{
		Schema: map[string]interface{}{
			"type": "object",
			"properties": map[string]interface{}{
				"title":   map[string]interface{}{"type": "string"},
				"authors": map[string]interface{}{"type": "array", "items": map[string]interface{}{"type": "string"}},
				"date":    map[string]interface{}{"type": "string"},
			},
			"required":             []string{"title", "authors", "date"},
			"additionalProperties": false,
		},
		Llm: &kreuzberg.LlmConfig{
			Model: "openai/gpt-4o-mini",
		},
		Strict: boolPtr(true),
	},
}

result, err := kreuzberg.ExtractFileSync("paper.pdf", config)
if err != nil {
	log.Fatal(err)
}

if result.StructuredOutput != "" {
	fmt.Println(result.StructuredOutput)
}
```

### VLM OCR

Use a vision-language model as an OCR backend:

```go title="vlm_ocr.go"
config := &kreuzberg.ExtractionConfig{
	ForceOCR: boolPtr(true),
	OCR: &kreuzberg.OcrConfig{
		Backend: "vlm",
		VlmConfig: &kreuzberg.LlmConfig{
			Model: "openai/gpt-4o-mini",
		},
	},
}

result, err := kreuzberg.ExtractFileSync("scan.pdf", config)
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## Code Intelligence

Kreuzberg uses [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) to parse and analyze source code files across 248 programming languages. When extracting code files, the result metadata includes structural analysis, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available via the `CodeProcessResult` type when the format metadata type is `"code"`.

```go title="code_intelligence.go"
package main

import (
	"encoding/json"
	"fmt"
	"log"

	kreuzberg "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		TreeSitter: &kreuzberg.TreeSitterConfig{
			Process: &kreuzberg.TreeSitterProcessConfig{
				Structure:  boolPtr(true),
				Imports:    boolPtr(true),
				Exports:    boolPtr(true),
				Comments:   boolPtr(true),
				Docstrings: boolPtr(true),
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("app.py", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	// Access code intelligence from metadata JSON
	if result.Metadata.Format.Type == "code" {
		raw, _ := json.Marshal(result.Metadata.Format)
		var code kreuzberg.CodeProcessResult
		if err := json.Unmarshal(raw, &code); err == nil {
			fmt.Printf("Language: %s\n", code.Language)
			fmt.Printf("Functions/classes: %d\n", len(code.Structure))
			fmt.Printf("Imports: %d\n", len(code.Imports))

			for _, item := range code.Structure {
				fmt.Printf("  %s: %v at line %d\n", item.Kind, item.Name, item.Span.StartLine)
			}
		}
	}
}

func boolPtr(b bool) *bool { return &b }
```

For configuration details, see the [Code Intelligence Guide](../guides/code-intelligence.md).

---

## Related Resources

- **Source:** [packages/go/v4/](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go/v4) (Go binding implementation)
- **FFI Bridge:** [crates/kreuzberg-ffi/](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-ffi) (C FFI layer)
- **Rust Core:** [crates/kreuzberg/](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg) (extraction logic)
- **E2E Tests:** [e2e/go/](https://github.com/kreuzberg-dev/kreuzberg/tree/main/e2e/go) (auto-generated test fixtures)
- **CI:** [.github/workflows/ci.yaml](https://github.com/kreuzberg-dev/kreuzberg/blob/main/.github/workflows/ci.yaml) (test pipeline)
