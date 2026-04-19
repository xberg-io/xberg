# Advanced Features

## Text Chunking

Split extracted text into chunks for RAG systems, vector databases, or LLM context windows. Four strategies: **Text** (splits on whitespace/punctuation boundaries), **Markdown** (structure-aware, preserves headings, lists, code blocks), **Yaml** (section-aware, preserves YAML document structure), and **Semantic** (topic-aware, splits at natural document boundaries).

### Semantic

The semantic chunker produces topic-coherent chunks by splitting at natural document boundaries. It requires either an embedding model for topic detection or uses structural heuristics as fallback.

Set `chunker_type` to `"semantic"`:

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

=== "WASM"

    --8<-- "snippets/wasm/config/chunking_config.md"

### Chunk Output

Each chunk in `result.chunks` contains:

| Field | Description |
|-------|-------------|
| `content` | Chunk text |
| `metadata.byte_start` / `byte_end` | Byte offsets in the original text |
| `metadata.chunk_index` / `total_chunks` | Position in sequence |
| `metadata.token_count` | Token count (when embeddings enabled) |
| `metadata.heading_context` | Active heading hierarchy (Markdown chunker only) |
| `embedding` | Embedding vector (when configured) |

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

Detect languages in extracted text using [`whatlang`](https://crates.io/crates/whatlang). Supports 60+ languages with ISO 639-3 codes.

By default, only the primary language is returned. Set `detect_multiple: true` to detect all languages in a document: the text is chunked into 200-character segments and language frequencies are aggregated, returning all detected languages sorted by prevalence.

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

Generate embeddings for semantic search and RAG using local ONNX models. Requires the `embeddings` feature. Embeddings are generated in-process with no external API calls.

| Preset | Model | Dimensions | Max Tokens | Use Case |
|--------|-------|-----------|------------|----------|
| `fast` | all-MiniLM-L6-v2 (quantized) | 384 | 512 | Quick prototyping, development, resource-constrained |
| `balanced` | BGE-base-en-v1.5 | 768 | 1024 | General-purpose RAG, production deployments, English |
| `quality` | BGE-large-en-v1.5 | 1024 | 2000 | Complex documents, maximum accuracy, sufficient compute |
| `multilingual` | multilingual-e5-base | 768 | 1024 | International documents, mixed-language content |

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

| Level | Reduction | Effect |
|-------|-----------|--------|
| `off` | 0% | Pass-through |
| `moderate` | 15–25% | Stopwords + redundancy removal |
| `aggressive` | 30–50% | Semantic clustering + importance scoring |

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

| Factor | Weight | Detects |
|--------|--------|---------|
| OCR Artifacts | 30% | Scattered chars, repeated punctuation, malformed words |
| Script Content | 20% | JavaScript, CSS, HTML tags |
| Navigation Elements | 10% | Breadcrumbs, pagination, skip links |
| Document Structure | 20% | Sentence/paragraph length, punctuation distribution |
| Metadata Quality | 10% | Presence of title, author, subject |

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
