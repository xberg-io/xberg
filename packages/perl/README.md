# Kreuzberg Perl Bindings

Perl bindings for the Kreuzberg document intelligence framework.

## Installation

### From CPAN (when published)

```bash
cpanm Kreuzberg
```

### From Source

```bash
cd packages/perl
perl Makefile.PL
make
make test
make install
```

### Prerequisites

1. **Perl 5.20+** is required
2. **FFI::Platypus 2.00+** for FFI bindings
3. **The Kreuzberg FFI library** (libkreuzberg_ffi) must be installed

## Library Installation

The Kreuzberg FFI library must be installed on your system. You can:

1. Build from source:
   ```bash
   cargo build --release --package kreuzberg-ffi
   ```

2. Or set the `KREUZBERG_LIB_PATH` environment variable to point to the library:
   ```bash
   export KREUZBERG_LIB_PATH=/path/to/libkreuzberg_ffi.so  # Linux
   export KREUZBERG_LIB_PATH=/path/to/libkreuzberg_ffi.dylib  # macOS
   ```

## Usage

### Basic Extraction

```perl
use Kreuzberg;

# Extract text from a file
my $result = Kreuzberg::extract_file('/path/to/document.pdf');
print $result->content;
print $result->mime_type;
```

### With Configuration

```perl
use Kreuzberg;
use Kreuzberg::Config;

my $config = Kreuzberg::Config->new(
    enable_ocr      => 1,
    ocr_language    => 'eng',
    extract_tables  => 1,
);

my $result = Kreuzberg::extract_file('/path/to/image.png', $config);
print $result->content;
```

### Extract from Bytes

```perl
use Kreuzberg;

my $bytes = read_file_content('/path/to/file.pdf');
my $result = Kreuzberg::extract_bytes($bytes, 'application/pdf');
print $result->content;
```

### Batch Extraction

```perl
use Kreuzberg;

my @results = Kreuzberg::batch_extract_files([
    '/path/to/file1.pdf',
    '/path/to/file2.docx',
    '/path/to/file3.txt',
]);

for my $result (@results) {
    print "Content: ", $result->content, "\n";
    print "MIME: ", $result->mime_type, "\n\n";
}
```

### MIME Type Detection

```perl
use Kreuzberg;

my $mime = Kreuzberg::detect_mime_type('/path/to/file');
print "MIME type: $mime\n";

# Validate MIME type support
if (Kreuzberg::validate_mime_type('application/pdf')) {
    print "PDF is supported\n";
}
```

### Accessing Result Properties

```perl
my $result = Kreuzberg::extract_file('/path/to/document.pdf');

# Basic properties
print $result->content;      # Extracted text
print $result->mime_type;    # Detected MIME type
print $result->language;     # Detected language

# Structured data (returns decoded JSON)
my $metadata = $result->metadata;
my $tables = $result->tables;
my $chunks = $result->chunks;
my $pages = $result->pages;

# Convert to hash
my $hash = $result->to_hash;
```

## Configuration Options

| Option | Type | Description |
|--------|------|-------------|
| `enable_ocr` | bool | Enable OCR for images |
| `ocr_language` | string | OCR language code (e.g., 'eng', 'deu') |
| `ocr_backend` | string | OCR backend ('tesseract', 'easyocr', 'paddleocr') |
| `extract_tables` | bool | Extract tables from documents |
| `extract_images` | bool | Extract images from documents |
| `enable_chunking` | bool | Enable text chunking |
| `chunk_size` | int | Maximum chunk size in characters |
| `chunk_overlap` | int | Overlap between chunks |
| `detect_language` | bool | Enable language detection |
| `pdf_dpi` | int | DPI for PDF rendering |
| `output_format` | string | Output format ('text', 'markdown', 'html') |

## Supported Formats

Kreuzberg supports 50+ document formats including:

- **Documents**: PDF, DOCX, ODT, RTF
- **Spreadsheets**: XLSX, XLS, ODS, CSV
- **Presentations**: PPTX, PPT, ODP
- **Images**: PNG, JPEG, TIFF, WebP, BMP (with OCR)
- **Text**: TXT, Markdown, HTML, XML
- **Email**: EML, MSG
- **Archives**: ZIP, TAR, 7Z
- **And more...**

## API Reference

See the POD documentation:

```bash
perldoc Kreuzberg
perldoc Kreuzberg::Config
perldoc Kreuzberg::Result
```

## Testing

```bash
perl Makefile.PL
make test
```

## License

MIT License - see the [LICENSE](../../LICENSE) file.

## Links

- [Main Documentation](https://kreuzberg.dev/)
- [GitHub Repository](https://github.com/kreuzberg-dev/kreuzberg)
- [Issue Tracker](https://github.com/kreuzberg-dev/kreuzberg/issues)
