# Fence & Upstream-Sync Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fence the fork's additions from upstream-owned files so future `xberg-io/xberg` resyncs are a bounded, documented merge instead of a 650-file reconciliation.

**Architecture:** A two-tier fence. Tier 1 (overlay) = fork-only crates/packages/dirs + one extracted API module, all isolated so upstream cannot touch them. Tier 2 (carry-patch) = the ~31 files both sides edit, tracked in a manifest with a machine check. Add `upstream` remote + a selective-sync workflow. All on the current `rc.5` base; no upstream catch-up merge in this project.

**Tech Stack:** Rust 2024 (cargo workspaces, feature flags), Bash (POSIX; runs under Git Bash on Windows), Markdown/TSV docs, ai-rulez generator.

## Global Constraints

- Workspace version floor: `1.0.0-rc.5` (do not bump in this project — copy verbatim in any manifest).
- Upstream remote = `https://github.com/xberg-io/xberg.git` named `upstream`; `origin` stays `https://github.com/jamon8888/xberg.git`.
- Merge-base with upstream at plan authoring: `e702310938` (2026-07-04). Scripts must not hard-code it — compute `git merge-base HEAD upstream/main` at runtime.
- Never edit `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` directly — they are generated from `.ai-rulez/`. Edit source, then `ai-rulez generate` + `task ai-rulez:generate`.
- Commit style: conventional commits, imperative, <72 char subject. **No AI attribution / Co-Authored-By lines** (repo rule `no-ai-signatures`, critical).
- Do not run `git pull --rebase`/`rebase`/`merge` against upstream in this project. The dry-run in Task 6 uses a single `cherry-pick` on a throwaway branch, then aborts/resets.
- Fork-only crates (Tier-1, verified absent from `xberg-io/main`): `xberg-rag-node`, `xberg-doc-store`, `xberg-gliner-candle`, `xberg-pdfium-render`.
- Fork-only packages/dirs (Tier-1): `packages/xberg-wasm-runtime`, `packages/xberg-web-ui`, `mcp-server/`, `docs/superpowers/`.

---

## File Structure

- `docs/superpowers/upstream/sync.md` — the selective-sync runbook (create)
- `docs/superpowers/upstream/carry-patches.md` — human-readable Tier-2 register (create)
- `docs/superpowers/upstream/carry-patches.tsv` — machine-checkable Tier-2 list: `path<TAB>tier<TAB>owning_feature<TAB>note` (create)
- `scripts/upstream-diff.sh` — report which carry-patch files a candidate upstream ref touches (create)
- `scripts/check-carry-patches.sh` — fail if a Tier-2 file changed vs merge-base without a manifest entry (create)
- `Cargo.toml` — fence `members`, `[workspace.dependencies]`, `[patch.crates-io]` into append-only fork blocks (modify)
- `crates/xberg/src/api/rag/mod.rs` — fork-owned API module: process + rehydrate (create)
- `crates/xberg/src/api/rag/handlers.rs` — `process_handler`, `rehydrate_handler` moved here (create)
- `crates/xberg/src/api/rag/types.rs` — `Process*` / `Rehydrate*` request/response types moved here (create)
- `crates/xberg/src/api/handlers.rs` — remove moved handler bodies; keep upstream shape (modify)
- `crates/xberg/src/api/types.rs` — remove moved `Process*`/`Rehydrate*` types; `ApiState.rehydration_store` field stays as tracked carry-patch (modify)
- `crates/xberg/src/api/router.rs` — route wiring stays (carry-patch), imports point at `rag` module (modify)
- `crates/xberg/src/api/mod.rs` — `pub mod rag;` gated on fork feature (modify)
- `crates/xberg/Cargo.toml` — add fork feature `process-api`; gate module deps under it (modify)
- `crates/xberg/tests/api_route_parity.rs` — route-set parity + module-boundary test (create)
- `.ai-rulez/` — correct the `upstream-sync` rule source (modify; exact path found in Task 5)

---

## Task 1: Upstream remote + selective-sync runbook + diff script

**Files:**
- Create: `scripts/upstream-diff.sh`
- Create: `docs/superpowers/upstream/sync.md`

**Interfaces:**
- Produces: `scripts/upstream-diff.sh` — run as `bash scripts/upstream-diff.sh [<upstream-ref>]` (default `upstream/main`); prints, to stdout, the subset of `carry-patches.tsv` paths that the upstream ref changed since the merge-base. Exit 0 always (report tool). Consumed by the runbook and by a human before a resync.

- [ ] **Step 1: Make the upstream remote permanent and fetch**

```bash
git remote get-url upstream >/dev/null 2>&1 || git remote add upstream https://github.com/xberg-io/xberg.git
git fetch upstream main
git merge-base HEAD upstream/main   # expect a sha (e702310938... at authoring)
```
Expected: a commit sha prints (a common ancestor exists).

- [ ] **Step 2: Write the diff script**

```bash
#!/usr/bin/env bash
# Report which tracked carry-patch files an upstream ref changed since the merge-base.
set -euo pipefail
REF="${1:-upstream/main}"
ROOT="$(git rev-parse --show-toplevel)"
TSV="$ROOT/docs/superpowers/upstream/carry-patches.tsv"
MB="$(git merge-base HEAD "$REF")"
echo "merge-base: $MB"
echo "upstream ref: $REF ($(git rev-parse --short "$REF"))"
echo "--- carry-patch files touched upstream since merge-base ---"
if [ ! -f "$TSV" ]; then echo "(no manifest yet: $TSV)"; exit 0; fi
touched="$(git diff --name-only "$MB" "$REF")"
n=0
while IFS=$'\t' read -r path tier feature note; do
  [ "$path" = "path" ] && continue          # header
  [ -z "${path:-}" ] && continue
  if printf '%s\n' "$touched" | grep -qxF "$path"; then
    printf '  %-55s [%s] %s\n' "$path" "$feature" "$note"
    n=$((n+1))
  fi
done < "$TSV"
echo "--- $n carry-patch file(s) need manual review on resync ---"
```
Write the above to `scripts/upstream-diff.sh`.

- [ ] **Step 3: Make it executable and run it (manifest absent yet → graceful)**

Run: `bash scripts/upstream-diff.sh`
Expected: prints merge-base + `(no manifest yet: ...)` and exits 0.

- [ ] **Step 4: Write the runbook**

Create `docs/superpowers/upstream/sync.md` with these exact contents:

```markdown
# Selective upstream sync (xberg-io/xberg)

Upstream is `xberg-io/xberg` (remote `upstream`). `origin` is the fork
`jamon8888/xberg`. Our additions are fenced: Tier-1 overlay (upstream never
touches) and Tier-2 carry-patches (tracked in `carry-patches.tsv`).

## Steps
1. `git fetch upstream main`
2. `bash scripts/upstream-diff.sh` — lists the Tier-2 files upstream changed.
   That list is your entire manual merge surface. Tier-1 needs no action.
3. Sync core:
   - single fix: `git cherry-pick <sha>`
   - broader: `git merge upstream/main` (resolve only reported Tier-2 files)
4. Resolve each reported Tier-2 file (see `carry-patches.md` per-file notes).
5. `cargo check -p <affected-crate>` for each crate whose files changed;
   run the crate's targeted tests.
6. If a carry-patch was absorbed upstream, remove its row from
   `carry-patches.tsv` and its note from `carry-patches.md` in the same commit.

## Not covered here
Full `rc.24` catch-up is a separate, deliberate project. The fence makes it
affordable; do not do it as a side effect of a routine sync.
```

- [ ] **Step 5: Commit**

```bash
git add scripts/upstream-diff.sh docs/superpowers/upstream/sync.md
git commit -m "docs(upstream): add selective-sync runbook and diff script"
```

---

## Task 2: Carry-patch manifest + guard check

**Files:**
- Create: `docs/superpowers/upstream/carry-patches.tsv`
- Create: `docs/superpowers/upstream/carry-patches.md`
- Create: `scripts/check-carry-patches.sh`

**Interfaces:**
- Consumes: the merge-base and the co-modified file list established in the spec.
- Produces: `carry-patches.tsv` (authoritative machine list, columns `path\ttier\tfeature\tnote`); `scripts/check-carry-patches.sh` — run as `bash scripts/check-carry-patches.sh`; exits non-zero if any file that differs from the merge-base under the tracked crate roots is a `crates/xberg*` co-modification missing from the TSV.

- [ ] **Step 1: Write the manifest TSV**

Create `docs/superpowers/upstream/carry-patches.tsv` (tab-separated; header row first). Rows for every Tier-2 file from the spec:

```
path	tier	feature	note
crates/xberg/src/api/types.rs	carry	process-api	ApiState.rehydration_store field added; rest of Process* moved to api/rag/types.rs
crates/xberg/src/api/router.rs	carry	process-api	/v1/process + rehydrate route wiring + ApiState construction
crates/xberg/src/api/mod.rs	carry	process-api	pub mod rag; behind process-api
crates/xberg/src/api/handlers.rs	carry	process-api	handler bodies moved to api/rag/handlers.rs; keep upstream shape
crates/xberg/Cargo.toml	carry	multiple	fork features: ner-candle, ner-candle-wasm, redaction-rehydrate, doc-store-sqlite, process-api; deps: xberg-doc-store, xberg-gliner-candle, aes-gcm, scrypt
crates/xberg/src/text/redaction/engine.rs	carry	redaction-rehydrate	14 interleaved hunks through redact(); highest non-RAG merge risk
crates/xberg/src/text/ner/gline.rs	carry	ner-candle	12 interleaved hunks; GlineBackend + ensure_model changes
crates/xberg/src/text/redaction/patterns/mod.rs	carry	redaction	pattern registration
crates/xberg/src/text/redaction/patterns/iban.rs	carry	redaction	pattern tweak
crates/xberg/src/text/redaction/patterns/credit_card.rs	carry	redaction	pattern rewrite
crates/xberg/src/lib.rs	carry	multiple	module wiring
crates/xberg/src/engine/mod.rs	carry	multiple	hook lines
crates/xberg/src/engine/parsed.rs	carry	multiple	marker line
crates/xberg/src/engine/structured/chunk.rs	carry	structured	marker line
crates/xberg/src/engine/structured/citations.rs	carry	structured	marker line
crates/xberg/src/engine/structured/prompts.rs	carry	structured	marker lines
crates/xberg/src/engine/structured/rasterize.rs	carry	structured	marker line
crates/xberg/src/chunking/tokenizer_cache.rs	carry	chunking	one-line change
crates/xberg/src/core/config/mod.rs	carry	multiple	one-line change
crates/xberg-rag/Cargo.toml	carry	graph	graphqlite backend deps
crates/xberg-rag/src/query.rs	carry	graph	Graph RetrieveMode variant (HIGH RISK: upstream added scoring.rs)
crates/xberg-rag/src/backends/sqlite.rs	carry	graph	graphqlite dispatch
crates/xberg-rag/src/backends/memory.rs	carry	graph	graph parity
crates/xberg-rag/src/pipeline.rs	carry	graph	graph wiring
crates/xberg-rag/src/types.rs	carry	graph	graph types
crates/xberg-ffi/include/xberg.h	carry	ffi	FFI surface for additions
crates/xberg-ffi/cbindgen.toml	carry	ffi	header gen config
crates/xberg-ffi/src/lib.rs	carry	ffi	FFI exports for additions
crates/xberg-wasm/src/lib.rs	carry	wasm	integration point (14 new files are Tier-1 overlay, not here)
crates/xberg-wasm/Cargo.toml	carry	wasm	wasm feature wiring
crates/xberg-node/src/lib.rs	carry	node	integration point
crates/xberg-node/index.d.ts	carry	node	generated d.ts additions
```

- [ ] **Step 2: Write the human register**

Create `docs/superpowers/upstream/carry-patches.md`: a short intro ("These files are edited by both the fork and upstream. `carry-patches.tsv` is authoritative; this file adds resolution guidance.") followed by an `## On resync` subsection per HIGH-RISK entry: `crates/xberg-rag/src/query.rs` (reconcile `Graph` variant against upstream's `RetrieveMode` + new `scoring.rs`), `redaction/engine.rs`, `ner/gline.rs`. One paragraph each describing what our change does and what to watch for.

- [ ] **Step 3: Write the guard check**

```bash
#!/usr/bin/env bash
# Fail if a co-modified crates/xberg* file drifted from merge-base but is absent from the TSV.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
TSV="$ROOT/docs/superpowers/upstream/carry-patches.tsv"
MB="$(git merge-base HEAD upstream/main 2>/dev/null || true)"
if [ -z "$MB" ]; then echo "no upstream merge-base; skipping"; exit 0; fi
tracked="$(cut -f1 "$TSV" | tail -n +2 | sort -u)"
missing=0
while IFS= read -r f; do
  case "$f" in
    crates/xberg/src/api/rag/*) continue ;;                 # Tier-1 extracted module
    crates/xberg/*|crates/xberg-rag/*|crates/xberg-ffi/*|crates/xberg-wasm/src/lib.rs|crates/xberg-wasm/Cargo.toml|crates/xberg-node/src/lib.rs|crates/xberg-node/index.d.ts)
      if ! printf '%s\n' "$tracked" | grep -qxF "$f"; then
        echo "UNTRACKED co-modified file: $f"; missing=1
      fi ;;
  esac
done < <(git diff --name-only "$MB" HEAD)
[ "$missing" -eq 0 ] && echo "carry-patch manifest OK" || { echo "add the above to carry-patches.tsv"; exit 1; }
```
Write to `scripts/check-carry-patches.sh`.

- [ ] **Step 4: Run the guard; expect clean**

Run: `bash scripts/check-carry-patches.sh`
Expected: `carry-patch manifest OK` (all currently-drifted tracked files are listed). If it reports UNTRACKED files, add them to the TSV and re-run until clean. (This doubles as verification that the manifest is complete.)

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/upstream/carry-patches.tsv docs/superpowers/upstream/carry-patches.md scripts/check-carry-patches.sh
git commit -m "docs(upstream): track Tier-2 carry-patch files with guard check"
```

---

## Task 3: Fence the workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml` (`members`, `[workspace.dependencies]`, `[patch.crates-io]`)

**Interfaces:**
- Consumes: fork-only crate list from Global Constraints.
- Produces: an append-only fork region so upstream `Cargo.toml` merges never touch fork members. No behavior change — purely reordering + comment fences.

- [ ] **Step 1: Capture current resolution as baseline**

Run: `cargo metadata --format-version 1 --no-deps | python -c "import sys,json;print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"`
Save the printed list (this is the invariant Step 4 checks against).

- [ ] **Step 2: Reorder `members` into upstream + fork blocks**

Edit `Cargo.toml` `members` so upstream crates keep upstream ordering, and the four fork-only crates plus fork rust packages move to a fenced trailing block:

```toml
members = [
    # === upstream (xberg-io/xberg) — keep in upstream order; do not interleave ===
    "crates/xberg",
    "crates/xberg-candle-ocr",
    "crates/xberg-cli",
    "crates/xberg-ffi",
    "crates/xberg-gliner",
    "crates/xberg-jni",
    "crates/xberg-libheif",
    "crates/xberg-node",
    "crates/xberg-paddle-ocr",
    "crates/xberg-php",
    "crates/xberg-py",
    "crates/xberg-rag",
    "crates/xberg-tesseract",
    "crates/xberg-wasm",
    "packages/dart/rust",
    "packages/swift/rust",
    "tools/benchmark-harness",
    # === fork overlay (absent from xberg-io) — APPEND-ONLY; upstream merges never touch below ===
    "crates/xberg-doc-store",
    "crates/xberg-gliner-candle",
    "crates/xberg-pdfium-render",
    "crates/xberg-rag-node",
]
```

- [ ] **Step 3: Fence fork entries in `[workspace.dependencies]` and `[patch.crates-io]`**

In `[workspace.dependencies]`, group the fork path-deps (`xberg-doc-store`, `xberg-gliner-candle`, `xberg-rag`* keep upstream ones in place) — move the fork-only ones (`xberg-doc-store`, `xberg-gliner-candle`) under a `# === fork overlay deps ===` comment at the end of the table. Do the same in `[patch.crates-io]` for `xberg-doc-store`, `xberg-candle-ocr` if fork-only. Leave upstream-present entries (`xberg`, `xberg-gliner`, `xberg-rag`, `xberg-tesseract`, `xberg-paddle-ocr`) untouched in place.

- [ ] **Step 4: Verify identical resolution**

Run: `cargo metadata --format-version 1 --no-deps | python -c "import sys,json;print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"`
Expected: byte-identical to the Step 1 list. Also run `cargo check -p xberg-cli` and expect success (no resolution regression).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml
git commit -m "chore(workspace): fence fork crates into append-only overlay block"
```

---

## Task 4: Extract the process/rehydrate API into a fork module

**Files:**
- Create: `crates/xberg/src/api/rag/mod.rs`, `crates/xberg/src/api/rag/handlers.rs`, `crates/xberg/src/api/rag/types.rs`
- Modify: `crates/xberg/src/api/handlers.rs`, `crates/xberg/src/api/types.rs`, `crates/xberg/src/api/router.rs`, `crates/xberg/src/api/mod.rs`, `crates/xberg/Cargo.toml`
- Test: `crates/xberg/tests/api_route_parity.rs`

**Interfaces:**
- Consumes: existing `ApiState` (in `api/types.rs`), `xberg_doc_store::{RehydrationStore, rehydration_store_from_env}`.
- Produces: fork feature `process-api = ["api", "redaction-rehydrate", "dep:xberg-doc-store"]`; module `crate::api::rag` exposing `pub fn routes() -> axum::Router<ApiState>` plus `process_handler`, `rehydrate_handler`, and the `Process*`/`Rehydrate*` types. `ApiState.rehydration_store` field remains in `api/types.rs` (tracked carry-patch). Route paths unchanged: `POST /v1/process`, `POST /v1/documents/{rehydration_key}/rehydrate`.

- [ ] **Step 1: Write the failing route-parity + boundary test**

Create `crates/xberg/tests/api_route_parity.rs`. The test must make a **real assertion** that proves the two fork routes are matched by the extracted module's router — not merely that a router was constructed. Preferred form: drive the router with `tower::ServiceExt::oneshot` and assert `/v1/process` is routed (status **not** `404 NOT_FOUND`; a `400`/`405`/`415` on an empty/malformed body still proves the path matched). Build the required `ApiState` by reusing whatever construction helper the existing api tests already use — grep `crates/xberg/tests` and `crates/xberg/src/api` for an existing `ApiState { .. }` test builder and reuse it.

```rust
// Verifies the fork API extraction preserves the /v1/process route in the fenced module.
#![cfg(feature = "process-api")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // oneshot

// NOTE: `test_api_state()` reuses the existing api-test ApiState builder.
// If no such helper exists, the implementer adds a minimal one in a shared
// test-support module rather than duplicating construction.

#[tokio::test]
async fn process_route_is_matched_by_rag_module() {
    let app = xberg::api::rag::routes().with_state(test_api_state());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/process")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Route exists → not a 404. (Empty body may yield 400/415/422; all prove match.)
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}
```

**Fallback (only if `ApiState` genuinely cannot be constructed in a test):** replace the route test with a serde round-trip test on a moved type — deserialize a `ProcessRequest` JSON literal and assert its fields — and report in the task report that route coverage then rests on the Step 6 feature-matrix compile gate. Do **not** submit a test that asserts nothing.

- [ ] **Step 2: Add the `process-api` feature and run test to confirm it fails to compile**

In `crates/xberg/Cargo.toml` `[features]` add:

```toml
# Fork-only: extract-and-fence the /v1/process + rehydrate API surface.
process-api = ["api", "redaction-rehydrate", "dep:xberg-doc-store"]
```

Run: `cargo test -p xberg --features process-api --test api_route_parity`
Expected: FAIL — `module `rag` is private` / `no function `routes``. Confirms the test drives the extraction.

- [ ] **Step 3: Create the fork module files**

`crates/xberg/src/api/rag/mod.rs`:

```rust
//! Fork-only API surface (absent from xberg-io/xberg): the /v1/process
//! pipeline (extract → NER → redact) and encrypted rehydration.
//! Fenced here so upstream `api/handlers.rs` and `api/types.rs` stay
//! byte-close to upstream. Gated on the `process-api` feature.
pub mod handlers;
pub mod types;

use axum::routing::post;
use axum::Router;
use super::types::ApiState;

/// Routes owned by the fork. Merged into the main router by `router.rs`.
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/process", post(handlers::process_handler))
        .route(
            "/v1/documents/{rehydration_key}/rehydrate",
            post(handlers::rehydrate_handler),
        )
}
```

`crates/xberg/src/api/rag/types.rs`: move the `ProcessRequest`, `ProcessOperations`, `ProcessRedactOperation`, `ProcessResponse`, `RehydrateRequest`, `RehydrateResponse` struct definitions verbatim out of `api/types.rs` into this file, preserving their `use`/path references (`crate::core::config::...`, `crate::types::ExtractedDocument`). Add `use` re-exports as needed.

`crates/xberg/src/api/rag/handlers.rs`: move the bodies of `process_handler` and `rehydrate_handler` verbatim from `api/handlers.rs`, updating imports to `use super::types::{ProcessRequest, ...};` and `use crate::api::types::ApiState;`.

- [ ] **Step 4: Remove moved symbols from upstream-shaped files; wire the module**

- In `crates/xberg/src/api/handlers.rs`: delete the two moved handler fns (the 3 appended hunks). Leave the file otherwise identical to upstream.
- In `crates/xberg/src/api/types.rs`: delete the moved `Process*`/`Rehydrate*` structs. **Keep** the `pub rehydration_store: Arc<dyn xberg_doc_store::RehydrationStore>` field on `ApiState` (tracked carry-patch), gated:

```rust
#[cfg(feature = "process-api")]
pub rehydration_store: std::sync::Arc<dyn xberg_doc_store::RehydrationStore>,
```

- In `crates/xberg/src/api/mod.rs` add:

```rust
#[cfg(feature = "process-api")]
pub mod rag;
```

- In `crates/xberg/src/api/router.rs`: change the moved-handler imports to come from `rag`, gate the `ApiState { rehydration_store, .. }` construction and the two `.route(...)` lines behind `#[cfg(feature = "process-api")]`, and merge the fork routes:

```rust
#[cfg(feature = "process-api")]
let router = router.merge(super::rag::routes());
```

Remove the two inline `.route("/v1/process", ...)` / `.route(".../rehydrate", ...)` lines (now provided by `rag::routes()`).

- [ ] **Step 5: Run the parity test — expect pass**

Run: `cargo test -p xberg --features process-api --test api_route_parity`
Expected: PASS.

- [ ] **Step 6: Feature-matrix compile check (no regressions)**

Run each and expect success:
```bash
cargo check -p xberg --no-default-features
cargo check -p xberg --features api
cargo check -p xberg --features process-api
cargo check -p xberg --no-default-features --features wasm-target
```
Expected: all compile. `--features api` (upstream-shaped, without `process-api`) must NOT reference `rag`/`rehydration_store` — proves the fence.

- [ ] **Step 7: Commit**

```bash
git add crates/xberg/src/api/rag crates/xberg/src/api/handlers.rs crates/xberg/src/api/types.rs crates/xberg/src/api/router.rs crates/xberg/src/api/mod.rs crates/xberg/Cargo.toml crates/xberg/tests/api_route_parity.rs
git commit -m "refactor(api): fence /v1/process + rehydrate into process-api module"
```

- [ ] **Step 8: Update the manifest to reflect the extraction**

Edit `docs/superpowers/upstream/carry-patches.tsv`: change the `crates/xberg/src/api/handlers.rs` note to "upstream-shaped again; process handlers live in api/rag/". Run `bash scripts/check-carry-patches.sh` → expect `carry-patch manifest OK`. Commit:

```bash
git add docs/superpowers/upstream/carry-patches.tsv
git commit -m "docs(upstream): note api handlers re-fenced to api/rag"
```

---

## Task 5: Correct the stale `upstream-sync` AI-rulez rule

**Files:**
- Modify: `.ai-rulez/` upstream-sync rule source (exact path found in Step 1)
- Modify (generated, via generator only): `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`

**Interfaces:**
- Consumes: the corrected facts (upstream = xberg-io/xberg; origin = fork; RAG crate is upstream; Tier-1/Tier-2 sets).
- Produces: regenerated governance docs consistent with the fence.

- [ ] **Step 1: Locate the rule source**

Run: `grep -rl "upstream-sync\|mirrors origin/main\|jamon8888/xberg" .ai-rulez/`
Expected: one or more source files under `.ai-rulez/`. Open the one defining the `upstream-sync` rule.

- [ ] **Step 2: Rewrite the rule body**

Replace the stale content with: upstream = `xberg-io/xberg` (remote `upstream`); `origin` = `jamon8888/xberg` (the fork); `main` on the fork is NOT an upstream mirror; RAG (`crates/xberg-rag`) is upstream, our additions are `xberg-rag-node`/`xberg-doc-store`/`xberg-gliner-candle`/`xberg-pdfium-render` + `mcp-server` + the `process-api` module + graphqlite backend; point to `docs/superpowers/upstream/{sync.md,carry-patches.md}`; state the append-only `Cargo.toml` fence rule. Keep the "never edit CLAUDE.md directly" and "regenerate after editing rules" guidance.

- [ ] **Step 3: Regenerate and verify**

Run:
```bash
ai-rulez generate
task ai-rulez:generate
git diff --stat CLAUDE.md AGENTS.md GEMINI.md
```
Expected: the three generated files show the corrected upstream-sync section; no unrelated churn.

- [ ] **Step 4: Commit source + generated together**

```bash
git add .ai-rulez/ CLAUDE.md AGENTS.md GEMINI.md
git commit -m "docs(ai-rulez): correct upstream-sync to xberg-io + fence model"
```

---

## Task 6: End-to-end verification & sync dry-run

**Files:** none (verification only)

**Interfaces:**
- Consumes: everything above.
- Produces: recorded evidence the fence holds and the sync workflow works.

- [ ] **Step 1: Wasm smoke build**

Run: `cargo check -p xberg-wasm --no-default-features --features wasm-target`
Expected: success (the 14 additive files + `ner-candle-wasm` + the `lib.rs` carry-patch compile).

- [ ] **Step 2: Guard + manifest consistency**

Run: `bash scripts/check-carry-patches.sh`
Expected: `carry-patch manifest OK`.

- [ ] **Step 3: Sync dry-run on a throwaway branch**

Pick one trivial upstream-only core commit that does NOT touch any carry-patch file:
```bash
git fetch upstream main
bash scripts/upstream-diff.sh > /tmp/xberg-diff.txt; cat /tmp/xberg-diff.txt
sha=$(git log upstream/main --oneline -- crates/xberg/src/core | head -1 | awk '{print $1}')
git switch -c tmp/sync-dryrun
git cherry-pick -n "$sha" || true      # -n: stage only, no commit
git cherry-pick --abort 2>/dev/null || git reset --hard HEAD
git switch -   && git branch -D tmp/sync-dryrun
```
Expected: `upstream-diff.sh` prints a bounded list; the cherry-pick stages without touching Tier-1 overlay paths. Record the printed carry-patch count.

- [ ] **Step 4: Lint pass**

Run: `prek run --all-files`
Expected: clean (re-stage and re-run if hooks rewrite files).

- [ ] **Step 5: Final commit if anything was restaged by lint**

```bash
git add -A
git commit -m "chore: lint fixups for fence foundation" || echo "nothing to commit"
```

---

## Self-Review notes (author)

- **Spec coverage:** 4.1→Task 3; 4.2→Task 4; 4.3→Task 2; 4.4→Task 1; 4.5→Task 5; §7 verification→Task 6. All covered.
- **Extraction boundary corrected from spec:** `api/types.rs` is not fully extractable — `ApiState.rehydration_store` is an interleaved field and stays a tracked carry-patch (reflected in Task 4 Step 4 and the TSV). The spec's "return handlers.rs/types.rs to byte-identical" is refined: `handlers.rs` returns upstream-shaped; `types.rs` retains one gated field.
- **Feature name:** `process-api` used consistently (Tasks 2, 4). Depends on real fork features `redaction-rehydrate` + dep `xberg-doc-store` verified present in `crates/xberg/Cargo.toml`.
- **No catch-up merge:** Task 6 dry-run uses `cherry-pick -n` + abort/reset on a throwaway branch, honoring the Global Constraint.
