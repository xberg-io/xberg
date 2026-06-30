# GLiNER2 ONNX Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let xberg's NER pipeline run GLiNER2 ("schema-prompt") ONNX exports — e.g. `fastino/gliner2`-lineage models — in addition to the GLiNER1 ("span-mode") models it already supports, selectable per-call via `NerConfig.hf_architecture`.

**Architecture:** GLiNER1 and GLiNER2 are different model architectures with different ONNX tensor contracts (confirmed by reading `crates/xberg-gliner/src/tensor.rs`/`session.rs` against the upstream `lion-ai/gliner2-base-v1-onnx` reference `example.rs`). They cannot share one inference path. Add a parallel `Gliner2` engine to `xberg-gliner` (new tokenization, tensor-building, and decode modules, all prefixed `v2_`) alongside the existing `Gliner` (v1) engine, sharing only the crate's error type and the `Span`/`SpanOutput`/`greedy_search` decode-merge utilities. Wire a new `GlinerArchitecture` enum through `NerConfig` → `CustomGlinerSource` → `GlineBackend`, which holds either engine behind an internal enum and dispatches at inference time. The pinned `xberg-io/gliner-models` catalog stays GLiNER1-only; GLiNER2 is only reachable via the `hf_repo` custom-source override built in the prior session turn.

**Tech Stack:** Rust, `ort` 2.0.0-rc.12 (ONNX Runtime), `tokenizers` 0.23, `ndarray`. NAPI-RS bindings in `crates/xberg-node` (hand-patched — see Global Constraints). TypeScript MCP server in `mcp-server/`.

## Global Constraints

- `task`/`alef` CLIs are **not installed on this machine** (confirmed via exhaustive PATH and disk search — no binary, no sibling `alef` repo). Every task that touches `crates/xberg-node/src/lib.rs` or `crates/xberg-node/index.d.ts` must hand-edit those generated files, mirroring Alef's existing codegen patterns exactly (field order, `#[napi(js_name = "...")]` camelCase, `#[serde(rename = "...")]`). When `alef`/`task` become available again, run `task alef:generate` once and confirm it produces no diff.
- `crates/xberg/` is documented in `CLAUDE.md` as "zero changes, always conflict-free... never modify; pull changes from upstream directly." This plan modifies it anyway (no upstream equivalent of this feature exists). This was already true before this plan — `crates/xberg/src/core/config/ner.rs`, `text/ner/gline.rs`, `text/ner/mod.rs`, `plugins/processor/builtin/ner.rs`, and `text/redaction/engine.rs` are already locally modified from the prior session turn. This plan adds further changes to the same files plus `crates/xberg-gliner/` (which has zero upstream-conflict risk — it's already a fork-local crate per `crate-structure` context).
- `cargo check` does **not** type-check `#[cfg(test)]` code by default — verify each task's tests with `cargo test -p <crate> --lib -- --skip smoke_test`, not just `cargo check`. (A real bug from this exact gap was caught and fixed in `gline.rs` this session: `backend_cache_key`'s signature changed but two test call sites weren't updated, and `cargo check` didn't catch it.)
- Workspace-pinned `ort` version is `2.0.0-rc.12` (`Cargo.toml:87`) — confirmed to match the version the reference GLiNER2 ONNX example (`lion-ai/gliner2-base-v1-onnx/example.rs`) was written against.
- No new Cargo dependencies are needed in `xberg-gliner` — `regex`, `tokenizers`, `ndarray`, `ort`, `thiserror`, `parking_lot` already cover everything GLiNER2 needs.
- GLiNER2's published ONNX exports hardcode batch size 1 in their tensor shapes (`input_ids: (1, seq_len)`, `span_idx: (1, num_words*max_width, 2)`, no batching dimension elsewhere). `Gliner2::inference` accepts exactly one text per call — this matches how `gline.rs` already calls the engine in practice (`TextInput::from_str(&[text.as_str()], &label_refs)`, always one text).
- GLiNER2's `span_scores` output is **already post-sigmoid** (the reference example compares `score >= THRESHOLD` directly, no sigmoid). This is different from GLiNER1's `logits` output, which is pre-sigmoid (`crates/xberg-gliner/src/decode.rs:262-264` applies `sigmoid()` before thresholding). Do not apply sigmoid in the v2 decode path.
- The exact model the user wants (`fastino/gliner2-privacy-filter-PII-multi`) ships `model.safetensors` only — **no ONNX export exists for it**. This plan makes xberg capable of running *any* GLiNER2 ONNX export; it does not and cannot make that specific model usable without someone exporting it to ONNX first (a separate, out-of-scope, Python-side task). The smoke test in Task 9 targets `lion-ai/gliner2-base-v1-onnx` instead — the only publicly available monolithic single-file GLiNER2 ONNX export found.

---

### Task 1: GLiNER2 pre-tokenized tokenizer wrapper

**Files:**
- Create: `crates/xberg-gliner/src/v2_tokenizer.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add `mod v2_tokenizer;`)

**Interfaces:**
- Consumes: nothing new (uses `crate::{GlinerError, Result}` from `error.rs`, and the `tokenizers` crate already in `Cargo.toml`).
- Produces: `pub(crate) struct PretokenizedEncoding { pub(crate) ids: Vec<i64>, pub(crate) word_ids: Vec<Option<u32>> }`, `pub(crate) trait PretokenizingTokenizer { fn encode_pretokenized(&self, words: Vec<&str>) -> Result<PretokenizedEncoding>; }`, `pub(crate) struct V2Tokenizer` implementing it, with `V2Tokenizer::from_file<P: AsRef<Path>>(path: P) -> Result<Self>`. Task 2 consumes the trait (not the concrete type) so tests can substitute a fake.

- [ ] **Step 1: Write the file**

```rust
// crates/xberg-gliner/src/v2_tokenizer.rs
use std::path::Path;

use crate::{GlinerError, Result};

/// Pre-tokenized encoding: token ids plus the source-word index of each token,
/// as returned by `tokenizers::Tokenizer::encode` in pre-tokenized mode.
pub(crate) struct PretokenizedEncoding {
    pub(crate) ids: Vec<i64>,
    pub(crate) word_ids: Vec<Option<u32>>,
}

/// Encodes a pre-split sequence of words, tracking which output token came from
/// which input word. GLiNER2's schema-prompt framing needs this mapping to locate
/// `[P]`/`[E]` marker tokens and text-word start positions in the final sequence.
pub(crate) trait PretokenizingTokenizer {
    fn encode_pretokenized(&self, words: Vec<&str>) -> Result<PretokenizedEncoding>;
}

/// Wraps a raw `tokenizers::Tokenizer` for GLiNER2's pre-tokenized encoding mode.
///
/// Unlike [`crate::tokenizer::HFTokenizer`] (whole-string encode), GLiNER2 requires
/// word-level pre-tokenized input so the resulting token-to-word mapping can be used
/// to locate `text_positions` and `schema_positions` in the encoded sequence.
pub(crate) struct V2Tokenizer {
    inner: tokenizers::Tokenizer,
}

impl V2Tokenizer {
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let inner = tokenizers::Tokenizer::from_file(path)
            .map_err(|error| GlinerError::Tokenizer(format!("failed to load tokenizer from file: {error}")))?;
        Ok(Self { inner })
    }
}

impl PretokenizingTokenizer for V2Tokenizer {
    fn encode_pretokenized(&self, words: Vec<&str>) -> Result<PretokenizedEncoding> {
        let encoding = self
            .inner
            .encode(words, false)
            .map_err(|error| GlinerError::Tokenizer(format!("failed to encode pre-tokenized input: {error}")))?;
        Ok(PretokenizedEncoding {
            ids: encoding.get_ids().iter().map(|&id| i64::from(id)).collect(),
            word_ids: encoding.get_word_ids().to_vec(),
        })
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/xberg-gliner/src/lib.rs`, the module list is alphabetically sorted:

```rust
mod config;
mod decode;
mod engine;
mod error;
mod input;
mod preprocess;
mod session;
mod splitter;
mod tensor;
mod tokenizer;
```

Add `mod v2_tokenizer;` after `mod tokenizer;`:

```rust
mod config;
mod decode;
mod engine;
mod error;
mod input;
mod preprocess;
mod session;
mod splitter;
mod tensor;
mod tokenizer;
mod v2_tokenizer;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p xberg-gliner`
Expected: clean compile, no warnings. (`V2Tokenizer`/`PretokenizingTokenizer` are unused at this point — `cargo check` will warn `dead_code` since nothing references them yet. That's expected and will resolve once Task 2 consumes the trait. If the warning bothers you, confirm it disappears after Task 2; do not silence it with `#[allow]` here.)

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/v2_tokenizer.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add GLiNER2 pre-tokenized tokenizer wrapper"
```

---

### Task 2: GLiNER2 word splitter

**Files:**
- Create: `crates/xberg-gliner/src/v2_splitter.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add `mod v2_splitter;`)
- Test: inline `#[cfg(test)]` block in the same file (this module has no model/tokenizer dependency, so it's fully unit-testable in isolation — no fakes needed)

**Interfaces:**
- Consumes: `crate::{Result, Token}` (the `Token` type from `input.rs`, already `pub` with a public `text()`/`start()`/`end()` API).
- Produces: `pub(crate) struct V2Splitter` with `V2Splitter::new() -> Result<Self>` and `fn split(&self, input: &str) -> Vec<Token>`. Task 3 (preprocessing) consumes this.

- [ ] **Step 1: Write the failing test**

```rust
// crates/xberg-gliner/src/v2_splitter.rs
use regex::Regex;

use crate::{Result, Token};

pub(crate) const V2_SPLITTER_REGEX: &str =
    r"(?i)(?:https?://[^\s]+|www\.[^\s]+)|[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}|@[a-z0-9_]+|\w+(?:[-_]\w+)*|\S";

/// GLiNER2 word splitter. Lowercases the input before matching, mirroring the
/// `fastino/gliner2` reference preprocessing the model was trained against.
///
/// Byte offsets are taken from the lowercased copy and applied back to the
/// original text. This holds for ASCII and most Latin-script text. Characters
/// whose lowercase form changes byte length (e.g. Turkish dotted İ) can yield
/// misaligned spans — the upstream reference implementation has the same
/// limitation, so this preserves parity rather than diverging from it.
pub(crate) struct V2Splitter {
    regex: Regex,
}

impl V2Splitter {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            regex: Regex::new(V2_SPLITTER_REGEX)?,
        })
    }

    pub(crate) fn split(&self, input: &str) -> Vec<Token> {
        let lowered = input.to_lowercase();
        self.regex
            .find_iter(&lowered)
            .map(|m| Token::new(m.start(), m.end(), m.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_lowercases_plain_words() {
        let splitter = V2Splitter::new().expect("valid regex");
        let tokens = splitter.split("Steve Jobs founded Apple Inc.");
        let texts: Vec<&str> = tokens.iter().map(Token::text).collect();
        assert_eq!(texts, vec!["steve", "jobs", "founded", "apple", "inc", "."]);
    }

    #[test]
    fn matches_emails_and_urls_as_single_tokens() {
        let splitter = V2Splitter::new().expect("valid regex");
        let tokens = splitter.split("contact ada@example.com or https://example.com/path now");
        let texts: Vec<&str> = tokens.iter().map(Token::text).collect();
        assert_eq!(
            texts,
            vec!["contact", "ada@example.com", "or", "https://example.com/path", "now"]
        );
    }

    #[test]
    fn preserves_byte_offsets_into_original_text() {
        let splitter = V2Splitter::new().expect("valid regex");
        let text = "Apple Inc. was founded in Cupertino.";
        let tokens = splitter.split(text);
        let cupertino = tokens.iter().find(|token| token.text() == "cupertino").expect("found");
        assert_eq!(&text[cupertino.start()..cupertino.end()], "Cupertino");
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p xberg-gliner --lib v2_splitter`
Expected: 3 tests pass (`splits_and_lowercases_plain_words`, `matches_emails_and_urls_as_single_tokens`, `preserves_byte_offsets_into_original_text`).

- [ ] **Step 3: Register the module**

In `crates/xberg-gliner/src/lib.rs`, after the `mod v2_tokenizer;` line added in Task 1:

```rust
mod v2_splitter;
mod v2_tokenizer;
```

(Alphabetical: `v2_splitter` before `v2_tokenizer`.)

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/v2_splitter.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add GLiNER2 word splitter"
```

---

### Task 3: GLiNER2 schema-prompt preprocessing

**Files:**
- Create: `crates/xberg-gliner/src/v2_preprocess.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add `mod v2_preprocess;`)

**Interfaces:**
- Consumes: `PretokenizingTokenizer`/`PretokenizedEncoding` (Task 1), `V2Splitter` (Task 2), `crate::{GlinerError, Result, Token}`.
- Produces: `pub(crate) struct V2Encoded { pub(crate) input_ids: Vec<i64>, pub(crate) text_positions: Vec<i64>, pub(crate) schema_positions: Vec<i64>, pub(crate) words: Vec<Token> }`, `pub(crate) fn encode_v2(text: &str, labels: &[String], tokenizer: &impl PretokenizingTokenizer, splitter: &V2Splitter) -> Result<V2Encoded>`. Task 5 (engine) consumes this directly.

- [ ] **Step 1: Write the failing test**

```rust
// crates/xberg-gliner/src/v2_preprocess.rs
use crate::v2_splitter::V2Splitter;
use crate::v2_tokenizer::{PretokenizedEncoding, PretokenizingTokenizer};
use crate::{GlinerError, Result, Token};

const SCHEMA_TOKEN_P: &str = "[P]";
const SCHEMA_TOKEN_E: &str = "[E]";
const SEP_TEXT_TOKEN: &str = "[SEP_TEXT]";

pub(crate) struct V2Encoded {
    pub(crate) input_ids: Vec<i64>,
    pub(crate) text_positions: Vec<i64>,
    pub(crate) schema_positions: Vec<i64>,
    pub(crate) words: Vec<Token>,
}

/// Build the GLiNER2 schema-prompt token sequence: `( [P] entities ( [E] label1 [E] label2 ... ) )`.
/// Multi-word labels expand to one schema token per whitespace-separated word, matching
/// the upstream `fastino/gliner2` reference preprocessing.
fn build_schema_tokens(labels: &[String]) -> Vec<String> {
    let mut schema = vec![
        "(".to_string(),
        SCHEMA_TOKEN_P.to_string(),
        "entities".to_string(),
        "(".to_string(),
    ];
    for label in labels {
        schema.push(SCHEMA_TOKEN_E.to_string());
        for part in label.split_whitespace() {
            schema.push(part.to_string());
        }
    }
    schema.push(")".to_string());
    schema.push(")".to_string());
    schema
}

pub(crate) fn encode_v2(
    text: &str,
    labels: &[String],
    tokenizer: &impl PretokenizingTokenizer,
    splitter: &V2Splitter,
) -> Result<V2Encoded> {
    let schema_tokens = build_schema_tokens(labels);
    let words = splitter.split(text);
    let num_schema_words = schema_tokens.len() + 1; // +1 for [SEP_TEXT]

    let mut full_sequence: Vec<&str> = schema_tokens.iter().map(String::as_str).collect();
    full_sequence.push(SEP_TEXT_TOKEN);
    full_sequence.extend(words.iter().map(Token::text));

    let encoding: PretokenizedEncoding = tokenizer.encode_pretokenized(full_sequence)?;

    let mut text_positions = Vec::with_capacity(words.len());
    for word_index in 0..words.len() {
        let full_word_index = (num_schema_words + word_index) as u32;
        let position = encoding
            .word_ids
            .iter()
            .position(|word_id| *word_id == Some(full_word_index))
            .ok_or_else(|| {
                GlinerError::Tokenizer(format!(
                    "GLiNER2 tokenizer dropped text word {word_index} during pre-tokenized encoding"
                ))
            })?;
        text_positions.push(position as i64);
    }

    let mut schema_positions = Vec::new();
    for (index, token) in schema_tokens.iter().enumerate() {
        if token == SCHEMA_TOKEN_P || token == SCHEMA_TOKEN_E {
            let full_word_index = index as u32;
            let position = encoding
                .word_ids
                .iter()
                .position(|word_id| *word_id == Some(full_word_index))
                .ok_or_else(|| {
                    GlinerError::Tokenizer(format!(
                        "GLiNER2 tokenizer dropped schema marker '{token}' at schema word {index} during encoding"
                    ))
                })?;
            schema_positions.push(position as i64);
        }
    }

    Ok(V2Encoded {
        input_ids: encoding.ids,
        text_positions,
        schema_positions,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One output token per input word, in order — `word_ids[i] == Some(i)`.
    /// Makes position assertions trivial: every schema/text position should
    /// equal the corresponding word's index in `full_sequence`.
    struct FakeTokenizer;

    impl PretokenizingTokenizer for FakeTokenizer {
        fn encode_pretokenized(&self, words: Vec<&str>) -> Result<PretokenizedEncoding> {
            Ok(PretokenizedEncoding {
                ids: (0..words.len() as i64).collect(),
                word_ids: (0..words.len() as u32).map(Some).collect(),
            })
        }
    }

    #[test]
    fn builds_schema_tokens_with_one_entry_per_label_word() {
        let labels = vec!["person".to_string(), "company name".to_string()];
        let schema = build_schema_tokens(&labels);
        assert_eq!(
            schema,
            vec!["(", "[P]", "entities", "(", "[E]", "person", "[E]", "company", "name", ")", ")"]
        );
    }

    #[test]
    fn computes_text_and_schema_positions() {
        let labels = vec!["person".to_string(), "city".to_string()];
        // schema_tokens = ["(", "[P]", "entities", "(", "[E]", "person", "[E]", "city", ")", ")"]
        // len = 10, num_schema_words = 11 (+1 for [SEP_TEXT])
        let splitter = V2Splitter::new().expect("valid regex");
        let encoded = encode_v2("Ada lives", &labels, &FakeTokenizer, &splitter).expect("encoded");

        assert_eq!(encoded.words.len(), 2);
        // text words start at full_sequence index 11 and 12
        assert_eq!(encoded.text_positions, vec![11, 12]);
        // [P] is schema word 1, [E] tokens are schema words 4 and 6
        assert_eq!(encoded.schema_positions, vec![1, 4, 6]);
        assert_eq!(encoded.input_ids.len(), 13); // 10 schema + 1 sep + 2 words
    }

    #[test]
    fn errors_when_tokenizer_drops_a_required_word() {
        struct DroppingTokenizer;
        impl PretokenizingTokenizer for DroppingTokenizer {
            fn encode_pretokenized(&self, _words: Vec<&str>) -> Result<PretokenizedEncoding> {
                Ok(PretokenizedEncoding {
                    ids: vec![1, 2, 3],
                    word_ids: vec![Some(0), Some(1), Some(2)],
                })
            }
        }

        let splitter = V2Splitter::new().expect("valid regex");
        let labels = vec!["person".to_string()];
        let result = encode_v2("Ada lives here", &labels, &DroppingTokenizer, &splitter);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p xberg-gliner --lib v2_preprocess`
Expected: 3 tests pass (`builds_schema_tokens_with_one_entry_per_label_word`, `computes_text_and_schema_positions`, `errors_when_tokenizer_drops_a_required_word`).

If `computes_text_and_schema_positions` fails on the exact numbers, recompute by hand from `build_schema_tokens`'s output for `["person", "city"]` before changing the assertion — the schema token list is deterministic and the test comment shows the expected list.

- [ ] **Step 3: Register the module**

In `crates/xberg-gliner/src/lib.rs`:

```rust
mod v2_preprocess;
mod v2_splitter;
mod v2_tokenizer;
```

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/v2_preprocess.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add GLiNER2 schema-prompt preprocessing"
```

---

### Task 4: GLiNER2 span_idx tensor builder

**Files:**
- Create: `crates/xberg-gliner/src/v2_tensor.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add `mod v2_tensor;`)

**Interfaces:**
- Consumes: `crate::config::MAX_SPANS_PER_SEQUENCE`, `crate::{GlinerError, Result}`.
- Produces: `pub(crate) fn build_span_idx(num_words: usize, max_width: usize) -> Result<ndarray::Array3<i64>>`. Task 5 (engine) consumes this.

GLiNER2's `span_idx` layout is **dense** (always `num_words * max_width` entries, padded with `(0, 0)`) — unlike GLiNER1's `make_span_tensors` in `tensor.rs`, which truncates per-row and relies on a separate `span_mask` tensor that GLiNER2 doesn't have. This cannot reuse `make_span_tensors`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/xberg-gliner/src/v2_tensor.rs
use ndarray::Array3;

use crate::config::MAX_SPANS_PER_SEQUENCE;
use crate::{GlinerError, Result};

/// Build the dense `span_idx` tensor GLiNER2 expects: every `(start, start+width-1)`
/// pair for `width` in `1..=max_width`, padded with `(0, 0)` once `start+width`
/// exceeds `num_words`. GLiNER2 has no `span_mask` input — out-of-range spans are
/// filtered during decode instead (by checking `end >= num_words`).
pub(crate) fn build_span_idx(num_words: usize, max_width: usize) -> Result<Array3<i64>> {
    let num_spans = num_words.checked_mul(max_width).ok_or_else(|| {
        GlinerError::InvalidInput(format!(
            "span tensor size overflow for {num_words} words and width {max_width}"
        ))
    })?;
    if num_spans > MAX_SPANS_PER_SEQUENCE {
        return Err(GlinerError::InvalidInput(format!(
            "span count must be at most {MAX_SPANS_PER_SEQUENCE}, got {num_spans}"
        )));
    }

    let mut span_idx = Array3::<i64>::zeros((1, num_spans.max(1), 2));
    for start in 0..num_words {
        for width in 1..=max_width {
            let dimension = start * max_width + (width - 1);
            let end = start + width;
            if end <= num_words {
                span_idx[[0, dimension, 0]] = start as i64;
                span_idx[[0, dimension, 1]] = (end - 1) as i64;
            }
        }
    }
    Ok(span_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_dense_span_pairs_with_zero_padding() {
        // 2 words, max_width 2 -> spans: (0,0) (0,1) (1,1) (0,0)-padding
        let span_idx = build_span_idx(2, 2).expect("span idx");
        assert_eq!(span_idx.shape(), &[1, 4, 2]);
        assert_eq!((span_idx[[0, 0, 0]], span_idx[[0, 0, 1]]), (0, 0)); // start=0 width=1
        assert_eq!((span_idx[[0, 1, 0]], span_idx[[0, 1, 1]]), (0, 1)); // start=0 width=2
        assert_eq!((span_idx[[0, 2, 0]], span_idx[[0, 2, 1]]), (1, 1)); // start=1 width=1
        assert_eq!((span_idx[[0, 3, 0]], span_idx[[0, 3, 1]]), (0, 0)); // start=1 width=2 -> out of range, padded
    }

    #[test]
    fn rejects_span_count_overflow() {
        assert!(build_span_idx(usize::MAX, 2).is_err());
    }

    #[test]
    fn rejects_span_count_above_limit() {
        assert!(build_span_idx(MAX_SPANS_PER_SEQUENCE + 1, 1).is_err());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p xberg-gliner --lib v2_tensor`
Expected: 3 tests pass.

- [ ] **Step 3: Register the module**

In `crates/xberg-gliner/src/lib.rs`:

```rust
mod v2_preprocess;
mod v2_splitter;
mod v2_tensor;
mod v2_tokenizer;
```

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/v2_tensor.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add GLiNER2 dense span_idx tensor builder"
```

---

### Task 5: GLiNER2 session schema and decode

**Files:**
- Create: `crates/xberg-gliner/src/v2_session.rs`
- Create: `crates/xberg-gliner/src/v2_decode.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add both modules)

**Interfaces:**
- Consumes: `crate::session::{build_session, validate_schema_names}` (both already `pub(crate)`, reused as-is — `build_session` has no v1-specific logic), `crate::decode::{Span, SpanOutput, greedy_search}` (need to confirm/widen visibility — see Step 1), `crate::{GlinerError, Result, Token}`.
- Produces: `pub const INPUT_NAMES_V2: [&str; 5]`, `pub const OUTPUT_NAMES_V2: [&str; 1]`, tensor name constants, `pub(crate) fn validate_session_schema_v2(session: &Session) -> Result<()>`, `pub(crate) fn decode_span_scores(...) -> Result<SpanOutput>`. Task 6 (engine) consumes both.

- [ ] **Step 1: Widen `decode.rs` visibility for reuse**

`Span::new`, `SpanOutput::new`, and `greedy_search` are currently `pub(crate)` in `crates/xberg-gliner/src/decode.rs` — already visible crate-wide, so no change is needed there. Confirm by reading the file: `grep -n "pub(crate) fn new\|pub(crate) fn greedy_search" crates/xberg-gliner/src/decode.rs` should show `Span::new` (line ~17), `SpanOutput::new` (line ~84), and `greedy_search` (line ~225) all already `pub(crate)`. If any of them is bare `fn` (private to `decode.rs` only), widen it to `pub(crate) fn` now — do not change `pub fn` items.

- [ ] **Step 2: Write `v2_session.rs`**

```rust
// crates/xberg-gliner/src/v2_session.rs
use ort::session::Session;

use crate::Result;
use crate::session::validate_schema_names;

/// GLiNER2 tensor input names expected by schema-prompt ONNX exports.
pub const INPUT_NAMES_V2: [&str; 5] = [
    "input_ids",
    "attention_mask",
    "text_positions",
    "schema_positions",
    "span_idx",
];

/// GLiNER2 tensor output names expected by schema-prompt ONNX exports.
pub const OUTPUT_NAMES_V2: [&str; 1] = ["span_scores"];

pub(crate) const TENSOR_V2_INPUT_IDS: &str = "input_ids";
pub(crate) const TENSOR_V2_ATTENTION_MASK: &str = "attention_mask";
pub(crate) const TENSOR_V2_TEXT_POSITIONS: &str = "text_positions";
pub(crate) const TENSOR_V2_SCHEMA_POSITIONS: &str = "schema_positions";
pub(crate) const TENSOR_V2_SPAN_IDX: &str = "span_idx";
pub(crate) const TENSOR_V2_SPAN_SCORES: &str = "span_scores";

pub(crate) fn validate_session_schema_v2(session: &Session) -> Result<()> {
    let inputs = session
        .inputs()
        .iter()
        .map(|input| input.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("input", &INPUT_NAMES_V2, &inputs)?;

    let outputs = session
        .outputs()
        .iter()
        .map(|output| output.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("output", &OUTPUT_NAMES_V2, &outputs)
}
```

- [ ] **Step 3: Write `v2_decode.rs` with its test**

```rust
// crates/xberg-gliner/src/v2_decode.rs
use ndarray::{ArrayViewD, Ix4};

use crate::decode::{Span, SpanOutput, greedy_search};
use crate::{GlinerError, Result, Token};

/// Decode GLiNER2's `span_scores` output `(1, num_labels, num_words, max_width)`
/// into entity spans. Unlike GLiNER1's `logits`, `span_scores` values are already
/// post-sigmoid probabilities — do not apply `sigmoid()` here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_span_scores(
    span_scores: ArrayViewD<'_, f32>,
    text: &str,
    words: &[Token],
    labels: &[String],
    threshold: f32,
    max_width: usize,
    flat_ner: bool,
    dup_label: bool,
    multi_label: bool,
) -> Result<SpanOutput> {
    let num_words = words.len();
    let expected_shape = vec![1, labels.len(), num_words, max_width];
    let actual_shape = span_scores.shape().to_vec();
    if actual_shape != expected_shape {
        return Err(GlinerError::UnexpectedLogitsShape {
            expected: expected_shape,
            actual: actual_shape,
        });
    }

    let span_scores = span_scores
        .into_dimensionality::<Ix4>()
        .map_err(|_| GlinerError::UnexpectedLogitsShape {
            expected: expected_shape.clone(),
            actual: actual_shape,
        })?;

    let mut spans = Vec::new();
    for (label_index, label) in labels.iter().enumerate() {
        for start in 0..num_words {
            for width_index in 0..max_width {
                let end = start + width_index;
                if end >= num_words {
                    continue;
                }
                let probability = span_scores[[0, label_index, start, width_index]];
                if probability >= threshold {
                    let start_token = &words[start];
                    let end_token = &words[end];
                    let source = text
                        .get(start_token.start()..end_token.end())
                        .ok_or(GlinerError::InvalidOffsets {
                            start: start_token.start(),
                            end: end_token.end(),
                        })?
                        .to_string();
                    spans.push(Span::new(
                        0,
                        start_token.start(),
                        end_token.end(),
                        source,
                        label.clone(),
                        probability,
                    )?);
                }
            }
        }
    }

    spans.sort_unstable_by_key(Span::offsets);
    let resolved = greedy_search(&spans, flat_ner, dup_label, multi_label);

    Ok(SpanOutput::new(vec![text.to_string()], labels.to_vec(), vec![resolved]))
}

#[cfg(test)]
mod tests {
    use ndarray::Array4;

    use super::*;

    #[test]
    fn decodes_spans_above_threshold_without_sigmoid() {
        let text = "Ada lives";
        let words = vec![Token::new(0, 3, "Ada"), Token::new(4, 9, "lives")];
        let labels = vec!["person".to_string()];
        // shape (1, 1 label, 2 words, max_width 2)
        let mut scores = Array4::<f32>::zeros((1, 1, 2, 2));
        scores[[0, 0, 0, 0]] = 0.9; // "Ada" alone, score already a probability
        let output = decode_span_scores(scores.into_dyn().view(), text, &words, &labels, 0.5, 2, true, false, false)
            .expect("decoded");
        assert_eq!(output.spans[0].len(), 1);
        assert_eq!(output.spans[0][0].text(), "Ada");
        assert_eq!(output.spans[0][0].probability(), 0.9);
    }

    #[test]
    fn rejects_unexpected_shape() {
        let text = "Ada";
        let words = vec![Token::new(0, 3, "Ada")];
        let labels = vec!["person".to_string()];
        let scores = Array4::<f32>::zeros((1, 1, 1, 1));
        // wrong max_width argument (2, but tensor only has width 1)
        let result = decode_span_scores(scores.into_dyn().view(), text, &words, &labels, 0.5, 2, true, false, false);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 4: Register both modules**

In `crates/xberg-gliner/src/lib.rs`:

```rust
mod v2_decode;
mod v2_preprocess;
mod v2_session;
mod v2_splitter;
mod v2_tensor;
mod v2_tokenizer;
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p xberg-gliner --lib v2_decode`
Expected: 2 tests pass (`decodes_spans_above_threshold_without_sigmoid`, `rejects_unexpected_shape`).

- [ ] **Step 6: Commit**

```bash
git add crates/xberg-gliner/src/v2_session.rs crates/xberg-gliner/src/v2_decode.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add GLiNER2 session schema and span_scores decode"
```

---

### Task 6: GLiNER2 engine (`Gliner2`)

**Files:**
- Create: `crates/xberg-gliner/src/v2_engine.rs`
- Modify: `crates/xberg-gliner/src/lib.rs` (add `mod v2_engine;` and re-export `Gliner2`, `INPUT_NAMES_V2`, `OUTPUT_NAMES_V2`)

**Interfaces:**
- Consumes: everything from Tasks 1–5 (`V2Tokenizer`, `V2Splitter`, `encode_v2`, `build_span_idx`, `validate_session_schema_v2`, `decode_span_scores`), plus `crate::session::build_session`, `crate::{GlinerError, Parameters, Result, RuntimeConfig, SpanOutput, TextInput}`.
- Produces: `pub struct Gliner2` with `pub fn new(...)`, `pub fn with_runtime(...)`, `pub fn inference(&self, input: TextInput) -> Result<SpanOutput>` — same public shape as `Gliner` (Task 7 in `xberg/src/text/ner/gline.rs` consumes this).

- [ ] **Step 1: Write the file**

```rust
// crates/xberg-gliner/src/v2_engine.rs
use std::path::Path;

use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::Tensor;
use parking_lot::Mutex;

use crate::session::build_session;
use crate::v2_decode::decode_span_scores;
use crate::v2_preprocess::encode_v2;
use crate::v2_session::{
    TENSOR_V2_ATTENTION_MASK, TENSOR_V2_INPUT_IDS, TENSOR_V2_SCHEMA_POSITIONS, TENSOR_V2_SPAN_IDX,
    TENSOR_V2_SPAN_SCORES, TENSOR_V2_TEXT_POSITIONS, validate_session_schema_v2,
};
use crate::v2_splitter::V2Splitter;
use crate::v2_tensor::build_span_idx;
use crate::v2_tokenizer::V2Tokenizer;
use crate::{GlinerError, Parameters, Result, RuntimeConfig, SpanOutput, TextInput};

/// GLiNER2 schema-prompt inference engine.
///
/// Unlike [`crate::Gliner`] (span-mode, batched), GLiNER2's published ONNX
/// exports hardcode a batch dimension of 1 — `inference` accepts exactly one
/// text per call.
pub struct Gliner2 {
    params: Parameters,
    splitter: V2Splitter,
    tokenizer: V2Tokenizer,
    session: Mutex<Session>,
}

impl Gliner2 {
    /// Load a GLiNER2 schema-prompt ONNX model and tokenizer from local files.
    pub fn new<PT, PM>(params: Parameters, tokenizer_path: PT, model_path: PM) -> Result<Self>
    where
        PT: AsRef<Path>,
        PM: AsRef<Path>,
    {
        Self::with_runtime(params, RuntimeConfig::default(), tokenizer_path, model_path)
    }

    /// Load a GLiNER2 schema-prompt ONNX model and tokenizer from local files with runtime options.
    pub fn with_runtime<PT, PM>(
        params: Parameters,
        runtime: RuntimeConfig,
        tokenizer_path: PT,
        model_path: PM,
    ) -> Result<Self>
    where
        PT: AsRef<Path>,
        PM: AsRef<Path>,
    {
        params.validate()?;
        let tokenizer = V2Tokenizer::from_file(tokenizer_path)?;
        let session = build_session(model_path, &runtime)?;
        validate_session_schema_v2(&session)?;
        Ok(Self {
            params,
            splitter: V2Splitter::new()?,
            tokenizer,
            session: Mutex::new(session),
        })
    }

    /// Run schema-prompt inference. `input` must contain exactly one text.
    pub fn inference(&self, input: TextInput) -> Result<SpanOutput> {
        if input.texts.len() != 1 {
            return Err(GlinerError::InvalidInput(format!(
                "Gliner2::inference accepts exactly one text per call, got {}",
                input.texts.len()
            )));
        }
        let text = input.texts[0].clone();
        let labels = input.entities.clone();

        let encoded = encode_v2(&text, &labels, &self.tokenizer, &self.splitter)?;
        let seq_len = encoded.input_ids.len();
        let num_words = encoded.words.len();

        let input_ids = Array2::from_shape_vec((1, seq_len), encoded.input_ids)
            .map_err(|error| GlinerError::InvalidInput(format!("failed to build GLiNER2 input_ids tensor: {error}")))?;
        let attention_mask = Array2::from_shape_vec((1, seq_len), vec![1i64; seq_len]).map_err(|error| {
            GlinerError::InvalidInput(format!("failed to build GLiNER2 attention_mask tensor: {error}"))
        })?;
        let text_positions = Array1::from_vec(encoded.text_positions);
        let schema_positions = Array1::from_vec(encoded.schema_positions);
        let span_idx = build_span_idx(num_words, self.params.max_width)?;

        let input_ids = Tensor::from_array(input_ids)?;
        let attention_mask = Tensor::from_array(attention_mask)?;
        let text_positions = Tensor::from_array(text_positions)?;
        let schema_positions = Tensor::from_array(schema_positions)?;
        let span_idx = Tensor::from_array(span_idx)?;

        let span_scores = {
            let mut session = self.session.lock();
            let outputs = session.run(ort::inputs![
                TENSOR_V2_INPUT_IDS => input_ids,
                TENSOR_V2_ATTENTION_MASK => attention_mask,
                TENSOR_V2_TEXT_POSITIONS => text_positions,
                TENSOR_V2_SCHEMA_POSITIONS => schema_positions,
                TENSOR_V2_SPAN_IDX => span_idx,
            ])?;
            outputs
                .get(TENSOR_V2_SPAN_SCORES)
                .ok_or(GlinerError::MissingOutput(TENSOR_V2_SPAN_SCORES))?
                .try_extract_array::<f32>()?
                .to_owned()
        };

        decode_span_scores(
            span_scores.view(),
            &text,
            &encoded.words,
            &labels,
            self.params.threshold,
            self.params.max_width,
            self.params.flat_ner,
            self.params.dup_label,
            self.params.multi_label,
        )
    }
}
```

- [ ] **Step 2: Register the module and re-export the public API**

In `crates/xberg-gliner/src/lib.rs`:

```rust
mod config;
mod decode;
mod engine;
mod error;
mod input;
mod preprocess;
mod session;
mod splitter;
mod tensor;
mod tokenizer;
mod v2_decode;
mod v2_engine;
mod v2_preprocess;
mod v2_session;
mod v2_splitter;
mod v2_tensor;
mod v2_tokenizer;

pub use config::{Parameters, RuntimeConfig};
pub use decode::{Span, SpanOutput};
pub use engine::Gliner;
pub use error::{GlinerError, Result};
pub use input::{TextInput, Token};
pub use session::{INPUT_NAMES, OUTPUT_NAMES};
pub use v2_engine::Gliner2;
pub use v2_session::{INPUT_NAMES_V2, OUTPUT_NAMES_V2};

pub(crate) use decode::EntityContext;
pub(crate) use preprocess::EncodedInput;

#[cfg(test)]
mod tests;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p xberg-gliner`
Expected: clean compile, no warnings.

Run: `cargo test -p xberg-gliner --lib`
Expected: all existing v1 tests plus all new v2 tests pass (the crate has no `#[ignore]`d tests at this layer yet — that's added in Task 9 against `xberg`, not here).

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner/src/v2_engine.rs crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): add Gliner2 schema-prompt inference engine"
```

---

### Task 7: `GlinerArchitecture` config type

**Files:**
- Modify: `crates/xberg/src/core/config/ner.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub enum GlinerArchitecture { Gliner1, Gliner2 }` (with `Default` = `Gliner1`), `NerConfig.hf_architecture: Option<GlinerArchitecture>`. Task 8 consumes both.

- [ ] **Step 1: Add the enum**

In `crates/xberg/src/core/config/ner.rs`, after the existing `NerBackendKind` enum (end of file):

```rust
/// GLiNER ONNX architecture family. Determines which tensor I/O contract and
/// preprocessing pipeline xberg uses — only relevant when `hf_repo` is set,
/// since the pinned `xberg-io/gliner-models` catalog is always [`GlinerArchitecture::Gliner1`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum GlinerArchitecture {
    /// Span-mode GLiNER (`gliner-community`/`urchade` lineage, the pinned
    /// `xberg-io/gliner-models` catalog, and most GLiNER fine-tunes including
    /// the `knowledgator/gliner-pii-*` family).
    #[default]
    Gliner1,
    /// Schema-prompt GLiNER2 (`fastino/gliner2` lineage). Requires an ONNX export
    /// with `[P]`/`[E]`/`[SEP_TEXT]` special tokens in its tokenizer and the
    /// `input_ids`/`attention_mask`/`text_positions`/`schema_positions`/`span_idx`
    /// tensor contract. Most GLiNER2 model cards ship safetensors only and have
    /// no ONNX export — check the repo's file list for a `.onnx` file before
    /// pointing `hf_repo` at it.
    Gliner2,
}
```

- [ ] **Step 2: Add the field to `NerConfig`**

Find the `hf_tokenizer_file` field (added in the prior session turn) and add `hf_architecture` immediately after it, before `llm`:

```rust
    /// Path to the tokenizer file within `hf_repo` (e.g. `"tokenizer.json"`).
    /// Required when `hf_repo` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.1.0"))]
    pub hf_tokenizer_file: Option<String>,
    /// GLiNER architecture family for `hf_repo`. Ignored when `hf_repo` is unset.
    /// Defaults to [`GlinerArchitecture::Gliner1`] when `hf_repo` is set and this is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.2.0"))]
    pub hf_architecture: Option<GlinerArchitecture>,
    /// Optional LLM configuration — only used by [`NerBackendKind::Llm`]. Token usage
    /// for LLM backends is recorded in `ExtractedDocument::llm_usage`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<super::llm::LlmConfig>,
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p xberg --features ner-onnx,ner-llm`
Expected: clean compile. `GlinerArchitecture` and `hf_architecture` are unused warnings at this point — expected, resolved in Task 8.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg/src/core/config/ner.rs
git commit -m "feat(ner): add GlinerArchitecture config type"
```

---

### Task 8: Wire architecture through `CustomGlinerSource` and `GlineBackend`

**Files:**
- Modify: `crates/xberg/src/text/ner/gline.rs`
- Modify: `crates/xberg/src/plugins/processor/builtin/ner.rs`
- Modify: `crates/xberg/src/text/redaction/engine.rs`

**Interfaces:**
- Consumes: `GlinerArchitecture` (Task 7), `xberg_gliner::Gliner2` (Task 6).
- Produces: `CustomGlinerSource.architecture: GlinerArchitecture`, `custom_source_from_parts(repo, model_file, tokenizer_file, architecture)` (4 args now), `GlineBackend` dispatches to either engine internally. No new public API beyond the new field — `ner.rs`'s `make_backend()` and `redaction/engine.rs`'s `make_ner_backend()` are the only callers and are updated in this same task.

- [ ] **Step 1: Update the `xberg_gliner` import**

In `crates/xberg/src/text/ner/gline.rs`, change:

```rust
use xberg_gliner::{Gliner, Parameters, RuntimeConfig, TextInput};
```

to:

```rust
use xberg_gliner::{Gliner, Gliner2, Parameters, RuntimeConfig, TextInput};
```

- [ ] **Step 2: Add `architecture` to `CustomGlinerSource` and thread it through `custom_source_from_parts`**

Replace the existing struct and function:

```rust
/// Caller-supplied override pointing GLiNER at an arbitrary Hugging Face repo
/// instead of the pinned `xberg-io/gliner-models` catalog.
///
/// Files downloaded from a custom repo are **not** checksum-verified — the
/// catalog's `checksums.sha256` only covers the pinned models xberg publishes.
/// Callers choosing a custom repo are trusting that source directly.
#[derive(Debug, Clone)]
pub struct CustomGlinerSource {
    /// Hugging Face repo id, e.g. `"gliner-community/gliner_small-v2.5"`.
    pub repo: String,
    /// Path to the ONNX model file within `repo`.
    pub model_file: String,
    /// Path to the tokenizer file within `repo`.
    pub tokenizer_file: String,
    /// Which GLiNER tensor I/O contract `model_file` uses.
    pub architecture: crate::core::config::ner::GlinerArchitecture,
}

/// Build a [`CustomGlinerSource`] from optional config fields.
///
/// Returns `Ok(None)` when `repo`/`model_file`/`tokenizer_file` are all unset
/// (use the pinned catalog), `Ok(Some(_))` when all three are set, and `Err`
/// when only some are set. `architecture` is independent of that all-or-nothing
/// rule — `None` defaults to [`crate::core::config::ner::GlinerArchitecture::Gliner1`].
pub fn custom_source_from_parts(
    repo: Option<&str>,
    model_file: Option<&str>,
    tokenizer_file: Option<&str>,
    architecture: Option<crate::core::config::ner::GlinerArchitecture>,
) -> Result<Option<CustomGlinerSource>> {
    match (repo, model_file, tokenizer_file) {
        (None, None, None) => Ok(None),
        (Some(repo), Some(model_file), Some(tokenizer_file)) => Ok(Some(CustomGlinerSource {
            repo: repo.to_string(),
            model_file: model_file.to_string(),
            tokenizer_file: tokenizer_file.to_string(),
            architecture: architecture.unwrap_or_default(),
        })),
        _ => Err(crate::XbergError::validation(
            "NerConfig.hf_repo, hf_model_file, and hf_tokenizer_file must all be set together, or all left unset",
        )),
    }
}
```

- [ ] **Step 3: Fold architecture into the custom cache key**

Replace `custom_cache_key`:

```rust
/// Content-derived cache directory name for a custom GLiNER source, so
/// distinct `(repo, model_file, tokenizer_file, architecture)` tuples never
/// collide and arbitrary caller-supplied strings never escape the cache directory.
fn custom_cache_key(
    repo: &str,
    model_file: &str,
    tokenizer_file: &str,
    architecture: crate::core::config::ner::GlinerArchitecture,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(repo.as_bytes());
    hasher.update(b"\0");
    hasher.update(model_file.as_bytes());
    hasher.update(b"\0");
    hasher.update(tokenizer_file.as_bytes());
    hasher.update(b"\0");
    hasher.update(architecture_tag(architecture).as_bytes());
    hex::encode(hasher.finalize())
}

fn architecture_tag(architecture: crate::core::config::ner::GlinerArchitecture) -> &'static str {
    use crate::core::config::ner::GlinerArchitecture;
    match architecture {
        GlinerArchitecture::Gliner1 => "gliner1",
        GlinerArchitecture::Gliner2 => "gliner2",
    }
}
```

Update the two call sites:

In `ensure_custom_model`, change:

```rust
    let cache_key = custom_cache_key(repo, model_file, tokenizer_file);
```

to:

```rust
    let cache_key = custom_cache_key(repo, model_file, tokenizer_file, source.architecture);
```

In `backend_cache_key`, change:

```rust
        Some(source) => format!(
            "custom:{}",
            custom_cache_key(source.repo.trim(), source.model_file.trim(), source.tokenizer_file.trim())
        ),
```

to:

```rust
        Some(source) => format!(
            "custom:{}",
            custom_cache_key(
                source.repo.trim(),
                source.model_file.trim(),
                source.tokenizer_file.trim(),
                source.architecture,
            )
        ),
```

- [ ] **Step 4: Make `GlineBackend` hold either engine**

Replace the `GlineBackend` struct and its `impl` block:

```rust
enum GlinerEngine {
    V1(Gliner),
    V2(Gliner2),
}

impl GlinerEngine {
    fn inference(&self, input: TextInput) -> xberg_gliner::Result<xberg_gliner::SpanOutput> {
        match self {
            Self::V1(engine) => engine.inference(input),
            Self::V2(engine) => engine.inference(input),
        }
    }
}

/// `xberg-gliner` ONNX backend wrapper.
///
/// Holds an initialised GLiNER (v1 span-mode or v2 schema-prompt) model.
/// Inference is synchronous and internally serialized around the underlying
/// ONNX Runtime session.
pub struct GlineBackend {
    /// xberg GLiNER model alias or catalog id used to load this model.
    pub repo_id: String,
    /// Local path to the cached ONNX model file.
    pub model_path: PathBuf,
    /// Local path to the cached tokenizer file.
    pub tokenizer_path: PathBuf,
    model: Arc<GlinerEngine>,
}

impl GlineBackend {
    /// Build a backend for `model_name`, or the default model when `None`.
    ///
    /// Downloads the ONNX weights and tokenizer from `xberg-io/gliner-models`
    /// on first use. After this returns, inference is available without
    /// further network I/O.
    pub fn new(model_name: Option<&str>) -> Result<Self> {
        let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
        Self::new_with_thread_budget(model_name, None, thread_budget)
    }

    /// Build a backend from a caller-supplied Hugging Face repo, bypassing the
    /// pinned catalog. See [`CustomGlinerSource`] for the checksum caveat.
    pub fn new_with_custom_source(source: &CustomGlinerSource) -> Result<Self> {
        let thread_budget = crate::core::config::concurrency::resolve_thread_budget(None);
        Self::new_with_thread_budget(None, Some(source), thread_budget)
    }

    fn new_with_thread_budget(
        model_name: Option<&str>,
        custom_source: Option<&CustomGlinerSource>,
        thread_budget: usize,
    ) -> Result<Self> {
        let files = match custom_source {
            Some(source) => ensure_custom_model(source, None)?,
            None => {
                let requested = requested_model_name(model_name)?;
                ensure_model(&requested, None)?
            }
        };
        let architecture = custom_source
            .map(|source| source.architecture)
            .unwrap_or_default();
        let engine = match architecture {
            crate::core::config::ner::GlinerArchitecture::Gliner1 => GlinerEngine::V1(
                Gliner::with_runtime(
                    Parameters::default(),
                    RuntimeConfig::default().with_intra_threads(thread_budget),
                    &files.tokenizer_path,
                    &files.model_path,
                )
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to initialise GLiNER model '{}': {error}", files.id),
                    plugin_name: "ner-gliner".to_string(),
                })?,
            ),
            crate::core::config::ner::GlinerArchitecture::Gliner2 => GlinerEngine::V2(
                Gliner2::with_runtime(
                    Parameters::default(),
                    RuntimeConfig::default().with_intra_threads(thread_budget),
                    &files.tokenizer_path,
                    &files.model_path,
                )
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to initialise GLiNER2 model '{}': {error}", files.id),
                    plugin_name: "ner-gliner".to_string(),
                })?,
            ),
        };
        Ok(Self {
            repo_id: files.id,
            model_path: files.model_path,
            tokenizer_path: files.tokenizer_path,
            model: Arc::new(engine),
        })
    }
}
```

- [ ] **Step 5: Update `detect_labels`'s inference call**

Find `detect_labels` and change `backend.inference(input)` (the line reads `let output = backend.inference(input)...`) — no change needed there, since `GlinerEngine::inference` has the exact same signature as `Gliner::inference`/`Gliner2::inference` (`fn(&self, TextInput) -> xberg_gliner::Result<SpanOutput>`), and `backend: Arc<GlinerEngine>` is already what `self.model` resolves to via `Arc::clone(&self.model)`. No code changes required in this method — the existing call site already works because `GlinerEngine::inference` (added in Step 4) matches the shape it expects.

- [ ] **Step 6: Update the two call sites outside `gline.rs`**

In `crates/xberg/src/plugins/processor/builtin/ner.rs`, find `make_backend()`'s `Onnx` arm and change:

```rust
                let custom_source = crate::text::ner::gline::custom_source_from_parts(
                    config.hf_repo.as_deref(),
                    config.hf_model_file.as_deref(),
                    config.hf_tokenizer_file.as_deref(),
                )?;
```

to:

```rust
                let custom_source = crate::text::ner::gline::custom_source_from_parts(
                    config.hf_repo.as_deref(),
                    config.hf_model_file.as_deref(),
                    config.hf_tokenizer_file.as_deref(),
                    config.hf_architecture,
                )?;
```

Apply the identical change in `crates/xberg/src/text/redaction/engine.rs`'s `make_ner_backend()` (same `Onnx` arm pattern).

- [ ] **Step 7: Verify it compiles and existing tests still pass**

Run: `cargo check -p xberg --features ner-onnx,ner-llm`
Expected: clean compile.

Run: `cargo test -p xberg --features ner-onnx,ner-llm --lib ner::gline -- --skip smoke_test`
Expected: all tests pass, including the `backend_cache_key`/`custom_cache_key`-adjacent tests fixed earlier this session.

- [ ] **Step 8: Commit**

```bash
git add crates/xberg/src/text/ner/gline.rs crates/xberg/src/plugins/processor/builtin/ner.rs crates/xberg/src/text/redaction/engine.rs
git commit -m "feat(ner): dispatch GLiNER1/GLiNER2 engines by architecture"
```

---

### Task 9: Ignored smoke test against a real GLiNER2 ONNX export

**Files:**
- Modify: `crates/xberg/src/text/ner/gline.rs`

**Interfaces:**
- Consumes: `GlineBackend::new_with_custom_source`, `CustomGlinerSource`.
- Produces: nothing new — this is a verification-only task, following the exact pattern of the existing `smoke_test_real_inference` test in the same file.

- [ ] **Step 1: Add the test**

In `crates/xberg/src/text/ner/gline.rs`'s `#[cfg(all(test, feature = "ner-onnx"))] mod tests` block, after the existing `smoke_test_real_inference` test:

```rust
    /// Smoke test — downloads a real GLiNER2 ONNX export and runs one inference.
    /// `lion-ai/gliner2-base-v1-onnx` is the only publicly available monolithic
    /// single-file GLiNER2 ONNX export found (most GLiNER2 model cards ship
    /// safetensors only). Excluded from normal CI; run with:
    ///   cargo test -p xberg --features ner-onnx,ner --lib ner::gline -- --ignored gliner2
    #[ignore]
    #[tokio::test]
    async fn smoke_test_gliner2_real_inference() {
        let source = CustomGlinerSource {
            repo: "lion-ai/gliner2-base-v1-onnx".to_string(),
            model_file: "model.onnx".to_string(),
            tokenizer_file: "tokenizer.json".to_string(),
            architecture: crate::core::config::ner::GlinerArchitecture::Gliner2,
        };
        let backend = GlineBackend::new_with_custom_source(&source).expect("GlineBackend::new_with_custom_source failed");
        let entities = backend
            .detect(
                "Steve Jobs founded Apple Inc. in Cupertino, California on April 1, 1976.",
                &[],
            )
            .await
            .expect("detect failed");
        assert!(!entities.is_empty(), "expected at least one entity");
        let texts: Vec<&str> = entities.iter().map(|entity| entity.text.as_str()).collect();
        assert!(
            texts.iter().any(|text| text.eq_ignore_ascii_case("steve jobs") || text.eq_ignore_ascii_case("apple inc")),
            "expected at least one known entity, got: {texts:?}"
        );
    }
```

- [ ] **Step 2: Run it (manually — this hits the network and downloads a real model)**

Run: `cargo test -p xberg --features ner-onnx,ner --lib ner::gline -- --ignored gliner2`
Expected: PASS, with at least one entity matching "steve jobs" or "apple inc" (case-insensitive). If it fails, the most likely causes in order of likelihood:
1. `lion-ai/gliner2-base-v1-onnx`'s tokenizer doesn't expose `[P]`/`[E]`/`[SEP_TEXT]` as single tokens the way assumed — inspect `tokenizer_config.json`'s `added_tokens` for that repo.
2. The model's actual ONNX input/output tensor names differ slightly from what `validate_session_schema_v2` expects — the error message will show the actual names; update `INPUT_NAMES_V2`/`OUTPUT_NAMES_V2` in `v2_session.rs` (Task 5) to match, not the test.
3. `detect`'s default labels (`person, organization, location, date, email` — lowercase, single-word style) might not match this model's expected label format as closely as `["person", "company", "date"]` used in the upstream reference — if entities come back empty rather than erroring, try calling with explicit `categories: &[EntityCategory::Person, EntityCategory::Organization]` instead of `&[]` before concluding the engine itself is broken.

This test is allowed to be flaky/network-dependent like its v1 sibling — do not block the plan on getting it green in CI; it exists for manual verification and as living documentation of how to point xberg at a GLiNER2 model.

- [ ] **Step 3: Commit**

```bash
git add crates/xberg/src/text/ner/gline.rs
git commit -m "test(ner): add ignored GLiNER2 smoke test against lion-ai/gliner2-base-v1-onnx"
```

---

### Task 10: Hand-patch `xberg-node` bindings for `hf_architecture`

**Files:**
- Modify: `crates/xberg-node/src/lib.rs`
- Modify: `crates/xberg-node/index.d.ts`

**Interfaces:**
- Consumes: `GlinerArchitecture` (Task 7).
- Produces: `JsGlinerArchitecture` NAPI enum, `JsNerConfig.hf_architecture: Option<JsGlinerArchitecture>`, wired into both `From` impls. Task 11 (mcp-server) consumes the resulting `NerConfig.hfArchitecture` TypeScript field.

This follows the exact hand-patch pattern already used this session for `hf_repo`/`hf_model_file`/`hf_tokenizer_file` (`task`/`alef` are still unavailable — see Global Constraints).

- [ ] **Step 1: Add the `JsGlinerArchitecture` enum**

In `crates/xberg-node/src/lib.rs`, immediately after the existing `JsNerBackendKind` enum + its `Default` impl (search for `pub enum JsNerBackendKind`):

```rust
/// GLiNER ONNX architecture family.
#[napi(string_enum = "snake_case", js_name = "GlinerArchitecture")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum JsGlinerArchitecture {
    /// Span-mode GLiNER (the pinned catalog and most GLiNER fine-tunes).
    Gliner1,
    /// Schema-prompt GLiNER2 (`fastino/gliner2` lineage).
    Gliner2,
}

#[allow(clippy::derivable_impls)]
impl Default for JsGlinerArchitecture {
    fn default() -> Self {
        Self::Gliner1
    }
}
```

- [ ] **Step 2: Add the field to `JsNerConfig`**

Find `pub hf_tokenizer_file: Option<String>,` inside `struct JsNerConfig` and add immediately after it:

```rust
    #[napi(js_name = "hfArchitecture")]
    #[serde(rename = "hfArchitecture")]
    pub hf_architecture: Option<JsGlinerArchitecture>,
```

- [ ] **Step 3: Wire both `From` impls**

In `impl From<JsNerConfig> for xberg::NerConfig`, find `__result.hf_tokenizer_file = val.hf_tokenizer_file;` and add immediately after it:

```rust
        __result.hf_architecture = val.hf_architecture.map(Into::into);
```

In `impl From<xberg::NerConfig> for JsNerConfig`, find `hf_tokenizer_file: val.hf_tokenizer_file.map(|v| v.to_string()),` and add immediately after it:

```rust
            hf_architecture: val.hf_architecture.map(Into::into),
```

- [ ] **Step 4: Add `From` conversions for the enum itself**

Immediately after `JsGlinerArchitecture`'s `Default` impl (Step 1), or alongside the other `JsNerBackendKind`-to-`xberg::NerBackendKind` conversions further down the file (search for `impl From<JsNerBackendKind> for xberg::NerBackendKind` to find the right neighborhood):

```rust
impl From<JsGlinerArchitecture> for xberg::GlinerArchitecture {
    fn from(val: JsGlinerArchitecture) -> Self {
        match val {
            JsGlinerArchitecture::Gliner1 => Self::Gliner1,
            JsGlinerArchitecture::Gliner2 => Self::Gliner2,
        }
    }
}

impl From<xberg::GlinerArchitecture> for JsGlinerArchitecture {
    fn from(val: xberg::GlinerArchitecture) -> Self {
        match val {
            xberg::GlinerArchitecture::Gliner1 => Self::Gliner1,
            xberg::GlinerArchitecture::Gliner2 => Self::Gliner2,
        }
    }
}
```

If `xberg::GlinerArchitecture` is not directly importable (check whether `xberg::NerConfig`/`xberg::NerBackendKind` are re-exported at the crate root via `pub use` in `crates/xberg/src/lib.rs`, or whether `xberg-node` uses a fully-qualified path like `xberg::core::config::ner::GlinerArchitecture` elsewhere in the file) — match whatever import style the file already uses for `NerBackendKind` exactly; do not introduce a new style.

- [ ] **Step 5: Update `index.d.ts`**

In `crates/xberg-node/index.d.ts`, find the `NerConfig` interface's `hfTokenizerFile?: string` entry (added this session) and add immediately after it:

```typescript
  /**
   * Which GLiNER tensor I/O contract `hfRepo` uses. Ignored when `hfRepo` is unset.
   * Defaults to `gliner1` when `hfRepo` is set and this is omitted.
   */
  hfArchitecture?: GlinerArchitecture
```

Add the `GlinerArchitecture` type declaration near `NerBackendKind`'s declaration (search for `export const enum NerBackendKind` or `export type NerBackendKind`, match its exact style):

```typescript
export const enum GlinerArchitecture {
  Gliner1 = 'gliner1',
  Gliner2 = 'gliner2'
}
```

- [ ] **Step 6: Verify the Rust side compiles**

Run: `cargo check -p xberg-node --features default 2>&1 | tail -60`

This crate has a large default feature set (ner-onnx, ner-llm, etc. — see `crate-structure` context) and may take a while. Expected: clean compile. If there's a mismatch between `napi(string_enum)` casing and what `xberg::GlinerArchitecture`'s `#[serde(rename_all = "snake_case")]` produces (both should be `"gliner1"`/`"gliner2"`), fix the NAPI side to match the Rust core's serde casing — the core type is the source of truth.

- [ ] **Step 7: Commit**

```bash
git add crates/xberg-node/src/lib.rs crates/xberg-node/index.d.ts
git commit -m "feat(node): expose GlinerArchitecture/hfArchitecture in NerConfig bindings"
```

---

### Task 11: Wire `hf_architecture` into the MCP server tools and README

**Files:**
- Modify: `mcp-server/src/tools/intelligence.ts`
- Modify: `mcp-server/src/tools/ingest.ts`
- Modify: `mcp-server/README.md`

**Interfaces:**
- Consumes: `NerConfig.hfArchitecture`/`GlinerArchitecture` (Task 10).
- Produces: nothing new downstream — this is the final, user-facing layer.

This follows the exact pattern already used this session for `hf_repo`/`hf_model_file`/`hf_tokenizer_file` in both files.

- [ ] **Step 1: Update `extract_entities` in `intelligence.ts`**

After the `hf_tokenizer_file` Zod param (added this session, search for `hf_tokenizer_file: z.string().optional()`), add:

```typescript
      hf_architecture: z.enum(["gliner1", "gliner2"]).optional().describe(
        "Only used when hf_repo is set. Which GLiNER tensor contract hf_repo uses. Defaults to 'gliner1'. Most GLiNER2 model cards ship safetensors only (no ONNX export) — confirm an .onnx file exists in hf_repo before setting this to 'gliner2'."
      ),
```

In the handler's parameter destructuring (search for `async ({ input, backend, categories, model, hf_repo, hf_model_file, hf_tokenizer_file, llm_model, disable_ocr })`), add `hf_architecture`:

```typescript
    async ({ input, backend, categories, model, hf_repo, hf_model_file, hf_tokenizer_file, hf_architecture, llm_model, disable_ocr }) => {
```

In the `nerConfig` object construction, after `hfTokenizerFile: backend === "onnx" ? hf_tokenizer_file : undefined,` add:

```typescript
          hfArchitecture: backend === "onnx" ? hf_architecture : undefined,
```

- [ ] **Step 2: Update `ingest_folder` in `ingest.ts`**

After the `ner_hf_tokenizer_file` Zod param, add:

```typescript
      ner_hf_architecture: z.enum(["gliner1", "gliner2"]).optional().describe(
        "Only used when ner_hf_repo is set. Which GLiNER tensor contract ner_hf_repo uses. Defaults to 'gliner1'. Most GLiNER2 model cards ship safetensors only (no ONNX export) — confirm an .onnx file exists in ner_hf_repo before setting this to 'gliner2'."
      ),
```

Add `ner_hf_architecture` to the handler's destructured parameters (alongside `ner_hf_tokenizer_file`):

```typescript
    async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, use_ner, ner_backend, ner_model, ner_hf_repo, ner_hf_model_file, ner_hf_tokenizer_file, ner_hf_architecture, ner_llm_model, ner_categories }) => {
```

In the `ner` config object, after `hfTokenizerFile: ner_backend === "onnx" ? ner_hf_tokenizer_file : undefined,` add:

```typescript
                    hfArchitecture: ner_backend === "onnx" ? ner_hf_architecture : undefined,
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd mcp-server && npx tsc --noEmit`
Expected: no errors. If `NerConfig`'s generated `.d.ts` type (from Task 10) doesn't yet match what `npm run build`'s last compiled `@xberg-io/xberg` package exposes, rebuild the native binary first per the steps already in flight this session (`cargo build --release -p xberg-node`, then copy the DLL — see the rest of this session's work, not part of this plan).

- [ ] **Step 4: Update the README**

In `mcp-server/README.md`, find the block added this session describing the `hf_repo`/`hf_model_file`/`hf_tokenizer_file` override (search for `knowledgator/gliner-pii-base-v1.0`), and add a paragraph after the existing code example:

```markdown
GLiNER2 models (`fastino/gliner2` lineage) use a different ONNX tensor contract than GLiNER1 — set `hf_architecture: "gliner2"` (or `ner_hf_architecture` on `ingest_folder`) when pointing at one. Most GLiNER2 model cards on HuggingFace ship `safetensors` only with no ONNX export; check the repo's file list for a `.onnx` file before trying this. `lion-ai/gliner2-base-v1-onnx` is a confirmed-working monolithic single-file GLiNER2 ONNX export:

```jsonc
{
  "backend": "onnx",
  "hf_repo": "lion-ai/gliner2-base-v1-onnx",
  "hf_model_file": "model.onnx",
  "hf_tokenizer_file": "tokenizer.json",
  "hf_architecture": "gliner2"
}
```
```

- [ ] **Step 5: Commit**

```bash
git add mcp-server/src/tools/intelligence.ts mcp-server/src/tools/ingest.ts mcp-server/README.md
git commit -m "feat(mcp): expose hf_architecture for GLiNER2 ONNX models"
```

---

## After this plan

- Rebuild `xberg-node`'s native binary (`cargo build --release -p xberg-node`, copy the DLL per the Windows Build Notes in `CLAUDE.md`) and re-run `npx tsc --noEmit` / `npx vitest run` in `mcp-server/` — both already in flight from the prior session turn for the `hf_repo`/`hf_model_file`/`hf_tokenizer_file` fields, so this plan's `hf_architecture` field rides along in the same rebuild rather than needing a second one.
- When `task`/`alef` become available again on this machine, run `task alef:generate` and confirm `git diff --exit-code packages/ crates/xberg-node/` is empty — both this plan's hand-patches and the prior turn's should match what Alef would generate, since both followed its exact codegen conventions.
- Out of scope for this plan, flagged for the user: making `fastino/gliner2-privacy-filter-PII-multi` itself usable requires someone to export it from `safetensors` to ONNX (Python-side, using `gliner2-onnx`-style tooling) and host the result somewhere `hf_repo` can reach — this plan makes xberg capable of running that export once it exists, not the export itself.
