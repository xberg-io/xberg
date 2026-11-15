Independent read-only review of PR #2886

Reviewed head: 521a56aed99428edf7917fd0cbb49b6acc848def.
Comparison base: a844bdc89ccb78e5ed13af68cec4c852c056b536.
GitHub head was rechecked before recording this report and matched. All 12 files in the actual merge-base diff were inspected. Committed objects were read directly; no checkout or worktree mutation was needed.

[Medium] crates/xberg/src/extractors/pdf/ocr/read_reuse.rs:296 - bounded retention

The size check includes serde-skipped payloads only on the top-level ExtractedDocument, but insert stores a recursive read.clone() (line 98). A custom backend reporting OcrBackendType::Tesseract can return an otherwise small result with children[i].result.formatted_content or images[i].ocr_result.formatted_content containing an arbitrarily large String. Those nested strings are omitted by serde and are not added by this tuple, yet Clone copies them into the retained entry. Thus an entry can pass a small retention limit while retaining a much larger allocation; retention happens before downstream adoption and before finish filters contradicted pages.

Evidence: types/extraction.rs:134 derives both Clone and Serialize; :317 carries child documents; :472-474 skips formatted_content from serde; :511-519 recursively clones ArchiveEntry.result; :888 carries nested image OCR results. read_reuse.rs:174 permits custom Tesseract-compatible backends, and :188-190 uses builtin identity only to decide normalization, not to disallow their retention. The top-level raw-frame/usage exclusions at :285 do not reject nested results. extraction/derive/ocr_document.rs:35-37 returns immediately when document is absent, and adoption is after retention in pipeline.rs.

Fix: recursively account for every retained document's skipped heap payload, or conservatively refuse retention when children or nested image OCR results are present (including those reachable through internal documents). Add a regression with a tiny visible payload and an oversized nested formatted_content, asserting no entry is retained and the final backend read runs. Rule: requested bounded-retention-memory invariant; correctness-review, Bounded work: bound every allocation and buffer from the memory budget. This is a source-proven accounting omission; no runtime memory measurement or Rust reproducer was run.

Other audited paths:

- Source/table fidelity: hits move the complete ExtractedDocument rather than reconstructing it; per-page geometry correction, adoption, scoring, and table assembly remain after the wrapper. Stock normalization removes only discard_suspected_ocr_noise, which config_to_tesseract does not consume. Custom backends hash that policy without normalization.
- Identity: page number, backend name/version, Arc instance, canonical tessdata root, language and optional osd file contents, original/effective configuration, skipped security/acceleration fields, correction, and encoded-byte length/hash participate. Both pre-read and post-read revisions are checked. Changed config or encoding remains a miss; a mismatched key does not remove the original entry.
- Budget: both wrappers run after retained raster reservation. Changed effective decode limits change identity. Encoded bytes, not just RGB pixels, are hashed. Retention sum uses checked serialization writes, a finite cap, and one entry per page; the nested-payload omission above is the exception identified.
- Lifecycle: probe state is extraction-local, finish drops uncontradicted pages, take consumes an exact match once, and state is released with the extraction. Failed Result values are not inserted. Standard scan/orientation paths bypass retention.
- Cancellation/observers: cancellation checks bracket collection and hit publication; recognition retains its existing cancellation checks. Geometry/orientation observers and invalidated operation cacheability bypass keys. Runtime race testing was not performed.
- Retries: multi-stage pipeline routes retain stage execution; sparse wrapper call policies are unchanged. Scan/orientation read paths remain outside reuse, and retries after the wrapped raw read remain in their existing callers. No separate actionable retry defect established.

Verification:

GitHub CI Focused PR run 42903381353 is bound to the exact head and has exactly three completed successful checks: Focused pdf-ocr, Focused api-jobs, Focused nli. Independently read the PDF job log (130134959554): 41 one-test collection summaries and 41 one-test successful execution summaries, with no ignored cases in those summaries; script inventory is 31 new cases plus 10 legacy cases. The final log states 41 exact tests. The evidence-count validator was deliberately given 40 pass records and rejected them before accepting the complete 41-record log. This validates evidence counting, not local execution of the Rust tests or a negative control of the CI shell harness itself.

Evidence files beside this report: codex-521a56a-ci.json, codex-521a56a-pdf-ci.log, codex-521a56a.diff. No source files were edited; no local builds, Rust tests, model downloads, approvals, PR comments, merge, or pushes were performed. Basemind CLI and repository skills were consulted before code navigation. No subagent capability was available in this review session.

Limits: static review plus existing exact-head CI evidence. No independent compiled execution, RSS profiling, adversarial nested-payload runtime reproduction, or full feature/backend matrix. Serialization size is an accounting proxy, not a measured process-memory bound. CI's source/table and real-stock controls were inspected but were not independently rerun. No claim of zero defects outside the requested scope.

Counts: High 0, Medium 1, Low 0.
