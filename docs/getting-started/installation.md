---
description: "Install Kreuzberg in your language of choice — Python, TypeScript, Rust, Go, Ruby, Java, C#, PHP, Elixir, R, C, or via CLI/Docker."
---

# Installation

Kreuzberg has native bindings for 12 languages and a CLI. Pick your stack, run one command, and you're ready to extract.

All packages ship **prebuilt binaries** for Linux (x86_64 / aarch64), macOS (Apple Silicon), and Windows — no compilation required.

## Choose your language

<div class="grid cards" markdown>

-   :fontawesome-brands-python:{ .lg .middle } **Python**

    ---

    ```bash
    pip install kreuzberg
    ```

    [Quick Start](quickstart.md) · [API Reference](../reference/api-python.md)

-   :fontawesome-brands-node-js:{ .lg .middle } **TypeScript (Node.js / Bun)**

    ---

    ```bash
    npm install @kreuzberg/node
    ```

    Native NAPI bindings — fastest option for server-side JS.

    [API Reference](../reference/api-typescript.md) · [Details](#typescript)

-   :fontawesome-brands-js:{ .lg .middle } **TypeScript (Browser / Edge)**

    ---

    ```bash
    npm install @kreuzberg/wasm
    ```

    Pure WASM — works in browsers, Deno, Cloudflare Workers.

    [API Reference](../reference/api-wasm.md) · [Details](#typescript)

-   :fontawesome-brands-rust:{ .lg .middle } **Rust**

    ---

    ```bash
    cargo add kreuzberg
    ```

    [API Reference](../reference/api-rust.md)

-   :fontawesome-brands-golang:{ .lg .middle } **Go**

    ---

    ```bash
    go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest
    ```

    Requires Go 1.26+, cgo, and `libkreuzberg_ffi.a`.

    [API Reference](../reference/api-go.md)

-   :fontawesome-brands-java:{ .lg .middle } **Java**

    ---

    ```gradle
    implementation 'dev.kreuzberg:kreuzberg:4.4.2'
    ```

    Requires Java 25+ (FFM API). [Maven Central →](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)

    [API Reference](../reference/api-java.md) · [Details](#java)

-   :material-language-ruby:{ .lg .middle } **Ruby**

    ---

    ```bash
    gem install kreuzberg
    ```

    Requires Ruby 3.2+.

    [API Reference](../reference/api-ruby.md)

-   :material-language-csharp:{ .lg .middle } **C# / .NET** <span class="version-badge unreleased">Unreleased</span>

    ---

    ```bash
    dotnet add package Kreuzberg
    ```

    Requires .NET 10.0+.

    [API Reference](../reference/api-csharp.md) · [Guide](../guides/csharp.md)

-   :fontawesome-brands-php:{ .lg .middle } **PHP** <span class="version-badge unreleased">Unreleased</span>

    ---

    ```bash
    composer require kreuzberg/kreuzberg
    ```

    Requires PHP 8.2+ with `ext-ffi`.

    [API Reference](../reference/api-php.md)

-   :material-language-elixir:{ .lg .middle } **Elixir**

    ---

    ```elixir
    {:kreuzberg, "~> 4.0"}
    ```

    Add to `mix.exs`, then `mix deps.get`.

    [API Reference](../reference/api-elixir.md) · [Details](#elixir)

-   :material-language-r:{ .lg .middle } **R** <span class="version-badge unreleased">Unreleased</span>

    ---

    ```r
    install.packages("kreuzberg",
      repos = "https://kreuzberg-dev.r-universe.dev")
    ```

    Requires R ≥ 4.2, Rust toolchain.

    [API Reference](../reference/api-r.md)

-   :material-console:{ .lg .middle } **CLI / Docker**

    ---

    ```bash
    brew install kreuzberg-dev/tap/kreuzberg
    ```

    [CLI Usage](../cli/usage.md) · [Details](#cli--docker)

</div>

---

## Language-specific notes

Most languages need nothing beyond the install command above. The sections below cover edge cases and alternative install methods only where they matter.

### TypeScript

Kreuzberg ships two npm packages optimized for different runtimes:

| Package | Best for | Performance |
|---|---|---|
| `@kreuzberg/node` | Node.js, Bun — server-side apps | Native (100%) |
| `@kreuzberg/wasm` | Browsers, Deno, Cloudflare Workers | WASM (~60-80%) |

Both are also available via **pnpm** (`pnpm add`) and **yarn** (`yarn add`).

!!! note "pnpm workspaces"
    In monorepos, add this to your root `.npmrc` so platform-specific optional deps resolve correctly:
    ```ini
    auto-install-peers=true
    ```

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

### Java

=== "Maven"

    ```xml
    <dependency>
        <groupId>dev.kreuzberg</groupId>
        <artifactId>kreuzberg</artifactId>
        <version>4.4.2</version>
    </dependency>
    ```

=== "Gradle"

    ```gradle
    implementation 'dev.kreuzberg:kreuzberg:4.4.2'
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

### Rust

Enable features selectively:

Add to Maven `pom.xml`:

```xml title="pom.xml"
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.4.5</version>
</dependency>
```

Or pin a version in `Cargo.toml`:

```gradle title="build.gradle"
implementation 'dev.kreuzberg:kreuzberg:4.4.5'
```

### C / C++ <span class="version-badge unreleased">Unreleased</span>

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

[API Reference →](../reference/api-c.md)

### CLI / Docker

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

[CLI Usage →](../cli/usage.md) · [API Server Guide →](../guides/api-server.md)

---

## System requirements

Most packages ship prebuilt binaries — you don't need anything extra. These matter only when building from source or enabling OCR:

| Dependency | When you need it |
|---|---|
| Rust toolchain (`rustup`) | Building any native binding from source |
| C/C++ compiler | Building native bindings (Xcode CLI tools / `build-essential` / MSVC) |
| Tesseract OCR | Optional — `brew install tesseract` / `apt install tesseract-ocr` |
| PDFium | Auto-fetched during builds |

WASM packages (`@kreuzberg/wasm`) have **zero** system dependencies.

---

## Development setup

Working on the Kreuzberg repo itself:

```bash
task setup      # installs all language toolchains
task lint        # linters across all languages
task dev:test    # full test suite
```

See [Contributing](../contributing.md) for conventions and expectations.
