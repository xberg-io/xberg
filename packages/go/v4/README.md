# Kreuzberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
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
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.2.4" alt="Go">
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
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Documentation">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>


High-performance document intelligence for Go backed by the Rust core that powers every Kreuzberg binding.

> **Version 4.1.0**
> Report issues at [github.com/kreuzberg-dev/kreuzberg](https://github.com/kreuzberg-dev/kreuzberg/issues).

## Install

Kreuzberg Go binaries are **statically linked** — once built, they are self-contained and require no runtime library dependencies. Only the static library is needed at build time.

### Quick Start (Monorepo Development)

For development in the Kreuzberg monorepo:

```bash
# Build the static FFI library
cargo build -p kreuzberg-ffi --release

# Go build will automatically link against the static library
# (from target/release/libkreuzberg_ffi.a)
cd packages/go/v4
go build -v

# Run your binary (no library path needed - it's statically linked)
./v4
```

That's it! The resulting binary is self-contained and has no runtime dependencies on Kreuzberg libraries.

### Using Go Modules

To use this package via `go get`:

```bash
# Get the latest release
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest

# Or a specific version
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@v4.1.0
```

You'll need to provide the static library at build time. See [Building with Static Libraries](#building-with-static-libraries) below.

### Building with Static Libraries

When building outside the Kreuzberg monorepo, you need to provide the static library (`.a` file on Unix, `.lib` on Windows).

#### Option 1: Download Pre-built Static Library

Download the static library for your platform from [GitHub Releases](https://github.com/kreuzberg-dev/kreuzberg/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/kreuzberg-dev/kreuzberg/releases/download/v4.1.0/go-ffi-linux-x86_64.tar.gz
tar -xzf go-ffi-linux-x86_64.tar.gz

# Copy to a permanent location
mkdir -p ~/kreuzberg/lib
cp kreuzberg-ffi/lib/libkreuzberg_ffi.a ~/kreuzberg/lib/
```

Then build with `CGO_LDFLAGS`:

```bash
# Linux/macOS
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build

# Windows (MSVC)
set CGO_LDFLAGS=-L%USERPROFILE%\kreuzberg\lib -lkreuzberg_ffi
go build
```

#### Option 2: Build Static Library Yourself

If pre-built libraries aren't available for your platform:

```bash
# Clone the repository
git clone https://github.com/kreuzberg-dev/kreuzberg.git
cd kreuzberg

# Build the static library
cargo build -p kreuzberg-ffi --release

# The static library is now at: target/release/libkreuzberg_ffi.a
# Copy it to a permanent location
mkdir -p ~/kreuzberg/lib
cp target/release/libkreuzberg_ffi.a ~/kreuzberg/lib/

# Now you can build Go projects
cd ~/my-go-project
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build
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

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	result, err := v4.ExtractFileSync("document.pdf", nil)
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
CGO_LDFLAGS="-L$HOME/kreuzberg/lib -lkreuzberg_ffi" go build

# Run - no library paths needed!
./myapp
```

The binary is self-contained and can be distributed without any Kreuzberg library dependencies.

## Examples

### Extract bytes

```go
data, err := os.ReadFile("slides.pptx")
if err != nil {
	log.Fatal(err)
}
result, err := v4.ExtractBytesSync(data, "application/vnd.openxmlformats-officedocument.presentationml.presentation", nil)
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
result, err := v4.ExtractFileSync("scanned.pdf", cfg)
```

### Async (context-aware) extraction

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

result, err := v4.ExtractFile(ctx, "large.pdf", nil)
if err != nil {
	log.Fatal(err)
}
fmt.Println("Content length:", len(result.Content))
```

### Batch extract

```go
paths := []string{"doc1.pdf", "doc2.docx", "report.xlsx"}
results, err := v4.BatchExtractFilesSync(paths, nil)
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

- **GoDoc**: [pkg.go.dev/github.com/kreuzberg-dev/kreuzberg/packages/go/v4](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg/packages/go/v4)
- **Full documentation**: [kreuzberg.dev](https://kreuzberg.dev) (configuration, formats, OCR backends)

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `ld returned 1 exit status` or `undefined reference to 'html_to_markdown_...'` | The static library wasn't found. Make sure `CGO_LDFLAGS` points to the directory containing `libkreuzberg_ffi.a`: `CGO_LDFLAGS="-L/path/to/lib -lkreuzberg_ffi" go build` |
| `cannot find -lkreuzberg_ffi` | The static library file is missing or in the wrong location. Download it from [GitHub Releases](https://github.com/kreuzberg-dev/kreuzberg/releases) or build it yourself: `cargo build -p kreuzberg-ffi --release` |
| `undefined: v4.ExtractFile` | This function was removed in v4.1.0. Use `ExtractFileSync` and wrap in goroutine if needed (see migration guide) |
| `Missing dependency: tesseract` | Install the OCR backend and ensure it is on `PATH`. Errors bubble up as `*v4.MissingDependencyError`. |
| `undefined: C.customValidator` during build | Export the callback with `//export` in a `*_cgo.go` file before using it in `Register*` helpers. |
| `Missing dependency: onnxruntime` | Install ONNX Runtime at build time: `brew install onnxruntime` (macOS), `apt install libonnxruntime libonnxruntime-dev` (Linux), `scoop install onnxruntime` (Windows). Required for embeddings functionality. |
| Embeddings not available on Windows MinGW | Windows MinGW builds cannot link ONNX Runtime (MSVC-only). Use Windows MSVC build for embeddings support, or build without embeddings feature. |

## Testing / Tooling

- `task go:lint` – runs `gofmt` and `golangci-lint` (`golangci-lint` pinned to v2.8.0).
- `task go:test` – executes `go test ./...` (after building the static FFI library).
- `task e2e:go:verify` – regenerates fixtures via the e2e generator and runs `go test ./...` inside `e2e/go`.

Need help? Join the [Discord](https://discord.gg/xt9WY3GnKR) or open an issue with logs, platform info, and the steps you tried.
