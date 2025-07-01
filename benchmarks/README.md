# Kreuzberg Benchmarks

Performance benchmarking suite for the Kreuzberg text extraction library, focusing on sync vs async performance comparison.

## Features

- **Comprehensive Performance Metrics**: Memory usage, CPU utilization, execution time, GC collections
- **Sync vs Async Comparison**: Direct performance comparison between synchronous and asynchronous implementations
- **Flame Graph Generation**: Visual profiling with py-spy integration
- **JSON Output**: Structured results for CI/CD integration and historical tracking
- **Rich CLI Interface**: Beautiful terminal output with progress bars and tables
- **Stress Testing**: High-load benchmarks for performance limits

## Installation

```bash
cd benchmarks
uv sync
```

## Usage

### Basic Benchmarking

```bash
# Run all benchmarks
kreuzberg-bench run

# Run only sync benchmarks
kreuzberg-bench run --sync-only

# Run only async benchmarks
kreuzberg-bench run --async-only

# Run direct sync vs async comparison
kreuzberg-bench run --comparison-only
```

### Advanced Options

```bash
# Include flame graphs
kreuzberg-bench run --flame

# Include stress tests
kreuzberg-bench run --stress

# Custom output directory
kreuzberg-bench run --output-dir ./my-results

# Custom test files directory
kreuzberg-bench run --test-files-dir ../tests/test_source_files
```

### Analysis

```bash
# Analyze benchmark results
kreuzberg-bench analyze results/latest.json

# Compare two benchmark runs
kreuzberg-bench compare results/run1.json results/run2.json

# Save comparison to file
kreuzberg-bench compare results/run1.json results/run2.json --output comparison.json
```

## Output Format

Results are saved as JSON with the following structure:

```json
{
  "name": "kreuzberg_sync_vs_async",
  "timestamp": "2025-01-01T12:00:00",
  "system_info": {
    "platform": "macOS-15.5-arm64-arm-64bit",
    "python_version": "3.12.10",
    "cpu_count": 14,
    "memory_total_gb": 48.0
  },
  "summary": {
    "total_duration_seconds": 94.129,
    "total_benchmarks": 177,
    "successful_benchmarks": 57,
    "success_rate_percent": 32.2
  },
  "results": [
    {
      "name": "sync_pdf_small_default",
      "success": true,
      "performance": {
        "duration_seconds": 8.022,
        "memory_peak_mb": 27.8,
        "memory_average_mb": 25.1,
        "cpu_percent_average": 75.2,
        "cpu_percent_peak": 90.5,
        "gc_collections": {0: 2, 1: 1, 2: 0}
      },
      "metadata": {
        "file_type": "pdf",
        "config": "default"
      }
    }
  ]
}
```

## CI Integration

### GitHub Actions

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v4
        with:
          python-version: '3.12'

      - name: Install dependencies
        run: |
          cd benchmarks
          uv sync

      - name: Run benchmarks
        run: |
          cd benchmarks
          kreuzberg-bench run --output-dir ./results

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmarks/results/

      - name: Compare with baseline
        if: github.event_name == 'pull_request'
        run: |
          # Download baseline results from main branch
          # Compare and comment on PR
```
