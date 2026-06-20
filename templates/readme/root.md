# Kreuzberg

{% include 'partials/badges.html.jinja' %}

Extract text, metadata, transcripts, and code intelligence from 96 file formats and 306 programming languages at native speeds without needing a GPU.

## What and Why?

Kreuzberg is a document-intelligence framework with a Rust core and native bindings for 16 languages. It turns documents, images, audio, and source code into clean, structured text — extracting tables, metadata, transcripts, and code intelligence from 96 file formats and 306 programming languages.

Modern AI and RAG pipelines need fast, reliable extraction without a GPU or a stack of heavyweight dependencies. Kreuzberg delivers that from a single Rust core: SIMD-accelerated parsing, pure-Rust PDF, streaming for multi-GB files, and consistent output across every binding. Run it as a library, CLI, REST API, or MCP server.

OCR (Tesseract, PaddleOCR, EasyOCR, and VLM across 143 vision providers), Whisper audio/video transcription, chunking, language detection, embeddings, and structured LLM extraction are all built in.

### Features

| Feature | Description |
| ------- | ----------- |
| **96 file formats** | PDF, Office, images, HTML/XML, email, archives, and academic formats across 8 categories |
| **306 languages** | Code intelligence — functions, classes, imports, symbols, docstrings — via tree-sitter |
| **Polyglot** | Native bindings for Rust, Python, Node.js, WebAssembly, Ruby, Go, Java, Kotlin, C#, PHP, Elixir, R, Dart, Swift, Zig, and C |
| **OCR** | Tesseract (incl. WASM), PaddleOCR, EasyOCR, and VLM OCR across 143 vision providers — extensible via plugins |
| **Transcription** | Whisper ONNX transcripts for MP3, M4A, WAV, WebM, and MP4 audio tracks |
| **LLM intelligence** | Structured JSON extraction, embeddings, and VLM OCR through [liter-llm](https://github.com/kreuzberg-dev/liter-llm), including local engines |
| **Deployment** | Use as a library, CLI tool, REST API server, or MCP server |
| **High performance** | Rust core with pure-Rust PDF, SIMD optimizations, full parallelism, and streaming for multi-GB files |
| **Token-efficient output** | TOON wire format uses ~30–50% fewer tokens than JSON for LLM/RAG pipelines |
| **Extensible** | Plugin system for custom OCR backends, validators, post-processors, extractors, and renderers |

### Supported Formats

96 file formats across 8 categories — Office documents, images (OCR-enabled), web and structured data, email, archives, academic, and audio/video — plus code intelligence for 306 programming languages. See the [format reference](https://docs.kreuzberg.dev/reference/formats/) for the complete list.

<div align="center">
  <a href="https://github.com/kreuzberg-dev/kreuzberg/stargazers">
    <img src="docs/assets/star.gif" alt="Star Kreuzberg on GitHub" width="640">
  </a>
</div>

<p align="center"><strong>⭐ Star this repo to show your support — it helps others discover Kreuzberg.</strong></p>

## Quick Start

### Language Packages

<details open>
<summary><strong>Python</strong></summary>

```sh
pip install kreuzberg
```

```sh
uv add kreuzberg
```

See [Python README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/python) for full documentation.

</details>

<details>
<summary><strong>Node.js</strong></summary>

```sh
npm install @kreuzberg/node
```

See [Node.js README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-node) for full documentation.

</details>

<details>
<summary><strong>Rust</strong></summary>

```sh
cargo add kreuzberg
```

See [Rust README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg) for full documentation.

</details>

<details>
<summary><strong>Go</strong></summary>

```sh
go get github.com/kreuzberg-dev/kreuzberg/v5
```

See [Go README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go/v5) for full documentation.

</details>

<details>
<summary><strong>Java</strong></summary>

Available on Maven Central as `dev.kreuzberg:kreuzberg`. See [Java README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/java) for the dependency snippet and current version.

</details>

<details>
<summary><strong>C#</strong></summary>

```sh
dotnet add package Kreuzberg
```

See [C# README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/csharp) for full documentation.

</details>

<details>
<summary><strong>Ruby</strong></summary>

```sh
gem install kreuzberg
```

See [Ruby README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/ruby) for full documentation.

</details>

<details>
<summary><strong>PHP</strong></summary>

```sh
composer require kreuzberg/kreuzberg
```

See [PHP README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/php) for full documentation.

</details>

<details>
<summary><strong>Elixir</strong></summary>

Add `{:kreuzberg, "~> 5.0"}` to your `mix.exs` dependencies. See [Elixir README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/elixir) for full documentation.

</details>

<details>
<summary><strong>WebAssembly</strong></summary>

```sh
npm install @kreuzberg/wasm
```

See [WebAssembly README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-wasm) for full documentation.

</details>

<details>
<summary><strong>R</strong></summary>

Install from r-universe. See [R README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/r) for full documentation.

</details>

<details>
<summary><strong>Kotlin (Android)</strong></summary>

Available on Maven Central as `dev.kreuzberg:kreuzberg-android`. See [Kotlin README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/kotlin-android) for the dependency snippet and current version.

</details>

<details>
<summary><strong>Swift</strong></summary>

Add via Swift Package Manager. See [Swift README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/swift) for full documentation.

</details>

<details>
<summary><strong>Dart / Flutter</strong></summary>

```sh
dart pub add kreuzberg
```

See [Dart README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/dart) for full documentation.

</details>

<details>
<summary><strong>Zig</strong></summary>

Add via `zig fetch`. See [Zig README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/zig) for full documentation.

</details>

<details>
<summary><strong>C/C++ (FFI)</strong></summary>

Build from source as part of this workspace. See [C (FFI) README](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-ffi) for full documentation.

</details>

<details>
<summary><strong>CLI</strong></summary>

```sh
brew install kreuzberg-dev/tap/kreuzberg
```

See [CLI usage](https://docs.kreuzberg.dev/cli/usage/) for full documentation.

</details>

<details>
<summary><strong>Docker</strong></summary>

```sh
docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest
```

See [Docker guide](https://docs.kreuzberg.dev/guides/docker/) for API, CLI, and MCP server modes.

</details>

### AI Coding Assistants

Install the Kreuzberg plugin from the [`kreuzberg-dev/plugins`](https://github.com/kreuzberg-dev/plugins) marketplace. It ships the Kreuzberg agent skills (extraction APIs, OCR backends, configuration, language conventions) and works with every major coding agent — expand your harness below.

<details open>
<summary><strong>Claude Code</strong></summary>

```text
/plugin marketplace add kreuzberg-dev/plugins
/plugin install kreuzberg@kreuzberg
```

</details>

<details>
<summary><strong>Codex CLI</strong></summary>

```text
/plugins add https://github.com/kreuzberg-dev/plugins
```

Then search for `kreuzberg` and select **Install Plugin**.

</details>

<details>
<summary><strong>Cursor</strong></summary>

Settings → Plugins → Add from URL → `https://github.com/kreuzberg-dev/plugins`, then select **kreuzberg**.

</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

```text
gemini extensions install https://github.com/kreuzberg-dev/plugins
```

</details>

<details>
<summary><strong>Factory Droid</strong></summary>

```text
droid plugin marketplace add https://github.com/kreuzberg-dev/plugins
droid plugin install kreuzberg@kreuzberg
```

</details>

<details>
<summary><strong>GitHub Copilot CLI</strong></summary>

```text
copilot plugin marketplace add https://github.com/kreuzberg-dev/plugins
copilot plugin install kreuzberg@kreuzberg
```

</details>

<details>
<summary><strong>opencode</strong></summary>

Add the package to `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["@kreuzberg/opencode-kreuzberg"]
}
```

</details>

## Documentation

Full guides, API references for every binding, and the complete format and configuration reference live at **[kreuzberg.dev](https://kreuzberg.dev/)**. Try it in the browser with the [live demo](https://docs.kreuzberg.dev/demo.html).

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Join our [Discord community](https://discord.gg/xt9WY3GnKR) for questions and discussion.

## Part of Kreuzberg.dev

- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.

## License

Elastic License 2.0 (ELv2) — see [LICENSE](LICENSE) for details. See [the Elastic License](https://www.elastic.co/licensing/elastic-license) for the full text.
