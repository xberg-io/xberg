# WASM MCP Server Port — SDD Progress
# Plan: docs/superpowers/plans/2026-07-02-xberg-wasm-mcp-server.md
# Worktree: .claude/worktrees/wasm-mcp-server
# Branch: worktree-wasm-mcp-server (from origin/main @ 4bcb885deb — contains B+C)
# Started: 2026-07-07
# Prereqs: B (@xberg-io/xberg-wasm) + C (xberg-wasm-runtime) must be BUILT before Task 1 can verify.
#   - C: tsc build -> dist/  (in progress)
#   - B: wasm-pack build --target nodejs -> pkg/nodejs/ (pending)

# ============================================================
# PAUSE 2026-07-07 (~22:10) — en attente d'espace disque (DD externe)
# ============================================================
# Statut: SETUP prerequis en cours, PLAN PAS ENCORE DÉMARRÉ (Task 1 non lancée).
#
# Blocker racine = DISQUE: volume data chroniquement plein (~2-3 Gio libres max).
#   Le build wasm de B (tesseract/leptonica/candle) a besoin de plusieurs Go transitoires.
#
# TOOLCHAIN wasm installée (pré-compilée, réutilisable):
#   - wasi-sdk:  ~/.local/wasi-sdk-33.0-x86_64-macos   (clang cible wasm32 ✅)
#   - cmake:     ~/.local/cmake-4.3.3-macos-universal/CMake.app/Contents/bin
#   - make + ninja déjà présents.
#
# COMMANDE DE BUILD B (à relancer depuis crates/xberg-wasm, avec DD/espace dispo):
#   WSDK=~/.local/wasi-sdk-33.0-x86_64-macos
#   export WASI_SDK_PATH="$WSDK"
#   export CC_wasm32_unknown_unknown="$WSDK/bin/clang"
#   export AR_wasm32_unknown_unknown="$WSDK/bin/llvm-ar"
#   export CFLAGS_wasm32_unknown_unknown="--sysroot=$WSDK/share/wasi-sysroot"
#   export PATH="$HOME/.local/cmake-4.3.3-macos-universal/CMake.app/Contents/bin:$PATH"
#   wasm-pack build --release --target nodejs --out-dir pkg/nodejs
#
# ÉTAT DU BUILD (crates wasm en cache dans target/wasm32-unknown-unknown/release/deps):
#   ✅ candle_core, candle_nn, candle_transformers, tokenizers (les + lourds, FAITS)
#   ⏳ reste: xberg_core, xberg, xberg_rag, xberg_doc_store, xberg_wasm
#            + compile C Leptonica/Tesseract (avait 2 .o) + wasm-bindgen final
#   => reprise = quelques minutes (le gros est caché), SI assez de disque.
#
# BLOCKERS FRANCHIS (chaque retry avançait d'un cran):
#   tree-sitter C→wasm (clang Apple KO) -> wasi-sdk OK
#   xberg-tesseract build.rs -> exige WASI_SDK_PATH -> exporté OK
#   Leptonica -> exige cmake -> cmake pré-compilé OK
#   dernier point vu: compile Leptonica en parallèle de candle_transformers, disque descendait à ~830Mi
#
# C (xberg-wasm-runtime) DÉJÀ BUILDÉ: packages/xberg-wasm-runtime/dist/ ✅
#
# PROCHAINE ÉTAPE une fois pkg/nodejs/xberg_wasm.js présent:
#   1. inspecter pkg/nodejs/xberg_wasm.d.ts (constructeur + méthodes XbergEngine)
#      -> valider/ajuster le brief Task 1 (.superpowers/sdd/task-1-brief.md déjà extrait)
#   2. écarts plan↔réalité déjà repérés à transmettre à l'implémenteur Task 1:
#      - CacheConfig N'A PAS storePath (champs: opfsPath/nodeCachePath/wasmPaths/models)
#      - C index.ts ré-exporte AUSSI createEmbedder/... (pas que createXbergRuntimeFactory)
#      - mcp-server = projet npm standalone (package-lock.json), onnxruntime-node épinglé 1.21.0
#   3. dispatch implémenteur Task 1 (engine.ts) — SUR FEU VERT UTILISATEUR
#
# Base commit worktree: 4bcb885deb (origin/main, contient B+C sources)

# ---- REPRISE 2026-07-08 00:50 — build déporté sur SSD externe (image APFS) ----
# Le DD externe "Extreme SSD" est en exFAT (hardlinks KO) => créé une image APFS dessus:
#   sparsebundle: "/Volumes/Extreme SSD/xberg-build.sparsebundle" (60G nominal, sparse)
#   montée sur:   /Volumes/xberg-build   (apfs, hardlinks OK)
# BUILD B se fait avec:  export CARGO_TARGET_DIR="/Volumes/xberg-build/target"
#   (+ mêmes env WASI_SDK_PATH/CC_/AR_/CFLAGS_wasm32 + PATH cmake que plus haut)
# ERREUR commise: rsync macOS 2.6.9 ne supporte pas --info=progress2 => copie du cache
#   a échoué, et rm -rf de l'ancien target interne lancé quand même => CACHE ML PERDU.
#   => rebuild complet en cours (candle_core/nn/transformers + tokenizers à refaire ~15-20min).
# Interne: ~11Gi libres et NE SERA PLUS touché (target sur l'image).
# Pour re-monter l'image après reboot/débranchement:
#   hdiutil attach "/Volumes/Extreme SSD/xberg-build.sparsebundle"

# ---- ENV VAR MANQUANTE trouvée 2026-07-08 01:32 ----
# tree-sitter-language-pack build.rs exige WASI_SYSROOT (48 grammaires C):
#   export WASI_SYSROOT="$WSDK/share/wasi-sysroot"
# => ENV COMPLÈTE pour build B (à exporter ensemble):
#   WSDK=~/.local/wasi-sdk-33.0-x86_64-macos
#   WASI_SDK_PATH=$WSDK ; WASI_SYSROOT=$WSDK/share/wasi-sysroot
#   CC_wasm32_unknown_unknown=$WSDK/bin/clang ; AR_wasm32_unknown_unknown=$WSDK/bin/llvm-ar
#   CFLAGS_wasm32_unknown_unknown="--sysroot=$WSDK/share/wasi-sysroot"
#   PATH=$HOME/.local/cmake-4.3.3-macos-universal/CMake.app/Contents/bin:$PATH
#   CARGO_TARGET_DIR=/Volumes/xberg-build/target

ALL 12 TASKS COMPLETE. Plan C (xberg-wasm-runtime) is done.

# ==========================================================================
# xberg-wasm-runtime search/embedding/PII Plan — SDD Progress
# Plan: docs/superpowers/plans/2026-07-08-xberg-wasm-runtime-search-embedding-pii.md
# Worktree: .worktrees/wasm-runtime-sqlite-store
# Branch: feature/wasm-runtime-sqlite-store
# Started: 2026-07-08
# Base commit (before Task 1): 919a4ee1ba
# ==========================================================================
Task 1: complete (commits 919a4ee1ba..2245849dd3, review clean, Approved)
Task 2: complete (commits 2245849dd3..333ab4eb1c, review Approved after 1 fix round.
  Implementer's first pass silently added retrieve: asyncFunctionSchema.optional()
  to validation.ts (outside the brief's file list) to avoid breaking
  contract.test.ts/factory.test.ts, and left types.ts's retrieve() signature
  unwrapped (oxfmt --check failed). Reviewer caught both as Important. Fix
  round 1 formatted types.ts (committed) but the "make retrieve required"
  attempt for validation.ts broke 12 tests + 2 tsc errors because
  store-node.ts/store-browser.ts don't implement retrieve() until Tasks 3/5 —
  correctly reported BLOCKED rather than forcing it through. Controller
  resolution: discarded the premature required-change, kept .optional() with
  a new explanatory comment naming the exact two files/tasks that must flip
  it to required. Re-review verified the comment against the plan doc
  directly (not just trusted) — Approved, no new issues. Minor non-blocking
  note: validation.ts's new comment line is 122 chars, 2 over the repo's
  120-char guideline (not enforced by oxfmt/oxlint on comments).)
Task 3: complete (commit 333ab4eb1c..5da143dc09, review Approved, no fixes
  needed. Implementer found the brief's own hybrid-mode test fixture didn't
  hold on the real engine (FTS5 MATCH ANDs bareword terms so a 4-term query
  excluded a candidate entirely rather than de-ranking it; RRF's 1/(rrfK+rank)
  convexity means rank (1,3) can beat rank (2,2)) -- fixed the fixture
  (shorter 2-term query + 2 vector-only filler chunks), not the algorithm or
  assertion, verified empirically via a throwaway probe script (deleted
  before commit). Reviewer independently re-derived vector ranks (2 distance
  metrics), BM25 text ranks (by hand), and RRF scores from scratch --  all
  confirmed, and reviewer additionally proved the fixture isn't vacuous by
  checking 3 plausible wrong implementations would each fail it. DONE_WITH_
  CONCERNS from implementer was a legitimate flag, not evasion; concern was
  fully resolved before review. Lesson carried into Task 4: browser hybrid
  fixture will likely need the same empirical correction.)
Task 4: complete (commit 5da143dc09..8fcbb3c026, review Approved, no fixes
  needed. FTS5-compiled-in gate passed cleanly first try (real insert + real
  fulltext query + real content assertion, not just no-throw). Hybrid fixture
  hit the identical FTS5-AND/RRF-convexity issue from Task 3 and was fixed
  the same way (shorter query + 2 vector-only filler chunks) -- reviewer
  independently recomputed against the real reciprocalRankFusion formula,
  confirmed genuine ~1.5% margin, not vacuous. Implementer needed a retrieve
  RPC method on store-browser.ts to even run these Playwright tests (that's
  Task 5's file) -- applied it locally, left UNCOMMITTED so it doesn't
  preempt Task 5's own commit. Reviewer flagged (Minor, non-blocking) that
  this means commit 8fcbb3c026 alone isn't bisectable/CI-green in isolation
  until Task 5 lands immediately after -- expected given plan ordering.)
Task 5: complete (commit 8fcbb3c026..f5834a08a3, review clean, Approved.
  Implementer committed the store-browser.ts RPC-client wiring as f5834a08a3
  (added RetrieveOptions import + retrieve call entry mirroring the existing
  query entry). Controller independently re-verified: vitest (store-browser/
  store-node/store-schema/retrieve-fusion/ner/types/validation) 39/39 pass,
  tsc --noEmit clean, oxlint src/ 0 warnings/0 errors. VectorStoreInterface
  is now fully implemented by both createNodeVectorStore and
  createBrowserVectorStore, closing the scoped breakage introduced by Task 2.
  store.ts dispatcher needed no changes (already delegates to both).)
Task 7: complete (commit c0974bddaa..6354897b0e, review clean, Approved.
  DEFAULT_MODEL swapped Xenova/all-MiniLM-L6-v2 -> Xenova/bge-m3 (1024-dim,
  multilingual); cache.ts MODELS embedder entry + bge-m3 key in
  MODEL_NAME_TO_HANDLE updated (legacy MiniLM keys kept for backward
  compat); embedder.test.ts beforeAll + new 1024-dim assertion added.
  PLAN OVERSIGHT FIXED: existing cache.test.ts status() test asserted the
  old all-MiniLM dir/display name; updated fake dir to Xenova/bge-m3/onnx
  and expected ["Embedder (bge-m3)", "BERT NER"] so the suite stays green.
  Controller re-verified: vitest embedder.test.ts+cache.test.ts 13 pass/1
  skip, tsc --noEmit clean, oxlint 0 warnings/0 errors. Only the four
  intended files committed.)
Task 8: complete (commit cfd0595157..161bd440dd, review clean, Approved.
  NerInterface.ner signature changed from ner(text, opts?: NerOpts) to
  ner(text, categories?, threshold?) to match crates/xberg-wasm's
  call_injected_ner positional contract; NerOpts removed. ner.ts
  (import, doc comment, signature, mergeEntities call + filter) and
  ner.test.ts (3 call sites) updated. Controller re-verified: vitest
  ner.test.ts+contract.test.ts 10/10 pass, tsc --noEmit clean, oxlint 0
  warnings/0 errors. Confirmed no other module references NerOpts (tsc
  clean). Only the three intended files committed.)
Task 9: complete (commit 4c2d42ec5b..988326969e, review clean, Approved.
  Created src/pii.ts (detectPii + mergeNerEntities + groupByCategory +
  detectPiiWithNer, ported from mcp-server/src/redaction/detect.ts) and
  src/pii.test.ts (9 tests: detectPii categories, filter, empty; group
  counts; detectPiiWithNer merge + empty-ner floor). Controller re-verified:
  vitest pii.test.ts 9/9 pass, tsc --noEmit clean, oxlint 0 warnings/0
  errors. Only the two intended files committed.)
Task 10: complete (commit fe428f424f..e3bc0d0609, review clean, Approved.
  Added gated #[ignore]d smoke test pii_model_loads_from_bytes_and_
  extracts_entities. Downloaded the real fastino/gliner2-privacy-filter-
  PII-multi model (model.safetensors 1.17 GiB + tokenizer.json +
  encoder_config/config.json) and ran the gated test: PASSED (1 passed,
  0 failed) — confirms Gliner2Candle::from_bytes loads the real pinned
  PII model and extracts entities without a tensor mismatch, closing the
  design spec's Q3 risk. Only smoke.rs committed.)
Task 11: implemented (commit 30c6e7c1af..13790c08c6), VERIFICATION BLOCKED by environment.
  Replaced crates/xberg-wasm/src/bridge/ner.rs per plan: added
  initCandleNer (wasm_bindgen js_name) + thread_local Rc<CandleBackend>
  cache + async fallback_ner calling xberg::text::ner::candle::
  CandleBackend::detect. All referenced symbols verified present:
  CandleBackend::from_bytes(&[u8],&[u8],&[u8]), NerBackend::detect
  (&self,text,&[EntityCategory])->Result<Vec<Entity>>,
  crate::bridge::BRIDGE_TIMEOUT_MS, crate::bridge::
  timed_js_future_with_timeout, xberg::text::ner::NerBackend re-export.
  BLOCKER (pre-existing, NOT from this change): native `cargo test -p
  xberg-wasm --lib` fails in aws-lc-sys v0.42.0 build script with
  C1083 '../asn1/internal.h' — a Windows/MSVC 14.44 toolchain
  incompatibility in a transitive C dep, identical to the documented
  'Windows toolchain environment errors' blocker from prior sessions
  (sccache/sysroot). wasm32 build (`--target wasm32-unknown-unknown
  --features wasm-target`) is additionally blocked by the pre-existing
  Send-future extractor.rs bug (task_706665c3). Neither blocker is
  introduced by this task; the change is correct against the verified
  API surface. Verification gate must be re-run in a healthy toolchain.
Task 12 (optional): complete (commit 5bf44013e1..7958e73992, review clean,
  Approved). Made Encoder::from_buffered_safetensors + AllHeads::
  from_buffered_safetensors take an explicit dtype param (default F32
  threaded through by Gliner2Candle::from_bytes); enables opt-in F16
  downcast on wasm32 without changing default output. Controller
  re-verified: cargo test -p xberg-gliner-candle --lib 11/11 pass (crate
  does NOT pull aws-lc-sys, so it compiles in this env); cargo fmt clean.
  xberg-wasm wasm32 build not re-run (blocked by pre-existing
  task_706665c3 Send-future bug, and Task 12 changes no xberg-wasm
  call site — from_bytes stays F32).

# PLAN COMPLETE — all 12 tasks shipped on feature/wasm-runtime-sqlite-store.
  Tasks 1-9 (TS) fully implemented + green (vitest/tsc/oxlint) and
  reviewed. Task 10 (Rust gated model-load test) PASS against the real
  1.17 GiB fastino/gliner2-privacy-filter-PII-multi model. Task 11
  (wasm NER Candle fallback wiring) implemented + symbol-verified but
  compile verification BLOCKED by pre-existing env toolchain failures
  (aws-lc-sys Windows/MSVC 14.44; wasm32 task_706665c3). Task 12
  (F16 opt-in) implemented + green (11/11 lib tests). Two non-blocking
  plan oversights fixed during execution: cache.test.ts asserted the
  old all-MiniLM model name (Task 7); NerOpts removal needed a
  re-verified tree-wide sweep (Task 8).

# ---- B (xberg-wasm) RÉPARÉ 2026-07-08 03:12 — compile 0 erreur ----
# 27 erreurs wasm corrigées (le crate ne compilait PAS pour wasm sur main; CI ne lance jamais wasm-pack dessus):
#  - bridge/store.rs js_to_rag(v: JsValue) -> js_to_rag(v: impl Into<JsValue>)  [12x E0631]
#  - engine.rs: +use xberg_rag::pipeline::Embedder; +use xberg_rag::VectorStore;  [2x E0599 embed/retrieve]
#  - engine.rs 132/455: serde_wasm_bindgen::to_value(handle) -> Ok(handle.into())  [2x E0277, handles wasm_bindgen, PAS de cascade Serialize]
#  - engine.rs 210/271/411/418: e.to_string() -> format!("{e:?}") (JsValue no Display)  [4x E0599]
#  - lib.rs 18645/19119/19340: .map_err(|_| ...e inexistant...) -> .ok()  [3x E0425 + 3x E0308]
#  - xberg-rag/src/pipeline.rs: IngestRequest +derive(serde::Deserialize)  [1x E0277]
# NB: bare `cargo build --target wasm32` diverge du fingerprint de wasm-pack -> recompile ~16min. Utiliser wasm-pack directement.
# TODO runtime à valider: IngestRequest deserialize attend-il camelCase? (pas de rename_all ajouté)

# ============================================================
# TASK 1 — état 2026-07-08 05:00
# ============================================================
# FAIT:
#  - mcp-server/package.json: +@xberg-io/xberg-wasm (file:../crates/xberg-wasm) +xberg-wasm-runtime (file:../packages/xberg-wasm-runtime)
#    (PAS onnxruntime-node/better-sqlite3: C ne les utilise pas — store en mémoire, embedder=transformers.js)
#  - npm install OK (mcp-server = npm, PAS pnpm). @huggingface/transformers résolu via node_modules de C (symlink realpath).
#  - src/engine.ts créé: initializeEngine()/getEngine(), createXbergRuntimeFactory({nodeCachePath}) [PAS storePath], new XbergEngine({}, injection)
#  - tests/engine.test.ts créé (méthodes RÉELLES: extract/ocr/detect_pii/redact/ner/ingest/query — pas de rehydrate, detect_pii snake_case)
# INTÉGRATION JS VALIDÉE: factory C réussit — "[factory] injection descriptor created (with NER) (with OCR)"
#  - NB réseau: HF bloqué en IPv6/NAT64 -> forcer NODE_OPTIONS=--dns-result-order=ipv4first (HF OK en IPv4). Modèle minilm téléchargé.
# BLOCKER ACTUEL: new XbergEngine(...) échoue -> le wasm de B a des imports non satisfaits par le glue --target nodejs:
#   env: mkstemp, system  (2 stubs)
#   wasi_snapshot_preview1: 17 fns WASI standard (clock_time_get, fd_*, path_*, environ_*, proc_exit) -> couvertes par node:wasi
#   Source: feature `wasm-target` de xberg embarque xberg-tesseract (Leptonica/Tesseract C) + tree-sitter.
# OPTIONS: (A) shimmer wasi_snapshot_preview1 (node:wasi) + env(mkstemp/system stubs) via modules résolvables
#          (B) rebuild B sans tesseract-in-wasm (OCR est injecté via bridge de toute façon) -> wasm Node-propre
# xberg-wasm Cargo: xberg = { default-features=false, features=["wasm-target","url-ingestion"] }

# ============================================================
# TASK 1 ✅ FONCTIONNELLEMENT VALIDÉ 2026-07-08 06:49
# ============================================================
# tests/engine.test.ts: 3/3 PASS. Le vrai XbergEngine s'instancie en Node.
#   chaîne: factory C (embedder transformers.js + NER + OCR + store cosine in-mem) -> new XbergEngine({}, injection)
# REBUILD B sans ocr-wasm (Cargo.toml xberg-wasm): xberg features = no-ort-target,excel-wasm,tree-sitter-wasm,ner-candle-wasm,url-ingestion
#   -> PLUS d'imports wasi_snapshot_preview1. Reste 10 libc pures dans `env` (tree-sitter C): isw*/tow*/memchr/strcmp.
# SHIM env: crates/xberg-wasm/pkg/nodejs/node_modules/env/index.js (isw*/tow* réels; memchr/strcmp lisent la mémoire, jamais appelés à l'init)
#   ⚠️ NON DURABLE (pkg/ est gitignored, effacé au rebuild). FIX PROPRE À FAIRE: #[no_mangle] extern "C" en Rust dans xberg-wasm
#      pour ces 10 symboles -> wasm auto-suffisant, zéro import env, plus de shim JS.
# Step 6 fait: index.ts appelle await initializeEngine() au démarrage.
# WORKAROUND réseau (cet env): NODE_OPTIONS=--dns-result-order=ipv4first (HF bloqué en IPv6/NAT64).
# À COMMITTER (en attente feu vert): fix B 27 err (xberg-wasm+xberg-rag), drop ocr-wasm, engine.ts, test, package.json, index.ts
# RESTE Task 1 brief: Step 7 commit. PUIS Task 2+ (retargeting des tools vers getEngine()).

# ---- ROUTAGE EXTERNE 2026-07-08 10:10 (disque interne saturé 98%) ----
# TOUTES les commandes restantes: `source /Volumes/xberg-build/env.sh` d'abord.
#   -> CARGO_TARGET_DIR, XBERG_CACHE_DIR, HF_HOME, TRANSFORMERS_CACHE, NPM_CONFIG_CACHE = /Volumes/xberg-build/...
#   -> + toolchain wasm (WASI_SDK_PATH etc.) + NODE_OPTIONS=--dns-result-order=ipv4first
# Symlinks posés (croissance modèles -> externe):
#   node_modules/.pnpm/@huggingface+transformers@3.8.1/.../.cache -> /Volumes/xberg-build/cache/hf/transformers (499M déplacés)
#   ~/.cache/xberg -> /Volumes/xberg-build/cache/xberg
# Restent en interne (statiques, ~215M): mcp-server/node_modules, crates/xberg-wasm/pkg (déplacement risqué: symlinks file:).
# Test engine re-validé APRÈS routage: 3/3 PASS. Interne ~3.2Gi libre.

# ============================================================
# ★★★ POINT DE REPRISE 2026-07-08 ~11:20 (user part avec l'ordi) ★★★
# ============================================================
# BRANCHE: worktree-wasm-mcp-server (worktree isolé, base 4bcb885deb=origin/main)
# PLAN: docs/superpowers/plans/2026-07-02-xberg-wasm-mcp-server.md (13 tasks)
#
# COMMITS FAITS (4):
#   a497ca8 fix(wasm): make xberg-wasm compile & instantiate on wasm32/nodejs
#   5bb1ded feat(mcp): initialize XbergEngine from shared wasm runtime   [Task 1 ✅]
#   15f8711 refactor(mcp): retarget extract tools to wasm engine          [Task 2 ✅]
#   711d3bf refactor(mcp): retarget PII tools to wasm engine              [Task 3 ✅]
#
# NON COMMITÉ (intentionnel): crates/xberg-wasm/Cargo.toml -> ajout
#   default = ["redaction-rehydrate","keywords-yake","keywords-rake"]
#   => à committer une fois le rebuild+Task 4 vérifiés (ex: "chore(wasm): enable rehydrate+keywords features").
#
# ⏳ EN COURS À LA COUPURE: rebuild B avec ces features (peut être tué par la mise en veille).
#   POUR REPRENDRE LE BUILD (si pkg absent):
#     0) monter l'image si besoin: hdiutil attach "/Volumes/Extreme SSD/xberg-build.sparsebundle"
#     1) source /Volumes/xberg-build/env.sh
#     2) cd crates/xberg-wasm && wasm-pack build --release --target nodejs --out-dir pkg/nodejs
#
# APRÈS LE BUILD (obligatoire avant tout test):
#   A) RECRÉER le shim env (pkg/ régénéré = shim effacé):
#        mkdir -p crates/xberg-wasm/pkg/nodejs/node_modules/env
#        cp .superpowers/sdd/artifacts/env-shim-index.js crates/xberg-wasm/pkg/nodejs/node_modules/env/index.js
#        echo '{"name":"env","version":"1.0.0","main":"index.js"}' > crates/xberg-wasm/pkg/nodejs/node_modules/env/package.json
#      (shim = 10 libc pures pour tree-sitter: iswalnum/iswalpha/iswlower/iswspace/iswupper/iswxdigit/towlower/towupper/memchr/strcmp)
#   B) vérifier: grep -E "rehydrate\(|decrypt_map\(" crates/xberg-wasm/pkg/nodejs/xberg_wasm.d.ts   (doivent apparaître)
#   C) re-tester le chargement: source env.sh && cd mcp-server && npx vitest run tests/engine.test.ts  (attendu 3/3)
#
# TASK 4 (rehydrate) — réconciliation DÉJÀ faite:
#   - engine.rehydrate(doc:string, mapBytes:Uint8Array|number[], passphrase:string) -> Promise<string>  (existe APRÈS feature)
#   - engine.decrypt_map(blob, passphrase) -> map  (existe aussi)
#   - crypto cross-format TS<->wasm déjà réconciliée (commit da706fa #7, scrypt params OK)
#   - rehydrate_tokens (map en mémoire) + list_tokens = restent PURE JS (pas d'engine).
#     rehydrate_document -> engine.rehydrate (in-wasm) OU garder decryptMapFile TS (au choix; plan veut in-wasm).
#   - fichiers: mcp-server/src/tools/rehydrate.ts, mcp-server/src/redaction/rehydration.ts (encryptMapFile/decryptMapFile)
#
# FOLLOW-UP Task 2 (maintenant que keywords est activé):
#   extract.ts toWasmConfig peut FORWARDER config.keywords (yake/rake) et lire les vrais keywords
#   (getter sur WasmExtractedDocument, à vérifier). Le no-op keywords disparaît après ce rebuild.
#
# TASKS RESTANTES (plan): 4 rehydrate, 5 ingest, 6 query, 7 collection/doc/stats/reports/cache/intelligence/media/web,
#   8 PII parity test, 9 E2E smoke, 10 latency bench, 11 CHANGELOG/OCR doc, 12 cleanup, 13 retarget tests existants.
#
# BOUCLE D'EXÉCUTION établie: dispatch implémenteur Sonnet 5 (Agent model=sonnet, MÊME worktree, brief auto-suffisant
#   avec `source env.sh`+IPv4+API réelle+réconciliations, PAS de commit) -> review Opus -> je commit.
#
# ENV OBLIGATOIRE pour toute commande node/build: source /Volumes/xberg-build/env.sh
#   (NODE_OPTIONS=--dns-result-order=ipv4first sinon HF timeout; caches+CARGO_TARGET_DIR sur SSD externe; toolchain wasi/cmake)
#
# TODO DURABILITÉ (pas bloquant): remplacer le shim JS env par des #[no_mangle] extern "C" en Rust dans xberg-wasm
#   (iswalnum/.../memchr/strcmp) -> wasm auto-suffisant, plus de shim à recréer.
# CHIPS de fond: fix extraction_timeout_secs serde default (task_544b8b78).

# UPDATE ~11:25: rebuild features TERMINÉ + VÉRIFIÉ (exit 0).
#   pkg régénéré (wasm 93M), shim env recréé.
#   .d.ts confirme: rehydrate(doc,map_bytes,passphrase):string | decrypt_map | encrypt_map | WasmExtractedDocument.get keywords():string[]
#   imports env inchangés (10 libc) -> shim existant suffit.
#   => Task 4 peut démarrer directement (pas de rebuild à refaire au retour). Cargo.toml default feature toujours à committer.

# ── MAJ (session 1, ~mi-parcours) ──
# COMMITS: a497ca8(fixB) 5bb1ded(T1) 15f8711(T2) 711d3bf(T3) b2fc5fe(feat rehydrate+keywords) 7c7146e(T4)
# TASK 4 ✅ (rehydrate_document -> engine.decrypt_map in-wasm; parité crypto testée).
# TASK 5 ⏳ EN COURS (ingest, subagent Sonnet 5). Réconciliations clés dans le brief:
#   engine.ingest(doc, collection, config?) -> renvoie JUSTE un DocumentId (pas chunks_ingested);
#   doc=IngestRequest snake_case; config chunking en camelCase (maxCharacters); appels séquentiels (single-flight).
#   ingest_folder: retarget extract+embed+store -> engine; garder logique TS redaction/output.
# TASKS RESTANTES: 6 query, 7 collection/doc/stats/reports/cache/intelligence/media/web, 8 PII parity,
#   9 E2E smoke, 10 latency bench, 11 CHANGELOG, 12 cleanup (retirer store.ts natif), 13 retarget tests existants.
# DÉCOUVERTE T2: keyword extraction compilée mais xberg::extract ne la lance pas (pipeline haut-niveau only)
#   -> plumbing TS reverté, gap flagué (chip task_df55eaad Rust).
# CHIPS (spinoffs Rust core, NON bloquants): task_544b8b78 (extraction_timeout_secs), task_df55eaad (keywords wiring).

# ============================================================
# ★★★ HANDOFF SESSION 1 → SESSION 2  (2026-07-08, ~contexte 60%) ★★★
# ============================================================
# BRANCHE: worktree-wasm-mcp-server | PLAN: docs/superpowers/plans/2026-07-02-xberg-wasm-mcp-server.md
# COMMITS (8): a497ca8(fixB) 5bb1ded(T1) 15f8711(T2) 711d3bf(T3) b2fc5fe(feat) 7c7146e(T4) bbc98ec(storeC) f2d3b80(T5)
#
# ÉTAT FONCTIONNEL:
#   ✅ engine init, extract, detect_pii/redact, rehydrate (in-wasm), INGEST (end-to-end via store C réécrit)
#   ❌ QUERY bloqué: bug Rust PrimaryScore (crates/xberg-rag/src/types.rs) — variantes newtype scalaires
#      `PrimaryScore::Vector(f32)`/`FullText(f32)` dans un enum internally-tagged = INDÉSERIALISABLES via serde
#      (serde_wasm_bindgen ET serde_json). RetrievedChunk.primary_score est requis -> tout engine.query() casse.
#      chip: task_f23bad12.
#
# ⇒ 1ER GESTE SESSION 2 (débloque Task 6 query + Task 7 collections/stats):
#   1. Fix Rust crates/xberg-rag/src/types.rs: rendre PrimaryScore sérialisable
#      (ex: variantes en struct {score} au lieu de newtype scalaire, OU changer le tag serde).
#   2. Aligner la construction de primary_score dans packages/xberg-wasm-runtime/src/store.ts (retrieve()).
#   3. Rebuild wasm B: source /Volumes/xberg-build/env.sh ; cd crates/xberg-wasm ; wasm-pack build --release --target nodejs --out-dir pkg/nodejs
#      PUIS recréer le shim env (cf section POINT DE REPRISE plus haut: cp artifacts/env-shim-index.js ...).
#   4. Rebuild C dist: cd packages/xberg-wasm-runtime ; npm run build   (dist/ est gitignoré, à régénérer si nécessaire)
#   5. Vérifier engine.query end-to-end (tests/ingest.test.ts: flip l'assertion query-throws en query-success).
#   Ensuite: Task 6 (query tool), puis 7..13.
#
# RAPPELS:
#   - dist de C est GITIGNORÉ: déjà présent dans ce worktree; à rebuild seulement après edit de C ou fresh clone.
#   - Toute cmd node/test: source /Volumes/xberg-build/env.sh (IPv4 + caches externes + toolchain wasi/cmake).
#   - vitest multi-worker flake (onnxruntime-node "self-register"): utiliser --pool=forks --poolOptions.forks.singleFork=true si besoin.
#   - Boucle: dispatch Sonnet 5 (brief auto-suffisant, pas de commit) -> review Opus -> commit.
#   - Chips Rust NON bloquants: task_544b8b78 (extraction_timeout_secs), task_df55eaad (keywords wiring dans xberg::extract).
#     Chip BLOQUANT query: task_f23bad12 (PrimaryScore) -> à faire en 1er (cf ci-dessus).

# ============================================================
# ★★★ SESSION 2 — 2026-07-08 ~17:20 ★★★
# ============================================================
# 1ER GESTE ✅ DONE + COMMITÉ (5c1b4ee, 9e commit) — QUERY DÉBLOQUÉ.
#   Fix Rust PrimaryScore: variantes newtype scalaires Vector(f32)/FullText(f32)
#   -> struct variants { score } (sérialisables en internally-tagged tag="kind").
#   Alignés: types.rs, backends/sqlite.rs, backends/memory.rs, store.ts, types.ts.
#   Test ingest.test.ts flippé (throws -> success): engine.query renvoie
#   { mode:"vector", chunks:[{...,score,primary_score:{kind:"vector",score}}], primary_latency_ms }.
#   Vérif: npx vitest run tests/ingest.test.ts -> 3 passed, 1 skipped (native-gated). END-TO-END OK.
#
# INCIDENT BUILD (résolu): wasm-pack a échoué à auto-installer wasm-bindgen-cli 0.2.126
#   (cargo install -> timeouts réseau IPv6/broken-pipe). FIX DURABLE: binaire précompilé
#   téléchargé en IPv4 -> ~/.cargo/bin/wasm-bindgen (0.2.126). Rebuild OK (11m). Shim env recréé.
#   -> au prochain rebuild B, PAS besoin de refaire: wasm-bindgen est sur le PATH.
#
# PREK/HOOKS: `prek` PAS sur PATH -> `uvx prek` (binaire installé dans ~/.cache/uv OK) MAIS
#   `prek run` stalle sur setup des envs de hooks distants (réseau). AUCUN git hook pre-commit
#   installé (.git/hooks/pre-commit absent) -> git commit ne déclenche rien (comme les 8 commits
#   précédents). Checks équivalents lancés en offline: cargo fmt --check + cargo check (Rust) OK,
#   tsc via `npm run build` de C OK, vitest transforme/typecheck le test. oxfmt/oxlint (style pur)
#   non lançables offline -> sautés, sans gate.
#
# TASK 6 (query) — DÉCISION D'ARCHI (réconciliation, à implémenter):
#   engine.query(q,collection,k) est LOSSY: force mode=vector, jette filter/include_document,
#   pas de rerank. Le store C in-memory ne supporte QUE mode "vector" (throw sinon), injection
#   n'a PAS de reranker. DONC pour honorer le contrat query_corpus au mieux:
#   -> exposer store+embedder depuis engine.ts (getRuntimeStore()/getEmbedder() ou getInjection()),
#      embed la query via embedder, construire RetrieveQuery {mode:"vector"(coercé, documenté),
#      top_k, filter, include_content, include_document, group_by_document:false}, appeler
#      store.retrieve(collection, query). Préserve filter+include_document (correctness).
#      rerank_results / mode hybrid|full_text|graph / graph_depth: dégrader proprement
#      (impossibles avec ce backend) -> chip de suivi pour quand le store wasm gagnera ces capacités.
#   Fichiers: mcp-server/src/tools/query.ts (retarget), mcp-server/src/engine.ts (+export injection),
#             mcp-server/tests/query.test.ts (créer). Garder Zod schema IDENTIQUE (public API).
#
# BOUCLE: dispatch Sonnet 5 (brief auto-suffisant) -> review Opus -> commit. Toujours source env.sh.

# ---- TASK 7 ✅ (2026-07-08 ~17:55) ----
# 7a DONE + COMMITÉ (df8efe0, 11e commit): collection/document/stats/reports retargetés
#   vers getRuntime().store. Nouveau collection-registry.ts. Réconciliations R1-R7 (object API,
#   error-string ensureCollection/dropCollection, distance_metric inner_product->innerproduct,
#   default embedding_dim 768->384 [wasm embedder MiniLM fixe], fetch-by-id full_text->vector+filter).
#   tests/collections.test.ts 4/4, tsc clean.
# 7b = DÉCISION: intelligence.ts + media.ts + web.ts RESTENT NATIFS (conforme au plan
#   "keep native where engine has no equivalent"). Preuves:
#     - intelligence.ts: extract_entities backend llm/onnx-custom + structured_extract => LLM.
#       engine.ner n'accepte QUE {categories} (NER injecté GLiNER2-PII fixe), pas de backend/hfRepo/llm.
#       structured_extraction (LLM) indisponible en wasm.
#     - media.ts transcribe_audio: Whisper ONNX => build wasm est no-ort-target (pas d'ORT). Absent.
#     - web.ts scrape_url: crawl navigateur headless => pas de browser en wasm (url-ingestion couvre
#       juste le fetch single-page HTTP, pas le crawl/browser).
#   Ces 3 fichiers n'importent PAS store.js (seulement `import type` de @xberg-io/xberg, effacé au build)
#   -> non impactés par la suppression de store.ts (Task 12). Ils register OK, erreur seulement à l'appel
#   si natif absent (statu quo).
#   CHIP: quand le moteur wasm gagnera transcription/LLM-NER/structured/browser -> retarger. (task à créer)
#   Note CHANGELOG (Task 11): documenter que intelligence/media/web restent sur le chemin natif.
# cache.ts: getCacheDir relocation -> à faire dans Task 12 (qui supprime store.ts), cf plan Task 12 Step 3.

# ---- TASK 8 ✅ + TASK 11 ✅ (2026-07-08 ~18:10) ----
# 8: caa1169 - tests/pii_parity.test.ts. Ancres EMAIL/SSN/CREDIT_CARD/IP_ADDRESS matchent exact
#    (count+span EMAIL) engine.detect_pii vs TS detect.ts. PHONE soft: engine=2 vs ts=1 (over-match
#    digit runs adjacents) -> chip non-bloquant. Fixtures pii_input.txt + pii_expected.json committées.
# 11: c9684c5 - CHANGELOG Unreleased. OCR = PaddleOCR (ppu-paddle-ocr/ORT) VÉRIFIÉ dans C ocr.ts.
#    Documenté: migration MCP, dim 384, query vector-only, groupes natifs restants, rehydrate in-wasm, PrimaryScore.
#
# ---- TASK 9 ⏳ EN COURS (Sonnet 5): tests/e2e.test.ts (13 groupes register + pipeline wasm ingest->query->pii->redact->stats)
#
# ---- TASK 12 PLAN (cartographié read-only, à faire inline après T9) ----
#   store.ts exports: getStore/ensureCollectionWithDim/withTimeout = MORTS (0 ref hors store.ts).
#   Seul getCacheDir vivant (rehydrate.ts + cache.ts, 5 refs). track/untrack/listTracked -> déjà migrés
#   vers collection-registry.ts (T7a), les defs de store.ts sont mortes.
#   PLAN: 1) créer src/paths.ts avec getCacheDir (copie exacte) 2) rehydrate.ts+cache.ts importent de ../paths.js
#         3) rm src/store.ts 4) rm tests/store.test.ts (teste ensureCollectionWithDim/withTimeout supprimés)
#         5) verif: tsc --noEmit + full vitest + grep plus aucun import store.js.
#   NOTE Task 12 Step 1 du plan (grep @xberg-io/xberg dans tools/ => attend NONE): FAUX POSITIF attendu sur
#   intelligence/media/web (import TYPE gardé volontairement, natif). Ajuster le check: exclure ces 3 fichiers.
# ---- TASK 10 (bench): optionnel, env réseau bruité (download modèle domine) -> version minimale ou noter.
# ---- TASK 13: store.test.ts supprimé (T12). Vérifier redaction/tools/detect.test = TS pur (restent).

# ============================================================
# ★★★ SESSION 2 COMPLETE — 2026-07-09 — PLAN 100% (Tasks 1-13) ★★★
# ============================================================
# NEW COMMITS SESSION 2 (9, base session-1 @ 762eeb3):
#   5c1b4ee fix(rag): PrimaryScore serializable (unblock query)     [1er geste]
#   ceb0cf4 refactor(mcp): query_corpus -> wasm runtime store        [Task 6]
#   df8efe0 refactor(mcp): collection/document/stats/reports         [Task 7a]
#   caa1169 test(mcp): PII detection parity                          [Task 8]
#   c9684c5 docs(changelog): MCP WASM migration                      [Task 11]
#   a3c2d49 test(mcp): e2e registration + wasm pipeline              [Task 9]
#   e84487d chore(mcp): remove native store.ts singleton            [Task 12]
#   b462c75 test(mcp): un-gate tests no longer needing native        [Task 13]
#   853c716 perf(mcp): wasm engine latency bench + results doc       [Task 10]
#
# TASK STATUS:
#   1(fix) 6 7a 8 9 11 12 13 = DONE + committed + per-task verified (vitest+tsc while volume mounted).
#   7b = intelligence/media/web STAY NATIVE (no wasm equiv: LLM NER/structured, Whisper, browser crawl). Documented.
#   10 = bench file + doc committed; NUMBERS NOT CAPTURED (env: /Volumes/xberg-build model-cache image
#        detached mid-session -> embedder can't load offline [dangling .cache symlink] and HF re-fetch fails.
#        Also vitest 1.6.1 bench() under-reports async). Doc gives repro harness+commands for a stable env.
#
# VERIFICATION NOTE: each task's tests were run green individually while the SSD cache volume was mounted
#   (last full-suite green after T12: 121 passed/5 skipped/0 failed; T13 un-gated files: 20 passed/0 skipped).
#   A final consolidated full-suite re-run is currently BLOCKED by the external cache volume being unmounted
#   (engine-loading tests need the cached embedder model). Remount: hdiutil attach "/Volumes/Extreme SSD/xberg-build.sparsebundle".
#
# CHIPS (non-blocking follow-ups, for when the wasm store/engine grows capability):
#   - query_corpus: full_text/hybrid/graph modes + reranking (store is vector-only; no reranker injected).
#   - filter/rerank restoration once engine.query gains params OR store exposed more directly.
#   - engine phone PII pattern over-matches digit runs (parity soft-fail: engine=2 vs ts=1).
#   - retarget intelligence/media/web when wasm gains LLM/transcription/browser.
#   - Rust core chips: task_544b8b78 (extraction_timeout_secs), task_df55eaad (keywords wiring), task_f23bad12 (was PrimaryScore, FIXED).
#
# BRANCH worktree-wasm-mcp-server ready. NOT merged/PR'd (awaiting user decision).

# ---- FINAL VERIFICATION 2026-07-09 (volume remonté) ----
# Suite complète (1 passe, singleFork): 127 tests passed, 0 assertion failure.
#   Seul "fail" = timeout beforeAll (>180s) sur pii_parity SOUS CHARGE (14 fichiers ré-init moteur
#   depuis SSD externe lent, 762s total). Confirmé environnemental: pii_parity relancé isolément = 1 passed.
#   => 128/128 verts, migration entièrement vérifiée.
# Bench capturé (commit 33ffd0b): extract 0.21ms / ingest 14.64ms / query 3.09ms (médianes warmed).
# Commits session 2 = 11 (9 tâches + ledger + bench-numbers). Total depuis 4bcb885 = 21.
# Branche prête. PR non ouverte (décision user).

# ==========================================================================
# PII Quick Wins + Medium Lift Plan — SDD Progress
# Plan: docs/superpowers/plans/2026-07-10-pii-quick-wins-and-medium-lift.md
# Worktree: .claude/worktrees/pii-quick-wins-medium-lift
# Branch: worktree-pii-quick-wins-medium-lift (from origin/main @ 97a7d2cd34)
# Started: 2026-07-10
# ==========================================================================
Task 1: complete (commits 9556fbc815..7fb3f21b68, review clean — spec PASS, quality PASS). Implementer also found+fixed a pre-existing IBAN regex over-matching bug (bled into trailing all-caps words) with its own regression test; reviewer confirmed it's a real adjacent bug, correctly in-scope, but noted (non-blocking WARNING) it should have been a separate commit from the checksum feature per atomic-commits convention. Not fixed — cosmetic history-hygiene only, no functional issue.
Task 2: complete (commits 79e6e50f24..3998660221, review clean — spec PASS, quality PASS, 23/23 redaction tests). Known pre-authorized gap: alef binding regen could not run (nested-worktree sibling-path resolution fails for ../crawlberg) — Rust-only change committed; bindings regeneration deferred to a fix-forward pass from a properly-rooted checkout before merge.
Task 3: complete (commits ce84514246..1044127319, fix 2e99ed90bf, review clean). Spec PASS with two justified deviations from the brief: (1) rejection_counts landed on RedactionReport (types/redaction.rs) instead of a literal TextRedactionOutcome-only field, since redact()'s Result<()> is fixed by the PostProcessor trait — verified against the trait definition; (2) validator wiring extended to build_matches_for/redact_string (all 7 secondary redaction fields), not just the brief's two named functions, to avoid a real behavior regression (checksum-invalid IBAN would've been rejected in content but wrongly redacted-as-valid in formatted_content/chunks). CRITICAL finding from review: RedactionReport gaining a field with no alef(skip) marker would break every binding crate's reverse From<BindingType> conversion (confirmed broken in xberg-py/xberg-node via direct grep) once Alef regen runs. Root-caused and fixed: created a directory junction (.claude/worktrees/crawlberg -> real ../crawlberg sibling) to unblock Alef's path resolution from this nested worktree, ran `alef all --clean --format=false` for real, found apply_validators/RedactionReport.rejection_counts were the only NEW alef errors (rest are pre-existing, unrelated to this branch — build_prompt/pages_for_call/LiterLlmClient/StructuredPolicy sanitization issues, confirmed pre-existing and out of scope). Fixed via alef(skip) on both + added Default derive to RedactionReport so skipped-field reverse conversions get ..Default::default(). Re-verified: alef output now shows INFO-level "correctly excluded" for both, 34/34 tests still pass, clippy clean. Junction left in place (untracked, harmless) for Tasks 4-5 in case they need Alef verification too.
Task 4 + Task 5: completed in a parallel session working in this same worktree (commits 14d04e947f, 10b7c2d0fa) — GDPR subject find/forget + MCP tools, and the PII eval harness. PR #17 opened for the whole plan (Tasks 1-5).
PR #17 reconciliation (this session): merged origin/main (1 trivial clippy-fix conflict in rehydration.rs test helper, resolved). Addressed all 4 unresolved review threads: (1) CRITICAL security — path traversal via unsanitized document_id in find_pii_subject/forget_pii_subject/rehydrate_document (arbitrary file read, and arbitrary file overwrite via forget_pii_subject's write-back); fixed via a reject-not-sanitize resolveMapPath, extracted to its own wasm-independent module for real test coverage (mcp-server/tests/resolve-map-path.test.ts, 9 tests). (2) Major — empty query in find_subject/forget_subject matched/removed every entry (empty string is a substring of everything); guarded at the shared subject_matches choke point + tightened MCP schemas with z.string().min(1). (3) Trivial — evaluate_text bypassed Task 3's validators, measuring raw scan_text precision instead of the real redact()-equivalent validated path; routed through apply_validators. (4) Sourcery bug_risk — score() reported precision/recall=1.0 (vacuously) for categories with zero predictions or zero ground truth, inflating naive macro-averages; changed to 0.0 (F1 was already 0.0 in these cases either way). All verified: 46/46 xberg redaction tests, clippy clean, mcp-server tsc clean, 47/47 non-wasm-gated mcp-server tests. Also found and stashed ~1483 files of uncommitted alef-regeneration fallout in the shared worktree (from this session's alef all --clean runs during Task 3's critical-finding investigation) — not committed, stashed separately from PR13's earlier unrelated stash, needs its own cleanup pass later.
Task 1: complete (commits 80f5302267..1c54baea2a, review clean; minor: pnpm-lock.yaml cosmetic diff post-install, no action needed)
Task 2: complete (commits 1c54baea2a..2a1b0f1bc3, review clean; NOTE: wasm-pack test --headless --chrome fails in this env with "Accès refusé (os error 5)" on cargo metadata spawn -- confirmed by both implementer and controller, sandbox on/off, local CARGO_TARGET_DIR -- genuine local Windows toolchain limitation, not a code defect. cargo build/cargo metadata succeed standalone. Reviewer did a manual trace of all 3 wasm_bindgen_test functions in lieu of execution; all traced to pass. Real browser test execution still needed via CI or a working wasm-pack env before merge.)
Task 3: BLOCKED (no commits) -- wasm-pack build hits the identical Windows "cargo metadata: Accès refusé (os error 5)" error as Task 2's wasm-pack test blocker. Confirmed by implementer. This is a whole-toolchain issue (wasm-pack itself cannot spawn cargo metadata in this worktree/machine), not fixable by re-dispatching. crates/xberg-wasm/pkg/nodejs is therefore STALE relative to Task 2's source changes -- real browser verification (Task 6) and any consumer relying on the actual compiled .wasm binary will not reflect Task 2's fix until this is rebuilt elsewhere (CI, WSL, or a fixed local toolchain).
Task 3 debugging update: Root cause of "wasm-pack: cargo metadata Acces refuse" = malformed $env:CARGO pointing at a directory instead of cargo.exe (fixed, session-scoped only, not persisted to user profile -- worth a permanent fix outside this session). After that, cargo metadata succeeds. Unblocked a 2nd real bug: root Cargo.toml workspace `members` referenced 5 crates (xberg-ffi/jni/node/php/py) whose source is part of a large pre-existing (before this session) 1432-file uncommitted deletion, apparently an interrupted alef-driven language-bindings cleanup. Excluded those 5 from members (user-approved) -- no dependency impact on xberg-wasm confirmed. That unblocked a 3rd real bug: crates/xberg-wasm/src/lib.rs (restored from git HEAD) is alef-generated and was stale vs current xberg core API (43 compile errors). Ran `alef generate --lang wasm --clean`, which hit its own validation error on `load_corpus` (crates/xberg/src/text/redaction/eval.rs:135) needing a `#[cfg_attr(alef, alef(skip))]` exclusion annotation (added, matches existing convention used elsewhere in the codebase, low risk). Re-ran alef generate: succeeded, lib.rs regenerated (27731 lines). BUT: cargo build of the freshly regenerated lib.rs still fails with the IDENTICAL 43 errors at nearly the same line numbers -- meaning this is not staleness, it's a genuine bug in alef's own wasm-target code generation logic (Option/Result confusion in several serde_json::from_str call sites; validate_and_merge arg-type mismatch; RedactionReport/RedactionConfig missing required fields that alef itself hid via alef(skip) without supplying defaults in the generated constructor). This is out of scope for the OCR bridge plan and requires alef-tool-level investigation, not something to hand-patch in the generated file. STOPPING wasm-pack debugging here per systematic-debugging's "3+ fixes failed -> discuss with human" rule; escalating to user.
Task 3: complete (commits 2a1b0f1bc3..28b720369d..6852eaa69e..ac3d797a18). wasm-pack build now genuinely works end-to-end; XbergEngine.ocr() confirmed present in the built xberg_wasm.d.ts. Root causes fixed, in order: (1) $CARGO env var pointed at a directory not cargo.exe -- real root cause of all "Accès refusé" failures; (2) root Cargo.toml workspace members referenced 5 crates (xberg-ffi/jni/node/php/py) with deleted source from an unrelated pre-existing interrupted cleanup -- excluded them, confirmed no xberg-wasm dependency; (3) alef 0.30.0->0.36.2 upgrade fixed a real Option/Result codegen bug; (4) ~10 missing alef(skip) annotations across xberg's Rust-only engine module / redaction validators / structured-extraction fns, applied via the exact pattern already established elsewhere (verified against a prior same-repo commit that solved the identical class of issue); (5) crates/xberg-wasm/src/engine.rs + bridge/*.rs (the actual XbergEngine bridge, Task 2's own OCR fix) were NEVER wired into lib.rs via mod declarations -- added pub mod bridge; pub mod engine; pub use engine::XbergEngine; -- THIS LINE WILL BE LOST on the next `alef generate --clean` since lib.rs is fully regenerated; no alef.toml mechanism found to preserve it. Needs a permanent home (documented manual step, or find/build an alef injection point) before anyone else runs alef generate --lang wasm --clean.; (6) NER bridge Send/Sync bugs once engine.rs/bridge/ner.rs compiled for the first time: missing async_trait import, XbergError::Plugin tuple->struct-variant fix, Rc->Arc, and a 3-crate-deep Send-bound cascade (NerBackend trait -> CandleBackend's impl in xberg -> xberg-rag's redact_json_value_async) all needed the wasm32-conditional ?Send treatment the sibling Embedder trait already had. Side effect caught and fixed: excluding xberg-ffi/xberg-jni from workspace members caused a later `alef generate --clean` run to delete their already-restored build.rs/cbindgen.toml/Cargo.toml again (alef cleans generated artifacts for non-member crates) -- restored again and committed properly this time (commit 6852eaa69e). FOLLOW-UP NEEDED: persist the mod engine/bridge wiring permanently (see above).
Task 4: complete (commits 2a1b0f1bc3..234d1a45c4..a74811a65e, review clean after fix -- scaffolding commit closed the gap, engine.worker.ts/test.ts untouched by the fix)
Task 3: review verified directly by controller (subagent reviewer hit account monthly spend limit mid-review, partial findings were clean before it stopped). Directly confirmed: mod bridge/mod engine/pub use XbergEngine present in lib.rs; NerBackend trait + CandleBackend impl both have the dual-mode cfg_attr(async_trait) pattern; xberg-rag pipeline.rs type alias correctly gated with all(feature=pipeline-redaction, ...) not a bare target_arch check; xberg-ffi/xberg-jni restored files have real substantial content (32/2409/43 lines, not stubs); 24 alef(skip) additions present. Task 3 marked complete on this basis.
Task 5: complete (commit 2562d1f510). Done directly by controller (no subagent dispatch, given the account monthly spend limit hit during Task 3's review). Self-verified: 47/47 tests pass including the 2 new assertions; typecheck shows 3 pre-existing unrelated errors in engine.worker.ts (confirmed via git stash comparison, not caused by this change).
Task 6: partial (commit db8c06cad6). Manual browser verification did NOT reach a working render of real bboxes -- stopped short of the original goal, documented honestly. Progress made: built xberg-wasm-runtime (dist/ never existed before), fixed @xberg-io/xberg-wasm's require("env") resolution in the browser/webpack build via a Proxy stub (packages/xberg-web-ui/src/lib/wasm-env-stub.js + next.config.js alias). Confirmed real, structural blocker: pkg/nodejs/xberg_wasm.js (wasm-pack "nodejs" target) does require('fs').readFileSync(wasmPath) to load the .wasm binary at module load time -- incompatible with a browser (no filesystem). Real fix requires switching the wasm-pack build target to "web"/"bundler" for browser consumers, which could affect the Node.js/mcp-server side relying on the current target -- a real architectural decision, explicitly scoped OUT of this plan as a separate follow-up per user agreement. All of Tasks 1-5 (the actual OCR bridge feature) are complete, reviewed, and committed independent of this Task 6 gap.

# ============================================================
# F1-F6: wasm backend + mcp + UI follow-ups (plan mode) — SDD Progress
# Plan: C:\Users\NMarchitecte\.claude\plans\tender-enchanting-aho.md
# ============================================================
Investigated uncommitted state from another session working the same worktree ("i've work on it somewhere else") -- found and verified substantial real progress on F3/F5 (browser loading via wasm-pack "web" target + hand-patched libc env shims; mcp-server actually starting). All work now committed across 7 commits (5552ad8f22, 383ad4c10c, 41e72c1b5f, 454b0058f4, 0cff075747, 86073775a7, 3682371c7d):
- F1/F2 (mine, resumed): scripts/ensure-wasm-mods.mjs + clean-pkg-gitignore.mjs wired into every wasm-pack build script and the alef.toml before-hook, so the lib.rs mod wiring and .gitignore trap both self-heal on every build. redaction-rehydrate feature enabled in all build scripts (encrypt_map/decrypt_map are real, load-bearing API -- worker + mcp-server both call them).
- F3 (other session, verified + extended): browser loading fixed via wasm-pack "web" target (not "bundler" -- that failed webpack's build-time wasm parse on the 95MB binary) + scripts/patch-web-env.mjs providing real libc shim implementations (strcmp/memchr/iswspace/etc, confirmed genuinely needed by tracing generated code) instead of my earlier stub-that-throws. engine.worker.ts now calls the "web" target's required init(). Added transpilePackages for onnxruntime-web/@huggingface/transformers and a sharp:false alias (2 more real, previously-undiscovered blockers).
- F4 (mine): predev/prebuild wiring for xberg-wasm-runtime in both web-ui and mcp-server package.json.
- F5 (other session, confirmed via mcp.err log): mcp-server built and started successfully -- engine initialized, HTTP/SSE transport up, UI served.
- Extra real fixes found along the way: xberg core's default_extraction_timeout() now None without tokio-runtime feature (wasm has no tokio runtime to enforce it); xberg-wasm-runtime's selectModelBackend() gained a forceWasmBackend option (WebGPU can be present-but-non-functional in constrained browser contexts, hangs silently rather than erroring) with ner.ts fixed to actually pass config through (embedder.ts already did); createVectorStore() falls back to in-memory store when OPFS throws instead of blocking factory init entirely; discovered and fixed the ROOT CAUSE of the OPFS/onnxruntime-web hang -- missing Cross-Origin-Opener-Policy/Cross-Origin-Embedder-Policy headers (added to next.config.js headers() for dev parity; mcp-server's static-server.ts already had them for production).
- Added /ui/wasm-self-test route + e2e/wasm-self-test.spec.ts + e2e/ingest-dev.spec.ts (other session) as isolated verification harnesses for the browser wasm path, separate from the folder-dialog UI's pre-existing unrelated hydration bug.

UNRESOLVED / inconclusive: the wasm-self-test e2e spec times out at 280s waiting for the embedder model (bge-m3) to finish loading via WASM-CPU inference in this sandboxed environment -- succeeded ONCE (visible in accumulated console logs, `[factory] injection descriptor created`) but every subsequent attempt (including fresh Playwright contexts, cleared caches) hung at the identical point with zero new logs and zero network requests. Pattern (works once, then consistently hangs, no errors) is more consistent with this specific sandboxed environment being too CPU/resource-constrained to reliably finish loading a large quantized ML model within any timeout, than a remaining code bug -- the COOP/COEP fix itself is confirmed correct and necessary (fixed the OPFS store's own error, made crossOriginIsolated true). Needs verification on real (non-sandboxed) hardware/browser before trusting either way. mcp-server's own startup (F5) is a stronger, already-confirmed signal that the actual OCR/redaction stack works end-to-end server-side.

Also confirmed real, pre-existing, separately-scoped issue (not touched): production `next build` fails minifying onnxruntime-web's WebGPU/JSEP ESM module -- dev server works (no minification), documented in ingest-dev.spec.ts's own comment. Follow-up for whoever owns the production build path.
