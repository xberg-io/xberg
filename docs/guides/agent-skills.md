# AI Coding Assistants

Kreuzberg ships with an [Agent Skill](https://agentskills.io) that teaches AI coding assistants how to use the library correctly. The skill provides comprehensive API knowledge for Python, Node.js/TypeScript, Rust, and CLI, covering extraction, configuration, OCR, chunking, embeddings, batch processing, error handling, and plugins.

## What Are Agent Skills?

Agent Skills are structured knowledge files that follow the open [Agent Skills](https://agentskills.io) standard. They are automatically discovered by AI coding assistants and provide context about how to use a library, its APIs, and best practices. Unlike traditional documentation, skills are optimized for AI consumption with progressive disclosure: a concise main file for common tasks, with detailed reference files loaded on demand.

## Supported AI Coding Assistants

The Kreuzberg skill works with any tool supporting the Agent Skills standard:

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini CLI** (Google)
- **Cursor**
- **VS Code** (with AI extensions)
- **Amp**
- **Goose**
- **Roo Code**

## What the Skill Covers

The main skill file (`skills/kreuzberg/SKILL.md`) provides quick-start guidance for all four primary interfaces. Detailed reference files are available for deep dives into specific topics.

### Extraction Flows

The skill covers all extraction patterns across languages:

=== "Python"

    ```python
    from kreuzberg import extract_file, extract_file_sync

    # Async extraction
    result = await extract_file("document.pdf")
    print(result.content)       # Extracted text
    print(result.metadata)      # Document metadata
    print(result.tables)        # Structured tables

    # Sync extraction
    result = extract_file_sync("document.pdf")
    ```

=== "Node.js"

    ```typescript
    import { extractFile, extractFileSync } from '@kreuzberg/node';

    // Async extraction
    const result = await extractFile('document.pdf');
    console.log(result.content);
    console.log(result.metadata);

    // Sync extraction
    const result = extractFileSync('document.pdf');
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file, extract_file_sync, ExtractionConfig};

    // Async extraction
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;

    // Sync extraction (requires tokio-runtime feature)
    let result = extract_file_sync("document.pdf", None, &config)?;
    ```

=== "CLI"

    ```bash
    # Text output
    kreuzberg extract document.pdf

    # JSON output
    kreuzberg extract document.pdf --format json

    # Markdown content format
    kreuzberg extract document.pdf --output-format markdown
    ```

### Configuration

The skill covers the full configuration system including OCR, chunking, output format, PDF options, and language detection:

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, OcrConfig, ChunkingConfig

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
        output_format="markdown",
    )
    result = await extract_file("document.pdf", config=config)
    ```

=== "Node.js"

    ```typescript
    const config = {
        ocr: { backend: 'tesseract', language: 'eng' },
        chunking: { maxChars: 1000, maxOverlap: 200 },
        outputFormat: 'markdown',
    };
    const result = await extractFile('document.pdf', null, config);
    ```

=== "TOML"

    ```toml
    output_format = "markdown"

    [ocr]
    backend = "tesseract"
    language = "eng"

    [chunking]
    max_chars = 1000
    max_overlap = 200
    ```

### Chunking and Embeddings

The skill covers text chunking for RAG pipelines and vector embedding generation:

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, ChunkingConfig

    config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
    )
    result = await extract_file("document.pdf", config=config)

    for chunk in result.chunks:
        print(f"Chunk {chunk.metadata.chunk_index}: {chunk.content[:100]}...")
        if chunk.embedding:
            print(f"  Embedding dimensions: {len(chunk.embedding)}")
    ```

=== "Node.js"

    ```typescript
    const config = {
        chunking: { maxChars: 1000, maxOverlap: 200 },
    };
    const result = await extractFile('document.pdf', null, config);

    for (const chunk of result.chunks ?? []) {
        console.log(`Chunk ${chunk.metadata.chunkIndex}: ${chunk.content.slice(0, 100)}...`);
    }
    ```

### Batch Processing

The skill covers batch extraction for processing multiple documents concurrently:

=== "Python"

    ```python
    from kreuzberg import batch_extract_files

    results = await batch_extract_files(["doc1.pdf", "doc2.docx", "doc3.xlsx"])
    for result in results:
        print(f"{len(result.content)} chars extracted")
    ```

=== "Node.js"

    ```typescript
    import { batchExtractFiles } from '@kreuzberg/node';

    const results = await batchExtractFiles(['doc1.pdf', 'doc2.docx']);
    ```

=== "CLI"

    ```bash
    kreuzberg batch *.pdf --format json
    ```

### Error Handling

The skill provides error handling patterns for each language with specific error types for parsing, OCR, validation, and missing dependencies.

### Plugin System

The skill covers the plugin architecture for custom post-processors, validators, and OCR backends.

## Skill File Structure

```
skills/kreuzberg/
├── SKILL.md                        # Main skill (~400 lines)
└── references/
    ├── python-api.md               # Complete Python API
    ├── nodejs-api.md               # Complete Node.js API
    ├── rust-api.md                 # Complete Rust API
    ├── cli-reference.md            # All CLI commands and flags
    ├── configuration.md            # Config file formats and schema
    ├── supported-formats.md        # All 62+ supported formats
    ├── advanced-features.md        # Plugins, embeddings, MCP, security
    └── other-bindings.md           # Go, Ruby, Java, C#, PHP, Elixir
```

The main `SKILL.md` file is kept under 500 lines for efficient AI consumption. Reference files provide deep-dive details that AI tools load on demand when more context is needed.

## How It Works

When you open a project that uses Kreuzberg (or a project with the skill files present), your AI coding assistant automatically discovers `skills/kreuzberg/SKILL.md` and loads it as context. This means the AI:

1. Knows all available extraction functions and their correct signatures
2. Uses the right field names for configuration (e.g., `max_chars` not `max_characters` in Python)
3. Handles Rust feature gates correctly (e.g., `tokio-runtime` for sync functions)
4. Follows language-specific conventions (snake_case in Python/Rust, camelCase in Node.js)
5. Generates correct error handling patterns for each language

## Using Without the Repository

If you're using Kreuzberg in a project that doesn't include the skill files, you can add them:

```bash
# Copy the skill into your project
cp -r path/to/kreuzberg/skills/kreuzberg skills/kreuzberg
```

Or reference the skill directly from the Kreuzberg repository if your AI coding assistant supports remote skills.

## Further Reading

- [Agent Skills Standard](https://agentskills.io) — The open standard for AI coding assistant skills
- [Extraction Basics](extraction.md) — Detailed extraction guide
- [Advanced Features](advanced.md) — Chunking, embeddings, language detection
- [Configuration](configuration.md) — Full configuration reference
- [Plugin System](plugins.md) — Creating custom plugins
- [API Server & MCP](api-server.md) — Server deployment and MCP integration
