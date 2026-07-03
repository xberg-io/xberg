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
