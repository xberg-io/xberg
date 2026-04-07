# Java API Reference <span class="version-badge">v4.7.4</span>

Complete reference for the Kreuzberg Java bindings using Java 25+ Foreign Function & Memory API (FFM/Panama).

## Installation

Add the dependency to your Maven `pom.xml`:

```xml title="pom.xml"
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.7.4</version>
</dependency>
```

Or with Gradle:

```gradle title="build.gradle"
dependencies {
    implementation 'dev.kreuzberg:kreuzberg:4.7.4'
}
```

**Requirements:**

- Java 25 or later
- libkreuzberg_ffi native library (auto-loaded)
- Optional: Tesseract or EasyOCR/PaddleOCR for OCR functionality

View package on [Maven Central](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg).

---

## Core Functions

### batchExtractBytes()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```java title="Java"
public static List<ExtractionResult> batchExtractBytes(List<BytesWithMime> items, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `items` (List<BytesWithMime>): List of byte data with MIME types
- `config` (ExtractionConfig): Optional extraction configuration applied to all items. Uses defaults if null.

**Returns:**

- `List<ExtractionResult>`: List of extraction results in the same order as input items

**Throws:**

- `KreuzbergException`: If batch extraction fails

---

**Example - Basic usage:**

```java title="BasicExtraction.java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;

try {
    // Extract content from a PDF file
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
    System.out.println(result.getContent());
    System.out.println("MIME Type: " + result.getMimeType());
} catch (IOException e) {
    System.err.println("File error: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}
```

**Example - With OCR:**

```java title="WithOcr.java"
import dev.kreuzberg.*;
import dev.kreuzberg.config.*;

// Configure OCR for scanned documents
ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
System.out.println(result.getContent());
```

**Example - With multiple options:**

```java title="AdvancedExtraction.java"
// Configure extraction with multiple options for comprehensive processing
ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .forceOcr(false)
    .enableQualityProcessing(true)
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+fra")
        .build())
    .pdfOptions(PdfConfig.builder()
        .extractImages(true)
        .extractMetadata(true)
        .build())
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```

---

### extractBytes()

Extract content from byte array (synchronous).

**Signature:**

```java title="Java"
public static ExtractionResult extractBytes(byte[] data, String mimeType, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `data` (byte[]): File content as bytes (must not be empty)
- `mimeType` (String): MIME type of the data (required for format detection)
- `config` (ExtractionConfig): Optional extraction configuration

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `KreuzbergException`: If extraction or validation fails

**Example - Basic usage:**

```java title="ByteExtraction.java"
import dev.kreuzberg.Kreuzberg;

// Extract from in-memory byte array
byte[] pdfBytes = /* read from file or stream */;
ExtractionResult result = Kreuzberg.extractBytes(pdfBytes, "application/pdf", null);
System.out.println(result.getContent());
```

**Example - With configuration:**

```java title="ByteExtraction.java"
// Extract from bytes with quality processing enabled
ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

byte[] docxBytes = /* ... */;
ExtractionResult result = Kreuzberg.extractBytes(docxBytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", config);
```

### batchExtractBytesAsync()

Extract multiple byte arrays in parallel (asynchronous).

**Signature:**

```java title="Java"
public static CompletableFuture<List<ExtractionResult>> batchExtractBytesAsync(
    List<BytesWithMime> items,
    ExtractionConfig config
)
```

**Returns:**

- `CompletableFuture<List<ExtractionResult>>`: Future that completes with the list of results

---

### batchExtractFiles()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```java title="Java"
public static List<ExtractionResult> batchExtractFiles(List<String> paths, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `paths` (List<String>): List of file paths to extract
- `config` (ExtractionConfig): Optional extraction configuration applied to all files

**Returns:**

- `List<ExtractionResult>`: List of extraction results (one per file)

**Throws:**

- `KreuzbergException`: If batch extraction fails

### batchExtractFilesAsync()

Extract multiple files in parallel (asynchronous).

**Signature:**

```java title="Java"
public static CompletableFuture<List<ExtractionResult>> batchExtractFilesAsync(
    List<String> paths,
    ExtractionConfig config
)
```

**Returns:**

- `CompletableFuture<List<ExtractionResult>>`: Future that completes with extraction results

---

### batchExtractFilesWithConfigs() <span class="version-badge">v4.7.4</span>

Extract multiple files in parallel with per-file configuration overrides (synchronous).

**Signature:**

```java title="Java"
public static List<ExtractionResult> batchExtractFilesWithConfigs(
    List<FileWithConfig> items,
    ExtractionConfig config
) throws KreuzbergException
```

**Parameters:**

- `items` (List<FileWithConfig>): List of file path + per-file config pairs. `null` config uses batch defaults.
- `config` (ExtractionConfig): Batch-level extraction configuration

---

### batchExtractBytesWithConfigs() <span class="version-badge">v4.7.4</span>

Extract multiple byte arrays in parallel with per-file configuration overrides (synchronous).

**Signature:**

```java title="Java"
public static List<ExtractionResult> batchExtractBytesWithConfigs(
    List<BytesWithMimeAndConfig> items,
    ExtractionConfig config
) throws KreuzbergException
```

---

### FileExtractionConfig <span class="version-badge">v4.7.4</span>

Per-file extraction configuration overrides for batch operations. All fields are `Optional<T>` — empty means "use the batch-level default."

```java title="FileExtractionConfig.java"
public record FileExtractionConfig(
    Optional<Boolean> enableQualityProcessing,
    Optional<OcrConfig> ocr,
    Optional<Boolean> forceOcr,
    Optional<ChunkingConfig> chunking,
    Optional<ImageExtractionConfig> images,
    Optional<PdfConfig> pdfOptions,
    Optional<TokenReductionConfig> tokenReduction,
    Optional<LanguageDetectionConfig> languageDetection,
    Optional<PageConfig> pages,
    Optional<PostProcessorConfig> postprocessor,
    Optional<String> outputFormat,
    Optional<String> resultFormat,
    Optional<Boolean> includeDocumentStructure
) {}
```

Batch-level fields (`maxConcurrentExtractions`, `useCache`, `acceleration`, `securityLimits`) cannot be overridden per file. See [Configuration Reference](configuration.md#fileextractionconfig) for details.

---

### clearDocumentExtractors()

Remove all registered custom document extractors.

**Signature:**

```java title="Java"
public static void clearDocumentExtractors() throws KreuzbergException
```

---

### clearOCRBackends()

Remove all registered custom OCR backends.

**Signature:**

```java title="Java"
public static void clearOCRBackends() throws KreuzbergException
```

---

### clearPostProcessors()

Remove all registered custom post-processors.

**Signature:**

```java title="Java"
public static void clearPostProcessors() throws KreuzbergException
```

---

### clearValidators()

Remove all registered custom validators.

**Signature:**

```java title="Java"
public static void clearValidators() throws KreuzbergException
```

---

### detectMimeType()

Detect MIME type from file path or raw bytes.

**Signature:**

```java title="Java"
public static String detectMimeType(String path) throws KreuzbergException
public static String detectMimeType(String path, boolean checkExists) throws KreuzbergException
public static String detectMimeType(byte[] data) throws KreuzbergException
```

**Parameters:**

- `path` (String): Path to the file
- `checkExists` (boolean): Whether to verify file existence (default: true)
- `data` (byte[]): Raw bytes to analyze

**Returns:**

- `String`: Detected MIME type (e.g., "application/pdf")

---

### detectMimeTypeFromPath()

Detect MIME type from a file path (alias for `detectMimeType(path, true)`).

**Signature:**

```java title="Java"
public static String detectMimeTypeFromPath(String path) throws KreuzbergException
```

---

### discoverExtractionConfig()

Discover extraction configuration from environment or configuration files.

**Signature:**

```java title="Java"
public static Optional<ExtractionConfig> discoverExtractionConfig() throws KreuzbergException
```

**Returns:**

- `Optional<ExtractionConfig>`: Discovered configuration if found

---

### getEmbeddingPreset()

Retrieve details of a specific embedding preset.

**Signature:**

```java title="Java"
public static Optional<EmbeddingPreset> getEmbeddingPreset(String name) throws KreuzbergException
```

---

### getExtensionsForMime()

Get common file extensions for a given MIME type.

**Signature:**

```java title="Java"
public static List<String> getExtensionsForMime(String mimeType) throws KreuzbergException
```

**Returns:**

- `List<String>`: List of extensions (e.g., ["pdf"])

---

### getVersion()

Get the current version of the Kreuzberg library.

**Signature:**

```java title="Java"
public static String getVersion()
```

---

### listDocumentExtractors()

List names of all registered document extractors.

**Signature:**

```java title="Java"
public static List<String> listDocumentExtractors() throws KreuzbergException
```

---

### listEmbeddingPresets()

List names of all available embedding presets.

**Signature:**

```java title="Java"
public static List<String> listEmbeddingPresets() throws KreuzbergException
```

---

### listOCRBackends()

List names of all registered OCR backends.

**Signature:**

```java title="Java"
public static List<String> listOCRBackends() throws KreuzbergException
```

---

### listPostProcessors()

List names of all registered post-processors.

**Signature:**

```java title="Java"
public static List<String> listPostProcessors() throws KreuzbergException
```

---

### listValidators()

List names of all registered validators.

**Signature:**

```java title="Java"
public static List<String> listValidators() throws KreuzbergException
```

---

### loadExtractionConfigFromFile()

Load extraction configuration from a file.

**Signature:**

```java title="Java"
public static ExtractionConfig loadExtractionConfigFromFile(Path path) throws KreuzbergException
```

---

### registerOcrBackend()

Register a custom OCR backend.

**Signature:**

```java title="Java"
public static void registerOcrBackend(String name, OcrBackend backend) throws KreuzbergException
public static void registerOcrBackend(String name, OcrBackend backend, List<String> supportedLanguages) throws KreuzbergException
```

---

### registerPostProcessor()

Register a custom post-processor.

**Signature:**

```java title="Java"
public static void registerPostProcessor(String name, PostProcessor processor) throws KreuzbergException
public static void registerPostProcessor(String name, PostProcessor processor, int priority, ProcessingStage stage) throws KreuzbergException
```

---

### registerValidator()

Register a custom validator.

**Signature:**

```java title="Java"
public static void registerValidator(String name, Validator validator) throws KreuzbergException
public static void registerValidator(String name, Validator validator, int priority) throws KreuzbergException
```

---

### unregisterDocumentExtractor()

Unregister a document extractor by name.

**Signature:**

```java title="Java"
public static void unregisterDocumentExtractor(String name) throws KreuzbergException
```

---

### unregisterOCRBackend()

Unregister an OCR backend by name.

**Signature:**

```java title="Java"
public static void unregisterOCRBackend(String name) throws KreuzbergException
```

---

### unregisterPostProcessor()

Unregister a post-processor by name.

**Signature:**

```java title="Java"
public static void unregisterPostProcessor(String name) throws KreuzbergException
```

---

### unregisterValidator()

Unregister a validator by name.

**Signature:**

```java title="Java"
public static void unregisterValidator(String name) throws KreuzbergException
```

---

### validateMimeType()

Validate a MIME type string and return its normalized form.

**Signature:**

```java title="Java"
public static String validateMimeType(String mimeType) throws KreuzbergException
```

---

## Configuration

### ExtractionConfig

Main extraction configuration using builder pattern.

**Builder Methods:**

```java title="ExtractionConfig.java"
// Build extraction configuration with all available options
ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig)                          // Text chunking configuration
    .concurrency(ConcurrencyConfig)                    // Concurrency control settings
    .enableQualityProcessing(false)                    // Enable quality processing (default: false)
    .forceOcr(false)                                   // Force OCR on all pages (default: false)
    .htmlOptions(HtmlOptions)                          // HTML conversion options
    .imageExtraction(ImageExtractionConfig)            // Image extraction settings
    .imagePreprocessing(ImagePreprocessingConfig)      // Image preprocessing
    .includeDocumentStructure(false)                   // Include document structure (default: false)
    .keywords(KeywordConfig)                           // Keyword extraction settings
    .languageDetection(LanguageDetectionConfig)        // Language detection settings
    .layout(LayoutDetectionConfig)                     // Layout detection settings
    .maxConcurrentExtractions(4)                       // Max concurrent extractions
    .ocr(OcrConfig)                                    // OCR configuration
    .outputFormat("plain")                             // Content format: "plain", "markdown", "djot", "html"
    .pdfOptions(PdfConfig)                             // PDF-specific options
    .postprocessor(PostProcessorConfig)                // Post-processor settings
    .resultFormat("unified")                           // Result format: "unified", "element_based"
    .securityLimits(SecurityLimitsConfig)              // Security limits configuration
    .tokenReduction(TokenReductionConfig)              // Token reduction configuration
    .useCache(true)                                    // Enable caching (default: true)
    .build();
```

**Static Methods:**

```java title="ConfigLoading.java"
// Load configuration from file (TOML, YAML, or JSON)
ExtractionConfig config = ExtractionConfig.fromFile("kreuzberg.toml");

// Automatically discover configuration file in current/parent directories
ExtractionConfig config = ExtractionConfig.discover(); // Returns null if not found
```

---

### OcrConfig

OCR configuration for text extraction from images.

**Builder Methods:**

```java title="OcrConfiguration.java"
// Configure OCR backend and language settings
OcrConfig ocr = OcrConfig.builder()
    .backend("tesseract")          // "tesseract", "easyocr", "paddleocr", etc.
    .language("eng")               // Language code(s), comma-separated for multiple
    .tesseractConfig(config)       // Tesseract-specific configuration
    .paddleOcrConfig(paddleConfig) // PaddleOCR-specific configuration
    .build();
```

**PaddleOcrConfig Fields:** <span class="version-badge">v4.7.4</span>

- `modelTier` (String): Model tier: "mobile" (lightweight, ~21MB total, fast) or "server" (high accuracy, ~172MB, best with GPU). Default: "mobile"
- `padding` (Integer): Padding in pixels (0-100) added around the image before detection. Default: 10

```java title="PaddleOcrConfiguration.java"
PaddleOcrConfig paddleConfig = PaddleOcrConfig.builder()
    .modelTier("server")
    .padding(10)
    .build();
```

**Example - Multi-language OCR:**

```java title="MultiLanguageOcr.java"
// Configure OCR to support multiple languages simultaneously
OcrConfig ocr = OcrConfig.builder()
    .backend("tesseract")
    .language("eng+fra+deu")       // English, French, and German
    .build();
```

---

### OcrBackend Interface

Custom OCR backend implementation for cloud-based or specialized OCR.

**Interface:**

```java title="Java"
public interface OcrBackend {
    /**
     * Process image and extract text.
     *
     * @param imageData raw image bytes
     * @param configJson OCR configuration as JSON
     * @return extracted text, or null if processing fails
     */
    String processImage(byte[] imageData, String configJson) throws Exception;

    /**
     * Languages supported by this backend.
     *
     * @return list of language codes (empty for all languages)
     */
    List<String> supportedLanguages();
}
```

**Example - Custom OCR Backend:**

```java title="CustomOcrBackend.java"
// Implement custom OCR backend for cloud-based or specialized OCR services
class CustomOcrBackend implements OcrBackend {
    @Override
    public String processImage(byte[] imageData, String configJson) throws Exception {
        // Call custom OCR service (e.g., Google Cloud Vision, AWS Textract)
        return callCustomOcrService(imageData);
    }

    @Override
    public List<String> supportedLanguages() {
        return List.of("eng", "fra", "deu");
    }
}

// Register the custom backend with Kreuzberg
OcrBackend backend = new CustomOcrBackend();
Kreuzberg.registerOcrBackend("custom-ocr", backend);
```

---

### ChunkingConfig

Configuration for splitting extracted text into chunks.

**Builder Methods:**

```java title="ChunkingConfiguration.java"
// Configure text chunking for RAG and embedding workflows
ChunkingConfig chunking = ChunkingConfig.builder()
    .maxChars(1000)              // Maximum characters per chunk
    .maxOverlap(200)             // Character overlap between chunks
    .preset("large")             // Preset: "small", "medium", "large"
    .enabled(true)               // Enable chunking (default: true)
    .embedding(embeddingMap)     // Embedding configuration
    .sizingTokenizer("bert-base-uncased") // Measure size by token count using a HuggingFace tokenizer
    // .sizingCharacters()       // Measure size by character count (default)
    // .sizingCacheDir("/tmp/tokenizers") // Optional: cache directory for tokenizer files
    .build();
```

---

### LanguageDetectionConfig

Configuration for automatic language detection.

**Builder Methods:**

```java title="LanguageDetection.java"
// Configure automatic language detection with confidence threshold
LanguageDetectionConfig langDetect = LanguageDetectionConfig.builder()
    .enabled(true)               // Enable language detection
    .minConfidence(0.8)          // Minimum confidence threshold (0.0-1.0)
    .build();
```

---

### PdfConfig

PDF-specific extraction options.

**Builder Methods:**

```java title="PdfConfiguration.java"
// Configure PDF-specific extraction options
PdfConfig pdf = PdfConfig.builder()
    .allowSingleColumnTables(false) // <span class="version-badge">v4.7.4</span> Allow extraction of single-column tables
    .extractImages(true)         // Extract images from PDF
    .extractMetadata(true)       // Extract PDF metadata
    .renderImages(false)         // Render pages as images for processing
    .build();
```

---

### ImageExtractionConfig

Configuration for image extraction from documents.

**Builder Methods:**

```java title="ImageExtraction.java"
// Configure image extraction settings
ImageExtractionConfig images = ImageExtractionConfig.builder()
    .extractImages(true)         // Enable image extraction
    .targetDpi(150)              // Target DPI for extraction
    .maxImageDimension(4096)     // Maximum image dimension in pixels
    .build();
```

---

### ImagePreprocessingConfig

Configuration for preprocessing images before OCR.

**Builder Methods:**

```java title="ImagePreprocessing.java"
// Configure image preprocessing to improve OCR accuracy
ImagePreprocessingConfig preproc = ImagePreprocessingConfig.builder()
    .targetDpi(300)              // Target DPI for OCR
    .denoise(true)               // Apply denoising
    .deskew(true)                // Deskew images
    .contrastEnhance(true)       // Enhance contrast
    .build();
```

---

### ConcurrencyConfig <span class="version-badge">v4.7.4</span>

Concurrency configuration for controlling parallel extraction.

**Builder Methods:**

```java title="ConcurrencyConfiguration.java"
// Configure concurrency control for parallel extraction
ConcurrencyConfig concurrency = ConcurrencyConfig.builder()
    .maxThreads(4)               // Maximum number of concurrent threads
    .build();
```

**Fields:**

- `maxThreads` (Integer): Maximum number of concurrent threads for parallel extraction. Default: null (system default)

**Example:**

```java title="ConcurrencyExample.java"
var config = ExtractionConfig.builder()
    .concurrency(ConcurrencyConfig.builder()
        .maxThreads(4)
        .build())
    .build();
```

---

### TokenReductionConfig

Configuration for token reduction (reducing extracted text size).

**Builder Methods:**

```java title="TokenReduction.java"
// Configure token reduction to minimize extracted text size
TokenReductionConfig tokenReduce = TokenReductionConfig.builder()
    .mode("moderate")            // Mode: "none", "light", "moderate", "aggressive"
    .preserveImportantWords(true) // Preserve important words
    .build();
```

---

### PostProcessorConfig

Configuration for post-processing.

**Builder Methods:**

```java title="PostProcessor.java"
// Configure post-processing for extraction results
PostProcessorConfig postproc = PostProcessorConfig.builder()
    .enabled(true)               // Enable post-processing
    .build();
```

---

### HtmlOptions

Configuration for HTML to Markdown conversion.

**Builder Methods:**

```java title="HtmlConfiguration.java"
// Configure HTML to Markdown conversion options
HtmlOptions html = HtmlOptions.builder()
    .headingStyle("atx")         // "atx", "underlined", "atx_closed"
    .codeBlockStyle("backticks") // "indented", "backticks", "tildes"
    .build();
```

---

### KeywordConfig

Configuration for keyword extraction.

**Builder Methods:**

```java title="KeywordExtraction.java"
// Configure automatic keyword extraction from content
KeywordConfig keywords = KeywordConfig.builder()
    .enabled(true)
    .maxKeywords(10)
    .minKeywordLength(3)
    .build();
```

---

## Results & Types

### ExtractionResult

Result of a document extraction operation. All fields follow camelCase naming conventions.

**Accessors:**

```java title="ResultAccess.java"
// Core fields
String content = result.getContent();                                // Extracted text content
String mimeType = result.getMimeType();                              // Detected MIME type
Metadata metadata = result.getMetadata();                            // Document metadata (typed)

// Extraction artifacts
List<Table> tables = result.getTables();                              // Extracted tables
List<Chunk> chunks = result.getChunks();                              // Text chunks
List<ExtractedImage> images = result.getImages();                    // Extracted images
List<PageContent> pages = result.getPages();                         // Per-page content

// Semantic & OCR elements
List<Element> elements = result.getElements();                       // Semantic elements
List<OcrElement> ocrElements = result.getOcrElements();              // OCR elements with geometry

// Structure & Analysis
Optional<DjotContent> djotContent = result.getDjotContent();         // Djot content structure
Optional<DocumentStructure> document = result.getDocumentStructure(); // Document structure
Optional<PageStructure> pageStructure = result.getPageStructure();   // Page structure info
List<String> detectedLanguages = result.getDetectedLanguages();      // All detected languages
Optional<String> detectedLanguage = result.getDetectedLanguage();    // Primary detected language
Optional<List<ExtractedKeyword>> keywords = result.getExtractedKeywords(); // Extracted keywords

// Quality & Warnings
Optional<Double> qualityScore = result.getQualityScore();            // Quality score (0.0–1.0)
Optional<List<ProcessingWarning>> warnings = result.getProcessingWarnings(); // Processing warnings
Optional<List<PdfAnnotation>> annotations = result.getAnnotations();  // PDF annotations

// Helper methods
int pageCount = result.getPageCount();                               // Total page count
int chunkCount = result.getChunkCount();                             // Total chunk count
Optional<Object> title = result.getMetadataField("title");           // Unified metadata access
```

**Example - Accessing results:**

```java title="ResultProcessing.java"
ExtractionResult result = Kreuzberg.extractFile("document.pdf");

// Display basic extraction statistics
System.out.println("Content length: " + result.getContent().length());
System.out.println("MIME: " + result.getMimeType());
System.out.println("Pages: " + result.getPageCount());
System.out.println("Chunks: " + result.getChunkCount());

// Access typed metadata
Metadata meta = result.getMetadata();
meta.getTitle().ifPresent(t -> System.out.println("Title: " + t));
meta.getAuthors().ifPresent(a -> System.out.println("Authors: " + String.join(", ", a)));

// Process chunks for RAG workflows
for (Chunk chunk : result.getChunks()) {
    System.out.println("Chunk [" + chunk.getMetadata().getChunkIndex() + "]: " + chunk.getContent());
}
```

#### pages

**Type**: `List<PageContent>`

Per-page extracted content when page extraction is enabled via `PageConfig.extractPages = true`.

Each page contains:

- Page number (1-indexed)
- Text content for that page
- Tables on that page
- Images on that page

**Example:**

```java title="PageExtraction.java"
import dev.kreuzberg.*;

var config = ExtractionConfig.builder()
    .pages(PageConfig.builder()
        .extractPages(true)
        .build())
    .build();

var result = Kreuzberg.extractFile("document.pdf", config);

if (result.getPages() != null) {
    for (var page : result.getPages()) {
        System.out.println("Page " + page.getPageNumber() + ":");
        System.out.println("  Content: " + page.getContent().length() + " chars");
        System.out.println("  Tables: " + page.getTables().size());
        System.out.println("  Images: " + page.getImages().size());
    }
}
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```java title="IteratePages.java"
import dev.kreuzberg.*;

var config = ExtractionConfig.builder()
    .pages(PageConfig.builder()
        .extractPages(true)
        .insertPageMarkers(true)
        .markerFormat("\n\n--- Page {page_num} ---\n\n")
        .build())
    .build();

var result = Kreuzberg.extractFile("document.pdf", config);

// Access combined content with page markers
System.out.println("Combined content with markers:");
System.out.println(result.getContent().substring(0, 500));
System.out.println();

// Access per-page content
if (result.getPages() != null) {
    for (var page : result.getPages()) {
        System.out.println("Page " + page.getPageNumber() + ":");
        String preview = page.getContent().substring(0, Math.min(100, page.getContent().length()));
        System.out.println("  " + preview + "...");
        if (!page.getTables().isEmpty()) {
            System.out.println("  Found " + page.getTables().size() + " table(s)");
        }
        if (!page.getImages().isEmpty()) {
            System.out.println("  Found " + page.getImages().size() + " image(s)");
        }
    }
}
```

### Metadata

Typed document metadata extracted from various formats.

**Accessors:**

```java title="MetadataAccess.java"
Optional<String> title = metadata.getTitle();
Optional<String> subject = metadata.getSubject();
Optional<List<String>> authors = metadata.getAuthors();
Optional<List<String>> keywords = metadata.getKeywords();
Optional<String> language = metadata.getLanguage();
Optional<String> createdAt = metadata.getCreatedAt();
Optional<String> modifiedAt = metadata.getModifiedAt();
Optional<String> createdBy = metadata.getCreatedBy();
Optional<String> modifiedBy = metadata.getModifiedBy();
Optional<PageStructure> pages = metadata.getPages();
Optional<String> category = metadata.getCategory();
Optional<List<String>> tags = metadata.getTags();
Optional<String> version = metadata.getDocumentVersion();
Optional<String> abstractText = metadata.getAbstractText();
Optional<String> outputFormat = metadata.getOutputFormat();
Optional<Long> durationMs = metadata.getExtractionDurationMs();

// Form-specific or post-processor metadata (legacy)
Map<String, Object> additional = metadata.getAdditional();
```

---

### Table

Represents a table extracted from a document.

**Accessors:**

```java title="TableAccess.java"
List<List<String>> cells = table.cells();              // 2D list of cell values
String markdown = table.markdown();                    // Markdown representation
int pageNumber = table.pageNumber();                   // Page number (1-indexed)
BoundingBox boundingBox = table.boundingBox();         // Bounding box coordinates

// Helper methods
int rows = table.getRowCount();                        // Number of rows
int cols = table.getColumnCount();                     // Number of columns
String cell = table.getCell(row, col);                // Get specific cell
List<String> row = table.getRow(rowIndex);            // Get specific row
```

**Example:**

```java title="TableProcessing.java"
List<Table> tables = result.getTables();

// Process all extracted tables
for (Table table : tables) {
    System.out.println("Table on page " + table.getPageNumber() + ":");
    System.out.println("Size: " + table.getRowCount() + " x " + table.getColumnCount());
    System.out.println(table.getMarkdown());

    // Iterate through all cells in the table
    for (int r = 0; r < table.getRowCount(); r++) {
        for (int c = 0; c < table.getColumnCount(); c++) {
            System.out.print(table.getCell(r, c) + " | ");
        }
        System.out.println();
    }
}
```

---

### Chunk

Represents a chunk of extracted text (for RAG/embeddings).

**Accessors:**

```java title="ChunkAccess.java"
String content = chunk.getContent();                   // Chunk text
ChunkMetadata metadata = chunk.getMetadata();          // Chunk metadata
Optional<List<Float>> embedding = chunk.getEmbedding(); // Embedding vector
```

**Example:**

```java title="ChunkProcessing.java"
// Configure chunking for RAG workflow
ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

// Process each chunk (e.g., for embedding generation)
for (Chunk chunk : result.getChunks()) {
    System.out.println("Chunk " + chunk.getIndex() + ": " + chunk.getContent().substring(0, 50) + "...");
}
```

---

### ChunkMetadata

Metadata describing where a chunk appears within the original document.

**Accessors:**

```java title="ChunkMetadataAccess.java"
long byteStart = metadata.getByteStart();              // UTF-8 start byte offset
long byteEnd = metadata.getByteEnd();                  // UTF-8 end byte offset
int chunkIndex = metadata.getChunkIndex();             // 0-based index
int totalChunks = metadata.getTotalChunks();           // Total chunks in document
Optional<Long> firstPage = metadata.getFirstPage();    // Start page number
Optional<Long> lastPage = metadata.getLastPage();      // End page number
Optional<Integer> tokenCount = metadata.getTokenCount(); // Token count
Optional<HeadingContext> headings = metadata.getHeadingContext(); // Heading hierarchy
```

**Fields:**

- `byteEnd` (long): UTF-8 byte offset in content (exclusive).
- `byteStart` (long): UTF-8 byte offset in content (inclusive).
- `chunkIndex` (int): Zero-based index of this chunk.
- `totalChunks` (int): Total number of chunks in document.
- `firstPage` (Optional\<Long\>): First page this chunk appears on (1-indexed).
- `lastPage` (Optional\<Long\>): Last page this chunk appears on (1-indexed).
- `tokenCount` (Optional\<Integer\>): Estimated token count (if configured).
- `headingContext` (Optional\<HeadingContext\>): Heading hierarchy for section-based chunking.

**Page tracking:** When `PageStructure.boundaries` is available and chunking is enabled, `firstPage` and `lastPage` are automatically calculated based on byte offsets.

**Example:**

```java title="ChunkMetadataExample.java"
import dev.kreuzberg.*;

var config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .build())
    .pages(PageConfig.builder()
        .extractPages(true)
        .build())
    .build();

var result = Kreuzberg.extractFile("document.pdf", config);

if (result.getChunks() != null) {
    for (var chunk : result.getChunks()) {
        var meta = chunk.getMetadata();
        String pageInfo = "";

        if (meta.getFirstPage().isPresent()) {
            int first = meta.getFirstPage().get();
            int last = meta.getLastPage().orElse(first);

            if (first == last) {
                pageInfo = " (page " + first + ")";
            } else {
                pageInfo = " (pages " + first + "-" + last + ")";
            }
        }

        System.out.printf(
            "Chunk [%d:%d]: %d chars%s%n",
            meta.getByteStart(),
            meta.getByteEnd(),
            meta.getCharCount(),
            pageInfo
        );
    }
}
```

---

### ExtractedImage

Represents an image extracted from a document.

**Accessors:**

```java title="ImageAccess.java"
byte[] data = image.data();                            // Image binary data
String format = image.format();                        // Image format (png, jpg, etc.)
String mimeType = image.mimeType();                    // MIME type
int pageNumber = image.pageNumber();                   // Page number (1-indexed)
ImageDimensions dimensions = image.dimensions();       // Image width and height
BoundingBox boundingBox = image.boundingBox();         // Location in document
Optional<String> ocrResult = image.ocrResult();        // Text extracted from image
```

---

### OcrElement

Represents a low-level text element detected via OCR.

**Accessors:**

```java title="OcrElementAccess.java"
String text = element.text();                          // Detected text
BoundingBox geometry = element.geometry();             // Element coordinates
double confidence = element.confidence();              // OCR confidence (0.0–1.0)
int level = element.level();                           // Semantic level (e.g., word, line)
```

---

### BoundingBox

Represents spatial coordinates for elements in a document.

**Accessors:**

```java title="BoundingBoxAccess.java"
double left = box.left();                              // X-coordinate (left)
double top = box.top();                                // Y-coordinate (top)
double width = box.width();                            // Element width
double height = box.height();                          // Element height
```

---

## Extensibility

### Custom Post-Processors

Post-processors enrich extraction results by transforming content or adding metadata.

**Interface:**

```java title="Java"
@FunctionalInterface
public interface PostProcessor {
    /**
     * Process and enrich an extraction result.
     *
     * @param result the extraction result
     * @return the processed result
     */
    ExtractionResult process(ExtractionResult result) throws KreuzbergException;

    /**
     * Compose with another processor.
     *
     * @param after the next processor
     * @return composed processor
     */
    default PostProcessor andThen(PostProcessor after) {
        return result -> after.process(this.process(result));
    }

    /**
     * Execution stage.
     *
     * @return the processing stage (EARLY, MIDDLE, LATE)
     */
    default ProcessingStage processingStage() {
        return ProcessingStage.MIDDLE;
    }

    /**
     * Execution priority within stage (higher = earlier).
     *
     * @return priority value
     */
    default int priority() {
        return 0;
    }
}
```

**Example - Word count processor:**

```java title="CustomPostProcessor.java"
import dev.kreuzberg.*;

// Create a post-processor that adds word count to metadata
PostProcessor wordCount = result -> {
    long count = result.getContent().split("\\s+").length;

    Map<String, Object> metadata = new HashMap<>(result.getMetadata());
    metadata.put("word_count", count);

    return new ExtractionResult(
        result.getContent(),
        result.getMimeType(),
        metadata,
        result.getTables(),
        result.getDetectedLanguages(),
        result.getChunks(),
        result.getImages(),
        result.isSuccess()
    );
};

// Register the processor with priority 50 in MIDDLE stage
Kreuzberg.registerPostProcessor("word-count", wordCount, 50, ProcessingStage.MIDDLE);

// Extract file and access the word count metadata
ExtractionResult result = Kreuzberg.extractFile("document.pdf");
System.out.println("Word count: " + result.getMetadata().get("word_count"));
```

**Example - Uppercase transformer:**

```java title="CustomPostProcessor.java"
// Create a post-processor that transforms content to uppercase
PostProcessor uppercase = result -> {
    return new ExtractionResult(
        result.getContent().toUpperCase(),
        result.getMimeType(),
        result.getMetadata(),
        result.getTables(),
        result.getDetectedLanguages(),
        result.getChunks(),
        result.getImages(),
        result.isSuccess()
    );
};

// Register the uppercase transformer
Kreuzberg.registerPostProcessor("uppercase", uppercase);
```

---

### Custom Validators

Validators check extraction results for quality or completeness.

**Interface:**

```java title="Java"
@FunctionalInterface
public interface Validator {
    /**
     * Validate an extraction result.
     *
     * @param result the extraction result
     * @throws ValidationException if validation fails
     */
    void validate(ExtractionResult result) throws ValidationException;

    /**
     * Compose with another validator.
     *
     * @param after the next validator
     * @return composed validator
     */
    default Validator andThen(Validator after) {
        return result -> {
            this.validate(result);
            after.validate(result);
        };
    }

    /**
     * Execution priority (higher = earlier).
     *
     * @return priority value
     */
    default int priority() {
        return 0;
    }
}
```

**Example - Minimum content length validator:**

```java title="CustomValidator.java"
// Create a validator that ensures minimum content length
Validator minLength = result -> {
    if (result.getContent().length() < 100) {
        throw new ValidationException(
            "Content too short: " + result.getContent().length() + " < 100"
        );
    }
};

// Register the validator
Kreuzberg.registerValidator("min-length", minLength);
```

**Example - Quality score validator:**

```java title="CustomValidator.java"
// Create a validator that checks extraction quality score
Validator qualityValidator = result -> {
    double score = result.getQualityScore() != null ? result.getQualityScore() : 0.0;

    if (score < 0.5) {
        throw new ValidationException(
            String.format("Quality score too low: %.2f < 0.50", score)
        );
    }
};

// Register the quality validator
Kreuzberg.registerValidator("quality", qualityValidator);
```

---

### Plugin Management

Register, list, and unregister plugins.

**Post-Processor Management:**

```java title="PluginManagement.java"
// Register post-processor with default settings
Kreuzberg.registerPostProcessor("processor-name", processor);

// Register with custom priority and execution stage
Kreuzberg.registerPostProcessor("processor-name", processor, 100, ProcessingStage.EARLY);

// Unregister a specific processor
Kreuzberg.unregisterPostProcessor("processor-name");

// List all registered post-processors
List<String> processors = Kreuzberg.listPostProcessors();

// Remove all post-processors
Kreuzberg.clearPostProcessors();
```

**Validator Management:**

```java title="PluginManagement.java"
// Register validator with default priority
Kreuzberg.registerValidator("validator-name", validator);

// Register with custom priority (higher = earlier execution)
Kreuzberg.registerValidator("validator-name", validator, 100);

// Unregister a specific validator
Kreuzberg.unregisterValidator("validator-name");

// List all registered validators
List<String> validators = Kreuzberg.listValidators();

// Remove all validators
Kreuzberg.clearValidators();
```

**OCR Backend Management:**

```java title="OcrBackendManagement.java"
// Register custom OCR backend
Kreuzberg.registerOcrBackend("backend-name", backend);

// Register with supported language filtering
Kreuzberg.registerOcrBackend("backend-name", backend, List.of("eng", "fra", "deu"));

// Unregister a specific OCR backend
Kreuzberg.unregisterOCRBackend("backend-name");

// List all registered OCR backends
List<String> backends = Kreuzberg.listOCRBackends();

// Remove all custom OCR backends
Kreuzberg.clearOCRBackends();
```

---

## MIME Type Detection

### detectMimeType()

Detect MIME type from file or bytes.

**Signatures:**

```java title="Java"
public static String detectMimeType(String path) throws KreuzbergException
public static String detectMimeType(String path, boolean checkExists) throws KreuzbergException
public static String detectMimeType(byte[] data) throws KreuzbergException
public static String detectMimeTypeFromPath(String path) throws KreuzbergException
```

**Example:**

```java title="MimeTypeDetection.java"
// Detect MIME type from file path
String mimeType = Kreuzberg.detectMimeType("document.pdf");

// Detect from path without checking file existence
String mimeType = Kreuzberg.detectMimeType("document.pdf", false);

// Detect from raw byte array
byte[] data = /* ... */;
String mimeType = Kreuzberg.detectMimeType(data);
```

---

### validateMimeType()

Validate and normalize a MIME type string.

**Signature:**

```java title="Java"
public static String validateMimeType(String mimeType) throws KreuzbergException
```

**Example:**

```java title="MimeTypeValidation.java"
// Validate and normalize a MIME type string
String validated = Kreuzberg.validateMimeType("application/pdf");
System.out.println(validated); // "application/pdf"
```

---

### getExtensionsForMime()

Get file extensions for a given MIME type.

**Signature:**

```java title="Java"
public static List<String> getExtensionsForMime(String mimeType) throws KreuzbergException
```

**Example:**

```java title="MimeExtensions.java"
// Get file extensions for PDF files
List<String> extensions = Kreuzberg.getExtensionsForMime("application/pdf");
System.out.println(extensions); // ["pdf"]

// Get file extensions for JPEG images (multiple extensions possible)
List<String> extensions = Kreuzberg.getExtensionsForMime("image/jpeg");
System.out.println(extensions); // ["jpg", "jpeg"]
```

---

## Embeddings

### embed()

Generate embeddings for a list of texts synchronously.

**Signature:**

```java title="Java"
public static float[][] embed(List<String> texts, EmbeddingConfig config) throws KreuzbergException
```

**Parameters:**

- `texts` (List\<String\>): List of strings to embed.
- `config` (EmbeddingConfig): Embedding configuration, or `null` for the default "balanced" preset.

**Returns:** `float[][]` — one embedding vector per input text.

**Throws:** `KreuzbergException` if embedding generation fails or the `embeddings` feature is not enabled.

**Example:**

--8<-- "snippets/java/utils/standalone_embed.md"

---

### embedAsync()

Async variant of `embed()`. Returns a `CompletableFuture` that resolves to the embedding vectors.

**Signature:**

```java title="Java"
public static CompletableFuture<float[][]> embedAsync(List<String> texts, EmbeddingConfig config)
```

Same parameters and return type as `embed()`, wrapped in a `CompletableFuture`.

---

### getEmbeddingPreset()

Get embedding preset configuration by name.

**Signature:**

```java title="Java"
public static Optional<EmbeddingPreset> getEmbeddingPreset(String name) throws KreuzbergException
```

**Example:**

```java title="EmbeddingPresets.java"
// Retrieve an embedding preset configuration by name
Optional<EmbeddingPreset> preset = Kreuzberg.getEmbeddingPreset("default");
if (preset.isPresent()) {
    EmbeddingPreset p = preset.get();
    System.out.println("Model: " + p.getModel());
    System.out.println("Dimensions: " + p.getDimensions());
}
```

---

### listEmbeddingPresets()

List all available embedding presets.

**Signature:**

```java title="Java"
public static List<String> listEmbeddingPresets() throws KreuzbergException
```

**Example:**

```java title="EmbeddingPresets.java"
// List all available embedding presets
List<String> presets = Kreuzberg.listEmbeddingPresets();
for (String preset : presets) {
    System.out.println("Available: " + preset);
}
```

---

## PDF Rendering

!!! info "Added in v4.7.4"

### Kreuzberg.renderPdfPage()

Render a single page of a PDF as a PNG image.

**Signature:**

```java title="Java"
public static byte[] renderPdfPage(Path path, int pageIndex, int dpi) throws IOException, KreuzbergException
```

**Parameters:**

- `path` (Path): Path to the PDF file
- `pageIndex` (int): Zero-based page index to render
- `dpi` (int): Resolution for rendering (e.g. 150)

**Returns:**

- `byte[]`: PNG-encoded bytes for the requested page

**Example:**

```java title="RenderSinglePage.java"
byte[] png = Kreuzberg.renderPdfPage(Path.of("document.pdf"), 0, 150);
Files.write(Path.of("first_page.png"), png);
```

---

### Kreuzberg.PdfPageIterator

A more memory-efficient alternative to rendering all pages at once when memory is a concern or when pages should be processed as they are rendered (e.g., sending each page to a vision model for OCR). Renders one page at a time, so only one raw image is in memory at a time.

**Signature:**

```java title="Java"
public static class PdfPageIterator implements Iterator<PageResult>, AutoCloseable {
    public static PdfPageIterator open(Path path, int dpi) throws IOException, KreuzbergException;
    public boolean hasNext();
    public PageResult next();
    public int pageCount();
    public void close();
}

public record PageResult(int pageIndex, byte[] data) {}
```

**Example:**

```java title="IteratePages.java"
try (var iter = Kreuzberg.PdfPageIterator.open(Path.of("document.pdf"), 150)) {
    while (iter.hasNext()) {
        Kreuzberg.PageResult page = iter.next();
        Files.write(Path.of("page_" + page.pageIndex() + ".png"), page.data());
    }
}
```

---

## Error Handling

### Exception Hierarchy

Kreuzberg uses a checked exception model for error handling.

```text
Exception
├── IOException (from java.io)
├── KreuzbergException
│   ├── ParsingException
│   ├── OcrException
│   ├── MissingDependencyException
│   ├── ValidationException
│   ├── PluginException
│   ├── CacheException
│   └── ImageProcessingException
```

### Specific Exceptions

**KreuzbergException** - Base exception for all Kreuzberg errors.

```java title="ErrorHandling.java"
// Handle general Kreuzberg exceptions
try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
    if (e.getCause() != null) {
        e.getCause().printStackTrace();
    }
}
```

**ParsingException** - Document parsing failure.

```java title="ErrorHandling.java"
// Handle document parsing errors (e.g., corrupted files)
try {
    ExtractionResult result = Kreuzberg.extractFile("corrupted.pdf");
} catch (ParsingException e) {
    System.err.println("Failed to parse document: " + e.getMessage());
}
```

**OcrException** - OCR processing failure.

```java title="OcrErrorHandling.java"
// Handle OCR-specific errors
try {
    ExtractionConfig config = ExtractionConfig.builder()
        .forceOcr(true)
        .build();
    ExtractionResult result = Kreuzberg.extractFile("image.png", config);
} catch (OcrException e) {
    System.err.println("OCR failed: " + e.getMessage());
}
```

**MissingDependencyException** - Required system dependency not found.

```java title="DependencyErrorHandling.java"
// Handle missing system dependencies (e.g., Tesseract not installed)
try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} catch (MissingDependencyException e) {
    System.err.println("Missing dependency: " + e.getMessage());
    System.err.println("Install Tesseract or configure alternative OCR backend");
}
```

**ValidationException** - Configuration or validation failure.

```java title="ValidationErrorHandling.java"
// Handle validation errors from custom validators
try {
    validator.validate(result);
} catch (ValidationException e) {
    System.err.println("Validation failed: " + e.getMessage());
}
```

### Comprehensive Error Handling

```java title="ComprehensiveErrorHandling.java"
// Comprehensive error handling for all exception types
try {
    ExtractionConfig config = ExtractionConfig.builder()
        .ocr(OcrConfig.builder().backend("tesseract").language("eng").build())
        .build();

    ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
    System.out.println("Success: " + result.getContent().length() + " characters");

} catch (ParsingException e) {
    System.err.println("Document format not supported or corrupted");
    e.printStackTrace();
} catch (OcrException e) {
    System.err.println("OCR processing failed");
    e.printStackTrace();
} catch (MissingDependencyException e) {
    System.err.println("Missing required dependency");
    System.err.println("Message: " + e.getMessage());
} catch (ValidationException e) {
    System.err.println("Configuration validation failed");
} catch (IOException e) {
    System.err.println("File not found or not readable: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
} finally {
    // Clean up resources if needed
}
```

---

## Utility Methods

### getVersion()

Get the Kreuzberg library version.

**Signature:**

```java title="Java"
public static String getVersion()
```

**Example:**

```java title="VersionInfo.java"
// Get the Kreuzberg library version
String version = Kreuzberg.getVersion();
System.out.println("Kreuzberg version: " + version);
```

---

## Advanced Usage

### Configuration Discovery

Automatically discover configuration from kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in the current or parent directories.

```java title="ConfigDiscovery.java"
// Automatically discover configuration file in directory tree
ExtractionConfig config = ExtractionConfig.discover();

if (config != null) {
    System.out.println("Configuration discovered!");
    ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
} else {
    System.out.println("No configuration file found, using defaults");
}
```

---

### Configuration from File

Load configuration from a file explicitly.

```java title="ConfigLoading.java"
// Load configuration from a specific file
ExtractionConfig config = ExtractionConfig.fromFile("kreuzberg.toml");
ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```

---

### Complex Configuration Example

```java title="ComplexConfiguration.java"
import dev.kreuzberg.*;
import dev.kreuzberg.config.*;
import java.nio.file.Path;

public class ComplexExample {
    public static void main(String[] args) throws Exception {
        // Build comprehensive extraction configuration with all options
        ExtractionConfig config = ExtractionConfig.builder()
            .useCache(true)
            .enableQualityProcessing(true)
            .forceOcr(false)

            // Configure OCR for multi-language support
            .ocr(OcrConfig.builder()
                .backend("tesseract")
                .language("eng+fra")
                .build())

            // Configure PDF extraction options
            .pdfOptions(PdfConfig.builder()
                .extractImages(true)
                .extractMetadata(true)
                .build())

            // Configure image preprocessing for better OCR results
            .imagePreprocessing(ImagePreprocessingConfig.builder()
                .targetDpi(300)
                .denoise(true)
                .deskew(true)
                .contrastEnhance(true)
                .build())

            // Configure chunking for RAG workflows
            .chunking(ChunkingConfig.builder()
                .maxChars(1000)
                .maxOverlap(200)
                .enabled(true)
                .build())

            // Configure automatic language detection
            .languageDetection(LanguageDetectionConfig.builder()
                .enabled(true)
                .minConfidence(0.8)
                .build())

            .build();

        // Register custom post-processor for content transformation
        PostProcessor uppercaser = result -> new ExtractionResult(
            result.getContent().toUpperCase(),
            result.getMimeType(),
            result.getMetadata(),
            result.getTables(),
            result.getDetectedLanguages(),
            result.getChunks(),
            result.getImages(),
            result.isSuccess()
        );

        // Register custom validator for quality checks
        Validator minLength = result -> {
            if (result.getContent().length() < 100) {
                throw new ValidationException("Content too short");
            }
        };

        Kreuzberg.registerPostProcessor("uppercase", uppercaser, 100, ProcessingStage.EARLY);
        Kreuzberg.registerValidator("min-length", minLength);

        // Extract document with all configurations applied
        ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

        // Display extraction results
        System.out.println("Content: " + result.getContent().substring(0, 100));
        System.out.println("Tables: " + result.getTables().size());
        System.out.println("Images: " + result.getImages().size());
        System.out.println("Chunks: " + result.getChunks().size());
        System.out.println("Language: " + result.getLanguage());
        System.out.println("MIME: " + result.getMimeType());
    }
}
```

---

### Batch Processing with Error Handling

```java title="BatchProcessing.java"
import dev.kreuzberg.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class BatchProcessor {
    public static void main(String[] args) throws Exception {
        // Find all PDF files in the documents directory
        List<Path> files = Files.list(Path.of("documents/"))
            .filter(p -> p.toString().endsWith(".pdf"))
            .toList();

        // Convert Path objects to String paths
        List<String> filePaths = new ArrayList<>();
        for (Path file : files) {
            filePaths.add(file.toString());
        }

        // Configure extraction with caching enabled
        ExtractionConfig config = ExtractionConfig.builder()
            .useCache(true)
            .build();

        try {
            // Process all files in parallel
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(filePaths, config);

            // Check results for each file
            for (int i = 0; i < filePaths.size(); i++) {
                ExtractionResult result = results.get(i);
                Path file = files.get(i);

                if (result.isSuccess()) {
                    System.out.println(file + ": " + result.getContent().length() + " chars");
                } else {
                    System.err.println(file + ": extraction failed");
                }
            }
        } catch (KreuzbergException e) {
            System.err.println("Batch extraction failed: " + e.getMessage());
        }
    }
}
```

---

## Performance Tips

1. **Reuse configurations** - Create one `ExtractionConfig` and use it for multiple extractions
2. **Batch processing** - Use `batchExtractFiles()` for multiple files instead of individual calls
3. **Cache enabled** - Keep caching enabled for repeated document processing
4. **OCR selective** - Only enable OCR on pages that need it (`forceOcr = false`)
5. **Image preprocessing** - Enable image preprocessing for better OCR accuracy
6. **Async operations** - Use async methods for non-blocking extraction in concurrent scenarios

---

## Supported File Formats

- **Documents**: PDF, DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODP, ODS
- **Images**: PNG, JPG, JPEG, GIF, BMP, WebP, TIFF
- **Web**: HTML, MHTML
- **Text**: TXT, CSV

---

## Java FFM API Details

The Kreuzberg Java bindings use Java's Foreign Function & Memory (FFM) API for direct FFI without JNI overhead.

**Memory Management:**

```java title="FfmMemoryManagement.java"
// FFM API uses Arena for automatic memory management
try (Arena arena = Arena.ofConfined()) {
    // FFI operations use arena for memory management
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} // Arena automatically cleaned up when try block exits
```

**Arena Types:**

- `Arena.ofConfined()` - Thread-confined arena (recommended)
- `Arena.ofShared()` - Shared arena for multi-threaded access

---

## Troubleshooting

**"Failed to load native library"** - Ensure libkreuzberg_ffi is in system library path.

```bash title="Terminal"
export LD_LIBRARY_PATH=/path/to/libkreuzberg_ffi:$LD_LIBRARY_PATH  # Linux/Unix
export DYLD_LIBRARY_PATH=/path/to/libkreuzberg_ffi:$DYLD_LIBRARY_PATH  # macOS
set PATH=C:\path\to\libkreuzberg_ffi;%PATH%  # Windows
```

**"Tesseract not found"** - Install Tesseract OCR:

```bash title="Terminal"
# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# macOS
brew install tesseract

# Windows
# Download from https://github.com/UB-Mannheim/tesseract/wiki
```

**"OutOfMemoryError with large files"** - Use streaming or batch processing with smaller batches.

---

### LayoutDetectionConfig <span class="version-badge">v4.7.4</span>

Configuration for ONNX-based document layout detection.

**Builder Methods:**

```java title="LayoutDetectionConfig.java"
LayoutDetectionConfig config = LayoutDetectionConfig.builder()
    .applyHeuristics(true)             // Apply heuristic post-processing (default: true)
    .confidenceThreshold(0.5)          // Min confidence threshold (0.0-1.0)
    .tableModel("tatr")               // Table structure model: "tatr", "slanet_wired", etc.
    .build();
```

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. The Java binding passes LLM configuration through the FFI layer as JSON. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `StructuredExtractionConfig` to extract structured data from documents using an LLM:

```java title="StructuredExtraction.java"
import dev.kreuzberg.*;

var schema = Map.of(
    "type", "object",
    "properties", Map.of(
        "title", Map.of("type", "string"),
        "authors", Map.of("type", "array", "items", Map.of("type", "string")),
        "date", Map.of("type", "string")
    ),
    "required", List.of("title", "authors", "date"),
    "additionalProperties", false
);

var config = ExtractionConfig.builder()
    .structuredExtraction(StructuredExtractionConfig.builder()
        .schema(schema)
        .llm(LlmConfig.builder().model("openai/gpt-4o-mini").build())
        .strict(true)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFileSync("paper.pdf", config);

if (result.getStructuredOutput() != null) {
    System.out.println(result.getStructuredOutput());
}
```

### VLM OCR

Use a vision-language model as an OCR backend:

```java title="VlmOcr.java"
var config = ExtractionConfig.builder()
    .forceOcr(true)
    .ocr(OcrConfig.builder()
        .backend("vlm")
        .vlmConfig(LlmConfig.builder().model("openai/gpt-4o-mini").build())
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFileSync("scan.pdf", config);
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---
