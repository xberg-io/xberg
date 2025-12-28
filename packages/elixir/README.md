# Kreuzberg for Elixir

High-performance document intelligence library for Elixir, powered by Rust.

## Installation

Add `kreuzberg` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:kreuzberg, "~> 4.0.0-rc"}
  ]
end
```

## Usage

```elixir
# Read a PDF file
pdf_data = File.read!("document.pdf")

# Extract content
{:ok, result} = Kreuzberg.extract(pdf_data, :pdf)

# Or use the bang variant
result = Kreuzberg.extract!(pdf_data, :pdf)
```

## Supported Formats

- PDF
- DOCX
- HTML
- Markdown
- Plain Text

## Features

- High-performance extraction powered by Rust
- OCR support for scanned documents
- Image extraction
- Metadata extraction
- Custom post-processors and validators

## Development Status

This package is currently under active development. Basic extraction functionality is being implemented.

## License

MIT
