# Xberg for Ruby

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
gem 'xberg'
```

Then execute:

```bash
bundle install
```

Or install it directly:

```bash
gem install xberg
```

## Quick Start

### Basic Usage

```ruby
require 'xberg'

# Simple synchronous extraction
result = Xberg.extract(Xberg::ExtractInput.file("document.pdf"))
puts result.content
```

### Async Extraction

```ruby
require 'xberg'

# Using Fiber for concurrency (Ruby 3.0+)
Fiber.new do
  result = Xberg.extract(Xberg::ExtractInput.file("document.pdf"))
  puts result.content
end.resume
```

### Batch Processing

```ruby
require 'xberg'

inputs = [
  Xberg::ExtractInput.file("doc1.pdf"),
  Xberg::ExtractInput.file("doc2.docx"),
  Xberg::ExtractInput.file("doc3.xlsx"),
]

results = Xberg.extract_batch(inputs)

results.each do |result|
  puts "Content length: #{result.content.length}"
end
```

## Configuration

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true,
  ocr: Xberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

result = Xberg.extract("document.pdf", config: config)
puts result.content
```

## OCR Support

### Tesseract Configuration

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng',
    tesseract_config: Xberg::TesseractConfig.new(
      psm: 6,
      enable_table_detection: true
    )
  )
)

result = Xberg.extract("scanned.pdf", config: config)
puts result.content
```

## Table Extraction

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  ocr: Xberg::OcrConfig.new(
    backend: 'tesseract',
    tesseract_config: Xberg::TesseractConfig.new(
      enable_table_detection: true
    )
  )
)

result = Xberg.extract("invoice.pdf", config: config)

result.tables.each_with_index do |table, index|
  puts "Table #{index}:"
  puts table.markdown
end
```

## Metadata Extraction

```ruby
require 'xberg'

result = Xberg.extract("document.pdf")

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
require 'xberg'

config = Xberg::ExtractionConfig.new(
  chunking: Xberg::ChunkingConfig.new(
    max_chars: 1000,
    max_overlap: 200
  )
)

result = Xberg.extract("long_document.pdf", config: config)

result.chunks.each_with_index do |chunk, index|
  puts "Chunk #{index}: #{chunk.length} characters"
end
```

## Password-Protected PDFs

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  pdf_options: Xberg::PdfConfig.new(
    passwords: ["password1", "password2"]
  )
)

result = Xberg.extract("protected.pdf", config: config)
puts result.content
```

## Language Detection

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true
  )
)

result = Xberg.extract("multilingual.pdf", config: config)
puts "Detected languages: #{result.detected_languages}"
```

## API Reference

### Main Methods

- `Xberg.extract(input, config: nil)` – Extract one `ExtractInput`
- `Xberg.extract_batch(inputs, config: nil)` – Batch processing
- `Xberg::ExtractInput.file(path, mime_type: nil, **overrides)` – File path input
- `Xberg::ExtractInput.bytes(data, mime_type:, **overrides)` – In-memory bytes input

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

Xberg is fully compatible with Ruby 4.0 (released December 25, 2025) and later. Key Ruby 4.0 features that work seamlessly:

- **Ruby Box** - Improved memory efficiency and performance
- **ZJIT Compiler** - Enhanced JIT compilation for faster execution
- **Ractor Improvements** - Better multi-threaded document processing
- **Set Promoted to Core** - No changes needed for Xberg

All tests pass with Ruby 4.0.1 with 100% compatibility. The gem compiles without any breaking changes.

## Development

Clone and setup:

```bash
git clone https://github.com/xberg-io/xberg.git
cd xberg
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

### "Could not find Xberg"

Reinstall the gem:

```bash
gem uninstall xberg
gem install xberg --no-document
```

### OCR not working

Verify Tesseract is installed:

```bash
tesseract --version
```

## Examples

### Process Directory of PDFs

```ruby
require 'xberg'
require 'pathname'

Dir.glob("documents/*.pdf").each do |file|
  puts "Processing: #{file}"
  result = Xberg.extract(file)
  puts "  Content length: #{result.content.length}"
  puts "  Language: #{result.detected_languages}"
end
```

### Extract and Parse Structured Data

```ruby
require 'xberg'
require 'json'

result = Xberg.extract("data.pdf")

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
require 'xberg'

config = Xberg::ExtractionConfig.new(
  images: Xberg::ImageExtractionConfig.new(
    extract_images: true
  )
)

result = Xberg.extract("document.pdf", config: config)

result.images&.each_with_index do |image, index|
  File.write("image_#{index}.png", image.data)
end
```

## Documentation

For comprehensive documentation, visit [https://xberg.io](https://xberg.io)

## Part of Xberg.dev

- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

{{ license }} License - see [LICENSE](../../LICENSE) for details.
