# Document Summarisation <span class="version-badge">v5.0.0-rc.3</span>

Produce a prose summary of an extracted document. Extractive backend uses pure-Rust TextRank; abstractive backend uses liter-llm. Result populates `ExtractionResult.summary`.

!!! Note "Feature gate"
    The `summarization` feature ships the TextRank backend (no LLM required) — included in `no-ort-target`, `wasm-target`, `android-target`, `full`. Enable `summarization-llm` for the abstractive backend.

## Backends

| Strategy | Cargo feature | Network | Quality | Latency |
|---|---|---|---|---|
| `Extractive` (default) | `summarization` | None — fully local | Sentence-level selection from source | < 100 ms typical |
| `Abstractive` | `summarization-llm` | LLM provider | Generates novel prose, can summarise across sentences | Provider-dependent |

## When to Use

- You need a one-paragraph TL;DR for indexing or search snippets.
- You need a deterministic, network-free summary (extractive only).
- You need a fluent abstractive summary for downstream LLM consumption.

## When Not to Use

- You need full per-section summaries. Chunk the document first and summarise each chunk separately.
- You need cross-document summarisation. Summarise per document, then summarise the summaries with the LLM backend.

## Configuration

=== "Python"

    --8<-- "snippets/python/summarization/extractive.md"

=== "TypeScript"

    --8<-- "snippets/typescript/summarization/extractive.md"

=== "Rust"

    --8<-- "snippets/rust/summarization/extractive.md"

=== "TOML"

    --8<-- "snippets/cli/summarization_toml.md"

## Abstractive Backend

Switch the strategy and attach an `LlmConfig`:

=== "Python"

    --8<-- "snippets/python/summarization/abstractive.md"

The model receives the extracted content and returns the summary verbatim. Token usage records in `ExtractionResult.llm_usage` with `source = "summarization"`.

## `max_tokens` Semantics

| Strategy | What `max_tokens` caps |
|---|---|
| `Extractive` | Loose whitespace tokens in the output summary. The TextRank selector stops appending sentences once it would exceed the cap. |
| `Abstractive` | The LLM provider's `max_tokens` request parameter. Counted in provider tokens. |

Leave `None` to let the backend pick a sensible default.

## Output Shape

```json
{
  "summary": {
    "text": "The contract sets out a 3-year support agreement with quarterly billing and a fixed escalation cap of 4%.",
    "strategy": "extractive",
    "token_count": 19
  }
}
```

## Provider Setup (Abstractive Only)

Pick any liter-llm provider — see [LLM Integration](llm-integration.md#supported-providers). For most documents, `gpt-4o-mini`, `claude-3-5-haiku`, or `google/gemini-2.0-flash` give good cost / quality trade-offs.

API-key precedence:

1. `SummarizationConfig.llm.api_key`
2. `KREUZBERG_LLM_API_KEY`
3. Per-provider env var

## Related

- [LLM Integration](llm-integration.md) — provider matrix, API-key precedence
- [Document Translation](translation.md) — sibling LLM post-processor
- [Configuration Reference](../reference/configuration.md#summarizationconfig)
