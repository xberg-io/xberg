# Gap Remediation Plan — xberg WASM runtime + PII pipeline

**Branch:** `feature/wasm-runtime-sqlite-store` (PR #13) + active `fix/redaction-clippy-fmt`
**Date:** 2026-07-10
**Scope:** Remediate the verified functional/security/CI gaps. Every item below was confirmed by
reading code or `gh api` CI data — no claim relies on PR descriptions or memory.

## Alignment with existing unmerged work (verified 2026-07-10)

I audited every unmerged branch/worktree + open PR before finalizing. Findings:

- **`pii-quick-wins-medium-lift`** (worktree `.claude/worktrees/...`) — its own plan
  `docs/superpowers/plans/2026-07-10-pii-quick-wins-and-medium-lift.md` covers PII
  **detection-accuracy**: Task 1 IBAN mod-97 (DONE, commit `7fb3f21b68`), Task 2
  `preserve_terms` allowlist, Task 3 `EntityValidator`+rejection counts, Task 4 GDPR
  find/forget, Task 5 eval harness. **None of these touch the gaps below** — they live in
  `crates/xberg/src/text/redaction/*`, while G1/G2 live in `xberg-rag/src/pipeline.rs` and
  `xberg-wasm`. Do NOT duplicate; coordinate so G1's pipeline edits don't clash with that
  plan's `RedactionConfig`/`engine.rs` changes (different files → low conflict).
- **`alef-regen-ner-candle` / `alef-regen-candle-bindings`** — **0 commits ahead of
  `origin/main`** → already merged. The earlier "bindings fail to compile" concern is resolved.
- **`codex/pr13-coderabbit-fixes`** — stale alternate; its `pipeline.rs` predates the
  `redact_request` feature (no `redact_*` fns) and its `cache.ts` still says OPFS "not yet
  implemented". Superseded by current HEAD.
- **`worktree/pr3-conflicts-coderabbit`** — stale; about PR #3 gliner2 conflicts + dart
  `build.rs` + CI. Not our gaps.
- **`fix/redaction-clippy-fmt`** — 1 commit (clippy `rehydration.rs`); does NOT fix the
  `cargo fmt` failure (G5). Fold G5's fmt fix into it or it stays red.

## Verified gap inventory

| ID | Gap | Severity | Evidence | Owner in other branches? |
|----|-----|----------|----------|--------------------------|
| G1 | PII in structured fields (`keywords`/`entities`/`labels`/`metadata`) redacted regex-only, not NER | 🔴 Security | `crates/xberg-rag/src/pipeline.rs:263-279`; CR #3555410976 (open) | No (distinct from pii-quick-wins plan) |
| G2 | `ingest()` in browser requires `initCandleNer` (Candle native); injected JS NER bridge ignored | 🔴 Functional/Sec | `crates/xberg-wasm/src/engine.rs` → `get_candle_ner()` (`bridge/ner.rs:48`) | No |
| G3 | Model cache OPFS (browser) not implemented | 🟠 Functional | `packages/xberg-wasm-runtime/src/cache.ts:54,90` | No (codex branch also unfixed) |
| G4 | `wasm32` build absent from CI (97MB cold-build removed in `19ede81b37`) | 🟠 Verif | no wasm job in PR13 check-runs | No |
| G5 | `cargo fmt` failure + 2× `Lint` failure block CI merge | 🟠 CI | `cargo fmt --check` diff (rehydration.rs, xberg-cli/main.rs, xberg-doc-store/sqlite.rs) | Partial (`fix/redaction-clippy-fmt` does clippy only) |
| G6 | iOS / Android / macOS / ubuntu-arm Rust checks fail | ❓ CI | PR13 check-runs (root cause undiagnosed) | No |

## G5 — Unblock CI formatting/lint (do FIRST)

**Problem.** `cargo fmt --all -- --check` fails on 3 files (mechanical collapses). Two `Lint` jobs
also fail (cause not yet read from logs).

**Approach.**
1. `cargo fmt --all` (fixes rehydration.rs, xberg-cli/main.rs, xberg-doc-store/sqlite.rs verbatim).
2. Read the 2 failing `Lint` job logs (`gh run view <id> --log`) — likely prek/python or markdown
   lint, unrelated to this branch. Fix or document exemption.
3. Commit; the existing `fix/redaction-clippy-fmt` branch should absorb/supersede this (it only
   fixed 1 clippy line, not fmt).

**Files.** `crates/xberg/src/text/redaction/rehydration.rs`, `crates/xberg-cli/src/main.rs`,
`crates/xberg-doc-store/src/backends/sqlite.rs`, `.pre-commit-config.yaml` / lint logs.
**Verification.** `cargo fmt --all -- --check` clean; re-run the 2 Lint jobs green.

## G1 — NER-redact structured ingest fields (highest-value security fix)

**Problem.** `redact_request` routes `keywords`/`entities`/`labels`/`metadata` through
`redact_string_sync` (regex only). Person/Org/Location PII that is not regex-shaped persists raw in
the vector store. `title`/`source_uri`/`external_id` already use NER via `redact_secondary_string`.

**Approach.**
1. Extend `redact_request` so keyword strings and JSON `string` leaves also go through the
   NER+regex path (`redact_secondary_string` / a new `redact_json_value_ner` variant) sharing the
   one `TokenCounter`.
2. Add a `pipeline-redaction` cfg-gated feature flag `ner_structured_fields` (default **on**) so
   feature-minimal builds keep the cheap regex-only behavior and avoid NER-cost regressions.
3. Apply the earlier CodeRabbit fail-closed fix (validate NER byte spans before creating map
   entries) inside the JSON-leaf NER loop — do not redact unverified spans.

**Files.** `crates/xberg-rag/src/pipeline.rs` (`redact_request`, `redact_json_value`),
`crates/xberg-rag/Cargo.toml` (feature), `crates/xberg-rag/src/pipeline.rs` tests (assert stored
chunks contain tokens for structured-field PII).
**Verification.** New test: ingest doc whose `keywords`/`metadata` carry a person name → stored
chunk content redacted + rehydration map contains it. Re-run `cargo test (no-network NER unit tests)`.

## G2 — Let browser `ingest()` use the injected JS NER bridge

**Problem.** `engine.ingest()` calls `get_candle_ner()` directly. The injected JS `ner` bridge
(`self.ner`) is only used by `engine.ner()`, so in-browser ingest PII is impossible unless Candle
compiles to wasm32 (uncertain).

**Approach.**
1. Add a `JsNerBridge` adapter in `crates/xberg-wasm/src/bridge/ner.rs` implementing
   `xberg::text::ner::NerBackend` over the injected JS object (reuse `call_injected_ner`).
2. In `engine.ingest()`, resolve NER **injected-first**: if `self.ner` present, wrap it in
   `JsNerBridge` and pass to `ingest_document`; else fall back to `get_candle_ner()`.
3. Update the constructor doc (already partially clarified) to state this resolution order.

**Files.** `crates/xberg-wasm/src/bridge/ner.rs` (adapter), `crates/xberg-wasm/src/engine.rs`
(ingest resolution).
**Verification.** Browser vitest: build engine with injected `ner` bridge, call `ingest()` with PII
text → returns rehydration map, no `initCandleNer` required. Keep the Candle fallback path tested in
Node.

## G3 — Browser OPFS model cache (highest effort, lowest urgency)

**Problem.** `CacheManager` only persists to `~/.cache/xberg` on Node; browser branch is stubbed
(`cache.ts:54,90`).

**Approach (scoped).** Reuse the existing OPFS SAH pool already wired for the vector store
(`store-worker.ts`) to add a small async key/value for model files (`model.safetensors`,
`tokenizer.json`, `encoder_config.json`):
1. Add `opfsModelExists(key)` / `opfsModelPut(key, bytes)` to the worker message protocol.
2. `CacheManager.cacheModel(...)` awaits OPFS when `typeof window !== undefined`, falls back to
   in-memory when unavailable.
3. Guard all OPFS access so Node returns the existing `~/.cache/xberg` path unchanged.

**Files.** `packages/xberg-wasm-runtime/src/cache.ts`, `store-worker.ts` (protocol),
`wasm/sqlite-vec/sqlite3-opfs-async-proxy.js` (already present).
**Verification.** Playwright: cold then warm model load in browser worker, assert second load hits
OPFS (no network). If OPFS proves too costly this cycle, **explicitly scope G3 down to
"document Node-only model cache, defer browser"** rather than ship a half-impl.

## G4 — Add a `wasm32` compile gate to CI

**Problem.** No wasm job exists; the `.wasm` artifact is unverified by CI.

**Approach.** Add a lightweight `cargo check --target wasm32-unknown-unknown -p xberg-wasm`
(job `wasm check`), avoiding the removed 97MB full `wasm-pack build`. This validates compilation
without artifact weight. Keep the full `wasm-pack build` as an opt-in/separate workflow (not blocking).

**Files.** `.github/workflows/ci-rust.yml` (new job), `crates/xberg-wasm/Cargo.toml` (confirm
`wasm-target` features compile under wasm32).
**Verification.** Job green on PR; documents that the wasm binary at least compiles.

## G6 — Diagnose mobile/arm Rust failures (diagnose-first, low priority)

**Problem.** iOS / Android / macOS / ubuntu-arm `cargo check` fail on PR13. Root cause unread.

**Approach.** `gh run view <id> --log-failed` for one iOS + one Android job. Hypothesize ORT /
`xberg-gliner-candle` / `xberg-paddle-ocr` not building on those targets (feature-flag or
`target_arch` issue), not our runtime gaps. Fix only if it is a real regression we introduced;
otherwise open a tracking issue.

## Recommended sequencing

1. **G5** (trivial, unblocks merge) → commit/push.
2. **G1** (security, contained, well-understood) → land with tests.
3. **G2** (browser parity) → depends on G1's span-validation discipline.
4. **G4** (cheap CI signal) → parallel with G2.
5. **G3** (defer/scope if over budget).
6. **G6** (diagnose; likely out of our scope).

## Implementation status

| ID | Status | Notes |
|----|--------|-------|
| G5 | **PARTIAL** | `cargo fmt --all` applied; `cargo fmt --check` passes. Two Lint failures diagnosed: (1) Elixir hex.pm DNS failure (`cdn.hex.pm` ENOTFOUND) — transient network, not our code. (2) Second Lint likely same. Commit pending. |
| G1 | **CODE + TEST DONE** | `redact_request` rewritten. `is_free_text_leaf` heuristic. Test: `ingest_redacts_pii_in_structured_keywords_and_metadata` covers keyword + metadata PII redaction. |
| G2 | **CODE DONE** | `JsNerBridge` adapter in `bridge/ner.rs` implements `NerBackend` over injected JS object. `engine.ingest()` resolves NER injected-first: JS bridge → Candle fallback → error. |
| G3 | **DEFERRED** | Node path works; browser OPFS cache documented as "not implemented" in `cache.ts`. Ship docs, not half-impl. |
| G4 | **DONE** | Added `wasm-check` job to `ci-rust.yaml`: `cargo check -p xberg-wasm --target wasm32-unknown-unknown --features wasm-target`. |
| G6 | **DIAGNOSED** | All Rust failures (macOS, arm, iOS, Android) are **stale Dart bindings**: `packages/dart/rust/src/frb_generated.rs` references undefined `var_hfRepo`, `var_hfModelFile`, etc., and `packages/dart/rust/src/lib.rs` is missing `NerConfig` fields (`lora_adapter_dir`, `model_dir`) and `NerBackendKind::Candle` match arm. Fix: run `task alef:generate` to regenerate Dart bindings. MCP failures: `store.ts:338` missing `close` method on `VectorStoreInterface`. |

## Critical review of this plan

- **G2 is the riskiest.** Routing `ingest` through the injected JS NER couples the wasm pipeline to
  a JS async contract that must match `call_injected_ner` exactly, and reintroduces the earlier
  fail-closed span-validation concern. If Candle genuinely works in wasm32, G2 may be lower value
  than it appears — **verify Candle-wasm feasibility before committing to G2**; if it works, G2
  becomes optional polish, not a blocker.
- **G1 cost is real.** NER on every JSON `metadata` leaf can blow up latency on large metadata.
  The `ner_structured_fields` default-on flag is a footgun (silent PII leak if a consumer disables
  it). Prefer: NER on `keywords` (bounded) always, and on JSON leaves only when leaf is a free-text
  string (heuristic: length > 20 chars, no key like `id`/`url`/`hash`). That bounds cost without a
  silent-PII toggle.
- **G3 may not be worth shipping this cycle.** Node path is correct; browser model cache is a
  nice-to-have. Shipping a stubbed-but-present OPFS path is worse than documenting the limitation.
  Recommend deferring to a follow-up unless a concrete browser workload needs offline models.
- **G4 is the highest ROI.** A `cargo check` wasm gate is nearly free and would have caught the
  wasm/TS drift we cannot currently verify locally. Do it early.
- **G5 must not be siloed in `fix/redaction-clippy-fmt`** — that branch only fixed clippy, not fmt;
  fold the fmt fix into the same branch or it will keep failing CI.
