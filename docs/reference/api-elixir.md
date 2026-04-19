# Elixir API Reference <span class="version-badge">v4.0.0</span>

Complete reference for the Kreuzberg Elixir API.

## Installation

Add to your `mix.exs`:

```elixir title="mix.exs"
def deps do
  [
    {:kreuzberg, "~> 4.9.1"}
  ]
end
```

Or install from Git:

```elixir title="mix.exs"
def deps do
  [
    {:kreuzberg, git: "https://github.com/kreuzberg-dev/kreuzberg.git"}
  ]
end
```

## Core Functions

### Kreuzberg.extract/3

Extract content from binary document data.

**Signature:**

```elixir title="Elixir"
@spec extract(
  binary(),
  String.t(),
  ExtractionConfig.t() | map() | keyword() | nil
) :: {:ok, ExtractionResult.t()} | {:error, String.t()}
```

**Parameters:**

- `input` (binary): Binary document data to extract from
- `mime_type` (String): MIME type of the document (for example, "application/pdf", "text/plain")
- `config` (ExtractionConfig | map | keyword | nil): Extraction configuration. Uses defaults if nil

**Returns:**

- `{:ok, ExtractionResult.t()}`: Successfully extracted content with metadata
- `{:error, reason}`: Extraction failed with error message

**Example - Basic usage:**

```elixir title="basic_extraction.exs"
{:ok, pdf_binary} = File.read("document.pdf")
{:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf")
IO.puts(result.content)
IO.puts("Pages: #{result.metadata["page_count"]}")
```

**Example - With configuration struct:**

```elixir title="with_config.exs"
config = %Kreuzberg.ExtractionConfig{
  ocr: %{"enabled" => true, "language" => "eng"}
}
{:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf", config)
```

**Example - With keyword list configuration:**

```elixir title="keyword_config.exs"
{:ok, result} = Kreuzberg.extract(
  pdf_binary,
  "application/pdf",
  ocr: %{"enabled" => true, "language" => "eng"}
)
```

---

### Kreuzberg.extract!/3

Extract content from binary data, raising on error.

**Signature:**

```elixir title="Elixir"
@spec extract!(
  binary(),
  String.t(),
  ExtractionConfig.t() | map() | keyword() | nil
) :: ExtractionResult.t()
```

**Parameters:**

Same as [`extract/3`](#kreuzbergextract3).

**Returns:**

- `ExtractionResult.t()`: Successfully extracted content

**Raises:**

- `Kreuzberg.Error`: If extraction fails

**Example:**

```elixir title="basic_extraction.exs"
result = Kreuzberg.extract!(pdf_binary, "application/pdf")
IO.puts(result.content)
```

---

### Kreuzberg.extract_file/3

Extract content from a file at the given path.

**Signature:**

```elixir title="Elixir"
@spec extract_file(
  String.t() | Path.t(),
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: {:ok, ExtractionResult.t()} | {:error, String.t()}
```

**Parameters:**

- `path` (String | Path): File path to extract
- `mime_type` (String | nil): Optional MIME type hint. If nil, MIME type is auto-detected
- `config` (ExtractionConfig | map | keyword | nil): Extraction configuration

**Returns:**

- `{:ok, ExtractionResult.t()}`: Successfully extracted content
- `{:error, reason}`: Extraction failed with error message

**Example - Basic usage:**

```elixir title="basic_extraction.exs"
{:ok, result} = Kreuzberg.extract_file("document.pdf")
IO.puts(result.content)
```

**Example - With explicit MIME type:**

```elixir title="with_mime.exs"
{:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf")
```

**Example - With configuration:**

```elixir title="with_config.exs"
config = %Kreuzberg.ExtractionConfig{
  ocr: %{"language" => "eng"}
}
{:ok, result} = Kreuzberg.extract_file("scanned.pdf", nil, config)
```

---

### Kreuzberg.extract_file!/3

Extract content from a file, raising on error.

**Signature:**

```elixir title="Elixir"
@spec extract_file!(
  String.t() | Path.t(),
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: ExtractionResult.t()
```

**Parameters:**

Same as [`extract_file/3`](#kreuzbergextract_file3).

**Returns:**

- `ExtractionResult.t()`: Successfully extracted content

**Raises:**

- `Kreuzberg.Error`: If extraction fails

**Example:**

```elixir title="basic_extraction.exs"
result = Kreuzberg.extract_file!("document.pdf")
IO.puts("Content: #{String.length(result.content)} characters")
```

---

## Batch Operations

### Kreuzberg.batch_extract_files/3

Extract content from multiple files in a batch operation.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_files(
  [String.t() | Path.t()],
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}
```

**Parameters:**

- `paths` (list): List of file paths to extract
- `mime_type` (String | nil): MIME type for all files (optional, defaults to nil for auto-detection)
- `config` (ExtractionConfig | map | keyword | nil): Extraction configuration applied to all files

**Returns:**

- `{:ok, results}`: List of ExtractionResult structs
- `{:error, reason}`: Error message if batch extraction fails

**Example:**

```elixir title="batch_extraction.exs"
paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
{:ok, results} = Kreuzberg.batch_extract_files(paths)

Enum.each(results, fn result ->
  IO.puts("Content: #{String.length(result.content)} characters")
end)
```

**Example - With configuration:**

```elixir title="batch_with_config.exs"
config = %Kreuzberg.ExtractionConfig{
  images: %{"enabled" => true}
}
{:ok, results} = Kreuzberg.batch_extract_files(paths, "application/pdf", config)
```

---

### Kreuzberg.batch_extract_files!/3

Extract content from multiple files, raising on error.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_files!(
  [String.t() | Path.t()],
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: [ExtractionResult.t()]
```

**Parameters:**

Same as [`batch_extract_files/3`](#kreuzbergbatch_extract_files3).

**Returns:**

- List of ExtractionResult structs

**Raises:**

- `Kreuzberg.Error`: If batch extraction fails

---

### Kreuzberg.batch_extract_bytes/3

Extract content from multiple binary inputs in a batch operation.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_bytes(
  [binary()],
  String.t() | [String.t()],
  ExtractionConfig.t() | map() | keyword() | nil
) :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}
```

**Parameters:**

- `data_list` (list): List of binary data inputs
- `mime_types` (String | list): List of MIME types (one per input) or single MIME type for all
- `config` (ExtractionConfig | map | keyword | nil): Extraction configuration applied to all items

**Returns:**

- `{:ok, results}`: List of ExtractionResult structs
- `{:error, reason}`: Error message if batch extraction fails

**Example - Single MIME type:**

```elixir title="batch_bytes.exs"
data_list = [pdf_binary1, pdf_binary2, pdf_binary3]
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, "application/pdf")
```

**Example - Multiple MIME types:**

```elixir title="batch_mixed.exs"
data_list = [pdf_binary, docx_binary, txt_binary]
mime_types = [
  "application/pdf",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "text/plain"
]
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, mime_types)
```

---

### Kreuzberg.batch_extract_bytes!/3

Extract content from multiple binary inputs, raising on error.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_bytes!(
  [binary()],
  String.t() | [String.t()],
  ExtractionConfig.t() | map() | keyword() | nil
) :: [ExtractionResult.t()]
```

**Parameters:**

Same as [`batch_extract_bytes/3`](#kreuzbergbatch_extract_bytes3).

**Returns:**

- List of ExtractionResult structs

**Raises:**

- `Kreuzberg.Error`: If batch extraction fails

---

## Async Operations

### Kreuzberg.extract_async/3

Extract content from binary data asynchronously using Tasks.

**Signature:**

```elixir title="Elixir"
@spec extract_async(
  binary(),
  String.t(),
  ExtractionConfig.t() | map() | keyword() | nil
) :: Task.t({:ok, ExtractionResult.t()} | {:error, String.t()})
```

**Parameters:**

- `input` (binary): Binary data to extract from
- `mime_type` (String): MIME type of the data
- `config` (ExtractionConfig | map | keyword | nil): Optional extraction configuration

**Returns:**

- A Task that will resolve to `{:ok, ExtractionResult.t()}` or `{:error, String.t()}`

**Example:**

```elixir title="async_extraction.exs"
task = Kreuzberg.extract_async(pdf_binary, "application/pdf")
{:ok, result} = Task.await(task)
IO.puts(result.content)
```

**Example - Multiple concurrent extractions:**

```elixir title="concurrent.exs"
tasks = [
  Kreuzberg.extract_async(pdf1, "application/pdf"),
  Kreuzberg.extract_async(pdf2, "application/pdf"),
  Kreuzberg.extract_async(pdf3, "application/pdf")
]

results = Task.await_many(tasks)
```

---

### Kreuzberg.extract_file_async/3

Extract content from a file asynchronously using Tasks.

**Signature:**

```elixir title="Elixir"
@spec extract_file_async(
  String.t() | Path.t(),
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: Task.t({:ok, ExtractionResult.t()} | {:error, String.t()})
```

**Parameters:**

- `path` (String | Path): File path as String or Path.t()
- `mime_type` (String | nil): Optional MIME type (defaults to nil for auto-detection)
- `config` (ExtractionConfig | map | keyword | nil): Optional extraction configuration

**Returns:**

- A Task that will resolve to `{:ok, ExtractionResult.t()}` or `{:error, String.t()}`

**Example:**

```elixir title="async_file.exs"
task = Kreuzberg.extract_file_async("document.pdf")
{:ok, result} = Task.await(task)
```

**Example - Process multiple files concurrently:**

```elixir title="concurrent_files.exs"
tasks = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
  |> Enum.map(&Kreuzberg.extract_file_async/1)

results = Task.await_many(tasks)
```

---

### Kreuzberg.batch_extract_files_async/3

Batch extract content from multiple files asynchronously.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_files_async(
  [String.t() | Path.t()],
  String.t() | nil,
  ExtractionConfig.t() | map() | keyword() | nil
) :: Task.t({:ok, [ExtractionResult.t()]} | {:error, String.t()})
```

**Parameters:**

- `paths` (list): List of file paths
- `mime_type` (String | nil): Optional MIME type for all files
- `config` (ExtractionConfig | map | keyword | nil): Optional extraction configuration

**Returns:**

- A Task that will resolve to `{:ok, [ExtractionResult.t()]}` or `{:error, String.t()}`

**Example:**

```elixir title="batch_async.exs"
paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
task = Kreuzberg.batch_extract_files_async(paths)
{:ok, results} = Task.await(task)
```

---

### Kreuzberg.batch_extract_bytes_async/3

Batch extract content from multiple binary inputs asynchronously.

**Signature:**

```elixir title="Elixir"
@spec batch_extract_bytes_async(
  [binary()],
  String.t() | [String.t()],
  ExtractionConfig.t() | map() | keyword() | nil
) :: Task.t({:ok, [ExtractionResult.t()]} | {:error, String.t()})
```

**Parameters:**

- `data_list` (list): List of binary data inputs
- `mime_types` (String | list): Single MIME type string or list of MIME types
- `config` (ExtractionConfig | map | keyword | nil): Optional extraction configuration

**Returns:**

- A Task that will resolve to `{:ok, [ExtractionResult.t()]}` or `{:error, String.t()}`

**Example:**

```elixir title="batch_bytes_async.exs"
data_list = [pdf1_binary, pdf2_binary, pdf3_binary]
task = Kreuzberg.batch_extract_bytes_async(data_list, "application/pdf")
{:ok, results} = Task.await(task)
```

---

## Cache Management

### Kreuzberg.cache_stats/0

Retrieve statistics about the extraction cache.

**Signature:**

```elixir title="Elixir"
@spec cache_stats() :: {:ok, map()} | {:error, String.t()}
```

**Returns:**

- `{:ok, stats}`: Map with cache statistics
  - `"total_files"` - Number of cached extraction results
  - `"total_size_mb"` - Total size of cache in megabytes
  - `"available_space_mb"` - Available disk space in megabytes
  - `"oldest_file_age_days"` - Age of oldest cached file in days
  - `"newest_file_age_days"` - Age of newest cached file in days
- `{:error, reason}`: Error message if retrieval fails

**Example:**

```elixir title="cache_stats.exs"
{:ok, stats} = Kreuzberg.cache_stats()
IO.puts("Cache entries: #{stats["total_files"]}")
IO.puts("Cache size: #{stats["total_size_mb"]} MB")
```

---

### Kreuzberg.cache_stats!/0

Retrieve cache statistics, raising on error.

**Signature:**

```elixir title="Elixir"
@spec cache_stats!() :: map()
```

**Returns:**

- Map with cache statistics

**Raises:**

- `Kreuzberg.Error`: If cache statistics retrieval fails

**Example:**

```elixir title="cache_stats.exs"
stats = Kreuzberg.cache_stats!()
IO.puts("Total cached files: #{stats["total_files"]}")
```

---

### Kreuzberg.clear_cache/0

Clear the extraction cache, removing all cached results.

**Signature:**

```elixir title="Elixir"
@spec clear_cache() :: :ok | {:error, String.t()}
```

**Returns:**

- `:ok`: Cache cleared successfully
- `{:error, reason}`: Error message if clearing fails

**Example:**

```elixir title="clear_cache.exs"
:ok = Kreuzberg.clear_cache()
{:ok, stats} = Kreuzberg.cache_stats()
IO.puts("Files after clear: #{stats["total_files"]}")
```

---

### Kreuzberg.clear_cache!/0

Clear the cache, raising on error.

**Signature:**

```elixir title="Elixir"
@spec clear_cache!() :: :ok
```

**Raises:**

- `Kreuzberg.Error`: If cache clearing fails

**Example:**

```elixir title="clear_cache.exs"
Kreuzberg.clear_cache!()
```

---

## Utility Functions

### Kreuzberg.detect_mime_type/1

Detect the MIME type of binary data using content inspection.

**Signature:**

```elixir title="Elixir"
@spec detect_mime_type(binary()) :: {:ok, String.t()} | {:error, String.t()}
```

**Parameters:**

- `data` (binary): Binary data to analyze

**Returns:**

- `{:ok, mime_type}`: Detected MIME type as a string
- `{:error, reason}`: Error if detection fails

**Example:**

```elixir title="mime_detection.exs"
{:ok, pdf_binary} = File.read("document.pdf")
{:ok, mime_type} = Kreuzberg.detect_mime_type(pdf_binary)
IO.puts("Detected MIME type: #{mime_type}")
# => "application/pdf"
```

---

### Kreuzberg.detect_mime_type_from_path/1

Detect the MIME type of a file using its path and extension.

**Signature:**

```elixir title="Elixir"
@spec detect_mime_type_from_path(String.t() | Path.t()) ::
  {:ok, String.t()} | {:error, String.t()}
```

**Parameters:**

- `path` (String | Path): File path to analyze

**Returns:**

- `{:ok, mime_type}`: Detected MIME type as a string
- `{:error, reason}`: Error if detection fails

**Example:**

```elixir title="mime_from_path.exs"
{:ok, mime_type} = Kreuzberg.detect_mime_type_from_path("document.pdf")
IO.puts("MIME type: #{mime_type}")
# => "application/pdf"
```

---

### Kreuzberg.validate_mime_type/1

Validate that a MIME type string is supported by Kreuzberg.

**Signature:**

```elixir title="Elixir"
@spec validate_mime_type(String.t()) :: {:ok, String.t()} | {:error, String.t()}
```

**Parameters:**

- `mime_type` (String): MIME type string to validate

**Returns:**

- `{:ok, mime_type}`: Returns the MIME type if valid
- `{:error, reason}`: Error if MIME type is not supported

**Example:**

```elixir title="validate_mime.exs"
{:ok, _} = Kreuzberg.validate_mime_type("application/pdf")
{:error, reason} = Kreuzberg.validate_mime_type("application/invalid")
```

---

### Kreuzberg.get_extensions_for_mime/1

Get all file extensions associated with a given MIME type.

**Signature:**

```elixir title="Elixir"
@spec get_extensions_for_mime(String.t()) :: {:ok, [String.t()]} | {:error, String.t()}
```

**Parameters:**

- `mime_type` (String): MIME type string

**Returns:**

- `{:ok, extensions}`: List of file extensions (without dot)
- `{:error, reason}`: Error if MIME type is not found

**Example:**

```elixir title="extensions.exs"
{:ok, exts} = Kreuzberg.get_extensions_for_mime("application/pdf")
IO.inspect(exts)
# => ["pdf"]

{:ok, exts} = Kreuzberg.get_extensions_for_mime("image/jpeg")
IO.inspect(exts)
# => ["jpg", "jpeg"]
```

---

### Kreuzberg.list_embedding_presets/0

List all available embedding model presets.

**Signature:**

```elixir title="Elixir"
@spec list_embedding_presets() :: {:ok, [String.t()]} | {:error, String.t()}
```

**Returns:**

- `{:ok, presets}`: List of preset names as strings
- `{:error, reason}`: Error if retrieval fails

**Example:**

```elixir title="presets.exs"
{:ok, presets} = Kreuzberg.list_embedding_presets()
IO.inspect(presets)
# => ["balanced", "fast", "quality", "multilingual"]
```

---

### Kreuzberg.get_embedding_preset/1

Get detailed information about a specific embedding preset.

**Signature:**

```elixir title="Elixir"
@spec get_embedding_preset(String.t()) :: {:ok, map()} | {:error, String.t()}
```

**Parameters:**

- `preset_name` (String): Name of the embedding preset

**Returns:**

- `{:ok, preset_info}`: Map containing preset details
  - `"name"` - Preset name
  - `"chunk_size"` - Chunk size in tokens
  - `"overlap"` - Chunk overlap in tokens
  - `"dimensions"` - Embedding vector dimension
  - `"description"` - Human-readable description
- `{:error, reason}`: Error if preset not found

**Example:**

```elixir title="preset_info.exs"
{:ok, preset} = Kreuzberg.get_embedding_preset("fast")
IO.puts("Preset: #{preset["name"]}")
IO.puts("Dimensions: #{preset["dimensions"]}")
```

---

### Kreuzberg.classify_error/1

Classify an error message into a semantic error category.

**Signature:**

```elixir title="Elixir"
@spec classify_error(String.t()) :: atom()
```

**Parameters:**

- `error_message` (String): Error message string to classify

**Returns:**

- Error category atom:
  - `:io_error` - File I/O related errors
  - `:invalid_format` - File format errors
  - `:invalid_config` - Configuration or parameter errors
  - `:ocr_error` - OCR engine or processing errors
  - `:extraction_error` - General extraction failures
  - `:unknown_error` - Errors that don't match other categories

**Example:**

```elixir title="error_classification.exs"
atom = Kreuzberg.classify_error("File not found: /path/to/file.pdf")
IO.inspect(atom)
# => :io_error

atom = Kreuzberg.classify_error("Invalid PDF format")
IO.inspect(atom)
# => :invalid_format
```

---

## Configuration

!!! Warning "Deprecated API"
    The `force_ocr` parameter has been deprecated in favor of the new `ocr` configuration object.

    **Old pattern (no longer supported):**
    ```elixir
    config = %Kreuzberg.ExtractionConfig{force_ocr: true}
    ```

    **New pattern:**
    ```elixir
    config = %Kreuzberg.ExtractionConfig{
      ocr: %Kreuzberg.OcrConfig{backend: "tesseract"}
    }
    ```

    The new approach provides more granular control over OCR behavior through the OcrConfig struct.

### ExtractionConfig

Main configuration struct for extraction operations.

**Type:**

```elixir title="Elixir"
@type t :: %Kreuzberg.ExtractionConfig{
  chunking: map() | nil,
  concurrency: map() | nil,
  enable_quality_processing: boolean(),
  force_ocr: boolean(),
  html_options: map() | nil,
  images: map() | nil,
  include_document_structure: boolean(),
  keywords: map() | nil,
  language_detection: map() | nil,
  layout: map() | nil,
  max_concurrent_extractions: non_neg_integer() | nil,
  ocr: map() | nil,
  output_format: String.t(),
  pages: map() | nil,
  pdf_options: map() | nil,
  postprocessor: map() | nil,
  result_format: String.t(),
  security_limits: map() | nil,
  token_reduction: map() | nil,
  use_cache: boolean(),
}
```

**Fields:**

- `chunking` (map | nil): Text chunking configuration.
- `concurrency` (map | nil): Concurrency configuration for extraction parallelization.
- `enable_quality_processing` (boolean): Enable quality post-processing (default: true).
- `force_ocr` (boolean): Force OCR even for searchable PDFs (default: false).
- `html_options` (map | nil): HTML to Markdown conversion options.
- `images` (map | nil): Image extraction configuration.
- `include_document_structure` (boolean): Include hierarchical document structure in results (default: false).
- `keywords` (map | nil): Keyword extraction configuration.
- `language_detection` (map | nil): Language detection settings.
- `layout` (map | nil): Layout detection configuration.
- `max_concurrent_extractions` (non_neg_integer | nil): Maximum concurrent extractions in batch operations.
- `ocr` (map | nil): OCR configuration.
- `output_format` (String): Content text format — `"plain"`, `"markdown"`, `"djot"`, `"html"` (default: `"plain"`).
- `pages` (map | nil): Page-level extraction configuration.
- `pdf_options` (map | nil): PDF-specific options.
- `postprocessor` (map | nil): Post-processor configuration.
- `result_format` (String): Result structure format — `"unified"`, `"element_based"` (default: `"unified"`).
- `security_limits` (map | nil): Security limits for extraction.
- `token_reduction` (map | nil): Token reduction settings.
- `use_cache` (boolean): Enable result caching (default: true).

**Example - Basic configuration:**

```elixir title="basic_config.exs"
config = %Kreuzberg.ExtractionConfig{
  use_cache: true,
  ocr: %Kreuzberg.OcrConfig{backend: "tesseract"}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)
```

**Example - OCR configuration:**

```elixir title="ocr_config.exs"
config = %Kreuzberg.ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "language" => "eng",
    "backend" => "tesseract"
  }
}

{:ok, result} = Kreuzberg.extract_file("scanned.pdf", nil, config)
```

**PaddleOCR-specific fields:** <span class="version-badge">v4.5.0</span>

When using PaddleOCR, the `ocr` map supports:

- `"model_tier"` (String): Model tier: "mobile" (lightweight, ~21MB total, fast) or "server" (high accuracy, ~172MB, best with GPU). Default: "mobile"
- `"padding"` (Integer): Padding in pixels (0-100) added around the image before detection. Default: 10

```elixir title="paddle_ocr_config.exs"
config = %Kreuzberg.ExtractionConfig{
  ocr: %{
    "backend" => "paddle-ocr",
    "language" => "en",
    "model_tier" => "server",
    "padding" => 10
  }
}
```

**Example - Chunking configuration:**

```elixir title="chunking_config.exs"
config = %Kreuzberg.ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunk_size" => 1024,
    "chunk_overlap" => 200,
    "chunking_strategy" => "semantic"
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)
```

**Example - Page extraction:**

```elixir title="page_config.exs"
config = %Kreuzberg.ExtractionConfig{
  pages: %{
    "extract_pages" => true,
    "insert_page_markers" => true,
    "marker_format" => "\n\n--- Page {page_num} ---\n\n"
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

if result.pages do
  Enum.each(result.pages, fn page ->
    IO.puts("Page #{page["page_number"]}: #{String.length(page["content"])} chars")
  end)
end
```

**Example - Image extraction:**

```elixir title="image_config.exs"
config = %Kreuzberg.ExtractionConfig{
  images: %{
    "enabled" => true,
    "min_width" => 100,
    "min_height" => 100,
    "format" => "png"
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

if result.images do
  IO.puts("Extracted #{length(result.images)} images")
end
```

**Example - PDF options with concurrency:**

```elixir title="pdf_config.exs"
config = %Kreuzberg.ExtractionConfig{
  pdf_options: %{
    "extract_images" => true,
    "extract_annotations" => true,
    "allow_single_column_tables" => true
  },
  concurrency: %{
    "max_threads" => 4
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)
```

**PDF Options Fields:**

When configuring `pdf_options` map:

- `"allow_single_column_tables"` (Boolean): <span class="version-badge">v4.5.0</span> Allow extraction of single-column tables. Default: false
- `"extract_annotations"` (Boolean): Extract PDF annotations. Default: false
- `"extract_images"` (Boolean): Extract images from PDF. Default: false
- `"extract_metadata"` (Boolean): Extract PDF metadata. Default: true
- `"passwords"` (List<String>): Passwords to try for encrypted PDFs. Default: nil

**Concurrency Configuration:** <span class="version-badge">v4.5.0</span>

When configuring `concurrency` map:

- `"max_threads"` (Integer): Maximum number of threads for parallel extraction. Default: nil (system default)

---

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Configure layout detection for document structure analysis.

**Fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `"confidence_threshold"` | Float\|nil | nil | Minimum confidence score (0.0-1.0) for layout detection results. If nil, no filtering applied |
| `"apply_heuristics"` | Boolean | true | Apply post-processing heuristics to refine layout results |
| `"table_model"` | String\|nil | nil | Table structure recognition model: `"tatr"` (default), `"slanet_wired"`, `"slanet_wireless"`, `"slanet_plus"`, `"slanet_auto"` |

**Example:**

```elixir title="layout_detection.exs"
config = %Kreuzberg.ExtractionConfig{
  layout: %{
    "confidence_threshold" => 0.5,
    "apply_heuristics" => true
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

if result.document do
  IO.puts("Document structure detected")
  IO.puts("Sections: #{length(result.document["sections"])}")
end
```

---

## Results & Types

### ExtractionResult

Result struct returned by all extraction functions.

**Type:**

```elixir title="Elixir"
@type t :: %Kreuzberg.ExtractionResult{
  annotations: [Kreuzberg.DocumentTextAnnotation.t()] | nil,
  chunks: [Kreuzberg.Chunk.t()] | nil,
  content: String.t(),
  detected_languages: [String.t()] | nil,
  djot_content: Kreuzberg.DjotContent.t() | nil,
  document: Kreuzberg.DocumentStructure.t() | nil,
  elements: [Kreuzberg.Element.t()] | nil,
  extracted_keywords: [Kreuzberg.Keyword.t()] | nil,
  images: [Kreuzberg.Image.t()] | nil,
  metadata: Kreuzberg.Metadata.t(),
  mime_type: String.t(),
  ocr_elements: [Kreuzberg.OcrElement.t()] | nil,
  pages: [Kreuzberg.Page.t()] | nil,
  processing_warnings: [Kreuzberg.ProcessingWarning.t()],
  quality_score: float() | nil,
  tables: [Kreuzberg.Table.t()],
}
```

**Fields:**

- `annotations` (list | nil): PDF annotations (text notes, highlights, links, stamps) as `Kreuzberg.DocumentTextAnnotation` structs.
- `chunks` (list | nil): List of text chunks with embeddings (`Kreuzberg.Chunk`) when chunking is enabled.
- `content` (String): Extracted text content.
- `detected_languages` (list | nil): List of detected language codes (ISO 639-1) if language detection is enabled.
- `djot_content` (Kreuzberg.DjotContent | nil): Rich Djot content structure.
- `document` (Kreuzberg.DocumentStructure | nil): Hierarchical document structure when `include_document_structure` is true.
- `elements` (list | nil): Semantic elements (`Kreuzberg.Element`) when `result_format` is `"element_based"`.
- `extracted_keywords` (list | nil): Extracted keywords with scores (`Kreuzberg.Keyword`).
- `images` (list | nil): List of extracted images (`Kreuzberg.Image`).
- `metadata` (Kreuzberg.Metadata): Document metadata.
- `mime_type` (String): MIME type of the processed document.
- `ocr_elements` (list | nil): OCR elements (`Kreuzberg.OcrElement`) with bounding geometry and confidence.
- `pages` (list | nil): Per-page extracted content (`Kreuzberg.Page`).
- `processing_warnings` (list): Non-fatal warnings (`Kreuzberg.ProcessingWarning`) from processing pipeline stages.
- `quality_score` (float | nil): Document quality score between 0.0 and 1.0.
- `tables` (list): List of extracted tables (`Kreuzberg.Table`).

**Example - Basic result access:**

```elixir title="result_access.exs"
{:ok, result} = Kreuzberg.extract_file("document.pdf")

IO.puts("Content: #{result.content}")
IO.puts("MIME type: #{result.mime_type}")
IO.puts("Page count: #{result.metadata["page_count"]}")
IO.puts("Tables: #{length(result.tables)}")

if result.detected_languages do
  IO.puts("Languages: #{Enum.join(result.detected_languages, ", ")}")
end
```

**Example - Processing tables:**

```elixir title="tables.exs"
{:ok, result} = Kreuzberg.extract_file("invoice.pdf")

Enum.each(result.tables, fn table ->
  IO.puts("Table on page #{table["page_number"]}:")
  IO.puts(table["markdown"])
  IO.puts("")
end)
```

**Example - Page iteration:**

```elixir title="pages.exs"
config = %Kreuzberg.ExtractionConfig{
  pages: %{"extract_pages" => true}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

if result.pages do
  Enum.each(result.pages, fn page ->
    IO.puts("Page #{page["page_number"]}:")
    IO.puts("  Content: #{String.length(page["content"])} chars")
    IO.puts("  Tables: #{length(page["tables"])}")
    IO.puts("  Images: #{length(page["images"])}")
  end)
end
```

---

### Metadata

Document metadata dictionary. Fields vary by document format.

**Common Fields:**

- `title` (String | nil): Document title.
- `subject` (String | nil): Document subject or description.
- `authors` (list(String) | nil): List of author names.
- `keywords` (list(String) | nil): List of keywords.
- `language` (String | nil): Primary language (ISO 639-1 code).
- `created_at` (String | nil): Creation date (ISO 8601).
- `modified_at` (String | nil): Last modification date (ISO 8601).
- `created_by` (String | nil): Application that created the document.
- `modified_by` (String | nil): Application that last modified the document.
- `pages` (Kreuzberg.PageStructure | nil): Page structure information.
- `format` (map | nil): Format-specific metadata (flattened fields).
- `image_preprocessing` (Kreuzberg.ImagePreprocessingMetadata | nil): Image preprocessing metadata.
- `json_schema` (map | nil): JSON schema if applicable.
- `error` (Kreuzberg.ErrorMetadata | nil): Error metadata if extraction partially failed.
- `category` (String | nil): Document category classification.
- `tags` (list(String) | nil): List of document tags.
- `document_version` (String | nil): Version of the document.
- `abstract_text` (String | nil): Abstract or summary of the document.
- `output_format` (String | nil): Output format used for extraction.
- `extraction_duration_ms` (non_neg_integer | nil): Time taken for extraction in milliseconds.
- `additional` (map): Additional metadata fields.

**Example:**

```elixir title="metadata.exs"
{:ok, result} = Kreuzberg.extract_file("document.pdf")
metadata = result.metadata

if metadata["format_type"] == "pdf" do
  IO.puts("Title: #{metadata["title"]}")
  IO.puts("Author: #{metadata["author"]}")
  IO.puts("Pages: #{metadata["page_count"]}")
end
```

---

### Table

Extracted table structure.

**Fields:**

- `cells` (list(list(String))): 2D array of table cells (rows x columns).
- `markdown` (String): Table rendered as markdown.
- `page_number` (non_neg_integer): Page number where table was found (0-indexed).
- `bounding_box` (Kreuzberg.BoundingBox | nil): Bounding box coordinates if available.

**Example:**

```elixir title="tables.exs"
{:ok, result} = Kreuzberg.extract_file("invoice.pdf")

Enum.each(result.tables, fn table ->
  IO.puts("Table on page #{table.page_number}:")
  IO.puts(table.markdown)

  # Access raw cell data
  Enum.each(table.cells, fn row ->
    IO.inspect(row)
  end)
end)
```

---

## Plugin System

### Kreuzberg.extract_with_plugins/4

Extract content with plugin processing support.

**Signature:**

```elixir title="Elixir"
@spec extract_with_plugins(
  binary(),
  String.t(),
  ExtractionConfig.t() | map() | keyword() | nil,
  keyword()
) :: {:ok, ExtractionResult.t()} | {:error, String.t()}
```

**Parameters:**

- `input` (binary): Binary document data to extract from
- `mime_type` (String): MIME type of the document
- `config` (ExtractionConfig | map | keyword | nil): Extraction configuration (optional)
- `plugin_opts` (keyword): Plugin options (optional):
  - `:validators` - List of validator modules to run before extraction
  - `:post_processors` - Map of stage atoms to lists of post-processor modules
    - `:early` - Applied first to extraction result
    - `:middle` - Applied after early processors
    - `:late` - Applied last before final validators
  - `:final_validators` - List of validator modules to run after post-processing

**Returns:**

- `{:ok, ExtractionResult.t()}`: Successfully extracted and processed content
- `{:error, reason}`: Extraction or processing failed

**Example - With validators and post-processors:**

```elixir title="with_plugins.exs"
{:ok, result} = Kreuzberg.extract_with_plugins(
  pdf_binary,
  "application/pdf",
  nil,
  validators: [MyApp.InputValidator],
  post_processors: %{
    early: [MyApp.EarlyProcessor],
    middle: [MyApp.MiddleProcessor],
    late: [MyApp.FinalProcessor]
  },
  final_validators: [MyApp.ResultValidator]
)
```

**Example - With only post-processors:**

```elixir title="post_processors.exs"
{:ok, result} = Kreuzberg.extract_with_plugins(
  pdf_binary,
  "application/pdf",
  %{use_cache: true},
  post_processors: %{
    early: [MyApp.Processor1, MyApp.Processor2]
  }
)
```

---

### Kreuzberg.Plugin.register_post_processor/2

Register a custom post-processor plugin.

**Signature:**

```elixir title="Elixir"
@spec register_post_processor(atom(), module()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `name` (atom): Unique identifier for the post-processor
- `module` (module): Module implementing the post-processor interface

**Module Interface:**

The post-processor module should implement:

- `process(data)` - Applies custom processing to extraction result data

**Example:**

```elixir title="post_processor.exs"
defmodule MyApp.TextNormalizer do
  def process(result) do
    normalized_content = result.content
      |> String.trim()
      |> String.downcase()

    {:ok, %{result | content: normalized_content}}
  end
end

:ok = Kreuzberg.Plugin.register_post_processor(:normalizer, MyApp.TextNormalizer)
```

---

### Kreuzberg.Plugin.register_validator/1

Register a custom validator plugin.

**Signature:**

```elixir title="Elixir"
@spec register_validator(module()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `module` (module): Module implementing the validator interface

**Module Interface:**

The validator module should implement:

- `validate(data)` - Validates data and returns `:ok` or `{:error, reason}`

**Example:**

```elixir title="validator.exs"
defmodule MyApp.StrictValidator do
  def validate(data) do
    if data && data != "" do
      :ok
    else
      {:error, "Data is empty"}
    end
  end
end

:ok = Kreuzberg.Plugin.register_validator(MyApp.StrictValidator)
```

---

### Kreuzberg.Plugin.register_ocr_backend/1

Register a custom OCR backend plugin.

**Signature:**

```elixir title="Elixir"
@spec register_ocr_backend(module()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `module` (module): Module implementing the OCR backend interface

**Module Interface:**

The OCR backend module should implement:

- `recognize(image_data, language)` - Performs OCR on image data
- `supported_languages()` - Returns list of supported language codes

**Example:**

```elixir title="ocr_backend.exs"
defmodule MyApp.CustomOCRBackend do
  def recognize(image_data, language) do
    # Custom OCR logic
    {:ok, "Extracted text"}
  end

  def supported_languages do
    ["en", "de", "fr"]
  end
end

:ok = Kreuzberg.Plugin.register_ocr_backend(MyApp.CustomOCRBackend)
```

---

## Validation

### Kreuzberg.validate_chunking_params/1

Validate chunking configuration parameters.

**Signature:**

```elixir title="Elixir"
@spec validate_chunking_params(map()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `params` (map): Map with keys:
  - `"max_chars"` or `:max_chars` - Maximum characters per chunk (required)
  - `"max_overlap"` or `:max_overlap` - Overlap between chunks (required)

**Returns:**

- `:ok`: Parameters are valid
- `{:error, reason}`: Parameters are invalid

**Example:**

```elixir title="validate_chunking.exs"
:ok = Kreuzberg.validate_chunking_params(%{
  "max_chars" => 1000,
  "max_overlap" => 200
})
```

---

### Kreuzberg.validate_language_code/1

Validate an ISO 639 language code.

**Signature:**

```elixir title="Elixir"
@spec validate_language_code(String.t()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `code` (String): Language code string (for example, "en", "eng", "de", "deu")

**Returns:**

- `:ok`: Language code is valid
- `{:error, reason}`: Language code is invalid

**Example:**

```elixir title="validate_language.exs"
:ok = Kreuzberg.validate_language_code("en")
:ok = Kreuzberg.validate_language_code("eng")
{:error, _} = Kreuzberg.validate_language_code("invalid")
```

---

### Kreuzberg.validate_dpi/1

Validate a DPI (dots per inch) value.

**Signature:**

```elixir title="Elixir"
@spec validate_dpi(integer()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `dpi` (integer): Positive integer representing DPI

**Returns:**

- `:ok`: DPI value is valid
- `{:error, reason}`: DPI value is invalid

**Example:**

```elixir title="validate_dpi.exs"
:ok = Kreuzberg.validate_dpi(300)
{:error, _} = Kreuzberg.validate_dpi(0)
```

---

### Kreuzberg.validate_confidence/1

Validate a confidence threshold value.

**Signature:**

```elixir title="Elixir"
@spec validate_confidence(float()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `confidence` (float): Confidence threshold between 0.0 and 1.0

**Returns:**

- `:ok`: Confidence value is valid
- `{:error, reason}`: Confidence value is invalid

**Example:**

```elixir title="validate_confidence.exs"
:ok = Kreuzberg.validate_confidence(0.5)
{:error, _} = Kreuzberg.validate_confidence(1.5)
```

---

### Kreuzberg.validate_ocr_backend/1

Validate an OCR backend name.

**Signature:**

```elixir title="Elixir"
@spec validate_ocr_backend(String.t()) :: :ok | {:error, String.t()}
```

**Parameters:**

- `backend` (String): OCR backend name

**Valid Backends:**

- "tesseract"
- "easyocr"
- "paddleocr"

**Example:**

```elixir title="validate_ocr.exs"
:ok = Kreuzberg.validate_ocr_backend("tesseract")
{:error, _} = Kreuzberg.validate_ocr_backend("invalid_backend")
```

---

## PDF Rendering

!!! Info "Added in v4.6.2"

### Kreuzberg.render_pdf_page/3

Render a single page of a PDF as a PNG image.

**Signature:**

```elixir title="Elixir"
@spec render_pdf_page(String.t(), non_neg_integer(), keyword()) :: {:ok, binary()} | {:error, term()}
def render_pdf_page(path, page_index, opts \\ [])
```

**Parameters:**

- `path` (String.t()): Path to the PDF file
- `page_index` (non_neg_integer()): Zero-based page index to render

**Options:**

- `:dpi` (integer): Resolution for rendering (default 150)

**Returns:**

- `{:ok, binary()}`: PNG-encoded binary for the requested page
- `{:error, reason}`: Error tuple if file cannot be read, rendered, or page index is out of bounds

**Example:**

```elixir title="render_single_page.exs"
{:ok, png} = Kreuzberg.render_pdf_page("document.pdf", 0)
File.write!("first_page.png", png)
```

---

## Error Handling

### Kreuzberg.Error

Exception struct for Kreuzberg extraction errors.

**Type:**

```elixir title="Elixir"
@type t :: %Kreuzberg.Error{
  message: String.t() | nil,
  reason: atom() | nil,
  context: map() | nil
}
```

**Error Reasons:**

- `:invalid_format` - File format errors
- `:invalid_config` - Configuration or parameter errors
- `:ocr_error` - OCR engine or processing errors
- `:extraction_error` - General extraction failures
- `:io_error` - File I/O related errors
- `:nif_error` - NIF-related errors
- `:unknown_error` - Errors that don't match other categories

**Example - Basic error handling:**

```elixir title="error_handling.exs"
try do
  result = Kreuzberg.extract_file!("document.pdf")
  IO.puts(result.content)
rescue
  e in Kreuzberg.Error ->
    IO.puts("Extraction failed: #{e.message}")
    IO.puts("Error type: #{e.reason}")
end
```

**Example - Pattern matching:**

```elixir title="error_matching.exs"
case Kreuzberg.extract_file("document.pdf") do
  {:ok, result} ->
    IO.puts(result.content)

  {:error, reason} ->
    error_type = Kreuzberg.classify_error(reason)

    case error_type do
      :io_error ->
        IO.puts("File not found or cannot be read")

      :invalid_format ->
        IO.puts("Unsupported or corrupted file format")

      :ocr_error ->
        IO.puts("OCR processing failed")

      _ ->
        IO.puts("Extraction failed: #{reason}")
    end
end
```

---

## Advanced Usage

### Concurrent Processing with Tasks

Process multiple documents concurrently using Elixir's Task module:

```elixir title="concurrent_processing.exs"
files = ["doc1.pdf", "doc2.pdf", "doc3.pdf", "doc4.pdf"]

# Create tasks for concurrent processing
tasks = Enum.map(files, fn file ->
  Task.async(fn ->
    case Kreuzberg.extract_file(file) do
      {:ok, result} -> {file, :ok, result}
      {:error, reason} -> {file, :error, reason}
    end
  end)
end)

# Wait for all tasks to complete
results = Task.await_many(tasks, :timer.minutes(5))

# Process results
Enum.each(results, fn
  {file, :ok, result} ->
    IO.puts("#{file}: #{String.length(result.content)} characters")

  {file, :error, reason} ->
    IO.puts("#{file}: Failed - #{reason}")
end)
```

---

### Streaming Large Documents

For very large documents, consider processing in chunks:

```elixir title="streaming.exs"
config = %Kreuzberg.ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunk_size" => 1024,
    "chunk_overlap" => 200
  }
}

{:ok, result} = Kreuzberg.extract_file("large_document.pdf", nil, config)

# Process chunks individually
if result.chunks do
  result.chunks
  |> Stream.each(fn chunk ->
    # Process each chunk
    content = chunk["content"]
    metadata = chunk["metadata"]

    IO.puts("Chunk: #{String.length(content)} chars")
    IO.puts("Byte range: #{metadata["byte_start"]}-#{metadata["byte_end"]}")
  end)
  |> Stream.run()
end
```

---

### Custom Post-Processing Pipeline

Build a custom processing pipeline using plugins:

```elixir title="pipeline.exs"
defmodule MyApp.Pipeline do
  # Early processor - clean HTML
  defmodule HTMLCleaner do
    def process(result) do
      cleaned_content = result.content
        |> String.replace(~r/<[^>]+>/, "")
        |> String.trim()

      {:ok, %{result | content: cleaned_content}}
    end
  end

  # Middle processor - normalize whitespace
  defmodule WhitespaceNormalizer do
    def process(result) do
      normalized_content = result.content
        |> String.replace(~r/\s+/, " ")
        |> String.trim()

      {:ok, %{result | content: normalized_content}}
    end
  end

  # Late processor - add metadata
  defmodule MetadataEnricher do
    def process(result) do
      enriched_metadata = Map.merge(result.metadata, %{
        "processed_at" => DateTime.utc_now() |> DateTime.to_iso8601(),
        "word_count" => result.content |> String.split() |> length()
      })

      {:ok, %{result | metadata: enriched_metadata}}
    end
  end

  def extract_with_pipeline(path) do
    Kreuzberg.extract_with_plugins(
      File.read!(path),
      "application/pdf",
      nil,
      post_processors: %{
        early: [HTMLCleaner],
        middle: [WhitespaceNormalizer],
        late: [MetadataEnricher]
      }
    )
  end
end

# Use the pipeline
{:ok, result} = MyApp.Pipeline.extract_with_pipeline("document.pdf")
IO.puts("Word count: #{result.metadata["word_count"]}")
```

---

## Type Reference

### BoundingBox

Coordinates for element positioning.

**Fields:**

- `x0` (float): Left x-coordinate.
- `y0` (float): Bottom y-coordinate.
- `x1` (float): Right x-coordinate.
- `y1` (float): Top y-coordinate.

---

### Chunk

A text fragment with embedding.

**Fields:**

- `content` (String): Text content.
- `embedding` (list(float) | nil): Vector embedding.
- `metadata` (Kreuzberg.ChunkMetadata): Positioning metadata.

---

### ChunkMetadata

Positioning and context for a chunk.

**Fields:**

- `byte_start` (non_neg_integer): Start byte offset.
- `byte_end` (non_neg_integer): End byte offset.
- `chunk_index` (non_neg_integer): Position index.
- `first_page` (non_neg_integer | nil): First page number.
- `heading_context` (map | nil): Hierarchical heading context.
- `last_page` (non_neg_integer | nil): Last page number.
- `token_count` (non_neg_integer | nil): Number of tokens.
- `total_chunks` (non_neg_integer): Total chunk count.

---

### DjotContent

Structured Djot document.

**Fields:**

- `attributes` (list): Element attributes.
- `blocks` (list(Kreuzberg.DjotFormattedBlock)): Block-level elements.
- `footnotes` (list(Kreuzberg.DjotFootnote)): Footnote definitions.
- `images` (list(Kreuzberg.DjotImage)): Extracted images.
- `links` (list(Kreuzberg.DjotLink)): Extracted links.
- `metadata` (Kreuzberg.Metadata): Document metadata.
- `plain_text` (String): Plain text fallback.
- `tables` (list(Kreuzberg.Table)): Extracted tables.

---

### DocumentNode

A node in the hierarchical document tree.

**Fields:**

- `annotations` (list(Kreuzberg.DocumentTextAnnotation)): Inline annotations.
- `bbox` (Kreuzberg.BoundingBox | nil): Node bounding box.
- `children` (list(non_neg_integer)): Indices of child nodes.
- `content` (map): Type-specific content data.
- `content_layer` (String | nil): Layer (body, header, etc.).
- `id` (String): Unique node identifier.
- `node_type` (String): Semantic type description.
- `page_number` (non_neg_integer | nil): Starting page.
- `page_number_end` (non_neg_integer | nil): Ending page.
- `parent` (non_neg_integer | nil): Index of parent node.

---

### DocumentStructure

Hierarchical representation of the document.

**Fields:**

- `nodes` (list(Kreuzberg.DocumentNode)): Flat list of nodes forming a tree.

---

### Element

Semantic element (Unstructured compatibility).

**Fields:**

- `element_id` (String): Deterministic unique ID.
- `element_type` (atom): Semantic type (for example, `:narrative_text`).
- `metadata` (Kreuzberg.ElementMetadata): Positioning and source metadata.
- `text` (String): Text content.

---

### Image

Extracted image with metadata.

**Fields:**

- `bits_per_component` (non_neg_integer | nil): Bit depth.
- `bounding_box` (map | nil): Box coordinates.
- `colorspace` (String | nil): Color space (for example, "RGB").
- `data` (binary): Raw image data.
- `description` (String | nil): Alt text or description.
- `format` (String): Format ("png", "jpeg", etc.).
- `height` (non_neg_integer | nil): Pixel height.
- `image_index` (non_neg_integer): Position in document.
- `is_mask` (boolean): Whether it's a mask image.
- `ocr_result` (Kreuzberg.ExtractionResult | nil): Recursive OCR results.
- `page_number` (non_neg_integer | nil): Page where found.
- `width` (non_neg_integer | nil): Pixel width.

---

### Keyword

Extracted keyword with relevance score.

**Fields:**

- `score` (float): Relevance score (0.0 to 1.0).
- `text` (String): Keyword text.

---

### OcrBoundingGeometry

Geometry for OCR elements.

**Fields:**

- `height` (float): Height in pixels.
- `left` (float): X-coordinate.
- `top` (float): Y-coordinate.
- `type` (String): Geometry type ("rect", etc.).
- `width` (float): Width in pixels.

---

### OcrElement

Detailed OCR element.

**Fields:**

- `backend_metadata` (map | nil): Backend-specific data.
- `confidence` (Kreuzberg.OcrConfidence | nil): Confidence scores.
- `geometry` (Kreuzberg.OcrBoundingGeometry | nil): Relative positioning.
- `level` (String | nil): Hierarchy level ("word", "line", etc.).
- `page_number` (non_neg_integer | nil): Page where found.
- `parent_id` (String | nil): ID of parent element.
- `rotation` (map | nil): Angle and orientation.
- `text` (String): Recognized text.

---

### Page

Single page from a document.

**Fields:**

- `content` (String): Page text.
- `hierarchy` (map | nil): Structural hierarchy.
- `images` (list(Kreuzberg.Image)): Page images.
- `is_blank` (boolean | nil): Blank page detection.
- `page_number` (non_neg_integer): 0-indexed page number.
- `tables` (list(Kreuzberg.Table)): Page tables.

---

### ProcessingWarning

Warning from the extraction pipeline.

**Fields:**

- `message` (String): Warning description.
- `source` (String): Component that issued the warning.

---

### Complete Type Specifications

```elixir title="types.exs"
# Core types
@type extraction_result :: {:ok, ExtractionResult.t()} | {:error, String.t()}
@type batch_result :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}

# Config & Options
@type config_option :: ExtractionConfig.t() | map() | keyword() | nil

# Async Task types
@type async_result :: Task.t(extraction_result())
@type async_batch_result :: Task.t(batch_result())

# Plugin types
@type ocr_backend :: module()
@type post_processor :: module()
@type validator :: module()

# Status & Stats
@type cache_stats :: %{
  required(String.t()) => non_neg_integer() | float()
}

# Errors
@type error_reason ::
  :extraction_error |
  :invalid_config |
  :invalid_format |
  :io_error |
  :nif_error |
  :ocr_error |
  :unknown_error
```

---

## System Requirements

**Elixir:** 1.12 or higher

**Erlang/OTP:** 24 or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- MacOS (x64, arm64)
- Windows (x64)

---

## Thread Safety

All Kreuzberg functions are process-safe and can be called from multiple Elixir processes concurrently. The underlying NIF implementation uses a thread pool for parallel processing.

**Example - Concurrent extraction:**

```elixir title="concurrent.exs"
files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]

tasks = Enum.map(files, fn file ->
  Task.async(fn ->
    Kreuzberg.extract_file(file)
  end)
end)

results = Task.await_many(tasks)
```

However, for better performance with multiple files, use the batch API:

```elixir title="batch.exs"
# More efficient approach
files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
{:ok, results} = Kreuzberg.batch_extract_files(files)
```

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. The Elixir binding passes LLM configuration as map options through the Rustler NIF layer. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Pass `structured_extraction` config to extract structured data from documents using an LLM:

```elixir title="structured_extraction.exs"
config = %{
  structured_extraction: %{
    schema: %{
      "type" => "object",
      "properties" => %{
        "title" => %{"type" => "string"},
        "authors" => %{"type" => "array", "items" => %{"type" => "string"}},
        "date" => %{"type" => "string"}
      },
      "required" => ["title", "authors", "date"],
      "additionalProperties" => false
    },
    llm: %{model: "openai/gpt-4o-mini"},
    strict: true
  }
}

{:ok, result} = Kreuzberg.extract_file("paper.pdf", config)

case result.structured_output do
  nil -> IO.puts("No structured output")
  output -> IO.puts(output)
end
```

### VLM OCR

Use a vision-language model as an OCR backend:

```elixir title="vlm_ocr.exs"
config = %{
  force_ocr: true,
  ocr: %{
    backend: "vlm",
    vlm_config: %{model: "openai/gpt-4o-mini"}
  }
}

{:ok, result} = Kreuzberg.extract_file("scan.pdf", config)
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## Version Information

Check the Kreuzberg version:

```elixir title="version.exs"
IO.puts(Application.spec(:kreuzberg, :vsn))
```

---

## Additional Resources

- [GitHub Repository](https://github.com/kreuzberg-dev/kreuzberg)
- [Installation Guide](../getting-started/installation.md)
- [Error Handling Reference](errors.md)
- [Type Reference](types.md)

---

## License

Kreuzberg Elixir package is released under the same license as the main Kreuzberg project.
