# Structured Extraction

<span class="version-badge new">v5.0</span>

Structured extraction combines document extraction with LLM-based schema completion to return data matching a JSON schema.

## Overview

Structured extraction runs in three phases:

1. **Extract** — Use Kreuzberg to extract text and/or images from the document.
2. **Call** — Send the extracted content to an LLM with a schema and system prompt (from a preset).
3. **Merge** — Combine results from multiple calls (e.g. per-page batches) using the preset's merge strategy.

Call mode and merge strategy are configured in the preset; heuristics can override them at extraction time.

## Call Modes

The `CallMode` enum governs how document content is sent to the LLM:

| Mode | Behavior |
|------|----------|
| `text_only` | Send extracted text only; no vision model call. Fastest, lowest cost. Best for text-heavy documents. |
| `vision_only` | Send page rasters only; no extracted text payload. Useful for scanned/handwritten documents. |
| `text_plus_vision` | Fuse extracted text with page rasters in a single multimodal call. Highest accuracy, highest cost. |

Example in Rust:

```rust
use kreuzberg::presets::{Preset, CallMode};

let preset = Preset {
    preferred_call_mode: CallMode::TextOnly,
    ..Default::default()
};
```

In configuration (TOML, YAML, JSON), use snake_case:

```toml
[structured_extraction]
preferred_call_mode = "text_only"
```

## Merge Modes

The `MergeMode` enum controls how partial results from batched calls combine:

| Mode | Behavior |
|------|----------|
| `object_merge` | Deep-merge JSON objects field by field. Later calls fill missing fields in earlier results. |
| `array_concat` | Concatenate top-level arrays across calls. |
| `object_first` | Keep the first non-empty result; ignore subsequent calls. |

Choose based on your schema:

```rust
use kreuzberg::presets::{Preset, MergeMode};

// Merge invoice items from multiple pages
let preset = Preset {
    merge_mode: MergeMode::ArrayConcat,
    ..Default::default()
};
```

In configuration:

```toml
[structured_extraction]
merge_mode = "array_concat"
```

### Merge Mode Examples

**invoice_items: array**

Multi-page invoices often have line items spread across pages. Use `array_concat`:

```json
// Page 1 result
{ "invoice_items": [{ "description": "Item A", "amount": 10.0 }] }

// Page 2 result
{ "invoice_items": [{ "description": "Item B", "amount": 20.0 }] }

// Merged (array_concat)
{ "invoice_items": [
  { "description": "Item A", "amount": 10.0 },
  { "description": "Item B", "amount": 20.0 }
] }
```

**document_metadata: object**

Metadata fields (vendor, invoice number) typically appear once. Use `object_merge`:

```json
// Page 1 result
{ "vendor": "ACME Corp", "invoice_date": "2025-01-15" }

// Page 2 result
{ "invoice_number": "INV-123" }

// Merged (object_merge)
{ "vendor": "ACME Corp", "invoice_date": "2025-01-15", "invoice_number": "INV-123" }
```

## Heuristics

<span class="version-badge new">v5.0</span>

Heuristics automatically decide whether and how to invoke structured extraction based on document characteristics. The `heuristics` feature gate must be enabled:

```toml
[dependencies]
kreuzberg = { version = "5.0", features = ["heuristics"] }
```

### Confidence Scoring

When enabled, extraction results carry an `extraction_confidence` score combining:

- **Text coverage** — Fraction of pages with usable text (0.0..=1.0)
- **OCR quality** — Mean recognition confidence from OCR elements (when OCR ran)
- **Schema compliance** — Whether the extraction validates against your schema

The combined score is a weighted blend on `[0, 1]`:

```rust
use kreuzberg::heuristics::{score_confidence, ConfidenceSignals, ConfidenceWeights, SchemaCompliance};

let signals = ConfidenceSignals {
    text_coverage: 0.95,
    ocr_aggregate: None,  // OCR did not run
    schema_compliance: SchemaCompliance::AllValid,
};

// Default weights: text_coverage (0.30) + schema_compliance (0.40) + ocr (0.30)
let confidence = score_confidence(signals, ConfidenceWeights::default());
assert!(confidence.combined > 0.8);
```

Use the confidence score to:

1. **Gate fallbacks** — Escalate to vision if confidence < 0.7
2. **Log quality metrics** — Track confidence per document type for process improvement
3. **Alert on degradation** — Flag documents with confidence < threshold for manual review

### Call-Mode Heuristics

The `choose_call_mode` function automatically selects the best call mode for a document:

```rust
use kreuzberg::heuristics::{StructuredInput, StructuredThresholds, choose_call_mode};

let input = StructuredInput {
    mime_type: "application/pdf".to_string(),
    page_count: 10,
    text_coverage: 0.92,  // 92% of pages have text
    avg_chars_per_page: 500.0,
    embedded_image_count: 2,
    user_force_vision: false,
};

let thresholds = StructuredThresholds::default();
let mode = choose_call_mode(&input, &thresholds);
// Result: StructuredCallMode::TextOnly (high text coverage, text-bearing format)
```

Rules applied in order:

1. `image/*` → `VisionOnly` (no native text layer)
2. `application/pdf` → `TextOnly` (Kreuzberg's OCR produces text for scanned PDFs)
3. Text-heavy DOCX/HTML/text → `TextOnly` (if avg_chars_per_page > threshold)
4. Anything else → `Skip`

After selection, two post-rule promotions apply:

- `user_force_vision=true` promotes `TextOnly` → `TextPlusVision`
- `enable_vision_fallback=true` promotes `TextOnly` → `TextOnlyWithVisionFallback` (try text first, escalate on low confidence)

### Tuning Thresholds

All heuristic thresholds are conservative defaults. Deployments should measure their corpus and override:

```rust
use kreuzberg::heuristics::StructuredThresholds;

let custom = StructuredThresholds {
    scan_max_coverage: 0.15,          // Your PDFs average 15% text coverage when scanned
    digital_min_coverage: 0.85,       // Your digital PDFs hit 85%+ coverage
    docx_text_min_density: 150.0,     // Your DOCX docs average 150 chars/page
    enable_vision_fallback: true,     // Run confidence-gated escalation
};
```

| Threshold | Default | Meaning |
|-----------|---------|---------|
| `scan_max_coverage` | 0.10 | PDFs below this threshold are treated as scanned/image-heavy |
| `digital_min_coverage` | 0.90 | PDFs at/above this with zero embedded images → `TextOnly` |
| `docx_text_min_density` | 200.0 | DOCX/HTML/text with avg chars/page above this → `TextOnly` |
| `enable_vision_fallback` | false | When true, use `TextOnlyWithVisionFallback` for confidence gating |

## Structured Call Modes

The runtime heuristic returns a `StructuredCallMode` (distinct from `CallMode`), which has five variants:

| Mode | Behavior |
|------|----------|
| `Skip` | Document is unsupported or not worth invoking the pipeline. |
| `TextOnly` | Send extracted text only. |
| `VisionOnly` | Send page rasters only. |
| `TextPlusVision` | Fuse text and images in a single call. |
| `TextOnlyWithVisionFallback` | Try text-only first; escalate to vision on low confidence. |

The `TextOnlyWithVisionFallback` mode is the bridge between heuristics and orchestration: extract with text-only, check confidence, and invoke vision only if needed (avoiding unnecessary vision calls).

## Example: Invoice Extraction

```rust
use kreuzberg::{
    extract_file, ExtractionConfig,
    presets::{Registry, resolve},
    heuristics::{
        score_confidence, ConfidenceSignals, StructuredInput, StructuredThresholds,
        choose_call_mode, SchemaCompliance,
    }
};
use std::collections::BTreeMap;

// Extract the document
let config = ExtractionConfig::default();
let result = extract_file("invoice.pdf", None, &config).await?;

// Load the invoice preset
let registry = Registry::global();
let preset = registry.get("invoice").expect("preset");
let resolved = resolve(preset, None, &BTreeMap::new())?;

// Score confidence
let signals = ConfidenceSignals::from_extraction_result(
    &result,
    SchemaCompliance::AllValid,  // Assume schema validation passed
    0.95,  // 95% of pages have text
);
let confidence = kreuzberg::heuristics::score_confidence(
    signals,
    Default::default()
);

// Decide call mode
let call_input = StructuredInput {
    mime_type: result.mime_type.clone(),
    page_count: result.pages.len() as u32,
    text_coverage: 0.95,
    avg_chars_per_page: (result.content.len() / result.pages.len()) as f64,
    embedded_image_count: result.images.len() as u32,
    user_force_vision: false,
};

let call_mode = choose_call_mode(&call_input, &StructuredThresholds::default());
// Dispatch to LLM based on call_mode with resolved preset
```

## Best Practices

1. **Measure your corpus** — Run heuristics on representative documents; adjust thresholds to your baseline.
2. **Test presets** — Verify system prompts and schemas on real data before deploying.
3. **Gate on confidence** — Use `extraction_confidence` to catch degraded results before they propagate downstream.
4. **Log decisions** — Record which call mode was chosen and why for process improvement.
5. **Cache preset fingerprints** — Use `Preset::fingerprint` as a cache-invalidation token; recompute workers when presets change.
