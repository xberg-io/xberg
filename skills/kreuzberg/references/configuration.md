# Configuration Reference

Kreuzberg uses a hierarchical configuration system supporting multiple formats and auto-discovery mechanisms. This reference covers all available configuration options, field names across programming languages, and loading strategies.

## Supported Formats

Kreuzberg configurations can be defined in three formats:

- **TOML** (recommended): `kreuzberg.toml`
- **YAML**: `kreuzberg.yaml`
- **JSON**: `kreuzberg.json`

All formats support the same schema and configuration options.

## Auto-Discovery

When no configuration file is explicitly specified, Kreuzberg searches for configuration files in the following order:

1. Current working directory: `kreuzberg.toml`, `kreuzberg.yaml`, `kreuzberg.json`
2. Parent directories (recursively up the tree, same file name pattern)

The first matching configuration file is loaded.

## Programmatic Loading

### Python

```python
from kreuzberg import ExtractionConfig

# Load from explicit path
config = ExtractionConfig.from_file("kreuzberg.toml")

# Auto-discover configuration
config = ExtractionConfig.discover()
```

### Node.js / TypeScript

```typescript
import { ExtractionConfig } from '@kreuzberg/node';

// Load from explicit path
const config = ExtractionConfig.fromFile('kreuzberg.toml');

// Auto-discover configuration
const config = ExtractionConfig.discover();
```

### CLI

```bash
# Explicit configuration file
kreuzberg extract --config kreuzberg.toml document.pdf

# Auto-discovery (searches default locations)
kreuzberg extract document.pdf
```

## Configuration Schema

The complete TOML schema with all available sections and options:

### Top-Level Options

```toml
use_cache = true
enable_quality_processing = true
force_ocr = false
output_format = "markdown"
result_format = "text"
max_concurrent_extractions = 4
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `use_cache` | boolean | `true` | Enable caching of extraction results |
| `enable_quality_processing` | boolean | `true` | Enable post-processing for output quality |
| `force_ocr` | boolean | `false` | Force OCR processing even for searchable PDFs |
| `output_format` | string | `"markdown"` | Output format (markdown, html, text) |
| `result_format` | string | `"text"` | Result format for structured output |
| `max_concurrent_extractions` | integer | `4` | Maximum concurrent document extractions |

### OCR Configuration

```toml
[ocr]
backend = "tesseract"
language = "eng"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `backend` | string | `"tesseract"` | OCR backend (currently tesseract) |
| `language` | string | `"eng"` | ISO 639-3 language code (eng, deu, fra, etc.) |

#### Tesseract Configuration

```toml
[ocr.tesseract_config]
psm = 3
oem = 3
min_confidence = 0.0
output_format = "text"
enable_table_detection = false
table_min_confidence = 0.5
table_column_threshold = 50
table_row_threshold_ratio = 0.5
use_cache = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `psm` | integer | `3` | Page Segmentation Mode (0-13) |
| `oem` | integer | `3` | OCR Engine Mode (0-3) |
| `min_confidence` | float | `0.0` | Minimum OCR confidence threshold (0.0-1.0) |
| `output_format` | string | `"text"` | Output format from OCR |
| `enable_table_detection` | boolean | `false` | Enable table detection during OCR |
| `table_min_confidence` | float | `0.5` | Minimum confidence for table cells |
| `table_column_threshold` | integer | `50` | Pixel threshold for column detection |
| `table_row_threshold_ratio` | float | `0.5` | Row height ratio threshold |
| `use_cache` | boolean | `true` | Cache OCR results |

#### Tesseract Preprocessing

```toml
[ocr.tesseract_config.preprocessing]
target_dpi = 300
auto_rotate = true
deskew = true
denoise = true
contrast_enhance = true
binarization_method = "otsu"
invert_colors = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `target_dpi` | integer | `300` | Target DPI for preprocessing |
| `auto_rotate` | boolean | `true` | Automatically detect and correct page rotation |
| `deskew` | boolean | `true` | Correct skewed pages |
| `denoise` | boolean | `true` | Remove noise from images |
| `contrast_enhance` | boolean | `true` | Enhance image contrast |
| `binarization_method` | string | `"otsu"` | Method for image binarization |
| `invert_colors` | boolean | `false` | Invert image colors if needed |

### PDF Options

```toml
[pdf_options]
extract_images = true
extract_metadata = true

[pdf_options.hierarchy]
enabled = true
k_clusters = 6
include_bbox = true
ocr_coverage_threshold = 0.5
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `extract_images` | boolean | `true` | Extract images from PDF documents |
| `extract_metadata` | boolean | `true` | Extract PDF metadata |
| `hierarchy.enabled` | boolean | `true` | Enable PDF hierarchy extraction (v4.0.0+) |
| `hierarchy.k_clusters` | integer | `6` | Number of clusters for hierarchy detection |
| `hierarchy.include_bbox` | boolean | `true` | Include bounding boxes in hierarchy |
| `hierarchy.ocr_coverage_threshold` | float | `0.5` | OCR coverage threshold for hierarchy (0.0-1.0) |

### Image Processing

```toml
[images]
extract_images = true
target_dpi = 300
max_image_dimension = 4096
auto_adjust_dpi = true
min_dpi = 72
max_dpi = 600
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `extract_images` | boolean | `true` | Extract images from documents |
| `target_dpi` | integer | `300` | Target DPI for image processing |
| `max_image_dimension` | integer | `4096` | Maximum image dimension in pixels |
| `auto_adjust_dpi` | boolean | `true` | Automatically adjust DPI based on image size |
| `min_dpi` | integer | `72` | Minimum DPI threshold |
| `max_dpi` | integer | `600` | Maximum DPI threshold |

### Chunking Configuration

```toml
[chunking]
max_chars = 1000
max_overlap = 200

[chunking.embedding]
batch_size = 32
normalize = true
show_download_progress = true
cache_dir = "~/.cache/kreuzberg/embeddings"

[chunking.embedding.model]
type = "preset"
name = "balanced"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_chars` | integer | `1000` | Maximum characters per chunk |
| `max_overlap` | integer | `200` | Overlap between consecutive chunks |
| `embedding.batch_size` | integer | `32` | Batch size for embedding generation |
| `embedding.normalize` | boolean | `true` | Normalize embeddings to unit length |
| `embedding.show_download_progress` | boolean | `true` | Show progress when downloading models |
| `embedding.cache_dir` | string | `"~/.cache/kreuzberg/embeddings"` | Directory for caching embeddings |
| `embedding.model.type` | string | `"preset"` | Model type: preset, fastembed, or custom |
| `embedding.model.name` | string | `"balanced"` | Preset model name (balanced, fast, accurate, multilingual) |
| `embedding.model.model` | string | | FastEmbed model identifier |
| `embedding.model.model_id` | string | | Custom HuggingFace model ID |
| `embedding.model.dimensions` | integer | | Embedding dimensions |

### Keywords Configuration

```toml
[keywords]
algorithm = "yake"
max_keywords = 10
min_score = 0.0
ngram_range = [1, 3]
language = "en"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `algorithm` | string | `"yake"` | Keyword extraction algorithm (yake or rake) |
| `max_keywords` | integer | `10` | Maximum keywords to extract |
| `min_score` | float | `0.0` | Minimum relevance score for keywords |
| `ngram_range` | array | `[1, 3]` | N-gram size range [min, max] |
| `language` | string | `"en"` | Language code for keyword extraction |

### Token Reduction

```toml
[token_reduction]
mode = "off"
preserve_important_words = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | string | `"off"` | Mode: off, aggressive, moderate, minimal |
| `preserve_important_words` | boolean | `true` | Preserve important words during reduction |

### Language Detection

```toml
[language_detection]
enabled = true
min_confidence = 0.8
detect_multiple = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable automatic language detection |
| `min_confidence` | float | `0.8` | Minimum confidence threshold for detection |
| `detect_multiple` | boolean | `false` | Detect multiple languages in document |

### Post-Processor

```toml
[postprocessor]
enabled = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable post-processing of extracted content |

## Naming Conventions

Kreuzberg uses consistent naming conventions across different contexts:

| Context | Convention | Example |
|---------|-----------|---------|
| Python | snake_case | `max_chars`, `pdf_options`, `use_cache` |
| Node.js / TypeScript | camelCase | `maxChars`, `pdfOptions`, `useCache` |
| Rust | snake_case | `max_chars`, `pdf_options`, `use_cache` |
| TOML / YAML / JSON | snake_case | `max_chars`, `pdf_options`, `use_cache` |
| CLI flags | kebab-case | `--max-chars`, `--pdf-options`, `--use-cache` |

When switching between languages, apply the appropriate conversion:
- Python → Node.js: `snake_case` to `camelCase`
- CLI → Python: `kebab-case` to `snake_case`
- TOML → Python: No conversion needed (both use `snake_case`)

## Environment Variables

The following environment variables can override configuration:

| Variable | Purpose | Example |
|----------|---------|---------|
| `KREUZBERG_HOST` | Server bind address (serve command) | `127.0.0.1` |
| `KREUZBERG_PORT` | Server port (serve command) | `8080` |

## Configuration Merging

Configuration sources are merged in priority order (highest to lowest):

1. **CLI flags** (highest priority)
2. **Inline JSON configuration** (programmatic)
3. **Configuration file** (lowest priority)

Later sources override earlier ones. For example, a CLI flag `--max-chars 2000` overrides `max_chars = 1000` in the configuration file.

## Example Configurations

### Minimal Configuration

```toml
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"
```

### High-Quality PDF Extraction

```toml
use_cache = true
enable_quality_processing = true
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng"

[ocr.tesseract_config]
psm = 3
oem = 3
enable_table_detection = true
table_min_confidence = 0.7

[pdf_options]
extract_images = true
extract_metadata = true

[pdf_options.hierarchy]
enabled = true
k_clusters = 6

[images]
extract_images = true
target_dpi = 300
```

### Semantic Search Configuration

```toml
[chunking]
max_chars = 800
max_overlap = 150

[chunking.embedding]
batch_size = 32
normalize = true
cache_dir = "~/.cache/kreuzberg/embeddings"

[chunking.embedding.model]
type = "preset"
name = "accurate"

[keywords]
algorithm = "yake"
max_keywords = 15
```

## Field Name Reference

Critical field names to use in configuration files:

- `max_chars` (NOT `max_characters`)
- `max_overlap` (NOT `overlap`)
- `table_min_confidence`
- `table_column_threshold`
- `table_row_threshold_ratio`
- `ocr_coverage_threshold`
- `k_clusters`
- `include_bbox`
- `enable_table_detection`
- `auto_rotate`
- `auto_adjust_dpi`
- `show_download_progress`
- `min_confidence`
- `detect_multiple`

Always verify field names against the source configuration file when adding new options.
