# Aggregation Schema v2.12.0

This document describes the structure of `aggregated.json` produced by `benchmark-harness consolidate`.

## Top-level Shape

```json
{
  "schema_version": "2.12.0",
  "by_framework_mode": {
    "<aggregate_key>": {
      /* FrameworkModeAggregation */
    }
  },
  "disk_sizes": {
    "framework": {
      /* DiskSizeInfo */
    }
  },
  "comparison": {
    /* ComparisonData */
  },
  "per_fixture_results": [
    /* PerFixtureRow[] */
  ],
  "metadata": {
    /* ConsolidationMetadata */
  },
  "run_provenance": [
    /* RunProvenanceRecord[] — v2.8.0+, see "Migration from v2.7.0 to v2.8.0" */
  ],
  "failure_summary": {
    /* FailureSummary — v2.9.0+, see "FailureSummary" */
  },
  "format_support": {
    /* FormatSupportMatrix — v2.9.0+, see "FormatSupportMatrix" */
  },
  "layout_impact": [
    /* LayoutImpact[] — v2.12.0+, see "LayoutImpact" */
  ],
  "head_to_head": [
    /* HeadToHead[] — v2.12.0+, see "HeadToHead" */
  ]
}
```

## Output Format Discriminator

The `output_format` field determines:

- **`markdown`**: Supports all metrics including SF1 (structural F1), layout percentiles, and all ranking tables
- **`plaintext`**: Text-only extraction; SF1 and layout percentiles are `null`; plaintext frameworks never appear in SF1 rankings

## by_framework_mode

Key format differs by framework family:

- **xberg** (`xberg-*`): `{framework_name}:{mode}` — the output format is already encoded
  in the framework name (e.g. `xberg-markdown-baseline`), so repeating it in the key is
  redundant.
- **competitors** (all other frameworks): `{framework}:{output_format}:{mode}` — format is not
  encoded in the name, so the key carries it explicitly.

Examples:

- `xberg-markdown-baseline:single`
- `xberg-plaintext-paddle-ocr:batch`
- `unstructured:plaintext:single`
- `docling:markdown:single`

Each entry contains:

```json
{
  "framework": "string", // Framework name without mode suffix
  "output_format": "markdown|plaintext", // Output format used
  "mode": "single|batch|...", // Execution mode
  "timing_regime": "cold_process|warm_in_process|mixed|unknown", // Process lifecycle measured (v2.10.0+); mixed is aggregate-only, unknown is unranked
  "cold_start": {
    /* DurationPercentiles */
  }, // Optional, if cold start data available
  "by_file_type": {
    "pdf": {
      "file_type": "pdf",
      "no_ocr": {
        /* PerformancePercentiles */
      },
      "with_ocr": {
        /* PerformancePercentiles */
      }
    }
  }
}
```

## PerformancePercentiles

Contains p50, p95, p99 for all metrics — see "Percentiles" immediately below for the p95/p99
suppression and dispersion contract (v2.10.0+):

```json
{
  "successful_sample_count": 42,
  "performance_sample_count": 40, // invocation-level process samples used for duration/throughput/memory/cpu_seconds; repeated invocations mean this may exceed total_sample_count (v2.11.0+)
  "total_sample_count": 50,
  "framework_errors": 0,
  "harness_errors": 5,
  "timeouts": 3,
  "empty_content": 0,       // framework produced no content at all
  "zero_overlap": 0,        // framework produced non-empty output with zero token overlap vs a non-empty ground truth
  "error_details": {
    "error message": 2
  },
  "duration": { "p50": 100.5, "p95": 150.2, "p99": null, "sample_count": 40, "std_dev": 22.1 },
  "throughput": { "p50": 5.2, "p95": 4.8, "p99": 3.1, "sample_count": 40, "std_dev": 0.9 },
  "memory": { "p50": 150.0, "p95": 200.0, "p99": 250.0, "sample_count": 40, "std_dev": 18.4 },
  "extraction_duration": { "p50": 80.0, "p95": 120.0, "p99": 160.0, "sample_count": 40, "std_dev": 12.0 }, // Optional
  "quality": {
    /* QualityPercentiles */
  }, // Optional, if quality data available
  "success_rate_percent": 84.0,
  "pages_per_sec": { "p50": 12.5, "p95": 8.0, "p99": 5.0, "sample_count": 40, "std_dev": 1.1 }, // Optional (v2.7.0+, PDF only)
  "cpu_seconds": { "p50": 1.2, "p95": 2.1, "p99": 3.4, "sample_count": 38, "std_dev": 0.6 }, // v2.7.0+
  "batch_size": 8, // Optional (v2.7.0+; derived from real batch membership since v2.10.0+, see Migration)
  "system_load": {
    /* SystemLoadPercentiles */
  }, // Optional (v2.7.0+)
  "throughput_excluded_sample_count": 0, // v2.8.0+
  "memory_methods": { "rusage_max_rss": 24 } // samples per memory method behind `memory` (v2.12.0+)
}
```

## Percentiles (v2.10.0+ contract)

Each metric (`duration`, `throughput`, `memory`, `extraction_duration`, `pages_per_sec`,
`cpu_seconds`) is a `Percentiles` object:

```json
{ "p50": 100.5, "p95": 150.2, "p99": null, "sample_count": 40, "std_dev": 22.1 }
```

- **`p50`**: always reported (`0.0` when `sample_count == 0`).
- **`p95`**: `null` when `sample_count < 20` (`MIN_SAMPLES_FOR_P95`).
- **`p99`**: `null` when `sample_count < 100` (`MIN_SAMPLES_FOR_P99`).
- **`sample_count`**: how many values this group was computed from.
- **`std_dev`**: sample standard deviation (Bessel-corrected) of the underlying values; `0.0` for
  `sample_count <= 1`.

R-7 interpolation places a percentile at index `p * (sample_count - 1)`. Below roughly
`1 / (1 - p)` samples that index coincides with (or sits immediately next to) the maximum observed
value, so the "percentile" is really just the largest sample or two wearing a statistical label —
real benchmark cohorts are frequently 4-8 fixtures, well under either threshold. `p95`/`p99` are
`null` in that case instead of a fabricated number that looks precise but isn't, and `sample_count`
/`std_dev` are always present so a reader can judge the underlying distribution regardless (see
"Migration from v2.9.0 to v2.10.0").

## QualityPercentiles

Includes p50, p95, p99 for all F1 metrics. Layout percentiles are `null` for plaintext-only frameworks:

```json
{
  "f1_text_p50": 0.92,
  "f1_text_p95": 0.88,
  "f1_text_p99": 0.75,
  "f1_numeric_p50": 0.85,
  "f1_numeric_p95": 0.8,
  "f1_numeric_p99": 0.7,
  "f1_layout_p50": 0.78, // null for plaintext output format
  "f1_layout_p95": 0.72, // null for plaintext output format
  "f1_layout_p99": 0.65, // null for plaintext output format
  "quality_score_p50": 0.85,
  "quality_score_p95": 0.8,
  "quality_score_p99": 0.7,
  "reading_order_p50": 0.81, // null when no result carries a reading-order score (v2.12.0+)
  "reading_order_sample_count": 38 // results with a reading-order score (v2.12.0+)
}
```

`reading_order_p50` is the median of `quality.reading_order_score` over the successful results that
have one; results without it (too few anchor tokens, see `quality::reading_order_score`) are not
counted as 0. It is report-only and is not part of `quality_score`.

## SystemLoadPercentiles (v2.7.0+)

A contention qualifier aggregated from `BenchmarkResult.system_load` snapshots in the group.
`null` when no result in the group carries a snapshot. See the "Tier A comparative performance
metrics" migration notes below for how to read `load_per_core`.

```json
{
  "load_per_core_p50": 0.35,
  "load_per_core_p95": 0.9,
  "contended_sample_count": 3,
  "total_sample_count": 20
}
```

## PerFixtureRow

One row per unique combination of (framework, output_format, execution_mode, fixture_id, ocr).
The batch dedup that collapses a native batch to one `performance_sample_count` in
`PerformancePercentiles` does **not** apply here: every per-document row from a batch is present
(see "Migration from v2.7.0 to v2.8.0").

```json
{
  "framework": "xberg-markdown-baseline",
  "output_format": "markdown",
  "execution_mode": "single",
  "timing_regime": "cold_process",
  "ocr": false,
  "fixture_id": "sample_doc_1",
  "file_type": "pdf",
  "duration_ms": 125.4,
  "peak_memory_mb": 180.5,
  "memory_method": "rusage_max_rss", // how peak RSS was measured (v2.12.0+)
  "self_reported_peak_memory_bytes": null, // framework's own report, diagnostic only (v2.12.0+)
  "f1_text": 0.92,
  "f1_layout": 0.78, // null for plaintext mode
  "f1_numeric": 0.85,
  "quality_score": 0.85,
  "correct": true,
  "success": true,
  "error_kind": null, // "FrameworkError", "HarnessError", "Timeout", "EmptyContent", "ZeroOverlap", etc. if !success
  "file_size": 45210, // v2.8.0+
  "throughput_bytes_per_sec": 361280.5, // v2.8.0+
  "avg_cpu_percent": 42.1, // v2.8.0+
  "cpu_seconds": 0.31, // v2.8.0+
  "baseline_memory_bytes": 12582912, // v2.8.0+
  "peak_memory_delta_bytes": 176160768, // v2.8.0+
  "p50_memory_bytes": 150000000, // v2.8.0+, this single measurement's own sampler percentile
  "p95_memory_bytes": 175000000, // v2.8.0+
  "p99_memory_bytes": 189000000, // v2.8.0+
  "extraction_duration_ms": 98.0, // Optional, v2.8.0+
  "subprocess_overhead_ms": 27.4, // Optional, v2.8.0+
  "cold_start_duration_ms": 210.0, // Optional, v2.8.0+; omitted for warm steady-state batch lanes
  "error_message": null, // Optional free-text error, v2.8.0+
  "quality": {
    /* QualityMetrics, including missing_tokens/extra_tokens — Optional, v2.8.0+ */
  },
  "pdf_metadata": {
    /* PdfMetadata — Optional, v2.8.0+ */
  },
  "framework_capabilities": {
    /* FrameworkCapabilities, including batch_capability — v2.8.0+ */
  },
  "system_load": {
    /* SystemLoad — Optional, v2.8.0+ */
  },
  "iterations": [
    {
      "iteration": 1,
      "success": true,
      "error_kind": "none",
      "duration": {"secs": 0, "nanos": 125000000},
      "extraction_duration": {"secs": 0, "nanos": 98000000},
      "subprocess_overhead": {"secs": 0, "nanos": 27000000},
      "metrics": { /* PerformanceMetrics for this invocation */ },
      "batch_sample_id": "...", // Optional; shared by sibling rows from one native batch
      "batch_performance_sample": true // Optional; exactly one sibling owns process metrics
    }
  ],
  "statistics": {
    /* DurationStatistics — Optional, v2.8.0+ */
  }
}
```

`ocr` is `true` or `false` only when actual OCR usage is known. It is `null` when the framework did not report OCR usage.
`iterations` contains every measured invocation, including a one-iteration run. Aggregate
performance percentiles expand these raw samples instead of treating the row-level mean as one
observation.

The `duration_ms`/`peak_memory_mb`/`f1_*`/`quality_score`/`correct` fields are the original
v2.3.0 convenience projection and remain unchanged. The v2.8.0 fields above make each row
losslessly carry every measured field from its source `BenchmarkResult` — including ones with no
earlier scalar equivalent, like the free-text `error_message` or the token-level
`quality.missing_tokens`/`extra_tokens` lists.

## FailureSummary (v2.9.0+)

Top-level `failure_summary` rolls the per-framework-mode error counts (which also live on each
[`PerformancePercentiles`](#performancepercentiles)) up to the cohort level, preserving the
framework-fault vs infrastructure split throughout:

```json
{
  "total": { /* FailureCounts */ },
  "by_framework_mode": { "<aggregate_key>": { /* FailureCounts */ } },
  "by_file_type": { "docx": { /* FailureCounts */ } }
}
```

Each `FailureCounts` object:

```json
{
  "framework_errors": 0,     // framework hit a hard error on a supported document
  "empty_content": 0,        // framework produced no content at all for a supported document
  "zero_overlap": 0,         // framework produced non-empty output with zero token overlap vs a non-empty ground truth
  "timeouts": 0,             // framework exceeded the timeout on a supported document
  "framework_fault_total": 0, // sum of the four above; these penalize quality + success rate
  "harness_errors": 0,       // our harness failed (process crash, bad JSON): never penalizes
  "config_setup_errors": 0,  // our config/setup failed (missing deps): never penalizes
  "infra_total": 0           // sum of the two infrastructure kinds above
}
```

Framework-fault failures are the framework's own fault — it was handed a document in a format it
declares support for and failed — and are scored as quality 0 in the percentiles. Infrastructure
failures are our harness's fault and are excluded from both the quality percentiles and the
success-rate denominator. `total` counts every document; `by_framework_mode` and `by_file_type`
partition the same failures by aggregate key and by file extension respectively.

`empty_content` and `zero_overlap` are both framework-fault failures but mean different things:
`empty_content` is a framework that produced no content at all (set directly by the adapter that
ran it); `zero_overlap` is a framework that produced non-empty output that shares zero tokens with
a non-empty ground truth (set by the runner's post-scoring reclassification — see "Zero-overlap
reclassification" below). Both are excluded from quality percentiles and both still contribute
their duration/throughput/memory/cpu_seconds/pages_per_sec measurements to `PerformancePercentiles`
— see "Timing eligibility" below.

### Zero-overlap reclassification

A result that reports `success = true` / `error_kind = None` but whose text quality score
(`f1_score_text`) is `0.0` against a non-empty ground truth is not a legitimate "perfect failure"
quality sample — it is non-empty garbage output masquerading as a successful extraction. The runner
reclassifies it to `success = false` / `error_kind = "ZeroOverlap"` before aggregation so it is
excluded from quality percentiles/rankings but still counted against coverage/success-rate stats,
exactly like any other framework-fault failure. Fixtures with an empty (post-trim) ground truth are
never reclassified this way, since there is no signal to score against.

### Timing eligibility

`PerformancePercentiles.duration/throughput/memory/cpu_seconds/pages_per_sec` and
`performance_sample_count` are computed over *timing-eligible* results, not strictly successful
ones: a result is timing-eligible if `success == true`, or if it was reclassified to
`error_kind == "ZeroOverlap"`. In both cases the framework actually ran to completion and produced
output within the measured window, so the timing measurement is valid even though the *quality* is
disqualified. Excluding zero-overlap results from timing (as if gated on raw `success`) would bias
every framework's speed percentiles: a competitor that returns fast garbage on unsupported/degraded
input would have its fastest samples silently dropped, inflating its measured duration and
depressing its measured throughput relative to a framework that rarely produces zero-overlap
output. `ErrorKind::EmptyContent` results remain excluded from timing, unchanged from prior
behavior — a framework that produced literally nothing has no meaningful timing sample to
contribute.

## FormatSupportMatrix (v2.9.0+)

Top-level `format_support` distinguishes formats a framework declares unsupported from formats it
attempted and failed. It uses the same capability table that filters fixtures before execution.

```json
{
  "file_types": ["docx", "pdf", "rtf"],
  "unsupported": {
    "docling": ["rtf"],
    "liteparse": ["docx", "rtf"]
  }
}
```

`file_types` is the sorted set observed anywhere in the consolidated results. Release aggregates
include xberg's full-corpus run, so this is the complete corpus format set there. A competitor-only
aggregate cannot list a format that produced no result for any selected framework. `unsupported`
maps each logical framework to observed file types absent from its declared capabilities; xberg is
never listed because it is the full-corpus subject under test. Both fields default empty when older
v2.9.0 aggregates are deserialized.

## LayoutImpact (v2.12.0+)

Top-level `layout_impact` answers "what does the layout pass buy". One entry per
(layout framework, baseline framework, `output_format`, `mode`, `file_type`) where both
frameworks have rows. Frameworks pair by name: the variant's `layout` segment is replaced by
`baseline` (`xberg-markdown-layout` with `xberg-markdown-baseline`,
`xberg-markdown-layout-paddle` with `xberg-markdown-baseline-paddle`), or dropped when no such
framework exists (`xberg-markdown-sceptre-ort-layout` with `xberg-markdown-sceptre-ort`); a
trailing model-version segment (`-layout-v3`) is discarded as a last resort.

```json
{
  "layout_framework": "xberg-markdown-layout",
  "baseline_framework": "xberg-markdown-baseline",
  "output_format": "markdown",
  "mode": "single",
  "file_type": "pdf",
  "ocr": false, // shared by every successful row on both sides, else null
  "baseline_ocr_fixture_count": 0, // paired fixtures the baseline variant OCRed
  "layout_ocr_fixture_count": 0, // paired fixtures the layout variant OCRed
  "paired_fixture_count": 41, // fixtures where both variants succeeded
  "sf1": { "baseline_p50": 0.5, "layout_p50": 0.7, "delta_p50": 0.18, "sample_count": 41 }, // null for plaintext
  "tf1": { /* PairedDelta */ },
  "reading_order": { /* PairedDelta */ }, // null when no paired fixture carries a score
  "quality_score": { /* PairedDelta */ },
  "duration_ms": { /* PairedDelta */ },
  "peak_memory_mb": { /* PairedDelta */ },
  "improved_fixture_ids": ["doc_a"],
  "regressed_fixture_ids": ["doc_b"],
  "unchanged_fixture_ids": ["doc_c"],
  "per_fixture": {
    "sf1": [{ "fixture_id": "doc_a", "baseline": 0.5, "layout": 0.8, "delta": 0.3 }],
    "tf1": [], "reading_order": [], "duration_ms": [], "peak_memory_mb": []
  }
}
```

- **`PairedDelta`**: `baseline_p50` and `layout_p50` are the medians of each side's per-fixture
  values; `delta_p50` is the **median of the per-fixture deltas** (`layout - baseline`), not
  `layout_p50 - baseline_p50`. `sample_count` is the number of fixtures with a value on both sides;
  every field is `0` when it is `0`.
- **Pairing** happens by fixture inside one (`output_format`, `mode`, `file_type`) bucket and only
  for fixtures where both variants succeeded. A bucket whose layout rows all failed still gets an
  entry, with `paired_fixture_count: 0`.
- **OCR** is a property of each row, not part of the bucket: failed rows never report it
  (`ocr: null`), and an auto-OCR variant decides per document. `ocr` is the value every successful
  row on both sides reports, and `null` when they disagree, any did not report it, or none
  succeeded; `baseline_ocr_fixture_count` / `layout_ocr_fixture_count` count the paired fixtures
  on which each side reported OCR use.
- **Classification**: a fixture is `improved` when its delta is at least `0.005`, `regressed` at most
  `-0.005`, otherwise `unchanged`. The metric is SF1 for markdown (`quality_score` for plaintext, or
  when no paired fixture has SF1).
- **Timing**: `duration_ms` and `peak_memory_mb` use only single-file, cold-process rows
  (`peak_memory_mb` also requires an isolated-process measurement). A native batch has one
  process-level measurement shared by all documents, so batch buckets report `sample_count: 0` there
  while quality deltas are unaffected.

## HeadToHead (v2.12.0+)

Top-level `head_to_head` compares every xberg framework with every non-xberg framework on the
documents both succeeded on, one entry per (xberg framework, competitor framework,
`output_format`, `mode`, `file_type`) in which the two share at least one fixture id. OCR use does
not split entries (see LayoutImpact): a scan xberg auto-OCRed and the competitor read without OCR
is one shared fixture.

```json
{
  "xberg_framework": "xberg-markdown-layout",
  "competitor_framework": "liteparse",
  "output_format": "markdown",
  "mode": "single",
  "file_type": "pdf",
  "ocr": false, // shared by every successful row on both sides, else null
  "xberg_ocr_fixture_count": 0, // shared fixtures xberg OCRed
  "competitor_ocr_fixture_count": 0, // shared fixtures the competitor OCRed
  "shared_fixture_count": 3, // both succeeded
  "xberg_only_fixture_ids": ["doc_d"], // xberg succeeded, competitor absent or failed
  "competitor_only_fixture_ids": [], // competitor succeeded, xberg absent or failed
  "duration_ms": { /* PairedDelta */ },
  "extraction_duration_ms": { /* PairedDelta */ },
  "peak_memory_mb": { /* PairedDelta */ },
  "tf1": { /* PairedDelta */ },
  "sf1": { /* PairedDelta */ }, // null for plaintext
  "reading_order": { /* PairedDelta */ }, // null when no shared fixture carries a score
  "quality_score": { /* PairedDelta */ }
}
```

`PairedDelta` reads with `baseline` = competitor and `layout` = xberg, so a positive `delta_p50` on
a quality metric means xberg scored higher. All medians cover only the shared fixtures, so a
competitor that ran 3 of 4 documents is compared on those 3 rather than credited or penalised for
the fourth.

## Migration from v2.11.0 to v2.12.0

Additive only; 2.11.0 aggregates still deserialize (every new field defaults).

- **`QualityPercentiles.reading_order_p50` / `reading_order_sample_count`**: report-only reading
  order median and its sample count.
- **`ComparisonData.pdf_reading_order_ranking_markdown` / `_plaintext`**: coverage-adjusted, same
  convention as the other `pdf_*` quality rankings: an OCR bucket attempted and failed entirely
  contributes 0 at its accountable weight. Frameworks with no reading-order sample in any bucket
  are omitted rather than scored 0.
- **`layout_impact`** and **`head_to_head`**: see above.
- **Peak memory method**: `peak_memory_bytes` is now `ru_maxrss` from `wait4(2)` on the harness's
  direct child (normalised to bytes) for every subprocess framework, on unix. It includes every
  descendant the child reaped and is the peak of the largest single process in the tree, not a
  sum. xberg's self-reported peak no longer replaces the measurement; it is kept in
  `self_reported_peak_memory_bytes`. Rows carry `memory_method` (`rusage_max_rss`, `sampled`,
  `self_reported`), and `PerformancePercentiles.memory_methods` counts them per bucket. Memory
  figures before 2.12.0 compared an exact xberg self-report with a polled sampler that missed
  short-lived competitor processes; do not compare memory across that boundary.

## Migration from v2.10.0 to v2.11.0

Additive raw-artifact fields plus a corrected aggregation interpretation; no field was removed or
renamed.

- Performance distributions expand every retained `iterations` entry instead of treating a
  document row's mean as one observation. Therefore `performance_sample_count` and each metric's
  `sample_count` are invocation-scoped and may exceed document-scoped `total_sample_count` when
  multiple measured iterations were requested. Quality, coverage, and success counts remain
  document-scoped.
- Each iteration now carries optional `success`, `error_kind`, `subprocess_overhead`,
  `batch_sample_id`, and `batch_performance_sample` fields. Their absence preserves legacy row-level
  behavior when older artifacts are read.
- Warm steady-state batch rows omit `cold_start_duration`: a single-file CLI probe is not a batch
  cold-start measurement. Complete warmup batches are discarded, and any failed warmup aborts the
  run rather than allowing a possibly contaminated retained Engine into measurement.
- Xberg multi-result batch inputs fail closed. Neither the CLI envelope nor the typed Engine result
  exposes sufficient per-child accounting to collapse timing and metadata into one honest
  performance row.

## Migration from v2.9.0 to v2.10.0

Additive only — no key-format change, no field removed or renamed. Fixes four statistical-integrity
defects where a published number looked like a real measurement but was actually a harness
artifact (tiny-n interpolation, a monitoring-resolution floor, a coarse ratio, or a silent
disappearance).

- **`Percentiles.p95`/`Percentiles.p99`** (nullable, was always-present `f64`; new
  **`sample_count`** and **`std_dev`** fields): `p95` is `null` when the underlying sample count is
  below `MIN_SAMPLES_FOR_P95` (20); `p99` is `null` below `MIN_SAMPLES_FOR_P99` (100). Real
  benchmark cohorts are frequently 4-8 fixtures, at which R-7 interpolation for p95/p99 was
  effectively just reading off the maximum sample rather than a genuine tail percentile — a single
  outlier document silently became "the p99 tail." `sample_count` and `std_dev` are always present
  so a reader can judge the underlying distribution's size and dispersion even when p95/p99 are
  suppressed, and so a `0.0` `p50` on a degenerate (empty) group is distinguishable from a real
  `0.0` measurement. This is a **breaking change for consumers that read `p95`/`p99` as a bare
  number** — they must now handle `null`. See "Percentiles (v2.10.0+ contract)" above.
- **`PerformancePercentiles.cpu_seconds`**: a `cpu_seconds` reading of exactly `0.0` is now treated
  as "below measurement resolution" (the process ran with fewer than 2 resource-sampler ticks — see
  `integrate_cpu_core_seconds` in `monitoring.rs`), not a real "zero CPU-seconds consumed"
  measurement (physically impossible for a process that ran to completion). Such readings are
  excluded from the `cpu_seconds` distribution entirely, so `cpu_seconds.sample_count` can be lower
  than `performance_sample_count` for a framework whose runs are frequently too fast to sample.
- **`PerformancePercentiles.batch_size`**: now derived from real batch membership
  (`framework_capabilities.batch_sample_id`) instead of the coarse `total_sample_count /
  performance_sample_count` ratio. That ratio was only correct when every eligible row was its own
  batch of one; once some rows were excluded from timing eligibility for a reason unrelated to
  batch grouping (e.g. `EmptyContent`), it silently reported the inverse timing-eligibility rate
  instead of a document count — 10 single-file results with 5 timing-eligible used to publish a
  fictitious `batch_size` of 2. Single-file mode now always reports `Some(1)`.
- **`ComparisonData.unranked_frameworks`** (`UnrankedFramework[]`, new, defaults to `[]`): records
  every framework attempted but excluded from a ranking, with why — see "UnrankedFramework
  (v2.10.0+)" above. Covers both the pre-existing "zero performance samples" exclusion (previously
  silent) and the new cpu_seconds-floor exclusion above.
- **`ComparisonData.cpu_seconds_ranking`**: a framework whose every performance sample measured
  `0.0` cpu_seconds is now excluded from this ranking (previously it could win rank 1 with that
  physically impossible value, since ascending sort put `0.0` first) and recorded in
  `unranked_frameworks` instead. The pre-existing smallest-positive-value `reference_cpu_seconds`
  logic for `relative` is unchanged for the frameworks that remain.
- **`FrameworkModeAggregation.timing_regime` and `PerFixtureRow.timing_regime`**: identify whether
  a measurement launches a fresh process (`cold_process`) or follows a discarded warmup in the
  same retained process (`warm_in_process`). Warm in-process diagnostics remain in detailed output
  but are excluded from competitive rankings and shared-corpus comparisons so unlike lifecycle
  costs are never ranked together.
- **`FrameworkCapabilities.resource_measurement_scope`**: `isolated_process` means RSS/CPU target
  the framework process tree. `harness_process_latency_only` means an in-process adapter cannot
  isolate those resources; raw RSS/CPU fields are zero placeholders and aggregate memory/CPU
  percentiles have `sample_count: 0`. `unknown` is the fail-closed default when the field is
  absent. Duration and throughput remain measured. Cold release artifacts require
  `isolated_process` in provenance, raw results, and aggregate fixture rows.

## Migration from v2.7.0 to v2.8.0

Additive only — no key-format change, no field removed or renamed. Motivated by an aggregation
audit that found several measured fields from `results.json` (and the entire `provenance.json`
sidecar) were silently dropped during consolidation.

- **`run_provenance`** (`RunProvenanceRecord[]`, top-level, new): one entry per input directory
  the `consolidate` command read from, folding in the `provenance.json` sidecar that
  `load_run_results` previously ignored entirely. `aggregate_new_format` itself always leaves
  this empty (it has no filesystem access); the `consolidate` CLI command populates it. Shape:

  ```json
  {
    "source_dir": "xberg-markdown-baseline-batch",
    "provenance": {
      /* RunProvenance — see "Run provenance sidecar" below — or null */
    },
    "missing_reason": "provenance.json not found in ..." // Present only when provenance is null
  }
  ```

  A missing `provenance.json` is recorded, not treated as an error — see
  `consolidate::load_run_provenance`.
- **`PerFixtureRow`**: extended with `file_size`, `throughput_bytes_per_sec`, `avg_cpu_percent`,
  `cpu_seconds`, `baseline_memory_bytes`, `peak_memory_delta_bytes`, `p50/p95/p99_memory_bytes`,
  `extraction_duration_ms`, `subprocess_overhead_ms`, `cold_start_duration_ms`, `error_message`,
  `quality` (the full `QualityMetrics`, including `missing_tokens`/`extra_tokens`),
  `pdf_metadata`, `framework_capabilities` (including `batch_capability`), `system_load`,
  `iterations`, and `statistics`. See the `PerFixtureRow` section above for the full shape.
- **`PerformancePercentiles.throughput_excluded_sample_count`** (`usize`, new): the number of
  successful performance samples excluded from the `throughput` percentiles because their
  throughput was zero, negative, or non-finite. The exclusion rule itself is unchanged from
  pre-v2.8.0 behavior; this field only makes a previously-silent exclusion visible.
- **`ConsolidationMetadata.disk_size_conflicts`** (`string[]`, new, defaults to `[]`): human-
  readable notes for any framework where two or more results reported a different
  `installation_size`. `disk_sizes` has always kept only the last-seen value per framework; this
  field surfaces when that last-writer-wins behavior actually discarded a conflicting value.

## Migration from v2.6.0 to v2.7.0

- The published SF1 (`f1_layout`) now folds in the D6 table cell-content dimension
  (GriTS-Con), weighted `1.5` — equal to table topology. A table with the right grid
  but wrong cell text now scores lower than one with correct content. D6 was already
  computed and reported in prior versions; it is now part of the rollup, so SF1 values
  and PDF rankings shift downward for frameworks that reconstruct grids but garble cells.
  Table-less documents are unaffected (D6 is gated on table presence).

### Tier A comparative performance metrics (additive)

All fields below are additive to the `2.7.0` schema (no version bump, no key-format change).
Consumers built against the fields documented in "Migration from v2.5.0" and earlier remain
compatible: every new field is either optional (`null`/absent when not applicable) or has a
type-appropriate default (`0.0` for numeric percentiles) when deserializing older artifacts that
predate it.

- **`PerformancePercentiles.pages_per_sec`** (optional `Percentiles`): pages-per-second
  percentiles, derived from the harness-side (framework-agnostic) `PdfMetadata.page_count`
  divided by wall-clock duration — never a framework's self-reported page count, so every
  framework is compared against the same ground truth. `null` when no result in the group
  carries a known PDF page count (non-PDF file types, or a PDF the harness could not size). For
  a native batch, the page counts of every document sharing one batch invocation are summed
  before dividing by the batch's shared makespan, mirroring how `throughput` is computed for
  batches from summed bytes rather than a single member row.
- **`PerformancePercentiles.cpu_seconds`** (`Percentiles`, always present): total process-tree
  CPU-time percentiles, in core-seconds. Computed by trapezoidal integration of the resource
  sampler's per-sample CPU percentage over its timeline (see
  `PerformanceMetrics.cpu_seconds` doc comment). **Precision is bounded by the sampling
  interval** (1-10ms, adaptive on file size): CPU bursts shorter than the gap between two
  samples are smoothed by the trapezoidal average rather than measured exactly, so treat
  `cpu_seconds` as an approximation, not an exact accounting figure.
- **`PerformancePercentiles.batch_size`** (optional `usize`): the approximate number of
  documents processed per one measured process invocation in this group — `1` for single-file
  mode, or the modal per-batch document count for a native batch (`total_sample_count /
  performance_sample_count`, rounded). This lets peak-RSS (and any other performance metric
  already in this struct) be read "keyed by batch size" using the single-file-vs-batch mode
  entries the benchmark matrix already runs, without adding a new axis to the
  `by_framework_mode` aggregate key.
- **`PerformancePercentiles.system_load`** (optional `SystemLoadPercentiles`): surfaces the
  previously-captured-but-discarded `BenchmarkResult.system_load` as a contention qualifier —
  `null` when no result in the group carries a snapshot. Read `load_per_core` *relatively* (was
  this bucket measured under comparable contention to another) rather than as an absolute
  number; `contended_sample_count` counts samples whose 1-minute load average per logical core
  exceeded the harness's contention threshold.
- **`ComparisonData.pages_per_sec_ranking`** / **`cpu_seconds_ranking`** (`RankedFramework[]`):
  new rankings mirroring `throughput_ranking` and `memory_ranking` respectively — pages/sec
  ranked descending (higher is better), CPU-seconds ranked ascending (lower is better). Only
  frameworks with at least one pages/sec observation appear in `pages_per_sec_ranking`. Like
  `throughput_ranking` and `memory_ranking`, both are scored within each `(output_format, mode)`
  segment — see the `RankedFramework` section below.
- **`ComparisonData.pareto_frontier`** (`ParetoPoint[]`): the non-dominated frontier over
  (pages/sec ↑, SF1 ↑, peak-RSS ↓) for markdown frameworks that carry both an SF1 term and a
  pages/sec observation (plaintext-only frameworks never carry SF1, so they are never eligible;
  see `ParetoPoint`'s doc comment for the dominance rule). Pure computation over already-reported
  percentiles — no new capture-time data required.

## Migration from v2.5.0 to v2.6.0

- `PerFixtureRow.ocr` changed from a required boolean to a nullable boolean so unknown OCR usage is not mislabeled as `false`.

## ComparisonData

Contains all cross-framework rankings split by output format for quality metrics:

```json
{
  "throughput_ranking": [
    /* RankedFramework[] — segmented by (output_format, mode), see below */
  ],
  "memory_ranking": [
    /* RankedFramework[] — segmented by (output_format, mode), see below */
  ],
  "quality_ranking_markdown": [
    /* RankedFramework[] — markdown-only (combined quality with SF1 term) */
  ],
  "quality_ranking_plaintext": [
    /* RankedFramework[] — plaintext-only (combined quality, no SF1 term) */
  ],
  "pdf_quality_ranking_markdown": [
    /* RankedFramework[] — markdown-only, never plaintext */
  ],
  "pdf_quality_ranking_plaintext": [
    /* RankedFramework[] — plaintext-only */
  ],
  "pdf_tf1_ranking_markdown": [
    /* RankedFramework[] — markdown-only */
  ],
  "pdf_tf1_ranking_plaintext": [
    /* RankedFramework[] — plaintext-only */
  ],
  "pdf_sf1_ranking_markdown": [
    /* RankedFramework[] — markdown-only, never plaintext */
  ],
  "pdf_reading_order_ranking_markdown": [
    /* RankedFramework[] — markdown-only, coverage-adjusted median reading order (v2.12.0+) */
  ],
  "pdf_reading_order_ranking_plaintext": [
    /* RankedFramework[] — plaintext-only (v2.12.0+) */
  ],
  "pages_per_sec_ranking": [
    /* RankedFramework[] — descending, higher pages/sec first (v2.7.0+); segmented by
       (output_format, mode), see below */
  ],
  "cpu_seconds_ranking": [
    /* RankedFramework[] — ascending, lower CPU-seconds first (v2.7.0+); segmented by
       (output_format, mode), see below */
  ],
  "deltas_vs_baseline": {
    "<aggregate_key>": {
      /* DeltaMetrics, computed against this entry's own (output_format, mode) segment's
         highest-throughput row — not a single cross-segment baseline. The segment baseline row
         itself gets no entry (see RankedFramework below for why segments exist). */
    }
  },
  "pareto_frontier": [
    /* ParetoPoint[] — non-dominated (pages/sec, SF1, peak-RSS) points, markdown only (v2.7.0+) */
  ],
  "unranked_frameworks": [
    /* UnrankedFramework[] — attempted frameworks excluded from a ranking above, with why
       (v2.10.0+, see below) */
  ]
}
```

### UnrankedFramework (v2.10.0+)

A framework present in `by_framework_mode` (it ran) but absent from `throughput_ranking` /
`memory_ranking` / `cpu_seconds_ranking` / `pages_per_sec_ranking` / `pareto_frontier`, because it
had no usable performance sample for that ranking. Every one of those rankings is gated on having
at least one performance sample; before this field existed, a framework whose every result failed
or was reclassified (or whose cpu_seconds measurements all hit the monitoring-resolution floor —
see "Migration from v2.9.0 to v2.10.0") simply vanished from the charts with nothing recording it
was attempted. This makes that absence explicit:

```json
{
  "framework_mode": "docling:markdown:single",
  "reason": "no usable performance samples (12 total result(s), none timing-eligible); excluded from throughput_ranking, memory_ranking, cpu_seconds_ranking, pages_per_sec_ranking, and pareto_frontier"
}
```

A framework can appear once for a total exclusion (as above) or, separately, for a
`cpu_seconds_ranking`-only exclusion when it has usable performance samples for every other
ranking but every one of its cpu_seconds readings was `0.0` (see "Migration from v2.9.0 to
v2.10.0"); `reason` is free text, not a fixed enum, so read it rather than pattern-matching a
specific wording.

### RankedFramework

`throughput_ranking`, `memory_ranking`, `cpu_seconds_ranking`, and `pages_per_sec_ranking` are each
ranked and scored **within** a framework's own `(output_format, mode)` segment, not globally —
`"rank": 1` / `"relative": 1.0` mean "best among frameworks sharing this same output format and
mode," not "best overall." A markdown row pays a serialization cost a plaintext row never incurs,
and a native-batch row amortizes process startup over every document in the batch while a
single-file row pays it once per document — pooling either pair together would rank frameworks on
harness plumbing rather than real performance (mirroring why `quality_ranking_markdown` /
`quality_ranking_plaintext` are already split by format above; see `shared_corpus_markdown` /
`shared_corpus_plaintext` in `ConsolidationMetadata`). `output_format` and `mode` on each entry
self-identify which segment `rank`/`relative` are scoped to.

```json
{
  "framework_mode": "xberg-markdown-baseline:single",
  "rank": 1,
  "value": 95.5, // The metric value (duration, throughput, etc.)
  "relative": 1.0, // Ratio relative to the best value WITHIN this entry's own segment (1.0 = best)
  "output_format": "markdown", // Which segment rank/relative are scoped to (v2.9.0+)
  "mode": "single" // Which segment rank/relative are scoped to, e.g. "single"/"batch" (v2.9.0+)
}
```

### ParetoPoint (v2.7.0+)

One non-dominated point in the (pages/sec, SF1, peak-RSS) multi-objective comparison. A
candidate is on the frontier when no other candidate dominates it: dominance requires being at
least as good on every objective and strictly better on at least one. `pages_per_sec` and `sf1`
are maximized; `peak_memory_mb` is minimized.

```json
{
  "framework_mode": "xberg-markdown-layout:single",
  "pages_per_sec": 12.5,
  "sf1": 0.82,
  "peak_memory_mb": 320.0
}
```

## Migration from v2.3.0 to v2.4.0

### Breaking Changes

1. **Schema version**: Bumped to `"2.4.0"`
2. **Xberg aggregate key format**: Changed from `framework:output_format:mode` to
   `framework_name:mode` for all `xberg-*` frameworks. Competitor key format
   (`framework:output_format:mode`) is unchanged.

### Xberg Consolidation

Language-binding frameworks (`xberg-py`, `xberg-node`, `xberg-rb`, `xberg-go`,
`xberg-java`, `xberg-csharp`, `xberg-elixir`, `xberg-php`, `xberg-rust`, etc.)
have been removed. They are replaced by three native pipelines run directly via the xberg CLI:

| Pipeline  | Markdown name                   | Plaintext name                   |
| --------- | ------------------------------- | -------------------------------- |
| Baseline  | `xberg-markdown-baseline`   | `xberg-plaintext-baseline`   |
| Layout    | `xberg-markdown-layout`     | `xberg-plaintext-layout`     |
| PaddleOCR | `xberg-markdown-paddle-ocr` | `xberg-plaintext-paddle-ocr` |

Batch variants append `-batch` to the framework name (e.g. `xberg-markdown-baseline-batch`),
which the harness normalises to aggregate key `xberg-markdown-baseline:batch`.

### Run provenance sidecar

The `run` command writes `provenance.json` beside the backward-compatible `results.json` array.
Schema version 3 records the Xberg repository commit/dirty bit, the ordered fixture descriptors
and document BLAKE3 digests, cohort manifest identity, adapter versions and executable digests,
explicit model revision identities, timing configuration, fixed batch partitions, requested
workers, framework-specific worker semantics, the configured Xberg thread budget, and explicit
timing-regime and resource-measurement-scope metadata.
Local absolute paths are never serialized.

Since v2.8.0, the `consolidate` command reads every `provenance.json` sidecar it finds alongside
a `results.json` (via `consolidate::load_run_provenance`) and folds it into the aggregate's
top-level `run_provenance` array — see "Migration from v2.7.0 to v2.8.0" above. A directory with
`results.json` but no `provenance.json` is recorded with `provenance: null`, not treated as an
error.

For Xberg rows, `frameworks[].configured_thread_budget` records an explicit
`--xberg-max-threads` value in either mode. For native batch rows without an explicit
value, it records the legacy `--max-concurrent` fallback passed to `xberg batch
--max-threads`. It is distinct from
`frameworks[].requested_workers`, which records the `--max-concurrent` document-concurrency
cap. `effective_workers` remains `null` because Xberg resolves effective document concurrency
from the workload. Non-Xberg rows and automatic-budget single-file rows omit
`configured_thread_budget`.

#### Run provenance migration from schema 2 to schema 3

- Added required measurement-contract identity through `FrameworkProvenance.timing_regime` and
  `FrameworkProvenance.resource_measurement_scope`.
- Missing fields still deserialize as `unknown` for diagnostics, but schema-2 sidecars are rejected
  by release validation rather than being interpreted as cold, isolated-process measurements.
- Release artifacts require schema 3 with `cold_process` and `isolated_process`; warm in-process
  diagnostics remain unranked and are not release-contract inputs.

#### Run provenance migration from schema 1 to schema 2

- Added optional `FrameworkProvenance.configured_thread_budget`.
- Existing schema-1 sidecars remain readable; a missing field deserializes as `null`.
- Consumers should treat `null` as unavailable, not infer a thread budget from worker fields.
- Native-batch producers that omit `--xberg-max-threads` record the legacy fallback
  value from `--max-concurrent`; single-file producers omit the field.

### Key Format Rationale

The format component is implicit in the xberg framework name itself. Duplicating it in the
aggregate key (`xberg-markdown-baseline:markdown:single`) would be redundant and confusing.
Competitor names carry no format information, so they continue to need it in the key
(`docling:markdown:single`).

## Migration from v2.2.0 to v2.3.0

### Breaking Changes

1. **Schema version**: Bumped to `"2.3.0"`
2. **Framework key format**: Changed from `framework:mode` to `framework:output_format:mode`
3. **QualityPercentiles**: Added p95 and p99 percentiles for all F1 metrics; `f1_layout_*` fields are now optional (null for plaintext)
4. **FrameworkModeAggregation**: Added `output_format` field
5. **ComparisonData**: Replaced `pdf_tf1_ranking` with `pdf_tf1_ranking_markdown` and `pdf_tf1_ranking_plaintext`; `pdf_sf1_ranking` renamed to `pdf_sf1_ranking_markdown` (now markdown-only)

### New Fields

- `per_fixture_results`: Array of individual fixture results preserving per-file measurements
- `PerFixtureRow`: New struct capturing individual extraction outcomes

### Plaintext-only Behavior

- Plaintext frameworks NEVER appear in `pdf_sf1_ranking_markdown`
- Plaintext frameworks NEVER appear in `pdf_tf1_ranking_markdown` (they get their own `pdf_tf1_ranking_plaintext`)
- SF1 and layout percentiles are `null` for plaintext output format
- Performance rankings (`throughput_ranking`, `memory_ranking`, `cpu_seconds_ranking`,
  `pages_per_sec_ranking`) include both formats, but never pool them together: each is scored
  within its own `(output_format, mode)` segment (v2.9.0+, see `RankedFramework` above)

## ConsolidationMetadata

```json
{
  "total_results": 500,
  "framework_count": 5,
  "file_type_count": 8,
  "shared_corpus_markdown": ["pdf"],
  "shared_corpus_plaintext": ["pdf"],
  "timestamp": "2025-05-09T10:15:30Z",
  "disk_size_conflicts": [] // v2.8.0+, see "Migration from v2.7.0 to v2.8.0"
}
```

- **`framework_count`** counts *logical* frameworks: all `xberg-*` pipeline variants collapse to a
  single `xberg` before counting (so 7 competitors + xberg = 8, not 11).
- **`shared_corpus_markdown` / `shared_corpus_plaintext`** are the file types the "overall"
  `quality_ranking_markdown` / `quality_ranking_plaintext` are actually computed over — the
  intersection of file types every candidate framework of that format attempted. When a
  single-format framework (e.g. PDF-only `liteparse`/`mineru`) is in the pool, this collapses to
  that one type (e.g. `["pdf"]`), and the "overall" ranking must be read as that-type-only rather
  than a true all-format comparison.
