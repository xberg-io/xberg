# PHP Benchmark Integration

This document describes the PHP benchmarking integration for the Kreuzberg benchmark harness.

## Overview

PHP has been integrated into the benchmark harness to enable performance comparisons with other language bindings (Python, Ruby, Node.js, Go, Java, C#, etc.).

## Files Created

### 1. Benchmark Extraction Script
**Location**: `/tools/benchmark-harness/scripts/kreuzberg_extract.php`

This script provides a standardized interface for the benchmark harness to extract documents using the PHP bindings.

**Supported modes:**
- `sync`: Single file extraction using `Kreuzberg\extract_file()`
- `batch`: Batch extraction using `Kreuzberg\batch_extract_files()`

**Usage:**
```bash
php kreuzberg_extract.php sync /path/to/document.pdf
php kreuzberg_extract.php batch /path/to/doc1.pdf /path/to/doc2.docx
```

**Debug mode:**
```bash
KREUZBERG_BENCHMARK_DEBUG=true php kreuzberg_extract.php sync /path/to/document.pdf
```

### 2. Adapter Functions
**Location**: `/tools/benchmark-harness/src/adapters/kreuzberg.rs`

Two adapter factory functions were added:
- `create_php_adapter()` - Creates adapter for PHP extraction (single mode)
- `create_php_batch_adapter()` - Creates adapter for batch PHP extraction

### 3. Main Registration
**Location**: `/tools/benchmark-harness/src/main.rs`

PHP adapters are now registered alongside other language bindings during benchmark initialization:
- `kreuzberg-php` - Single-file extraction
- `kreuzberg-php-batch` - Batch multi-file extraction

Each adapter runs with a matrix of [single-file, batch] execution modes for comprehensive performance testing.

## Prerequisites

### 1. PHP Installation
PHP 8.2 or higher is required:
```bash
# Check PHP version
php --version

# macOS (Homebrew)
brew install php@8.4

# Ubuntu/Debian
sudo apt install php8.2-cli

# Windows (via Chocolatey)
choco install php
```

### 2. Composer Dependencies
Install PHP package dependencies:
```bash
cd packages/php
composer install
```

### 3. Build PHP Extension
The PHP extension must be built and loaded before benchmarks can run:
```bash
cd crates/kreuzberg-php
bash build.sh
```

**Note**: The extension build is currently experiencing linker issues on some platforms. Once resolved, the extension will be available for benchmarking.

### 4. Enable Extension
After building, enable the extension by adding to `php.ini` or using `-d` flag:
```bash
# Option 1: Add to php.ini
extension=/path/to/kreuzberg.so

# Option 2: Use -d flag
php -d extension=/path/to/kreuzberg.so script.php
```

## Running PHP Benchmarks

### Quick Test
Once the extension is built and loaded, test the PHP benchmark script:
```bash
./tools/benchmark-harness/scripts/kreuzberg_extract.php sync test_documents/pdfs/sample.pdf
```

### Run Benchmark Harness
Run benchmarks with PHP included:
```bash
# Build the benchmark harness
cargo build --release -p benchmark-harness

# Run benchmarks for all Kreuzberg bindings (including PHP)
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures \
    --frameworks kreuzberg-php,kreuzberg-php-batch \
    --output results/php-benchmark

# Run comparison across multiple bindings
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures \
    --frameworks kreuzberg-native,kreuzberg-python,kreuzberg-ruby,kreuzberg-php,kreuzberg-node \
    --output results/multi-language
```

### Benchmark Modes

**Single-file mode** (sequential execution for latency comparison):
```bash
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures/pdf_small.json \
    --frameworks kreuzberg-php \
    --mode single-file \
    --output results/php-latency
```

**Batch mode** (concurrent execution for throughput measurement):
```bash
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures \
    --frameworks kreuzberg-php-batch \
    --mode batch \
    --output results/php-throughput
```

## Output Format

The benchmark script outputs JSON with timing information:

**Single file extraction:**
```json
{
  "content": "Extracted document text...",
  "metadata": {
    "title": "Document Title",
    "author": "Author Name"
  },
  "_extraction_time_ms": 125.5
}
```

**Batch extraction:**
```json
[
  {
    "content": "First document...",
    "metadata": {},
    "_extraction_time_ms": 45.2,
    "_batch_total_ms": 180.8
  },
  {
    "content": "Second document...",
    "metadata": {},
    "_extraction_time_ms": 45.2,
    "_batch_total_ms": 180.8
  }
]
```

## Benchmark Results

Results are written to the output directory in multiple formats:

- `results.json` - Raw benchmark results with timing data
- `by-extension.json` - Performance analysis grouped by file type
- `index.html` - Interactive HTML visualization (with `--format html` or `--format both`)

## Comparing with Other Bindings

To compare PHP performance with other language bindings:
```bash
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures \
    --frameworks kreuzberg-native,kreuzberg-python,kreuzberg-ruby,kreuzberg-php,kreuzberg-node,kreuzberg-go \
    --output results/language-comparison \
    --format both
```

Then view the HTML report:
```bash
open results/language-comparison/index.html
```

## Troubleshooting

### Extension Not Found
```
Error: Call to undefined function kreuzberg_extract_file()
```
**Solution**: Build and enable the PHP extension (see Prerequisites section)

### Composer Autoload Not Found
```
Error: Could not find autoload.php
```
**Solution**: Run `composer install` in `packages/php/`

### PHP Not Found
```
[adapter] ✗ kreuzberg-php (initialization failed: PHP not found)
```
**Solution**: Install PHP 8.2+ and ensure it's in your PATH

## Architecture

The PHP benchmark integration follows the same pattern as other language bindings:

1. **Benchmark Script** (`kreuzberg_extract.php`): Wraps PHP API calls and outputs JSON
2. **Adapter Functions** (in `kreuzberg.rs`): Factory functions to create subprocess adapters
3. **Subprocess Adapter**: Executes PHP script as subprocess and captures output
4. **Harness Integration**: Registers adapters in main benchmark runner

This design ensures fair comparison across all language bindings by:
- Using the same test documents
- Measuring the same operations
- Outputting results in the same format
- Running under the same concurrency constraints

## Next Steps

Once the PHP extension build is complete:

1. Build and install the PHP extension
2. Run the benchmark suite
3. Compare PHP performance with other bindings
4. Analyze results and optimize as needed

## Contributing

When adding new benchmark scenarios:

1. Add fixture files to `tools/benchmark-harness/fixtures/`
2. Ensure fixtures include expected output for validation
3. Test with multiple language bindings for consistency
4. Update documentation with new benchmark scenarios
