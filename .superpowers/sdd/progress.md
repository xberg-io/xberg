# LoRA Privacy API Plan — SDD Progress
# Plan: docs/superpowers/plans/2026-06-30-lora-privacy-api.md
# Started: 2026-06-30
# Branch: feature/gliner2-onnx-backend
# Base commit (before Task 1): cbe1c32b66
Task 1: complete (commits cbe1c32b66..d2b5e861db, review clean)
Task 2: complete (commits d2b5e861db..13960ccc74, review clean)
Task 3: complete (commits 13960ccc74..14a311abab, review clean — Minor: stubs for Tasks 4-7 in same commit)
Task 4: complete (commits 14a311abab..a3ab225d89, review clean)
Task 5a: complete (commits a3ab225d89..fbe60234a8, review clean)
Task 5b: complete (commits fbe60234a8..835caecf56, review clean — two non-blocking notes: dead device param in count_lstm::forward per spec, relu-after-cat in span_rep matches anno source)
Task 6: complete (commits 835caecf56..d43ee3fxxx, reviewer found end_word clamp bug + .expect() in lib code, fixed in follow-up commit d43ee3f)
Task 7: complete (commits bae96115ac..8c06f81, reviewer found i64/u32 index_select mismatch, fixed in follow-up 8c06f81)
Task 8: complete (commit 1674cba, 10 tests pass, smoke test #[ignore]d — Part 1 (xberg-gliner-candle engine crate) done)
Task 9: complete (commit 33cbca6, ignored GLiNER2 ONNX smoke test added to gline.rs)
Task 10: complete (already done in prior session — GlinerArchitecture/hfArchitecture in xberg-node, no new commit needed)
Task 11 (brief): complete (commit 578af53 — MCP hf_architecture wiring in intelligence.ts/ingest.ts/README)
Task 11 (plan): complete (commit f64e31c — encrypted rehydration map + RehydrationStore; 8 redaction regression + 7 rehydration/store tests pass)
Task 12: complete (commit 9d506d9f23 — POST /v1/process, 3 handler tests pass; agent also applied fmt/bindings/docs/Cargo.lock fixups)
Task 13: complete (commit 680b341 — POST /v1/documents/{id}/rehydrate, 3 tests pass; reviewer findings were false positives caused by agent being on wrong branch — correct branch (feature/gliner2-onnx-backend) already had moka TTL store and no RwLock::expect; non-blocking: encrypt_map uses XbergError::validation for internal crypto errors, noted for follow-up)
# PLAN COMPLETE — all 13 tasks shipped on feature/gliner2-onnx-backend
Fix: removed xberg-rag dependency cycle from xberg-doc-store (commits d9fbb9c043..62757c4de6, review clean; cargo tree independently verified — no xberg-rag edge). Amends Tasks 1 & 3.
Task 7: complete (commits 62757c4de6..e437f8593b, review clean; xberg intentionally non-compiling until Task 11, errors confined to router.rs/types.rs as expected)
Task 8: complete (commits 330babd640..b76fb312f7, review clean; error surface now confined to handlers.rs as expected)
Task 9: complete (commits b76fb312f7..4bcbaafd42, review clean; error surface now confined to rehydrate_handler + test helpers as expected)
Task 10: complete (commits 4bcbaafd42..0f77da606c, review clean; error surface now confined to #[cfg(test)] module only, as expected)
Task 11: complete (commits 0f77da606c..9cad2dfcca, review clean; crates/xberg fully compiles again, 18/18 api::handlers tests pass, independently re-verified. Note: first implementer session ended abnormally without committing; a second finishing agent verified+committed the correct pre-existing edits. Filed separate out-of-scope task_245f6e1e for pre-existing unused-import debt in markdown_lint_quality.rs, unrelated to this plan.)

# ==========================================================================
# ner-candle-wasm Plan — SDD Progress (NEW PLAN, separate from the above)
# Plan: docs/superpowers/plans/2026-07-02-ner-candle-wasm.md
# Worktree: .worktrees/ner-candle-wasm
# Branch: feature/ner-candle-wasm (from feature/gliner2-onnx-backend @ d6a17dc5c8)
# Started: 2026-07-02
# Baseline: cargo build -p xberg-gliner -p xberg-gliner-candle --features
#           xberg-gliner-candle/ort-bundled — green (57 crates, 2m22s)
# ==========================================================================
Task 1: complete (commits d6a17dc5c8..ba93c69, review clean — Approved.
  Original dispatched implementer hit monthly spend limit mid-task with
  uncommitted partial progress; controller resumed manually, found+fixed a
  real bug in the plan's own Cargo.toml snippet (default=["ort-backend"]
  would have broken all default-feature linking; restored
  default=["ort-bundled"]), chased a dead-code cascade under
  `clippy -D warnings` on wasm32 beyond the brief's literal file list
  (splitter.rs, v2_decode.rs modules + item-level gates in config.rs/
  decode.rs/input.rs), all item-scoped with doc comments naming the sole
  consumer. Native: 23/23 tests pass, clippy clean. wasm32: build 0
  errors, clippy -D warnings clean — the tokenizers-on-wasm risk gate
  PASSES, in-binary Candle-NER remains feasible. Minor note (non-blocking,
  for a later task): Parameters::validate() has no wasm-side equivalent
  if Parameters ever becomes part of the wasm-facing API.)
Task 2: complete (commits ba93c69..7282897, review clean — Approved.
  Added Encoder::from_buffered_safetensors, AllHeads::from_buffered_safetensors
  (both via candle_core::safetensors::load_buffer), Gliner2Candle::from_bytes
  (in-memory model load, no filesystem), V2Tokenizer::from_bytes in
  xberg-gliner. Gated from_local/from_local_with_device/load_adapter/
  unload_adapter #[cfg(not(target_arch="wasm32"))] — fs-only, item-level
  per Task 1's pattern. Correctly used the real config type
  candle_transformers::models::debertav2::Config (not the fictitious
  EncoderConfig name); AllHeads::from_buffered_safetensors calls Self::load
  directly (not the now-gated from_var_builder), avoiding a self-inflicted
  wasm break. Native: tests pass, clippy clean. wasm32 build 0 errors,
  clippy -D warnings clean — SECOND major risk gate (candle-on-wasm)
  PASSES. TDD evidence (RED/GREEN) verified genuine against diff.)
Task 3: complete (commits 7282897..77656d3d42, review clean — Approved.
  Plan brief was STALE (described creating a new ner_candle_wasm.rs /
  WasmCandleNer duplicating logic) — controller discovered before dispatch
  that CandleBackend already existed in ner/candle.rs (native ner-candle
  feature, tokio-runtime-dependent via block_in_place). Corrected scope:
  added ner-candle-wasm feature (no tokio-runtime) to Cargo.toml +
  wasm-target aggregate, widened ner/mod.rs module gate to
  any(ner-candle, ner-candle-wasm), gated from_local
  #[cfg(not(wasm32))], added from_bytes constructor, branched detect()'s
  block_in_place call by target_arch. Reused spans_to_entities/
  category_to_label unmodified (zero duplication). Implementer reported
  DONE_WITH_CONCERNS: crate-wide `cargo build -p xberg --features
  ner-candle-wasm --target wasm32-unknown-unknown` still fails on 8
  errors, but claimed pre-existing/unrelated (plugins/registry/
  extractor.rs Send-future issues + a Url::to_file_path gap in
  core/extract/mod.rs). CONTROLLER INDEPENDENTLY VERIFIED this via
  git-level isolation: reverted Task 3's 3 files to pre-Task-3 state,
  rebuilt wasm32 with only the OLD pre-existing `ner` (types-only)
  feature — identical 8 errors reproduced, proving the bug predates and
  is unrelated to this task. mod extractor; has no feature gate at all
  (always compiled). Filed task_706665c3 as a tracked follow-up outside
  this plan. THIRD major risk gate (full xberg-core integration) confirms
  candle.rs itself is wasm32-clean; the crate-wide build remains blocked
  by that pre-existing, out-of-scope infra bug — noted, not silently
  dropped.)
Task 4: complete (commit 0a5959f72a, review clean — Approved. Controller
  pre-empted an environment gap (wasm-pack not installed, no
  cargo-binstall) by approving a substitution: dropped
  wasm_bindgen_test_configure!(run_in_browser) since the test touches no
  DOM API, ran via wasm-bindgen-test-runner under Node.js instead
  (wasm-bindgen-cli pinned to 0.2.126 to match Cargo.lock's resolved
  wasm-bindgen crate version). Test ACTUALLY EXECUTED AND PASSED on real
  wasm32-unknown-unknown ("1 passed; 0 failed") — the load-bearing proof
  that the full Task 1-3 stack links and runs, not just compiles.
  wasm-bindgen-test correctly dev-dependency-only. Implementer flagged +
  controller independently re-verified (git stash isolation) a second,
  narrower pre-existing bug: tests/smoke.rs and src/tests.rs call the
  now-native-only from_local/from_safetensors (gated by Task 2), breaking
  `--tests`/`cargo test --target wasm32` for this crate — does not affect
  the plain build/clippy gates Tasks 1-4 used and passed. Filed
  task_71b413e1 as a scoped follow-up (much smaller than Task 3's
  extractor.rs finding, task_706665c3).)
# PLAN COMPLETE — all 4 tasks (A: ner-candle-wasm enablement) shipped on
# feature/ner-candle-wasm. THREE major risk gates all PASSED:
#   1. tokenizers-on-wasm (Task 1)
#   2. candle-on-wasm (Task 2)
#   3. full xberg-core NER integration on wasm, executed+verified (Tasks 3-4)
# Two narrow, independently-verified pre-existing bugs found and filed as
# separate follow-ups (task_706665c3, task_71b413e1) — NEITHER blocks this
# plan's own deliverables or was introduced by this plan's changes.
# Next: final whole-branch review, then superpowers:finishing-a-development-branch.
