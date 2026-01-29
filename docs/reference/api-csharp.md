# C# API Reference

Complete reference for the Kreuzberg .NET bindings using .NET 10.0 with P/Invoke interop.

## Installation

Add the NuGet package to your `.csproj`:

```xml title=".csproj"
<ItemGroup>
    <PackageReference Include="Kreuzberg" Version="4.2.4" />
</ItemGroup>
```

Or via the .NET CLI:

```bash title="Terminal"
dotnet add package Kreuzberg
```

**Requirements:**

- .NET 10.0 or later
- libkreuzberg_ffi native library (auto-loaded from NuGet)
- Optional: Tesseract or EasyOCR/PaddleOCR for OCR functionality

## Using the API

All APIs are exposed through the static `KreuzbergClient` class:

```csharp title="C#"
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
```

---

## Core Functions

### DetectMimeType()

Detects the MIME type of document bytes by examining file signatures.

**Signature:**

```csharp title="C#"
public static string DetectMimeType(ReadOnlySpan<byte> data)
```

**Parameters:**

- `data` (ReadOnlySpan<byte>): Document bytes to analyze. Must not be empty.

**Returns:**

- `string`: MIME type string (e.g., "application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")

**Throws:**

- `KreuzbergValidationException`: If data is empty
- `KreuzbergException`: If MIME detection fails

**Example:**

```csharp title="C#"
var data = File.ReadAllBytes("document");
var mimeType = KreuzbergClient.DetectMimeType(data);
Console.WriteLine(mimeType); // "application/pdf"
```

---

### DetectMimeTypeFromPath()

Detects the MIME type of a file from its path by reading file signatures.

**Signature:**

```csharp title="C#"
public static string DetectMimeTypeFromPath(string path)
```

**Parameters:**

- `path` (string): Absolute or relative path to the file to analyze. Must not be empty.

**Returns:**

- `string`: MIME type string (e.g., "application/pdf")

**Throws:**

- `KreuzbergValidationException`: If path is null, empty, or file not found
- `KreuzbergException`: If MIME detection fails

**Example:**

```csharp title="C#"
var mimeType = KreuzbergClient.DetectMimeTypeFromPath("/path/to/file.pdf");
Console.WriteLine(mimeType); // "application/pdf"
```

---

### GetExtensionsForMime()

Gets file extensions associated with a MIME type.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<string> GetExtensionsForMime(string mimeType)
```

**Parameters:**

- `mimeType` (string): MIME type string (e.g., "application/pdf"). Must not be empty.

**Returns:**

- `IReadOnlyList<string>`: List of file extensions (e.g., [".pdf"])

**Throws:**

- `KreuzbergValidationException`: If mimeType is null or empty
- `KreuzbergException`: If MIME type is not recognized

**Example:**

```csharp title="C#"
var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
Console.WriteLine(string.Join(", ", extensions)); // ".pdf"
```

---

### ExtractFileSync()

Extract content from a file synchronously.

**Signature:**

```csharp title="C#"
public static ExtractionResult ExtractFileSync(string path, ExtractionConfig? config = null)
```

**Parameters:**

- `path` (string): Absolute or relative path to the file to extract from. Must not be empty.
- `config` (ExtractionConfig?): Optional extraction configuration. Uses defaults if null.

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, tables, and detected languages

**Throws:**

- `ArgumentException`: If path is null or empty
- `KreuzbergValidationException`: If configuration is invalid
- `KreuzbergParsingException`: If document parsing fails
- `KreuzbergOcrException`: If OCR processing fails
- `KreuzbergException`: If extraction fails

**Example - Basic usage:**

```csharp title="C#"
var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
Console.WriteLine($"MIME Type: {result.MimeType}");
```

**Example - With configuration:**

```csharp title="C#"
var config = new ExtractionConfig
{
    EnableQualityProcessing = true,
    Ocr = new OcrConfig { Backend = "tesseract", Language = "eng" }
};
var result = KreuzbergClient.ExtractFileSync("scanned.pdf", config);
Console.WriteLine(result.Content);
```

---

### ExtractBytesSync()

Extract content from in-memory document bytes synchronously.

**Signature:**

```csharp title="C#"
public static ExtractionResult ExtractBytesSync(
    ReadOnlySpan<byte> data,
    string mimeType,
    ExtractionConfig? config = null)
```

**Parameters:**

- `data` (ReadOnlySpan<byte>): Document bytes to extract from. Must not be empty.
- `mimeType` (string): MIME type of the document (e.g., "application/pdf"). Must not be empty.
- `config` (ExtractionConfig?): Optional extraction configuration. Uses defaults if null.

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, tables, and detected languages

**Throws:**

- `KreuzbergValidationException`: If data is empty, mimeType is empty, or configuration is invalid
- `KreuzbergParsingException`: If document parsing fails
- `KreuzbergOcrException`: If OCR processing fails
- `KreuzbergException`: If extraction fails

**Example:**

```csharp title="C#"
var data = File.ReadAllBytes("document.pdf");
var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
Console.WriteLine(result.Content);
```

---

### BatchExtractFilesSync()

Extracts multiple files using the optimized batch pipeline synchronously.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<ExtractionResult> BatchExtractFilesSync(
    IReadOnlyList<string> paths,
    ExtractionConfig? config = null)
```

**Parameters:**

- `paths` (IReadOnlyList<string>): List of file paths to extract from. Must not be null or empty.
- `config` (ExtractionConfig?): Optional extraction configuration applied to all files. Uses defaults if null.

**Returns:**

- `IReadOnlyList<ExtractionResult>`: List of ExtractionResult objects, one per input file, in same order

**Throws:**

- `ArgumentNullException`: If paths is null
- `KreuzbergValidationException`: If any path is empty or configuration is invalid
- `KreuzbergException`: If batch extraction fails

**Example:**

```csharp title="C#"
var files = new[] { "doc1.pdf", "doc2.docx", "doc3.xlsx" };
var results = KreuzbergClient.BatchExtractFilesSync(files);
foreach (var result in results)
{
    Console.WriteLine($"Content: {result.Content.Length} characters");
}
```

---

### BatchExtractBytesSync()

Extracts multiple in-memory documents using the batch pipeline synchronously.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<ExtractionResult> BatchExtractBytesSync(
    IReadOnlyList<BytesWithMime> items,
    ExtractionConfig? config = null)
```

**Parameters:**

- `items` (IReadOnlyList<BytesWithMime>): List of byte data with MIME types. Must not be null or empty.
- `config` (ExtractionConfig?): Optional extraction configuration applied to all documents. Uses defaults if null.

**Returns:**

- `IReadOnlyList<ExtractionResult>`: List of ExtractionResult objects, one per input document, in same order

**Throws:**

- `ArgumentNullException`: If items is null
- `KreuzbergValidationException`: If any item is null, data is empty, MIME type is empty, or configuration is invalid
- `KreuzbergException`: If batch extraction fails

**Example:**

```csharp title="C#"
var items = new[]
{
    new BytesWithMime(File.ReadAllBytes("doc1.pdf"), "application/pdf"),
    new BytesWithMime(File.ReadAllBytes("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
};
var results = KreuzbergClient.BatchExtractBytesSync(items);
foreach (var result in results)
{
    Console.WriteLine($"MIME: {result.MimeType}");
}
```

---

### ExtractFileAsync()

Asynchronously extracts text, metadata, and structured information from a file.

**Signature:**

```csharp title="C#"
public static Task<ExtractionResult> ExtractFileAsync(
    string path,
    ExtractionConfig? config = null,
    CancellationToken cancellationToken = default)
```

**Parameters:**

- `path` (string): Absolute or relative path to the file to extract from. Must not be empty.
- `config` (ExtractionConfig?): Optional extraction configuration. Uses defaults if null.
- `cancellationToken` (CancellationToken): Cancellation token to cancel the operation.

**Returns:**

- `Task<ExtractionResult>`: Task that completes with ExtractionResult containing content, metadata, tables, and detected languages

**Throws:**

- `ArgumentException`: If path is null or empty
- `KreuzbergValidationException`: If configuration is invalid
- `KreuzbergParsingException`: If document parsing fails
- `KreuzbergOcrException`: If OCR processing fails
- `KreuzbergException`: If extraction fails
- `OperationCanceledException`: If cancellationToken is canceled

**Example:**

```csharp title="C#"
var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
Console.WriteLine(result.Content);
```

---

### ExtractBytesAsync()

Asynchronously extracts text, metadata, and structured information from in-memory document bytes.

**Signature:**

```csharp title="C#"
public static Task<ExtractionResult> ExtractBytesAsync(
    byte[] data,
    string mimeType,
    ExtractionConfig? config = null,
    CancellationToken cancellationToken = default)
```

**Parameters:**

- `data` (byte[]): Document bytes to extract from. Must not be null or empty.
- `mimeType` (string): MIME type of the document. Must not be empty.
- `config` (ExtractionConfig?): Optional extraction configuration. Uses defaults if null.
- `cancellationToken` (CancellationToken): Cancellation token to cancel the operation.

**Returns:**

- `Task<ExtractionResult>`: Task that completes with ExtractionResult

**Throws:**

- `KreuzbergValidationException`: If data is empty, mimeType is empty, or configuration is invalid
- `KreuzbergParsingException`: If document parsing fails
- `KreuzbergOcrException`: If OCR processing fails
- `KreuzbergException`: If extraction fails
- `OperationCanceledException`: If cancellationToken is canceled

**Example:**

```csharp title="C#"
var data = File.ReadAllBytes("document.pdf");
var result = await KreuzbergClient.ExtractBytesAsync(data, "application/pdf");
Console.WriteLine(result.Content);
```

---

### BatchExtractFilesAsync()

Asynchronously extracts multiple files using the optimized batch pipeline.

**Signature:**

```csharp title="C#"
public static Task<IReadOnlyList<ExtractionResult>> BatchExtractFilesAsync(
    IReadOnlyList<string> paths,
    ExtractionConfig? config = null,
    CancellationToken cancellationToken = default)
```

**Parameters:**

- `paths` (IReadOnlyList<string>): List of file paths to extract from. Must not be null or empty.
- `config` (ExtractionConfig?): Optional extraction configuration applied to all files. Uses defaults if null.
- `cancellationToken` (CancellationToken): Cancellation token to cancel the operation.

**Returns:**

- `Task<IReadOnlyList<ExtractionResult>>`: Task that completes with list of ExtractionResult objects, one per input file, in same order

**Throws:**

- `ArgumentNullException`: If paths is null
- `KreuzbergValidationException`: If any path is empty or configuration is invalid
- `KreuzbergException`: If batch extraction fails
- `OperationCanceledException`: If cancellationToken is canceled

**Example:**

```csharp title="C#"
var files = new[] { "doc1.pdf", "doc2.pdf", "doc3.pdf" };
var results = await KreuzbergClient.BatchExtractFilesAsync(files);
```

---

### BatchExtractBytesAsync()

Asynchronously extracts multiple in-memory documents using the batch pipeline.

**Signature:**

```csharp title="C#"
public static Task<IReadOnlyList<ExtractionResult>> BatchExtractBytesAsync(
    IReadOnlyList<BytesWithMime> items,
    ExtractionConfig? config = null,
    CancellationToken cancellationToken = default)
```

**Parameters:**

- `items` (IReadOnlyList<BytesWithMime>): List of byte data with MIME types. Must not be null or empty.
- `config` (ExtractionConfig?): Optional extraction configuration applied to all documents. Uses defaults if null.
- `cancellationToken` (CancellationToken): Cancellation token to cancel the operation.

**Returns:**

- `Task<IReadOnlyList<ExtractionResult>>`: Task that completes with list of ExtractionResult objects, one per input document, in same order

**Throws:**

- `ArgumentNullException`: If items is null
- `KreuzbergValidationException`: If any item is null, data is empty, MIME type is empty, or configuration is invalid
- `KreuzbergException`: If batch extraction fails
- `OperationCanceledException`: If cancellationToken is canceled

**Example:**

```csharp title="C#"
var items = new[]
{
    new BytesWithMime(File.ReadAllBytes("doc1.pdf"), "application/pdf"),
    new BytesWithMime(File.ReadAllBytes("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
};
var results = await KreuzbergClient.BatchExtractBytesAsync(items);
```

---

## Configuration

### DiscoverExtractionConfig()

Discovers extraction configuration by walking parent directories for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json.

**Signature:**

```csharp title="C#"
public static ExtractionConfig? DiscoverExtractionConfig()
```

**Returns:**

- `ExtractionConfig?`: ExtractionConfig if config file is found, null otherwise

**Throws:**

- `KreuzbergException`: If config file exists but is malformed

**Example:**

```csharp title="C#"
var config = KreuzbergClient.DiscoverExtractionConfig();
if (config != null)
{
    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
}
```

---

### LoadExtractionConfigFromFile()

Loads an extraction configuration from a TOML, YAML, or JSON file.

**Signature:**

```csharp title="C#"
public static ExtractionConfig LoadExtractionConfigFromFile(string path)
```

**Parameters:**

- `path` (string): Path to configuration file (must be .toml, .yaml, .yml, or .json). Must not be empty.

**Returns:**

- `ExtractionConfig`: ExtractionConfig deserialized from file

**Throws:**

- `KreuzbergValidationException`: If path is null, empty, or file not found
- `KreuzbergException`: If configuration file is malformed or cannot be parsed

**Example:**

```csharp title="C#"
var config = KreuzbergClient.LoadExtractionConfigFromFile("kreuzberg.toml");
var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
```

---

### GetVersion()

Returns the version string of the native Kreuzberg library.

**Signature:**

```csharp title="C#"
public static string GetVersion()
```

**Returns:**

- `string`: Version string in format "4.0.0" or similar

**Example:**

```csharp title="C#"
var version = KreuzbergClient.GetVersion();
Console.WriteLine($"Kreuzberg {version}");
```

---

## Plugin System

### Post-Processors

Custom post-processors can modify extraction results after extraction completes.

#### RegisterPostProcessor()

Registers a custom post-processor to process extraction results.

**Signature:**

```csharp title="C#"
public static void RegisterPostProcessor(IPostProcessor processor)
```

**Parameters:**

- `processor` (IPostProcessor): Implementation of IPostProcessor. Must not be null.

**Throws:**

- `ArgumentNullException`: If processor is null
- `KreuzbergException`: If registration fails

**Remarks:**

Post-processors are called after extraction completes and can modify the ExtractionResult. Multiple processors can be registered and will be called in priority order (higher priority first). This method is thread-safe and can be called from multiple threads concurrently.

**Example:**

```csharp title="C#"
public class UppercaseProcessor : IPostProcessor
{
    public string Name => "uppercase";
    public int Priority => 10;

    public ExtractionResult Process(ExtractionResult result)
    {
        result.Content = result.Content.ToUpper();
        return result;
    }
}

KreuzbergClient.RegisterPostProcessor(new UppercaseProcessor());
```

---

#### ListPostProcessors()

Lists the names of all registered post-processors.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<string> ListPostProcessors()
```

**Returns:**

- `IReadOnlyList<string>`: List of post-processor names

**Example:**

```csharp title="C#"
var processors = KreuzbergClient.ListPostProcessors();
Console.WriteLine($"Registered processors: {string.Join(", ", processors)}");
```

---

#### UnregisterPostProcessor()

Unregisters a previously registered post-processor by name.

**Signature:**

```csharp title="C#"
public static void UnregisterPostProcessor(string name)
```

**Parameters:**

- `name` (string): Name of the post-processor to unregister. Must not be empty.

**Throws:**

- `ArgumentException`: If name is null or empty
- `KreuzbergException`: If unregistration fails

---

#### ClearPostProcessors()

Unregisters and clears all registered post-processors.

**Signature:**

```csharp title="C#"
public static void ClearPostProcessors()
```

**Throws:**

- `KreuzbergException`: If clearing fails

---

### Validators

Custom validators can validate extraction results and reject invalid results.

#### RegisterValidator()

Registers a custom validator to validate extraction results.

**Signature:**

```csharp title="C#"
public static void RegisterValidator(IValidator validator)
```

**Parameters:**

- `validator` (IValidator): Implementation of IValidator. Must not be null.

**Throws:**

- `ArgumentNullException`: If validator is null
- `KreuzbergException`: If registration fails

**Remarks:**

Validators are called after extraction completes and can throw exceptions to reject invalid results. Multiple validators can be registered and will be called in priority order (higher priority first). This method is thread-safe and can be called from multiple threads concurrently.

**Example:**

```csharp title="C#"
public class MinimumContentValidator : IValidator
{
    public string Name => "min_content";
    public int Priority => 10;

    public void Validate(ExtractionResult result)
    {
        if (result.Content.Length < 100)
        {
            throw new InvalidOperationException("Content too short");
        }
    }
}

KreuzbergClient.RegisterValidator(new MinimumContentValidator());
```

---

#### ListValidators()

Lists the names of all registered validators.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<string> ListValidators()
```

**Returns:**

- `IReadOnlyList<string>`: List of validator names

---

#### UnregisterValidator()

Unregisters a previously registered validator by name.

**Signature:**

```csharp title="C#"
public static void UnregisterValidator(string name)
```

**Parameters:**

- `name` (string): Name of the validator to unregister. Must not be empty.

**Throws:**

- `ArgumentException`: If name is null or empty
- `KreuzbergException`: If unregistration fails

---

#### ClearValidators()

Unregisters and clears all registered validators.

**Signature:**

```csharp title="C#"
public static void ClearValidators()
```

**Throws:**

- `KreuzbergException`: If clearing fails

**Remarks:**

This method removes all registered validators from the extraction pipeline. After clearing, no validators will be called during extraction.

---

### OCR Backends

Custom OCR backends can be registered to process images.

#### RegisterOcrBackend()

Registers a custom OCR backend implemented in C#.

**Signature:**

```csharp title="C#"
public static void RegisterOcrBackend(IOcrBackend backend)
```

**Parameters:**

- `backend` (IOcrBackend): Implementation of IOcrBackend. Must not be null.

**Throws:**

- `ArgumentNullException`: If backend is null
- `KreuzbergException`: If registration fails

**Example:**

```csharp title="C#"
public class CustomOcr : IOcrBackend
{
    public string Name => "custom";

    public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
    {
        // Process image and return JSON result
        return "{\"text\": \"extracted text\"}";
    }
}

KreuzbergClient.RegisterOcrBackend(new CustomOcr());
```

---

#### ListOcrBackends()

Lists the names of all registered OCR backends.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<string> ListOcrBackends()
```

**Returns:**

- `IReadOnlyList<string>`: List of OCR backend names

**Example:**

```csharp title="C#"
var backends = KreuzbergClient.ListOcrBackends();
Console.WriteLine($"Available OCR backends: {string.Join(", ", backends)}");
```

---

#### UnregisterOcrBackend()

Unregisters a previously registered OCR backend by name.

**Signature:**

```csharp title="C#"
public static void UnregisterOcrBackend(string name)
```

**Parameters:**

- `name` (string): Name of the OCR backend to unregister. Must not be empty.

**Throws:**

- `ArgumentException`: If name is null or empty
- `KreuzbergException`: If unregistration fails

---

#### ClearOcrBackends()

Unregisters and clears all registered OCR backends.

**Signature:**

```csharp title="C#"
public static void ClearOcrBackends()
```

**Throws:**

- `KreuzbergException`: If clearing fails

**Remarks:**

This method removes all registered OCR backends from the extraction pipeline. After clearing, the default OCR backend (if any) will be used during extraction.

---

### Document Extractors

#### ListDocumentExtractors()

Lists the names of all registered document extractors.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<string> ListDocumentExtractors()
```

**Returns:**

- `IReadOnlyList<string>`: List of document extractor names

**Example:**

```csharp title="C#"
var extractors = KreuzbergClient.ListDocumentExtractors();
Console.WriteLine($"Available extractors: {string.Join(", ", extractors)}");
```

---

#### UnregisterDocumentExtractor()

Unregisters a previously registered document extractor by name.

**Signature:**

```csharp title="C#"
public static void UnregisterDocumentExtractor(string name)
```

**Parameters:**

- `name` (string): Name of the document extractor to unregister. Must not be empty.

**Throws:**

- `ArgumentException`: If name is null or empty
- `KreuzbergException`: If unregistration fails

---

#### ClearDocumentExtractors()

Unregisters and clears all registered document extractors.

**Signature:**

```csharp title="C#"
public static void ClearDocumentExtractors()
```

**Throws:**

- `KreuzbergException`: If clearing fails

**Remarks:**

This method removes all registered document extractors from the extraction pipeline. After clearing, only built-in extractors will be available.

---

## Embeddings

### ListEmbeddingPresets()

Lists the names of all available embedding presets.

**Signature:**

```csharp title="C#"
public static List<string> ListEmbeddingPresets()
```

**Returns:**

- `List<string>`: List of embedding preset names (e.g., ["default", "openai", "sentence-transformers"])

**Throws:**

- `KreuzbergException`: If preset enumeration fails

**Example:**

```csharp title="C#"
var presets = KreuzbergClient.ListEmbeddingPresets();
Console.WriteLine($"Available: {string.Join(", ", presets)}");
```

---

### GetEmbeddingPreset()

Gets an embedding preset by name.

**Signature:**

```csharp title="C#"
public static EmbeddingPreset? GetEmbeddingPreset(string name)
```

**Parameters:**

- `name` (string): The name of the embedding preset (e.g., "default", "openai"). Must not be empty.

**Returns:**

- `EmbeddingPreset?`: The EmbeddingPreset with matching name, or null if not found

**Throws:**

- `KreuzbergValidationException`: If name is null or empty
- `KreuzbergException`: If preset retrieval fails

**Example:**

```csharp title="C#"
var preset = KreuzbergClient.GetEmbeddingPreset("default");
if (preset != null)
{
    Console.WriteLine($"Model: {preset.ModelName}");
    Console.WriteLine($"Dimensions: {preset.Dimensions}");
    Console.WriteLine($"Chunk Size: {preset.ChunkSize}");
}
```

---

## Type Reference

### ExtractionResult

The main result of document extraction containing extracted content, metadata, and structured data.

**Properties:**

- `Content` (string): The extracted text content from the document.
- `MimeType` (string): The detected MIME type of the document (e.g., "application/pdf").
- `Metadata` (Metadata): Document metadata including language, format-specific info, and other attributes.
- `Tables` (List<Table>): Extracted tables from the document, if any.
- `DetectedLanguages` (List<string>?): Detected languages in the document, if language detection was enabled.
- `Chunks` (List<Chunk>?): Text chunks if chunking was enabled, each with metadata and optional embedding vector.
- `Images` (List<ExtractedImage>?): Images extracted from the document, if image extraction was enabled.
- `Pages` (List<PageContent>?): Per-page extracted content when page extraction is enabled.
- `Success` (bool): Indicates whether extraction completed successfully.

---

### ExtractionConfig

Configuration for document extraction, controlling extraction behavior and features.

**Properties:**

- `UseCache` (bool?): Whether to use caching for extraction results. Default is null (use server default).
- `EnableQualityProcessing` (bool?): Whether to enable quality processing to improve extraction quality.
- `Ocr` (OcrConfig?): OCR configuration for handling scanned documents and images. If null, OCR is disabled.
- `ForceOcr` (bool?): Whether to force OCR processing even for documents with text.
- `Chunking` (ChunkingConfig?): Text chunking configuration for splitting long documents.
- `Images` (ImageExtractionConfig?): Image extraction configuration.
- `PdfOptions` (PdfConfig?): PDF-specific extraction options.
- `TokenReduction` (TokenReductionConfig?): Token reduction configuration.
- `LanguageDetection` (LanguageDetectionConfig?): Language detection configuration.
- `Postprocessor` (PostProcessorConfig?): Post-processor configuration.
- `HtmlOptions` (HtmlConversionOptions?): HTML conversion options for HTML documents.
- `Keywords` (KeywordConfig?): Keyword extraction configuration.
- `Pages` (PageConfig?): Page extraction and tracking configuration.
- `MaxConcurrentExtractions` (int?): Maximum number of concurrent extractions in batch operations.

---

### OcrConfig

Configuration for OCR processing.

**Properties:**

- `Backend` (string?): The OCR backend to use (e.g., "tesseract").
- `Language` (string?): The language to recognize (e.g., "eng", "deu").
- `TesseractConfig` (TesseractConfig?): Tesseract-specific configuration options.

---

### Table

Represents a table extracted from a document.

**Properties:**

- `Cells` (List<List<string>>): Table cells arranged as rows (outer list) and columns (inner lists).
- `Markdown` (string): Table representation in Markdown format.
- `PageNumber` (int): The page number (1-indexed) where this table appears in the document.

---

### Chunk

A chunk of text from a document, used for splitting large documents into smaller pieces.

**Properties:**

- `Content` (string): The text content of this chunk.
- `Embedding` (float[]?): Optional embedding vector for the chunk, if embedding was enabled.
- `Metadata` (ChunkMetadata): Metadata about the chunk including position and token count.

---

### ChunkMetadata

Metadata about a chunk.

**Properties:**

- `ByteStart` (long): Starting byte offset in the document.
- `ByteEnd` (long): Ending byte offset in the document.
- `TokenCount` (int?): Token count for the chunk.
- `ChunkIndex` (int): Zero-based index of this chunk.
- `TotalChunks` (int): Total number of chunks in the document.
- `FirstPage` (int?): First page number (1-indexed) containing this chunk.
- `LastPage` (int?): Last page number (1-indexed) containing this chunk.

---

### Metadata

Document metadata including language, format-specific info, and other attributes.

**Properties:**

- `Language` (string?): Detected language of the document (e.g., "en", "de").
- `Date` (string?): Document date if available.
- `Subject` (string?): Document subject if available.
- `FormatType` (FormatType): The format type of the document (PDF, Excel, Email, etc.).
- `Format` (FormatMetadata): Format-specific metadata (varies by document type).
- `ImagePreprocessing` (ImagePreprocessingMetadata?): Image preprocessing information if OCR was used.
- `JsonSchema` (JsonNode?): JSON schema if document is structured as JSON.
- `Error` (ErrorMetadata?): Error information if extraction failed.
- `Pages` (PageStructure?): Page structure and boundaries information.

---

### BytesWithMime

Represents a document as bytes with its MIME type, used for batch extraction from in-memory data.

**Properties:**

- `Data` (byte[]): The document bytes.
- `MimeType` (string): The MIME type of the document (e.g., "application/pdf").

**Constructor:**

```csharp title="C#"
public BytesWithMime(byte[] data, string mimeType)
```

---

### IPostProcessor

Interface for custom post-processors that can modify extraction results.

**Properties:**

- `Name` (string): The unique name of this post-processor.
- `Priority` (int): The priority order (higher values run first).

**Methods:**

- `ExtractionResult Process(ExtractionResult result)`: Processes an extraction result, potentially modifying it.

---

### IValidator

Interface for custom validators that can validate extraction results.

**Properties:**

- `Name` (string): The unique name of this validator.
- `Priority` (int): The priority order (higher values run first).

**Methods:**

- `void Validate(ExtractionResult result)`: Validates an extraction result. Should throw an exception if validation fails.

---

### IOcrBackend

Interface for custom OCR backends for document extraction.

**Properties:**

- `Name` (string): The unique name of this OCR backend (e.g., "tesseract", "custom_ocr").

**Methods:**

- `string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)`: Processes image bytes and returns OCR results as JSON.

---

### EmbeddingPreset

Represents an embedding preset with configuration for text embedding generation.

**Properties:**

- `Name` (string): The name/identifier of this embedding preset (e.g., "default", "openai").
- `ChunkSize` (int): The recommended chunk size (in tokens or characters) for this embedding model.
- `Overlap` (int): The recommended overlap between chunks when chunking text for this model.
- `ModelName` (string): The name of the embedding model (e.g., "text-embedding-ada-002").
- `Dimensions` (int): The output dimensionality of the embedding vectors from this model.
- `Description` (string): Human-readable description of this embedding preset.

---

## Error Handling

All errors thrown by Kreuzberg inherit from `KreuzbergException` or its subclasses. Always wrap extraction calls in try-catch blocks.

**Exception Hierarchy:**

- `KreuzbergException` (base)
  - `KreuzbergValidationException`: Configuration or input validation failed
  - `KreuzbergParsingException`: Document parsing failed
  - `KreuzbergOcrException`: OCR processing failed
  - `KreuzbergCacheException`: Cache operation failed
  - `KreuzbergImageProcessingException`: Image preprocessing failed
  - `KreuzbergSerializationException`: JSON serialization/deserialization failed
  - `KreuzbergMissingDependencyException`: Required system dependency not found
  - `KreuzbergPluginException`: Plugin operation failed
  - `KreuzbergUnsupportedFormatException`: Document format not supported
  - `KreuzbergIOException`: File I/O error
  - `KreuzbergRuntimeException`: Runtime error

**Example - Error Handling:**

```csharp title="C#"
try
{
    var result = KreuzbergClient.ExtractFileSync("document.pdf");
    Console.WriteLine(result.Content);
}
catch (KreuzbergValidationException ex)
{
    Console.WriteLine($"Invalid configuration: {ex.Message}");
}
catch (KreuzbergParsingException ex)
{
    Console.WriteLine($"Failed to parse document: {ex.Message}");
}
catch (KreuzbergOcrException ex)
{
    Console.WriteLine($"OCR processing failed: {ex.Message}");
}
catch (KreuzbergException ex)
{
    Console.WriteLine($"Extraction failed: {ex.Message}");
}
```

---

## Platform Compatibility

### Supported Platforms

- **Windows**: x86_64, ARM64
- **macOS**: Intel (x86_64), Apple Silicon (ARM64)
- **Linux**: x86_64, ARM64

### Native Library Loading

The C# bindings use P/Invoke to load the native `libkreuzberg_ffi` library. The library is automatically included in the NuGet package and loaded at runtime.

**Linux library search paths:**
- `/usr/lib/libkreuzberg_ffi.so`
- `/usr/local/lib/libkreuzberg_ffi.so`
- Directories in `LD_LIBRARY_PATH`

**macOS library search paths:**
- `/usr/local/lib/libkreuzberg_ffi.dylib`
- `/opt/homebrew/lib/libkreuzberg_ffi.dylib`
- Directories in `DYLD_LIBRARY_PATH`

**Windows library search paths:**
- `kreuzberg_ffi.dll` in the application directory
- Directories in `PATH`

---

## See Also

- [Configuration Reference](configuration.md)
- [Data Types Reference](types.md)
- [Error Handling Guide](errors.md)
- [Supported Formats](formats.md)
