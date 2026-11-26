# `native-pdf-long-perf-v1` cohort

Twelve long or very large native PDFs (10,018 pages, 257 MB) for latency, memory and success-rate measurement only.
No member has ground truth, so run this cohort **without** `--measure-quality`: the runner rejects a quality run
when any fixture lacks ground truth (`load_quality_ground_truth` in `src/runner/helpers.rs`), and
`validate-artifacts` expects rows without ground truth to carry no quality block. Every document is pinned by
`test_documents/corpus.lock.json`.

`batch_size` is 1 because a batch invocation shares one `--timeout` across all of its documents. Every member sets
`metadata.opt_in`, so a directory scan without `--cohort` skips it.

| Fixture | Pages | Bytes | Content |
| --- | ---: | ---: | --- |
| `pdf/perf_intel_sdm.json` | 4778 | 52916900 | Intel SDM combined volumes; encrypted with an empty user password; dense tables |
| `pdf/perf_math_ml_textbook.json` | 1962 | 20812100 | math textbook (pdfTeX) |
| `pdf/perf_assembly_book.json` | 1082 | 9068750 | book with code listings (XeTeX) |
| `pdf/perf_bda3.json` | 677 | 35493143 | statistics textbook, math and many figures |
| `pdf/perf_xerox_manual.json` | 464 | 2435134 | product administration manual, tables and screenshots |
| `pdf/perf_islr.json` | 440 | 9433354 | textbook with figures and R code |
| `pdf/perf_iso_21111_10.json` | 214 | 2343170 | ISO standard, dense tables |
| `pdf/perf_ciml.json` | 193 | 2636899 | textbook with math (pdfTeX) |
| `pdf/perf_poc_gtfo_13.json` | 72 | 62061798 | polyglot journal issue, two-column, 60+ fonts, images |
| `pdf/perf_fundamentals_deep_learning.json` | 61 | 56715281 | image-heavy document (Quartz) |
| `pdf/perf_crdt_study.json` | 50 | 1537629 | research report, math and vector figures |
| `pdf/perf_100g_slides.json` | 25 | 1516903 | slide deck exported to PDF |

Xberg's own per-document limit is `--timeout` minus 60 seconds (`extraction_timeout_secs` in
`src/adapters/xberg.rs`), so raise `--timeout` for the longest members.
