# Xberg for Ruby

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://github.com/xberg-io/alef">
    <img src="https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6" alt="Bindings">
  </a>
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/xberg">
    <img src="https://img.shields.io/crates/v/xberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://pypi.org/project/xberg/">
    <img src="https://img.shields.io/pypi/v/xberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@xberg-io/xberg">
    <img src="https://img.shields.io/npm/v/@xberg-io/xberg?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@xberg-io/xberg-wasm">
    <img src="https://img.shields.io/npm/v/@xberg-io/xberg-wasm?label=WASM&color=007ec6" alt="WASM">
  </a>
  <a href="https://central.sonatype.com/artifact/io.xberg/xberg">
    <img src="https://img.shields.io/maven-central/v/io.xberg/xberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/go">
    <img src="https://img.shields.io/github/v/tag/xberg-io/xberg?label=Go&color=007ec6&filter=v1*" alt="Go">
  </a>
  <a href="https://www.nuget.org/packages/Xberg/">
    <img src="https://img.shields.io/nuget/v/Xberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/xberg-io/xberg">
    <img src="https://img.shields.io/packagist/v/xberg-io/xberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/xberg">
    <img src="https://img.shields.io/gem/v/xberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://hex.pm/packages/xberg">
    <img src="https://img.shields.io/hexpm/v/xberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://xberg-io.r-universe.dev/xberg">
    <img src="https://img.shields.io/badge/R-xberg-007ec6" alt="R">
  </a>
  <a href="https://pub.dev/packages/xberg">
    <img src="https://img.shields.io/pub/v/xberg?label=Dart&color=007ec6" alt="Dart">
  </a>
  <a href="https://central.sonatype.com/artifact/io.xberg/xberg-android">
    <img src="https://img.shields.io/maven-central/v/io.xberg/xberg-android?label=Kotlin&color=007ec6" alt="Kotlin">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/swift">
    <img src="https://img.shields.io/badge/Swift-SPM-007ec6" alt="Swift">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/zig">
    <img src="https://img.shields.io/badge/Zig-package-007ec6" alt="Zig">
  </a>
  <a href="https://github.com/xberg-io/xberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI">
  </a>
  <a href="https://github.com/xberg-io/xberg/pkgs/container/xberg">
    <img src="https://img.shields.io/badge/Docker-ghcr.io-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <!-- Project Info -->
  <a href="https://github.com/xberg-io/xberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License">
  </a>
  <a href="https://docs.xberg.io">
    <img src="https://img.shields.io/badge/Docs-xberg-007ec6" alt="Documentation">
  </a>
  <a href="https://huggingface.co/xberg-io">
    <img src="https://img.shields.io/badge/Hugging%20Face-Xberg-007ec6" alt="Hugging Face">
  </a>
</div>

<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white" alt="Join Discord">
  </a>
  <a href="https://docs.xberg.io/demo.html">
    <img height="22" src="https://img.shields.io/badge/Live%20Demo-Open-007ec6?logo=webassembly&logoColor=white" alt="Live Demo">
  </a>
  <a href="https://github.com/xberg-io/xberg/stargazers">
    <img height="22" src="https://img.shields.io/github/stars/xberg-io/xberg?style=social" alt="GitHub Stars">
  </a>
</div>

Extract text, tables, images, metadata, and code intelligence from 96 file formats and 306 programming languages including PDF, Office documents, images, and audio/video transcripts where native transcription is available. Ruby bindings with idiomatic Ruby API and native performance.

## What This Package Provides

- **Ruby-native extraction** — idiomatic Ruby objects over the shared Rust document engine.
- **Structured results** — an `ExtractionResult` envelope with `ExtractedDocument` items, errors, and summary counts.
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "document.pdf")
output = Xberg.extract(input, Xberg::ExtractionConfig.new)
document = output.results.first

puts document.content
puts "Results: #{output.summary.results}"
```

### Batch Processing

```ruby
require 'xberg'

bytes = File.binread("doc3.txt")
inputs = [
  Xberg::ExtractInput.new(kind: "uri", uri: "doc1.pdf"),
  Xberg::ExtractInput.new(kind: "uri", uri: "doc2.docx"),
  Xberg::ExtractInput.new(
    kind: "bytes",
    bytes: bytes,
    mime_type: "text/plain",
    filename: "doc3.txt"
  ),
]

output = Xberg.extract_batch(inputs, Xberg::ExtractionConfig.new)

output.results.each do |document|
  puts "Content length: #{document.content.length}"
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "document.pdf")
output = Xberg.extract(input, config)
document = output.results.first

puts document.content
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "scanned.pdf")
output = Xberg.extract(input, config)
document = output.results.first

puts document.content
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "invoice.pdf")
output = Xberg.extract(input, config)
document = output.results.first

document.tables.each_with_index do |table, index|
  puts "Table #{index}:"
  puts table.markdown
end
```

## Metadata Extraction

```ruby
require 'xberg'

input = Xberg::ExtractInput.new(kind: "uri", uri: "document.pdf")
output = Xberg.extract(input, Xberg::ExtractionConfig.new)
document = output.results.first

metadata = document.metadata
puts "Title: #{metadata.title}" if metadata&.title
if metadata&.authors
  puts "Authors: #{metadata.authors.join(', ')}"
end

puts "Languages: #{document.detected_languages}"

if document.images
  puts "Images found: #{document.images.count}"
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "long_document.pdf")
output = Xberg.extract(input, config)
document = output.results.first

document.chunks.each_with_index do |chunk, index|
  puts "Chunk #{index}: #{chunk.content.length} characters"
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "protected.pdf")
output = Xberg.extract(input, config)
document = output.results.first

puts document.content
```

## Language Detection

```ruby
require 'xberg'

config = Xberg::ExtractionConfig.new(
  language_detection: Xberg::LanguageDetectionConfig.new(
    enabled: true
  )
)

input = Xberg::ExtractInput.new(kind: "uri", uri: "multilingual.pdf")
output = Xberg.extract(input, config)
document = output.results.first

puts "Detected languages: #{document.detected_languages}"
```

## API Reference

### Main Methods

- `Xberg.extract(input, config)` – Extract one URI or bytes input.
- `Xberg.extract_batch(inputs, config)` – Extract multiple URI or bytes inputs.
- `Xberg::ExtractInput.new(kind: "uri", uri: "document.pdf")` – Local path, `file://`, or HTTP(S) URI input.
- `Xberg::ExtractInput.new(kind: "bytes", bytes: data, mime_type: "application/pdf")` – In-memory bytes input.

### Configuration Classes

- `ExtractionConfig` – Main configuration
- `OcrConfig` – OCR settings
- `TesseractConfig` – Tesseract-specific options
- `ChunkingConfig` – Text chunking settings
- `PdfConfig` – PDF-specific options
- `LanguageDetectionConfig` – Language detection settings

### Result Types

- `ExtractionResult` – Envelope with `results`, `errors`, and `summary`.
- `ExtractedDocument` – Per-document item at `output.results.first` with content, metadata, tables, and chunks.
- `Table` – Table with `cells`, `markdown`, and `page_number`.
- `Metadata` – Typed document metadata.

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
  input = Xberg::ExtractInput.new(kind: "uri", uri: file)
  output = Xberg.extract(input, Xberg::ExtractionConfig.new)
  document = output.results.first

  puts "  Content length: #{document.content.length}"
  puts "  Language: #{document.detected_languages}"
end
```

### Extract and Parse Structured Data

```ruby
require 'xberg'
require 'json'

input = Xberg::ExtractInput.new(kind: "uri", uri: "data.pdf")
output = Xberg.extract(input, Xberg::ExtractionConfig.new)
document = output.results.first

# Parse content as JSON (if applicable)
begin
  data = JSON.parse(document.content)
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

input = Xberg::ExtractInput.new(kind: "uri", uri: "document.pdf")
output = Xberg.extract(input, config)
document = output.results.first

document.images&.each_with_index do |image, index|
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

MIT License - see [LICENSE](../../LICENSE) for details.
