# Ruby API Reference <span class="version-badge">v4.0.0</span>

Complete reference for the Kreuzberg Ruby API.

## Installation

Add to your `Gemfile`:

```ruby title="Terminal"
gem 'kreuzberg'
```

Or install directly:

```bash title="Terminal"
gem install kreuzberg
```

## Core Functions

### Kreuzberg.batch_extract_bytes()

Extract text from multiple byte arrays in parallel (asynchronous via Tokio runtime).

**Signature:**

```ruby title="Ruby"
Kreuzberg.batch_extract_bytes(data_list, mime_types, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

- `data_list` (Array<String>): Array of byte strings (binary data)
- `mime_types` (Array<String>): Array of MIME types corresponding to each byte array
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration applied to all items

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

---

### Kreuzberg.batch_extract_bytes_sync()

Extract text from multiple byte arrays in parallel (synchronous).

**Signature:**

```ruby title="Ruby"
Kreuzberg.batch_extract_bytes_sync(data_list, mime_types, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

Same as [`batch_extract_bytes()`](#kreuzbergbatch_extract_bytes).

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

---

### Kreuzberg.batch_extract_files()

Extract content from multiple files in parallel (asynchronous via Tokio runtime).

**Signature:**

```ruby title="Ruby"
Kreuzberg.batch_extract_files(paths, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

- `paths` (Array<String>): Array of file paths to extract
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration applied to all files

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

---

### Kreuzberg.batch_extract_files_sync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```ruby title="Ruby"
Kreuzberg.batch_extract_files_sync(paths, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

Same as [`batch_extract_files()`](#kreuzbergbatch_extract_files).

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

---

### Kreuzberg.extract_bytes()

Extract content from bytes (asynchronous via Tokio runtime).

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_bytes(data, mime_type, config: nil) -> Kreuzberg::Result
```

**Parameters:**

- `data` (String): Binary data to extract (Ruby String in binary encoding)
- `mime_type` (String): MIME type of the data (required for format detection)
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration

**Returns:**

- `Kreuzberg::Result`: Extraction result object

---

### Kreuzberg.extract_bytes_sync()

Extract content from bytes (synchronous).

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_bytes_sync(data, mime_type, config: nil) -> Kreuzberg::Result
```

**Parameters:**

Same as [`extract_bytes()`](#kreuzbergextract_bytes).

**Returns:**

- `Kreuzberg::Result`: Extraction result object

---

### Kreuzberg.extract_file()

Extract content from a file (asynchronous via Tokio runtime).

**Note:** Ruby doesn't have native async/await. This uses a blocking Tokio runtime internally. For background processing, use `extract_file_sync` in a Thread.

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_file(path, mime_type: nil, config: nil) -> Kreuzberg::Result
```

**Parameters:**

- `path` (String): Path to the file to extract
- `mime_type` (String, nil): Optional MIME type hint
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration

**Returns:**

- `Kreuzberg::Result`: Extraction result object

---

### Kreuzberg.extract_file_sync()

Extract content from a file (synchronous).

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_file_sync(path, mime_type: nil, config: nil) -> Kreuzberg::Result
```

**Parameters:**

Same as [`extract_file()`](#kreuzbergextract_file).

**Returns:**

- `Kreuzberg::Result`: Extraction result object

---

## Configuration

### Hash Configuration

The simplest way to configure extraction is using a Hash:

**Example:**

```ruby title="config.rb"
config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseract_config: {
      psm: 6,
      enable_table_detection: true
    }
  },
  pdf_options: {
    passwords: ['password1', 'password2'],
    extract_images: true,
    image_dpi: 300
  },
  language_detection: {
    enabled: true,
    confidence_threshold: 0.7
  }
}

result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

**Available Options:**

- `ocr` (Hash): OCR configuration
  - `backend` (String): OCR backend ("tesseract", "paddle-ocr"). Default: "tesseract"
  - `language` (String): Language code (ISO 639-3). Default: "eng"
  - `tesseract_config` (Hash): Tesseract-specific options
    - `psm` (Integer): Page segmentation mode (0-13). Default: 3
    - `oem` (Integer): OCR engine mode (0-3). Default: 3
    - `enable_table_detection` (Boolean): Enable table detection. Default: false
    - `tessedit_char_whitelist` (String): Character whitelist. Default: nil
    - `tessedit_char_blacklist` (String): Character blacklist. Default: nil
  - `paddle_ocr_config` (Hash): PaddleOCR-specific options
    - `model_tier` (String): <span class="version-badge">v4.5.0</span> Model tier: "server" (high accuracy, ~88MB det model) or "mobile" (lightweight, ~4.5MB det model). Default: "server"
    - `padding` (Integer): <span class="version-badge">v4.5.0</span> Padding in pixels (0-100) added around the image before detection. Default: 10
    - `use_angle_cls` (Boolean): Use angle classification. Default: false
    - `det_db_thresh` (Float): Detection threshold. Default: 0.3
    - `rec_batch_num` (Integer): Recognition batch size. Default: 6
  - `element_config` (Hash): OCR element extraction options
    - `include_elements` (Boolean): Include semantic elements. Default: false
    - `min_confidence` (Float): Minimum confidence threshold (0.0-1.0).

- `pdf_options` (Hash): PDF-specific options
  - `allow_single_column_tables` (Boolean): Allow extraction of single-column tables. Default: false
  - `extract_annotations` (Boolean): Extract PDF annotations. Default: false
  - `extract_images` (Boolean): Extract images from PDF. Default: false
  - `extract_metadata` (Boolean): Extract PDF metadata. Default: true
  - `font_config` (Hash): Custom font settings
    - `enabled` (Boolean): Enable custom font loading. Default: true
    - `custom_font_dirs` (Array<String>): List of directories to search for fonts.
  - `hierarchy` (Hash): Document hierarchy detection
    - `enabled` (Boolean): Enable structural detection. Default: true
    - `k_clusters` (Integer): Number of font clusters for detection. Default: 6
  - `passwords` (Array<String>): Passwords to try for encrypted PDFs. Default: nil
  - `top_margin_fraction` (Float): Fractional top margin to ignore (0.0-1.0).
  - `bottom_margin_fraction` (Float): Fractional bottom margin to ignore (0.0-1.0).

- `concurrency` (Hash): Concurrency configuration for extraction parallelization
  - `max_threads` (Integer): Maximum number of threads for parallel extraction. Default: nil (system default)

- `chunking` (Hash): Text chunking options
  - `enabled` (Boolean): Enable chunking. Default: true
  - `max_chars` (Integer): Maximum chunk size in characters. Default: 1000
  - `max_overlap` (Integer): Overlap between chunks. Default: 200
  - `preset` (String): Chunking preset ("balanced", "semantic", "fixed").
  - `embedding` (Hash): Embedding model for semantic chunking

- `language_detection` (Hash): Language detection options
  - `enabled` (Boolean): Enable language detection. Default: true
  - `min_confidence` (Float): Minimum confidence (0.0-1.0). Default: 0.5
  - `detect_multiple` (Boolean): Detect multiple languages per document. Default: false

- `image_extraction` (Hash): Image processing options
  - `extract_images` (Boolean): Enable image extraction. Default: true
  - `target_dpi` (Integer): Desired DPI for images. Default: 300
  - `max_image_dimension` (Integer): Maximum dimension for scaling. Default: 2000
  - `auto_adjust_dpi` (Boolean): Enable automatic DPI adjustment. Default: true

- `token_reduction` (Hash): Token reduction strategies
  - `mode` (String): Mode ("off", "light", "moderate", "aggressive"). Default: "off"
  - `preserve_important_words` (Boolean): Preserve contextually important tokens. Default: true

---

### Kreuzberg::Config::Extraction

!!! warning "Deprecated API"
    The `force_ocr` parameter has been deprecated in favor of the new `ocr` configuration object.

    **Old pattern (no longer supported):**
    ```ruby
    config = Kreuzberg::Config::Extraction.new(force_ocr: true)
    ```

    **New pattern:**
    ```ruby
    config = Kreuzberg::Config::Extraction.new(
      ocr: Kreuzberg::OcrConfig.new(backend: "tesseract")
    )
    ```

    The new approach provides more granular control over OCR behavior through the OcrConfig object.

Object-oriented configuration using Ruby classes.

**Example:**

```ruby title="config.rb"
config = Kreuzberg::Config::Extraction.new(
  # OCR now configured via ocr.backend
  ocr: Kreuzberg::Config::Ocr.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

---

## Results & Types

### Kreuzberg::Result

Result object returned by all extraction functions.

**Attributes:**

- `annotations` (Array<Hash>, nil): Extracted PDF annotations
- `chunks` (Array<Hash>, nil): Text chunks if chunking is enabled
- `content` (String): Extracted text content
- `detected_languages` (Array<String>, nil): Array of detected language codes
- `djot_content` (String, nil): Extracted content in Djot format
- `document` (Hash, nil): Hierarchical document structure
- `elements` (Array<Hash>, nil): Semantic elements (e.g., headings, paragraphs)
- `extracted_keywords` (Array<String>, nil): Keywords extracted from the text
- `images` (Array<Hash>, nil): Extracted images
- `metadata` (Hash): Document metadata (format-specific fields)
- `metadata_json` (String): Serialized metadata in JSON format
- `mime_type` (String): MIME type of the processed document
- `ocr_elements` (Array<Hash>, nil): Low-level OCR elements (words, lines)
- `pages` (Array<Hash>, nil): Per-page extracted content
- `processing_warnings` (Array<String>): Warnings encountered during extraction
- `quality_score` (Float, nil): Estimated quality score of the extraction
- `tables` (Array<Hash>): Array of extracted tables

**Example:**

```ruby title="basic_extraction.rb"
result = Kreuzberg.extract_file_sync("document.pdf")

puts "Content: #{result.content}"
puts "MIME type: #{result.mime_type}"
puts "Page count: #{result.metadata['page_count']}"
puts "Tables: #{result.tables.length}"

if result.detected_languages
  puts "Languages: #{result.detected_languages.join(', ')}"
end
```

#### pages

**Type**: `Array<Hash> | nil`

Per-page extracted content when page extraction is enabled via `PageConfig.extract_pages = true`.

Each page hash contains:
- `page_number` (Integer): 1-indexed page number
- `content` (String): Text content for that page
- `tables` (Array<Hash>): Tables on that page
- `images` (Array<Hash>): Images on that page

**Example:**

```ruby title="page_extraction.rb"
require 'kreuzberg'

config = {
  pages: {
    extract_pages: true
  }
}

result = Kreuzberg.extract_file_sync("document.pdf", config: config)

if result.pages
  result.pages.each do |page|
    puts "Page #{page['page_number']}:"
    puts "  Content: #{page['content'].length} chars"
    puts "  Tables: #{page['tables'].length}"
    puts "  Images: #{page['images'].length}"
  end
end
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```ruby title="iterate_pages.rb"
require 'kreuzberg'

config = {
  pages: {
    extract_pages: true,
    insert_page_markers: true,
    marker_format: "\n\n--- Page {page_num} ---\n\n"
  }
}

result = Kreuzberg.extract_file_sync("document.pdf", config: config)

# Access combined content with page markers
puts "Combined content with markers:"
puts result.content[0..500]
puts

# Access per-page content
if result.pages
  result.pages.each do |page|
    puts "Page #{page['page_number']}:"
    content_preview = page['content'][0..100]
    puts "  #{content_preview}..."
    puts "  Found #{page['tables'].length} table(s)" if page['tables'].length > 0
    puts "  Found #{page['images'].length} image(s)" if page['images'].length > 0
  end
end
```

---

### Metadata Hash

Document metadata with cross-platform standardized fields.

**Fields:**

- `authors` (Array<String>): Document authors.
- `created_at` (String, nil): Creation timestamp (ISO 8601).
- `created_by` (String, nil): Application that created the document.
- `custom` (Hash): Custom/extended metadata fields.
- `date` (String, nil): Primary document date.
- `format_type` (String): Format discriminator (e.g., "pdf", "docx").
- `keywords` (Array<String>): Document keywords/tags.
- `language` (String, nil): Primary document language code.
- `modified_at` (String, nil): Last modification timestamp.
- `page_count` (Integer, nil): Total number of pages.
- `producer` (String, nil): Software that produced the file.
- `subject` (String, nil): Document subject or summary.
- `title` (String, nil): Document title.

**Example:**

```ruby title="metadata.rb"
result = Kreuzberg.extract_file_sync("document.pdf")
metadata = result.metadata

puts "Title: #{metadata['title']}"
puts "Format: #{metadata['format_type']}"
puts "Pages: #{metadata['page_count']}"
```

---

### Table Hash

Extracted table structure.

**Fields:**

- `cells` (Array<Array<String>>): 2D array of table cells (rows x columns)
- `markdown` (String): Table rendered as markdown
- `page_number` (Integer): Page number where table was found

**Example:**

```ruby title="basic_extraction.rb"
result = Kreuzberg.extract_file_sync("invoice.pdf")

result.tables.each do |table|
  puts "Table on page #{table['page_number']}:"
  puts table['markdown']
  puts
end
```

---

### ChunkMetadata Hash

Metadata for a single text chunk.

**Fields:**

- `byte_start` (Integer): UTF-8 byte offset in content (inclusive)
- `byte_end` (Integer): UTF-8 byte offset in content (exclusive)
- `char_count` (Integer): Number of characters in chunk
- `token_count` (Integer, nil): Estimated token count (if configured)
- `first_page` (Integer, nil): First page this chunk appears on (1-indexed, only when page boundaries available)
- `last_page` (Integer, nil): Last page this chunk appears on (1-indexed, only when page boundaries available)
- `heading_context` (HeadingContext, nil): Heading hierarchy when using Markdown chunker. Only populated when chunker_type is set to markdown.

**Page tracking:** When `PageStructure.boundaries` is available and chunking is enabled, `first_page` and `last_page` are automatically calculated based on byte offsets.

**Example:**

```ruby title="chunk_metadata.rb"
require 'kreuzberg'

config = {
  chunking: {
    chunk_size: 500,
    chunk_overlap: 50
  },
  pages: {
    extract_pages: true
  }
}

result = Kreuzberg.extract_file_sync("document.pdf", config: config)

# Access chunk metadata for page tracking
result.chunks&.each do |chunk|
  meta = chunk['metadata']
  page_info = ""

  if meta['first_page']
    if meta['first_page'] == meta['last_page']
      page_info = " (page #{meta['first_page']})"
    else
      page_info = " (pages #{meta['first_page']}-#{meta['last_page']})"
    end
  end

  puts "Chunk [#{meta['byte_start']}:#{meta['byte_end']}]: #{meta['char_count']} chars#{page_info}"
end
```

---

## Error Handling

All Kreuzberg errors are raised as standardized Ruby exceptions.

**Exception Types:**

- `Kreuzberg::Errors::ConfigurationError`: Raised for invalid configuration options.
- `Kreuzberg::Errors::ExtractionError`: Base class for all extraction-time failures.
- `Kreuzberg::Errors::FFIError`: Raised for low-level communication failures with the core engine.
- `Kreuzberg::Errors::NotFoundError`: Raised when a file or resource is not found.
- `Kreuzberg::Errors::TimeoutError`: Raised when an extraction operation times out.
- `Kreuzberg::Errors::ValidationError`: Raised when results fail registered validations.

**Example:**

```ruby title="error_handling.rb"
begin
  result = Kreuzberg.extract_file_sync("document.pdf")
  puts result.content
rescue Kreuzberg::Errors::ValidationError => e
  puts "Validation failed: #{e.message}"
rescue Kreuzberg::Errors::ExtractionError => e
  puts "Extraction failed: #{e.message}"
rescue StandardError => e
  puts "Unexpected error: #{e.message}"
end
```

---

## Cache Management

### Kreuzberg.clear_cache()

Clear the extraction cache.

**Signature:**

```ruby title="Ruby"
Kreuzberg.clear_cache() -> nil
```

**Example:**

```ruby title="basic_extraction.rb"
Kreuzberg.clear_cache
```

**Note:** Cache clearing is currently not implemented in the FFI layer (TODO).

---

### Kreuzberg.cache_stats()

Get cache statistics.

**Signature:**

```ruby title="Ruby"
Kreuzberg.cache_stats() -> Hash
```

**Returns:**

- Hash with `:total_entries` (Integer) and `:total_size_bytes` (Integer)

**Example:**

```ruby title="extract_from_bytes.rb"
stats = Kreuzberg.cache_stats
puts "Cache entries: #{stats[:total_entries]}"
puts "Cache size: #{stats[:total_size_bytes]} bytes"
```

**Note:** Cache statistics are currently not implemented in the FFI layer (TODO).

---

## CLI Proxy

### Kreuzberg::CLIProxy

Wrapper for running the Kreuzberg CLI from Ruby.

**Example:**

```ruby title="basic_extraction.rb"
cli = Kreuzberg::CLIProxy.new

# Extract a file
output = cli.extract("document.pdf")
puts output

# Batch extract
output = cli.batch(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
puts output

# Detect MIME type
mime_type = cli.detect("unknown-file.bin")
puts "MIME type: #{mime_type}"
```

---

## API Proxy

### Kreuzberg::APIProxy

Wrapper for running the Kreuzberg API server from Ruby.

**Example:**

```ruby title="basic_extraction.rb"
api = Kreuzberg::APIProxy.new

# Start server (blocks)
api.start(host: "0.0.0.0", port: 8000)

# Or in a thread
thread = Thread.new do
  api.start(host: "127.0.0.1", port: 9000)
end

# Later...
thread.kill
```

---

## MCP Proxy

### Kreuzberg::MCPProxy

Wrapper for running the Kreuzberg MCP server from Ruby.

**Example:**

```ruby title="basic_extraction.rb"
mcp = Kreuzberg::MCPProxy.new

# Start MCP server (blocks)
mcp.start
```

---

## System Requirements

**Ruby:** 3.0 or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- macOS (x64, arm64)
- Windows (x64)

---

## Thread Safety

All Kreuzberg functions are thread-safe and can be called from multiple threads concurrently.

**Example:**

```ruby title="basic_extraction.rb"
threads = []

files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
files.each do |file|
  threads << Thread.new do
    result = Kreuzberg.extract_file_sync(file)
    puts "#{file}: #{result.content.length} characters"
  end
end

threads.each(&:join)
```

However, for better performance, use the batch API instead:

```ruby title="batch_processing.rb"
# Better approach
results = Kreuzberg.batch_extract_files_sync(files)
results.each_with_index do |result, i|
  puts "#{files[i]}: #{result.content.length} characters"
end
```

## Batch Extract from Bytes

### `batch_extract_bytes_sync(data_list, mime_types, config = nil)`

Extract text from multiple byte arrays synchronously.

**Parameters:**
- `data_list` (Array<String>): Array of byte strings (binary data)
- `mime_types` (Array<String>): Array of MIME types corresponding to each byte array
- `config` (Hash, optional): Extraction configuration

**Returns:** Array<Hash> - Array of extraction results

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

# Read multiple files into memory
pdf_data = File.binread('invoice.pdf')
docx_data = File.binread('report.docx')
xlsx_data = File.binread('data.xlsx')

data_list = [pdf_data, docx_data, xlsx_data]
mime_types = [
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
]

results = Kreuzberg.batch_extract_bytes_sync(data_list, mime_types)

results.each_with_index do |result, i|
  puts "Document #{i}:"
  puts "  Content length: #{result[:content].length}"
  puts "  Format: #{result[:metadata][:format_type]}"
end
```

**With OCR Configuration:**

```ruby title="with_ocr.rb"
config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  }
}

results = Kreuzberg.batch_extract_bytes_sync(data_list, mime_types, config)
```

### `batch_extract_bytes(data_list, mime_types, config = nil)`

Extract text from multiple byte arrays asynchronously.

**Parameters:** Same as `batch_extract_bytes_sync`

**Returns:** Array<Hash> - Array of extraction results

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

# Asynchronous batch extraction from bytes
data_list = files.map { |f| File.binread(f) }
mime_types = files.map { |f| 'application/pdf' }

results = Kreuzberg.batch_extract_bytes(data_list, mime_types, {
  chunking: {
    max_chars: 1024,
    max_overlap: 200
  }
})

results.each_with_index do |result, i|
  puts "Document #{i}: #{result[:content][0..100]}..."
  if result[:chunks]
    puts "  Chunks: #{result[:chunks].length}"
  end
end
```

## Extensibility

Kreuzberg's plugin system allows you to extend functionality with custom post-processors, validators, and OCR backends.

### Custom Post-Processors

Post-processors modify extraction results after document processing. They can enrich metadata, transform content, or add custom logic.

**Interface:**

```ruby title="Ruby"
class CustomPostProcessor
  def call(result)
    # Modify result and return it
    # result is a Hash with keys: :content, :metadata, :tables, etc.
    result
  end

  def processing_stage
    # Optional: Return :early, :default, or :late
    :default
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

class WordCountProcessor
  def call(result)
    # Skip if no content
    return result if result[:content].nil? || result[:content].empty?

    # Count words
    word_count = result[:content].split.length

    # Add to metadata
    result[:metadata] ||= {}
    result[:metadata][:word_count] = word_count
    result[:metadata][:processed_by] = 'WordCountProcessor'

    result
  end

  def processing_stage
    :early  # Run early in the pipeline
  end
end

# Register the processor
processor = WordCountProcessor.new
Kreuzberg.register_post_processor('word_count', 100, processor)

# Use in extraction
result = Kreuzberg.extract_file_sync('document.pdf')
puts "Words: #{result[:metadata][:word_count]}"

# Unregister when done
Kreuzberg.unregister_post_processor('word_count')
```

**Stateful Processor Example:**

```ruby title="custom_post_processor.rb"
class PdfMetadataExtractor
  def initialize
    @processed_count = 0
    @start_time = Time.now
  end

  def call(result)
    # Only process PDFs
    return result unless result[:metadata][:mime_type] == 'application/pdf'

    @processed_count += 1

    # Add processing metadata
    result[:metadata][:pdf_processed] = true
    result[:metadata][:processing_order] = @processed_count
    result[:metadata][:processing_timestamp] = Time.now.to_i
    result[:metadata][:elapsed_seconds] = (Time.now - @start_time).round(2)

    result
  end

  def statistics
    {
      processed_count: @processed_count,
      running_for: (Time.now - @start_time).round(2)
    }
  end
end

processor = PdfMetadataExtractor.new
Kreuzberg.register_post_processor('pdf_metadata', 90, processor)

# Process multiple files
files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']
results = Kreuzberg.batch_extract_files_sync(files)

puts "Statistics: #{processor.statistics}"
```

### Custom Validators

Validators check extraction results and raise errors if validation fails.

**Interface:**

```ruby title="Ruby"
class CustomValidator
  def call(result)
    # Check result
    # Raise Kreuzberg::Errors::ValidationError if invalid
    # Return nil if valid
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

class MinimumLengthValidator
  def initialize(min_length: 100)
    @min_length = min_length
  end

  def call(result)
    content = result[:content] || ''

    if content.strip.length < @min_length
      raise Kreuzberg::Errors::ValidationError,
            "Content too short: #{content.length} chars, minimum #{@min_length} required"
    end
  end
end

# Register validator
validator = MinimumLengthValidator.new(min_length: 50)
Kreuzberg.register_validator('min_length', 100, validator)

# This will raise if content is too short
begin
  result = Kreuzberg.extract_file_sync('short_document.txt')
rescue Kreuzberg::Errors::ValidationError => e
  puts "Validation failed: #{e.message}"
end
```

**Quality Score Validator:**

```ruby title="custom_validator.rb"
class QualityScoreValidator
  def call(result)
    quality_score = result.quality_score || 0.0

    if quality_score < 0.5
      raise Kreuzberg::Errors::ValidationError,
            format('Quality score %.2f below threshold 0.50', quality_score)
    end
  end
end

validator = QualityScoreValidator.new
Kreuzberg.register_validator('quality_check', 90, validator)
```

### Custom OCR Backends

Implement custom OCR backends for specialized OCR engines or cloud services.

**Interface:**

```ruby title="Ruby"
class CustomOcrBackend
  def name
    'custom-ocr'
  end

  def supported_languages
    ['eng', 'fra', 'deu']
  end

  def process_image(image_data, language)
    # Process image and return OCR result
    # image_data is a binary string
    # Return Hash with: :content, :mime_type, :metadata, :tables
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'
require 'net/http'
require 'json'

class CloudOcrBackend
  def name
    'cloud-ocr'
  end

  def supported_languages
    ['eng', 'fra', 'deu', 'spa', 'jpn', 'chi_sim']
  end

  def process_image(image_data, language)
    # Send to cloud OCR service
    uri = URI('https://api.example.com/ocr')
    request = Net::HTTP::Post.new(uri)
    request['Content-Type'] = 'image/jpeg'
    request['Accept-Language'] = language
    request.body = image_data

    response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) do |http|
      http.request(request)
    end

    # Parse response
    data = JSON.parse(response.body)

    {
      content: data['text'],
      mime_type: 'text/plain',
      metadata: {
        backend: 'cloud-ocr',
        language: language,
        confidence: data['confidence']
      },
      tables: []
    }
  end
end

# Register OCR backend
backend = CloudOcrBackend.new
Kreuzberg.register_ocr_backend(backend)

# Use with extraction
config = {
  ocr: {
    backend: 'cloud-ocr',
    language: 'eng'
  }
}

result = Kreuzberg.extract_file_sync('scanned.pdf', config)
```

### Plugin Management

**Methods:**

- `Kreuzberg.clear_ocr_backends`: Unregister all OCR backends.
- `Kreuzberg.clear_post_processors`: Unregister all post-processors.
- `Kreuzberg.clear_validators`: Unregister all validators.
- `Kreuzberg.list_ocr_backends`: List names of registered OCR backends.
- `Kreuzberg.list_post_processors`: List names of registered post-processors.
- `Kreuzberg.list_validators`: List names of registered validators.
- `Kreuzberg.register_ocr_backend(backend)`: Register a new OCR backend.
- `Kreuzberg.register_post_processor(name, priority, processor)`: Register a new post-processor.
- `Kreuzberg.register_validator(name, priority, validator)`: Register a new validator.
- `Kreuzberg.unregister_ocr_backend(name)`: Unregister an OCR backend by name.
- `Kreuzberg.unregister_post_processor(name)`: Unregister a post-processor by name.
- `Kreuzberg.unregister_validator(name)`: Unregister a validator by name.

**Example:**

```ruby title="plugins.rb"
# List available OCR backends
puts "OCR Backends: #{Kreuzberg.list_ocr_backends.join(', ')}"

# Register and unregister
Kreuzberg.unregister_post_processor('word_count')
```
