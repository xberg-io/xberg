# Presets

Presets define structured extraction schemas with system prompts, merge strategies, and call-mode hints for the LLM pipeline.

## Overview

A preset encapsulates the configuration needed to extract structured data from a document:

- **Schema** — JSON Schema (Draft 2020-12) describing the extraction output shape.
- **System prompt** — Instruction text sent to the LLM to guide extraction.
- **Merge mode** — How partial results from multi-page documents combine.
- **Call mode** — Whether to extract from text only, vision only, or both.
- **Citations** — Whether the prompt asks the model to emit field citations (page, bbox).

The OSS library ships exactly one preset (`generic_document`) as a synthetic example. Downstream applications load additional presets at runtime via `Registry::extend_from_dir`.

## Loading Presets

### Global Registry

Access the embedded registry via `Registry::global()`:

```rust
use xberg::presets::Registry;

let registry = Registry::global();
let preset = registry.get("generic_document").expect("always present");
```

### Loading from Disk

Load additional presets from a directory at runtime:

```rust
use xberg::presets::Registry;
use std::path::Path;

let mut registry = Registry::load_embedded()?;
let count = registry.extend_from_dir(Path::new("/path/to/presets/"))?;
println!("Loaded {} presets", count);
```

Files are read from the root of the directory (non-recursive). Each `*.json` file is validated against the preset meta-schema; malformed files cause an error.

### Iterating Presets

List all available presets:

```rust
use xberg::presets::Registry;

let registry = Registry::global();
for preset in registry.iter() {
    println!("{}: {}", preset.id, preset.description);
}
```

Query summaries (lightweight metadata):

```rust
use xberg::presets::Registry;

let registry = Registry::global();
let summaries = registry.summaries();
// Use summaries in a UI listing or API response
```

## Preset Format

Presets are JSON files with the following structure:

```json
{
  "id": "my_invoice",
  "version": "v1",
  "schema_name": "InvoiceData",
  "description": "Extract invoice line items and totals.",
  "category": "finance",
  "tags": ["invoice", "accounting"],
  "schema": {
    "type": "object",
    "properties": {
      "vendor": { "type": "string" },
      "invoice_number": { "type": "string" },
      "line_items": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "description": { "type": "string" },
            "quantity": { "type": "number" },
            "unit_price": { "type": "number" }
          }
        }
      }
    }
  },
  "system_prompt": "Extract invoice data. Return vendor name, invoice number, and line items with descriptions, quantities, and unit prices.",
  "merge_mode": "object_merge",
  "preferred_call_mode": "text_only",
  "emit_citations": false
}
```

### Field Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Stable, URL-safe identifier (lowercase snake_case). Used as the preset lookup key. |
| `version` | string | yes | Monotonic version string (e.g. `"v1"`, `"v2"`). Allows preset evolution without ID collision. |
| `schema_name` | string | yes | Human-readable name forwarded to the LLM as the response tool/function name. |
| `description` | string | yes | One-line description shown in UI and preset listings. |
| `category` | string | yes | One of: `"finance"`, `"identity"`, `"legal"`, `"logistics"`, `"medical"`, `"hr"`, `"other"`. |
| `tags` | array | no | Free-form search/filter tags. Default: empty array. |
| `schema` | object | yes | JSON Schema (Draft 2020-12) describing the extraction output shape. |
| `system_prompt` | string | yes | Instruction text sent to the model. |
| `context_template` | string | no | Optional mustache-style template merged with caller-supplied context. |
| `merge_mode` | string | yes | One of: `"object_merge"`, `"array_concat"`, `"object_first"`. |
| `preferred_call_mode` | string | yes | One of: `"text_only"`, `"vision_only"`, `"text_plus_vision"`. |
| `emit_citations` | boolean | yes | When `true`, the prompt asks the model to wrap each field as `{value, page, bbox, confidence}`. |
| `sample` | object | no | Bundled sample input + reference output for preview/testing. |

## Resolving Presets

Presets can include optional context templates. Resolve a preset by merging caller-supplied context:

```rust
use xberg::presets::{Registry, resolve};
use std::collections::BTreeMap;

let registry = Registry::global();
let preset = registry.get("my_preset")?;

let mut context = BTreeMap::new();
context.insert("company_name", "ACME Corp");

let resolved = resolve(preset, None, &context)?;
// resolved.system_prompt has mustache variables replaced with context values
```

## Call Modes

Three call modes govern how documents are sent to the extraction pipeline:

| Mode | Behavior |
|------|----------|
| `text_only` | Send extracted text only; no vision model call. |
| `vision_only` | Send page rasters only; no extracted text payload. |
| `text_plus_vision` | Fuse extracted text with page rasters in a single multimodal call. |

The `preferred_call_mode` is a hint to the orchestrator. The actual call mode chosen may be overridden by heuristics (e.g. structured-extraction confidence gating) or user override.

## Merge Modes

Merge modes control how partial results from batched calls (e.g. per-page extraction of a multi-page document) combine:

| Mode | Behavior |
|------|----------|
| `object_merge` | Deep-merge JSON objects field by field. Later calls fill missing fields in earlier results. |
| `array_concat` | Concatenate top-level arrays across calls. |
| `object_first` | Keep the first non-empty result; ignore subsequent calls. |

Choose based on your schema shape:

- Invoice line-item arrays → `array_concat`
- Singleton document metadata → `object_merge`
- Page-by-page extraction where only the first page matters → `object_first`

## Feature Gate

Preset functionality is behind the `presets` feature. Enable it in `Cargo.toml`:

```toml
[dependencies]
xberg = { version = "1.0", features = ["presets"] }
```

## Best Practices

1. **Version your presets.** Include a monotonic version in both the `version` field and in your process (e.g. file naming). This allows schema evolution without ID collision.
2. **Validate schemas.** Use JSON Schema validators during development to catch shape mismatches early.
3. **Test prompts.** Verify that your system prompt produces the desired extraction on representative documents before deployment.
4. **Use meaningful tags.** Tags enable UI-level search and filtering across large catalogs.
5. **Provide samples.** Bundle representative input/output pairs so downstream tools (playgrounds, CI) can validate preset behavior.
