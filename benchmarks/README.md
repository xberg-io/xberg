# Kreuzberg Benchmarks

Performance benchmarking suite for the Kreuzberg text extraction library, with comprehensive testing capabilities.

## Features

- **Multiple Benchmark Types**: Baseline, statistical, serialization, and comprehensive performance benchmarks
- **Comprehensive Performance Metrics**: Memory usage, CPU utilization, execution time, cache performance
- **Statistical Analysis**: Multiple trials with proper statistical reporting (mean, stdev, median)
- **Cache Performance Testing**: Cold vs warm cache comparison with speedup calculations
- **Serialization Benchmarking**: JSON vs msgpack performance comparison
- **Rich CLI Interface**: Beautiful terminal output with progress bars and tables
- **Consistent Result Files**: No timestamps in filenames for easier tracking
- **JSON Output**: Structured results for CI/CD integration and historical tracking

## Installation

From the Kreuzberg workspace root:

```bash
# Install all dependencies including optional features
uv sync --all-extras

# The benchmarks package is automatically available
```

## Usage

**Important**: All commands must be run from the Kreuzberg workspace root directory.

### Quick Start

```bash
# Run baseline cache performance test
uv run python -m benchmarks.src baseline

# Run statistical benchmark with multiple trials
uv run python -m benchmarks.src statistical --trials 5

# Run serialization performance test
uv run python -m benchmarks.src serialization

# Run comprehensive benchmarks (sync only for speed)
uv run python -m benchmarks.src run --sync-only
```

### All Available Commands

```bash
# Show help and available commands
uv run python -m benchmarks.src --help

# Baseline: Cache performance testing
uv run python -m benchmarks.src baseline [--output FILE]

# Statistical: Multi-trial performance analysis
uv run python -m benchmarks.src statistical [--trials N] [--output FILE]

# Serialization: JSON vs msgpack comparison
uv run python -m benchmarks.src serialization [--output FILE]

# Run: Comprehensive benchmark suite
uv run python -m benchmarks.src run [OPTIONS]

# Compare: Compare two benchmark result files
uv run python -m benchmarks.src compare FILE1 FILE2 [--output FILE]

# Analyze: Analyze benchmark results
uv run python -m benchmarks.src analyze FILE [--quality]
```

### Comprehensive Benchmark Options

```bash
# Run only synchronous benchmarks (faster)
uv run python -m benchmarks.src run --sync-only

# Run only asynchronous benchmarks
uv run python -m benchmarks.src run --async-only

# Run direct sync vs async comparisons
uv run python -m benchmarks.src run --comparison-only

# Include stress test benchmarks
uv run python -m benchmarks.src run --stress

# Run backend comparison benchmarks
uv run python -m benchmarks.src run --backend-comparison

# Custom test files directory
uv run python -m benchmarks.src run --test-files-dir /path/to/test/files

# Generate flame graphs for profiling
uv run python -m benchmarks.src run --flame

# Custom suite name and output directory
uv run python -m benchmarks.src run --suite-name my_benchmark --output-dir my_results
```

### Result Files

Benchmark results are automatically saved to consistent filenames in the `results/` directory:

- `baseline.json` - Cache performance benchmark results
- `statistical.json` - Multi-trial statistical benchmark results
- `serialization.json` - Serialization performance comparison results
- `kreuzberg_sync_vs_async.json` - Comprehensive benchmark results
- `latest.json` - Copy of the most recent comprehensive benchmark

All result files use consistent naming without timestamps for easier tracking and comparison.

### Result Analysis

```bash
# Analyze any result file for performance insights
uv run python -m benchmarks.src analyze results/baseline.json

# Generate metadata quality report (for comprehensive benchmarks)
uv run python -m benchmarks.src analyze results/latest.json --quality

# Compare two benchmark runs
uv run python -m benchmarks.src compare results/baseline.json results/statistical.json --output comparison.json
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
      - name: Install uv
        uses: astral-sh/setup-uv@v4

      - name: Install dependencies
        run: uv sync --all-extras

      - name: Run baseline benchmark
        run: uv run python -m benchmarks.src baseline

      - name: Run statistical benchmark
        run: uv run python -m benchmarks.src statistical --trials 3

      - name: Run serialization benchmark
        run: uv run python -m benchmarks.src serialization

      - name: Run comprehensive benchmark (sync only)
        run: uv run python -m benchmarks.src run --sync-only

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmarks/results/

      - name: Compare with baseline
        if: github.event_name == 'pull_request'
        run: |
          # Download baseline results from main branch
          # Compare current results with baseline
          uv run python -m benchmarks.src compare baseline.json latest.json --output pr-comparison.json
```

### Running in CI/CD

The benchmarks are designed to work well in CI environments:

- Use `--sync-only` for faster execution
- Reduce trial counts for statistical benchmarks (`--trials 3`)
- Results are saved in consistent JSON format for easy comparison
- All optional dependencies are included with `uv sync --all-extras`

### Performance Tracking

```bash
# Create baseline on main branch
uv run python -m benchmarks.src run --sync-only --suite-name main_baseline

# Compare PR performance
uv run python -m benchmarks.src run --sync-only --suite-name pr_test
uv run python -m benchmarks.src compare results/main_baseline.json results/pr_test.json
```
