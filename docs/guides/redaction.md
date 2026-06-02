# Redaction & Anonymisation <span class="version-badge">v5.0.0-rc.3</span>

Rewrite every textual field of `ExtractionResult` to remove PII before the result leaves Kreuzberg. The pattern engine covers regex-detectable categories (emails, phones, SSNs, credit cards, IBANs, IP addresses, dates of birth, SWIFT/BIC, postal codes); the optional NER backend adds PERSON / ORGANIZATION / LOCATION. The audit trail lands on `ExtractionResult.redaction_report`.

!!! Note "Feature gate"
    Requires the `redaction` Cargo feature (pattern engine only; ships in `no-ort-target`, `wasm-target`, `android-target`, `full`). Enable `redaction-ml` to add the NER backend for name/organisation/location categories.

!!! Warning "Original text never leaves the pipeline"
    Redaction runs as the Late stage. After it runs, the original content is dropped. Only `result.redaction_report` carries byte offsets back into the original — use it to build audit logs, not to recover the source.

## When to Use

- You ship extracted content to a service that should never see PII.
- You need a deterministic, local pattern engine (no network calls) for regex-detectable PII.
- You need tenant-specific tokens (employee IDs, project codenames, internal product names) removed alongside built-in categories.

## When Not to Use

- You need to keep PII in the result for downstream NER or analytics. Run NER first and store the entities; redact in a second pass.
- You need to redact free-form names and your build doesn't include `redaction-ml`. The pattern engine cannot find arbitrary names — it covers only regex-detectable categories.

## Configuration

=== "Python"

    --8<-- "snippets/python/redaction/basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/redaction/basic.md"

=== "Rust"

    --8<-- "snippets/rust/redaction/basic.md"

=== "TOML"

    --8<-- "snippets/cli/redaction_toml.md"

## PII Categories

| `PiiCategory` | Detection | Notes |
|---|---|---|
| `Email` | Pattern | RFC-5322-ish. |
| `Phone` | Pattern | E.164 + national formats. |
| `Ssn` | Pattern | US SSN with 000/666/9xx exclusions. |
| `CreditCard` | Pattern | 13–19 digits + Luhn check. |
| `PostalCode` | Pattern | Multi-locale. |
| `IpAddress` | Pattern | IPv4 + IPv6. |
| `Iban` | Pattern | ISO country code + length + checksum. |
| `SwiftBic` | Pattern | See "Known limitations" — current regex over-matches plain English words. |
| `DateOfBirth` | Pattern | DOB heuristics. |
| `Person` | NER | Requires `RedactionConfig.ner = Some(NerConfig)`. |
| `Organization` | NER | Same. |
| `Location` | NER | Same. |
| `Custom(label)` | User-supplied | `custom_terms` or `custom_patterns`. |

## Strategies

| `RedactionStrategy` | Output | Use when |
|---|---|---|
| `Mask` (default) | `[REDACTED]` | You only need PII gone. |
| `Hash` | SHA-256 truncated to 16 hex chars | You need equality joins downstream without recovering the source. |
| `TokenReplace` | `[PERSON_1]`, `[PERSON_2]`, … per category | You need to preserve co-reference inside the document. |
| `Drop` | empty string | You need the span gone with no marker. |

## User-Supplied Terms and Patterns

The most-used surface in production. Pass literal strings or regex patterns the caller already knows are sensitive.

=== "Python"

    --8<-- "snippets/python/redaction/custom_terms.md"

=== "TypeScript"

    --8<-- "snippets/typescript/redaction/custom_terms.md"

=== "Rust"

    --8<-- "snippets/rust/redaction/custom_terms.md"

`RedactionTerm.value` is regex-escaped before matching — pass literal text without escaping. `RedactionPattern.pattern` uses the Rust `regex` crate dialect (no look-around). Case-insensitive by default; set `case_sensitive = true` for exact-byte match. Patterns are validated at config-construction time via `RedactionConfig::validate()`.

User hits always surface as `PiiCategory::Custom(label)` and are retained even when `RedactionConfig.categories` filters out the built-in detectors.

## Pairing with NER

To redact names, organisations, and locations, attach a `NerConfig`:

=== "Python"

    --8<-- "snippets/python/redaction/with_ner.md"

Choose the NER backend per the [NER guide](ner.md). The LLM backend works today; the gline-rs ONNX backend is pending an upstream `ort` bump.

## Output Shape

```json
{
  "content": "Contact [REDACTED] at [REDACTED]. Reference [PROJECT_1].",
  "redaction_report": {
    "total_redacted": 3,
    "findings": [
      { "start": 8, "end": 24, "category": "person", "strategy": "mask", "replacement_token": "[REDACTED]" },
      { "start": 28, "end": 50, "category": "email", "strategy": "mask", "replacement_token": "[REDACTED]" },
      { "start": 61, "end": 75, "category": { "custom": "Project" }, "strategy": "token_replace", "replacement_token": "[PROJECT_1]" }
    ]
  }
}
```

Offsets refer to the ORIGINAL pre-redaction content. Use them only for audit-trail reconstruction — the original bytes are gone by the time the result reaches the caller.

## Data Handling

The redaction post-processor:

- Runs locally. The pattern engine makes no network calls.
- Drops the original text. Only `redaction_report` carries spans back to the original — and only as numeric offsets, never as the original characters.
- Adjusts chunk byte ranges in place when `preserve_offsets = true` (default). Set `false` to keep chunk offsets pointing at the original document.

The NER backend, when enabled, follows whichever backend you configure — see [NER](ner.md) for the network-call surface of `ner-llm`.

## Known Limitations

- **SWIFT/BIC over-matches plain English words.** The current regex (`[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}(?:[A-Z0-9]{3})?`) accepts arbitrary 8/11-letter all-caps tokens after the engine uppercases the input. Until a country-allowlist lands, scope `RedactionConfig.categories` to the subset you actually need rather than redacting everything.
- **PERSON / ORGANIZATION / LOCATION require NER.** Without `RedactionConfig.ner`, those categories are silently skipped.

## Related

- [Named-Entity Recognition](ner.md) — supplies PERSON / ORGANIZATION / LOCATION
- [LLM Integration](llm-integration.md) — backend providers for the NER LLM path
- [Configuration Reference](../reference/configuration.md#redactionconfig) — full field reference
