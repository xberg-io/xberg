# Kreuzberg for Ruby

{% include 'partials/badges.html.jinja' %}

{{ description }}

## What This Package Provides

- **Ruby-native extraction** — idiomatic Ruby objects over the shared Rust document engine.
- **Structured results** — text, tables, images, metadata, language detection, chunks, and warnings.
- **OCR support** — Tesseract and PaddleOCR through the same configuration model as other bindings.
- **Cross-binding parity** — output matches the Python, Node.js, Go, Java, .NET, PHP, Elixir, R, Dart, Swift, Zig, WASM, and C FFI packages.

## Installation

Add to your Gemfile:

```ruby
gem 'kreuzberg'
```

Then execute:

```bash
bundle install
```

Or install it directly:

```bash
gem install kreuzberg
```

## Quick Start

### Basic Usage

```ruby
require 'kreuzberg'

# Simple synchronous extraction
result = Kreuzberg.extract_file("document.pdf")
puts result.content
```

### Async Extraction

```ruby
require 'kreuzberg'

# Using Fiber for concurrency (Ruby 3.0+)
Fiber.new do
  result = Kreuzberg.extract_file_async("document.pdf")
  puts result.content
end.resume
```

### Batch Processing

```ruby
require 'kreuzberg'

files = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]

results = files.map { |file| Kreuzberg.extract_file(file) }

results.each do |result|
  puts "Content length: #{result.content.length}"
end
```

## Configuration

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true,
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file("document.pdf", config: config)
puts result.content
```

## OCR Support

### Tesseract Configuration

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng',
    tesseract_config: Kreuzberg::TesseractConfig.new(
      psm: 6,
      enable_table_detection: true
    )
  )
)

result = Kreuzberg.extract_file("scanned.pdf", config: config)
puts result.content
```

## Table Extraction

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    tesseract_config: Kreuzberg::TesseractConfig.new(
      enable_table_detection: true
    )
  )
)

result = Kreuzberg.extract_file("invoice.pdf", config: config)

result.tables.each_with_index do |table, index|
  puts "Table #{index}:"
  puts table.markdown
end
```

## Metadata Extraction

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file("document.pdf")

# PDF metadata
if result.metadata[:pdf]
  pdf_meta = result.metadata[:pdf]
  puts "Title: #{pdf_meta[:title]}"
  puts "Author: #{pdf_meta[:author]}"
  puts "Pages: #{pdf_meta[:page_count]}"
end

# Detected languages
puts "Languages: #{result.detected_languages}"

# Images
if result.images
  puts "Images found: #{result.images.count}"
end
```

## Text Chunking

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 1000,
    max_overlap: 200
  )
)

result = Kreuzberg.extract_file("long_document.pdf", config: config)

result.chunks.each_with_index do |chunk, index|
  puts "Chunk #{index}: #{chunk.length} characters"
end
```

## Password-Protected PDFs

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  pdf_options: Kreuzberg::PdfConfig.new(
    passwords: ["password1", "password2"]
  )
)

result = Kreuzberg.extract_file("protected.pdf", config: config)
puts result.content
```

## Language Detection

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true
  )
)

result = Kreuzberg.extract_file("multilingual.pdf", config: config)
puts "Detected languages: #{result.detected_languages}"
```

## API Reference

### Main Methods

- `Kreuzberg.extract_file(path, config: nil)` – Extract from file
- `Kreuzberg.extract_file_async(path, config: nil)` – Async extraction
- `Kreuzberg.extract_bytes(data, mime_type, config: nil)` – Extract from bytes
- `Kreuzberg.batch_extract_files(paths, config: nil)` – Batch processing

### Configuration Classes

- `ExtractionConfig` – Main configuration
- `OcrConfig` – OCR settings
- `TesseractConfig` – Tesseract-specific options
- `ChunkingConfig` – Text chunking settings
- `PdfConfig` – PDF-specific options
- `LanguageDetectionConfig` – Language detection settings

### Result Object

- `content` – Extracted text
- `metadata` – File metadata as Hash
- `tables` – Array of ExtractedTable objects
- `detected_languages` – Array of language codes
- `chunks` – Array of text chunks
- `images` – Array of extracted images (if enabled)

## System Requirements

### Ruby Version

- **Ruby 3.2.0 or higher** (including Ruby 4.x)
- Ruby 4.0+ is fully supported with no code changes required
- Magnus bindings compile successfully on all supported Ruby versions

### Required

- Rust toolchain (for native extension compilation)

### Optional

```bash
# Tesseract OCR
brew install tesseract          # macOS
sudo apt-get install tesseract-ocr  # Ubuntu/Debian
```

### Ruby 4.0 Compatibility

Kreuzberg is fully compatible with Ruby 4.0 (released December 25, 2025) and later. Key Ruby 4.0 features that work seamlessly:

- **Ruby Box** - Improved memory efficiency and performance
- **ZJIT Compiler** - Enhanced JIT compilation for faster execution
- **Ractor Improvements** - Better multi-threaded document processing
- **Set Promoted to Core** - No changes needed for Kreuzberg

All tests pass with Ruby 4.0.1 with 100% compatibility. The gem compiles without any breaking changes.

## Development

Clone and setup:

```bash
git clone https://github.com/kreuzberg-dev/kreuzberg.git
cd kreuzberg
bundle install
```

Run tests:

```bash
rake test
```

## Troubleshooting

### Native extension compilation error

Ensure build tools are installed:

```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential ruby-dev

# Windows (via RubyInstaller)
ridk install
```

### "Could not find Kreuzberg"

Reinstall the gem:

```bash
gem uninstall kreuzberg
gem install kreuzberg --no-document
```

### OCR not working

Verify Tesseract is installed:

```bash
tesseract --version
```

## Examples

### Process Directory of PDFs

```ruby
require 'kreuzberg'
require 'pathname'

Dir.glob("documents/*.pdf").each do |file|
  puts "Processing: #{file}"
  result = Kreuzberg.extract_file(file)
  puts "  Content length: #{result.content.length}"
  puts "  Language: #{result.detected_languages}"
end
```

### Extract and Parse Structured Data

```ruby
require 'kreuzberg'
require 'json'

result = Kreuzberg.extract_file("data.pdf")

# Parse content as JSON (if applicable)
begin
  data = JSON.parse(result.content)
  puts "Parsed data: #{data}"
rescue JSON::ParserError
  puts "Content is not JSON"
end
```

### Save Extracted Images

```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  images: Kreuzberg::ImageExtractionConfig.new(
    extract_images: true
  )
)

result = Kreuzberg.extract_file("document.pdf", config: config)

result.images&.each_with_index do |image, index|
  File.write("image_#{index}.png", image.data)
end
```

## Documentation

For comprehensive documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## Part of Kreuzberg.dev

- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

{{ license }} License - see [LICENSE](../../LICENSE) for details.
