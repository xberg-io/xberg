# Carry-Patch Register

These files are edited by both the fork and upstream (`xberg-io/xberg`). The `carry-patches.tsv` file is the authoritative machine-checkable list; this file adds resolution guidance for high-risk patches.

## On resync

### `crates/xberg-rag/src/query.rs`

**Our change:** Added `Graph` variant to the `RetrieveMode` enum, enabling graph-based retrieval alongside Vector, FullText, and Hybrid modes.

**Watch for:** Upstream added `scoring.rs` in the same crate and may have modified `RetrieveMode` enum or added new retrieval strategies. When merging, verify that the `Graph` variant does not conflict with upstream's changes and that any new upstream variants are incorporated.

### `crates/xberg/src/text/redaction/engine.rs`

**Our change:** 14 interleaved hunks through the `redact()` function and related helpers, adding support for redaction strategies, rehydration map capture, and multi-strategy token handling.

**Watch for:** This is the highest non-RAG merge risk. Upstream may have refactored or optimized the redaction engine. Apply changes carefully using a three-way merge; test thoroughly with existing redaction tests after resolving.

### `crates/xberg/src/text/ner/gline.rs`

**Our change:** 12 interleaved hunks modifying `GlineBackend` and `ensure_model`, adding support for alternative model loading paths and confidence tracking.

**Watch for:** Upstream may have added new NER capabilities or refactored the backend interface. Preserve both our model-loading logic and upstream's changes; if the interface diverged significantly, coordinate carefully to avoid breaking the NER pipeline.
