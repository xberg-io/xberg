# WASM MCP Server — performance baseline

**Sub-project:** WASM-backed MCP server (`mcp-server/`) on `@xberg-io/xberg-wasm` + `xberg-wasm-runtime`.
**Bench file:** [`mcp-server/benchmarks/engine_vs_native.bench.ts`](../../../mcp-server/benchmarks/engine_vs_native.bench.ts)

## What this measures

Steady-state latency of the wasm engine's three hot paths, with the engine and the
384-dim embedder model **pre-warmed once** in `beforeAll` so per-iteration timings
exclude the one-off model download/load:

- `engine.extract` — extraction of a 284-byte `text/plain` document.
- `engine.ingest` — extract + embed + store one document.
- `engine.query` — embed a query + vector-retrieve (`top_k = 5`).

## No native comparison arm (by design)

The plan's optional "vs native" arm (`@xberg-io/xberg` + `xberg-rag-node`) is **omitted**:
the native NAPI binding is not built in this worktree, so there is no in-process native
path to time against. The whole point of the migration is that the MCP server no longer
depends on that binding; a like-for-like native number would require checking out the
pre-migration server and building the `.node` artifact, which is out of scope here.

## Numbers

Captured 2026-07-09, macOS (darwin-x64), model cache on an external-SSD APFS image.
Steady-state: engine + 384-dim embedder warmed once; each figure is over 20 iterations
after 3 warmup iterations, via a `performance.now()` harness (Vitest 1.6.1's experimental
`bench()` under-reports `async` functions — 0 samples — so it is not used for the numbers).

| Operation | median (ms) | mean (ms) | min–max (ms) | notes |
|---|---|---|---|---|
| `engine.extract` (284 B text) | 0.21 | 0.27 | 0.19–0.91 | pure wasm text parse |
| `engine.ingest` (extract+embed+store) | 14.64 | 14.56 | 11.6–24.2 | dominated by embedding inference |
| `engine.query` (embed+retrieve, k=5) | 3.09 | 3.70 | 2.6–13.3 | embed query + cosine over a small collection |

Reading: `extract` is sub-millisecond (in-wasm parse of a small text doc). `ingest` and
`query` are governed by embedder inference (transformers.js on CPU), not the store
backend — consistent with the expectation below. These are relative, machine- and
cache-dependent figures for regression tracking, not absolute SLAs.


## How to capture numbers (stable environment)

1. Remount the model-cache volume if it uses the external SSD image
   (`hdiutil attach "/Volumes/Extreme SSD/xberg-build.sparsebundle"`), or point the
   HF/transformers cache at a local path with a warm copy of the 384-dim embedder model.
2. `source /Volumes/xberg-build/env.sh` (sets `NODE_OPTIONS=--dns-result-order=ipv4first`;
   needed only for the first, online model fetch).
3. Reliable harness (drop-in Vitest test, `performance.now()` — median of 20 iterations
   after 3 warmup iterations, per path):

   ```ts
   // pre-warm: await initializeEngine(); ensure a 384-dim collection; ingest one doc.
   async function time(n, f) { for (let i=0;i<3;i++) await f();
     const t=[]; for (let i=0;i<n;i++){const a=performance.now(); await f(); t.push(performance.now()-a);}
     const s=t.sort((x,y)=>x-y); return { median: s[n>>1], mean: t.reduce((x,y)=>x+y)/n }; }
   // time(20, () => engine.extract(input, { extraction_timeout_secs: null }))
   // time(20, () => engine.ingest(doc, "bench_col"))
   // time(20, () => engine.query("…", "bench_col", 5))
   ```

Record the median/mean per path in the **Numbers** table above and commit.

## Expectations

No strict target is defined; this baseline is for regression tracking. `extract` is pure
wasm CPU work; `ingest`/`query` are dominated by embedding inference (transformers.js on
the same ORT/CPU path regardless of the wasm vs native store), so those two should be
governed by embedder latency, not the store backend.
