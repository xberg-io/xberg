---
description: "Install Kreuzberg — pick Python, TypeScript, Rust, Go, CLI/Docker, or any of 12 supported languages."
---

# Installation

Kreuzberg ships native bindings for 12 languages and a standalone CLI. Pick your stack, run one command, and start extracting.

Every package includes **prebuilt binaries** for Linux (x86_64 / aarch64), macOS (Apple Silicon), and Windows — no compile step needed.

!!! warning "Windows — ONNX Runtime required for Go, Elixir, and C/C++"
    Go, Elixir, and C/C++ bindings on Windows link against ONNX Runtime dynamically. You must have `onnxruntime.dll` on your `PATH` at runtime. Download it from the [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) (e.g. `onnxruntime-win-x64-1.24.1.zip`). Python, TypeScript, Java, C#, Ruby, PHP, and WASM are unaffected.

<div class="cli-hero" markdown>

## :material-console: CLI / Docker { #cli--docker }

The fastest way to try Kreuzberg - no SDK, no code, just your terminal.

=== "Install script"

    ```bash
    curl -fsSL https://raw.githubusercontent.com/kreuzberg-dev/kreuzberg/main/scripts/install.sh | bash
    ```

=== "Homebrew"

    ```bash
    brew install kreuzberg-dev/tap/kreuzberg
    ```

=== "Cargo"

    ```bash
    cargo install kreuzberg-cli
    ```

=== "Docker (CLI image)"

    ```bash
    docker pull ghcr.io/kreuzberg-dev/kreuzberg-cli:latest
    docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg-cli:latest extract /data/document.pdf
    ```

=== "Docker (full image)"

    ```bash
    docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest
    ```

[CLI Usage](../cli/usage.md){ .install-btn .install-btn--ghost }
[API Server Guide](../guides/api-server.md){ .install-btn .install-btn--solid }

</div>

---

## Choose your language

<div class="grid cards install-cards" markdown>

-   :fontawesome-brands-python:{ .lg .middle } **Python**

    ---

    ```bash
    pip install kreuzberg
    ```

    [API Reference](../reference/api-python.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :fontawesome-brands-node-js:{ .lg .middle } **TypeScript (Node.js / Bun)**

    ---

    ```bash
    npm install @kreuzberg/node
    ```

    [API Reference](../reference/api-typescript.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](#typescript){ .install-btn .install-btn--solid }

-   :fontawesome-brands-js:{ .lg .middle } **TypeScript (Browser / Edge)**

    ---

    ```bash
    npm install @kreuzberg/wasm
    ```

    [API Reference](../reference/api-wasm.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](#typescript){ .install-btn .install-btn--solid }

-   :fontawesome-brands-rust:{ .lg .middle } **Rust**

    ---

    ```bash
    cargo add kreuzberg
    ```

    [API Reference](../reference/api-rust.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :fontawesome-brands-golang:{ .lg .middle } **Go**

    ---

    ```bash
    go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest
    ```

    [API Reference](../reference/api-go.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :fontawesome-brands-java:{ .lg .middle } **Java**

    ---

    ```gradle
    implementation 'dev.kreuzberg:kreuzberg:4.7.3'
    ```

    [API Reference](../reference/api-java.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](#java){ .install-btn .install-btn--solid }

-   :material-language-ruby:{ .lg .middle } **Ruby**

    ---

    ```bash
    gem install kreuzberg
    ```

    [API Reference](../reference/api-ruby.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :material-language-csharp:{ .lg .middle } **C# / .NET**

    ---

    ```bash
    dotnet add package Kreuzberg
    ```

    [API Reference](../reference/api-csharp.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](../guides/csharp.md){ .install-btn .install-btn--solid }

-   :fontawesome-brands-php:{ .lg .middle } **PHP**

    ---

    ```bash
    composer require kreuzberg/kreuzberg
    ```

    [API Reference](../reference/api-php.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :simple-elixir:{ .lg .middle } **Elixir**

    ---

    ```elixir
    {:kreuzberg, "~> 4.0"}
    ```

    [API Reference](../reference/api-elixir.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](#elixir){ .install-btn .install-btn--solid }

-   :simple-r:{ .lg .middle } **R**

    ---

    ```r
    install.packages("kreuzberg",
      repos = "https://kreuzberg-dev.r-universe.dev")
    ```

    [API Reference](../reference/api-r.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](quickstart.md){ .install-btn .install-btn--solid }

-   :simple-cplusplus:{ .lg .middle } **C / C++**

    ---

    ```bash
    cargo build -p kreuzberg-ffi
    ```

    [API Reference](../reference/api-c.md){ .install-btn .install-btn--ghost }
    [:material-lightning-bolt: Quick Start](#c-c-v4.5.3){ .install-btn .install-btn--solid }

</div>

---

## System requirements

Most of the time you won't need anything beyond the install command above. The table below only matters if you're building from source or want OCR:

| Dependency | When you need it |
|---|---|
| Rust toolchain (`rustup`) | Building any native binding from source |
| C/C++ compiler | Building native bindings (Xcode CLI tools / `build-essential` / MSVC) |
| Tesseract OCR | Optional — `brew install tesseract` / `apt install tesseract-ocr` |
| PDFium | Auto-fetched during builds |

The WASM package (`@kreuzberg/wasm`) has **zero** system dependencies.

---

## Language-specific notes

For most languages the install command above is all you need. The sections below cover edge cases and alternative install methods where they come up.

### TypeScript

Two npm packages target different runtimes:

| Package | Best for | Performance |
|---|---|---|
| `@kreuzberg/node` | Node.js, Bun — server-side apps | Native (100%) |
| `@kreuzberg/wasm` | Browsers, Deno, Cloudflare Workers | WASM (~60-80%) |

Both work with **pnpm** (`pnpm add`) and **yarn** (`yarn add`) as well.

!!! note "pnpm workspaces"
    In monorepos, add this to your root `.npmrc` so platform-specific optional deps resolve correctly:
    ```ini
    auto-install-peers=true```

??? example "WASM — Browser usage"
    ```html
    <script type="module">
      import { initWasm, extractFromFile } from "@kreuzberg/wasm";

      await initWasm();

      const input = document.getElementById("file");
      input.addEventListener("change", async (e) => {
        const result = await extractFromFile(e.target.files[0]);
        console.log(result.content);
      });
    </script>
    <input type="file" id="file" />
    ```

??? example "WASM — Deno"
    ```typescript
    import { initWasm, extractFile } from "npm:@kreuzberg/wasm";

    await initWasm();
    const result = await extractFile("./document.pdf");
    console.log(result.content);
    ```

??? example "WASM — Cloudflare Workers"
    ```typescript
    import { initWasm, extractBytes } from "@kreuzberg/wasm";

    export default {
      async fetch(request: Request): Promise<Response> {
        await initWasm();
        const bytes = new Uint8Array(await request.arrayBuffer());
        const result = await extractBytes(bytes, "application/pdf");
        return Response.json({ content: result.content });
      },
    };
    ```

**Supported runtimes:** Chrome 74+, Firefox 79+, Safari 14+, Edge 79+, Node.js 22+, Deno 1.35+, Cloudflare Workers.

!!! warning "WASM Platform Limitations"
    The WASM binding does not support:

    - **Layout detection** (RT-DETR model inference requires ONNX Runtime unavailable in WebAssembly)
    - **Hardware acceleration config** (single-threaded WASM, no GPU access)
    - **Concurrency config** (single-threaded environment, `maxThreads` is ignored)
    - **Email codepage config** (EmailConfig not available)

    All other features (text extraction, OCR via Tesseract WASM, chunking, embeddings, metadata, tables, language detection, image extraction) work fully in WASM. See the [WASM API Reference](../reference/api-wasm.md#platform-limitations) for details.

### Java

=== "Maven"

    ```xml
    <dependency>
        <groupId>dev.kreuzberg</groupId>
        <artifactId>kreuzberg</artifactId>
        <version>4.7.3</version>
    </dependency>
    ```

=== "Gradle"

    ```gradle
    implementation 'dev.kreuzberg:kreuzberg:4.7.3'
    ```

Requires Java 25+ (FFM/Panama API). Native libraries are bundled in the JAR.

### Elixir

Add to `mix.exs`:

```elixir
def deps do
  [
    {:kreuzberg, "~> 4.0"}
  ]
end
```

```bash
mix deps.get
```

Ships prebuilt NIF binaries via RustlerPrecompiled. Falls back to compiling from source if no prebuilt matches your platform (requires Rust).

!!! warning "Windows"
    The Windows NIF links against ONNX Runtime dynamically. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

### Go

```bash
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest
```

!!! warning "Windows"
    The Go binding links against ONNX Runtime dynamically on Windows. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

!!! note "Windows feature limitations"
    The Go and C/C++ bindings on Windows (MinGW/GNU target) do not include **PaddleOCR**, **layout detection**, or **auto-rotate**. Tesseract OCR and all other features work normally. These limitations apply only to Windows; Linux and macOS builds include the full feature set.

### Rust

Enable features selectively in `Cargo.toml`:

```toml title="Cargo.toml"
[dependencies]
kreuzberg = { version = "4", features = ["tokio-runtime"] }
# Optional features: pdf, ocr, chunking
```

### C / C++

Build the FFI library from source:

```bash
cargo build --release -p kreuzberg-ffi
```

This produces `libkreuzberg_ffi.a` and a header at `crates/kreuzberg-ffi/kreuzberg.h`. Link into your project:

```makefile
HEADER_DIR = path/to/crates/kreuzberg-ffi
LIBDIR     = path/to/target/release

CFLAGS  = -Wall -Wextra -I$(HEADER_DIR)
LDFLAGS = -L$(LIBDIR) -lkreuzberg_ffi -lpthread -ldl -lm

my_app: my_app.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)
```

!!! tip "Platform-specific linker flags"
    **macOS:** add `-framework CoreFoundation -framework Security`
    **Windows:** add `-lws2_32 -luserenv -lbcrypt`

!!! warning "Windows"
    The Windows FFI library links against ONNX Runtime dynamically. `onnxruntime.dll` must be on your `PATH` at runtime — see the note at the top of this page.

[API Reference →](../reference/api-c.md)

---

## Development setup

Working on the Kreuzberg repo itself:

```bash
task setup      # installs all language toolchains
task lint        # linters across all languages
task dev:test    # full test suite
```

See [Contributing](../contributing.md) for conventions and expectations.
