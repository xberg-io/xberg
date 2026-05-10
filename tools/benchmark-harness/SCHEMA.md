# Aggregation Schema v2.3.0

This document describes the structure of `aggregated.json` produced by `benchmark-harness consolidate`.

## Top-level Shape

```json
{
  "schema_version": "2.3.0",
  "by_framework_mode": {
    "framework:output_format:execution_mode": { /* FrameworkModeAggregation */ }
  },
  "disk_sizes": {
    "framework": { /* DiskSizeInfo */ }
  },
  "comparison": { /* ComparisonData */ },
  "per_fixture_results": [ /* PerFixtureRow[] */ ],
  "metadata": { /* ConsolidationMetadata */ }
}
```

## Output Format Discriminator

The `output_format` field determines:
- **`markdown`**: Supports all metrics including SF1 (structural F1), layout percentiles, and all ranking tables
- **`plaintext`**: Text-only extraction; SF1 and layout percentiles are `null`; plaintext frameworks never appear in SF1 rankings

## by_framework_mode

Key format: `{framework}:{output_format}:{execution_mode}`

Examples:
- `kreuzberg-rust:markdown:single`
- `pdfplumber-python:plaintext:batch`
- `docling:markdown:single`

Each entry contains:
```json
{
  "framework": "string",                     // Framework name without format/mode
  "output_format": "markdown|plaintext",     // Output format used
  "mode": "single|batch|...",                // Execution mode
  "cold_start": { /* DurationPercentiles */ },  // Optional, if cold start data available
  "by_file_type": {
    "pdf": {
      "file_type": "pdf",
      "no_ocr": { /* PerformancePercentiles */ },
      "with_ocr": { /* PerformancePercentiles */ }
    }
  }
}
```

## PerformancePercentiles

Contains p50, p95, p99 for all metrics:

```json
{
  "successful_sample_count": 42,
  "total_sample_count": 50,
  "framework_errors": 0,
  "harness_errors": 5,
  "timeouts": 3,
  "empty_content": 0,
  "error_details": {
    "error message": 2
  },
  "duration": { "p50": 100.5, "p95": 150.2, "p99": 199.9 },
  "throughput": { "p50": 5.2, "p95": 4.8, "p99": 3.1 },
  "memory": { "p50": 150.0, "p95": 200.0, "p99": 250.0 },
  "cpu": { "p50": 50.0, "p95": 75.0, "p99": 90.0 },  // Optional
  "extraction_duration": { "p50": 80.0, "p95": 120.0, "p99": 160.0 },  // Optional
  "quality": { /* QualityPercentiles */ },  // Optional, if quality data available
  "success_rate_percent": 84.0
}
```

## QualityPercentiles

Includes p50, p95, p99 for all F1 metrics. Layout percentiles are `null` for plaintext-only frameworks:

```json
{
  "f1_text_p50": 0.92,
  "f1_text_p95": 0.88,
  "f1_text_p99": 0.75,
  "f1_numeric_p50": 0.85,
  "f1_numeric_p95": 0.80,
  "f1_numeric_p99": 0.70,
  "f1_layout_p50": 0.78,       // null for plaintext output format
  "f1_layout_p95": 0.72,       // null for plaintext output format
  "f1_layout_p99": 0.65,       // null for plaintext output format
  "quality_score_p50": 0.85,
  "quality_score_p95": 0.80,
  "quality_score_p99": 0.70
}
```

## PerFixtureRow

One row per unique combination of (framework, output_format, execution_mode, fixture_id, ocr):

```json
{
  "framework": "kreuzberg-rust",
  "output_format": "markdown",
  "execution_mode": "single",
  "ocr": false,
  "fixture_id": "sample_doc_1",
  "file_type": "pdf",
  "duration_ms": 125.4,
  "peak_memory_mb": 180.5,
  "f1_text": 0.92,
  "f1_layout": 0.78,     // null for plaintext mode
  "f1_numeric": 0.85,
  "quality_score": 0.85,
  "correct": true,
  "success": true,
  "error_kind": null     // "FrameworkError", "HarnessError", "Timeout", etc. if !success
}
```

## ComparisonData

Contains all cross-framework rankings split by output format for quality metrics:

```json
{
  "performance_ranking": [ /* RankedFramework[] */ ],
  "throughput_ranking": [ /* RankedFramework[] */ ],
  "memory_ranking": [ /* RankedFramework[] */ ],
  "cpu_ranking": [ /* RankedFramework[] */ ],
  "quality_ranking": [ /* RankedFramework[] */ ],
  "pdf_quality_ranking": [ /* RankedFramework[] */ ],
  "pdf_tf1_ranking_markdown": [ /* RankedFramework[] — markdown-only */ ],
  "pdf_tf1_ranking_plaintext": [ /* RankedFramework[] — plaintext-only */ ],
  "pdf_sf1_ranking_markdown": [ /* RankedFramework[] — markdown-only, never plaintext */ ],
  "deltas_vs_baseline": {
    "framework:output_format:mode": { /* DeltaMetrics */ }
  }
}
```

### RankedFramework

```json
{
  "framework_mode": "kreuzberg-rust:markdown:single",
  "rank": 1,
  "value": 95.5,      // The metric value (duration, throughput, etc.)
  "relative": 1.0     // Ratio relative to best (1.0 = best)
}
```

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
- All performance rankings (speed, memory, throughput) include both formats without discrimination

## ConsolidationMetadata

```json
{
  "total_results": 500,
  "framework_count": 5,
  "file_type_count": 8,
  "timestamp": "2025-05-09T10:15:30Z"
}
```
