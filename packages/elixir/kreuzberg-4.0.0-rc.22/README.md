# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python&color=007ec6)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript&color=007ec6)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM&color=007ec6)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby&color=007ec6)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java&color=007ec6)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Goldziher.Kreuzberg?label=C%23&color=007ec6)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-007ec6)](https://kreuzberg.dev/)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-007ec6)](https://discord.gg/pXxagNK2zN)

High-performance document intelligence for Elixir, powered by Rust.

Extract text, tables, images, and metadata from 56 file formats including PDF, DOCX, PPTX, XLSX, images, and more.

**Powered by a Rust core** – Native performance with Elixir's concurrency model.

> **Version 4.0.0 Release Candidate**
> This is a pre-release version. Please test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter.

## Features

- **56 file formats** – PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, XML, JSON, and more
- **OCR support** – Built-in Tesseract OCR for scanned documents and images
- **High performance** – Rust-powered extraction with Elixir's BEAM VM
- **Task-based async** – Native Task.async support for concurrent document processing
- **Table extraction** – Extract structured tables from documents
- **Language detection** – Automatic language detection for extracted text
- **Text chunking** – Split long documents into manageable chunks with embeddings
- **Plugin system** – Extensible architecture with validators, post-processors, and custom OCR backends
- **Caching** – Built-in result caching with GenServer-based cache management
- **Batch processing** – Efficient batch operations for multiple documents
- **OTP integration** – Supervised Plugin.Registry GenServer for runtime plugin management
- **Type-safe** – Comprehensive typespecs and structs for configuration and results

## Requirements

- Elixir 1.14 or higher
- Erlang/OTP 25 or higher
- Rust toolchain (for building from source)

### Optional System Dependencies

- **ONNX Runtime**: For embeddings functionality
  - macOS: `brew install onnxruntime`
  - Ubuntu: `sudo apt-get install libonnxruntime libonnxruntime-dev`
  - Windows: `scoop install onnxruntime` or download from [GitHub](https://github.com/microsoft/onnxruntime/releases)

- **Tesseract**: For OCR functionality
  - macOS: `brew install tesseract`
  - Ubuntu: `sudo apt-get install tesseract-ocr`
  - Windows: Download from [GitHub](https://github.com/tesseract-ocr/tesseract)

- **LibreOffice**: For legacy MS Office formats (.doc, .ppt)
  - macOS: `brew install libreoffice`
  - Ubuntu: `sudo apt-get install libreoffice`

- **Pandoc**: For advanced document conversion
  - macOS: `brew install pandoc`
  - Ubuntu: `sudo apt-get install pandoc`

## Installation

Add `kreuzberg` to your `mix.exs` dependencies:

```elixir
def deps do
  [
    {:kreuzberg, "~> 4.0.0-rc.22"}
  ]
end
```

Then run:

```bash
mix deps.get
```

## Quick Start

### Basic Extraction

```elixir
# Extract from a file
{:ok, result} = Kreuzberg.extract_file("document.pdf")
IO.puts(result.content)

# Extract with explicit MIME type
{:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf")

# Extract from binary data
pdf_binary = File.read!("document.pdf")
{:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf")
```

### Using Bang Variants

```elixir
# Raises Kreuzberg.Error on failure
result = Kreuzberg.extract_file!("document.pdf")
IO.puts(result.content)

# Pattern match on the result
%Kreuzberg.ExtractionResult{content: text, mime_type: mime} =
  Kreuzberg.extract_file!("document.pdf")
```

### With Configuration

```elixir
# Using struct configuration
config = %Kreuzberg.ExtractionConfig{
  use_cache: true,
  force_ocr: false,
  ocr: %{"backend" => "tesseract", "language" => "eng"}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf", config)

# Using keyword list configuration
{:ok, result} = Kreuzberg.extract_file(
  "document.pdf",
  "application/pdf",
  ocr: %{"backend" => "tesseract", "language" => "eng"},
  use_cache: true
)
```

## Configuration

### Extraction Configuration

```elixir
config = %Kreuzberg.ExtractionConfig{
  # Caching
  use_cache: true,

  # Quality processing
  enable_quality_processing: true,

  # OCR configuration
  force_ocr: false,
  ocr: %{
    "backend" => "tesseract",
    "language" => "eng",
    "tesseract_config" => %{
      "psm" => 6,
      "enable_table_detection" => true,
      "min_confidence" => 50.0
    }
  },

  # Text chunking
  chunking: %{
    "enabled" => true,
    "chunk_size" => 1000,
    "chunk_overlap" => 200,
    "embedding" => %{
      "model" => %{"type" => "preset", "name" => "balanced"},
      "normalize" => true
    }
  },

  # Image extraction
  images: %{
    "extract_images" => true,
    "target_dpi" => 300,
    "max_image_dimension" => 4096,
    "auto_adjust_dpi" => true
  },

  # Language detection
  language_detection: %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => false
  },

  # PDF options
  pdf_options: %{
    "extract_images" => true,
    "passwords" => ["password1", "password2"],
    "extract_metadata" => true
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf", config)
```

### OCR with Table Detection

```elixir
config = %Kreuzberg.ExtractionConfig{
  ocr: %{
    "backend" => "tesseract",
    "language" => "eng",
    "tesseract_config" => %{
      "enable_table_detection" => true,
      "psm" => 6,
      "preprocessing" => %{
        "auto_rotate" => true,
        "deskew" => true
      }
    }
  }
}

{:ok, result} = Kreuzberg.extract_file("invoice.pdf", "application/pdf", config)

# Access extracted tables
Enum.each(result.tables, fn table ->
  IO.puts("Table with #{length(table["rows"])} rows")
  IO.puts(table["markdown"])
end)
```

### Password-Protected PDFs

```elixir
config = %Kreuzberg.ExtractionConfig{
  pdf_options: %{
    "passwords" => ["password1", "password2", "password3"]
  }
}

{:ok, result} = Kreuzberg.extract_file("protected.pdf", "application/pdf", config)
```

## Batch Processing

### Batch File Extraction

```elixir
# Extract multiple files
paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
{:ok, results} = Kreuzberg.batch_extract_files(paths, "application/pdf")

Enum.each(results, fn result ->
  IO.puts("Content: #{String.slice(result.content, 0..100)}")
end)

# With configuration
config = %Kreuzberg.ExtractionConfig{use_cache: true}
{:ok, results} = Kreuzberg.batch_extract_files(paths, "application/pdf", config)

# Bang variant
results = Kreuzberg.batch_extract_files!(paths, "application/pdf")
```

### Batch Binary Extraction

```elixir
# Multiple binary inputs with same MIME type
data_list = [pdf_binary1, pdf_binary2, pdf_binary3]
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, "application/pdf")

# Different MIME types for each input
mime_types = ["application/pdf", "text/plain", "text/html"]
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, mime_types)

# With configuration
config = %Kreuzberg.ExtractionConfig{force_ocr: true}
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, mime_types, config)
```

## Async Operations with Tasks

Kreuzberg provides native Elixir Task-based async operations for concurrent document processing.

### Extract Single Document Asynchronously

```elixir
# Start async extraction
task = Kreuzberg.extract_async(pdf_binary, "application/pdf")

# Do other work...

# Await the result
{:ok, result} = Task.await(task)
IO.puts(result.content)
```

### Extract Multiple Documents Concurrently

```elixir
# Start multiple async extractions
tasks = [
  Kreuzberg.extract_file_async("doc1.pdf"),
  Kreuzberg.extract_file_async("doc2.pdf"),
  Kreuzberg.extract_file_async("doc3.pdf")
]

# Await all results
results = Task.await_many(tasks)

# Process results
Enum.each(results, fn
  {:ok, result} -> IO.puts("Success: #{String.slice(result.content, 0..50)}")
  {:error, reason} -> IO.puts("Error: #{reason}")
end)
```

### Async Batch Processing

```elixir
# Batch extract files asynchronously
paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
task = Kreuzberg.batch_extract_files_async(paths, "application/pdf")

# Await batch results
{:ok, results} = Task.await(task)

# With configuration
config = %Kreuzberg.ExtractionConfig{use_cache: true}
task = Kreuzberg.batch_extract_files_async(paths, "application/pdf", config)
{:ok, results} = Task.await(task, 30_000)  # 30 second timeout
```

### Complex Async Workflows

```elixir
# Process documents with pattern matching
with_results = fn paths ->
  paths
  |> Enum.map(&Kreuzberg.extract_file_async/1)
  |> Task.await_many()
  |> Enum.reduce({:ok, []}, fn
    {:ok, result}, {:ok, acc} -> {:ok, [result | acc]}
    {:error, reason}, _acc -> {:error, reason}
  end)
end

case with_results.(["file1.pdf", "file2.pdf"]) do
  {:ok, results} ->
    IO.puts("Extracted #{length(results)} documents")
  {:error, reason} ->
    IO.puts("Extraction failed: #{reason}")
end
```

## Plugin System

Kreuzberg features an extensible plugin architecture with validators, post-processors, and custom OCR backends. Plugins are managed by a GenServer-based registry that runs under your application's supervision tree.

### Post-Processors

Post-processors transform extraction results in customizable stages (early, middle, late).

```elixir
# Define a post-processor module
defmodule MyApp.HTMLCleaner do
  @behaviour Kreuzberg.Plugin.PostProcessor

  def process(result, _opts) do
    cleaned_content =
      result.content
      |> String.replace(~r/<[^>]+>/, "")
      |> String.trim()

    {:ok, %{result | content: cleaned_content}}
  end
end

# Register the plugin (typically in your application.ex)
:ok = Kreuzberg.Plugin.register_post_processor(:html_cleaner, MyApp.HTMLCleaner)

# Use with extraction
{:ok, result} = Kreuzberg.extract_with_plugins(
  pdf_binary,
  "application/pdf",
  nil,
  post_processors: %{
    early: [MyApp.HTMLCleaner]
  }
)
```

### Validators

Validators check inputs before extraction or results after extraction.

```elixir
# Define a validator module
defmodule MyApp.ContentValidator do
  @behaviour Kreuzberg.Plugin.Validator

  def validate(result) do
    if result && byte_size(result.content) > 0 do
      :ok
    else
      {:error, "Content is empty"}
    end
  end
end

# Register the validator
:ok = Kreuzberg.Plugin.register_validator(MyApp.ContentValidator)

# Use with extraction
{:ok, result} = Kreuzberg.extract_with_plugins(
  pdf_binary,
  "application/pdf",
  nil,
  validators: [MyApp.InputValidator],
  final_validators: [MyApp.ContentValidator]
)
```

### Multi-Stage Pipeline

```elixir
# Complex extraction pipeline with multiple plugins
{:ok, result} = Kreuzberg.extract_with_plugins(
  pdf_binary,
  "application/pdf",
  config,
  validators: [MyApp.InputValidator],
  post_processors: %{
    early: [MyApp.HTMLCleaner, MyApp.Normalizer],
    middle: [MyApp.EntityExtractor],
    late: [MyApp.Formatter]
  },
  final_validators: [MyApp.ContentValidator, MyApp.LengthValidator]
)
```

### Plugin Management

```elixir
# List registered plugins
{:ok, processors} = Kreuzberg.Plugin.list_post_processors()
{:ok, validators} = Kreuzberg.Plugin.list_validators()

# Unregister plugins
:ok = Kreuzberg.Plugin.unregister_post_processor(:html_cleaner)
:ok = Kreuzberg.Plugin.unregister_validator(MyApp.ContentValidator)

# Clear all plugins of a type
:ok = Kreuzberg.Plugin.clear_post_processors()
:ok = Kreuzberg.Plugin.clear_validators()
```

## Utilities

### MIME Type Detection

```elixir
# Detect MIME type from binary data
{:ok, mime_type} = Kreuzberg.detect_mime_type(binary_data)

# Detect MIME type from file path
{:ok, mime_type} = Kreuzberg.detect_mime_type_from_path("document.pdf")

# Validate MIME type
{:ok, _} = Kreuzberg.validate_mime_type("application/pdf")
{:error, _} = Kreuzberg.validate_mime_type("invalid/type")

# Get file extensions for MIME type
{:ok, extensions} = Kreuzberg.get_extensions_for_mime("application/pdf")
# => ["pdf"]
```

### Embedding Presets

```elixir
# List available embedding presets
{:ok, presets} = Kreuzberg.list_embedding_presets()
# => ["balanced", "fast", "quality", "multilingual"]

# Get preset details
{:ok, preset} = Kreuzberg.get_embedding_preset("balanced")
IO.inspect(preset)
# => %{
#   "name" => "balanced",
#   "chunk_size" => 512,
#   "overlap" => 128,
#   "dimensions" => 384,
#   "description" => "Balanced quality and speed"
# }
```

### Cache Management

```elixir
# Get cache statistics
{:ok, stats} = Kreuzberg.cache_stats()
IO.inspect(stats)
# => %{
#   "total_entries" => 42,
#   "total_size_bytes" => 1048576,
#   "hit_rate" => 0.85
# }

# Clear the cache
{:ok, _} = Kreuzberg.clear_cache()

# Bang variants
stats = Kreuzberg.cache_stats!()
:ok = Kreuzberg.clear_cache!()
```

### Error Classification

```elixir
# Classify error messages
error_type = Kreuzberg.classify_error("File not found: /path/to/file.pdf")
# => :io_error

error_type = Kreuzberg.classify_error("Invalid PDF format")
# => :invalid_format

# Get error category details
{:ok, details} = Kreuzberg.get_error_details()
IO.inspect(details[:io_error])
# => %{
#   "name" => "IO Error",
#   "description" => "File I/O related errors...",
#   "examples" => ["File not found", "Permission denied", ...]
# }
```

## Working with Results

### Extraction Result Structure

```elixir
result = Kreuzberg.extract_file!("document.pdf")

# Access extracted content
IO.puts(result.content)

# Access MIME type
IO.puts(result.mime_type)

# Access metadata
IO.inspect(result.metadata)

# Access tables
Enum.each(result.tables, fn table ->
  IO.puts("Table with #{length(table["rows"])} rows")
  IO.puts(table["markdown"])
end)

# Access chunks (if chunking is enabled)
if result.chunks do
  Enum.each(result.chunks, fn chunk ->
    IO.puts("Chunk: #{chunk["content"]}")
    IO.puts("Tokens: #{chunk["metadata"]["token_count"]}")
  end)
end

# Access images (if image extraction is enabled)
if result.images do
  Enum.each(result.images, fn image ->
    File.write!("image_#{image["index"]}.png", image["data"])
  end)
end

# Access detected languages
if result.detected_languages do
  Enum.each(result.detected_languages, fn lang ->
    IO.puts("Language: #{lang["language"]}, Confidence: #{lang["confidence"]}")
  end)
end
```

### Pattern Matching on Results

```elixir
# Extract specific fields
%Kreuzberg.ExtractionResult{
  content: text,
  mime_type: mime,
  metadata: metadata
} = Kreuzberg.extract_file!("document.pdf")

# Pattern match on optional fields
case Kreuzberg.extract_file("document.pdf") do
  {:ok, %{tables: tables}} when is_list(tables) and length(tables) > 0 ->
    IO.puts("Found #{length(tables)} tables")

  {:ok, %{content: content}} ->
    IO.puts("Extracted text: #{content}")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```

### Using with Pipes

```elixir
"document.pdf"
|> Kreuzberg.extract_file!()
|> Map.get(:content)
|> String.downcase()
|> String.split("\n")
|> Enum.filter(&(String.length(&1) > 10))
|> Enum.take(5)
|> Enum.join("\n")
|> IO.puts()
```

## Error Handling

### Standard Error Handling

```elixir
case Kreuzberg.extract_file("document.pdf") do
  {:ok, result} ->
    IO.puts("Success: #{result.content}")

  {:error, reason} ->
    error_type = Kreuzberg.classify_error(reason)

    case error_type do
      :io_error ->
        IO.puts("File not found or permission denied: #{reason}")

      :invalid_format ->
        IO.puts("Invalid or corrupted file: #{reason}")

      :ocr_error ->
        IO.puts("OCR processing failed: #{reason}")

      :extraction_error ->
        IO.puts("Extraction failed: #{reason}")

      _ ->
        IO.puts("Unknown error: #{reason}")
    end
end
```

### Using with Statement

```elixir
with {:ok, binary} <- File.read("document.pdf"),
     {:ok, mime_type} <- Kreuzberg.detect_mime_type(binary),
     {:ok, result} <- Kreuzberg.extract(binary, mime_type) do
  IO.puts("Successfully extracted: #{String.slice(result.content, 0..100)}")
else
  {:error, :enoent} ->
    IO.puts("File not found")

  {:error, reason} when is_binary(reason) ->
    IO.puts("Kreuzberg error: #{reason}")

  error ->
    IO.puts("Unexpected error: #{inspect(error)}")
end
```

### Exception-Based Error Handling

```elixir
try do
  result = Kreuzberg.extract_file!("document.pdf")
  IO.puts(result.content)
rescue
  e in Kreuzberg.Error ->
    IO.puts("Kreuzberg error: #{e.message}")
    IO.puts("Error type: #{e.reason}")
end
```

## Integration with Phoenix

### In a Phoenix Controller

```elixir
defmodule MyAppWeb.DocumentController do
  use MyAppWeb, :controller

  def upload(conn, %{"document" => %Plug.Upload{path: path}}) do
    case Kreuzberg.extract_file(path) do
      {:ok, result} ->
        json(conn, %{
          content: result.content,
          mime_type: result.mime_type,
          tables: result.tables
        })

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: reason})
    end
  end
end
```

### Async Processing with Task

```elixir
defmodule MyAppWeb.DocumentController do
  use MyAppWeb, :controller

  def extract_async(conn, %{"documents" => uploads}) do
    task = Task.async(fn ->
      paths = Enum.map(uploads, & &1.path)
      Kreuzberg.batch_extract_files(paths)
    end)

    case Task.await(task, 30_000) do
      {:ok, results} ->
        json(conn, %{
          count: length(results),
          results: Enum.map(results, &Map.take(&1, [:content, :mime_type]))
        })

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: reason})
    end
  end
end
```

## Supported Formats

- **Documents**: PDF, DOCX, DOC, PPTX, PPT, ODT, ODP
- **Spreadsheets**: XLSX, XLS, ODS, CSV
- **Images**: PNG, JPEG, TIFF, BMP, GIF
- **Web**: HTML, MHTML, Markdown
- **Data**: JSON, YAML, TOML, XML
- **Email**: EML, MSG
- **Archives**: ZIP, TAR, 7Z
- **Text**: TXT, RTF, MD

## Performance

Kreuzberg's Rust core provides exceptional performance:

- **PDF extraction**: 10-50x faster than pure Elixir solutions
- **Concurrent processing**: Native BEAM concurrency with Task-based async
- **Batch processing**: Efficient parallel extraction in Rust
- **Memory efficient**: Streaming parsers for large files
- **Caching**: GenServer-based cache for repeated extractions

## PDFium Integration

PDF extraction is powered by PDFium, which is automatically bundled with this package. No system installation required.

### Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | ✅ | Bundled |
| macOS ARM64 | ✅ | Bundled |
| macOS x86_64 | ✅ | Bundled |
| Windows x86_64 | ✅ | Bundled |

### Binary Size Impact

PDFium adds approximately 8-15 MB to the package size depending on platform. This ensures consistent PDF extraction across all environments without external dependencies.

## API Reference

### Main Extraction Functions

- `Kreuzberg.extract/3` – Extract from binary data
- `Kreuzberg.extract!/3` – Extract from binary data, raising on error
- `Kreuzberg.extract_file/3` – Extract from file path
- `Kreuzberg.extract_file!/3` – Extract from file path, raising on error
- `Kreuzberg.extract_with_plugins/4` – Extract with plugin processing

### Batch Operations

- `Kreuzberg.batch_extract_files/3` – Batch extract from file paths
- `Kreuzberg.batch_extract_files!/3` – Batch extract from file paths, raising on error
- `Kreuzberg.batch_extract_bytes/3` – Batch extract from binary data
- `Kreuzberg.batch_extract_bytes!/3` – Batch extract from binary data, raising on error

### Async Operations (via Task)

- `Kreuzberg.extract_async/3` – Async extract from binary data
- `Kreuzberg.extract_file_async/3` – Async extract from file path
- `Kreuzberg.batch_extract_files_async/3` – Async batch extract from files
- `Kreuzberg.batch_extract_bytes_async/3` – Async batch extract from binary data

### Utility Functions

- `Kreuzberg.detect_mime_type/1` – Detect MIME type from binary
- `Kreuzberg.detect_mime_type_from_path/1` – Detect MIME type from path
- `Kreuzberg.validate_mime_type/1` – Validate MIME type
- `Kreuzberg.get_extensions_for_mime/1` – Get file extensions for MIME type
- `Kreuzberg.list_embedding_presets/0` – List embedding presets
- `Kreuzberg.get_embedding_preset/1` – Get embedding preset details
- `Kreuzberg.classify_error/1` – Classify error message
- `Kreuzberg.get_error_details/0` – Get error category details

### Cache Operations

- `Kreuzberg.cache_stats/0` – Get cache statistics
- `Kreuzberg.cache_stats!/0` – Get cache statistics, raising on error
- `Kreuzberg.clear_cache/0` – Clear cache
- `Kreuzberg.clear_cache!/0` – Clear cache, raising on error

### Plugin Management

- `Kreuzberg.Plugin.register_post_processor/2` – Register post-processor
- `Kreuzberg.Plugin.register_validator/1` – Register validator
- `Kreuzberg.Plugin.register_ocr_backend/1` – Register OCR backend
- `Kreuzberg.Plugin.list_post_processors/0` – List post-processors
- `Kreuzberg.Plugin.list_validators/0` – List validators
- `Kreuzberg.Plugin.list_ocr_backends/0` – List OCR backends
- `Kreuzberg.Plugin.unregister_post_processor/1` – Unregister post-processor
- `Kreuzberg.Plugin.unregister_validator/1` – Unregister validator
- `Kreuzberg.Plugin.unregister_ocr_backend/1` – Unregister OCR backend

### Configuration Types

- `Kreuzberg.ExtractionConfig` – Main configuration struct

### Result Types

- `Kreuzberg.ExtractionResult` – Extraction result with content, metadata, tables, etc.

### Error Types

- `Kreuzberg.Error` – Exception with message and reason atom

## Examples

### Processing Multiple Files with Progress

```elixir
files = Path.wildcard("documents/*.pdf")
total = length(files)

files
|> Enum.with_index(1)
|> Enum.map(fn {file, index} ->
  IO.puts("Processing #{index}/#{total}: #{Path.basename(file)}")

  case Kreuzberg.extract_file(file) do
    {:ok, result} ->
      {file, :ok, String.length(result.content)}
    {:error, reason} ->
      {file, :error, reason}
  end
end)
|> Enum.each(fn
  {file, :ok, length} ->
    IO.puts("✓ #{Path.basename(file)}: #{length} characters")
  {file, :error, reason} ->
    IO.puts("✗ #{Path.basename(file)}: #{reason}")
end)
```

### Concurrent Processing with Task.Supervisor

```elixir
defmodule MyApp.DocumentProcessor do
  def process_documents(paths) do
    paths
    |> Task.async_stream(
      &process_document/1,
      max_concurrency: System.schedulers_online() * 2,
      timeout: 30_000
    )
    |> Enum.to_list()
  end

  defp process_document(path) do
    with {:ok, result} <- Kreuzberg.extract_file(path),
         {:ok, processed} <- post_process(result) do
      {:ok, processed}
    end
  end

  defp post_process(result) do
    # Custom processing logic
    {:ok, result}
  end
end
```

### GenServer-Based Document Queue

```elixir
defmodule MyApp.DocumentQueue do
  use GenServer

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def enqueue(path) do
    GenServer.cast(__MODULE__, {:enqueue, path})
  end

  def init(_opts) do
    {:ok, %{queue: :queue.new(), processing: false}}
  end

  def handle_cast({:enqueue, path}, state) do
    new_queue = :queue.in(path, state.queue)
    new_state = %{state | queue: new_queue}

    if not state.processing do
      send(self(), :process_next)
    end

    {:noreply, new_state}
  end

  def handle_info(:process_next, %{queue: queue} = state) do
    case :queue.out(queue) do
      {{:value, path}, new_queue} ->
        task = Kreuzberg.extract_file_async(path)

        spawn(fn ->
          case Task.await(task) do
            {:ok, result} ->
              IO.puts("Processed: #{path}")
            {:error, reason} ->
              IO.puts("Failed: #{path} - #{reason}")
          end

          send(self(), :process_next)
        end)

        {:noreply, %{queue: new_queue, processing: true}}

      {:empty, _} ->
        {:noreply, %{state | processing: false}}
    end
  end
end
```

## Troubleshooting

### Compilation Issues

If you encounter compilation errors, ensure you have the Rust toolchain installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then try recompiling:

```bash
mix deps.clean kreuzberg --build
mix deps.get
mix deps.compile
```

### OCR Not Working

Make sure Tesseract is installed and available:

```bash
tesseract --version
```

If Tesseract is installed but not detected, ensure it's in your PATH.

### Memory Issues with Large PDFs

For large documents, enable chunking to reduce memory usage:

```elixir
config = %Kreuzberg.ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "chunk_size" => 1000,
    "chunk_overlap" => 200
  }
}

{:ok, result} = Kreuzberg.extract_file("large_document.pdf", "application/pdf", config)
```

### Plugin Registry Not Started

If you see errors about the Plugin.Registry not being started, ensure your application is running:

```bash
iex -S mix
```

The Plugin.Registry GenServer is automatically started under the Kreuzberg.Application supervisor.

## Documentation

For comprehensive documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

API documentation is available at [https://hexdocs.pm/kreuzberg](https://hexdocs.pm/kreuzberg)

## License

MIT License - see [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## Community

Join our [Discord community](https://discord.gg/pXxagNK2zN) for support, discussions, and updates.
