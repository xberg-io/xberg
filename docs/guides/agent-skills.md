# AI Coding Assistants <span class="version-badge new">v4.2.15</span>

Kreuzberg ships with an [Agent Skill](https://agentskills.io) that teaches AI coding assistants how to use the library correctly — covering extraction, configuration, OCR, chunking, embeddings, batch processing, error handling, and plugins across Python, Node.js/TypeScript, Rust, and CLI.

## Supported Assistants

Works with any tool supporting the [Agent Skills](https://agentskills.io) standard: Claude Code, Codex, Gemini CLI, Cursor, VS Code (with AI extensions), Amp, Goose, and Roo Code.

## Installing

```bash title="Terminal"
# Install into current project (recommended)
npx skills add kreuzberg-dev/kreuzberg

# Install globally
npx skills add kreuzberg-dev/kreuzberg -g
```

Or copy manually:

```bash title="Terminal"
cp -r path/to/kreuzberg/skills/kreuzberg .claude/skills/kreuzberg
```

## What the Skill Provides

When your AI coding assistant discovers the skill, it knows:

- All extraction functions and their correct signatures across languages
- Configuration field names (e.g., `max_chars` not `max_characters` in Python)
- Rust feature gates (e.g., `tokio-runtime` for sync functions)
- Language-specific conventions (snake_case in Python/Rust, camelCase in Node.js)
- Error handling patterns for each language

### Skill Structure

```text
skills/kreuzberg/
├── SKILL.md                        # Main skill (~400 lines)
└── references/
    ├── python-api.md               # Complete Python API
    ├── nodejs-api.md               # Complete Node.js API
    ├── rust-api.md                 # Complete Rust API
    ├── cli-reference.md            # All CLI commands and flags
    ├── configuration.md            # Config file formats and schema
    ├── supported-formats.md        # All 91+ supported formats
    ├── advanced-features.md        # Plugins, embeddings, MCP, security
    └── other-bindings.md           # Go, Ruby, Java, C#, PHP, Elixir
```

The main file stays under 500 lines for efficient AI consumption. Reference files load on demand.

## Quick Examples

=== "Python"

    ```python
    from kreuzberg import extract_file, extract_file_sync, ExtractionConfig, OcrConfig

    result = await extract_file("document.pdf")
    print(result.content)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        output_format="markdown",
    )
    result = await extract_file("document.pdf", config=config)
    ```

=== "Node.js"

    ```typescript
    import { extractFile, extractFileSync } from '@kreuzberg/node';

    const result = await extractFile('document.pdf');
    console.log(result.content);
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file, ExtractionConfig};

    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;
    ```

=== "CLI"

    ```bash
    kreuzberg extract document.pdf
    kreuzberg extract document.pdf --format json --output-format markdown
    ```

## Further Reading

- [Agent Skills Standard](https://agentskills.io) — the open standard
- [Extraction Basics](extraction.md) — core extraction API
- [Configuration](configuration.md) — all configuration options
- [Advanced Features](advanced.md) — chunking, embeddings, language detection
- [Plugin System](plugins.md) — custom plugins
