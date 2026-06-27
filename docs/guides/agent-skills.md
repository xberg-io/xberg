# AI Coding Assistants

The Xberg plugin teaches AI coding assistants how to use the library — covering extraction, configuration, OCR, chunking, embeddings, batch processing, error handling, and plugins across Python, Node.js/TypeScript, Rust, and CLI.

## Installing

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

Settings → Plugins → Add from URL → `https://github.com/xberg-io/plugins`, then select **Xberg**.
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
    from xberg import extract, extract, ExtractionConfig, OcrConfig

    result = await extract("document.pdf")
    print(result.content)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        output_format="markdown",
    )
    result = await extract("document.pdf", config=config)
    ```

=== "Node.js"

    ```typescript
    import { extractFile, extractFileSync } from '@xberg-io/xberg';

    const result = await extractFile('document.pdf');
    console.log(result.content);
    ```

=== "Rust"

    ```rust
    use xberg::{extract, ExtractionConfig};

    let config = ExtractionConfig::default();
    let result = extract("document.pdf", None, &config).await?;
    ```

=== "CLI"

    ```bash
    xberg extract document.pdf
    xberg extract document.pdf --format json --output-format markdown
    ```

## Further Reading

- [Plugin marketplace](https://github.com/xberg-io/plugins) — install the plugin in every supported harness
- [Extraction Basics](extraction.md) — core extraction API
- [Configuration](configuration.md) — all configuration options
- [Advanced Features](advanced.md) — chunking, embeddings, language detection
- [Plugin System](plugins.md) — custom plugins
