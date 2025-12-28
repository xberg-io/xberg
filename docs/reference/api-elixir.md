# Elixir API Reference

Complete reference for the Kreuzberg Elixir API.

## Installation

Add to your `mix.exs`:

```elixir title="mix.exs"
def deps do
  [
    {:kreuzberg, "~> 0.1.0"}
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
- `mime_type` (String): MIME type of the document (e.g., "application/pdf", "text/plain")
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
  force_ocr: true,
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

### ExtractionConfig

Main configuration struct for extraction operations.

**Type:**

```elixir title="Elixir"
@type t :: %Kreuzberg.ExtractionConfig{
  chunking: map() | nil,
  ocr: map() | nil,
  language_detection: map() | nil,
  postprocessor: map() | nil,
  images: map() | nil,
  pages: map() | nil,
  token_reduction: map() | nil,
  keywords: map() | nil,
  pdf_options: map() | nil,
  use_cache: boolean(),
  enable_quality_processing: boolean(),
  force_ocr: boolean()
}
```

**Fields:**

- `chunking` (map | nil): Text chunking configuration
- `ocr` (map | nil): OCR configuration
- `language_detection` (map | nil): Language detection settings
- `postprocessor` (map | nil): Post-processor configuration
- `images` (map | nil): Image extraction configuration
- `pages` (map | nil): Page-level extraction configuration
- `token_reduction` (map | nil): Token reduction settings
- `keywords` (map | nil): Keyword extraction configuration
- `pdf_options` (map | nil): PDF-specific options
- `use_cache` (boolean): Enable result caching (default: true)
- `enable_quality_processing` (boolean): Enable quality post-processing (default: true)
- `force_ocr` (boolean): Force OCR even for searchable PDFs (default: false)

**Example - Basic configuration:**

```elixir title="basic_config.exs"
config = %Kreuzberg.ExtractionConfig{
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)
```

**Example - OCR configuration:**

```elixir title="ocr_config.exs"
config = %Kreuzberg.ExtractionConfig{
  force_ocr: true,
  ocr: %{
    "enabled" => true,
    "language" => "eng",
    "backend" => "tesseract"
  }
}

{:ok, result} = Kreuzberg.extract_file("scanned.pdf", nil, config)
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

---

## Results & Types

### ExtractionResult

Result struct returned by all extraction functions.

**Type:**

```elixir title="Elixir"
@type t :: %Kreuzberg.ExtractionResult{
  content: String.t(),
  mime_type: String.t(),
  metadata: map(),
  tables: [map()],
  detected_languages: [String.t()] | nil,
  chunks: [map()] | nil,
  images: [map()] | nil,
  pages: [map()] | nil
}
```

**Fields:**

- `content` (String): Extracted text content
- `mime_type` (String): MIME type of the processed document
- `metadata` (map): Document metadata (format-specific fields)
- `tables` (list): List of extracted tables
- `detected_languages` (list | nil): List of detected language codes (ISO 639-1) if language detection is enabled
- `chunks` (list | nil): List of text chunks with embeddings when chunking is enabled
- `images` (list | nil): List of extracted images when image extraction is enabled
- `pages` (list | nil): Per-page extracted content when page extraction is enabled

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

- `"language"` (String): Document language (ISO 639-1 code)
- `"date"` (String): Document date (ISO 8601 format)
- `"subject"` (String): Document subject
- `"format_type"` (String): Format discriminator ("pdf", "excel", "email", etc.)

**PDF-Specific Fields** (when `format_type == "pdf"`):

- `"title"` (String): PDF title
- `"author"` (String): PDF author
- `"page_count"` (integer): Number of pages
- `"creation_date"` (String): Creation date (ISO 8601)
- `"modification_date"` (String): Modification date (ISO 8601)
- `"creator"` (String): Creator application
- `"producer"` (String): Producer application
- `"keywords"` (String): PDF keywords

**Excel-Specific Fields** (when `format_type == "excel"`):

- `"sheet_count"` (integer): Number of sheets
- `"sheet_names"` (list): List of sheet names

**Email-Specific Fields** (when `format_type == "email"`):

- `"from_email"` (String): Sender email address
- `"from_name"` (String): Sender name
- `"to_emails"` (list): Recipient email addresses
- `"cc_emails"` (list): CC email addresses
- `"bcc_emails"` (list): BCC email addresses
- `"message_id"` (String): Email message ID
- `"attachments"` (list): List of attachment filenames

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

- `"cells"` (list): 2D array of table cells (rows x columns)
- `"markdown"` (String): Table rendered as markdown
- `"page_number"` (integer): Page number where table was found

**Example:**

```elixir title="tables.exs"
{:ok, result} = Kreuzberg.extract_file("invoice.pdf")

Enum.each(result.tables, fn table ->
  IO.puts("Table on page #{table["page_number"]}:")
  IO.puts(table["markdown"])

  # Access raw cell data
  Enum.each(table["cells"], fn row ->
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

- `code` (String): Language code string (e.g., "en", "eng", "de", "deu")

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
    text = chunk["text"]
    metadata = chunk["metadata"]

    IO.puts("Chunk: #{String.length(text)} chars")
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

### Complete Type Specifications

```elixir title="types.exs"
# Configuration types
@type config_option :: ExtractionConfig.t() | map() | keyword() | nil

# Result types
@type extraction_result :: {:ok, ExtractionResult.t()} | {:error, String.t()}
@type batch_result :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}

# Async types
@type async_result :: Task.t(extraction_result())
@type async_batch_result :: Task.t(batch_result())

# Plugin types
@type post_processor :: module()
@type validator :: module()
@type ocr_backend :: module()

# Cache types
@type cache_stats :: %{
  "total_files" => non_neg_integer(),
  "total_size_mb" => float(),
  "available_space_mb" => float(),
  "oldest_file_age_days" => non_neg_integer(),
  "newest_file_age_days" => non_neg_integer()
}

# Error types
@type error_reason ::
  :invalid_format |
  :invalid_config |
  :ocr_error |
  :extraction_error |
  :io_error |
  :nif_error |
  :unknown_error
```

---

## System Requirements

**Elixir:** 1.12 or higher

**Erlang/OTP:** 24 or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)
- LibreOffice (for legacy Office formats): `brew install libreoffice` (macOS) or `apt-get install libreoffice` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- macOS (x64, arm64)
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
