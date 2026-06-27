# Document Translation

Translate extracted documents into any language for normalized downstream processing. Extracted text, formatted markup, and chunks all translate together, keeping your search and retrieval indices in a single language.

!!! Note "Feature gate"
    Requires the `translation` Cargo feature. Included in `full`. Requires `liter-llm` for the underlying provider.

## When to Use

- You ingest documents in mixed languages and want a single normalised language for downstream search or analytics.
- You need per-chunk translation aligned with retrieval-augmented generation (RAG) indexes.
- You need Markdown/HTML preserved through translation (`preserve_markup = true`).

## When Not to Use

- You only need machine-translation of short user queries. Call the LLM provider directly.
- You need a deterministic, network-free pipeline. Translation always calls an LLM.

## Configuration

=== "Python"

    --8<-- "snippets/python/translation/basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/translation/basic.md"

=== "Rust"

    --8<-- "snippets/rust/translation/basic.md"

=== "TOML"

    --8<-- "snippets/cli/translation_toml.md"

## Preserve Markup

Set `preserve_markup = true` to translate `formatted_content` (Markdown / HTML) without losing formatting. The LLM is prompted to keep code fences, links, lists, and tables intact.

=== "Python"

    --8<-- "snippets/python/translation/preserve_markup.md"

## Language Codes

`target_lang` is a BCP-47 tag. Common values:

| Tag | Language |
|---|---|
| `en` | English |
| `de` | German |
| `fr` | French |
| `fr-CA` | French (Canada) |
| `es` | Spanish |
| `zh` | Chinese |
| `ja` | Japanese |
| `ar` | Arabic |
| `pt-BR` | Portuguese (Brazil) |

`source_lang` follows the same format; leave `None` for auto-detection.

## Output Shape

```json
{
  "translation": {
    "target_lang": "de",
    "source_lang": "en",
    "content": "Der Vertrag legt eine dreijährige Supportvereinbarung mit vierteljährlicher Abrechnung fest.",
    "formatted_content": "# Vertrag\n\nDie Laufzeit beträgt drei Jahre…"
  }
}
```

Chunks (when chunking is enabled) carry the translated text in place — `result.chunks[i].content` holds the translated chunk, not the source.

## Provider Setup

Pick any liter-llm provider — see [LLM Integration](llm-integration.md#supported-providers). For high-quality translation, `gpt-4o`, `claude-3-5-sonnet`, and `google/gemini-2.5-pro` are typical picks; `gpt-4o-mini` works for short documents.

API-key precedence:

1. `TranslationConfig.llm.api_key`
2. `XBERG_LLM_API_KEY`
3. Per-provider env var

## Related

- [LLM Integration](llm-integration.md) — provider matrix
- [Document Summarisation](summarization.md) — sibling LLM post-processor
- [Configuration Reference](../reference/configuration.md#translationconfig)
