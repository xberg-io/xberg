# PII Pipeline: Quick Wins + Medium Lift Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Source:** `docs/superpowers/results/2026-07-10-xberg-vs-anno-pii-ner-gap-analysis` (artifact, not committed to the repo) — a source-level comparison of xberg's redaction pipeline against `anno`/`cloakpipe`, read for a regulated-business/legal-lawyer product. This plan implements the artifact's "Quick wins" and "Medium lift" tiers only; "Major investment" items (human review loop, locale-specific validator packs, multi-backend NER ensemble) are out of scope here.

**Scope note vs. the artifact:** the artifact listed "per-category rejection counters" as a standalone quick win. On inspection, that isn't cheap to do in isolation: xberg has **no global state** (repo rule, high priority) and the pattern engine's `find_all(text) -> Vec<PatternMatch>` contract is shared by all 8 pattern modules — plumbing rejection reasons out of `find_all` means touching every module's return type for one feature. The validator trait (Medium Lift, Task 3) runs as a *post-aggregation* pass over an already-collected `Vec<PatternMatch>`, so rejection counting falls out of it for free with no signature changes anywhere. Rejection counters are folded into Task 3 below instead of being their own task.

**Tech stack:** Rust 2024 (core crate), TypeScript (mcp-server tools), existing `regex`/`aes-gcm`/`scrypt` dependencies — no new crates required for any task in this plan.

**Repo rules that apply throughout:**
- No global state — every new piece of state is a parameter or a struct field, never a `static`/`thread_local`.
- `RedactionConfig` carries `#[cfg_attr(feature = "alef-meta", alef(since = "..."))]` — any new public field needs its own `alef(since = "X.Y.Z")` marker. Adding a field with a serde default is additive/non-breaking (minor version), not a major bump.
- Commit procedure is lightweight: run `prek run --all-files` before committing; do not require a full `cargo test`/`cargo check` pass unless the task's own verification step says so.
- Conventional commits, imperative mood, first line under 72 chars.
- After any change under `crates/xberg-wasm/` or `packages/*/`, run `task alef:generate` and diff-check per the alef-generated-bindings rule — Tasks 1 and 2 touch `RedactionConfig`, which is alef-managed; regenerate bindings as part of those tasks' commits.

---

## Progress

**Status (2026-07-10): all five implementation tasks complete; Final verification (Task 6) partially run.** Work lives in the `pii-quick-wins-medium-lift` worktree.

| Task | Status | Verification |
| ---- | ------ | ----- |
| 1 — IBAN mod-97 checksum | Done (committed: `7fb3f21b68`) | `cargo test` redaction passes |
| 2 — `preserve_terms` allowlist | Done (committed: `3998660221`) | `cargo test` redaction passes |
| 3 — `EntityValidator` trait + rejection counts | Done (committed: `1044127319`) | `cargo test` redaction passes |
| 4 — GDPR subject find/forget | Done (uncommitted in worktree) | Rust: 11/11 rehydration tests pass. TS e2e (`mcp-server` vitest `rehydrate`) needs the wasm engine built + the embedder model downloaded — blocked in this Windows env. |
| 5 — PII eval harness | Done (uncommitted in worktree) | 4/4 eval tests pass; clippy clean |
| 6 — Final verification | Partial | Rust: 44 redaction tests + 26 rag pipeline tests pass; clippy `-D warnings` clean. `mcp-server npm test` + `prek run --all-files` blocked here (wasm build / model download + TS module-resolution). |

Task 4 and Task 5 are implemented and Rust-verified but **not yet committed** — the worktree also carries unrelated uncommitted `xberg-wasm` changes from the `feature/wasm-runtime-sqlite-store` base, so a clean per-task commit needs those separated first.

---

## Part 1: Quick Wins

### Task 1: IBAN mod-97 checksum validation

**Files:**
- Modify: `crates/xberg/src/text/redaction/patterns/iban.rs`

**Interfaces:**
- Consumes: nothing new — operates on the candidate string `iban::find_all` already isolates.
- Produces: `find_all` rejects candidates that pass the existing country-code + length checks but fail the ISO 13616 mod-97 checksum. No signature change.

**Current state:** `find_all` filters candidates by country-code allowlist and total length (15–34 chars) only — no checksum. Any string shaped like `FR76XXXXXXXXXXXXXXXXXXX` with a valid-looking country prefix currently passes.

- [x] **Step 1: Add the mod-97 checksum function**

In `iban.rs`, add a private helper next to the existing `find_all`:

```rust
/// ISO 13616 IBAN checksum: move the first 4 characters to the end, convert
/// letters to numbers (A=10, B=11, ... Z=35), and verify the resulting
/// number mod 97 equals 1. Rejects the ~1-in-100 non-checksum-valid strings
/// that the country-code + length filter alone lets through.
fn iban_checksum_valid(compact: &str) -> bool {
    if compact.len() < 4 {
        return false;
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    let mut remainder: u64 = 0;
    for c in rearranged.chars() {
        let value = if c.is_ascii_digit() {
            c.to_digit(10).unwrap() as u64
        } else if c.is_ascii_uppercase() {
            (c as u64) - ('A' as u64) + 10
        } else {
            return false;
        };
        // Fold digit-by-digit (or two-digit for letters) to avoid overflow
        // on IBANs up to 34 chars (~68 decimal digits after expansion).
        let digits = if value >= 10 { 2 } else { 1 };
        remainder = (remainder * 10u64.pow(digits) + value) % 97;
    }
    remainder == 1
}
```

- [x] **Step 2: Wire it into `find_all`**

In the existing `filter_map` closure, after the length check and before constructing `PatternMatch`, add:

```rust
if !iban_checksum_valid(&compact) {
    return None;
}
```

- [x] **Step 3: Tests**

Add to `iban.rs`'s `#[cfg(test)] mod tests`:
- `checksum_valid_iban_is_detected` — a real, checksum-valid IBAN (e.g. `FR7630006000011234567890189` — verify the checksum digit by hand or against a known-good test vector before hardcoding it).
- `checksum_invalid_iban_is_rejected` — the same IBAN with one digit flipped in the BBAN; assert `find_all` returns no match for it.
- Keep the existing country-code and length tests passing unchanged.

- [x] **Step 4: Verify**

```bash
cargo test -p xberg --lib text::redaction::patterns::iban
cargo clippy -p xberg --lib -- -D warnings
```

- [x] **Step 5: Commit**

```bash
git add crates/xberg/src/text/redaction/patterns/iban.rs
git commit -m "fix(redaction): validate IBAN mod-97 checksum, not just shape"
```

---

### Task 2: Preserve/allowlist terms on `RedactionConfig`

**Files:**
- Modify: `crates/xberg/src/core/config/redaction.rs` (add field)
- Modify: `crates/xberg/src/text/redaction/engine.rs` (apply the filter; de-duplicate the category-filter block)
- Regenerate: `task alef:generate` (RedactionConfig is alef-managed; diff-check `packages/`, `crates/xberg-wasm/`, `crates/xberg-ffi/` after)

**Interfaces:**
- Consumes: nothing new.
- Produces: `RedactionConfig.preserve_terms: Vec<RedactionTerm>` — reuses the existing `RedactionTerm` type (`label`, `value`, `case_sensitive`) for API symmetry with `custom_terms`, rather than inventing a new shape.

**Current state:** `custom_terms`/`custom_patterns` are a *force* list only. A false-positive NER hit on a legitimate public entity (a court name, the client's own company name) can only be suppressed by disabling that entity's whole category via `categories`, which also suppresses genuine hits in it. `engine.rs` has the category-filter logic duplicated verbatim in two places (`redact_inner` step 3, and `build_matches_for`) — this task extracts a shared helper and adds preserve-filtering to it in one place instead of three.

- [x] **Step 1: Add the field**

In `crates/xberg/src/core/config/redaction.rs`, add to `RedactionConfig` (after `custom_patterns`):

```rust
    /// Literal terms that must never be redacted, even if the pattern engine
    /// or NER backend would otherwise flag them.
    ///
    /// Use this for known-public entities that a NER model mistakes for PII
    /// (e.g. "Supreme Court", the caller's own organization name) — an
    /// allowlist counterpart to [`custom_terms`](Self::custom_terms), which
    /// is a forcelist. A term matches by exact value (respecting
    /// `case_sensitive`), not by category — it suppresses that literal
    /// string across every category, since a false positive doesn't know
    /// its own category was wrong.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.1.0"))]
    pub preserve_terms: Vec<RedactionTerm>,
```

Update `Default for RedactionConfig` to include `preserve_terms: Vec::new()`.

- [x] **Step 2: Extract a shared filter helper in `engine.rs`**

Replace the duplicated block:
```rust
if !config.categories.is_empty() {
    matches.retain(|m| matches!(m.category, PiiCategory::Custom(_)) || config.categories.contains(&m.category));
}
```
(present in both `redact_inner` step 3 and `build_matches_for`) with a single function:

```rust
/// Apply the category allowlist (if any categories were configured) and the
/// preserve-terms denylist. Shared by the main-content path and
/// `build_matches_for` (formatted_content + chunks) so preserve semantics
/// are identical everywhere redaction runs.
fn apply_category_and_preserve_filters(matches: &mut Vec<PatternMatch>, config: &RedactionConfig) {
    if !config.categories.is_empty() {
        matches.retain(|m| matches!(m.category, PiiCategory::Custom(_)) || config.categories.contains(&m.category));
    }
    if !config.preserve_terms.is_empty() {
        matches.retain(|m| {
            !config.preserve_terms.iter().any(|term| {
                if term.case_sensitive {
                    m.text == term.value
                } else {
                    m.text.eq_ignore_ascii_case(&term.value)
                }
            })
        });
    }
}
```

Call it from both sites (replacing the old inline block), before `dedupe_overlaps` in each.

- [x] **Step 3: Validate preserve terms**

In `RedactionConfig::validate()`, extend the existing `custom_terms` empty-value check to also cover `preserve_terms` (same rule: empty `value` is rejected).

- [x] **Step 4: Tests**

Add to `engine.rs`'s test module:
- `preserve_terms_suppresses_a_matching_ner_hit` — configure NER-detected `Person`/`Organization`, add a preserve term matching one detected span's text, assert it's absent from `findings` while other hits remain.
- `preserve_terms_is_case_insensitive_by_default` — mixed-case match still suppressed.
- `preserve_terms_respects_case_sensitive_flag` — set `case_sensitive: true`, assert a differently-cased occurrence is *not* suppressed.
- `preserve_terms_applies_to_chunks_and_formatted_content_too` — regression test for the de-duplication in Step 2; construct an `ExtractedDocument` with `chunks` set and confirm the preserved term survives redaction there as well as in `content`.

- [x] **Step 5: Regenerate bindings and verify**

```bash
task alef:generate
git diff --exit-code packages/ crates/xberg-node/ crates/xberg-wasm/ crates/xberg-ffi/  # confirm what changed, don't just discard it
cargo test -p xberg --lib text::redaction
cargo clippy -p xberg --lib -- -D warnings
```

- [x] **Step 6: Commit**

Commit the Rust source and regenerated bindings together:

```bash
git add crates/xberg/src/core/config/redaction.rs crates/xberg/src/text/redaction/engine.rs packages/ crates/xberg-node/ crates/xberg-wasm/ crates/xberg-ffi/
git commit -m "feat(redaction): add preserve_terms allowlist, dedupe category-filter logic"
```

---

## Part 2: Medium Lift

### Task 3: `EntityValidator` trait + rejection-count audit output

**Files:**
- Create: `crates/xberg/src/text/redaction/validators/mod.rs`
- Create: `crates/xberg/src/text/redaction/validators/iban.rs` (migrates Task 1's checksum)
- Create: `crates/xberg/src/text/redaction/validators/luhn.rs` (migrates the existing inline Luhn check from `patterns/credit_card.rs`)
- Modify: `crates/xberg/src/text/redaction/mod.rs` (declare `pub mod validators;`, re-export `EntityValidator`, `ValidationResult`, `RejectionCounts`)
- Modify: `crates/xberg/src/text/redaction/engine.rs` (run validators post-`dedupe_overlaps`, thread `RejectionCounts` into both redaction outcome types)
- Modify: `crates/xberg/src/text/redaction/engine.rs`'s `TextRedactionOutcome` and the plain `redact()` result type — add a `rejection_counts` field

**Interfaces:**
- Consumes: `Vec<PatternMatch>` (already exists), original document text (already available at the call site).
- Produces:
  - `pub trait EntityValidator: Send + Sync + std::fmt::Debug { fn label(&self) -> &'static str; fn validate(&self, entity: &PatternMatch, ctx: &str) -> ValidationResult; }`
  - `pub enum ValidationResult { Accept, Reject { reason: &'static str }, AdjustConfidence(f32) }` — note `PatternMatch` doesn't currently carry a confidence field; `AdjustConfidence` is a no-op placeholder for `Reject`/`Accept`-only validators until (if ever) confidence scoring is added to `PatternMatch`. Don't add confidence in this task — out of scope; implement `Accept`/`Reject` only and note `AdjustConfidence` as unused for now with a `#[allow(dead_code)]` or simply omit the variant until it has a real consumer (prefer omitting — YAGNI).
  - `pub type RejectionCounts = std::collections::BTreeMap<&'static str, usize>;`
  - `pub fn apply_validators(matches: Vec<PatternMatch>, text: &str, validators: &[Box<dyn EntityValidator>]) -> (Vec<PatternMatch>, RejectionCounts)`

**Design mirrors `anno-rag`'s `crates/anno-rag/src/validators/mod.rs` exactly** (same trait shape, same `apply_validators` semantics: first `Reject` short-circuits the validator chain for that entity, matched by `label()` against the entity's category). That crate is a good reference for the exact aggregation logic if anything below is ambiguous — re-read it before starting.

- [x] **Step 1: Define the trait and `apply_validators`**

Create `validators/mod.rs` with `EntityValidator`, `ValidationResult` (`Accept` / `Reject { reason: &'static str }` only — see note above), `RejectionCounts`, and `apply_validators`. Category-to-label matching: reuse the same `format!("{:?}", category)` string the engine already uses for `category_counts` (e.g. `"Iban"`, `"CreditCard"`), so a validator's `label()` returns e.g. `"Iban"` and gets matched the same way.

- [x] **Step 2: Migrate the IBAN checksum from Task 1 into `IbanChecksumValidator`**

Move `iban_checksum_valid` (Task 1) from `patterns/iban.rs` into `validators/iban.rs` as an `EntityValidator` impl. Remove the inline call added in Task 1's Step 2 from `patterns/iban.rs::find_all` — the checksum now runs once, post-aggregation, instead of inline in the pattern scan. Keep `patterns/iban.rs`'s country-code + length filtering in place (that's shape validation, still appropriate at scan time — only the checksum, which needs no regex-adjacent context, moves).

- [x] **Step 3: Add `LuhnValidator`, migrate `credit_card.rs`'s inline check**

Same pattern: move the Luhn check out of `patterns/credit_card.rs::find_all` into `validators/luhn.rs`, label `"CreditCard"`.

- [x] **Step 4: Wire `apply_validators` into `engine.rs`**

In both `redact_inner` and `redact_capturing_rehydration_map`, after `dedupe_overlaps`, call:
```rust
let default_validators: Vec<Box<dyn EntityValidator>> = vec![
    Box::new(validators::iban::IbanChecksumValidator),
    Box::new(validators::luhn::LuhnValidator),
];
let (matches, rejection_counts) = apply_validators(matches, text, &default_validators);
```
Thread `rejection_counts` into `TextRedactionOutcome` (new field) and into whatever the plain `redact()` returns (check `ExtractedDocument`'s redaction-summary field, or add one if none exists — read the current return type before deciding where this surfaces; don't invent a new public field if an equivalent summary slot already exists).

- [x] **Step 5: Tests**

- Move Task 1's checksum tests from `patterns/iban.rs` to `validators/iban.rs`, adjusted to call the validator directly.
- Move the existing Luhn tests from `patterns/credit_card.rs` similarly.
- New: `apply_validators_short_circuits_on_first_rejection` — a matched entity failing one validator doesn't get evaluated against a second one for the same label.
- New: `rejection_counts_are_keyed_by_reason_not_by_category` — two different failure reasons for the same category produce two separate counter entries.
- Full-pipeline regression: confirm `crates/xberg-rag/src/pipeline.rs`'s existing PII tests (which call into this engine) still pass unchanged — this task must not change redaction *output* for well-formed input, only add rejection visibility for malformed input that used to be silently dropped inline.

- [x] **Step 6: Verify**

```bash
cargo test -p xberg --lib text::redaction
cargo test -p xberg-rag --features "in-memory,pipeline-redaction"
cargo clippy -p xberg --lib -- -D warnings
```

- [x] **Step 7: Commit**

```bash
git add crates/xberg/src/text/redaction/
git commit -m "refactor(redaction): extract EntityValidator trait, surface rejection counts"
```

---

### Task 4: GDPR-style subject lookup and erasure (Art. 15 / 17 groundwork)

**Files:**
- Modify: `crates/xberg/src/text/redaction/rehydration.rs` (add `find_subject`, `forget_subject`)
- Modify: `mcp-server/src/tools/rehydrate.ts` (add `find_pii_subject`, `forget_pii_subject` MCP tools)
- Modify: `mcp-server/src/redaction/rehydration.ts` (TypeScript-side equivalents — must stay wire-compatible with the Rust implementation per the existing `SCRYPT_LOG_N`/`R`/`P` sync comment)

**Interfaces:**
- Consumes: `RehydrationMap` (already exists — `HashMap<String, String>`, token → original).
- Produces (Rust, `rehydration.rs`):
  ```rust
  /// One vault match — either direction of lookup.
  pub struct SubjectMatch {
      pub token: String,
      pub original: String,
  }

  /// Search a decrypted map for `query`, matching either the token or the
  /// original value (case-insensitive substring match on `original`; exact
  /// match on `token`, since tokens are structured like "[EMAIL_1]").
  pub fn find_subject(map: &RehydrationMap, query: &str) -> Vec<SubjectMatch>;

  /// Remove every mapping whose token or original value matches `query`.
  /// Returns the removed entries (the caller re-encrypts and persists the
  /// resulting map — this function does not touch disk).
  pub fn forget_subject(map: &mut RehydrationMap, query: &str) -> Vec<SubjectMatch>;
  ```

**Design note — no per-entity category in `RehydrationMap` today.** Unlike anno's vault (which stores `category` alongside each mapping), xberg's `RehydrationMap` is a flat `token -> original` map with no category field — but the token itself encodes the category (`"[EMAIL_1]"`, `"[PERSON_2]"`). Derive category for `SubjectMatch` by parsing the token's bracket contents up to the trailing `_<N>` rather than adding a new field to `RehydrationMap` (`HashMap<String,String>` is also the on-disk/wire format shared with the TypeScript side — changing its shape is a breaking wire-compat change, out of scope here). Add a small private `fn category_from_token(token: &str) -> Option<&str>` helper.

- [x] **Step 1: `find_subject` and `forget_subject` in `rehydration.rs`**

Implement per the signatures above. `find_subject` is a linear scan (maps here are one document's worth of PII, not a full corpus — no index needed). Case-insensitive substring match against `original` (a lawyer searching "Alice" should find "Alice Johnson"); exact match against `token`.

- [x] **Step 2: Tests in `rehydration.rs`**

- `find_subject_matches_by_original_value_substring`
- `find_subject_matches_by_exact_token`
- `find_subject_returns_empty_for_no_match`
- `forget_subject_removes_matching_entries_and_returns_them`
- `forget_subject_is_idempotent_on_repeated_calls` (second call returns empty, doesn't error)
- `forget_then_reencrypt_round_trips` — remove a subject, re-encrypt the map with `encrypt_map`, decrypt again, confirm the subject is genuinely gone (not just filtered client-side).

- [x] **Step 3: MCP tools**

In `mcp-server/src/tools/rehydrate.ts`, following the existing `rehydrate_document` pattern exactly (same `document_id`/`passphrase`/`rehydration_dir` params, same map-file-on-disk flow):

```
find_pii_subject(document_id, passphrase, query, rehydration_dir?)
  -> decrypts the map, calls the Rust find_subject via the WASM/native binding,
     returns { matches: [{token, original, category}] }

forget_pii_subject(document_id, passphrase, query, rehydration_dir?)
  -> decrypts the map, calls forget_subject, re-encrypts the reduced map,
     OVERWRITES the .map file on disk, returns an erasure receipt:
     { subject_ref: query, removed_count: N, removed_tokens: [...] }
```

`forget_pii_subject` is the only new tool in this plan that mutates state on disk — document that clearly in its MCP tool description (mirroring `redact_document`'s existing "destructive" framing per the pii-pipeline rule) and confirm the file was actually overwritten in its own test, not just that the in-memory map changed.

- [x] **Step 4: TypeScript-side parity**

If `mcp-server/src/redaction/rehydration.ts` has its own encrypt/decrypt implementation (separate from calling into the Rust/WASM engine) rather than delegating to it, add matching `findSubject`/`forgetSubject` functions there too, keyed the same way. Check which is actually the case before writing this step's real implementation — the existing `rehydrate_document` tool calls `engine.decrypt_map` (the native binding), so the new tools most likely also just call into Rust rather than reimplementing in TS; confirm before assuming a parallel TS implementation is needed at all.

- [x] **Step 5: Verify**

```bash
cargo test -p xberg --lib text::redaction::rehydration
cd mcp-server && npm test -- rehydrate
```

- [x] **Step 6: Commit**

```bash
git add crates/xberg/src/text/redaction/rehydration.rs mcp-server/src/tools/rehydrate.ts mcp-server/src/redaction/rehydration.ts
git commit -m "feat(pii): add subject find/forget for GDPR Art. 15/17 workflows"
```

---

### Task 5: PII detection accuracy evaluation harness

**Files:**
- Create: `crates/xberg/src/text/redaction/eval.rs`
- Create: `crates/xberg/tests/fixtures/pii_eval/` (labeled corpus — start small, grow over time)
- Create: `crates/xberg/tests/fixtures/pii_eval/annotations.toml` (ground-truth spans)

**Interfaces:**
- Consumes: `Vec<PatternMatch>` (detector output), a ground-truth span list.
- Produces:
  ```rust
  pub struct TrueSpan { pub start: usize, pub end: usize, pub category: String }
  pub struct CategoryScore { pub true_positives: usize, pub false_positives: usize, pub false_negatives: usize, pub precision: f64, pub recall: f64, pub f1: f64 }
  pub fn score(detected: &[PatternMatch], truth: &[TrueSpan]) -> std::collections::BTreeMap<String, CategoryScore>
  ```

**Design mirrors `anno-rag`'s `crates/anno-rag/src/pii_eval.rs`** — same overlap-span matching approach (a detection counts as a true positive if it overlaps a ground-truth span of the same category by at least one byte, not exact-offset match, since regex/NER span boundaries can differ by a character or two on real text). Re-read that file for the exact matching algorithm before implementing — it already solved the "how much overlap counts" question.

- [x] **Step 1: Scoring function**

Implement `score()` with overlap-span matching per-category precision/recall/F1, matching `anno-rag`'s algorithm.

- [x] **Step 2: Seed fixture corpus**

Unlike anno's corpus (real French legal document types), xberg's fixtures should reflect **xberg's actual target documents** — check with the document types xberg's current integration tests already use (`crates/xberg/tests/` fixtures) rather than inventing new synthetic text. Start with 5-10 documents covering the 12 built-in `PiiCategory` variants at least once each, with hand-verified ground-truth spans in `annotations.toml` (format: `[[span]] file = "..." start = N end = N category = "Email"`, one array-table per span).

- [x] **Step 3: Corpus loader + integration test**

`fn load_corpus(dir: &Path) -> Result<Vec<(String, Vec<TrueSpan>)>>` reading the fixture directory + TOML annotations. One `#[test]` that runs the full redaction pipeline (regex-only, no NER — keep this fast/deterministic for CI) against the corpus and asserts a minimum F1 floor per category (e.g. `>= 0.85` for regex-only categories with checksums; looser for NER-only categories if the test includes NER). This becomes the regression gate: a future change that silently degrades detection quality fails this test instead of shipping unnoticed.

- [x] **Step 4: Wire into CI-relevant task**

Check `Taskfile.yaml` for where `cargo test -p xberg` already runs in CI and confirm this new test executes there without needing a new task — it should, since it's a normal `#[test]`. If the corpus grows large enough to be slow, consider gating it behind a feature flag or `#[ignore]` + explicit CI step, but don't add that complexity until the corpus actually needs it (YAGNI — start simple).

- [x] **Step 5: Verify**

```bash
cargo test -p xberg --lib text::redaction::eval
cargo test -p xberg --test '*' pii_eval  # if fixtures live under tests/
```

- [x] **Step 6: Commit**

```bash
git add crates/xberg/src/text/redaction/eval.rs crates/xberg/tests/fixtures/pii_eval/
git commit -m "test(redaction): add precision/recall/F1 eval harness against labeled corpus"
```

---

## Sequencing

Tasks 1 → 2 → 3 → 4 → 5 in order — Task 3 depends on Task 1 (migrates its checksum), Task 3's rejection-count plumbing is easiest to test correctly once Task 1 exists to generate real rejections. Task 4 and Task 5 are independent of each other and of Task 3; either could run in parallel with it if using subagent-driven-development with multiple workers, but Task 4 touches `rehydration.rs` (untouched by Tasks 1-3) and Task 5 is additive-only (new files), so there's no file-level conflict risk either way.

## Final verification (after all 5 tasks)

```bash
cargo test -p xberg --lib text::redaction
cargo test -p xberg-rag --features "in-memory,pipeline-redaction"
cargo test -p xberg-rag --features "in-memory,pipeline"
cargo clippy -p xberg --lib -- -D warnings
cd mcp-server && npm test
prek run --all-files
```

Whole-plan review: confirm no regression in existing redaction output for well-formed input (Tasks 1 and 3 change *what gets rejected*, not what gets accepted-and-redacted for already-passing cases), and that `RedactionConfig`'s new `preserve_terms` field round-trips through every binding surface Alef regenerated in Task 2.
