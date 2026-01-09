# Benchmarking Methodology

## Test Setup

- **Platform**: Ubuntu 22.04 (GitHub Actions)
- **Iterations**: 3 runs per benchmark
- **Modes**: Single-file (latency) and Batch (throughput)
- **Documents**: 30+ test files covering all supported formats

## Frameworks Tested

### Kreuzberg Variants
- **Native** (Rust direct)
- **Python** (single, batch)
- **Node.js** (single, batch)
- **WebAssembly** (single, batch)
- **Ruby** (single, batch)
- **Elixir** (single, batch)
- **Java** (single, batch)
- **C#** (single, batch)
- **PHP** (single, batch)
- **Go** (single, batch)

### Competitors
- **Apache Tika** (single, batch)
- **Docling** (single, batch)
- **Unstructured** (single)
- **MarkItDown** (single)
- **Pandoc** (single)
- **PDFPlumber** (single, batch)
- **PyMuPDF4LLM** (single)
- **MinerU** (single, batch)

## Execution Modes

- **Single**: Process one document per function call. Measures per-document latency with sequential execution.
- **Batch**: Process multiple documents in one call. Measures throughput with optimized resource sharing and potential parallelism.

The benchmark harness automatically selects the appropriate mode based on the framework's capabilities. For languages with async support (Python, Node.js), the async implementation is used for better I/O performance.

## File Type Support

Not all frameworks support all file types. For example:
- **Pandoc** excels at text formats (PDF, DOCX, HTML, MD) but doesn't support images (JPG, PNG) or spreadsheets (XLSX)
- **Image processing** requires OCR capabilities (kreuzberg-native, some external tools)
- The visualizer automatically filters frameworks based on timeout detection to show only supported formats

## Metrics Explained

- **Duration (p95, p50)**: 95th and 50th percentile latency in milliseconds (**lower is better**)
- **Throughput**: Megabytes processed per second (**higher is better**)
- **Memory (peak, p95, p99)**: Memory usage percentiles in MB (**lower is better**, generally)
- **CPU**: Average CPU utilization percentage
- **Success Rate**: Percentage of files successfully processed (**higher is better**)

## Caveats

1. **Hardware-dependent**: Results vary by CPU/memory configuration
2. **File size distribution**: Affects throughput calculations
3. **OCR benchmarks**: Require Tesseract installation
4. **Network latency**: Not measured (local file I/O only)
5. **Memory measurement methodology**:
   - **Changed in v4.0.0-rc.30**: Memory measurements now include the entire process tree (parent + all child processes)
   - This provides accurate measurements for frameworks that spawn subprocesses (e.g., Pandoc, Tika, Docling)
   - Previous versions only measured the wrapper process (~12MB), not the actual extraction work
   - All frameworks now measured consistently using process tree traversal
6. **File type support**: Frameworks may not support all file types - the visualizer automatically filters based on timeout detection to show only supported formats

## Running Locally

```bash title="Terminal"
# Build benchmark harness
cargo build --release -p benchmark-harness

# Run benchmarks
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures/ \
    --frameworks kreuzberg-native,docling \
    --output ./benchmark-output \
    --format html

# Open results
open benchmark-output/index.html
```

See [Advanced Guide](../guides/advanced.md) for more options.
