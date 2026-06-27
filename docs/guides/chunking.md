# Text Chunking

Split extracted text into overlapping, structure-aware chunks ready to embed and index for RAG. Four strategies support different document types — text splits on whitespace/punctuation, Markdown preserves structure and code blocks, YAML maintains section hierarchy, and semantic chunking uses embeddings to detect topic shifts.

## Strategies

- **Text** — splits on whitespace/punctuation boundaries
- **Markdown** — structure-aware; preserves headings, lists, and code blocks
- **YAML** — section-aware; preserves YAML document structure
- **Semantic** — topic-aware; splits at natural document boundaries

## Semantic Chunking

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

## Configuration

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

## Chunk Output

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

## RAG Pipeline Example

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

## See also

- [Embeddings](embeddings.md) — generate vectors for semantic search
- [Configuration Reference](../reference/configuration.md#chunkingconfig) — all chunking options
