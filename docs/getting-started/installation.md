# Installation

Kreuzberg is available in multiple formats optimized for different runtimes: native bindings for server-side languages and WebAssembly for JavaScript environments. Choose the package that matches your runtime environment.

## Which Package Should I Install?

| Runtime/Environment | Package | Performance | Best For |
|-------------------|---------|-------------|----------|
| **Node.js** | `@kreuzberg/node` | ⚡ Fastest (native) | Server-side Node applications, native performance |
| **Bun** | `@kreuzberg/node` | ⚡ Fastest (native) | Bun runtime, native performance |
| **Browser** | `@kreuzberg/wasm` | ✓ Good (WASM) | Client-side apps, no native dependencies |
| **Deno** | `@kreuzberg/wasm` | ✓ Good (WASM) | Deno runtime, pure WASM execution |
| **Cloudflare Workers** | `@kreuzberg/wasm` | ✓ Good (WASM) | Serverless functions, edge computing |
| **Python** | `kreuzberg` | ⚡ Fastest (native) | Server-side Python, native performance |
| **Ruby** | `kreuzberg` | ⚡ Fastest (native) | Ruby applications, native performance |
| **Elixir** | `kreuzberg` | ⚡ Fastest (native) | Elixir/Phoenix apps, BEAM runtime |
| **Java** | `dev.kreuzberg:kreuzberg` | ⚡ Fastest (native) | Server-side Java apps, FFM API |
| **Go** | `github.com/.../go/v4` | ⚡ Fastest (native) | Server-side Go, cgo bindings |
| **PHP** | `kreuzberg/kreuzberg` | ⚡ Fastest (native) | PHP applications, ext-ffi |
| **C#/.NET** | `Kreuzberg` (NuGet) | ⚡ Fastest (native) | .NET applications, P/Invoke |
| **Rust** | `kreuzberg` crate | ⚡ Fastest (native) | Rust projects, full control |
| **CLI/Docker** | `kreuzberg-cli` | ⚡ Fastest (native) | Command-line usage, batch processing |

### Performance Notes

- **Native bindings** (@kreuzberg/node, kreuzberg Python/Ruby): ~100% performance, compiled C/C++ speed, full feature access
- **WASM**: ~60-80% performance relative to native, pure JavaScript, zero native dependencies, works anywhere JavaScript runs

Choose **native bindings** for server-side applications requiring maximum performance. Choose **WASM** for browser/edge environments or when avoiding native dependencies is essential.

## Architecture Support

All Kreuzberg packages support both x86_64 and aarch64 (ARM64) architectures:

- **x86_64**: Linux, Windows
- **aarch64 (ARM64)**: Linux, macOS (Apple Silicon)

Precompiled binaries are available for all major platform combinations. On aarch64 systems, installation is as fast as on x86_64—no compilation required.

## System Dependencies

System dependencies vary by package:

### Native Bindings Only (Python, Ruby, Node.js)

- Rust toolchain (`rustup`) for building the core and bindings.
- C/C++ build tools (Xcode Command Line Tools on macOS, MSVC Build Tools on Windows, `build-essential` on Linux).
- Tesseract OCR (optional but recommended). Install via Homebrew (`brew install tesseract`), apt (`sudo apt install tesseract-ocr`), or Windows installers.
- Pdfium binaries are fetched automatically during builds; no manual steps required.

### WASM (@kreuzberg/wasm)

No system dependencies required. WASM binaries are prebuilt and included in the npm package.

## Python

```bash title="Terminal"
pip install kreuzberg
```

```bash title="Terminal"
uv pip install kreuzberg
```

Optional extras:

```bash title="Terminal"
pip install 'kreuzberg[easyocr]'
```

```bash title="Terminal"
pip install 'kreuzberg[paddleocr]'
```

PyPI wheels include precompiled binaries for Linux (x86_64, aarch64), macOS (Apple Silicon), and Windows.

Next steps: [Python Quick Start](quickstart.md) • [Python API Reference](../reference/api-python.md)

## TypeScript (Node.js / Bun) - Native

Use `@kreuzberg/node` for server-side TypeScript/Node.js applications requiring maximum performance.

```bash title="Terminal"
npm install @kreuzberg/node
```

```bash title="Terminal"
pnpm add @kreuzberg/node
```

```bash title="Terminal"
yarn add @kreuzberg/node
```

The package ships with prebuilt N-API binaries for Linux, macOS (Apple Silicon), and Windows. If you need to build from source, ensure Rust is available on your PATH and rerun the install command.

N-API binaries support Linux x86_64, Linux aarch64, macOS (Apple Silicon), and Windows.

**Note for pnpm workspaces**: If you're using pnpm in a monorepo/workspace setup, you may need to configure automatic peer dependency installation. Add the following to your `.npmrc` file in the workspace root:

```ini title=".npmrc"
auto-install-peers=true
```

This ensures the platform-specific optional dependencies are installed correctly.

**Performance**: Native bindings provide ~100% performance through NAPI-RS compiled bindings.

Next steps: [TypeScript Quick Start](../guides/extraction.md#typescript-nodejs) • [TypeScript API Reference](../reference/api-typescript.md)

## TypeScript/JavaScript (Browser / Edge) - WASM

Use `@kreuzberg/wasm` for client-side JavaScript applications, serverless environments, and runtimes where native binaries are unavailable or undesirable.

### When to Use WASM

- Client-side browser applications
- Cloudflare Workers or other edge computing platforms
- Deno or other JavaScript runtimes
- Environments where native dependencies cannot be installed
- Scenarios where package size reduction matters

### When to Use Native (@kreuzberg/node)

- Server-side Node.js applications (10-40% faster)
- Bun runtime
- Maximum performance requirements
- Full feature access with no limitations

### Installation

```bash title="Terminal"
npm install @kreuzberg/wasm
```

```bash title="Terminal"
pnpm add @kreuzberg/wasm
```

```bash title="Terminal"
yarn add @kreuzberg/wasm
```

**Performance**: WASM bindings provide ~60-80% of native performance with zero native dependencies.

### Browser Usage

```html title="index.html"
<!DOCTYPE html>
<html>
<head>
    <script type="module">
        import { initWasm, extractFromFile } from '@kreuzberg/wasm';

        window.initKreuzberg = async () => {
            await initWasm();
            console.log('Kreuzberg initialized');
        };

        window.extractFile = async (file) => {
            const result = await extractFromFile(file);
            console.log(result.content);
        };
    </script>
</head>
<body>
    <input type="file" id="file" />
</body>
</html>
```

### Deno

```typescript title="main.ts"
import { initWasm, extractFile } from 'npm:@kreuzberg/wasm';

await initWasm();
const result = await extractFile('./document.pdf');
console.log(result.content);
```

### Cloudflare Workers

```typescript title="worker.ts"
import { initWasm, extractBytes } from '@kreuzberg/wasm';

export default {
    async fetch(request: Request): Promise<Response> {
        await initWasm();

        const file = await request.arrayBuffer();
        const bytes = new Uint8Array(file);
        const result = await extractBytes(bytes, 'application/pdf');

        return new Response(JSON.stringify({ content: result.content }));
    },
};
```

### Optional Features

OCR support requires browser Web Workers and additional memory. Enable it selectively:

```typescript title="ocr-example.ts"
import { initWasm, enableOcr, extractFromFile } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file');
fileInput.addEventListener('change', async (e) => {
    const file = e.target.files[0];

    if (file.type.startsWith('image/')) {
        // Enable OCR only for images
        await enableOcr();
    }

    const result = await extractFromFile(file);
    console.log(result.content);
});
```

WASM bindings work in:
- Modern browsers (Chrome 74+, Firefox 79+, Safari 14+, Edge 79+)
- Node.js 22+
- Deno 1.35+
- Cloudflare Workers
- Other JavaScript runtimes with WebAssembly support

Next steps: [WASM Quick Start](quickstart.md#basic-extraction) • [WASM API Reference](../reference/api-wasm.md)

## Ruby

```bash title="Terminal"
gem install kreuzberg
```

Bundler projects can add it to the Gemfile:

```ruby title="Gemfile"
gem 'kreuzberg', '~> 4.0'
```

Native extension builds require Ruby 3.3+ plus MSYS2 on Windows. Set `RBENV_VERSION`/`chruby` accordingly and ensure `bundle config set build.kreuzberg --with-cflags="-std=c++17"` if your compiler defaults are older.

Next steps: [Ruby Quick Start](../guides/extraction.md#ruby) • [Ruby API Reference](../reference/api-ruby.md)

## Rust

```bash title="Terminal"
cargo add kreuzberg
```

Or edit `Cargo.toml` manually:

```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
```

Enable optional features as needed:

```bash title="Terminal"
cargo add kreuzberg --features "excel stopwords ocr"
```

Next steps: [Rust API Reference](../reference/api-rust.md)

## Elixir

Add Kreuzberg to your `mix.exs`:

```elixir title="mix.exs"
def deps do
  [
    {:kreuzberg, "~> 4.0"}
  ]
end
```

Then install:

```bash title="Terminal"
mix deps.get
```

The package ships with prebuilt native binaries for Linux, macOS (Apple Silicon), and Windows via RustlerPrecompiled. If prebuilt binaries are unavailable for your platform, the package will automatically fall back to compiling from source, which requires Rust to be available on your PATH.

Precompiled NIF binaries for Linux (x86_64, aarch64), macOS (Apple Silicon), and Windows.

**Performance**: Native NIF bindings provide ~100% performance through Rustler compiled bindings.

Next steps: [Elixir Quick Start](quickstart.md) • [Elixir API Reference](../reference/api-elixir.md)

## Java

Add to Maven `pom.xml`:

```xml title="pom.xml"
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0</version>
</dependency>
```

Or Gradle:

```gradle title="build.gradle"
implementation 'dev.kreuzberg:kreuzberg:4.0.0'
```

**Requirements:** Java 25+ (FFM/Panama API)

The package bundles native libraries for Linux (x86_64, aarch64), macOS (Apple Silicon), and Windows.

View on [Maven Central](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg).

Next steps: [Java Quick Start](quickstart.md) • [Java API Reference](../reference/api-java.md)

## Go

```bash title="Terminal"
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@latest
```

**Requirements:** Go 1.25+ with cgo, C compiler, libkreuzberg_ffi.a static library at build time

Kreuzberg Go binaries are **statically linked** — once built, they are self-contained and require no runtime library dependencies.

For external projects, download pre-built static libraries from [GitHub Releases](https://github.com/kreuzberg-dev/kreuzberg/releases) or build from source.

Next steps: [Go Quick Start](quickstart.md) • [Go API Reference](../reference/api-go.md)

## PHP

```bash title="Terminal"
composer require kreuzberg/kreuzberg
```

**Requirements:** PHP 8.2+, ext-ffi enabled

The package includes prebuilt native extensions for Linux (x86_64, aarch64), macOS (Apple Silicon), and Windows.

Next steps: [PHP Quick Start](quickstart.md) • [PHP API Reference](../reference/api-php.md)

## C# / .NET

```bash title="Terminal"
dotnet add package Kreuzberg
```

Or via Package Manager Console:

```powershell title="Package Manager Console"
Install-Package Kreuzberg
```

**Requirements:** .NET 10.0+

The package includes prebuilt native libraries for Linux (x86_64, aarch64), macOS (Apple Silicon), and Windows.

Next steps: [C# Quick Start](quickstart.md) • [C# API Reference](../reference/api-csharp.md) • [C# Bindings Guide](../guides/csharp.md)

## CLI

Homebrew tap (macOS / Linux):

```bash title="Terminal"
brew install kreuzberg-dev/tap/kreuzberg
```

Cargo install:

```bash title="Terminal"
cargo install kreuzberg-cli
```

Docker image:

```bash title="Terminal"
docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest       # Core image with essential features
docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest   # Full image with all extensions
```

Next steps: [CLI Usage](../cli/usage.md) • [API Server Guide](../guides/api-server.md)

## Development Environment

To work on the repository itself:

```bash title="Terminal"
task setup      # Install all dependencies (Python, Node.js, Ruby, Rust)
task lint       # Run linters across all languages
task dev:test   # Execute full test suite (Rust, Python, Ruby, TypeScript)
```

See [Contributing](../contributing.md) for branch naming, coding conventions, and test expectations.
