# Kreuzberg for Ruby

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://github.com/kreuzberg-dev/alef">
    <img src="https://img.shields.io/badge/bindings%20by-alef%20%D7%90-007ec6" alt="Bindings by alef">
  </a>
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://pypi.org/project/kreuzberg/">
    <img src="https://img.shields.io/pypi/v/kreuzberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/node">
    <img src="https://img.shields.io/npm/v/@kreuzberg/node?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/wasm">
    <img src="https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM&color=007ec6" alt="WASM">
  </a>
  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go/v5">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v5*" alt="Go">
  </a>
  <a href="https://www.nuget.org/packages/Kreuzberg/">
    <img src="https://img.shields.io/nuget/v/Kreuzberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/kreuzberg/kreuzberg">
    <img src="https://img.shields.io/packagist/v/kreuzberg/kreuzberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/kreuzberg">
    <img src="https://img.shields.io/gem/v/kreuzberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://pub.dev/packages/kreuzberg">
    <img src="https://img.shields.io/pub/v/kreuzberg?label=Dart&color=007ec6" alt="Dart">
  </a>
  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg-android">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg-android?label=Kotlin&color=007ec6" alt="Kotlin">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/swift">
    <img src="https://img.shields.io/badge/Swift-SPM-007ec6" alt="Swift">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/zig">
    <img src="https://img.shields.io/badge/Zig-package-007ec6" alt="Zig">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-ghcr.io-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/charts%2Fkreuzberg">
    <img src="https://img.shields.io/badge/Helm-ghcr.io-007ec6?logo=helm&logoColor=white" alt="Helm">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-Elastic--2.0-007ec6" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-007ec6" alt="Documentation">
  </a>
  <a href="https://huggingface.co/Kreuzberg">
    <img src="https://img.shields.io/badge/Hugging%20Face-Kreuzberg-007ec6" alt="Hugging Face">
  </a>
</div>

<div align="center" style="margin: 24px 0 0;">
  <a href="https://kreuzberg.dev">
    <img width="1128" height="191" alt="Kreuzberg" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />
  </a>
</div>

<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="32" src="https://img.shields.io/badge/Discord-Join%20our%20community-007ec6?logo=discord&logoColor=white" alt="Join Discord">
  </a>
  <a href="https://docs.kreuzberg.dev/demo.html">
    <img height="32" src="https://img.shields.io/badge/Live%20Demo-Open-007ec6?logo=webassembly&logoColor=white" alt="Live Demo">
  </a>
</div>

Extract text, tables, images, and metadata from 91+ file formats and 248 programming languages including PDF, Office documents, and images. Ruby bindings with idiomatic Ruby API and native performance.

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

Elastic-2.0 License - see [LICENSE](../../LICENSE) for details.
