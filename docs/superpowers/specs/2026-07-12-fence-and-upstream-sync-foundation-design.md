# Project A — Fence & Upstream-Sync Foundation

**Date:** 2026-07-12
**Status:** Design (awaiting review)
**Scope:** Sub-project A of a 3-part revamp (A: fence & sync foundation · B: RAG bindings for Python + Node only · C: CI simplification). B and C are out of scope here and get their own specs.

## 1. Background & corrected premise

The working repo (`C:\Users\NMarchitecte\xberg`, `origin` = `github.com/jamon8888/xberg`) is a fork carrying local additions on top of the public project `xberg-io/xberg`. The original request assumed "RAG is our pure addition" and "core is a clean upstream mirror." Direct investigation of `xberg-io/xberg` disproved both:

- **`crates/xberg-rag` is already upstream.** So is `xberg-gliner` and `xberg-candle-ocr`. RAG-the-crate is not our addition.
- **Upstream ships RAG in no language binding.** `xberg-py` and `xberg-node` upstream have no RAG dependency. Binding RAG into Python/Node is genuinely our value-add (that is Project B).
- **The fork is a live 8-day split, not stale.** Merge-base `e702310938` (2026-07-04). Upstream added **198** commits since; the fork added **292**. Fork is `1.0.0-rc.5`, upstream is `1.0.0-rc.24`.
- **Our `xberg-rag` has diverged from upstream's**: we added `backends/graphqlite.rs` + a `Graph` `RetrieveMode` variant; upstream added `scoring.rs` we lack.
- **The stale `upstream-sync` guidance in `.ai-rulez/` (rendered into `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`) is wrong** — it names `jamon8888/xberg` as upstream and claims core is unmodified. This must be corrected.

### Fork-only crates & packages (verified: present in fork, absent from `xberg-io/main`)

- Crates: `xberg-rag-node`, `xberg-doc-store`, `xberg-gliner-candle`, `xberg-pdfium-render`
- Packages: `packages/xberg-wasm-runtime`, `packages/xberg-web-ui`
- Root dirs: `mcp-server/`, `docs/superpowers/`

### Co-modified files (both fork and upstream edited the same file since the merge-base)

The **entire** real upstream-resync conflict surface, by nature of edit:

| File | Fork diff | Edit nature | Tier |
|---|---|---|---|
| `crates/xberg/src/api/handlers.rs` | +335/−0 | 3 appended blocks | **Extract** |
| `crates/xberg/src/api/types.rs` | +98/−0 | appended | **Extract** |
| `crates/xberg/src/text/redaction/engine.rs` | +559/−20 | 14 hunks interleaved through `redact()` | **Carry-patch** |
| `crates/xberg/src/text/ner/gline.rs` | +326/−37 | 12 hunks through `ensure_model`/`GlineBackend` | **Carry-patch** |
| `crates/xberg/src/api/router.rs` | +8/−1 | registration hook | Carry-patch (1-liner) |
| `crates/xberg/src/lib.rs` | +5/−4 | module wiring | Carry-patch (1-liner) |
| `crates/xberg/src/engine/mod.rs` | +6/−0 | hook | Carry-patch (1-liner) |
| `crates/xberg/src/text/redaction/patterns/mod.rs` | +6/−3 | registration | Carry-patch (1-liner) |
| `crates/xberg/src/text/redaction/patterns/iban.rs` | +44/−4 | pattern tweak | Carry-patch |
| `crates/xberg/src/text/redaction/patterns/credit_card.rs` | +17/−32 | pattern rewrite | Carry-patch |
| `crates/xberg/src/{engine/parsed.rs, engine/structured/{chunk,citations,prompts,rasterize}.rs, chunking/tokenizer_cache.rs, core/config/mod.rs}` | 1–2 lines each | markers/hooks | Carry-patch (trivial) |
| `crates/xberg/Cargo.toml` | +19/−2 | deps/features | Carry-patch |
| `crates/xberg-rag/{Cargo.toml, src/backends/sqlite.rs, memory.rs, pipeline.rs, query.rs, types.rs}` | (6 files) | graphqlite + Graph variant woven in | Carry-patch |
| `crates/xberg-ffi/{include/xberg.h, cbindgen.toml, src/lib.rs}` | (3 files) | FFI surface for additions | Carry-patch |
| `crates/xberg-wasm/{src/lib.rs, Cargo.toml}` | (2 files) | integration points | Carry-patch |
| `crates/xberg-node/{src/lib.rs, index.d.ts}` | (2 files) | integration points | Carry-patch |

Note: `crates/xberg-wasm` also contains **14 fork-only new files** — additive, not a conflict surface. Wasm work is overwhelmingly additive.

## 2. Goals

1. Establish a **two-tier fence** so upstream-owned files stop mixing with fork additions, and future `xberg-io/xberg` resyncs are a bounded, documented merge.
2. **Extract** the two purely-additive API files into a fork-owned module, removing them from the conflict surface.
3. **Track** the remaining co-modified files in an explicit, machine-checkable carry-patch manifest.
4. Add `xberg-io/xberg` as the `upstream` remote and define a **selective-sync workflow**.
5. **Correct** the stale `upstream-sync` documentation at its `.ai-rulez/` source and regenerate.
6. Do all of the above **on the current `rc.5` base** — no upstream catch-up merge in this project (that is a deliberate future project, made cheap by this fence).

## 3. Non-goals (explicitly out of scope for Project A)

- Merging upstream `rc.24` (the 198-commit catch-up). Deferred; the fence is what makes it affordable.
- Adding/altering RAG language bindings (Project B).
- CI/workflow simplification (Project C).
- Refactoring `redaction/engine.rs` or `ner/gline.rs` behind plugin seams — rejected: they are interleaved through upstream function bodies that upstream also actively edits, so a seam needs upstream buy-in and would fight their churn.

## 4. Architecture — the two-tier fence

```
xberg-io/xberg  (upstream remote)  ──selective sync──▶  fork core (rc.5)
                                                          │
      ┌───────────────────────────────────────────────────┤
      │ TIER 1 — OVERLAY (clean; upstream never touches)   │
      │   fork-only crates:  xberg-rag-node, xberg-doc-store,
      │                      xberg-gliner-candle, xberg-pdfium-render
      │   fork-only packages: xberg-wasm-runtime, xberg-web-ui
      │   root dirs:          mcp-server/, docs/superpowers/
      │   extracted module:   crates/xberg/src/api/rag/ (from handlers.rs+types.rs)
      │   wasm additive:      crates/xberg-wasm/src/<14 new files>
      ├───────────────────────────────────────────────────┤
      │ TIER 2 — CARRY-PATCH (tracked; manual merge on resync) │
      │   redaction/engine.rs, ner/gline.rs                 │
      │   ~15 one-line hooks (router.rs, lib.rs, engine/mod.rs, …) │
      │   xberg-rag graphqlite+Graph (6 files)              │
      │   ffi/wasm/node integration points                  │
      └───────────────────────────────────────────────────┘
```

### Component 4.1 — Workspace fence (append-only overlay block)

`Cargo.toml` `members` currently interleaves upstream and fork crates alphabetically, guaranteeing merge conflicts. Change: keep the upstream member list in **upstream order** and move all **fork-only** members into a clearly delimited, append-only block at the end:

```toml
members = [
    # === upstream (xberg-io/xberg) — keep in upstream order, do not interleave ===
    "crates/xberg", "crates/xberg-candle-ocr", "crates/xberg-cli", ...
    # === fork overlay (xberg-io absent) — append-only; upstream merges never touch below ===
    "crates/xberg-doc-store",
    "crates/xberg-gliner-candle",
    "crates/xberg-pdfium-render",
    "crates/xberg-rag-node",
]
```

Same treatment for `[workspace.dependencies]` and `[patch.crates-io]` fork entries (fenced comment block). This converts `Cargo.toml` from a guaranteed-conflict file into an append-only one: upstream edits the top region, fork owns the bottom region.

### Component 4.2 — Extract additive API surface

- Move the 3 appended blocks of `api/handlers.rs` (+335) and all of `api/types.rs` (+98) into a new fork-owned module `crates/xberg/src/api/rag/` (`mod.rs`, `handlers.rs`, `types.rs`).
- Replace their inline presence with a **single registration hook**: `api/router.rs` gains one `.merge(rag::routes())` line; `api/mod.rs`/`lib.rs` gains `mod rag;` behind the fork feature flag.
- Gate the module on a fork-only cargo feature (e.g. `rag-api`) so upstream absence of the module never breaks a from-upstream build of core.
- Result: `handlers.rs` and `types.rs` return to byte-identical-to-upstream (modulo the one-line hook, which is a tracked carry-patch), removing the two largest files from the conflict set.

### Component 4.3 — Carry-patch manifest

A tracked file `docs/superpowers/upstream/carry-patches.md` (+ a checkable `carry-patches.tsv`) enumerating every Tier-2 file with: path, why it is patched, owning feature, and a short "on-resync" note. A helper script `scripts/upstream-diff.sh` reports, for a candidate upstream ref, which carry-patch files upstream touched — so a resync's manual surface is known before starting.

### Component 4.4 — Upstream remote & selective-sync workflow

- Register remote `upstream = https://github.com/xberg-io/xberg.git` (done in investigation; make permanent + documented). `origin` stays `jamon8888/xberg`.
- Documented workflow (`docs/superpowers/upstream/sync.md`):
  1. `git fetch upstream main`
  2. `scripts/upstream-diff.sh` → list of touched carry-patch files (the manual surface)
  3. For a core-only fix: `git cherry-pick <sha>` or a scoped `git merge` limited to core paths.
  4. Overlay (Tier 1) needs no action — upstream cannot touch it.
  5. Resolve only the reported Tier-2 files; re-run targeted build/tests for affected crates.
- This project does **not** execute a full sync; it establishes the machinery and verifies it on a trivial upstream cherry-pick as a smoke test.

### Component 4.5 — Fix stale `.ai-rulez/` documentation

Edit the `upstream-sync` source under `.ai-rulez/` (not `CLAUDE.md` directly — it is generated) to reflect: upstream = `xberg-io/xberg`; `jamon8888/xberg` = fork/`origin`; RAG crate is upstream; list the real Tier-1/Tier-2 sets; point to the new `docs/superpowers/upstream/` docs. Regenerate via `ai-rulez generate` + `task ai-rulez:generate`; commit `.ai-rulez/` + regenerated `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` together.

## 5. Data flow — a resync after the fence

```
fetch upstream/main
   └─▶ upstream-diff.sh
          ├─ Tier-1 overlay ........... untouched (no action)
          ├─ Tier-2 carry-patch ....... N files flagged → manual 3-way merge
          └─ pure upstream core ....... fast-forward / cherry-pick clean
   └─▶ targeted build+test of crates whose files changed
   └─▶ update carry-patches.md if a patch was absorbed/obsoleted upstream
```

## 6. Error handling & risks

- **Feature-gate correctness:** the extracted `rag-api` module must compile-guard cleanly so `--no-default-features` and upstream-shaped builds don't reference it. Verify with a `cargo check` matrix (default, no-default, `wasm-target`, `full`).
- **Behavior preservation on extraction:** moving handlers must not change routes or types. Guard with a before/after route-list assertion and existing API/e2e tests for the affected endpoints.
- **Wasm safety:** the 14 additive wasm files + `xberg-wasm/src/lib.rs` hook must keep `wasm-target` building (`SyncExtractor` constraints, no tokio). Verify `cargo check -p xberg-wasm --no-default-features --features wasm-target` and a `wasm-pack build` smoke.
- **graphqlite/Graph carry-patch drift:** upstream's `scoring.rs` + their `RetrieveMode` edits may collide with our `Graph` variant on a future sync. Documented in the manifest as the highest-risk Tier-2 item; not resolved now.
- **Manifest rot:** the manifest is only useful if kept current — enforced by a lightweight pre-commit/CI check (Project C will formalize) that fails if a Tier-2 file changes without a manifest entry.

## 7. Testing / verification

- `cargo check` feature matrix on `crates/xberg` and `crates/xberg-wasm` (default / no-default / wasm-target / full).
- Targeted `cargo test -p xberg --lib` for `api::rag` and redaction/NER modules touched.
- Route-parity assertion: extracted API exposes the identical route set as before extraction.
- `wasm-pack build crates/xberg-wasm` smoke.
- Dry-run the sync workflow: cherry-pick one trivial upstream core commit through the documented steps; confirm `upstream-diff.sh` output matches reality.
- `prek run --all-files` clean; regenerated AI-rulez docs match (`git diff --exit-code` after generate).

## 8. Deliverables

1. Fenced `Cargo.toml` (members/deps/patch append-only blocks).
2. `crates/xberg/src/api/rag/` extracted module + one-line registration hook, feature-gated.
3. `docs/superpowers/upstream/{sync.md, carry-patches.md, carry-patches.tsv}`.
4. `scripts/upstream-diff.sh`.
5. Permanent `upstream` remote + corrected `.ai-rulez/` `upstream-sync` rule and regenerated generated docs.
6. Verification evidence (feature-matrix check output, route-parity test, wasm smoke, sync dry-run).

## 9. What this unlocks

- **Project B** (RAG bindings, py+node only) builds on the fenced overlay: the new binding surface lands in Tier-1 crates and a feature on `xberg-py`/`xberg-node` tracked as a small carry-patch.
- **Project C** (CI simplification) can split core-CI vs overlay-CI along the exact Tier boundary defined here.
- **Future rc.24 catch-up** becomes a ~2-substantive-file merge instead of a 650-file reconciliation.
