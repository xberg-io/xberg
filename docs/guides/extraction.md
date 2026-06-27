# Extraction Basics

Two extraction functions are the public entry points:

| Function        | Input model      | Purpose                                   |
| --------------- | ---------------- | ----------------------------------------- |
| `extract`       | `ExtractInput`   | Extract one URI or in-memory byte payload |
| `extract_batch` | `ExtractInput[]` | Extract multiple URI and byte inputs      |

`ExtractInput` uses `kind = "uri"` for local paths, `file://` URIs, and HTTP(S)
URLs. Use `kind = "bytes"` for in-memory payloads. `extract` and
`extract_batch` return an `ExtractionOutput` envelope with `results`, `errors`,
`summary`, and optional crawl metadata.

## Extract One Input

=== "Python"

    ```python title="extract_one.py"
    from xberg import ExtractInput, extract

    output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
    print(output.results[0].content)
    ```

=== "TypeScript"

    ```typescript title="extract-one.ts"
    import { ExtractInputKind, extract } from "@xberg-io/xberg";

    const output = await extract({
      kind: ExtractInputKind.Uri,
      uri: "document.pdf",
    });
    console.log(output.results[0].content);
    ```

=== "Rust"

    ```rust title="extract_one.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig};

    let config = ExtractionConfig::default();
    let output = extract(ExtractInput::uri("document.pdf"), &config).await?;
    println!("{}", output.results[0].content);
    ```

## Extract from Bytes

When content is already loaded in memory, pass bytes through `ExtractInput`
with an explicit MIME type.

=== "Python"

    ```python title="extract_from_bytes.py"
    from xberg import ExtractInput, extract

    with open("document.pdf", "rb") as file:
        data = file.read()

    output = await extract(
        ExtractInput(
            kind="bytes",
            bytes=data,
            mime_type="application/pdf",
            filename="document.pdf",
        )
    )
    ```

=== "TypeScript"

    ```typescript title="extract-bytes.ts"
    import { readFile } from "node:fs/promises";
    import { ExtractInputKind, extract } from "@xberg-io/xberg";

    const data = await readFile("document.pdf");
    const output = await extract({
      kind: ExtractInputKind.Bytes,
      bytes: data,
      mimeType: "application/pdf",
      filename: "document.pdf",
    });
    ```

=== "Rust"

    ```rust title="extract_from_bytes.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig};

    let data = std::fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let output = extract(
        ExtractInput::bytes(data, "application/pdf", Some("document.pdf".to_string())),
        &config,
    )
    .await?;
    ```

## Batch Processing

`extract_batch` accepts a list of `ExtractInput` values. Mix URI and byte inputs
in one request when a pipeline receives documents from multiple sources.

=== "Python"

    ```python title="extract_batch.py"
    from xberg import ExtractInput, extract_batch

    inputs = [
        ExtractInput(kind="uri", uri="report.pdf"),
        ExtractInput(kind="uri", uri="scan.tiff", mime_type="image/tiff"),
    ]

    output = await extract_batch(inputs)
    for result in output.results:
        print(result.content[:200])
    ```

=== "TypeScript"

    ```typescript title="extract-batch.ts"
    import { ExtractInputKind, extractBatch } from "@xberg-io/xberg";

    const output = await extractBatch([
      { kind: ExtractInputKind.Uri, uri: "report.pdf" },
      { kind: ExtractInputKind.Uri, uri: "scan.tiff", mimeType: "image/tiff" },
    ]);
    for (const result of output.results) {
      console.log(result.content.slice(0, 200));
    }
    ```

=== "Rust"

    ```rust title="extract_batch.rs"
    use xberg::{extract_batch, ExtractInput, ExtractionConfig};

    let config = ExtractionConfig::default();
    let inputs = vec![
        ExtractInput::uri("report.pdf"),
        ExtractInput {
            uri: Some("scan.tiff".to_string()),
            mime_type: Some("image/tiff".to_string()),
            ..Default::default()
        },
    ];

    let output = extract_batch(inputs, &config).await?;
    ```

### Per-Input Configuration

When a batch contains a mix of document types that need different settings,
attach per-input overrides to `ExtractInput` while sharing a common batch config.

=== "Python"

    ```python title="mixed_batch.py"
    from xberg import (
        ExtractionConfig,
        ExtractInput,
        FileExtractionConfig,
        extract_batch,
    )

    config = ExtractionConfig(output_format="markdown")

    inputs = [
        ExtractInput(kind="uri", uri="report.pdf"),
        ExtractInput(
            kind="uri",
            uri="scan.tiff",
            config=FileExtractionConfig(force_ocr=True),
        ),
        ExtractInput(
            kind="uri",
            uri="notes.html",
            config=FileExtractionConfig(output_format="plain"),
        ),
    ]

    output = await extract_batch(inputs, config)
    ```

=== "TypeScript"

    ```typescript title="mixed_batch.ts"
    import { ExtractInputKind, extractBatch } from "@xberg-io/xberg";

    const output = await extractBatch(
      [
        { kind: ExtractInputKind.Uri, uri: "report.pdf" },
        {
          kind: ExtractInputKind.Uri,
          uri: "scan.tiff",
          config: { forceOcr: true },
        },
        {
          kind: ExtractInputKind.Uri,
          uri: "notes.html",
          config: { outputFormat: "plain" },
        },
      ],
      { outputFormat: "markdown" },
    );
    ```

=== "Rust"

    ```rust title="mixed_batch.rs"
    use xberg::{
        extract_batch, ExtractInput, ExtractInputKind, ExtractionConfig,
        FileExtractionConfig, OutputFormat,
    };

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let inputs = vec![
        ExtractInput::uri("report.pdf"),
        ExtractInput {
            kind: ExtractInputKind::Uri,
            uri: Some("scan.tiff".to_string()),
            config: Some(FileExtractionConfig {
                force_ocr: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        ExtractInput {
            kind: ExtractInputKind::Uri,
            uri: Some("notes.html".to_string()),
            config: Some(FileExtractionConfig {
                output_format: Some(OutputFormat::Plain),
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    let output = extract_batch(inputs, &config).await?;
    ```

Fields set to `None` in `FileExtractionConfig` inherit the batch default.
Batch-level concerns like `max_concurrent_extractions`, `use_cache`, and
`security_limits` cannot be overridden per input. See the
[Configuration Reference](../reference/configuration.md#fileextractionconfig)
for the full list of overridable fields.

## Content Filtering

Xberg strips running headers, footers, watermarks, and cross-page repeating text
by default so downstream RAG and LLM pipelines see clean body content.
`ContentFilterConfig` lets you opt back in when those regions carry useful text.

By default headers, footers, and watermarks are stripped and cross-page repeating
text is deduplicated; see
[ContentFilterConfig](../reference/configuration.md#contentfilterconfig) for
field-level defaults and per-format behavior.

=== "Python"

    ```python title="keep_headers_footers.py"
    from xberg import (
        ContentFilterConfig,
        ExtractionConfig,
        ExtractInput,
        extract,
    )

    config = ExtractionConfig(
        content_filter=ContentFilterConfig(
            include_headers=True,
            include_footers=True,
        ),
    )

    output = await extract(
        ExtractInput(kind="uri", uri="contract.pdf"),
        config=config,
    )
    ```

=== "TypeScript"

    ```typescript title="disable_repeating_text.ts"
    import { ExtractInputKind, extract } from "@xberg-io/xberg";

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: "brochure.pdf" },
      {
        contentFilter: {
          stripRepeatingText: false,
        },
      },
    );
    ```

=== "Rust"

    ```rust title="content_filter.rs"
    use xberg::{extract, ContentFilterConfig, ExtractInput, ExtractionConfig};

    let config = ExtractionConfig {
        content_filter: Some(ContentFilterConfig {
            include_headers: true,
            include_footers: true,
            strip_repeating_text: true,
            include_watermarks: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let output = extract(ExtractInput::uri("contract.pdf"), &config).await?;
    ```

When a layout-detection model is active, it can independently classify regions
as page headers or footers and strip them per page. Setting
`include_headers=True` / `include_footers=True` also disables that per-page
stripping. See the
[reference page](../reference/configuration.md#contentfilterconfig) for the full
field semantics and per-format behavior.

## Supported Formats

Xberg supports 96 file formats across 8 categories:

| Category          | Extensions                                               | Notes                               |
| ----------------- | -------------------------------------------------------- | ----------------------------------- |
| **PDF**           | `.pdf`                                                   | Native text + OCR for scanned pages |
| **Images**        | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.webp`, `.heic`, `.heif`, `.avif` | OCR backend; HEIC/HEIF/AVIF need `heic` feature + libheif |
| **Office**        | `.docx`, `.pptx`, `.xlsx`                                | Modern formats via native parsers   |
| **Legacy Office** | `.doc`, `.ppt`                                           | Native OLE/CFB parsing              |
| **Email**         | `.eml`, `.msg`                                           | Full support including attachments  |
| **Web**           | `.html`, `.htm`                                          | Converted to Markdown with metadata |
| **Text**          | `.md`, `.txt`, `.xml`, `.json`, `.yaml`, `.toml`, `.csv` | Direct extraction                   |
| **Archives**      | `.zip`, `.tar`, `.tar.gz`, `.tar.bz2`                    | Recursive extraction                |

### Image metadata and EXIF

For every supported image format — JPEG, PNG, TIFF, WebP, BMP, GIF, JPEG 2000,
HEIC, HEIF, AVIF — Xberg returns an `ImageMetadata` block on
`metadata.format` containing:

- **`width`** / **`height`** in pixels
- **`format`** — uppercase format tag (e.g. `JPEG`, `PNG`, `HEIF`)
- **`exif`** — a key/value map of EXIF tags

EXIF extraction is powered by the pure-Rust `nom-exif` integration and covers
camera identity (Make, Model, LensModel, LensSpecification, Software),
timestamps (DateTimeOriginal, CreateDate, OffsetTime, SubSecTime), full
exposure parameters (ExposureTime, FNumber, ISO, ApertureValue,
ShutterSpeedValue, ExposureProgram, ExposureMode, MeteringMode, Flash,
SceneCaptureType), the complete GPS block (GPSLatitude, GPSLongitude,
GPSAltitude, GPSTimeStamp, GPSDateStamp, GPSSpeed, GPSImgDirection,
GPSMapDatum, GPSProcessingMethod), color space, thumbnail offsets, and
provenance fields (Copyright, ImageDescription, ImageUniqueID).

EXIF works on every target, including `wasm-target` and `android-target`,
because `nom-exif` is pure Rust. HEIC / HEIF / AVIF pixel decoding requires
the `heic` Cargo feature and the system `libheif` library, and is therefore
**native-only** — see the [installation guide](../getting-started/installation.md#heif--heic--avif-support).

When the `heic` feature is enabled, HEIC / HEIF / AVIF inputs are decoded to
RGBA via `libheif`, re-encoded as PNG, and then flow through the standard
OCR / layout pipeline. EXIF is read from the original HEIC bytes before the
PNG re-encode so no metadata is lost.

## Page Tracking

Xberg can track page boundaries and extract per-page content. Page tracking availability depends on the format:

- **PDF** — Full byte-accurate page tracking with O(1) lookup
- **PPTX** — Slide boundary tracking (each slide = one page)
- **DOCX** — Best-effort detection using explicit `<w:br type="page"/>` tags
- **Other formats** — No page tracking

Enable page extraction with `PageConfig`:

```python title="page_tracking.py"
config = ExtractionConfig(
    pages=PageConfig(
        insert_page_markers=True,
        marker_format="\n\n<!-- PAGE {page_num} -->\n\n"
    )
)
```

Page markers like `<!-- PAGE 1 -->` are inserted at boundaries in the `content` field — useful for LLMs that need to understand document layout. When both page tracking and chunking are enabled, chunks automatically include `first_page` and `last_page` metadata.

See [PageConfig Reference](../reference/configuration.md#pageconfig) for all options and [Advanced Page Tracking](./advanced.md) for chunk-to-page mapping examples.

## Code File Extraction

Source code files (`.py`, `.rs`, `.ts`, `.go`, etc.) go through tree-sitter and produce a `ProcessResult` on `ExtractionResult.code_intelligence` (structure, imports/exports, symbols, docstrings, diagnostics, semantic chunks). Code files bypass text chunking — TSLP's function/class-aware `CodeChunks` map directly to Xberg `Chunk`s with semantic `chunk_type` and heading context.

See [Code Intelligence](code-intelligence.md) for usage and [`TreeSitterProcessConfig`](../reference/configuration.md#treesitterprocessconfig) for fields.

## PDF Page Rendering

Render individual PDF pages as PNG images. Unlike the extraction pipeline (which parses text, tables, metadata), this API produces raw pixel data for thumbnails, vision model input, or custom OCR pipelines.

### Two Approaches

| API               | When to use                                                            |
| ----------------- | ---------------------------------------------------------------------- |
| `render_pdf_page` | You know which page you need, or only need a few pages                 |
| `PdfPageIterator` | Process every page sequentially without loading all images into memory |

### DPI Configuration

| DPI           | Pixel size (US Letter) | Use case                        |
| ------------- | ---------------------- | ------------------------------- |
| 72            | 612 x 792              | Thumbnails, quick previews      |
| 150 (default) | 1275 x 1650            | General-purpose, screen display |
| 300           | 2550 x 3300            | OCR input, print quality        |

**Tip:** Use 300 DPI when rendering pages for OCR or vision models. The default 150 DPI may reduce recognition accuracy on small text.

## MIME Type Detection

When extracting from bytes, `ExtractInput` requires an explicit MIME type since there's no file extension to infer it from. For file paths, auto-detection from the extension is automatic.

### Example: Override MIME Type

```python title="Python"
from xberg import ExtractInput, extract

# File without extension — provide MIME type explicitly
result = await extract(
    ExtractInput(
        kind="uri",
        uri="document_copy",
        mime_type="application/pdf",
    ),
    config=config,
)
```

## Error Handling

All extraction functions raise typed exceptions on failure. Catch specific exceptions to handle different failure modes:

=== "Python"

    --8<-- "snippets/python/utils/error_handling.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling.md"

=== "R"

    --8<-- "snippets/r/api/error_handling.md"

=== "C"

    --8<-- "snippets/c/api/error_handling.md"

=== "Wasm"

    --8<-- "snippets/wasm/api/error_handling_wasm.md"

!!! Warning "System Errors"
`OSError` (Python), `IOException` (Rust), and system-level errors always propagate through. These indicate real system problems (permissions, disk space, etc.) that your application should handle.

## Next Steps

- [Configuration](configuration.md) — all configuration options and file formats
- [OCR Guide](ocr.md) — set up optical character recognition
- [Advanced Features](advanced.md) — chunking, language detection, embeddings
- [Element-Based Output](output-formats.md#element-based-output) — structured element arrays for RAG
- [Document Structure](output-formats.md#document-structure) — hierarchical tree output
