# Xberg WASM Runtime: Embedding Batch Concurrency Benchmark — Design Spec

**Date:** 2026-07-07
**Status:** Draft (benchmark-first, not yet approved for planning)
**Scope:** Sub-project **C** (`packages/xberg-wasm-runtime`), specifically `embedder.ts`'s internal batching strategy.
**Depends on:** Nothing structurally — this can run independently of the SQLite store work. Best done after [`2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md`](../plans/2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md)'s Task 9 (embedding cache) lands, since the cache changes what "batch of texts" actually reaches the model (cache hits never reach `extractor()` at all), and the benchmark should measure the real post-cache workload.

## Context

`embedder.ts` processes texts in batches of `DEFAULT_BATCH_SIZE = 32`, awaiting each batch fully before starting the next (`for` loop with a sequential `await`, not `Promise.all`). This is deliberate — a prior fix-round subagent tried to "optimize" this into `Promise.all` to satisfy a lint rule, and the change was caught and reverted because it broke output ordering (results from concurrently-resolving batches landed in resolution-timing order, not input order) and defeated the memory-bounding comment's stated intent ("process in batches to manage memory").

That revert was correct — the sequential version is the only *safe* option that was actually verified. But "sequential" and "safe" aren't the same claim as "fastest safe option." A documentation-grounded research pass found **no transformers.js-specific guidance either way** on whether concurrent batch calls are safe, and only indirect evidence from generic ONNX Runtime docs suggesting `Run()` is thread-safe for concurrent calls on CPU (not GPU, and not transformers.js's own wrapper behavior specifically). This spec exists because "we don't know, so we picked the safe option" is a reasonable place to stop, but it leaves real throughput on the table if bounded concurrency turns out to be both safe and meaningfully faster — and the only way to know is to measure, not guess.

## What This Is NOT

This is explicitly **not** a plan to change `embedder.ts`'s production code yet. It's a spec for a benchmark that produces evidence either:
(a) bounded concurrency is measurably faster and doesn't corrupt output order or blow memory, in which case a follow-up plan implements it with the same order-preservation guarantee the current sequential code has, or
(b) it isn't meaningfully faster (e.g. the underlying ONNX Runtime session already serializes `Run()` calls internally, or WASM's single-threaded-by-default nature in Node makes "concurrent" `await`s a no-op), in which case the sequential implementation is confirmed correct AND confirmed non-wasteful, and this line of investigation closes.

## Benchmark Design

**Files (when planned):**
- Create: `packages/xberg-wasm-runtime/bench/embedder-batch-concurrency.bench.ts` (a `vitest bench` file, not a regular test — this repo's `vitest` version supports `bench()`; confirm the installed version's bench API before planning, per `vitest.config.ts`'s current `^4.1.9`).

**What to measure, precisely:**

1. **Baseline**: current sequential implementation, embedding a fixed corpus of N texts (suggest N=200, short sentences, to get several batches of 32 without an unreasonably long benchmark run) — wall-clock time, peak memory (`process.memoryUsage().heapUsed` sampled during the run, not just before/after).

2. **Bounded concurrency variant**: a modified `embed()` that runs at most C batches concurrently (try C=2 and C=4 as two separate variants) via a semaphore, writing each batch's results into a **pre-sized output array indexed by original position** (not `.push()`) — this is the specific correctness fix that makes concurrency safe regardless of resolution order, and any implementation that skips this indexing and uses `.push()` instead should be rejected outright regardless of its speed, since it reintroduces exactly the bug that was reverted.

3. **Fully concurrent (`Promise.all` over all batches at once)**: included as a reference point / worst-case-memory data point, even though it's the version already known to be unsafe for the `.push()` ordering reason — with the indexed-array fix applied, it becomes safe again for *ordering*, but this variant specifically tests whether unbounded concurrency causes real memory pressure (many batches' tensor outputs resident simultaneously), which is the other half of why the original code was sequential.

**Success criteria for moving to a follow-up plan:**
- A bounded-concurrency variant (C=2 or C=4) must be at least 20% faster wall-clock than the sequential baseline to be worth the added complexity (semaphore logic, indexed-write correctness surface) — a marginal 5% gain isn't worth the risk given this exact code path already caused one real regression this session.
- Peak memory for the winning variant must not exceed roughly `C ×` the sequential baseline's peak (i.e., confirm memory scales with concurrency as expected, not worse) — if it's wildly higher than that (suggesting some other retention issue), that's a red flag to investigate rather than a green light to ship.
- Must run this benchmark against BOTH the Node/onnxruntime-node path (available now) and, ideally, a real browser WebGPU/WASM-threaded path — but if a real-browser benchmark harness doesn't exist yet, Node-only results are an acceptable first pass; the browser numbers may differ significantly given WASM's threading model differs from Node's, and that caveat should be stated explicitly rather than assuming Node results generalize.

## Non-Goals

- Making the embedding cache (prerequisite plan's Task 9) itself concurrent — that's a separate, much simpler question (Map reads/writes) not covered here.
- Changing `ner.ts`'s batching (NER doesn't currently batch texts the way `embedder.ts` does — check its current implementation before assuming this benchmark's findings transfer).
- Any change to `DEFAULT_BATCH_SIZE` itself (32) — that's a different tuning question (per-batch size vs. concurrent batch count) and should stay fixed while this benchmark varies concurrency, to keep the experiment isolated to one variable.

## Open Questions for the Planning Phase

1. Is `vitest bench` the right tool, or should this be a standalone Node script outside the test suite (to avoid vitest's own overhead skewing wall-clock measurements)? Lean toward a standalone script for the actual timing runs, with `vitest bench` only if its own overhead is confirmed negligible.
2. Should the benchmark corpus be realistic text (e.g. sampled from an actual document corpus) or synthetic (repeated short phrases)? Realistic is more representative but risks flakiness from real model behavior on edge-case text; synthetic is more reproducible. Lean toward a small, fixed, checked-in realistic sample (not generated at benchmark time) for reproducibility.
3. If results are inconclusive (e.g. gains within noise, or results differ significantly between runs), the honest conclusion is "stay sequential, insufficient evidence to change a working, correctness-verified implementation" — this should be stated as an acceptable, non-failure outcome up front, not treated as the benchmark having "failed."
