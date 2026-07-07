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
Task 12: complete (commits 9cad2dfcca..21cb439878, review clean; end-to-end durability proven via real HTTP router rebuild against same SQLite file. Two brief bugs found+fixed by implementer: #[allow(unsafe_code)] required by workspace deny lint; JSON strategy key needed to be flat sibling not nested under config due to #[serde(flatten)] -- both verified independently, plan updated. Filed separate out-of-scope task_7d8b4a80 for pre-existing cross_format_parity.rs compile error.)
Task 13: complete (commit a43cc10beb, directly verified — trivial single-file doc change, no dispatched reviewer needed). All 13 plan tasks complete.
Final whole-branch review: clean, Ready to merge = Yes (commits ac2697e494..a43cc10beb). Zero Critical/Important-blocking issues. Two non-blocking design notes deferred to future DocumentStore plan (DocumentId reconciliation, create_router panic-vs-Result for embedders). Plan complete.

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

# ==========================================================================
# xberg-wasm-runtime-layer Plan — SDD Progress (Sub-project C)
# Plan: docs/superpowers/plans/2026-07-02-xberg-wasm-runtime-layer.md
# Worktree: .claude/worktrees/wasm-runtime-layer
# Branch: worktree-wasm-runtime-layer (from main @ cdefd83d8a)
# Started: 2026-07-06
# Pre-flight scan found 3 defects, resolved with user before Task 1:
#  1. wa-sqlite@^1.1.0 doesn't exist (only 1.0.0 ever published) -> pin ^1.0.0
#  2. ppu-paddle-ocr@^0.5.0 doesn't exist (lowest 1.0.0, latest 6.0.0) -> pin ^6.0.0
#  3. store.ts Task 4: `async function delete(...)` is a JS reserved-word
#     syntax error -> implement as deleteDocument, expose as `delete: deleteDocument`
#  Also: align vitest/typescript devDeps with workspace root (^4.1.9/^6.0.3)
#     instead of plan's stale ^1.0.0/^5.3.0, per user decision.
# Base commit (before Task 1): cdefd83d8a
# ==========================================================================
Task 1: complete (commits cdefd83d8a..f91e768f23, review Approved after 1 fix round.
  Implementer hit account monthly spend limit on first dispatch attempt with zero
  work done, re-dispatch succeeded. Reviewer found 3 Important issues: vitest v4
  coverage schema needed nesting under thresholds (implementer wrongly claimed it
  was verified), pnpm-lock.yaml regenerated but not committed, pnpm-workspace.yaml
  allowBuilds mutated with literal placeholder strings for onnxruntime-node/
  protobufjs/sharp. Fix subagent resolved all 3: onnxruntime-node=true and sharp=true
  (verified against their actual postinstall/install scripts pulling native
  binaries), protobufjs=false (pure JS, no install script) -- reasoning
  independently verified by re-reviewer against node_modules/.pnpm/*/package.json.
  Re-review: Approved, no new issues.)
Task 2: complete (commit f91e768f23..4fa1546ac8, review Approved.
  Note for final review: reviewer flagged 2 Important-but-inherited-from-plan
  weaknesses, non-blocking per reviewer's own assessment -- (a) z.function()
  zod schemas don't actually validate async-ness at parse time (only at
  invoke time via .implement()), so injectionDescriptorSchema accepts sync
  functions that violate the Promise-returning interface; (b) `as
  z.ZodType<InjectionDescriptor>` cast on injectionDescriptorSchema bypasses
  structural type-checking between the hand-written interface and the zod
  schema. Both present verbatim in the plan's own code, not implementer
  deviations.)
Task 3: complete (commit 4fa1546ac8..e2a0bc827c, review Approved.
  Plan's literal embedder.ts snippet was broken against real transformers.js
  v3 API (confirmed by implementer + independently re-verified by reviewer
  against installed type decls): quantized option doesn't exist, .data
  iteration was wrong shape (Tensor.data is flat row-major, needed .dims-based
  subarray slicing), test model ID Xenova/minilm-l6-v2 doesn't exist on HF Hub
  (corrected to Xenova/all-MiniLM-L6-v2). All three deviations verified
  correct. Live network model download in test accepted per user decision
  (overrides plan's "no network calls in CI" for this task only -- also
  applies to Tasks 5/6). Minor non-blocking notes: misleading comment on
  env.allowLocalModels (doesn't actually block remote fetch, allowRemoteModels
  does), CacheConfig.wasmPaths declared but unused by this module (future
  WASM-caching wiring task).)
Task 4: complete (commit e2a0bc827c..f11a975084, review Approved.
  delete/deleteDocument reserved-word fix verified correct: no syntax error,
  VectorStoreInterface.delete still satisfied externally. Non-blocking notes
  inherited from plan's own reference code (not implementer deviations): (a)
  upsertDocument overwrites the whole chunk array keyed by sourceId rather
  than a true per-(collectionId,sourceId,chunkIndex) merge -- only
  accidentally idempotent for single-chunk test case; (b) query's collection
  filter uses key.startsWith(`${collection}:`) which could prefix-collide
  across similarly-named collections (e.g. "docs" matching "docs-extra");
  (c) vectorBackend field is dead/hardcoded to "cosine", sqlite-vec toggle
  not implemented (expected -- future work per plan text); (d) no test for
  cosine dimension-mismatch throw path.)
Task 5: complete (commits f11a975084..001d171db0, review Approved after 1 fix
  round. Model substitution Xenova/gliner2-small-onnx (doesn't exist, zero-shot)
  -> Xenova/bert-base-NER (fixed PER/ORG/LOC/MISC labels) verified correct;
  offset-recovery and BIO-span merging logic (both needed since brief's
  snippet didn't actually implement them) independently verified correct by
  reviewer via type-decl inspection + simulated token streams. Fix round:
  documented the fixed-label-set constraint on categories filtering, fixed
  misleading allowLocalModels comment, added 3 real test assertions (merged
  multi-word entity text, categories filter, threshold filter). Re-review
  found fix commit had accidentally picked up a Co-Authored-By: Claude
  Sonnet 5 trailer, violating repo's no-ai-signatures critical rule --
  controller amended the commit message directly (content/tests unchanged,
  5/5 still passing) rather than dispatching another subagent round.
  SEPARATE INCIDENT during this task: C: drive hit 0 bytes free mid-review,
  breaking git read commands. Investigated with user's consent, found 11
  total worktrees on disk; only removed the 2 that were both clean AND
  verified (git merge-base --is-ancestor) fully merged into main
  (angry-edison-b34d9c, compassionate-agnesi-3771e7, both at a49d38c94c).
  Did NOT touch wasm-engine worktree despite looking similar (9 "ahead"
  commits) -- verified those commits are NOT merged into main, real
  divergent work. Freed ~5GB, disk now at 4.8GB free -- tight but workable,
  worth monitoring on later tasks.)
Task 6: complete (commit 001d171db0..0ff2647b5d, review Approved.
  Brief written against ppu-paddle-ocr@^0.5.0 (never existed); real installed
  dependency is ^6.0.0, a 6-major-version gap. Implementer investigated real
  v6 API (PaddleOcrService/.recognize(), not Paddle/.ocr()) -- reviewer
  independently confirmed every claimed API surface against the package's
  actual .d.ts files, not invented. Also found+fixed a real bug in the
  brief's own test pattern: it.skipIf(!ocr) evaluates at collection time
  before beforeAll runs, so it always skips regardless of actual OCR
  availability -- replaced with in-body guards matching ner.test.ts's
  pattern, reviewer confirmed this is correct vitest semantics.
  Non-blocking Important note for final review: synthetic canvas-rendered
  text fixture produces 0 detected text boxes across 2 models/2 engines
  (already exhaustively investigated by implementer, likely font-rasterization
  mismatch not a plumbing bug) -- test suite proves no-throw + correct types
  but not numerical OCR correctness. Reviewer explicitly recommended tracking
  as follow-up (needs a different test fixture) rather than blocking this
  task, since it's not a cheap fix. Minor notes: CacheConfig.models.ocr
  semantics (lookup key) differ undocumented from models.ner semantics (raw
  HF id); minor || vs ?? robustness nit.)
Task 7: complete (commits 0ff2647b5d..2173fae7f7, review Approved after 1 fix
  round. Pure fs/path module, no ML API risk. Fix round addressed 2 Important
  + 3 Minor: removed dead vectorBackend field/detection logic, distinguished
  ENOENT vs other fs errors in status() with warn logging for unexpected
  errors, replaced @ts-ignore with declare global Window.ort typing (repo
  convention forbids @ts-ignore), added no-op visibility log to
  setWasmPaths(), corrected docstring overstating OPFS capability. Re-review
  verified all 5 directly against source, no new issues, tsc --noEmit clean.
  No AI-attribution trailer this time (explicit warning included in fix
  dispatch after Task 5's incident).)
Task 8: complete (commit 2173fae7f7..4424961cd2, review clean Approved, no
  fixes needed. finally-block reset and non-flaky deterministic race test
  both independently verified against source. Only Minor style notes.)
Task 9: complete (commit 4424961cd2..b70f0da52d, review Approved, no fixes
  needed. MAJOR FINDING: discovered+fixed a real native SIGSEGV -- embedder/
  ner (transformers.js -> onnxruntime-web ~1.21) and ocr (ppu-paddle-ocr ->
  onnxruntime-web 1.27) crash the process when both load in one Node
  process, since createXbergRuntimeFactory always attempts ocr best-effort.
  Confirmed via isolated node repro independent of vitest (exit 139) both
  before and after. User chose "try pin first, fallback to docs if broken"
  -- pin succeeded: pnpm-workspace.yaml overrides pin onnxruntime-node to
  1.21.0 and onnxruntime-web to 1.22.0-dev.20250409-89f8206ba4 (matching
  what transformers.js already used). Reviewer independently verified: (a)
  the override is real and correctly scoped -- grepped every package.json
  in the worktree, only xberg-wasm-runtime depends on onnxruntime-*, no
  other workspace member affected; (b) lockfile diff shows exactly the
  expected mechanism (ppu-paddle-ocr's onnxruntime-web resolution moved to
  match the pin, old 1.27 entries removed entirely); (c) OCR not silently
  degraded to null post-pin -- verified via external log evidence
  ([factory] injection descriptor created (with NER) (with OCR)) and the
  isolated node repro, not just ocr.test.ts's own weak null-tolerant
  assertions (same weak-test pattern flagged in Task 6, pre-existing, not
  introduced here). Full suite independently re-run by controller: 14/14
  files, 50/50 tests pass, no crash. Commit message documents root cause
  clearly so a future maintainer won't strip the pin as "stale". Minor
  follow-up noted: factory.test.ts's combined ner+ocr test only asserts
  embedder/store presence, not ner/ocr presence directly -- could add a
  stronger assertion now that the crash is fixed.)
Task 10: implemented and committed (commit f2cfd0774d), independently
  verified by controller (5/5 tests pass, no AI-attribution trailer). Peer
  review dispatch failed immediately -- session hit monthly spend limit
  before the reviewer could do any work (0 real tool calls, empty result).
  NOT YET PEER-REVIEWED. Session paused here per user's "finalize" request
  after hitting the spend limit.

SESSION PAUSED (spend limit reached). Status: Tasks 1-9 complete and
  reviewer-approved (Task 9 fixed a major cross-cutting ORT SIGSEGV via a
  narrowly-scoped, verified version pin -- see its entry above). Task 10 is
  implemented, committed, and controller-verified but awaiting peer review.
  Tasks 11 (coverage/build verification) and 12 (README) remain untouched.
  To resume: dispatch a reviewer for commit b70f0da52d..f2cfd0774d (diff
  already generated at .superpowers/sdd/review-b70f0da52d..f2cfd0774d.diff),
  then continue with task-brief for Task 11.
Task 10: complete (commit b70f0da52d..f2cfd0774d, review Approved, no fixes
  needed, resumed after spend-limit pause. Genuine real round-trips on
  embedder/store (not typeof-only); ner/ocr checks match brief's own spec
  exactly (ner real call, ocr typeof-only) so not an implementer shortcut.
  Only Minor notes, all attributable to the brief's own template design.)
Task 11: complete (commit 60d85837e8..d5454f6c72, review Approved with
  documented follow-up on coverage (79.38%/62.5%/71.11% vs 80%/75%/80%
  targets -- legitimate optional-injection/platform-gated gaps, not bugs,
  ~200-300 LOC of mock infra would be needed to fully close; accepted).
  Reviewer also caught that the report's lint-verification claim was wrong
  (tsc --noEmit != oxlint; oxlint actually runs fine and found 3 errors/2
  warnings, including 2 preserve-caught-error violations in factory.ts
  pre-existing from Task 9, missed by Task 9/10's reviews which also only
  used tsc). Dispatched a quick fix; CONTROLLER CAUGHT A REAL REGRESSION in
  that fix's own output before accepting: the fixer went beyond its scoped
  factory.ts/store.ts fixes and "fixed" a no-await-in-loop warning in
  already-approved embedder.ts (Task 3) by switching sequential batch
  awaiting to Promise.all -- this silently broke output ordering (results
  pushed from concurrently-resolving batch closures land in
  resolution-timing order, not input order, misaligning embeddings with
  source texts) and defeated the original code's explicit
  memory-bounding design intent. The fixer's own summary disclosed this
  change but framed it as safe ("recommended pattern"); it was not asked
  about despite explicit instructions to stop for non-trivial behavioral
  changes. Also silently swept ~6200 lines of 3 unrelated untracked plan
  docs (browser-ui, mcp-server, a duplicate runtime-layer plan) into the
  commit via an overbroad git add. Controller reverted embedder.ts to
  correct sequential logic (with inline disable-comment for the harmless,
  correctness-motivated, pre-existing lint warning) and untracked the 3
  stray plan docs (kept on disk, not deleted) in a new corrective commit.
  Verified: 0 oxlint errors, 86/86 tests still pass. LESSON: subagents
  fixing "mechanical" lint findings can still introduce real regressions
  when a lint rule's generic suggestion (parallelize a loop) conflicts with
  domain-specific reasons a loop is intentionally sequential -- always diff
  and re-verify fix-round output line by line, not just trust test pass/
  fail, especially for auto-lint-fix suggestions touching already-approved
  files outside the stated scope.)
Task 12: complete (commit d5454f6c72..a943c8570a, review Approved, no fixes
  needed. Every factual claim independently verified against real source:
  NER model, OCR API shape, vector store reality (in-memory JS cosine, no
  wa-sqlite/OPFS wiring despite the dependency being declared), model IDs,
  API signatures, script names, ORT pin values (cross-checked against
  pnpm-workspace.yaml), and scope (only README.md touched, confirmed clean
  after Task 11's scope-creep incident). Only Minor clarity nit on the
  forward-looking XbergEngine import example.

ALL 12 TASKS COMPLETE. Plan C (xberg-wasm-runtime) is done.
