# `native-pdf-large-v1` cohort

Sixteen native (text-layer) PDFs, 374 pages and 70.8 MB in total, selected for length and file size rather than
for the 1-20 page structure mix of `native-pdf-structured-v1`. Every member has text and Markdown ground truth from
a source independent of Xberg and LiteParse. `pdf/` members are pinned by `test_documents/corpus.lock.json`;
`.corpus-cache/pdf/` members are materialized by `scripts/corpus_cache_manifest.py` and their ground truth is pinned
by `out_gt_md_sha256` in `test_documents/ground_truth/corpus_manifest.json`. `sha256` below is the document digest.

## Members

| Fixture | Document | sha256 | Bytes | Pages | Features | GT source |
| --- | --- | --- | ---: | ---: | --- | --- |
| `pdf/1509.07624.json` | `.corpus-cache/pdf/1509.07624.pdf` | `200731b99ded4510e9534967ca72ff236089e6b486701687084fff9c0594820e` | 4641446 | 48 | dense math, 77 fonts | READoc (arXiv LaTeX) |
| `pdf/2207.09253.json` | `.corpus-cache/pdf/2207.09253.pdf` | `04c7fabd6af0642c29f349912b611522bd80fefb24977a95fc471258010407c6` | 465615 | 43 | math, nested lists, 121 images | READoc (arXiv LaTeX) |
| `pdf/2301.08131.json` | `.corpus-cache/pdf/2301.08131.pdf` | `86d09a8d1b0fe2c73eef9ae73efd256459011763564556ffaafb36a2e9433028` | 285242 | 37 | math, long prose | READoc (arXiv LaTeX) |
| `pdf/1004.1028.json` | `.corpus-cache/pdf/1004.1028.pdf` | `c7d69e9bcf3d5e859ba2ebedab889c9980386b5239fe32701c63357904084394` | 2281941 | 33 | two-column, math, footnotes, 137 images | READoc (arXiv LaTeX) |
| `pdf/1602.03238.json` | `.corpus-cache/pdf/1602.03238.pdf` | `0cf66a9e3e2cc5ecb45d70060dc2130abbfc3fe9c9ffb3746beba35d2af6a0a9` | 403799 | 32 | dense math, 118 images | READoc (arXiv LaTeX) |
| `pdf/cond-mat0208183.json` | `.corpus-cache/pdf/cond-mat0208183.pdf` | `5d89283cb0253298c2a824df8cf8e3da368b5c2aa9c2df4320d4a8253a92d160` | 365801 | 32 | math, 998 image XObjects (dvips) | READoc (arXiv LaTeX) |
| `pdf/2310.02163.json` | `.corpus-cache/pdf/2310.02163.pdf` | `90c1cf66edab0b7c1dcbfc0a7bfaad2c593af9d353fc2c9d7466eab88bef8865` | 700016 | 28 | math, captions, footnotes | READoc (arXiv LaTeX) |
| `pdf/2211.13451.json` | `.corpus-cache/pdf/2211.13451.pdf` | `4e4449c884f2fcf5e1b0aff326d9e27928512b0bb7beabf8633110d8ab0cdbcd` | 274177 | 24 | densest text (~3.1k chars/page), math | READoc (arXiv LaTeX) |
| `pdf/3562713.json` | `pdf/3562713.pdf` | `3e03f3efa84ae57c4f73f50e6e31c8d9e56a88913ab34c7bf010c756a2d05a41` | 147831 | 27 | code blocks, nested lists | READoc (GitHub README) |
| `pdf/2026-14578.json` | `pdf/2026-14578.pdf` | `e32de17658362f84b44a776882e13754d7fb18e8c464d71f46e066b6dbfdfcbf` | 2118095 | 18 | government, three-column, tables | Federal Register XML |
| `pdf_large_tables.json` | `pdf/large.pdf` | `9ed56fdf6cdac02a1b559b59e787b18eb26bf67fbcc216f7d4ba33e72a83d593` | 2636957 | 10 | two-column paper, many tables | vision |
| `pdf/2209.05611.json` | `.corpus-cache/pdf/2209.05611.pdf` | `42936ed02e33397935fa80b65fab7af3847c82fdadb9572d9da86ab3e98fd197` | 15974809 | 12 | two-column, large embedded images | READoc (arXiv LaTeX) |
| `pdf/2303.08597.json` | `.corpus-cache/pdf/2303.08597.pdf` | `96989af33efa14c509c96f998dadaaf3f661f68d38dde12554688ba86809236c` | 12638756 | 6 | two-column, large embedded images | READoc (arXiv LaTeX) |
| `pdf/139609621.json` | `pdf/139609621.pdf` | `e2ac6107d60e6fedbc17409b785fde09bf27a779a449307193e293870e96358e` | 10883039 | 12 | image-heavy, code | READoc (GitHub README) |
| `pdf/84034880.json` | `pdf/84034880.pdf` | `36963cce595a8a787d1ae7492a1dfa695d6de4810359c2be4cf1bed5194e810f` | 11552701 | 6 | image-heavy, little text | READoc (GitHub README) |
| `pdf/588538823.json` | `pdf/588538823.pdf` | `e24269011f7654af3cf9a24329b116f41eb6beaa29b791c1b7b040e58cf94b61` | 5427567 | 6 | image-heavy, little text | READoc (GitHub README) |

## Bounds

- Ten members are also in `native-pdf-structured-v1`; the new ones are `cond-mat0208183` and the five files over
  5 MB. Results on the shared members are not independent evidence across the two cohorts.
- READoc arXiv ground truth is author LaTeX through pandoc and carries no tables, so table structure is scored only
  on `2026-14578` and `pdf_large_tables`.
- Every member has `role: tune` or no role; none is held out.
- The longest member has 48 pages. Longer native PDFs in the corpus (manuals and books of 100-4778 pages) have no
  independent ground truth, and a cohort cannot carry performance-only rows: `--measure-quality` rejects any fixture
  without ground truth (`runner/helpers.rs`, `load_quality_ground_truth`). The pinned corpus manifest lists about
  100 ACCEPT-gated READoc documents of 30-121 pages in the `extended` tier; materializing those with
  `scripts/build_corpus.py` is the route to a longer GT-backed cohort.
