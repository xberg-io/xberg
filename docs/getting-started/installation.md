---
description: "Install Xberg — pick Python, TypeScript, Rust, Go, Java/Kotlin, CLI/Docker, or another supported SDK."
---

# Installation

Polyglot SDKs plus a standalone CLI. Most packages ship **prebuilt binaries** for Linux (x86_64/aarch64), macOS, and Windows — no compile step needed.

<div class="cli-hero" markdown>

## :material-console: CLI / Docker { #cli--docker }

No SDK, no code — just your terminal.

=== "Install script"

    ```bash
    curl -fsSL https://raw.githubusercontent.com/xberg-io/xberg/main/scripts/install.sh | bash
    ```

=== "Homebrew"

    ```bash
    brew trust xberg-io/tap
    brew install xberg-io/tap/xberg
    ```

=== "Cargo"

    ```bash
    cargo install xberg-cli
    ```

=== "Docker (CLI image)"

    ```bash
    docker pull ghcr.io/xberg-io/xberg-cli:latest
    docker run -v $(pwd):/data ghcr.io/xberg-io/xberg-cli:latest extract /data/document.pdf
    ```

=== "Docker (full image)"

    ```bash
    docker pull ghcr.io/xberg-io/xberg:latest
    ```

!!! Note "MCP Server included"

    Prebuilt binaries (Homebrew, install.sh, Docker full) include the MCP server. If building from source with `cargo install xberg-cli`, add `--features mcp` (or `--features mcp-http` for HTTP transport) to include it.

[CLI Usage](../cli/usage.md){ .install-btn .install-btn--ghost }
[API Server Guide](../guides/api-server.md){ .install-btn .install-btn--solid }

</div>

!!! Warning "x86_64 CPU — AVX/AVX2 instruction set required"

    The bundled ONNX Runtime binaries require **AVX/AVX2** CPU instructions. CPUs without AVX support (e.g. Intel Atom, Celeron N5105/Jasper Lake, older pre-2011 processors) will crash with an `invalid opcode` trap when using ONNX-dependent features. The affected features are **PaddleOCR**, **layout detection**, **embeddings**, **reranking**, **auto-rotate**, and **transcription**. All other Xberg functionality (text extraction, Tesseract OCR, chunking, metadata, etc.) works normally on any x86_64 CPU. ARM platforms (aarch64) are unaffected.

!!! Warning "Windows — ONNX Runtime required for Go, Elixir, and C/C++"

    Go, Elixir, and C/C++ bindings on Windows link against ONNX Runtime dynamically. You must have `onnxruntime.dll` on your `PATH` at runtime. Download it from the [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) (for example `onnxruntime-win-x64-1.24.1.zip`). Python, TypeScript, Java, C#, Ruby, PHP, and Wasm are unaffected.

## Choose your language

<div class="grid cards install-cards" markdown>

- :fontawesome-brands-python:{ .lg .middle } **Python**

  ***

  ```bash
  pip install xberg
  ```

  [API Reference](../reference/api-python.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-node-js:{ .lg .middle } **TypeScript (Node.js / Bun)**

  ***

  ```bash
  npm install @xberg-io/xberg
  ```

  [API Reference](../reference/api-typescript.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#typescript){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-js:{ .lg .middle } **TypeScript (Browser / Edge)**

  ***

  ```bash
  npm install @xberg-io/xberg-wasm
  ```

  [API Reference](../reference/api-wasm.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#typescript){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-rust:{ .lg .middle } **Rust**

  ***

  ```bash
  cargo add xberg
  ```

  [API Reference](../reference/api-rust.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-golang:{ .lg .middle } **Go**

  ***

  ```bash
  go get github.com/xberg-io/xberg@latest
  ```

  [API Reference](../reference/api-go.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-java:{ .lg .middle } **Java**

  ***

  ```gradle
  implementation 'io.xberg:xberg:1.0.0-rc.1'
  ```

  [API Reference](../reference/api-java.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#java){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-kotlin:{ .lg .middle } **Kotlin Android**

  ***

  ```kotlin
  implementation("io.xberg:xberg-android:1.0.0-rc.1")
  ```

  [API Reference](../reference/api-kotlin-android.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#kotlin){ .install-btn .install-btn--solid .install-btn--sm }

- :material-language-ruby:{ .lg .middle } **Ruby**

  ***

  ```bash
  gem install xberg
  ```

  [API Reference](../reference/api-ruby.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-swift:{ .lg .middle } **Swift**

  ***

  ```swift
  .package(url: "https://github.com/xberg-io/xberg.git", from: "1.0.0-rc.1")
  ```

  [API Reference](../reference/api-swift.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#swift){ .install-btn .install-btn--solid .install-btn--sm }

- :material-language-csharp:{ .lg .middle } **C# / .NET**

  ***

  ```bash
  dotnet add package Xberg
  ```

  [API Reference](../reference/api-csharp.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](../reference/api-csharp.md){ .install-btn .install-btn--solid .install-btn--sm }

- :fontawesome-brands-php:{ .lg .middle } **PHP**

  ***

  ```bash
  composer require xberg-io/xberg
  ```

  [API Reference](../reference/api-php.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :simple-elixir:{ .lg .middle } **Elixir**

  ***

  ```elixir
  {:xberg, "~> 1.0.0-rc.1"}
  ```

  [API Reference](../reference/api-elixir.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#elixir){ .install-btn .install-btn--solid .install-btn--sm }

- :simple-r:{ .lg .middle } **R**

  ***

  ```r
  install.packages("xberg",
    repos = "https://xberg-io.r-universe.dev")
  ```

  [API Reference](../reference/api-r.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid .install-btn--sm }

- :simple-cplusplus:{ .lg .middle } **C / C++**

  ***

  ```bash
  cargo build -p xberg-ffi
  ```

  [API Reference](../reference/api-c.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#c-c){ .install-btn .install-btn--solid .install-btn--sm }

- :material-language-dart:{ .lg .middle } **Dart / Flutter**

  ***

  ```bash
  dart pub add xberg
  ```

  [API Reference](../reference/api-dart.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#dart){ .install-btn .install-btn--solid .install-btn--sm }

- :material-language-zig:{ .lg .middle } **Zig**

  ***

  ```bash
  zig fetch --save https://github.com/xberg-io/xberg/archive/refs/tags/v1.0.0-rc.1.tar.gz
  ```

  [API Reference](../reference/api-zig.md){ .install-api-link }
  [:material-lightning-bolt: Quick Start](#zig){ .install-btn .install-btn--solid .install-btn--sm }

</div>

---

## System requirements

Only relevant if building from source or enabling OCR:

| Dependency                | When you need it                                                                       |
| ------------------------- | -------------------------------------------------------------------------------------- |
| AVX/AVX2 CPU instructions | Required for ONNX Runtime features (PaddleOCR, layout detection, embeddings, reranking, auto-rotate, transcription) on x86_64 |
| Rust toolchain (`rustup`) | Building any native binding from source                                                |
| C/C++ compiler            | Building native bindings (Xcode command-line tools / `build-essential` / MSVC)         |
| Tesseract OCR             | Optional — `brew install tesseract` / `apt install tesseract-ocr`                      |
| libheif (HEIC / HEIF / AVIF) | Optional — `brew install libheif` / `apt install libheif-dev` / `dnf install libheif-devel` |

PDF extraction uses pdf_oxide and has no external PDF runtime dependency.

The Wasm package (`@xberg-io/xberg-wasm`) has **zero** system dependencies.

### HEIF / HEIC / AVIF support { #heif--heic--avif-support }

Pixel decoding for Apple HEIC photos, HEIF still images, AVIF, HEIC sequences
(`.heics`), and AVCS requires the **`heic` Cargo feature** plus the system
`libheif` library (with `libde265` for HEVC and `libaom` for AV1):

- **macOS**: `brew install libheif`
- **Debian / Ubuntu**: `apt install libheif-dev`
- **Fedora**: `dnf install libheif-devel`
- **Windows (vcpkg)**: `vcpkg install libheif[hevc,aom]:x64-windows`

Enable the feature when building from source:

```toml
xberg = { version = "5", features = ["heic", "ocr"] }
```

`heic` is included in the `full` aggregate feature. HEIC pixel decoding is
**not available** on `wasm-target` or `android-target` (libheif is a C library
with no working WASM/Android build story). EXIF metadata extraction from HEIC
/ HEIF / AVIF works on **every** target via the pure-Rust `nom-exif`
integration.

### GPU Acceleration

Xberg bundles a CPU-only ONNX Runtime — ML features (PaddleOCR, layout detection, embeddings, reranking, auto-rotate, transcription) work out of the box on CPU.

For GPU acceleration, install a GPU-enabled ONNX Runtime and set `ORT_DYLIB_PATH`:

| Platform        | Install                                                                                  | Set ORT_DYLIB_PATH                                 |
| --------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------- |
| Linux (CUDA)    | Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) | `export ORT_DYLIB_PATH=/path/to/libonnxruntime.so` |
| Python (any OS) | `pip install onnxruntime-gpu`                                                            | Point at the pip package's `capi/` directory       |
| macOS (CoreML)  | Works with bundled ORT — no extra setup needed                                           | —                                                  |

See [AccelerationConfig](../reference/configuration.md#accelerationconfig) and [ORT_DYLIB_PATH](../reference/environment-variables.md#ort_dylib_path) for details.

---

## Language-specific notes

Edge cases and alternative install methods where they come up.

### TypeScript

Two npm packages target different runtimes:

| Package           | Best for                           | Performance    |
| ----------------- | ---------------------------------- | -------------- |
| `@xberg-io/xberg` | Node.js, Bun — server-side apps    | Native (100%)  |
| `@xberg-io/xberg-wasm` | Browsers, Deno, Cloudflare Workers | Wasm (~60-80%) |

Both work with **pnpm** (`pnpm add`) and **Yarn** (`yarn add`) as well.

!!! Note "pnpm workspaces"

    In monorepos, add this to your root `.npmrc` so platform-specific optional deps resolve correctly:

    ```ini
    auto-install-peers=true
    ```

??? Note "Wasm — Browser usage"

    ```html
    <script type="module">
      import { ExtractInputKind, initWasm, extract } from "@xberg-io/xberg-wasm";

      await initWasm();

      const input = document.getElementById("file");
      input.addEventListener("change", async (e) => {
        const file = e.target.files?.[0];
        if (!file) return;

        const output = await extract({
          kind: ExtractInputKind.Bytes,
          bytes: new Uint8Array(await file.arrayBuffer()),
          mimeType: file.type || "application/octet-stream",
          filename: file.name,
        });
        console.log(output.results[0].content);
      });
    </script>

    <input type="file" id="file" />
    ```

??? Note "Wasm — Deno"

    ```typescript
    import { ExtractInputKind, initWasm, extract } from "npm:@xberg-io/xberg-wasm";

    await initWasm();
    const output = await extract({
      kind: ExtractInputKind.Uri,
      uri: "./document.pdf",
    });
    console.log(output.results[0].content);
    ```

??? Note "Wasm — Cloudflare Workers"

    ```typescript
    import { ExtractInputKind, initWasm, extract } from "@xberg-io/xberg-wasm";

    export default {
      async fetch(request: Request): Promise<Response> {
        await initWasm();
        const bytes = new Uint8Array(await request.arrayBuffer());
        const output = await extract({
          kind: ExtractInputKind.Bytes,
          bytes,
          mimeType: "application/pdf",
        });
        return Response.json({ content: output.results[0]?.content ?? "" });
      },
    };
    ```

**Supported runtimes:** Chrome 74+, Firefox 79+, Safari 14+, Edge 79+, Node.js 22+, Deno 1.35+, Cloudflare Workers.

!!! Warning "Wasm Platform Limitations"

    The Wasm binding does not support:

    - **Layout detection** (RT-DETR model inference requires ONNX Runtime unavailable in WebAssembly)
    - **PaddleOCR, embeddings, reranking, auto-rotate, and transcription inference** (all require ONNX Runtime)
    - **LLM/VLM features** (liter-llm is not part of the `wasm-target` feature set)
    - **Hardware acceleration config** (single-threaded WASM, no GPU access)
    - **Native server features** (`api`, `mcp`, CLI binary)
    - **Browser filesystem paths** (use `kind = "bytes"` for browser file uploads; path APIs require Node/Deno/Bun filesystem access)
    - **Email codepage config** (EmailConfig not available)

    Pure-Rust extraction formats, OCR via Tesseract WASM, chunking, metadata, tables, language detection, SVG handling, redaction, summarization, QR-code detection, and image extraction work in WASM. See the [WASM API Reference](../reference/api-wasm.md) for details.

### Java

=== "Maven"

    ```xml
    <dependency>
        <groupId>io.xberg</groupId>
        <artifactId>xberg</artifactId>
        <version>1.0.0-rc.1</version>
    </dependency>
    ```

=== "Gradle"

    ```gradle
    implementation 'io.xberg:xberg:1.0.0-rc.1'
    ```

Requires Java 25+ (FFM/Panama API). Native libraries are bundled in the JAR.

### Elixir

Add to `mix.exs`:

```elixir
def deps do
  [
    {:xberg, "~> 1.0.0-rc.1"}
  ]
end
```

```bash
mix deps.get
```

Ships prebuilt NIF binaries via RustlerPrecompiled. Falls back to compiling from source if no prebuilt matches your platform (requires Rust).

!!! Warning "Windows"

    The Windows NIF links against ONNX Runtime dynamically. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

### Go

```bash
go get github.com/xberg-io/xberg@latest
```

!!! Warning "Windows"

    The Go binding links against ONNX Runtime dynamically on Windows. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

!!! Note "Windows feature limitations"

    The Go and C/C++ bindings on Windows (MinGW/GNU target) do not include ORT-dependent inference features: **PaddleOCR**, **layout detection**, **embeddings**, **reranking**, **auto-rotate**, or **transcription**. Tesseract OCR and non-ORT features work normally. These limitations apply only to Windows; Linux and macOS builds include the full feature set.

### Rust

Enable features selectively in `Cargo.toml`:

```toml title="Cargo.toml"
[dependencies]
xberg = { version = "5", features = ["pdf", "ocr", "chunking"] }
# Default features are tokio-runtime + simd-utf8; format and analysis features are opt-in.
```

### C / C++

Build the FFI library from source:

```bash
cargo build --release -p xberg-ffi
```

This produces `libxberg_ffi.a` and a header at `crates/xberg-ffi/xberg.h`. Link into your project:

```makefile
HEADER_DIR = path/to/crates/xberg-ffi
LIBDIR     = path/to/target/release

CFLAGS  = -Wall -Wextra -I$(HEADER_DIR)
LDFLAGS = -L$(LIBDIR) -lxberg_ffi -lpthread -ldl -lm

my_app: my_app.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)
```

!!! Tip "Platform-specific linker flags"

    **macOS:** add `-framework CoreFoundation -framework Security`

    **Windows:** add `-lws2_32 -luserenv -lbcrypt`

!!! Warning "Windows"

    The Windows FFI library links against ONNX Runtime dynamically. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

[API Reference →](../reference/api-c.md)

### Dart / Flutter { #dart }

Pure-Dart and Flutter consumers share the same package. Dart SDK 3.0 or higher is required. Flutter is supported on macOS, iOS, Android, Linux, and Windows; Flutter Web is not supported because the runtime is a native dynamic library delivered via flutter_rust_bridge. For Flutter projects use `flutter pub add xberg` instead of `dart pub add xberg`.

### Kotlin { #kotlin }

Kotlin/JVM consumers use the Java artifact (`io.xberg:xberg`) directly; Kotlin interoperates with the generated Java records and static facade.

Kotlin Android uses the Android AAR (`io.xberg:xberg-android`). It embeds JNI libraries for `arm64-v8a` and `x86_64`, targets Android API 21+, and uses the `android-target` feature set, which excludes ORT-dependent inference features.

### Swift { #swift }

Swift Package Manager from `swift-tools-version: 6.0` upward. Targets macOS 13+ and iOS 16+; Linux is not currently declared in `Package.swift`. Once the package ships its `binaryTarget`, no manual cargo build is needed; in the interim, building the library locally requires `cargo build -p xberg-swift` against the workspace.

### Zig { #zig }

Requires Zig 0.16.0 or higher (declared via `minimum_zig_version` in `build.zig.zon`). The Zig binding consumes the C FFI surface from `xberg-ffi` via `linkSystemLibrary`; the build expects the consumer to provide a search path to the prebuilt `libxberg_ffi` and the C header `xberg.h`. The `zig fetch` command above pins the source archive in `build.zig.zon`; wire it into `build.zig` via `b.dependency("xberg", ...)`.

---

## Development setup

For working on the Xberg repository itself:

```bash
task setup      # installs all language toolchains
task lint        # linters across all languages
task dev:test    # full test suite
```

See [Contributing](../contributing.md) for conventions and expectations.
