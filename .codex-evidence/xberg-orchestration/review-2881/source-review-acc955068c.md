# Independent source review: PR #2881

Reviewed head: `acc955068c81bc5f21fec2360c9218628f8f691c`.
Base/merge base: `a844bdc89ccb78e5ed13af68cec4c852c056b536`.
Scope: all four changed files (175 insertions, 21 deletions), original closure, actual caller, admission population, module/feature gates, underlying structure predicates, and existing structure tests. Read AGENTS.md, coordination, Basemind, testing-conventions, and PDF-backends skills. Basemind index describes main, so exact-head conclusions use immutable Git blobs, not indexed/current-worktree source. No worktree was necessary and no foreign worktree was accessed.

[Medium] crates/xberg/src/extractors/pdf/ocr/pipeline.rs:2907 - correctness / feature gating

The new non-native-OCR fallback references `super::native_word_coverage`, but `ocr/mod.rs:44-45` declares that module only under `all(feature = "ocr", not(target_arch = "wasm32"))`. This fallback is explicitly selected under the negation of that condition. The enclosing function at pipeline.rs:725-734 is also compiled for `pdf + ocr-pipeline` without `ocr`; Cargo.toml:226-237 establishes this as the supported Tesseract-free pipeline feature. Consequently that combination (and wasm32 PDF OCR pipeline builds) now contains an unresolved module reference even if the final loop would be empty at runtime. Previously its plain predicate directly referenced the available `document` module.

Fix: retain `super::document::plain_ocr_page_can_yield_to_native(doc, text)` in the fallback branch. Alternatively place the extracted predicate in a module available under the enclosing function's gate and correctly gate its native-only dependencies. Add a feature-compilation regression gate for `pdf,ocr-pipeline` without `ocr`, and the relevant wasm32 feature leg. No compilation was run, as requested; severity is conservatively Medium for the source-derived build failure.

Rule: review correctness/type safety and AGENTS verification-before-completion; the refactor must retain validity for the caller's existing feature configurations.

## Native predicate equivalence and actual wiring

The original closure at base pipeline.rs:2894 onward and head native_word_coverage.rs:684-700 match after normalizing whitespace, optional trailing argument commas, the text/read parameter rename, and removed same-module qualifiers. A Python source comparison asserted equality on the exact immutable blobs. Its in-memory negative control removed the admission filter and confirmed inequality. The first comparison rejected a formatting-only trailing comma difference; normalization was narrowed to account for that syntactic difference before rerunning. This is a static comparison, not execution of Rust behavior or a Rust mutation test.

- Plain acceptance is first and can return true without either ownership set.
- Words-covered membership returns the certified-plain-payload AND exact-text result immediately. A false result does not fall through to ink ownership, including when both sets contain the page.
- Ink ownership is still required for the segment/table routes. All arguments are immutable borrows; passing `Option<&Arc<AHashMap<...>>>` copies a reference-bearing Option and does not transfer the Arc/map or mutate membership.
- Head pipeline.rs:2891-2909 still zips the actual structured document and raw OCR read. Either absent => false without calling the predicate.
- Head pipeline.rs:2896-2903 passes the actual native_words_covered_pages, native_ink_owned_pages, native_coverage_layers and clean_native_ink_pages, with the loop's actual page number. No constant or surrogate admission flag is used.
- clean_native_ink_pages is a local HashSet populated at pipeline.rs:2650-2676 only when the first-site admission succeeds and its marker-free coverage layer exists. The extracted helper uses `.filter(|_| clean_native_ink_pages.contains(&page))` before `.get(&page)`, preserving same-page admission and lookup.
- Missing coverage Option, absent same-page map entry, or absent same-page clean admission => clean_ink_layer None. native_word_coverage.rs:657-666 then requires exact table/read inventory rather than allowing omitted words. Marker-bearing Some layers take the same strict route. The exact non-table segmentation alternative remains unchanged.
- `complete_native_pages` iteration, formula ownership (`None` formula page protects every page), automatic-preservation eligibility, and removal of replacement/structured/confidence/preprocessing entries remain with the caller and unchanged (pipeline.rs:2890-2917).

## New tests and controls, reviewed from source

The nine new #[test] declarations are wired at pipeline.rs:9007-9009 under test+pdf+ocr+non-wasm, with no ignored tests. They construct real documents, a real table and real page sets. The default fixture contains ink ownership for page 7, clean admission only for page 3, and a same-page coverage layer; its structured body omits "apples" from the raw read. Actual table words are included in both read and structure.

| Case | Expected | What it distinguishes |
| --- | --- | --- |
| Omitted body word, same-page coverage, only other-page admission | false | Removing the admission filter would permit the omission |
| Add same-page clean admission | true | Overly strict rejection / always-false helper |
| Restore exact body inventory without clean admission | true | Exact None fallback must remain usable |
| Same-page admission but only other-page layer | false | A lookup must use this page |
| Add words-covered membership alongside clean ink ownership | false | Words-covered must return before ink branch |
| Remove ink ownership | false | Clean admission alone cannot authorize table replacement |
| Add non-private metadata | false | Rich payload still protects OCR |
| Add an uncertified retained word | false | Clean admission does not authorize invented content |
| Exact plain document, no ink ownership | true | Plain acceptance precedes ownership checks |

Existing native_word_coverage_structure_tests.rs:99-123 directly covers `None` and marker-bearing layers, both omitted-word rejection and exact-inventory acceptance. The new wrapper tests exercise effective None through missing admission and missing same-page lookup, but never pass the coverage Option itself as None. They do not execute the entire mixed pipeline, the real ink proof, or mutation-test the actual production call-site wiring. Those are review limits, not additional defects established by this diff. Reading the production caller and admission population supplies the wiring evidence beyond those unit tests.

## Limits and disposition

Read-only source review; only this requested untracked evidence report was written. No source edits, compiled checks, test runs, dependency downloads, Git mutation, approval, merge, push, or external messages. No independent execution of the reported 90 tests or nine prepared mutation controls; three green CI checks are caller-supplied context and were not independently fetched or treated as semantic proof. The source comparison checked nonempty extracted bodies and the exact nine new test declarations, with the admission-filter negative control described above.

Native predicate extraction is equivalent; feature-gated fallback is not valid across the existing caller configurations. Claude integrates any fix and verification.

Counts: High 0; Medium 1; Low 0.
