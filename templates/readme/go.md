# Xberg

{% include 'partials/badges.html.jinja' %}

High-performance document intelligence for Go backed by the Rust core that powers every Xberg binding.

> **Version {{ version }}**
> Report issues at [github.com/xberg-io/xberg](https://github.com/xberg-io/xberg/issues).

## What This Package Provides

- **Go module over the Rust core** — context-aware extraction with Go structs and errors.
- **Structured results** — text, tables, images, metadata, language detection, chunks, and warnings.
- **Managed native setup** — download the matching checksummed release archive and generate a version-checked cgo shim.
- **Dynamic and static Xberg linking** — dynamic by default, with an explicit static-archive mode on glibc Linux, macOS, and Windows x86_64.
- **Cross-binding parity** — output matches the Python, Node.js, Ruby, Java, .NET, PHP, Elixir, Dart, Swift, Zig, WASM, and C FFI packages.

## Install

The Go module uses cgo and needs a platform-specific `xberg-ffi` release archive. The generated setup command downloads the archive, verifies its SHA-256 sidecar, and writes a machine-local link shim into your application package.

### Quick Start (Monorepo Development)

For development in the Xberg monorepo:

```bash
# Build the FFI library
cargo build -p xberg-ffi --release

# Stage it in the platform-specific path used by the Go package.
./scripts/stage_go_native_local.sh

# Verify the package builds and passes its tests. cgo links against the
# native library staged under packages/go/.lib automatically.
cd packages/go
go build ./...
go test ./...
```

`packages/go` is a library (`package xberg`), so `go build` produces no executable here. Build a binary from your own `package main` that imports it.

### Using Go Modules

To use this package via `go get`:

```bash
# Get the latest release
go get {{ package_name }}@latest

# Or a specific version
go get {{ package_name }}@v{{ version }}
```

> ⚠️ Do not run `go get github.com/xberg-io/xberg` — the repository root is not a Go module.
> Always target the `/packages/go` subdirectory as shown above; the bare-root path resolves
> against a stale Go module-proxy cache entry and fails.

Run setup from the package that imports Xberg, then build normally:

```bash
go run {{ package_name }}/cmd/setup
go build
```

Re-run setup after upgrading the Go module. The generated shim carries a version sentinel, so a stale native library fails at compile time instead of loading against a mismatched API.

### Supported pre-built platforms

| Platform | Default dynamic mode | `-link static` |
| --- | --- | --- |
| Linux x86_64 (glibc) | Yes | Yes |
| Linux ARM64 (glibc) | Yes | Yes |
| Linux x86_64 (musl/Alpine) | Yes | No |
| Linux ARM64 (musl/Alpine) | Yes | No |
| macOS ARM64 | Yes | Yes |
| macOS x86_64 | Yes | Yes |
| Windows x86_64 (MSVC) | Yes | Yes |

Linux libc is detected automatically. Use `-platform linux-x86_64-musl` or `-platform linux-aarch64-musl` only when detection is unavailable in a minimal container.

### Building with Static Libraries

Static mode selects the explicit Rust static archive (`.a` on Unix, `.lib` on Windows) and the archive's recorded native linker flags:

```bash
go run {{ package_name }}/cmd/setup -link static
go build
```

This statically links the Xberg Rust archive. It does not guarantee a fully static executable: system libraries and native dependencies such as ONNX Runtime may remain dynamic. Musl release archives currently support dynamic mode only.

#### Option 1: Download Pre-built Static Library

Download the static library for your platform from [GitHub Releases](https://github.com/xberg-io/xberg/releases):

```bash
# Example: Linux x86_64 (glibc)
curl -LO https://github.com/xberg-io/xberg/releases/download/v{{ version }}/xberg-go-v{{ version }}-linux-x86_64.tar.gz
tar -xzf xberg-go-v{{ version }}-linux-x86_64.tar.gz

# Copy to a permanent location
mkdir -p ~/xberg/lib
cp xberg-go-v{{ version }}-linux-x86_64/lib/libxberg_ffi.a ~/xberg/lib/
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

ONNX Runtime may remain a runtime dependency even when the Xberg static archive is selected. Check the release archive's `native-static-libs.txt` and the resulting executable's dynamic dependencies for your platform.

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
	input := xberg.ExtractInputFromURI("document.pdf")
	output, err := xberg.Extract(*input, xberg.ExtractionConfig{})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	if len(output.Results) == 0 {
		log.Fatal("extract produced no results")
	}
	result := output.Results[0]

	fmt.Println("MIME:", result.MimeType)
	fmt.Println("First 200 chars:")
	fmt.Println(result.Content[:200])
}
```

Build and run:

```bash
# Run setup once in this package, then build.
go run {{ package_name }}/cmd/setup
go build

# Run. Dynamic mode uses the rpath written by setup.
./myapp
```

## Examples

### Extract bytes

```go
data, err := os.ReadFile("slides.pptx")
if err != nil {
	log.Fatal(err)
}
filename := "slides.pptx"
input := xberg.ExtractInputFromBytes(
	data,
	"application/vnd.openxmlformats-officedocument.presentationml.presentation",
	&filename,
)
output, err := xberg.Extract(*input, xberg.ExtractionConfig{})
if err != nil {
	log.Fatal(err)
}
result := output.Results[0]
fmt.Println("MIME:", result.MimeType)
```

### Use advanced configuration

```go
lang := "eng"
useCache := true
cfg := xberg.ExtractionConfig{
	UseCache: &useCache,
	ForceOcr: false,
	Images:   &xberg.ImageExtractionConfig{},
	Ocr: &xberg.OcrConfig{
		Backend: "tesseract",
		Language: []string{lang},
	},
}
input := xberg.ExtractInputFromURI("scanned.pdf")
output, err := xberg.Extract(*input, cfg)
result := output.Results[0]
```

### URL extraction

```go
input := xberg.ExtractInputFromURI("https://example.com/report.pdf")
output, err := xberg.Extract(*input, xberg.ExtractionConfig{})
if err != nil {
	log.Fatal(err)
}
fmt.Println("Results:", len(output.Results))
```

### Batch extract

```go
inputs := []xberg.ExtractInput{
	*xberg.ExtractInputFromURI("doc1.pdf"),
	*xberg.ExtractInputFromURI("https://example.com/report.pdf"),
}
output, err := xberg.ExtractBatch(inputs, xberg.ExtractionConfig{})
if err != nil {
	log.Fatal(err)
}
for i, res := range output.Results {
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
	if err := xberg.RegisterValidator("go-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
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
| `ld returned 1 exit status` or `cannot find -lxberg_ffi`                  | Run `go run {{ package_name }}/cmd/setup` from the package you are building. Re-run it after every Xberg module upgrade. |
| `native library missing after download/verification` with `-link static`  | Static mode is available for glibc Linux, macOS, and Windows x86_64 release archives. Use dynamic mode on musl, or build `xberg-ffi` from source. |
| `undefined: xberg.Extract`                                                  | Regenerate the binding or update to Xberg v1; extraction is exposed as `Extract(ExtractInput, ExtractionConfig)`.                                                                                           |
| `Missing dependency: tesseract`                                                | Install the OCR backend and ensure it is on `PATH`. Errors bubble up as typed Xberg errors.                                                                                                               |
| `undefined: C.customValidator` during build                                    | Export the callback with `//export` in a `*_cgo.go` file before using it in `Register*` helpers.                                                                                                                    |
| `Missing dependency: onnxruntime`                                              | Install ONNX Runtime at build time: `brew install onnxruntime` (macOS), `apt install libonnxruntime libonnxruntime-dev` (Linux), `scoop install onnxruntime` (Windows). Required for embeddings functionality.      |
| Embeddings not available on Windows MinGW                                      | Windows MinGW builds cannot link ONNX Runtime (MSVC-only). Use Windows MSVC build for embeddings support, or build without embeddings feature.                                                                      |

## Testing / Tooling

- `task go:lint` – runs `gofmt` and `golangci-lint` (`golangci-lint` pinned to v2.11.3).
- `task go:test` – executes `go test ./...` (after building the static FFI library).
- `task e2e:go:verify` – regenerates fixtures via the e2e generator and runs `go test ./...` inside `e2e/go`.

Need help? Join the [Discord](https://discord.gg/xt9WY3GnKR) or open an issue with logs, platform info, and the steps you tried.

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — the open-source content-intelligence engine: text, tables, and metadata from 107 formats (141 file extensions), with OCR, transcription, and code intelligence. MIT.
- [Xberg Pro](https://xberg.io) — a complete self-hosted content-intelligence backend in a single container. Commercial.
- [Xberg Enterprise](https://xberg.io) — the distributed, governed content-intelligence platform, scaled on Kubernetes with team governance and support. Commercial.
- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 165 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.
