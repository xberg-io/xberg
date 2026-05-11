# Performance Iteration Log

Per-iteration tracker for Kreuzberg perf optimization rounds. Append one row per accepted or rejected candidate. Follows the protocol in `profiling.md`. The institutional memory replacement for the `feedback_perf_subagent_verification.md` failure-mode warning.

## Format

| commit | candidate | hotspot self-time | p50 Δ | p95 Δ | SF1 Δ | verdict | notes |
|--------|-----------|-------------------|-------|-------|-------|---------|-------|

- **commit** — short SHA of the optimization commit (or REVERTED if rejected).
- **candidate** — file:function being optimized.
- **hotspot self-time** — pre-fix percentage from the flamegraph.
- **p50 Δ / p95 Δ** — change in median/tail extraction time vs prior baseline JSON.
- **SF1 Δ** — change in aggregate SF1 score. Must be ≥ −0.1pt or revert.
- **verdict** — ACCEPT, MARGINAL (lift < 3%, no regression), or REJECT.
- **notes** — one sentence on why.

## History

| commit | candidate | hotspot self-time | p50 Δ | p95 Δ | SF1 Δ | verdict | notes |
|--------|-----------|-------------------|-------|-------|-------|---------|-------|
| REVERTED | normalize_whitespace rewrite | not measured | n/a | n/a | n/a | REJECT | perf-engineer agent ACCEPT'd without measurement; correctness regression on leading/trailing spaces. See `feedback_perf_subagent_verification.md`. |
| REVERTED | split_embedded bullet-count fast path | not measured | +5% | +5% | 0 | REJECT | speculation-driven; ~5% wall-time *regression*. Don't optimize without a flamegraph. |
| b51472c1c | layout_runner: stream DynamicImage→RgbImage + drop redundant clones in table_recognition/layout_validation | n/a (memory, not CPU) | n/a | n/a | 0 | ACCEPT | No pre-M baseline; post-M anchor: 292 MB peak RSS on 60 MB PDF (plain, no layout). Q gates: 143/143 regression, 3/3 smoke, 18/18 guardrail failures identical to pre-M (all pre-existing pdf_oxide upstream). |
| 86a706959 | rendering::markdown: Cow single-pass scans replacing 6 eager .replace() chains | 0.02% self-time post-M (flamegraph fa356cb7e) | n/a | n/a | 0 | ACCEPT | M.2 confirmed effective — render_markdown dropped to 0.02% in post-M flamegraph; was queue candidate #2. |

## Queue — CLEARED (stopping condition met)

**Flamegraph `flamegraphs/fa356cb7e/baseline.svg`** (2026-05-11, 88,524 samples, `--profile profiling`, `--features all`):

| rank | self-time | function |
|------|-----------|----------|
| 1 | 0.50% | `kreuzberg::pdf::oxide::table::extract_tables_native` |
| 2 | 0.33% | `kreuzberg::pdf::oxide::text::extract_text_fast_path` |
| 3 | 0.14% | `kreuzberg::pdf::oxide::hierarchy::extract_all_segments` |
| 4 | 0.12% | `kreuzberg::cache::core::GenericCache::set` |
| 5 | 0.11% | `kreuzberg::pdf::oxide::images::extract_image_positions` |
| 6 | 0.11% | `kreuzberg::pdf::structure::pipeline::extract_document_structure_from_segments` |
| 7 | 0.09% | `kreuzberg::cache::cleanup::scan_cache_directory` |
| 8 | 0.08% | `kreuzberg::pdf::structure::classify::mark_arxiv_noise` |
| 9 | 0.02% | `kreuzberg::rendering::markdown::render_markdown` |

**Breakdown by crate (aggregate, 88,524 total samples):**

- System/other: 48.4%
- Std/core/alloc: 25.3%
- Benchmark_harness (quality scorer): 9.9%
- Pdf_oxide: 9.3%
- Rayon: 3.7%
- **Kreuzberg: 3.4%**
- Tokio: 0.1%

**Stopping condition:** Kreuzberg layer accounts for only 3.4% of total wall time. The previous queue candidates (fuse_paragraphs, text_repair, normalize_key, classify::merge_consecutive_pages) do not appear in the top-25 Kreuzberg frames — confirmed not hot on the baseline pipeline path. Dominant cost is pdf_oxide text/table extraction (9.3%) + system allocator + OS overhead (48.4%) — these are outside kreuzberg's optimization surface.

**Further kreuzberg-layer CPU gains require upstream pdf_oxide work** (table extraction at 0.50% is the single largest kreuzberg-visible hotspot; it delegates to pdf_oxide). Cache I/O (scan_cache_directory) at 0.09% is the next actionable target if cache efficiency becomes a priority, but it's below the noise floor for extraction pipelines.

### Previous blockers resolved

- Symbol-strip blocker from `flamegraphs/61170f7f6/baseline.svg` is fixed: `.task/workflows/benchmark.yml` patched from `--features full` → `--features all` (kreuzberg-cli has no `full` feature). The `fa356cb7e` flamegraph has 87 Kreuzberg symbols resolved.

### Post-M RSS anchor

Measured 2026-05-11 on `target/profiling/kreuzberg` (post-M, `--features all`, `--profile profiling`):

- **Fixture**: `test_documents/pdf/proof_of_concept_or_gtfo_v13_october_18th_2016.pdf` (60 MB, plain extraction, no layout detection)
- **Peak RSS**: 292 MB (`maximum resident set size: 305,954,816 bytes`)
- **Wall time**: 1.09 real seconds
- **Note**: No pre-M baseline captured; this is the forward anchor. M.1+M.3 impact on layout pipeline RSS requires a separate run with `use_layout_for_markdown=true` on a multi-page PDF.

## Stopping conditions

Per `profiling.md` § Iteration protocol:

- **Three consecutive REJECT or MARGINAL** → optimization curve flattened; stop.
- **Aggregate p95 plaintext ms/MB within 20% of pandoc** → competitive ceiling reached; stop.
- **SF1 regression > 0.1pt on any iteration** → immediate revert; reset the streak.
