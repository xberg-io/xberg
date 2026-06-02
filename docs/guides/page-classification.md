# Page Classification <span class="version-badge">v5.0.0-rc.3</span>

Classify each page of a document against a caller-supplied label set. Single-label (exactly one) or multi-label (any subset). Result populates `ExtractionResult.page_classifications`.

!!! Note "Feature gate"
    Requires the `classification` Cargo feature. Included in `full`. Requires `liter-llm` for the underlying provider.

## When to Use

- Routing: assign each page to a downstream queue ("invoice", "contract", "id_document", "receipt").
- Filtering: drop or down-rank pages that match a "irrelevant" or "boilerplate" label.
- Document triage: bucket multi-page PDFs into per-page categories without writing a custom classifier.

## When Not to Use

- You need whole-document classification, not per-page. Use [Structured Extraction](llm-integration.md#structured-extraction) with a single string field.
- You have a custom-trained classifier already. Wrap it as a [post-processor plugin](plugins.md) instead.

## Configuration

=== "Python"

    --8<-- "snippets/python/classification/basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/classification/basic.md"

=== "Rust"

    --8<-- "snippets/rust/classification/basic.md"

=== "TOML"

    --8<-- "snippets/cli/classification_toml.md"

## Single-Label vs Multi-Label

`multi_label = false` (default) forces the model to return exactly one label per page. `multi_label = true` lets the model return any subset. Pick the latter when pages can legitimately match more than one category ("invoice" + "purchase_order" on the same page).

=== "Python"

    --8<-- "snippets/python/classification/multi_label.md"

## Custom Prompt (Minijinja)

Override the default classification prompt with a Minijinja template:

```python title="Python"
from kreuzberg import ExtractionConfig, PageClassificationConfig, LlmConfig

config = ExtractionConfig(
    page_classification=PageClassificationConfig(
        labels=["invoice", "contract", "id_document", "receipt"],
        prompt_template=(
            "You are a document triage assistant.\n"
            "Classify the page below using these labels: {{ labels }}.\n"
            "Multi-label: {{ multi_label }}.\n\n"
            "Page text:\n{{ page_text }}"
        ),
        llm=LlmConfig(model="openai/gpt-4o-mini"),
    ),
)
```

| Variable | Description |
|---|---|
| `{{ labels }}` | The configured label list. |
| `{{ page_text }}` | The page's extracted text. |
| `{{ multi_label }}` | Boolean — `true` when multi-label. |

The output is JSON-schema-enforced: the response must be a JSON array of strings drawn from the configured `labels`.

## Output Shape

`ExtractionResult.page_classifications` is `Option<Vec<PageClassification>>`. JSON shape:

```json
{
  "page_classifications": [
    { "page_number": 1, "labels": [{ "label": "invoice", "confidence": 0.94 }] },
    { "page_number": 2, "labels": [{ "label": "purchase_order", "confidence": 0.88 }, { "label": "invoice", "confidence": 0.71 }] }
  ]
}
```

`labels` always carries at least one entry in single-label mode. In multi-label mode it may be empty if the model declines to pick anything.

## Provider Setup

Pick any liter-llm provider. The provider matrix from [LLM Integration](llm-integration.md#supported-providers) applies here. For high-volume classification, `gpt-4o-mini`, `claude-3-5-haiku`, and `google/gemini-2.0-flash` give good cost/accuracy trade-offs.

API-key precedence chain:

1. `PageClassificationConfig.llm.api_key`
2. `KREUZBERG_LLM_API_KEY`
3. Per-provider env var (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, …)

## Related

- [LLM Integration](llm-integration.md) — provider matrix, local engines, API-key precedence
- [Structured Extraction](llm-integration.md#structured-extraction) — full-schema LLM extraction
- [Configuration Reference](../reference/configuration.md#pageclassificationconfig)
