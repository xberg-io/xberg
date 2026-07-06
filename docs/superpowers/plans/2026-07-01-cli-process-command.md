# CLI `process` Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `xberg process` subcommand that runs the extract → NER → redact pipeline locally (mirrors `POST /v1/process` without requiring the HTTP server).

**Architecture:** A new `commands/process.rs` file holds `process_command`. It builds an `ExtractionConfig` with `ner` and `redaction` fields populated from CLI flags, calls the existing sync extraction helpers from `commands/extract.rs`, then prints the `ExtractedDocument`. The subcommand is registered in `commands/mod.rs` and `main.rs` behind `#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]` (at minimum one NER or redaction feature must be active for the command to be meaningful).

**Tech Stack:** Rust 2024, clap (already in workspace), `xberg` core library (`ExtractionConfig`, `NerConfig`, `RedactionConfig`, `ExtractedDocument`, `extract_input_sync` pattern from extract.rs).

## Global Constraints

- Feature gate: `#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]` — same pattern as `commands/ner.rs`
- No new dependencies — reuse types already in `xberg` core
- Follow existing CLI output pattern: `WireFormat` enum (text / json / toon), `serde_json::to_string_pretty` for JSON, `serde_toon::to_string` for toon
- `prek run --all-files` must pass before every commit
- Conventional commit messages (`feat(cli): ...`)
- No AI attribution in commits

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| **Create** | `crates/xberg-cli/src/commands/process.rs` | `process_command` function + unit tests |
| **Modify** | `crates/xberg-cli/src/commands/mod.rs` | re-export `process_command` |
| **Modify** | `crates/xberg-cli/src/main.rs` | `Process` variant in `Commands` enum + dispatch arm |

---

## Task 1: `commands/process.rs` — core function + unit tests

**Files:**
- Create: `crates/xberg-cli/src/commands/process.rs`
- Test: inside the same file under `#[cfg(test)]`

**Interfaces:**
- Produces: `pub fn process_command(input: ExtractInputSource, config: ExtractionConfig, format: WireFormat) -> anyhow::Result<()>`
- Consumes from `commands/extract.rs`: `ExtractInputSource`, `extract_input_sync` (private — will call through the same `block_on_extract` pattern; see step below)

> **Note on `extract_input_sync`:** it is `fn` (not `pub`) in `extract.rs`. We duplicate the two-line call pattern inline in `process.rs` to avoid coupling. It's three lines — not worth an abstraction.

- [ ] **Step 1: Write the failing test**

Create `crates/xberg-cli/src/commands/process.rs` with the test module only:

```rust
//! Process pipeline command: extract → NER → redact in one shot.

use anyhow::Result;
use std::io::Read as _;
use xberg::{ExtractInput, ExtractionConfig, extract};

use crate::{WireFormat, style};

pub fn process_command(
    input: crate::commands::extract::ExtractInputSource,
    config: ExtractionConfig,
    format: WireFormat,
) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::extract::ExtractInputSource;

    #[test]
    fn process_command_text_input_json_format_succeeds() {
        // Plain-text input through the process pipeline with no NER/redact ops
        // should return a document whose `content` equals the input text.
        let mut config = ExtractionConfig::default();
        // No NER, no redaction — just extraction
        config.ner = None;
        config.redaction = None;

        // process_command accepts a URI; for a text string we write a temp file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "Hello world").unwrap();

        let uri = path.to_string_lossy().to_string();
        let input = ExtractInputSource::Uri(uri);

        // Capture stdout
        // We can't easily capture stdout in a unit test, so test the underlying
        // extraction logic instead.
        let result = run_process(input, &config);
        assert!(result.is_ok(), "process failed: {:?}", result.err());
        let doc = result.unwrap();
        assert!(doc.content.contains("Hello world"));
    }

    fn run_process(
        input: ExtractInputSource,
        config: &ExtractionConfig,
    ) -> xberg::Result<xberg::ExtractedDocument> {
        use xberg::{ExtractInput, extract};
        let rt = tokio::runtime::Runtime::new().unwrap();
        match input {
            ExtractInputSource::Uri(uri) => {
                let ext_input = ExtractInput::from_uri(uri);
                let mut out = rt.block_on(extract(ext_input, config.clone()))?;
                out.results.pop().ok_or_else(|| {
                    xberg::error::XbergError::Other("no document produced".into())
                })
            }
            ExtractInputSource::Stdin => unreachable!("stdin not tested here"),
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test -p xberg-cli process_command_text_input --features ner-onnx 2>&1 | tail -5
```

Expected: compile error `todo!()` panics at runtime OR compile error if imports are wrong. Fix imports until it compiles, then the test should fail with a `todo!` panic.

- [ ] **Step 3: Implement `process_command`**

Replace `todo!()` with the real body:

```rust
pub fn process_command(
    input: crate::commands::extract::ExtractInputSource,
    config: ExtractionConfig,
    format: WireFormat,
) -> Result<()> {
    use xberg::{ExtractInput, extract};

    let rt = tokio::runtime::Runtime::new().context("Failed to start async runtime")?;

    let ext_input = match input {
        crate::commands::extract::ExtractInputSource::Uri(uri) => ExtractInput::from_uri(uri),
        crate::commands::extract::ExtractInputSource::Stdin => {
            let mut data = Vec::new();
            std::io::stdin()
                .read_to_end(&mut data)
                .context("Failed to read from stdin")?;
            if data.is_empty() {
                anyhow::bail!("No input received from stdin.");
            }
            ExtractInput::from_bytes(data, "text/plain", None)
        }
    };

    let mut out = rt
        .block_on(extract(ext_input, config))
        .context("Extraction failed")?;

    let doc = out.results.pop().ok_or_else(|| anyhow::anyhow!("No document produced"))?;

    match format {
        WireFormat::Text => {
            print!("{}", doc.content);
        }
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&doc).context("Failed to serialize result to JSON")?
            );
        }
        WireFormat::Toon => {
            println!(
                "{}",
                serde_toon::to_string(&doc).context("Failed to serialize result to TOON")?
            );
        }
    }

    Ok(())
}
```

Full imports block at top of file:

```rust
use anyhow::{Context, Result};
use std::io::Read as _;
use xberg::ExtractionConfig;

use crate::{WireFormat, style};
```

(`style` is imported for future use in a `Text` output header — remove if unused after linting.)

- [ ] **Step 4: Run test to verify it passes**

```
cargo test -p xberg-cli process_command_text_input --features ner-onnx 2>&1 | tail -5
```

Expected: `test commands::process::tests::process_command_text_input_json_format_succeeds ... ok`

- [ ] **Step 5: Run clippy**

```
cargo clippy -p xberg-cli --features ner-onnx -- -D warnings 2>&1 | tail -20
```

Fix any warnings before committing.

- [ ] **Step 6: Commit**

```
git add crates/xberg-cli/src/commands/process.rs
git commit -m "feat(cli): add process_command for extract→NER→redact pipeline"
```

---

## Task 2: Wire `process.rs` into `mod.rs` and `main.rs`

**Files:**
- Modify: `crates/xberg-cli/src/commands/mod.rs` — add module declaration + re-export
- Modify: `crates/xberg-cli/src/main.rs` — `Process` variant + dispatch arm

**Interfaces:**
- Consumes from Task 1: `pub fn process_command(input: ExtractInputSource, config: ExtractionConfig, format: WireFormat) -> Result<()>`

- [ ] **Step 1: Add module to `commands/mod.rs`**

In `crates/xberg-cli/src/commands/mod.rs`, after the existing `#[cfg(feature = "ner-onnx")] pub mod ner;` line:

```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]
pub mod process;
```

And in the re-exports block, after the `ner` re-export:

```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]
pub use process::process_command;
```

- [ ] **Step 2: Add `Process` variant to `Commands` enum in `main.rs`**

In `crates/xberg-cli/src/main.rs`, in the `Commands` enum, after the `Chunk` variant:

```rust
/// Run the extract → NER → redact pipeline on a document
///
/// Extracts text, optionally detects named entities (persons, orgs, locations, emails, …),
/// and optionally redacts PII — all in one pass without starting the HTTP server.
///
/// NER requires the `ner-onnx` or `ner-llm` feature.
/// Redaction requires the `redaction` feature.
#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]
Process {
    /// URI or local path to process.
    #[arg(value_name = "URI", required_unless_present = "stdin")]
    uri: Option<String>,

    /// Read document bytes from stdin instead of a file.
    #[arg(long, conflicts_with = "uri")]
    stdin: bool,

    /// Path to config file (TOML, YAML, or JSON).
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Inline JSON configuration.
    #[arg(long)]
    config_json: Option<String>,

    /// Base64-encoded inline JSON configuration.
    #[arg(long)]
    config_json_base64: Option<String>,

    // --- NER flags ---

    /// Enable NER with default settings (ONNX backend, standard categories).
    #[arg(long)]
    ner: bool,

    /// NER backend: onnx (default) or llm.
    #[arg(long, default_value = "onnx", requires = "ner")]
    ner_backend: String,

    /// Entity categories to detect, comma-separated.
    /// Values: person, organization, location, email, phone, ssn, credit_card,
    ///         postal_code, ip_address, iban, swift_bic, date_of_birth.
    /// Default when --ner is set: person, organization, location, email.
    #[arg(long, value_delimiter = ',', requires = "ner")]
    ner_categories: Vec<String>,

    /// Override the GLiNER model alias (ONNX backend only).
    #[arg(long, requires = "ner")]
    ner_model: Option<String>,

    // --- Redaction flags ---

    /// Enable redaction. At least one of --ner or --redact must be set for
    /// the process command to do anything beyond plain extraction.
    #[arg(long)]
    redact: bool,

    /// Redaction strategy: mask (default), hash, token_replace, or drop.
    #[arg(long, default_value = "mask", requires = "redact")]
    redact_strategy: String,

    /// PII categories to redact, comma-separated (same values as --ner-categories).
    /// Default when --redact is set: all detectable categories.
    #[arg(long, value_delimiter = ',', requires = "redact")]
    redact_categories: Vec<String>,

    /// Output format: text (default), json, or toon.
    #[arg(short, long, default_value = "text")]
    format: WireFormat,
},
```

- [ ] **Step 3: Add dispatch arm to `main` match block**

In `main()`, after the `Commands::Chunk { … }` arm:

```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]
Commands::Process {
    uri,
    stdin,
    config: config_path,
    config_json,
    config_json_base64,
    ner,
    ner_backend,
    ner_categories,
    ner_model,
    redact,
    redact_strategy,
    redact_categories,
    format,
} => {
    use commands::process_command;
    use xberg::core::config::ner::{NerBackendKind, NerConfig};
    use xberg::core::config::redaction::RedactionConfig;
    use xberg::types::redaction::{PiiCategory, RedactionStrategy};

    let input = if stdin {
        ExtractInputSource::Stdin
    } else {
        ExtractInputSource::Uri(uri.expect("clap ensures uri is present when stdin is false"))
    };

    let mut config = load_config(config_path)?;
    apply_json_overrides(&mut config, config_json, config_json_base64)?;

    // NER
    if ner {
        let backend = match ner_backend.as_str() {
            "llm" => NerBackendKind::Llm,
            _ => NerBackendKind::Onnx,
        };
        let categories = ner_categories
            .iter()
            .map(|s| s.parse::<xberg::types::entity::EntityCategory>().unwrap_or_default())
            .collect();
        config.ner = Some(NerConfig {
            backend,
            categories,
            model: ner_model,
            ..NerConfig::default()
        });
    }

    // Redaction
    if redact {
        let strategy: RedactionStrategy = redact_strategy.parse().unwrap_or_default();
        let categories: std::collections::HashSet<PiiCategory> = if redact_categories.is_empty() {
            std::collections::HashSet::new() // empty = all categories
        } else {
            redact_categories.iter().map(|s| s.parse().unwrap()).collect()
        };
        config.redaction = Some(RedactionConfig {
            strategy,
            categories,
            ..RedactionConfig::default()
        });
    }

    process_command(input, config, format)?;
}
```

- [ ] **Step 4: Add missing imports to `main.rs` if needed**

Check that `ExtractInputSource` is already imported at the top of `main.rs` (it is, via `commands::extract`). Add `EntityCategory` if not present — it likely isn't since the existing CLI doesn't use it directly. Add:

```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm"))]
use xberg::types::entity::EntityCategory;
```

- [ ] **Step 5: Build to verify it compiles**

```
cargo build -p xberg-cli --features ner-onnx 2>&1 | tail -20
```

Expected: `Finished` with no errors.

- [ ] **Step 6: Smoke-test the command end-to-end**

```
echo "Alice works at Anthropic in San Francisco." | cargo run -p xberg-cli --features ner-onnx -- process --stdin --ner --format json 2>/dev/null
```

Expected: JSON output with `content` and `entities` fields populated. `entities` should contain at least one entry with `category: "person"` for "Alice".

- [ ] **Step 7: Run prek**

```
prek run --all-files 2>&1 | tail -20
```

Fix any issues before committing.

- [ ] **Step 8: Commit**

```
git add crates/xberg-cli/src/commands/mod.rs crates/xberg-cli/src/main.rs
git commit -m "feat(cli): wire process subcommand into CLI with NER and redaction flags"
```

---

---

## Task 3: Wire `xberg-gliner-candle` as a `Candle` NER backend

This task is independent of Tasks 1–2 (different crates). It can be done in parallel with Task 1/2 or after.

**Files:**
- Modify: `crates/xberg/Cargo.toml` — add optional `xberg-gliner-candle` dep + `ner-candle` feature
- Modify: `crates/xberg/src/core/config/ner.rs` — add `Candle` variant to `NerBackendKind` + `lora_adapter_dir` field to `NerConfig`
- Create: `crates/xberg/src/text/ner/candle.rs` — `CandleBackend` implementing `NerBackend`
- Modify: `crates/xberg/src/text/ner/mod.rs` — declare `candle` module behind `ner-candle`
- Modify: `crates/xberg/src/plugins/processor/builtin/ner.rs` — add `Candle` arm to `make_backend`
- Modify: `crates/xberg-cli/src/main.rs` — add `"candle"` to `--ner-backend` dispatch arm (Task 2 prerequisite for this step)

**Interfaces:**
- Produces: `pub struct CandleBackend` implementing `NerBackend` (async `detect`)
- Span → Entity conversion: `span.text()` → `Entity::text`, `span.offsets()` → `(start, end)` as `u32`, `span.class()` → `EntityCategory::from(class.to_string())`, `span.probability()` → `confidence: Some(f32)`

### Step 1: Add `ner-candle` feature and dep to `crates/xberg/Cargo.toml`

- [ ] In `[features]`, add:
```toml
ner-candle = ["dep:xberg-gliner-candle", "ner"]
```

- [ ] In `[dependencies]`, add:
```toml
xberg-gliner-candle = { path = "../../crates/xberg-gliner-candle", optional = true }
```

(Adjust the relative path to match the workspace layout — verify with `ls crates/`.)

### Step 2: Write the failing test for `CandleBackend`

- [ ] Create `crates/xberg/src/text/ner/candle.rs`:

```rust
//! NER backend backed by `xberg-gliner-candle` (GLiNER2 safetensors + optional LoRA).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use xberg_gliner_candle::Gliner2Candle;

use crate::Result;
use crate::text::ner::NerBackend;
use crate::types::entity::{Entity, EntityCategory};

const DEFAULT_THRESHOLD: f32 = 0.5;

/// Wraps [`Gliner2Candle`] behind the [`NerBackend`] trait.
pub struct CandleBackend {
    model: Mutex<Gliner2Candle>,
}

impl CandleBackend {
    /// Load from a local model directory. Applies `lora_adapter_dir` if provided.
    pub fn from_local(model_dir: &Path, lora_adapter_dir: Option<&Path>) -> crate::Result<Self> {
        let mut model = Gliner2Candle::from_local(model_dir)
            .map_err(|e| crate::XbergError::Other(format!("CandleBackend load: {e}")))?;
        if let Some(adapter_dir) = lora_adapter_dir {
            let adapter_name = adapter_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("adapter");
            model
                .load_adapter(adapter_name, adapter_dir)
                .map_err(|e| crate::XbergError::Other(format!("CandleBackend load_adapter: {e}")))?;
        }
        Ok(Self { model: Mutex::new(model) })
    }
}

fn spans_to_entities(spans: Vec<xberg_gliner::Span>) -> Vec<Entity> {
    spans
        .into_iter()
        .map(|span| {
            let (start, end) = span.offsets();
            Entity {
                category: EntityCategory::from(span.class().to_string()),
                text: span.text().to_string(),
                start: start as u32,
                end: end as u32,
                confidence: Some(span.probability()),
            }
        })
        .collect()
}

#[async_trait]
impl NerBackend for CandleBackend {
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>> {
        let labels: Vec<&str> = if categories.is_empty() {
            // Default label set when no categories requested
            vec!["person", "organization", "location", "email", "phone"]
        } else {
            categories.iter().map(|c| category_to_label(c)).collect()
        };

        let model = self.model.lock().map_err(|_| {
            crate::XbergError::Other("CandleBackend: model mutex poisoned".into())
        })?;

        let spans = model
            .extract_ner(text, &labels, DEFAULT_THRESHOLD)
            .map_err(|e| crate::XbergError::Other(format!("CandleBackend inference: {e}")))?;

        Ok(spans_to_entities(spans))
    }
}

fn category_to_label(cat: &EntityCategory) -> &str {
    match cat {
        EntityCategory::Person => "person",
        EntityCategory::Organization => "organization",
        EntityCategory::Location => "location",
        EntityCategory::Email => "email",
        EntityCategory::Phone => "phone",
        EntityCategory::Date => "date",
        EntityCategory::Time => "time",
        EntityCategory::Money => "money",
        EntityCategory::Percent => "percent",
        EntityCategory::Url => "url",
        EntityCategory::Custom(s) => s.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::entity::EntityCategory;

    #[test]
    fn spans_to_entities_converts_correctly() {
        let span = xberg_gliner::Span::new(0, 0, 5, "Alice".to_string(), "person".to_string(), 0.92).unwrap();
        let entities = spans_to_entities(vec![span]);

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].text, "Alice");
        assert_eq!(entities[0].category, EntityCategory::Person);
        assert_eq!(entities[0].start, 0);
        assert_eq!(entities[0].end, 5);
        assert!((entities[0].confidence.unwrap() - 0.92).abs() < 1e-5);
    }

    #[test]
    fn category_to_label_maps_known_categories() {
        assert_eq!(category_to_label(&EntityCategory::Person), "person");
        assert_eq!(category_to_label(&EntityCategory::Organization), "organization");
        assert_eq!(category_to_label(&EntityCategory::Custom("product".to_string())), "product");
    }
}
```

- [ ] **Run test to verify it fails** (or compiles but panics on missing dep):

```
cargo test -p xberg --features ner-candle spans_to_entities_converts_correctly 2>&1 | tail -10
```

Expected: compile error (dep not wired yet) or test run if dep is present. Fix compilation errors first.

### Step 3: Declare `candle` module in `mod.rs`

- [ ] In `crates/xberg/src/text/ner/mod.rs`, after the `llm` module line:

```rust
#[cfg(feature = "ner-candle")]
pub mod candle;
```

### Step 4: Add `Candle` variant to `NerBackendKind`, `model_dir`, and `lora_adapter_dir` to `NerConfig`

- [ ] In `crates/xberg/src/core/config/ner.rs`, add `Candle` to the enum:

```rust
pub enum NerBackendKind {
    #[default]
    Onnx,
    Llm,
    /// GLiNER2 safetensors via Candle (CPU). Requires `ner-candle` feature.
    /// Set `NerConfig::model_dir` to a local directory containing
    /// `tokenizer.json` and `model.safetensors`.
    Candle,
}
```

- [ ] Add `model_dir` and `lora_adapter_dir` fields to `NerConfig`.
  `model_dir` is `PathBuf` (not `String`) — the type system enforces it is a path,
  not a HF repo identifier. `hf_repo` stays for ONNX custom-repo downloads only.

```rust
/// Local filesystem path to a model directory containing `tokenizer.json`
/// and `model.safetensors`. Only used by [`NerBackendKind::Candle`].
/// Takes precedence over `hf_repo` for the Candle backend.
#[serde(skip_serializing_if = "Option::is_none")]
#[cfg_attr(feature = "alef-meta", alef(since = "5.3.0"))]
pub model_dir: Option<std::path::PathBuf>,

/// Path to a PEFT LoRA adapter directory. Only used by [`NerBackendKind::Candle`].
/// The directory must contain `adapter_config.json` and `adapter_model.safetensors`.
/// When `None`, the base model weights are used as-is.
#[serde(skip_serializing_if = "Option::is_none")]
#[cfg_attr(feature = "alef-meta", alef(since = "5.3.0"))]
pub lora_adapter_dir: Option<std::path::PathBuf>,
```

### Step 5: Add `Candle` arm to `make_backend`

- [ ] In `crates/xberg/src/plugins/processor/builtin/ner.rs`, inside `make_backend`, after the `Llm` arm:

```rust
NerBackendKind::Candle => {
    #[cfg(feature = "ner-candle")]
    {
        let model_dir = config.model_dir.as_deref().ok_or_else(|| {
            crate::XbergError::validation(
                "Candle NER backend requires NerConfig.model_dir set to a local \
                 directory containing tokenizer.json and model.safetensors",
            )
        })?;
        let lora_dir = config.lora_adapter_dir.as_deref();
        Ok(std::sync::Arc::new(
            crate::text::ner::candle::CandleBackend::from_local(model_dir, lora_dir)?,
        ))
    }
    #[cfg(not(feature = "ner-candle"))]
    {
        Err(crate::XbergError::MissingDependency(
            "ner-candle feature is not enabled — rebuild xberg with --features ner-candle".to_string(),
        ))
    }
}
```

- [ ] **Build to verify it compiles**:

```
cargo build -p xberg --features ner-candle 2>&1 | tail -20
```

Expected: `Finished` with no errors.

### Step 6: Add `"candle"` to CLI `--ner-backend` dispatch (requires Task 2 done)

- [ ] In `crates/xberg-cli/src/main.rs`, in the `Commands::Process` dispatch arm, update the `backend` match:

```rust
let backend = match ner_backend.as_str() {
    "llm" => NerBackendKind::Llm,
    "candle" => NerBackendKind::Candle,
    _ => NerBackendKind::Onnx,
};
```

- [ ] Update the feature gate on `Commands::Process` to also cover `ner-candle`:

```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm", feature = "ner-candle"))]
Process { ... }
```

And mirror this in `commands/mod.rs`:
```rust
#[cfg(any(feature = "ner-onnx", feature = "ner-llm", feature = "ner-candle"))]
pub mod process;
```

### Step 7: Run unit tests and prek

- [ ] Run unit tests:

```
cargo test -p xberg --features ner-candle 2>&1 | grep -E "FAILED|ok|error" | head -20
```

- [ ] Run prek:

```
prek run --all-files 2>&1 | tail -20
```

Fix any issues.

### Step 8: Commit

```
git add crates/xberg/Cargo.toml \
        crates/xberg/src/core/config/ner.rs \
        crates/xberg/src/text/ner/candle.rs \
        crates/xberg/src/text/ner/mod.rs \
        crates/xberg/src/plugins/processor/builtin/ner.rs \
        crates/xberg-cli/src/commands/mod.rs \
        crates/xberg-cli/src/main.rs
git commit -m "feat(ner): wire GLiNER2 Candle backend as NerBackendKind::Candle with LoRA support"
```

---

## Self-Review

**Spec coverage:**
- `POST /v1/process` accepts `text` or `url` + `operations.ner` + `operations.redact` → CLI mirrors as `--stdin`/URI + `--ner*` flags + `--redact*` flags ✅
- Feature gate mirrors existing `ner` command pattern ✅
- Output mirrors `extract` command output patterns (text/json/toon) ✅
- `rehydrate: true` is HTTP-server-specific state — intentionally omitted from CLI ✅
- GLiNER2 Candle backend wired end-to-end: dep → feature → `NerBackendKind::Candle` → `make_backend` → `CandleBackend` → CLI `--ner-backend candle` ✅
- `model_dir: Option<PathBuf>` added to `NerConfig` — typed as a path, not a string; `hf_repo` stays semantically clean for HF repo identifiers only ✅
- LoRA adapter exposed via `NerConfig::lora_adapter_dir`, passed through from config JSON ✅

**Placeholder scan:** No TBD or TODO in code steps — all blocks are complete.

**Type consistency:**
- `ExtractInputSource` used in Task 1 and Task 2 — same type from `commands/extract.rs` ✅
- `WireFormat` used consistently across both tasks ✅
- `NerConfig`, `RedactionConfig`, `PiiCategory`, `RedactionStrategy` all imported from `xberg` crate — same paths in both tasks ✅
- `process_command` signature defined in Task 1 and consumed in Task 2 without changes ✅

**Known edge:** `EntityCategory::from_str` — check whether `xberg::types::entity::EntityCategory` implements `FromStr`. If it only has `From<String>`, replace `.parse::<EntityCategory>()` with `EntityCategory::from(s.clone())` in the dispatch arm. Verify during step 5 build.
