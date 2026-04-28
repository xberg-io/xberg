# WebAssembly

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
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.0.0" alt="Go">
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
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://artifacthub.io/packages/search?repo=kreuzberg">
    <img src="https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/kreuzberg" alt="Artifact Hub">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-007ec6" alt="Documentation">
  </a>
  <a href="https://docs.kreuzberg.dev/demo.html">
    <img src="https://img.shields.io/badge/%E2%96%B6%EF%B8%8F_Live_Demo-007ec6" alt="Live Demo">
  </a>
  <a href="https://huggingface.co/Kreuzberg">
    <img src="https://img.shields.io/badge/%F0%9F%A4%97_Hugging_Face-007ec6" alt="Hugging Face">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Extract text, tables, images, and metadata from 91+ file formats and 248 programming languages including PDF, Office documents, and images. WebAssembly bindings for browsers, Deno, and Cloudflare Workers with portable deployment and multi-threading support.

## Installation

### Package Installation


Install via one of the supported package managers:



**npm:**
```bash
npm install @kreuzberg/wasm
```




**pnpm:**
```bash
pnpm add @kreuzberg/wasm
```




**yarn:**
```bash
yarn add @kreuzberg/wasm
```





### System Requirements

- Modern browser with WebAssembly support, or Deno 1.0+, or Cloudflare Workers
- Optional: [Tesseract WASM](https://github.com/naptha/tesseract.js) for OCR functionality


## Quick Start

### Basic Extraction

Extract text, metadata, and structure from any supported document format:

```ts
import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function main() {
	await initWasm();

	const buffer = await fetch("document.pdf").then((r) => r.arrayBuffer());
	const bytes = new Uint8Array(buffer);

	const result = await extractBytes(bytes, "application/pdf");

	console.log("Extracted content:");
	console.log(result.content);
	console.log("MIME type:", result.mimeType);
	console.log("Metadata:", result.metadata);
}

main().catch(console.error);
```

### Common Use Cases

#### Extract with Custom Configuration

Most use cases benefit from configuration to control extraction behavior:


**With OCR (for scanned documents):**

```ts
import { enableOcr, extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractWithOcr() {
	await initWasm();

	try {
		await enableOcr();
		console.log("OCR enabled successfully");
	} catch (error) {
		console.error("Failed to enable OCR:", error);
		return;
	}

	const bytes = new Uint8Array(await fetch("scanned-page.png").then((r) => r.arrayBuffer()));

	const result = await extractBytes(bytes, "image/png", {
		ocr: {
			backend: "tesseract-wasm",
			language: "eng",
		},
	});

	console.log("Extracted text:");
	console.log(result.content);
}

extractWithOcr().catch(console.error);
```



#### Table Extraction


See [Table Extraction Guide](https://kreuzberg.dev/features/table-extraction/) for detailed examples.



#### Processing Multiple Files


```ts
import { extractBytes, initWasm } from "@kreuzberg/wasm";

interface DocumentJob {
	name: string;
	bytes: Uint8Array;
	mimeType: string;
}

async function _processBatch(documents: DocumentJob[], concurrency: number = 3) {
	await initWasm();

	const results: Record<string, string> = {};
	const queue = [...documents];

	const workers = Array(concurrency)
		.fill(null)
		.map(async () => {
			while (queue.length > 0) {
				const doc = queue.shift();
				if (!doc) break;

				try {
					const result = await extractBytes(doc.bytes, doc.mimeType);
					results[doc.name] = result.content;
				} catch (error) {
					console.error(`Failed to process ${doc.name}:`, error);
				}
			}
		});

	await Promise.all(workers);
	return results;
}
```




#### Async Processing

For non-blocking document processing:

```ts
import { extractBytes, getWasmCapabilities, initWasm } from "@kreuzberg/wasm";

async function extractDocuments(files: Uint8Array[], mimeTypes: string[]) {
	const caps = getWasmCapabilities();
	if (!caps.hasWasm) {
		throw new Error("WebAssembly not supported");
	}

	await initWasm();

	const results = await Promise.all(files.map((bytes, index) => extractBytes(bytes, mimeTypes[index])));

	return results.map((r) => ({
		content: r.content,
		pageCount: r.metadata?.pageCount,
	}));
}

const fileBytes = [new Uint8Array([1, 2, 3])];
const mimes = ["application/pdf"];

extractDocuments(fileBytes, mimes)
	.then((results) => console.log(results))
	.catch(console.error);
```





### Next Steps

- **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** - Platform-specific setup
- **[API Documentation](https://kreuzberg.dev/api/)** - Complete API reference
- **[Examples & Guides](https://kreuzberg.dev/guides/)** - Full code examples and usage guides
- **[Configuration Guide](https://kreuzberg.dev/guides/configuration/)** - Advanced configuration options



## Features

### Supported File Formats (91+)

91+ file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

#### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.docm`, `.dotx`, `.dotm`, `.dot`, `.odt` | Full text, tables, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.xltx`, `.xlt`, `.ods` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.pptm`, `.ppsx`, `.potx`, `.potm`, `.pot`, `.ppt` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |
| **Database** | `.dbf` | Table data extraction, field type support |
| **Hangul** | `.hwp`, `.hwpx` | Korean document format, text extraction |

#### Images (OCR-Enabled)

| Category | Formats | Features |
|----------|---------|----------|
| **Raster** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif` | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.jbig2`, `.jb2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | OCR via hayro-jpeg2000 (pure Rust decoder), JBIG2 support, table detection, format-specific metadata |
| **Vector** | `.svg` | DOM parsing, embedded text, graphics metadata |

#### Web & Data

| Category | Formats | Features |
|----------|---------|----------|
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg` | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` | Schema detection, nested structures, validation |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.djot`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, Djot, reStructuredText, Org Mode |

#### Email & Archives

| Category | Formats | Features |
|----------|---------|----------|
| **Email** | `.eml`, `.msg` | Headers, body (HTML/plain), attachments, threading |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | File listing, nested archives, metadata |

#### Academic & Scientific

| Category | Formats | Features |
|----------|---------|----------|
| **Citations** | `.bib`, `.biblatex`, `.ris`, `.nbib`, `.enw`, `.csl` | Structured parsing: RIS (structured), PubMed/MEDLINE, EndNote XML (structured), BibTeX, CSL JSON |
| **Scientific** | `.tex`, `.latex`, `.typst`, `.jats`, `.ipynb`, `.docbook` | LaTeX, Jupyter notebooks, PubMed JATS |
| **Documentation** | `.opml`, `.pod`, `.mdoc`, `.troff` | Technical documentation formats |

#### Code Intelligence (248 Languages)

| Feature | Description |
|---------|-------------|
| **Structure Extraction** | Functions, classes, methods, structs, interfaces, enums |
| **Import/Export Analysis** | Module dependencies, re-exports, wildcard imports |
| **Symbol Extraction** | Variables, constants, type aliases, properties |
| **Docstring Parsing** | Google, NumPy, Sphinx, JSDoc, RustDoc, and 10+ formats |
| **Diagnostics** | Parse errors with line/column positions |
| **Syntax-Aware Chunking** | Split code by semantic boundaries, not arbitrary byte offsets |

Powered by [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — [documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev).

**[Complete Format Reference](https://kreuzberg.dev/reference/formats/)**

### Key Capabilities

- **Text Extraction** - Extract all text content with position and formatting information
- **Metadata Extraction** - Retrieve document properties, creation date, author, etc.
- **Table Extraction** - Parse tables with structure and cell content preservation
- **Image Extraction** - Extract embedded images and render page previews
- **OCR Support** - Integrate multiple OCR backends for scanned documents

- **Async/Await** - Non-blocking document processing with concurrent operations


- **Plugin System** - Extensible post-processing for custom text transformation


- **Batch Processing** - Efficiently process multiple documents in parallel
- **Memory Efficient** - Stream large files without loading entirely into memory
- **Language Detection** - Detect and support multiple languages in documents

- **Code Intelligence** - Extract structure, imports, exports, symbols, and docstrings from [248 programming languages](https://docs.tree-sitter-language-pack.kreuzberg.dev) via tree-sitter

- **Configuration** - Fine-grained control over extraction behavior

### Performance Characteristics

| Format | Speed | Memory | Notes |
|--------|-------|--------|-------|
| **PDF (text)** | 10-100 MB/s | ~50MB per doc | Fastest extraction |
| **Office docs** | 20-200 MB/s | ~100MB per doc | DOCX, XLSX, PPTX |
| **Images (OCR)** | 1-5 MB/s | Variable | Depends on OCR backend |
| **Archives** | 5-50 MB/s | ~200MB per doc | ZIP, TAR, etc. |
| **Web formats** | 50-200 MB/s | Streaming | HTML, XML, JSON |



## OCR Support

Kreuzberg supports multiple OCR backends for extracting text from scanned documents and images:



- **Tesseract-Wasm**


### OCR Configuration Example

```ts
import { enableOcr, extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractWithOcr() {
	await initWasm();

	try {
		await enableOcr();
		console.log("OCR enabled successfully");
	} catch (error) {
		console.error("Failed to enable OCR:", error);
		return;
	}

	const bytes = new Uint8Array(await fetch("scanned-page.png").then((r) => r.arrayBuffer()));

	const result = await extractBytes(bytes, "image/png", {
		ocr: {
			backend: "tesseract-wasm",
			language: "eng",
		},
	});

	console.log("Extracted text:");
	console.log(result.content);
}

extractWithOcr().catch(console.error);
```




## Async Support

This binding provides full async/await support for non-blocking document processing:

```ts
import { extractBytes, getWasmCapabilities, initWasm } from "@kreuzberg/wasm";

async function extractDocuments(files: Uint8Array[], mimeTypes: string[]) {
	const caps = getWasmCapabilities();
	if (!caps.hasWasm) {
		throw new Error("WebAssembly not supported");
	}

	await initWasm();

	const results = await Promise.all(files.map((bytes, index) => extractBytes(bytes, mimeTypes[index])));

	return results.map((r) => ({
		content: r.content,
		pageCount: r.metadata?.pageCount,
	}));
}

const fileBytes = [new Uint8Array([1, 2, 3])];
const mimes = ["application/pdf"];

extractDocuments(fileBytes, mimes)
	.then((results) => console.log(results))
	.catch(console.error);
```




## Plugin System

Kreuzberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://kreuzberg.dev/guides/plugins/).







## Batch Processing

Process multiple documents efficiently:

```ts
import { extractBytes, initWasm } from "@kreuzberg/wasm";

interface DocumentJob {
	name: string;
	bytes: Uint8Array;
	mimeType: string;
}

async function _processBatch(documents: DocumentJob[], concurrency: number = 3) {
	await initWasm();

	const results: Record<string, string> = {};
	const queue = [...documents];

	const workers = Array(concurrency)
		.fill(null)
		.map(async () => {
			while (queue.length > 0) {
				const doc = queue.shift();
				if (!doc) break;

				try {
					const result = await extractBytes(doc.bytes, doc.mimeType);
					results[doc.name] = result.content;
				} catch (error) {
					console.error(`Failed to process ${doc.name}:`, error);
				}
			}
		});

	await Promise.all(workers);
	return results;
}
```



## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://kreuzberg.dev/guides/configuration/)**

## Documentation

- **[Official Documentation](https://kreuzberg.dev/)**
- **[API Reference](https://kreuzberg.dev/reference/api-wasm/)**
- **[Examples & Guides](https://kreuzberg.dev/guides/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Discussions**: [Ask questions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
