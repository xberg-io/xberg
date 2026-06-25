# Named-Entity Recognition

Detect named entities (people, organisations, locations, dates, money amounts, emails, phones, URLs, plus caller-supplied custom labels) in extracted text. Result populates `ExtractionResult.entities`.

!!! Note "Feature gate"
    The result types ship in the `ner` Cargo feature (included in `no-ort-target`, `wasm-target`, `android-target`, and `full`). Choose a backend: `ner-onnx` (`xberg-gliner` ONNX) or `ner-llm` (liter-llm).

## Backends

| Backend | Cargo feature | When to use | Status |
|---|---|---|---|
| `Onnx` (`xberg-gliner`) | `ner-onnx` | High throughput, local inference, deterministic | Available. |
| `Llm` (liter-llm) | `ner-llm` | Domain-specific zero-shot labels, any of 143 providers | Available today. |

The ONNX backend downloads supported Xberg GLiNER aliases and catalog ids from
`xberg-io/gliner-models`. The runtime consumes exported ONNX artifacts and
tokenizer files; it does not load arbitrary source PyTorch model repositories.
If the artifact repository is private or not publicly readable, authenticate
with Hugging Face using credentials supported by `hf-hub` before warming the
cache or running inference.

## When to Use

- You need entity tags attached to extracted text for retrieval, faceting, or compliance review.
- You need PII categories surfaced for downstream redaction (NER pairs with the redaction post-processor — see [Redaction & Anonymisation](redaction.md)).
- You need zero-shot labelling against caller-supplied categories ("Treatment", "Vessel", "Product") that fall outside the GLiNER taxonomy.

## When Not to Use

- You only need regex-detectable PII (emails, phones, IBANs, SSNs). The redaction pattern engine is 1000× cheaper. See [Redaction & Anonymisation](redaction.md).
- You want sub-100ms latency on a hot path with a large LLM. Prefer the ONNX backend (`ner-onnx`) for deterministic local inference.

## Configuration

=== "Python"

    --8<-- "snippets/python/ner/basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ner/basic.md"

=== "Rust"

    --8<-- "snippets/rust/ner/basic.md"

=== "CLI"

    --8<-- "snippets/cli/ner_basic.md"

=== "TOML"

    --8<-- "snippets/cli/ner_toml.md"

## Custom Labels (Zero-Shot)

Pass arbitrary labels via `NerConfig.custom_labels`. The LLM backend folds each label into the structured-output schema; the ONNX backend uses GLiNER's native zero-shot inference.

=== "Python"

    --8<-- "snippets/python/ner/custom_labels.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ner/custom_labels.md"

=== "Rust"

    --8<-- "snippets/rust/ner/custom_labels.md"

Custom hits surface as `EntityCategory::Custom(label)` in the resulting `Entity` stream. Casing of the supplied label is preserved.

## Output Shape

`ExtractionResult.entities` is `Option<Vec<Entity>>`, populated when NER ran and produced at least one detection. JSON shape:

```json
{
  "entities": [
    { "category": "person", "text": "Ada Lovelace", "start": 42, "end": 54, "confidence": 0.93 },
    { "category": { "custom": "Treatment" }, "text": "metformin", "start": 120, "end": 129, "confidence": 0.81 }
  ]
}
```

Byte offsets refer to `result.content`. When the redaction post-processor rewrites the document, NER offsets are recomputed against the redacted text — use the audit trail in `result.redaction_report` to reconstruct positions in the original.

## Categories

| `EntityCategory` | Description |
|---|---|
| `Person` | Person names. |
| `Organization` | Organisations, companies, institutions. |
| `Location` | Geographic locations. |
| `Date` | Date mentions. |
| `Time` | Time-of-day mentions. |
| `Money` | Monetary amounts with currency. |
| `Percent` | Percentages. |
| `Email` | Email addresses. |
| `Phone` | Phone numbers. |
| `Url` | URLs. |
| `Custom(label)` | Caller-supplied zero-shot label. |

## LLM Backend Setup

When `backend = "llm"`, configure the model via `NerConfig.llm`. The API-key precedence chain matches [LLM Integration](llm-integration.md#api-key-configuration):

1. `NerConfig.llm.api_key`
2. `XBERG_LLM_API_KEY`
3. Per-provider env var (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, …)

Local engines (Ollama, LM Studio, vLLM) need no key.

## Known Limitations

- The LLM backend's accuracy depends on the chosen model. Use `gpt-4o-mini` or larger for production NER.

## Related

- [Redaction & Anonymisation](redaction.md) — uses NER for PERSON / ORGANIZATION / LOCATION categories
- [LLM Integration](llm-integration.md) — full LLM provider matrix, local engine setup, API-key precedence
- [Configuration Reference](../reference/configuration.md#nerconfig) — full field reference
