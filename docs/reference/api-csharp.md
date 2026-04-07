# C# API Reference

Complete reference for the Kreuzberg .NET bindings using .NET 10.0 with P/Invoke interop.

## Installation

Add the NuGet package to your `.csproj`:

```xml title=".csproj"
<ItemGroup>
    <PackageReference Include="Kreuzberg" Version="4.7.4" />
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

### BatchExtractFilesWithConfigsAsync() <span class="version-badge">v4.5.0</span>

Asynchronously extracts multiple files with per-file configuration overrides.

**Signature:**

```csharp title="C#"
public static Task<IReadOnlyList<ExtractionResult>> BatchExtractFilesWithConfigsAsync(
    IReadOnlyList<FileWithConfig> items,
    ExtractionConfig? config = null)
```

---

### BatchExtractFilesWithConfigsSync() <span class="version-badge">v4.5.0</span>

Synchronous variant of `BatchExtractFilesWithConfigsAsync`.

**Signature:**

```csharp title="C#"
public static IReadOnlyList<ExtractionResult> BatchExtractFilesWithConfigsSync(
    IReadOnlyList<FileWithConfig> items,
    ExtractionConfig? config = null)
```

---

### BatchExtractBytesWithConfigsAsync() / BatchExtractBytesWithConfigsSync() <span class="version-badge">v4.5.0</span>

Batch extract byte arrays with per-file configuration overrides. Async and sync variants follow the same pattern.

---

### FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All properties are nullable — `null` means "use the batch-level default."

```csharp title="C#"
public class FileExtractionConfig
{
    public bool? EnableQualityProcessing { get; set; }
    public OcrConfig? Ocr { get; set; }
    public bool? ForceOcr { get; set; }
    public ChunkingConfig? Chunking { get; set; }
    public ImageExtractionConfig? Images { get; set; }
    public PdfConfig? PdfOptions { get; set; }
    public TokenReductionConfig? TokenReduction { get; set; }
    public LanguageDetectionConfig? LanguageDetection { get; set; }
    public PageConfig? Pages { get; set; }
    public PostProcessorConfig? Postprocessor { get; set; }
    public string? OutputFormat { get; set; }
    public string? ResultFormat { get; set; }
    public bool? IncludeDocumentStructure { get; set; }
}
```

Batch-level fields (`MaxConcurrentExtractions`, `UseCache`, `Acceleration`, `SecurityLimits`) cannot be overridden per file. See [Configuration Reference](configuration.md#fileextractionconfig) for details.

---

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

### EmbedSync()

Generate embeddings for a list of texts synchronously.

**Signature:**

```csharp
public IEnumerable<float[]> EmbedSync(IEnumerable<string> texts, EmbeddingConfig? config = null)
```

**Parameters:**

- `texts` (`IEnumerable<string>`): List of strings to embed.
- `config` (`EmbeddingConfig?`, optional): Embedding configuration.

**Returns:** `IEnumerable<float[]>` — one embedding vector per input text.

**Example:**

--8<-- "snippets/csharp/utils/standalone_embed.md"

---

### EmbedAsync()

Async variant of `EmbedSync()`.

**Signature:**

```csharp
public Task<IEnumerable<float[]>> EmbedAsync(IEnumerable<string> texts, EmbeddingConfig? config = null)
```

Same parameters as `EmbedSync()`, returns a `Task`.

---

## Type Reference

### ExtractionResult

The main result of document extraction containing extracted content, metadata, and structured data.

- `Annotations` (List<PdfAnnotation>?): PDF annotations extracted from the document, if any.
- `Chunks` (List<Chunk>?): Text chunks if chunking was enabled, each with metadata and optional embedding vector.
- `Content` (string): The extracted text content from the document.
- `DetectedLanguages` (List<string>?): Detected languages in the document, if language detection was enabled.
- `DjotContent` (DjotContent?): Rich Djot content structure when extracting Djot documents.
- `Document` (DocumentStructure?): Structured document representation with hierarchical node tree.
- `Elements` (List<Element>?): Semantic elements extracted from the document.
- `ExtractedKeywords` (List<ExtractedKeyword>?): Extracted keywords when keyword extraction is enabled.
- `Images` (List<ExtractedImage>?): Images extracted from the document, if image extraction was enabled.
- `Metadata` (Metadata): Document metadata including language, format-specific info, and other attributes.
- `MimeType` (string): The detected MIME type of the document (e.g., "application/pdf").
- `OcrElements` (List<OcrElement>?): OCR elements extracted from documents.
- `Pages` (List<PageContent>?): Per-page extracted content when page extraction is enabled.
- `ProcessingWarnings` (List<ProcessingWarning>?): Non-fatal warnings collected during processing.
- `QualityScore` (double?): Document quality score (0.0-1.0) from quality analysis.
- `Tables` (List<Table>): Extracted tables from the document, if any.

---

### ExtractionConfig

Configuration for document extraction, controlling extraction behavior and features.

**Properties:**

- `Chunking` (ChunkingConfig?): Text chunking configuration for splitting long documents. Includes `Sizing` property (ChunkSizingConfig?) to control how chunk size is measured -- by character count (default) or by token count using a HuggingFace tokenizer. ChunkSizingConfig has properties: `SizingType` (string: `"characters"` or `"tokenizer"`), `Model` (string: HuggingFace model ID, e.g. `"bert-base-uncased"`), and `CacheDir` (string?: optional tokenizer cache directory).
- `EnableQualityProcessing` (bool?): Whether to enable quality processing to improve extraction quality.
- `ForceOcr` (bool?): Whether to force OCR processing even for documents with text.
- `HtmlOptions` (HtmlConversionOptions?): HTML conversion options for HTML documents.
- `Images` (ImageExtractionConfig?): Image extraction configuration.
- `Keywords` (KeywordConfig?): Keyword extraction configuration.
- `LanguageDetection` (LanguageDetectionConfig?): Language detection configuration.
- `MaxConcurrentExtractions` (int?): Maximum number of concurrent extractions in batch operations.
- `Layout` (LayoutDetectionConfig?): Layout detection configuration for document page analysis. If null, layout detection is disabled.
- `Ocr` (OcrConfig?): OCR configuration for handling scanned documents and images. If null, OCR is disabled.
- `Pages` (PageConfig?): Page extraction and tracking configuration.
- `PdfOptions` (PdfConfig?): PDF-specific extraction options.
- `Concurrency` (ConcurrencyConfig?): Concurrency configuration for extraction parallelization.
- `Postprocessor` (PostProcessorConfig?): Post-processor configuration.
- `TokenReduction` (TokenReductionConfig?): Token reduction configuration.
- `UseCache` (bool?): Whether to use caching for extraction results. Default is null (use server default).

---

### OcrConfig

Configuration for OCR processing.

**Properties:**

- `Backend` (string?): The OCR backend to use (e.g., "tesseract").
- `Language` (string?): The language to recognize (e.g., "eng", "deu").
- `TesseractConfig` (TesseractConfig?): Tesseract-specific configuration options.

---

---

### BoundingBox

Bounding box coordinates for element positioning on a page.

**Properties:**

- `X0` (double): Left x-coordinate.
- `Y0` (double): Bottom y-coordinate.
- `X1` (double): Right x-coordinate.
- `Y1` (double): Top y-coordinate.

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

### Chunk

A chunk of text from a document, used for splitting large documents into smaller pieces.

**Properties:**

- `Content` (string): The text content of this chunk.
- `Embedding` (float[]?): Optional embedding vector for the chunk, if embedding was enabled.
- `Metadata` (ChunkMetadata): Metadata about the chunk including position and token count.

---

### ChunkMetadata

Metadata about a text chunk including position, token count, and page information.

**Properties:**

- `ByteEnd` (long): Ending byte position of this chunk in the document.
- `ByteStart` (long): Starting byte position of this chunk in the document.
- `ChunkIndex` (int): Zero-based index of this chunk among all chunks.
- `FirstPage` (int?): Page number (1-indexed) of the first page this chunk starts on.
- `HeadingContext` (HeadingContext?): Heading hierarchy for this chunk's section.
- `LastPage` (int?): Page number (1-indexed) of the last page this chunk ends on.
- `TokenCount` (int?): Token count for this chunk.
- `TotalChunks` (int): Total number of chunks the document was split into.

---

### DjotContent

Comprehensive Djot document structure with semantic preservation.

**Properties:**

- `Attributes` (List<List<object>>): Attributes mapped by element identifier.
- `Blocks` (List<FormattedBlock>): Structured block-level content.
- `Footnotes` (List<Footnote>): Footnote definitions.
- `Images` (List<DjotImage>): Extracted images with metadata.
- `Links` (List<DjotLink>): Extracted links with URLs.
- `Metadata` (Metadata): Metadata from YAML frontmatter.
- `PlainText` (string): Plain text representation.
- `Tables` (List<Table>): Extracted tables as structured data.

---

### DocumentNode

A single node in the document tree structure.

**Properties:**

- `Annotations` (List<DocumentTextAnnotation>): Inline annotations on this node's text content.
- `Bbox` (BoundingBox?): Bounding box in document coordinates, if available.
- `Children` (List<uint>): Child node indices in reading order.
- `Content` (NodeContent): Node content with type-specific data.
- `ContentLayer` (string): Content layer classification (Body, Header, Footer, Footnote).
- `Id` (string): Deterministic identifier generated from content hash and position.
- `Page` (uint?): Page number where this node starts (1-indexed).
- `PageEnd` (uint?): Page number where this node ends.
- `Parent` (uint?): Parent node index (null means this is a root-level node).

---

### DocumentStructure

Top-level structured document representation with a hierarchical node tree.

**Properties:**

- `Nodes` (List<DocumentNode>): All nodes in the document, stored in document/reading order.

---

### Element

A semantic element extracted from a document.

**Properties:**

- `ElementId` (string): Unique identifier for this element (deterministic hash-based ID).
- `ElementType` (ElementType): Semantic type classification (Title, NarrativeText, Heading, ListItem, Table, Image, etc.).
- `Metadata` (ElementMetadata): Metadata about this element.
- `Text` (string): Text content of this element.

---

### ElementMetadata

Metadata for a semantic element extracted from a document.

**Properties:**

- `Additional` (Dictionary<string, string>?): Additional custom metadata fields.
- `Coordinates` (BoundingBox?): Bounding box coordinates for this element.
- `ElementIndex` (int?): Position index of this element in the document sequence.
- `Filename` (string?): Source filename or document name.
- `PageNumber` (int?): Page number (1-indexed) where this element appears.

---

### EmbeddingPreset

Represents an embedding preset with configuration for text embedding generation.

**Properties:**

- `ChunkSize` (int): The recommended chunk size (in tokens or characters) for this embedding model.
- `Description` (string): Human-readable description of this embedding preset.
- `Dimensions` (int): The output dimensionality of the embedding vectors from this model.
- `ModelName` (string): The name of the embedding model (e.g., "text-embedding-ada-002").
- `Name` (string): The name/identifier of this embedding preset (e.g., "default", "openai").
- `Overlap` (int): The recommended overlap between chunks when chunking text for this model.

---

### ErrorMetadata

Metadata about an error that occurred during document extraction.

**Properties:**

- `ErrorType` (string): The type or category of the error.
- `Message` (string): Human-readable error message describing what went wrong.

---

### ExtractedImage

Represents an image extracted from a document with metadata and optional OCR results.

**Properties:**

- `Colorspace` (string?): Color space representation (e.g., "RGB", "CMYK", "DeviceGray").
- `Data` (byte[]): Raw image data as bytes (PNG, JPEG, etc.).
- `Format` (string): Image format (e.g., "PNG", "JPEG", "TIFF").
- `Height` (uint?): Image height in pixels.
- `ImageIndex` (int): Zero-based index of this image among all extracted images.
- `PageNumber` (int?): Page number (1-indexed) where this image appears.
- `Width` (uint?): Image width in pixels.

---

### ExtractedKeyword

Represents an extracted keyword from keyword extraction algorithms (YAKE, RAKE).

**Properties:**

- `Algorithm` (string): Algorithm that extracted this keyword (e.g., "yake", "rake").
- `Positions` (List<int>?): Optional positions where keyword appears in text (character offsets).
- `Score` (float): Relevance score (higher is better, algorithm-specific range).
- `Text` (string): The keyword text.

---

### FormatMetadata

Container for format-specific metadata based on the document type.

**Properties:**

- `Archive` (ArchiveMetadata?): Archive-specific metadata (if Type is Archive).
- `Email` (EmailMetadata?): Email-specific metadata (if Type is Email).
- `Excel` (ExcelMetadata?): Excel-specific metadata (if Type is Excel).
- `Image` (ImageMetadata?): Image-specific metadata (if Type is Image).
- `Pdf` (PdfMetadata?): PDF-specific metadata (if Type is Pdf).
- `Pptx` (PptxMetadata?): PowerPoint-specific metadata (if Type is Pptx).
- `Type` (FormatType): The detected document format type.

---

### ImagePreprocessingMetadata

Metadata about image preprocessing operations applied during extraction.

**Properties:**

- `AutoAdjusted` (bool): Whether the DPI was automatically adjusted.
- `CalculatedDpi` (int?): Calculated DPI from image metadata, if available.
- `DimensionClamped` (bool): Whether the image dimensions were clamped to a maximum.
- `FinalDpi` (int): Final DPI after preprocessing.
- `NewDimensions` (int[]?): New image dimensions [width, height] after preprocessing.
- `OriginalDimensions` (int[]?): Original image dimensions [width, height] in pixels.
- `OriginalDpi` (double[]?): Original image DPI [horizontal, vertical].
- `ResampleMethod` (string?): Resampling method used for resizing (e.g., "lanczos", "bilinear").
- `ResizeError` (string?): Error message if resizing failed, if any.
- `ScaleFactor` (double): Scale factor applied to the image.
- `SkippedResize` (bool): Whether resizing was skipped (e.g., image already at target size).
- `TargetDpi` (int): Target DPI used for preprocessing.

---

### IOcrBackend

Interface for custom OCR backends for document extraction.

**Properties:**

- `Name` (string): The unique name of this OCR backend (e.g., "tesseract", "custom_ocr").

**Methods:**

- `string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)`: Processes image bytes and returns OCR results as JSON.

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

### Metadata

Document-level metadata extracted during processing.

**Properties:**

- `AbstractText` (string?): Abstract or summary text (from frontmatter).
- `Authors` (List<string>?): Primary author(s).
- `Category` (string?): Document category (from frontmatter or classification).
- `CreatedAt` (string?): Creation timestamp (ISO 8601 format).
- `CreatedBy` (string?): User who created the document.
- `DocumentVersion` (string?): Document version string (from frontmatter).
- `Error` (ErrorMetadata?): Error information if extraction failed.
- `ExtractionDurationMs` (long?): Extraction duration in milliseconds.
- `Format` (FormatMetadata?): Format-specific metadata (varies by document type).
- `ImagePreprocessing` (ImagePreprocessingMetadata?): Image preprocessing information if OCR was used.
- `JsonSchema` (JsonNode?): JSON schema if document is structured as JSON.
- `Keywords` (List<string>?): Keywords/tags from document metadata.
- `Language` (string?): Detected or specified language of the document (e.g., "en", "de").
- `ModifiedAt` (string?): Last modification timestamp (ISO 8601 format).
- `ModifiedBy` (string?): User who last modified the document.
- `OutputFormat` (string?): Output format identifier.
- `Pages` (PageStructure?): Page structure and boundaries information for paginated documents.
- `Subject` (string?): Document subject or description.
- `Tags` (List<string>?): Document tags (from frontmatter).
- `Title` (string?): Document title (from metadata).

---

### OcrConfig

Configuration for OCR processing.

**Properties:**

- `Backend` (string?): The OCR backend to use (e.g., "tesseract", "paddle").
- `ElementConfig` (OcrElementConfig?): Configuration for OCR element extraction.
- `Language` (string?): The language to recognize (e.g., "eng", "deu").
- `PaddleOcrConfig` (PaddleOcrConfig?): PaddleOCR-specific configuration options (see below).
- `TesseractConfig` (TesseractConfig?): Tesseract-specific configuration options.

**PaddleOcrConfig Properties:** <span class="version-badge">v4.5.0</span>

- `ModelTier` (string?): Model tier: "mobile" (lightweight, ~21MB total, fast) or "server" (high accuracy, ~172MB, best with GPU). Default: "mobile"
- `Padding` (int?): Padding in pixels (0-100) added around the image before detection. Default: 10

---

### OcrElement

An OCR element extracted from a document containing recognized text and geometric information.

**Properties:**

- `BackendMetadata` (Dictionary<string, object>?): Backend-specific metadata.
- `Confidence` (OcrConfidence?): Confidence scores for text detection (0.0-1.0) and character recognition (0.0-1.0).
- `Geometry` (OcrBoundingGeometry?): Bounding geometry information (type, left, top, width, height, and polygon points).
- `Level` (string?): Hierarchical level of this element (word, line, block, page).
- `PageNumber` (int?): Page number (1-indexed) where this element appears.
- `ParentId` (string?): Parent element ID for hierarchical relationships.
- `Rotation` (OcrRotation?): Rotation information (angle, confidence) if the element is rotated.
- `Text` (string?): Recognized text content.

---

### ArchiveMetadata

Metadata specific to archive files (ZIP, TAR, etc.).

**Properties:**

- `CompressedSize` (long?): Total compressed size in bytes.
- `FileCount` (int): Number of files in the archive.
- `FileList` (List<string>): List of file paths within the archive.
- `Format` (string): Archive format name (e.g., "zip", "tar", "gz").
- `TotalSize` (long): Total uncompressed size in bytes.

---

### EmailMetadata

Metadata specific to email messages.

**Properties:**

- `Attachments` (List<string>?): List of attachment filenames in the email.
- `BccEmails` (List<string>): List of BCC recipient email addresses.
- `CcEmails` (List<string>): List of CC recipient email addresses.
- `FromEmail` (string?): Sender email address.
- `FromName` (string?): Sender display name.
- `MessageId` (string?): Unique message identifier from the email headers.
- `ToEmails` (List<string>): List of recipient email addresses.

---

### ExcelMetadata

Metadata specific to Excel spreadsheet documents.

**Properties:**

- `SheetCount` (int): Number of sheets in the workbook.
- `SheetNames` (List<string>): Names of the sheets in the workbook.

---

### HtmlMetadata

Metadata specific to HTML documents.

**Properties:**

- `Author` (string?): Author from the HTML meta tags.
- `BaseHref` (string?): Base href from the HTML base element.
- `CanonicalUrl` (string?): Canonical URL from the HTML link element.
- `Description` (string?): Meta description from the HTML document.
- `Headers` (List<HeaderMetadata>): Headers/headings found in the HTML document.
- `Images` (List<HtmlImageMetadata>): Images found in the HTML document.
- `Keywords` (List<string>): Meta keywords from the HTML document.
- `Language` (string?): Document language from the HTML lang attribute.
- `Links` (List<LinkMetadata>): Links found in the HTML document.
- `MetaTags` (Dictionary<string, string>): Additional meta tag key-value pairs.
- `OpenGraph` (Dictionary<string, string>): Open Graph metadata key-value pairs.
- `StructuredData` (List<StructuredData>): Structured data (JSON-LD, etc.) found in the HTML document.
- `TextDirection` (string?): Text direction (e.g., "ltr", "rtl").
- `Title` (string?): Document title from the HTML title element.
- `TwitterCard` (Dictionary<string, string>): Twitter Card metadata key-value pairs.

---

### ImageMetadata

Metadata specific to image files.

**Properties:**

- `Exif` (Dictionary<string, string>): EXIF metadata key-value pairs, if available.
- `Format` (string): Image format name (e.g., "PNG", "JPEG", "TIFF").
- `Height` (uint): Image height in pixels.
- `Width` (uint): Image width in pixels.

---

### OcrMetadata

Metadata specific to OCR-processed documents.

**Properties:**

- `Language` (string): Language used for OCR processing.
- `OutputFormat` (string): Output format of the OCR results.
- `Psm` (int): Page Segmentation Mode (PSM) used by the OCR engine.
- `TableCols` (int?): Number of columns in detected tables.
- `TableCount` (int): Number of tables detected by OCR.
- `TableRows` (int?): Number of rows in detected tables.

---

### PageBoundary

Represents character offset boundaries for a page in the extracted content.

**Properties:**

- `ByteEnd` (long): Ending byte offset for this page.
- `ByteStart` (long): Starting byte offset for this page.
- `PageNumber` (int): The page number (1-indexed).

---

### PageContent

Represents the extracted text and structured content for a specific page.

**Properties:**

- `Content` (string): The extracted text content for this page.
- `Hierarchy` (PageHierarchy?): Structural hierarchy information for the page.
- `Images` (List<ExtractedImage>): Images extracted from this page.
- `IsBlank` (bool): Whether the page is determined to be blank.
- `PageNumber` (int): The page number (1-indexed).
- `Tables` (List<Table>): Tables extracted from this page.

---

### PageInfo

Detailed metadata for an individual page in the document.

**Properties:**

- `Dimensions` (double[]?): Page dimensions [width, height] in points (PDF) or pixels (images).
- `Hidden` (bool?): Whether this page is marked as hidden (e.g., in presentations).
- `ImageCount` (int?): Number of images found on this page.
- `IsBlank` (bool?): Whether this page contains no meaningful content.
- `Number` (int): The page number (1-indexed).
- `TableCount` (int?): Number of tables found on this page.
- `Title` (string?): Page title (usually populated for presentations).

---

### PageStructure

Information about the global page structure of a document.

**Properties:**

- `Boundaries` (List<PageBoundary>?): Character offset boundaries for each page.
- `Pages` (List<PageInfo>?): Detailed per-page metadata.
- `TotalCount` (int): Total number of pages, slides, or sheets in the document.
- `UnitType` (string): The unit type of the pagination (e.g., "page", "slide", "sheet").

---

### PdfAnnotation

Represents an annotation (comment, highlight, etc.) extracted from a PDF document.

**Properties:**

- `AnnotationType` (string): The type of annotation (e.g., "text", "highlight", "link").
- `BoundingBox` (PdfAnnotationBoundingBox?): The bounding box coordinates of the annotation.
- `Content` (string?): The text content of the annotation.
- `PageNumber` (int): The page number (1-indexed) where the annotation appears.

---

### PdfMetadata

Metadata specific to PDF documents.

**Properties:**

- `Author` (string?): Document author from PDF metadata.
- `CreationDate` (string?): Document creation date.
- `Creator` (string?): Creator application.
- `Keywords` (List<string>?): Keywords from PDF metadata.
- `ModificationDate` (string?): Document modification date.
- `PageCount` (int?): Total number of pages in the PDF document.
- `Producer` (string?): PDF producer application.
- `Subject` (string?): Document subject.
- `Title` (string?): Document title.

---

### PptxMetadata

Metadata specific to PowerPoint presentation documents.

**Properties:**

- `SlideCount` (int): Total number of slides in the presentation.
- `SlideNames` (List<string>): Names of slides (if available).

---

### ProcessingWarning

A non-fatal warning from a processing pipeline stage.

**Properties:**

- `Message` (string): Human-readable description of the warning.
- `Source` (string): The pipeline stage that produced this warning.

---

### Table

Represents a table extracted from a document.

**Properties:**

- `Cells` (List<List<string>>): Table cells arranged as rows (outer list) and columns (inner lists).
- `Markdown` (string): Table representation in Markdown format.
- `PageNumber` (int): The page number (1-indexed) where this table appears in the document.

---

### TextMetadata

Metadata specific to plain text and Markdown documents.

**Properties:**

- `CharacterCount` (int): Total number of characters in the document.
- `CodeBlocks` (List<List<string>>?): Code blocks found in the text document, each as [language, code].
- `Headers` (List<string>?): Headers found in the text document.
- `LineCount` (int): Total number of lines in the document.
- `Links` (List<List<string>>?): Links found in the text document, each as [text, url].
- `WordCount` (int): Total number of words in the document.

---

### XmlMetadata

Metadata specific to XML documents.

**Properties:**

- `ElementCount` (int): Total number of XML elements in the document.
- `UniqueElements` (List<string>): List of unique XML element names found in the document.

---

---

## PDF Rendering

!!! info "Added in v4.6.2"

### KreuzbergClient.RenderPdfPage()

Render a single page of a PDF as a PNG image.

**Signature:**

```csharp title="C#"
public static byte[] RenderPdfPage(string path, int pageIndex, int dpi = 150)
```

**Parameters:**

- `path` (string): Path to the PDF file
- `pageIndex` (int): Zero-based page index to render
- `dpi` (int): Resolution for rendering (default 150)

**Returns:**

- `byte[]`: PNG-encoded bytes for the requested page

**Example:**

```csharp title="RenderSinglePage.cs"
byte[] png = KreuzbergClient.RenderPdfPage("document.pdf", 0);
File.WriteAllBytes("first_page.png", png);
```

---

### PdfPageIterator

A more memory-efficient alternative to rendering all pages at once when memory is a concern or when pages should be processed as they are rendered (e.g., sending each page to a vision model for OCR). Renders one page at a time, so only one raw image is in memory at a time.

**Signature:**

```csharp title="C#"
public class PdfPageIterator : IEnumerable<PageResult>, IDisposable
{
    public static PdfPageIterator Open(string path, int dpi = 150);
    public int PageCount { get; }
}

public record PageResult(int PageIndex, byte[] Data);
```

**Example:**

```csharp title="IteratePages.cs"
using var iter = PdfPageIterator.Open("document.pdf");
foreach (var page in iter)
{
    File.WriteAllBytes($"page_{page.PageIndex}.png", page.Data);
}
```

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

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. The C# binding passes LLM configuration through the FFI/P/Invoke layer as JSON. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `StructuredExtractionConfig` to extract structured data from documents using an LLM:

```csharp title="StructuredExtraction.cs"
using Kreuzberg;
using System.Text.Json;

var schema = new Dictionary<string, object>
{
    ["type"] = "object",
    ["properties"] = new Dictionary<string, object>
    {
        ["title"] = new Dictionary<string, string> { ["type"] = "string" },
        ["authors"] = new Dictionary<string, object>
        {
            ["type"] = "array",
            ["items"] = new Dictionary<string, string> { ["type"] = "string" }
        },
        ["date"] = new Dictionary<string, string> { ["type"] = "string" },
    },
    ["required"] = new[] { "title", "authors", "date" },
    ["additionalProperties"] = false,
};

var config = new ExtractionConfig
{
    StructuredExtraction = new StructuredExtractionConfig
    {
        Schema = schema,
        Llm = new LlmConfig { Model = "openai/gpt-4o-mini" },
        Strict = true,
    },
};

var result = KreuzbergClient.ExtractFileSync("paper.pdf", config: config);

if (result.StructuredOutput is not null)
{
    Console.WriteLine(result.StructuredOutput);
}
```

### VLM OCR

Use a vision-language model as an OCR backend:

```csharp title="VlmOcr.cs"
var config = new ExtractionConfig
{
    ForceOcr = true,
    Ocr = new OcrConfig
    {
        Backend = "vlm",
        VlmConfig = new LlmConfig { Model = "openai/gpt-4o-mini" },
    },
};

var result = KreuzbergClient.ExtractFileSync("scan.pdf", config: config);
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## See Also

- [Configuration Reference](configuration.md)
- [Data Types Reference](types.md)
- [Error Handling Guide](errors.md)
- [Supported Formats](formats.md)

---

### ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Configuration for concurrent extraction parallelization.

**Properties:**

- `MaxThreads` (int?): Maximum number of threads to use for concurrent extraction operations. If null, uses system default.

---

### PdfConfig

Configuration for PDF-specific extraction options.

**Properties:**

- `AllowSingleColumnTables` <span class="version-badge">v4.5.0</span> (bool?): Allow extraction of single-column tables. Default: false.
- `ExtractImages` (bool?): Extract images from the PDF. Default: false.
- `ExtractMetadata` (bool?): Extract PDF metadata. Default: true.
- `ExtractAnnotations` (bool?): Extract PDF annotations. Default: false.

---

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Configuration for ONNX-based document layout detection.

**Properties:**

- `ApplyHeuristics` (bool?): Whether to apply heuristic post-processing to refine layout regions. Default: true.
- `ConfidenceThreshold` (double?): Minimum confidence threshold for detected layout regions (0.0-1.0).
- `TableModel` (string?): Table structure recognition model. Options: `"tatr"` (default), `"slanet_wired"`, `"slanet_wireless"`, `"slanet_plus"`, `"slanet_auto"`. Default: null (uses `"tatr"`).

---
