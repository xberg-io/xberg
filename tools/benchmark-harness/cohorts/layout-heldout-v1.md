# `layout-heldout-v1` cohort

Frozen five-document cohort for the PP-DocLayoutV3 vs RT-DETR held-out comparison (#2706).
Every fixture is committed under `fixtures/`; every document is pinned by
`test_documents/corpus.lock.json` and materialized from the public bucket
(`python3 test_documents/scripts/fetch_corpus.py --include 'pdf/**' --include 'pdf_scanned/**' --include 'vendored/docling/pdf/**'`).

`sha256` is the document digest, identical to the `corpus.lock.json` pin. `fixture_sha256` is the
descriptor digest; both are recorded so the cohort cannot drift silently.

## Members

| Class | Fixture | fixture sha256 | Document | sha256 | Bytes | Pages | Source | License |
| --- | --- | --- | --- | --- | ---: | ---: | --- | --- |
| Born-digital report | `pdf/2026-13845.json` | `bc103e1e5eb24fd843ee05b8a4ebd438a9f2265753945b2889ad0a631282f7d6` | `pdf/2026-13845.pdf` | `34df5e526716bd994552319cc63c4a3dcb30eadf795ceca9445b9b4d21968957` | 281140 | 7 | U.S. Federal Register (OFR/GPO) | US-PD (17 U.S.C. §105) |
| Two-column paper | `pdf/docling_2305_03393v1.json` | `03f4cf61580c47e5a9244812481d7e7e6cd08c66ea43f46657db61e84bda26f4` | `vendored/docling/pdf/2305.03393v1.pdf` | `62f2a2163d768d5b125a207967797aefa6c9cc113de8bb5c725c582595dd0c1d` | 4318934 | 14 | ds4sd/docling test suite (paper: Lysak et al., "Optimized Table Tokenization for Table Structure Recognition") | MIT (`test_documents/vendored/docling/LICENSE`) |
| Table page | `pdf/ft_ACN_2009_page_102_t0.json` | `9869ab78069d529a31e59da839122683b353d9d656497fd324343898468612b3` | `pdf/ft_ACN_2009_page_102_t0.pdf` | `f8af9cee9feb9c6fb3ab0860b3ad9189dfb776111f95c3506feb4e1f69e87123` | 10762 | 1 | bsmock/FinTabNet.c | CDLA-Permissive-2.0 |
| Non-Latin page | `rtl_arabic_native.json` | `a256e680afa986362622d705ea90b29ee216763c8e60c8d87ee3d8ac1d4cad6b` | `pdf/rtl_arabic_native.pdf` | `399aab1645159c8d7b62601d2cedda52baa77b4b518b73b0095676519efb4a23` | 53779 | 1 | authored for this corpus (`test_documents/ATTRIBUTIONS.md`, "Synthetic Arabic and Hebrew right-to-left fixtures") | MIT |
| Scanned page | `ordinance_2197_scanned.json` | `2d0ed0baf6fb4b584e2229bb7554ac6040929c8cad9e98b0637b54f71efdddac` | `pdf_scanned/ordinance_2197_scanned.pdf` | `94dcfbd413fb69aedc0b2005893ddba2ea77be82da3ff3631dd17ff4c983749a` | 538907 | 16 | City of Sugar Land, Texas, Ordinance No. 2197 (office-scanner capture) | US public domain (edict of government) |

## Why these are held out

`#2702` selected the detector and its only document fixture was `pdf/google_doc_document.pdf`
(`crates/xberg/tests/layout_model_selection.rs`). `#2693` introduced the cross-runtime comparator
against the generator-reproducible `scripts/ci/fixtures/v3_cross_runtime_synthetic.json` only.
Neither commit reads any member above. `pdf/2715244.pdf` — the corpus manifest's only
`role: eval` PDF fixture — is deliberately **not** a member; it already anchors the harness's
`native-clean`/`figures` cohorts, so including it would blur tuning and evaluation.

## Bounds

- The cohort is a detector-level evaluation set. It carries committed text/markdown ground truth
  for four of five members but **no per-page layout-region bbox annotations**, so layout-region F1
  cannot be computed without new annotation work. See the #2706 result comment.
- `vendored/docling/pdf/2305.03393v1.pdf` is third-party paper content redistributed by docling's
  own test suite under its repository `LICENSE`; the attribution above is what the submodule states,
  not an independent rights review.
- The 16-page ordinance is sampled with `--max-pages 2`, so it covers its first two prose pages
  only, not the survey/architectural exhibit sheets later in the document.
