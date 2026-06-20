# AI Coding Assistants

The Kreuzberg plugin teaches AI coding assistants how to use the library — covering extraction, configuration, OCR, chunking, embeddings, batch processing, error handling, and plugins across Python, Node.js/TypeScript, Rust, and CLI.

## Installing

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

Settings → Plugins → Add from URL → `https://github.com/kreuzberg-dev/plugins`, then select **Kreuzberg**.
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

## What the Skill Provides

When your AI coding assistant discovers the skill, it knows:

- All extraction functions and their correct signatures across languages
- Configuration field names (for example, `max_chars` not `max_characters` in Python)
- Rust feature gates (for example, `tokio-runtime` for sync functions)
- Language-specific conventions (snake_case in Python/Rust, camelCase in Node.js)
- Error handling patterns for each language

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

- [Plugin marketplace](https://github.com/kreuzberg-dev/plugins) — install the plugin in every supported harness
- [Extraction Basics](extraction.md) — core extraction API
- [Configuration](configuration.md) — all configuration options
- [Advanced Features](advanced.md) — chunking, embeddings, language detection
- [Plugin System](plugins.md) — custom plugins
