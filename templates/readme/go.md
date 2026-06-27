# Xberg

{% include 'partials/badges.html.jinja' %}

High-performance document intelligence for Go backed by the Rust core that powers every Xberg binding.

> **Version {{ version }}**
> Report issues at [github.com/xberg-io/xberg](https://github.com/xberg-io/xberg/issues).

## What This Package Provides

- **Go module over the Rust core** — context-aware extraction with Go structs and errors.
- **Structured results** — text, tables, images, metadata, language detection, chunks, and warnings.
- **Static-link workflow** — build against `xberg-ffi` and ship a self-contained Go binary.
- **Cross-binding parity** — output matches the Python, Node.js, Ruby, Java, .NET, PHP, Elixir, R, Dart, Swift, Zig, WASM, and C FFI packages.

## Install

Xberg Go binaries are **statically linked** — once built, they are self-contained and require no runtime library dependencies. Only the static library is needed at build time.

### Quick Start (Monorepo Development)

For development in the Xberg monorepo:

```bash
# Build the static FFI library
cargo build -p xberg-ffi --release

# Go build will automatically link against the static library
# (from target/release/libxberg_ffi.a)
cd packages/go
go build -v

# Run your binary (no library path needed - it's statically linked)
./v4
```

That's it! The resulting binary is self-contained and has no runtime dependencies on Xberg libraries.

### Using Go Modules

To use this package via `go get`:

```bash
# Get the latest release
go get {{ package_name }}@latest

# Or a specific version
go get {{ package_name }}@v{{ version }}
```

You'll need to provide the static library at build time. See [Building with Static Libraries](#building-with-static-libraries) below.

### Building with Static Libraries

When building outside the Xberg monorepo, you need to provide the static library (`.a` file on Unix, `.lib` on Windows).

#### Option 1: Download Pre-built Static Library

Download the static library for your platform from [GitHub Releases](https://github.com/xberg-io/xberg/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/xberg-io/xberg/releases/download/v{{ version }}/go-ffi-linux-x86_64.tar.gz
tar -xzf go-ffi-linux-x86_64.tar.gz

# Copy to a permanent location
mkdir -p ~/xberg/lib
cp xberg-ffi/lib/libxberg_ffi.a ~/xberg/lib/
```

Then build with `CGO_LDFLAGS`:

```bash
# Linux/macOS
CGO_LDFLAGS="-L$HOME/xberg/lib -lxberg_ffi" go build

# Windows (MSVC)
set CGO_LDFLAGS=-L%USERPROFILE%\xberg\lib -lxberg_ffi
go build
```

#### Option 2: Build Static Library Yourself

If pre-built libraries aren't available for your platform:

```bash
# Clone the repository
git clone https://github.com/xberg-io/xberg.git
cd xberg

# Build the static library
cargo build -p xberg-ffi --release

# The static library is now at: target/release/libxberg_ffi.a
# Copy it to a permanent location
mkdir -p ~/xberg/lib
cp target/release/libxberg_ffi.a ~/xberg/lib/

# Now you can build Go projects
cd ~/my-go-project
CGO_LDFLAGS="-L$HOME/xberg/lib -lxberg_ffi" go build
```

### System Requirements

#### ONNX Runtime (for embeddings)

If using embeddings functionality, ONNX Runtime must be installed **at build time**:

```bash
# macOS
brew install onnxruntime

# Ubuntu/Debian
sudo apt install libonnxruntime libonnxruntime-dev

# Windows (MSVC)
scoop install onnxruntime
# OR download from https://github.com/microsoft/onnxruntime/releases
```

The resulting binary will have ONNX Runtime statically linked or dynamically linked depending on how the FFI library was built. Check the build configuration.

**Note:** Windows MinGW builds do not support embeddings (ONNX Runtime requires MSVC). Use Windows MSVC for embeddings support.

## Quickstart

```go
package main

import (
	"fmt"
	"log"

	"{{ package_name }}"
)

func main() {
	result, err := v4.ExtractSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("MIME:", result.MimeType)
	fmt.Println("First 200 chars:")
	fmt.Println(result.Content[:200])
}
```

Build and run:

```bash
# Build (make sure you have the static library available - see Install)
CGO_LDFLAGS="-L$HOME/xberg/lib -lxberg_ffi" go build

# Run - no library paths needed!
./myapp
```

The binary is self-contained and can be distributed without any Xberg library dependencies.

## Examples

### Extract bytes

```go
data, err := os.ReadFile("slides.pptx")
if err != nil {
	log.Fatal(err)
}
result, err := v4.ExtractSync(data, "application/vnd.openxmlformats-officedocument.presentationml.presentation", nil)
if err != nil {
	log.Fatal(err)
}
fmt.Println(result.Metadata.FormatType())
```

### Use advanced configuration

```go
lang := "eng"
cfg := &v4.ExtractionConfig{
	UseCache:        true,
	ForceOCR:        false,
	ImageExtraction: &v4.ImageExtractionConfig{Enabled: true},
	OCR: &v4.OcrConfig{
		Backend: "tesseract",
		Language: &lang,
	},
}
result, err := v4.ExtractSync("scanned.pdf", cfg)
```

### Async (context-aware) extraction

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

result, err := v4.Extract(ctx, "large.pdf", nil)
if err != nil {
	log.Fatal(err)
}
fmt.Println("Content length:", len(result.Content))
```

### Batch extract

```go
paths := []string{"doc1.pdf", "doc2.docx", "report.xlsx"}
results, err := v4.ExtractBatchSync(paths, nil)
if err != nil {
	log.Fatal(err)
}
for i, res := range results {
	if res == nil {
		continue
	}
	fmt.Printf("[%d] %s => %d bytes\n", i, res.MimeType, len(res.Content))
}
```

### Register a validator

```go
//export customValidator
func customValidator(resultJSON *C.char) *C.char {
	// Validate JSON payload and return an error string (or NULL if ok)
	return nil
}

func init() {
	if err := v4.RegisterValidator("go-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
		log.Fatalf("validator registration failed: %v", err)
	}
}
```

## API Reference

- **GoDoc**: [pkg.go.dev/{{ package_name }}](<https://pkg.go.dev/{{ package_name }}>)
- **Full documentation**: [xberg.io](https://xberg.io) (configuration, formats, OCR backends)

## Troubleshooting

| Issue                                                                          | Fix                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ld returned 1 exit status` or `undefined reference to 'html_to_markdown_...'` | The static library wasn't found. Make sure `CGO_LDFLAGS` points to the directory containing `libxberg_ffi.a`: `CGO_LDFLAGS="-L/path/to/lib -lxberg_ffi" go build`                                           |
| `cannot find -lxberg_ffi`                                                  | The static library file is missing or in the wrong location. Download it from [GitHub Releases](https://github.com/xberg-io/xberg/releases) or build it yourself: `cargo build -p xberg-ffi --release` |
| `undefined: v4.Extract`                                                    | This function was removed in v4.1.0. Use `ExtractSync` and wrap in goroutine if needed (see migration guide)                                                                                                    |
| `Missing dependency: tesseract`                                                | Install the OCR backend and ensure it is on `PATH`. Errors bubble up as `*v4.MissingDependencyError`.                                                                                                               |
| `undefined: C.customValidator` during build                                    | Export the callback with `//export` in a `*_cgo.go` file before using it in `Register*` helpers.                                                                                                                    |
| `Missing dependency: onnxruntime`                                              | Install ONNX Runtime at build time: `brew install onnxruntime` (macOS), `apt install libonnxruntime libonnxruntime-dev` (Linux), `scoop install onnxruntime` (Windows). Required for embeddings functionality.      |
| Embeddings not available on Windows MinGW                                      | Windows MinGW builds cannot link ONNX Runtime (MSVC-only). Use Windows MSVC build for embeddings support, or build without embeddings feature.                                                                      |

## Testing / Tooling

- `task go:lint` – runs `gofmt` and `golangci-lint` (`golangci-lint` pinned to v2.11.3).
- `task go:test` – executes `go test ./...` (after building the static FFI library).
- `task e2e:go:verify` – regenerates fixtures via the e2e generator and runs `go test ./...` inside `e2e/go`.

Need help? Join the [Discord](https://discord.gg/xt9WY3GnKR) or open an issue with logs, platform info, and the steps you tried.

## Part of Xberg.dev

- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.
