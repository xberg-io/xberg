# Advanced Features

## Text Chunking

Split extracted text into chunks for RAG, vector databases, or LLM context windows. Four strategies:

- **Text** — splits on whitespace/punctuation boundaries
- **Markdown** — structure-aware; preserves headings, lists, and code blocks
- **YAML** — section-aware; preserves YAML document structure
- **Semantic** — topic-aware; splits at natural document boundaries

### Semantic

Set `chunker_type` to `"semantic"`. Uses an embedding model for topic detection when one is configured; otherwise falls back to structural heuristics.

```python
config = ExtractionConfig(
    chunking=ChunkingConfig(chunker_type="semantic")
)
```

**Behavior:**

- **Without embeddings** — Uses structural heuristics: detects headers (ALL CAPS, numbered sections) and paragraph boundaries
- **With embeddings** — Compares consecutive paragraphs via embeddings to detect topic shifts, merging paragraphs below the `topic_threshold` (default: 0.5)

Use `topic_threshold` to control sensitivity: higher values (0.7–0.9) preserve more fine-grained topics, lower values (0.1–0.3) merge aggressive. Only applies when an embedding model is configured.

### Configuration

=== "Python"

    --8<-- "snippets/python/config/chunking_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/chunking_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/chunking_config.md"

=== "Go"

    --8<-- "snippets/go/config/chunking_config.md"

=== "Java"

    --8<-- "snippets/java/config/chunking_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/chunking_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/chunking_config.md"

=== "R"

    --8<-- "snippets/r/config/chunking_config.md"

=== "Wasm"

    --8<-- "snippets/wasm/config/chunking_config.md"

### Chunk Output

Each chunk in `result.chunks` contains:

| Field                                   | Description                                      |
| --------------------------------------- | ------------------------------------------------ |
| `content`                               | Chunk text                                       |
| `metadata.byte_start` / `byte_end`      | Byte offsets in the original text                |
| `metadata.chunk_index` / `total_chunks` | Position in sequence                             |
| `metadata.token_count`                  | Token count (when embeddings enabled)            |
| `metadata.heading_context`              | Active heading hierarchy (Markdown chunker only) |
| `metadata.heading_path` | Flattened RAG-shaped heading breadcrumb (e.g., `["Title", "Section", "Subsection"]`) for vector database retrieval and context. |
| `embedding`                             | Embedding vector (when configured)               |

Chunks can be sized by token count instead of characters — enable the `chunking-tokenizers` feature and set `sizing` to `token`.

### RAG Pipeline Example

=== "Python"

    --8<-- "snippets/python/utils/chunking_rag.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/chunking_rag.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/chunking_rag.md"

=== "Go"

    --8<-- "snippets/go/advanced/chunking_rag.md"

=== "Java"

    --8<-- "snippets/java/advanced/chunking_rag.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/chunking_rag.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/chunking_rag.md"

=== "R"

    --8<-- "snippets/r/advanced/chunking_rag.md"

## Language Detection

Detect languages in extracted text using [`whatlang`](https://crates.io/crates/whatlang) — 60+ languages with ISO 639-3 codes. Set `detect_multiple: true` to chunk the text into 200-char segments and return all detected languages sorted by prevalence.

### Configuration

=== "Python"

    --8<-- "snippets/python/config/language_detection_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/language_detection_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/language_detection_config.md"

=== "Go"

    --8<-- "snippets/go/config/language_detection_config.md"

=== "Java"

    --8<-- "snippets/java/config/language_detection_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/language_detection_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/language_detection_config.md"

=== "R"

    --8<-- "snippets/r/config/language_detection_config.md"

### Multilingual Example

=== "Python"

    --8<-- "snippets/python/utils/language_detection_multilingual.md"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata/language_detection_multilingual.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/language_detection_multilingual.md"

=== "Go"

    --8<-- "snippets/go/advanced/language_detection_multilingual.md"

=== "Java"

    --8<-- "snippets/java/advanced/language_detection_multilingual.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/language_detection_multilingual.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/language_detection_multilingual.md"

=== "R"

    --8<-- "snippets/r/advanced/language_detection_multilingual.md"

## Embedding Generation

Local in-process embeddings via ONNX for semantic search and RAG — no external API calls. Requires the `embeddings` feature.

| Preset         | Model                        | Dimensions | Max Tokens | Use Case                                                |
| -------------- | ---------------------------- | ---------- | ---------- | ------------------------------------------------------- |
| `fast`         | all-MiniLM-L6-v2 (quantized) | 384        | 512        | Quick prototyping, development, resource-constrained    |
| `balanced`     | BGE-base-en-v1.5             | 768        | 1024       | General-purpose RAG, production deployments, English    |
| `quality`      | BGE-large-en-v1.5            | 1024       | 2000       | Complex documents, maximum accuracy, sufficient compute |
| `multilingual` | multilingual-e5-base         | 768        | 1024       | International documents, mixed-language content         |

### In-Process Embedding Backends (Plugin Variant)

Plug a caller-managed embedder (e.g. `llama-cpp-python`, `sentence-transformers`) into Xberg via the `Plugin` variant of `EmbeddingModelType` — Xberg calls back into the registered backend instead of running its own ONNX model.

1. Register the backend once at startup via `xberg::plugins::register_embedding_backend(Arc::new(MyEmbedder))`. The backend implements `EmbeddingBackend` (a `Plugin`-inheriting async trait with `dimensions()` and `embed(texts) -> Vec<Vec<f32>>`).
2. Reference it by name in `EmbeddingConfig`: `{ "model": { "type": "plugin", "name": "my-embedder" } }`.
3. Optional: set `EmbeddingConfig.max_embed_duration_secs` (default 60) to bound the wait on a hung backend; `None` disables the timeout.

The CLI (`xberg embed --provider plugin --plugin my-embedder`), MCP server (`embed_text` tool, `embedding_plugin` parameter), REST API, and env var `XBERG_EMBEDDING_PLUGIN_NAME` all accept the Plugin variant once a backend is registered.

**Fork-safety**: Python callers running under `multiprocessing`, `gunicorn`'s prefork worker, or Celery prefork must re-register the backend in each child process — native-backed embedders (including `llama-cpp-python`) aren't fork-safe. Use `os.register_at_fork(after_in_child=reregister_fn)` to automate the re-registration.

### Configuration

=== "Python"

    --8<-- "snippets/python/utils/embedding_with_chunking.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/embedding_with_chunking.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/embedding_with_chunking.md"

=== "Go"

    --8<-- "snippets/go/advanced/embedding_with_chunking.md"

=== "Java"

    --8<-- "snippets/java/advanced/embedding_with_chunking.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/embedding_with_chunking.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/embedding_with_chunking.md"

=== "R"

    --8<-- "snippets/r/advanced/embedding_with_chunking.md"

### Vector Database Integration

=== "Python"

    --8<-- "snippets/python/utils/vector_database_integration.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/vector_database_integration.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/vector_database_integration.md"

=== "Go"

    --8<-- "snippets/go/advanced/vector_database_integration.md"

=== "Java"

    --8<-- "snippets/java/advanced/vector_database_integration.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/vector_database_integration.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/vector_database_integration.md"

=== "R"

    --8<-- "snippets/r/advanced/vector_database_integration.md"

## Token Reduction

Reduce token count while preserving meaning for LLM pipelines.

| Level        | Reduction | Effect                                   |
| ------------ | --------- | ---------------------------------------- |
| `off`        | 0%        | Pass-through                             |
| `moderate`   | 15–25%    | Stopwords + redundancy removal           |
| `aggressive` | 30–50%    | Semantic clustering + importance scoring |

### Configuration

=== "Python"

    --8<-- "snippets/python/config/token_reduction_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/token_reduction_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/token_reduction_config.md"

=== "Go"

    --8<-- "snippets/go/config/token_reduction_config.md"

=== "Java"

    --8<-- "snippets/java/config/token_reduction_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/token_reduction_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/token_reduction_config.md"

=== "R"

    --8<-- "snippets/r/config/token_reduction_config.md"

### Example

=== "Python"

    --8<-- "snippets/python/utils/token_reduction_example.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/token_reduction_example.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/token_reduction_example.md"

=== "Go"

    --8<-- "snippets/go/advanced/token_reduction_example.md"

=== "Java"

    --8<-- "snippets/java/advanced/token_reduction_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/token_reduction_example.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/token_reduction_example.md"

=== "R"

    --8<-- "snippets/r/advanced/token_reduction_example.md"

## Keyword Extraction

Extract keywords using YAKE or RAKE algorithms. Requires the `keywords` feature flag. See [Keyword Extraction](keywords.md) for algorithm details and parameter reference.

### Configuration

=== "Python"

    --8<-- "snippets/python/config/keyword_extraction_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/keyword_extraction_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/keyword_extraction_config.md"

=== "Go"

    --8<-- "snippets/go/config/keyword_extraction_config.md"

=== "Java"

    --8<-- "snippets/java/config/keyword_extraction_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/keyword_extraction_config.md"

=== "R"

    --8<-- "snippets/r/config/keyword_extraction_config.md"

### Example

=== "Python"

    --8<-- "snippets/python/utils/keyword_extraction_example.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/keyword_extraction_example.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/keyword_extraction_example.md"

=== "Go"

    --8<-- "snippets/go/advanced/keyword_extraction_example.md"

=== "Java"

    --8<-- "snippets/java/advanced/keyword_extraction_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_example.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/keyword_extraction_example.md"

=== "R"

    --8<-- "snippets/r/advanced/keyword_extraction_example.md"

## Quality Processing

Score extracted text for quality issues (0.0–1.0, where 1.0 is highest quality). Detects OCR artifacts, script content, navigation elements, and structural issues.

| Factor              | Weight | Detects                                                |
| ------------------- | ------ | ------------------------------------------------------ |
| OCR Artifacts       | 30%    | Scattered chars, repeated punctuation, malformed words |
| Script Content      | 20%    | JavaScript, CSS, HTML tags                             |
| Navigation Elements | 10%    | Breadcrumbs, pagination, skip links                    |
| Document Structure  | 20%    | Sentence/paragraph length, punctuation distribution    |
| Metadata Quality    | 10%    | Presence of title, author, subject                     |

Score ranges: `0.0–0.3` very low, `0.3–0.6` low, `0.6–0.8` moderate, `0.8–1.0` high.

### Configuration

=== "Python"

    --8<-- "snippets/python/config/quality_processing_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/quality_processing_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/quality_processing_config.md"

=== "Go"

    --8<-- "snippets/go/config/quality_processing_config.md"

=== "Java"

    --8<-- "snippets/java/config/quality_processing_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/quality_processing_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/quality_processing_config.md"

=== "R"

    --8<-- "snippets/r/config/quality_processing_config.md"

### Example

=== "Python"

    --8<-- "snippets/python/utils/quality_processing_example.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/quality_processing_example.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/quality_processing_example.md"

=== "Go"

    --8<-- "snippets/go/advanced/quality_processing_example.md"

=== "Java"

    --8<-- "snippets/java/advanced/quality_processing_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/quality_processing_example.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/quality_processing_example.md"

=== "R"

    --8<-- "snippets/r/advanced/quality_processing_example.md"

## PDF Form Fields

Extract form fields (text inputs, checkboxes, radio buttons, dropdowns, signature fields) from fillable PDFs via AcroForm metadata. Enabled by default via `PdfConfig.extract_form_fields`.

### Overview

Fillable PDFs store form structure in two ways:

- **AcroForm** — Static form specification with field metadata. Fully supported.
- **XFA** — XML-based dynamic forms. Currently returns empty results (use AcroForm as workaround).

Extracted fields appear in `result.form_fields` as a list of `PdfFormField` structs, each carrying:

| Property | Type | Description |
|----------|------|-------------|
| `name` | string | Leaf field name in the hierarchy (e.g., `"line_total"`). |
| `full_name` | string | Dotted path from root (e.g., `"invoice.line_items[0].line_total"`). |
| `field_type` | enum | One of: `Text`, `Checkbox`, `Radio`, `Choice`, `Signature`, `Button`, `Unknown`. |
| `value` | string | Current field value (if filled). |
| `default_value` | string | Default value from the form template. |
| `flags` | u32 | Bitmask: read-only, required, multiline, password, and so on. |
| `page` | u32 | 1-indexed page number the field appears on. |
| `bbox` | `BoundingBox` | Widget location on the page (x, y, width, height). |
| `max_length` | u32 | Maximum input length (text fields only). |
| `tooltip` | string | Hover text or field description. |

### Configuration

Enable form field extraction (default):

```toml title="xberg.toml"
[pdf]
extract_form_fields = true
```

Disable (to skip form processing):

```toml
[pdf]
extract_form_fields = false
```

### Processing Form Fields

=== "Python"

    ```python
    from xberg import extract, ExtractionConfig, PdfConfig

    config = ExtractionConfig(
        pdf=PdfConfig(extract_form_fields=True)
    )
    result = extract("form.pdf", config=config)

    for field in result.form_fields:
        print(f"Field: {field.full_name} = {field.value or '(empty)'}")
        print(f"  Type: {field.field_type}, Page: {field.page}")
    ```

=== "TypeScript"

    ```typescript
    import { extractFile, ExtractionConfig } from "xberg";

    const config: ExtractionConfig = {
      pdf: { extract_form_fields: true }
    };
    const result = await extractFile("form.pdf", config);

    for (const field of result.form_fields) {
      console.log(`Field: ${field.full_name} = ${field.value || "(empty)"}`);
      console.log(`  Type: ${field.field_type}, Page: ${field.page}`);
    }
    ```

=== "Rust"

    ```rust
    use xberg::{extract, ExtractionConfig, PdfConfig};

    let config = ExtractionConfig {
        pdf: Some(PdfConfig {
            extract_form_fields: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("form.pdf", None, &config).await?;
    for field in &result.form_fields {
        println!("Field: {} = {}", field.full_name, field.value.as_deref().unwrap_or("(empty)"));
        println!("  Type: {:?}, Page: {:?}", field.field_type, field.page);
    }
    ```

=== "Go"

    ```go
    package main

    import (
        "fmt"
        xberg "github.com/xberg-io/xberg/packages/go"
    )

    func main() {
        config := xberg.NewExtractionConfig()
        config.Pdf.ExtractFormFields = true

        result, err := xberg.ExtractFile("form.pdf", config)
        if err != nil {
            panic(err)
        }

        for _, field := range result.FormFields {
            value := ""
            if field.Value != nil {
                value = *field.Value
            }
            fmt.Printf("Field: %s = %s\n", field.FullName, value)
            fmt.Printf("  Type: %v, Page: %v\n", field.FieldType, field.Page)
        }
    }
    ```

### Use Cases

**Form Auto-Fill**

Extract field values to populate templates or CRMs:

```python
result = extract("invoice_form.pdf")
form_data = {f.full_name: f.value for f in result.form_fields if f.value}
# Submit form_data to downstream system
```

**Form Validation**

Check required fields and validate data before processing:

```python
required_fields = {f for f in result.form_fields if f.flags & 0x01}  # Check required bit
unfilled = {f.full_name for f in required_fields if not f.value}
if unfilled:
    print(f"Missing required fields: {unfilled}")
```

**Form-to-Data Conversion**

Convert fillable forms to structured JSON:

```python
form_json = {
    f.full_name: {
        "value": f.value,
        "type": f.field_type,
        "page": f.page,
        "bbox": f.bbox
    }
    for f in result.form_fields
}
```

### Limitations

- **XFA forms** — Dynamic XML-based forms are not yet supported; `form_fields` will be empty. Use AcroForm-based templates instead.
- **Flattened PDFs** — If form content is rendered into the PDF content stream (vs. stored as field metadata), the form structure is lost. Only editable/unfilled forms preserve field metadata.
- **Appearance streams** — Custom visual styling (button backgrounds, text colors) is not extracted; field values and types only.

### Best Practices

1. **Check field types** — Use `field_type` to handle different input types (text vs. checkbox vs. dropdown).
2. **Validate input** — Check `max_length` and format requirements before use.
3. **Preserve layout** — Use `bbox` and `page` to reconstruct form layout programmatically.
4. **Default values** — When a field has no `value`, consider using `default_value` as fallback.
5. **Test on real forms** — Form structures vary; test your extraction logic on representative PDFs from your sources.

## Combining Features

=== "Python"

    --8<-- "snippets/python/advanced/combining_all_features.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/combining_all_features.md"

=== "Rust"

    --8<-- "snippets/rust/api/combining_all_features.md"

=== "Go"

    --8<-- "snippets/go/api/combining_all_features.md"

=== "Java"

    --8<-- "snippets/java/api/combining_all_features.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/combining_all_features.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/combining_all_features.md"

=== "R"

    --8<-- "snippets/r/api/combining_all_features.md"
