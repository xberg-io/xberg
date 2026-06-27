# Xberg

{% include 'partials/badges.html.jinja' %}

Extract text, metadata, transcripts, and code intelligence from 96 file formats and 306 programming languages at native speeds without needing a GPU.

> **Xberg is the next iteration of [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg-v4-lts).** Same document-intelligence engine, rebuilt and rebranded under a fresh v1 line.

## What and Why?

Xberg is a document-intelligence framework with a Rust core and native bindings for 16 languages. It turns documents, images, audio, and source code into clean, structured text — extracting tables, metadata, transcripts, and code intelligence from 96 file formats and 306 programming languages.

Modern AI and RAG pipelines need fast, reliable extraction without a GPU or a stack of heavyweight dependencies. Xberg delivers that from a single Rust core: SIMD-accelerated parsing, pure-Rust PDF, streaming for multi-GB files, and consistent output across every binding. Run it as a library, CLI, REST API, or MCP server.

OCR (Tesseract, PaddleOCR, EasyOCR, and VLM across 143 vision providers), Whisper audio/video transcription, chunking, language detection, embeddings, and structured LLM extraction are all built in.

### Features

| Feature | Description |
| ------- | ----------- |
| **96 file formats** | PDF, Office, images, HTML/XML, email, archives, and academic formats across 8 categories |
| **306 languages** | Code intelligence — functions, classes, imports, symbols, docstrings — via tree-sitter |
| **Polyglot** | Native bindings for Rust, Python, Node.js, WebAssembly, Ruby, Go, Java, Kotlin, C#, PHP, Elixir, R, Dart, Swift, Zig, and C |
| **OCR** | Tesseract (incl. WASM), PaddleOCR, EasyOCR, and VLM OCR across 143 vision providers — extensible via plugins |
| **Transcription** | Whisper ONNX transcripts for MP3, M4A, WAV, WebM, and MP4 audio tracks |
| **LLM intelligence** | Structured JSON extraction, embeddings, and VLM OCR through [liter-llm](https://github.com/xberg-io/liter-llm), including local engines |
| **Deployment** | Use as a library, CLI tool, REST API server, or MCP server |
| **High performance** | Rust core with pure-Rust PDF, SIMD optimizations, full parallelism, and streaming for multi-GB files |
| **Token-efficient output** | TOON wire format uses ~30–50% fewer tokens than JSON for LLM/RAG pipelines |
| **Extensible** | Plugin system for custom OCR backends, validators, post-processors, extractors, and renderers |

### Supported Formats

96 file formats across 8 categories — Office documents, images (OCR-enabled), web and structured data, email, archives, academic, and audio/video — plus code intelligence for 306 programming languages. See the [format reference](https://docs.xberg.io/reference/formats/) for the complete list.

<div align="center">
  <a href="https://github.com/xberg-io/xberg/stargazers">
    <img src="docs/assets/star.gif" alt="Star Xberg on GitHub" width="640">
  </a>
</div>

<p align="center"><strong>⭐ Star this repo to show your support — it helps others discover Xberg.</strong></p>

## Quick Start

### Language Packages

<details open>
<summary><strong>Python</strong></summary>

```sh
pip install xberg
```

```sh
uv add xberg
```

See [Python README](https://github.com/xberg-io/xberg/tree/main/packages/python) for full documentation.

</details>

<details>
<summary><strong>Node.js</strong></summary>

```sh
npm install @xberg-io/xberg
```

See [Node.js README](https://github.com/xberg-io/xberg/tree/main/crates/xberg-node) for full documentation.

</details>

<details>
<summary><strong>Rust</strong></summary>

```sh
cargo add xberg
```

See [Rust README](https://github.com/xberg-io/xberg/tree/main/crates/xberg) for full documentation.

</details>

<details>
<summary><strong>Go</strong></summary>

```sh
go get github.com/xberg-io/xberg
```

See [Go README](https://github.com/xberg-io/xberg/tree/main/packages/go) for full documentation.

</details>

<details>
<summary><strong>Java</strong></summary>

Available on Maven Central as `io.xberg:xberg`. See [Java README](https://github.com/xberg-io/xberg/tree/main/packages/java) for the dependency snippet and current version.

</details>

<details>
<summary><strong>C#</strong></summary>

```sh
dotnet add package Xberg
```

See [C# README](https://github.com/xberg-io/xberg/tree/main/packages/csharp) for full documentation.

</details>

<details>
<summary><strong>Ruby</strong></summary>

```sh
gem install xberg
```

See [Ruby README](https://github.com/xberg-io/xberg/tree/main/packages/ruby) for full documentation.

</details>

<details>
<summary><strong>PHP</strong></summary>

```sh
composer require xberg-io/xberg
```

See [PHP README](https://github.com/xberg-io/xberg/tree/main/packages/php) for full documentation.

</details>

<details>
<summary><strong>Elixir</strong></summary>

Add `{:xberg, "~> 1.0"}` to your `mix.exs` dependencies. See [Elixir README](https://github.com/xberg-io/xberg/tree/main/packages/elixir) for full documentation.

</details>

<details>
<summary><strong>WebAssembly</strong></summary>

```sh
npm install @xberg-io/xberg-wasm
```

See [WebAssembly README](https://github.com/xberg-io/xberg/tree/main/crates/xberg-wasm) for full documentation.

</details>

<details>
<summary><strong>R</strong></summary>

Install from r-universe. See [R README](https://github.com/xberg-io/xberg/tree/main/packages/r) for full documentation.

</details>

<details>
<summary><strong>Kotlin (Android)</strong></summary>

Available on Maven Central as `io.xberg:xberg-android`. See [Kotlin README](https://github.com/xberg-io/xberg/tree/main/packages/kotlin-android) for the dependency snippet and current version.

</details>

<details>
<summary><strong>Swift</strong></summary>

Add via Swift Package Manager. See [Swift README](https://github.com/xberg-io/xberg/tree/main/packages/swift) for full documentation.

</details>

<details>
<summary><strong>Dart / Flutter</strong></summary>

```sh
dart pub add xberg
```

See [Dart README](https://github.com/xberg-io/xberg/tree/main/packages/dart) for full documentation.

</details>

<details>
<summary><strong>Zig</strong></summary>

Add via `zig fetch`. See [Zig README](https://github.com/xberg-io/xberg/tree/main/packages/zig) for full documentation.

</details>

<details>
<summary><strong>C/C++ (FFI)</strong></summary>

Build from source as part of this workspace. See [C (FFI) README](https://github.com/xberg-io/xberg/tree/main/crates/xberg-ffi) for full documentation.

</details>

<details>
<summary><strong>CLI</strong></summary>

```sh
brew install xberg-io/tap/xberg
```

See [CLI usage](https://docs.xberg.io/cli/usage/) for full documentation.

</details>

<details>
<summary><strong>Docker</strong></summary>

```sh
docker pull ghcr.io/xberg-io/xberg:latest
```

See [Docker guide](https://docs.xberg.io/guides/docker/) for API, CLI, and MCP server modes.

</details>

<details>
<summary><strong>MCP Server</strong></summary>

Run Xberg as a [Model Context Protocol](https://modelcontextprotocol.io/) server. The prebuilt
binaries (Homebrew, `install.sh`, Docker) include it; from source, enable the `mcp` feature.

```sh
# Prebuilt (Homebrew / install.sh / Docker) — MCP is included
brew install xberg-io/tap/xberg
xberg mcp                                   # stdio (default)

# From source — enable the mcp feature
cargo install xberg-cli --features mcp
xberg mcp

# HTTP transport instead of stdio
xberg mcp --transport http --host 127.0.0.1 --port 8001
```

Add it to an MCP client (Claude Desktop `claude_desktop_config.json`, Cursor `.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "xberg": { "command": "xberg", "args": ["mcp"] }
  }
}
```

See the [MCP integration guide](https://docs.xberg.io/guides/mcp-integration/) for tools,
resources, prompts, HTTP transport, and configuration.

</details>

### AI Coding Assistants

Install the Xberg plugin from the [`xberg-io/plugins`](https://github.com/xberg-io/plugins) marketplace. It ships the Xberg agent skills (extraction APIs, OCR backends, configuration, language conventions) and works with every major coding agent — expand your harness below.

<details open>
<summary><strong>Claude Code</strong></summary>

```text
/plugin marketplace add xberg-io/plugins
/plugin install xberg@xberg
```

</details>

<details>
<summary><strong>Codex CLI</strong></summary>

```text
/plugins add https://github.com/xberg-io/plugins
```

Then search for `xberg` and select **Install Plugin**.

</details>

<details>
<summary><strong>Cursor</strong></summary>

Settings → Plugins → Add from URL → `https://github.com/xberg-io/plugins`, then select **xberg**.

</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

```text
gemini extensions install https://github.com/xberg-io/plugins
```

</details>

<details>
<summary><strong>Factory Droid</strong></summary>

```text
droid plugin marketplace add https://github.com/xberg-io/plugins
droid plugin install xberg@xberg
```

</details>

<details>
<summary><strong>GitHub Copilot CLI</strong></summary>

```text
copilot plugin marketplace add https://github.com/xberg-io/plugins
copilot plugin install xberg@xberg
```

</details>

<details>
<summary><strong>opencode</strong></summary>

Add the package to `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["@xberg-io/opencode-xberg"]
}
```

</details>

## Documentation

Full guides, API references for every binding, and the complete format and configuration reference live at **[xberg.io](https://xberg.io/)**. Try it in the browser with the [live demo](https://docs.xberg.io/demo.html).

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Join our [Discord community](https://discord.gg/xt9WY3GnKR) for questions and discussion.

## Part of Xberg.dev

- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.

## License

MIT License (MIT) — see [LICENSE](LICENSE) for details. See [the MIT License](https://www.opensource.org/licenses/MIT) for the full text.
