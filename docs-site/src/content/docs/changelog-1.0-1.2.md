---
title: "Changelog: 1.0-1.2"
slug: changelog-1.0-1.2
---

[Current releases](/changelog/).

## [1.2.9] - 2026-09-24

### Added

- **(ocr):** `ExtractionConfig` gains `ocr_near_empty_fallback`, `ocr_scanned_page_quality_gate` and `ocr_embedded_images`, so the near-empty PDF fallback, the scanned-page quality gate and embedded-image OCR can each be enabled without an `ocr` block. `None` keeps the previous behaviour; a config setting any of them is rejected by versions before 1.2.9. (GH#1752)

### Fixed

- **(pdf):** A two-column split no longer lands inside a figure caption, which interleaved caption text with the body column. (GH#1763)
- **(pdf):** A page whose content stream decompresses to more than 100 MB no longer loses its text layer; `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB` now sets the whole decompression ceiling, including for the `ParserOptions` presets. (GH#1754)
- **(pdf):** A deeply nested page tree returns an error instead of aborting the process with a stack overflow, and a bad branch no longer loses the whole document. (GH#1755)
- **(pdf):** A stray `MoveTo` no longer stretches a path's bounding box to the page, which produced a false page-sized table and suppressed the real one. (GH#1759)
- **(pdf):** A table's sparse rows no longer move the two-column split into the right column. (GH#1756)
- **(pdf):** A wrapped numbered heading keeps its continuation line instead of losing it to the paragraph below. (GH#1758)
- **(pdf):** A heading beside a two-column gutter no longer absorbs the other column's opening line. (GH#1757)
- **(pdf):** Heuristic table detection no longer turns the last lines of a paragraph and a neighbouring heading into a false table. (GH#1760)
- **(pdf):** A four-column table no longer pulls the two-column split inside itself. (GH#1762)
- **(pdf):** Merged cells, full-width section rows and the unshaded rows of zebra-striped tables stay in their ruled table instead of spilling into paragraphs. (GH#1765)
- **(image):** The image decode budget error no longer reports a pixel count against a byte budget. (GH#1761)
- **(ocr):** `disable_ocr` now also stops OCR, and the image decoding it needs, for images embedded in DOCX, PPTX, ODT and HTML.
- **(pdf):** A label-value form is no longer mistaken for a two-column layout, which could drop cells from a heading.
- **(ocr):** A PDF whose font places a single-byte code page's letters (e.g. cp1251 Cyrillic) at Latin-1 positions is routed to OCR instead of returning mojibake. (GH#1767)

## [1.2.8] - 2026-09-23

### Fixed

- **(ocr):** With layout detection enabled, PDF OCR no longer upscales page rasters about 4x instead of 2x, reducing memory use and recognition time. (GH#1753)

## [1.2.7] - 2026-09-22

### Added

- **(config):** `ConcurrencyConfig::max_concurrent_ocr` and the `--max-concurrent-ocr` CLI flag set the number of concurrent Tesseract sessions; the first extraction in a process fixes the value. **Breaking (Rust):** struct literals without `..Default::default()` must add the field. (GH#1727)

### Changed

- **(ocr):** Concurrent Tesseract recognition defaults to the thread budget, limited by free memory, instead of a fixed 4. (GH#1727)
- **(pdf):** Scan detection and fabricated-mapping OCR routing run in parallel across the thread budget. (GH#1723)
- **(pdf/ocr):** The per-page OCR batch is sized by page raster cost and free memory instead of `max_content_size`, allowing more parallelism at higher peak memory. (GH#1724)
- **(pdf):** Embedded-image extraction runs in parallel across the thread budget, at higher peak memory. A full-page image on a page with native text is no longer decoded when only OCR would read it; it is returned with empty `data`. (GH#1732)
- **(pdf):** Scan detection no longer decodes embedded images; an image the decoder rejects now counts toward the page's image coverage. (GH#1732)
- **(pdf):** Fabricated-mapping OCR routing no longer reads each page's text a second time, shortening native extraction. (GH#1744)
- **(pdf):** Pages sharing a `/Font` dictionary no longer repeat TrueType cmap sharing on every page. (GH#1746)
- **(ocr):** `candle-deepseek-ocr` is about 7.8x faster per page on GPU. (GH#1711, GH#1714)

### Fixed

- **(pdf):** Layout detection no longer fails after about twelve pages of a long document; `security_limits.max_content_size` no longer bounds the retained layout rasters. (GH#1721)
- **(ocr):** OCR with layout detection no longer refuses batches of five or more pages under the default `max_content_size`, which returned native text without OCR. (GH#1731)
- **(ocr):** The `auto` device preference logs a warning naming the accelerator and error when it falls back to the CPU; `candle-deepseek-ocr` resolves its device once per engine instead of per page. (GH#1722)
- **(pdf):** A row-padded image buffer returns an error instead of panicking. (GH#1735)
- **(pdf):** A page's extracted text no longer depends on the order pages are read. (GH#1725)
- **(pipeline):** Embedded images extracted without OCR (Markdown SVG data URIs, Jupyter images, ODT/DOCX/PPTX pictures) are returned in `images` again. (GH#1703)
- **(ocr):** `candle-glm-ocr` resolves its layout model once per process instead of once per page. (GH#1718)
- **(pdf):** A document where no page could be checked for text plausibility now carries a processing warning. (GH#1709)
- **(pdf):** A numbered heading wrapping to three or more lines keeps all its lines. (GH#1740)
- **(pdf):** A numbered heading whose number and title are separated by a kern no longer absorbs the other column's line on a two-column page. (GH#1738)
- **(config):** The configured `max_threads` is no longer ignored when two extractions start at the same time. (GH#1750)
- **(pdf):** On a two-column page containing a table, the column split no longer lands inside the table. (GH#1742)

## [1.2.6] - 2026-09-20

### Added

- **(pdf/ocr):** `OcrQualityThresholds::enable_plausibility_ocr_routing` and `OcrQualityThresholds::min_reliable_language_chunk_ratio` control language-plausibility OCR routing; `PdfMetadata.implausible_text_pages` lists pages whose text does not read as any language. (GH#1696)
- **(config):** `ExtractionConfig::runs_ocr_on_embedded_images` and `ExtractionConfig::wants_own_bytes_in_result` are exposed on every binding. (GH#1662)

### Changed

- **(ocr):** The `ocr`, `ocr-wasm` and `ocr-pipeline` Cargo features now enable `language-detection`. (GH#1696)

### Fixed

- **(ocr):** OCR no longer returns every embedded image's bytes: `images` carries data only when image extraction, captioning, QR-code detection, `pdf_options.ocr_inline_images` or `images.include_page_rasters` is requested. `counts.images` is unchanged. (GH#1703)
- **(pdf):** A page whose glyph mapping resolves to wrong but real letters is routed to OCR under the default `Auto` strategy; without an `ocr` config it keeps its text and gets a warning. (GH#1696)
- **(pdf):** A page whose text has no usable glyph mapping is routed to OCR under the default `Auto` strategy. (GH#1667)
- **(pdf):** The mixed native/OCR path no longer drops native tables on pages it did not OCR. (GH#1670)
- **(pdf):** `include_document_structure` alone now produces the structured document. For that configuration the plain-text `content` changes: table rows render one per line and headings and footers reflow. (GH#1668)
- **(docx):** Paragraphs inherit list numbering from their style, so Word's built-in list styles produce list items. (GH#1663)
- **(ocr):** Low OCR confidence now lowers the reported quality score. (GH#1669, GH#1694)
- **(ocr):** The aggregate OCR confidence is populated for page-level results and weighted by word count on both routes, which changes the value the embedded-image route reported. (GH#1677)
- **(ocr):** A cold start loads one candle OCR engine instead of one per waiting caller, which could exhaust GPU memory. (GH#1683)
- **(ocr):** The Tesseract language probe at backend registration finds the installed languages instead of falling back to a fixed list. (GH#1671)
- **(ocr):** A page warning reports the outcome of the embedded-image OCR retry. (GH#1673)
- **(ocr):** The mixed native/OCR route renders pages in parallel, so OCR throughput scales with the thread budget. (GH#1666)
- **(ocr):** Images embedded in DOCX, PPT, PPTX and HTML are read before OCR instead of failing with "Could not determine image format". (GH#1662)
- **(ocr):** The mixed native/OCR route caps its render batch against `security_limits.max_content_size` instead of skipping every page of an oversized batch. (GH#1665)
- **(ocr):** The candle OCR backends no longer reject every rendered PDF page. (GH#1672)
- **(ruby):** Reading a tagged enum's payload field no longer raises `NoMethodError`. (alef 0.93.0)
- **(ocr):** Candle VLM backends drop lines in a script the configured languages do not cover, or stray LaTeX on a plain-text task, with a warning. (GH#1676)
- **(ocr):** `candle-deepseek-ocr` reads the whole page: the token cap is configurable (default 4096), large pages are tiled into 640 px crops, the dtype follows the device, weights auto-download so `model_path` is optional, and repetition loops are truncated. (GH#1674)
- **(ocr):** `candle-deepseek-ocr` no longer fails on the first 640 px crop with `shape mismatch in mul`. (GH#1674)
- **(ocr):** `candle-deepseek-ocr` applies a causal mask during prefill. (GH#1701)
- **(ocr):** `candle-glm-ocr` no longer garbles hex identifiers such as UUIDs; the repetition penalty defaults to 1.0. (GH#1675)

## [1.2.5] - 2026-09-18

1.2.4 was not published to any package registry; 1.2.5 includes its changes.

### Fixed

- **(python):** Config fields holding a nested data-carrying enum (e.g. `CaptioningConfig.llm`) are no longer dropped by `from_json` / `to_json`. (alef#394)
- **(ppt):** Embedded OLE objects in legacy `.ppt` files (Word, Excel, PowerPoint) are extracted into `children`, subject to the same archive limits as `.pptx`. (GH#1660)
- **(pdf):** A ruled table drawn with one bar per cell no longer loses all but its last two rows. (GH#1656)

## [1.2.4] - 2026-09-18

### Added

- **(llm):** `LlmConfig::max_response_bytes` bounds LLM provider response bodies; defaults to `None` (unbounded) and rejects `Some(0)`. (xberg-io/xberg-enterprise#1568, xberg-io/xberg-enterprise#1861)

### Fixed

- **(pdf):** A page-sized background rectangle no longer produces a whole-page table. (GH#1656)
- **(pdf):** A single-column page of hanging-number headings is no longer read as two columns. (GH#1655)
- **(ocr):** `security_limits` now applies to Tesseract image decoding; limits set on `OcrConfig` apply when `ExtractionConfig` sets none. (GH#1651)
- **(go):** `Register*` no longer leaks its `cgo.Handle` when the C vtable allocation fails. Reported by @OvOhao (GHSA-q5pq-8g86-v9j9).
- **(tables):** In OCR and native-PDF table reconstruction, a multi-word cell no longer bleeds into the next column, a drift-split right-aligned amount column is merged, a sparse but headed column or sparse first row no longer breaks the table, and an isolated punctuation cell no longer fuses two columns. (GH#1649)

## [1.2.3] - 2026-09-16

### Added

- **(ocr):** `list_ocr_backend_capabilities()` returns each registered backend's languages, and `ocr_backend_supports_language()` checks one language. (GH#1643)
- **(ocr):** `metadata.additional.ocr_page_coordinate_frames` reports each OCR'd page's raster size for mixed-page PDFs. (GH#1645)
- **(pdf):** `metadata.additional.pdf_page_coordinate_frames` reports each page's MediaBox origin, size and rotation for hierarchy bounding boxes, in unrotated PDF user space. (GH#1653, GH#1654)
- **(ner):** Rust callers can share the process-wide GLiNER backend cache via `text::ner::gline::get_or_init_backend`.

### Fixed

- **(ocr):** A single blank quantity cell in an invoice table is re-read from its pixels, and headers split across two words (`UNIT`/`PRICE`) are rejoined.
- **(reranker, embeddings, transcription):** A cancelled `rerank_async`, `embed_texts_async` or transcription call no longer releases its concurrency permit while inference keeps running. (GH#1641)
- **(pdf):** A wrapped numbered heading in a heading-size font keeps its second line. (GH#1650)
- **(packaging):** The Helm chart is published again; it was missing for 1.1.4 through 1.2.2, and the 1.2.2 chart has been published.
- **(node):** JavaScript plugins registered through the Node binding now run: async methods are awaited and a thrown error becomes a plugin error. (GH#1636)

### Changed

- **(pdf):** A page whose MediaBox is not rooted at the origin reports its correct size and reading order. (GH#1653)
- Upgraded the `crawlberg` dependency to 1.7.0, whose crawl types xberg re-exports; no Rust API changes.

## [1.2.2] - 2026-09-15

### Fixed

- **(pdf):** A two-column page is no longer split inside a column because one line crosses the gutter, which spliced words from both columns. Reported and fixed by Data Polder. (GH#1619)
- **(pdf):** A wrapped numbered heading closes above a run-in sub-heading instead of absorbing it. (GH#1637)
- **(pdf):** A hanging-number heading no longer swallows body text indented to its title's edge, and a wrapped heading no longer absorbs the content below it. (GH#1634)
- **(pdf render):** The `s` (close-and-stroke) operator is painted and no longer floods the next fill. Text extraction was unaffected. (GH#1633)
- **(pdf render):** Glyphs whose inferred Unicode is whitespace are no longer skipped when rendering. (GH#1632)
- **(ppt):** Legacy `.ppt` slide titles are read from the file, so title-only, multi-line and long titles are kept. (GH#1635)
- **(ppt):** A deck saved more than once returns its latest slide text instead of the oldest save's. (GH#1639)
- **(ppt):** Speaker notes attach to the slide that owns them, and the notes master's placeholder text is no longer emitted. (GH#1640)
- **(cli):** The `xberg-cli` release archives, including the Linux musl builds, are published again; 1.2.0 and 1.2.1 shipped none. (GH#1638)
- **(php):** Setting `outputFormat` no longer throws `invalid type: string "markdown"`; `EntityCategory` and `PiiCategory` had the same defect. Regression in 1.2.0.
- **(wasm):** `metadata.format` fields such as `title` and `sheetCount` are readable again. Regression in 1.2.0.

### Changed

- **Breaking (Rust):** `extraction::ppt::PptSlideText` gains `title` and `notes` fields. Not exposed through any binding.
- **Breaking (PHP binding):** `OutputFormat`, `EntityCategory` and `PiiCategory` serialize as their Rust counterparts do (`"markdown"`, `{"custom":"latex"}`) instead of `{"type":"markdown"}`. `from_json` still accepts the old form.
- **Breaking (WebAssembly binding):** `metadata.format` and other flattened enum fields are plain objects instead of `Map`, as `index.d.ts` declares. Non-flattened enum fields such as `chunking.sizing` are unchanged.
- **Breaking (C#, Dart, Go, Java, Kotlin, Python, Swift bindings):** Enum variants ending in an acronym followed by a lowercase letter are spelled correctly: `StructuredDataType.RdFa` becomes `Rdfa` (C#, Dart, Go, Swift), Kotlin's `R_D_FA` becomes `RDFA` and `PADDLE_O_C_R` becomes `PADDLE_OCR`, and Python's stub and runtime agree on `RDFA`. Wire values are unchanged.

---

## [1.2.1] - 2026-09-14

### Fixed

- **(swift):** `EntityCategory`, `PiiCategory` and `ConfidenceSemantics` decode correctly, so results containing entities no longer throw `DecodingError.typeMismatch`.

### Changed

- Dependencies upgraded, including `crawlberg` 1.6.3 and `tree-sitter-language-pack` 1.19.1.

## [1.2.0] - 2026-09-13

### Added

- **(server):** `ServerConfig::job_timeout_secs` (default 600, `XBERG_JOB_TIMEOUT_SECS` or `server.job_timeout_secs`) sets the fallback timeout for `POST /extract-async` jobs without `extraction_timeout_secs`; it was a fixed 300 seconds.
- **(embeddings, reranking):** `evict_model` (in `embeddings`, `reranking`, `sparse_embeddings` and `late_interaction`), `clear_engine_cache`, `xberg::clear_engine_caches` and `set_engine_cache_limit` release cached models; caches stay unbounded by default. (GH#1626)

### Fixed

- **(python):** `OutputFormat.custom` returns the custom label instead of `None`.
- **(python):** The type stub no longer declares a nonexistent `type` attribute on 23 enum classes such as `EntityCategory`.
- **(python):** Public option classes such as `ExtractInput` support `from_json` when imported from `xberg`, as they do from `xberg._xberg`.
- **(ruby):** `FormatMetadata.from_hash` and `DiffLine.from_hash` no longer return a `nil` payload for every variant. ([#1594](https://github.com/xberg-io/xberg/issues/1594))
- **(python, elixir):** The Python stub and Elixir typespec for format metadata name the real payload fields instead of `_0` / `metadata:`. ([#1594](https://github.com/xberg-io/xberg/issues/1594))
- **(go):** Errors keep the native error message (e.g. the actual timeout values) while still matching sentinels such as `ErrTimeout` via `errors.Is`. Regression in 1.1.0.
- **(pdf):** Type0 fonts using a non-Identity predefined or embedded CMap get correct glyph widths, fixing stretched or overlapping text and spurious paragraph breaks; unsupported legacy CMaps log a warning. (GH#1631)
- **(pdf):** A reconstructed table cell reads left to right, fixing misplaced sub/superscripts and values swapped between merged columns; OCR table cells too. (GH#1628)
- **(pdf):** A body paragraph is no longer deleted because it repeats text found elsewhere on the page. (GH#1623)
- **(pdf):** A paragraph whose words a table already carries is no longer also emitted as prose beside it. (GH#1622)
- **(pdf):** Text inside a table's bounding box but outside its reconstructed grid is no longer deleted. (GH#1616)
- **(pdf):** Body text is no longer promoted to a heading. (GH#1599)
- **(pdf):** A numbered heading that wraps onto a second line keeps its full title. Regression in 1.1.5. (GH#1615)
- **(pdf):** De-hyphenation no longer joins compounds like `long-term` into `longterm` when the hyphen falls on a line break. (GH#1613)
- **(ocr):** Image OCR honours a PNG's embedded `pHYs` density instead of assuming 72 DPI. (GH#1630)
- **(ocr):** OCR text in a scanned region detected as a table but not recognisable as one is emitted in reading order instead of discarded. (GH#1622)
- **(ocr):** On the scanned-PDF layout-markdown path, heading/list restructuring falls back to the unrestructured OCR text when it would lose content. (GH#1622)
- **(ocr):** An extraction cancelled by `extraction_timeout_secs` stops its per-page PDF OCR and reports `Cancelled`.
- **(ocr):** An extraction that did not request OCR no longer fails when no OCR backend is registered; automatic OCR triggers skip with a warning. (GH#1610)
- **(ocr):** `OcrConfig` accepts all Tesseract language codes, such as `fao`. (GH#1621)
- **(ppt):** Legacy `.ppt` embedded pictures report the slide they belong to. (GH#1620)
- **(ppt):** Legacy `.ppt` no longer returns deleted slide revisions or slides out of order. (GH#1614)
- **(ppt):** Legacy `.ppt` slide titles held in the outline are no longer lost. (GH#1612)
- **(confidence):** `extraction_confidence` scores a requested structured extraction that produced no output as `AllInvalid` instead of fully valid. (GH#1624)
- **(mime):** `detect_mime_type_from_bytes` reports non-UTF-8 text (e.g. Windows-1252 CSV) as `text/plain` instead of `UnsupportedFormat`. (GH#1625)

### Changed

- **Breaking (Java binding):** Enum constants are `SCREAMING_SNAKE_CASE` (`LinkStyle.Inline` becomes `LinkStyle.INLINE`). JSON wire values are unchanged.
- **Breaking (Ruby binding):** Single-payload enum variants such as `EntityCategory::Custom` serialize as `{"custom" => "my-label"}` instead of `{"custom" => {"_0" => "my-label"}}`.
- **Breaking (PHP binding):** `EntityCategory`, `PiiCategory` and `OutputFormat` replace their constants with static factories (`EntityCategory::person()`, `EntityCategory::custom($label)`) and a readonly `$custom` property, so custom labels are no longer lost.
- **Breaking (Node binding):** The JSON surface is camelCase throughout, nested types included, replacing the parallel snake_case structural types.
- **Breaking (Node, Swift bindings):** `FormatMetadata` serializes flat (`{"format_type": "pdf", "page_count": 12, ...}`) as in the core and REST API. Node's `doc.metadata.format.pdf.pageCount` becomes `doc.metadata.format.page_count` (snake_case, typed as a discriminated union) and matches WebAssembly; Swift's `formatMetadataFromJson` returns a `Codable` enum carrying the payload. Other bindings are unchanged on the wire. ([#1594](https://github.com/xberg-io/xberg/issues/1594))
- **Breaking (Rust, Java):** `ServerConfig` adds `job_timeout_secs`: exhaustive Rust struct literals and the Java record's canonical constructor must include it (`ServerConfig.builder()` is unaffected). Other bindings and deserialization are source-compatible.

## [1.1.5] - 2026-09-10

### Fixed

- **(java):** The Java binding compiles again; 1.1.4 published no Java artifact.

## [1.1.4] - 2026-09-09

### Changed

- **Breaking (Ruby):** `FormatMetadata` arrives as a flat hash (`{format_type: 'excel', sheet_count: 2, ...}`) instead of nesting the payload under `_0`. Recorded retroactively. (GH#1594)

### Fixed

- **(pdf):** Reading order no longer separates a subscript from the symbol it belongs to. (GH#1600)
- **(pdf):** Table detection no longer bridges two separate tables and absorbs the section heading between them. (GH#1601)
- **(pdf):** Two-column detection no longer moves the split to a hanging-number indent, which pulled clause numbers out of their clauses. (GH#1603)
- **(pdf):** A numbered heading wrapping onto a shorter second line is no longer split. (GH#1605)
- **(pdf):** A numbered heading whose number and title are separate text spans is recognised instead of merged into its body. (GH#1609)
- **(pdf):** Headings whose number follows a word, such as `ARTIKEL 1.`, `Chapter 1`, `Annex III` or `Exhibit A`, are recognised instead of merged with neighbouring lines. (GH#1608)
- **(pdf):** Two-word numbered headings such as `3. PRIJZEN` are detected as headings. (GH#1611)
- **(ocr):** A markdown table rebuild that loses words is rejected with a warning instead of replacing the page text. (GH#1599)
- **(ocr):** PaddleOCR's default `model_tier` of `mobile` uses the 9.9 MB `small` detection model instead of the 62 MB `medium` one; documented model sizes are corrected. (GH#1602)
- **(php):** The PHP extension loads on Debian 12 and other GCC 12 distributions. (GH#1606)

## [1.1.3] - 2026-09-08

### Added

- **(docx):** `Table.cell_styles` and `GridCell.heading_level` / `GridCell.style_name` expose a DOCX table cell's paragraph style, such as `Heading1`. Cell text is unchanged, and tables without cell styles serialize as before. (GH#1587)

### Fixed

- **(pdf):** Ligature repair no longer joins two real words, such as `relief for` into `relieffor`. (GH#1591)
- **(pdf):** Outline named destinations with UTF-16BE keys (as written by Adobe Distiller) resolve to their page. (GH#1589)
- **(pdf):** Table detection no longer creates a column boundary from a rule that stops short of the row, which could turn framed prose into a table and lose text. (GH#1588)
- **(docx):** A table spanning pages no longer collapses the page count. (GH#1592)
- **(mime):** `MimeDetectionPolicy::ContentOnly` accepts legacy OLE2 Office files (.doc/.xls/.ppt) passed by path, as it already did for bytes. (GH#1590)
- **(ocr):** Tesseract `psm = 0` (orientation detection only, no text) is rejected at config validation; valid values are 1-13. (GH#1586)

## [1.1.2] - 2026-09-07

### Changed

- **Breaking:** `TesseractConfig.psm` is `Option<i32>` (`null`/`None`/absent in the bindings) and defaults to unset instead of 3. A `TesseractConfig` with default fields no longer changes the automatic PSM selection; an explicit `psm` is still honoured. Bindings that model `psm` as an integer expose a presence check, e.g. `xberg_tesseract_config_has_psm` in the C API.

### Fixed

- **(ocr):** A numbered or bulleted list on a scanned page is no longer reconstructed as a table.
- **(ocr):** A table on a scanned page is no longer returned twice, as paragraphs and as cells.
- **(ocr):** `PdfConfig.top_margin_fraction` / `bottom_margin_fraction` default to 0.0 instead of 0.06/0.05, which since 1.1.0 dropped OCR text near the top and bottom of scanned pages and forced the slower per-page OCR route.
- **(ocr):** OCR of rendered PDF pages keeps the Tesseract backend's processing warnings and OCR metadata (`psm`, `language`, `tesseract_dict_invalid_word_ratio`).
- **(ocr):** `OcrConfig.language` is no longer ignored when a `TesseractConfig` is supplied.
- **(ocr):** Supplying an `ImageExtractionConfig` no longer suppresses OCR of scanned PDFs.
- **(ocr):** Rendered PDF pages honour `target_dpi`, `min_dpi`, `max_dpi` and `auto_adjust_dpi`; the default stays 150 DPI.
- **(pdf):** Suspended hyphens are no longer joined (`onderhouds- en` became `onderhoudsen`).
- **(pdf):** An unruled full-width band in a ruled table is no longer cut into columns mid-word.
- **(tables):** Dash, minus and exponent normalisation applies only to numeric table columns instead of rewriting text in prose tables. ([#1582](https://github.com/xberg-io/xberg/issues/1582))
- **(php):** Windows PHP extension archives are published; 1.1.0 and 1.1.1 had no PHP binaries. ([#1585](https://github.com/xberg-io/xberg/issues/1585))

## [1.1.1] - 2026-09-07

### Changed

- **Breaking:** `OutputFormat::Structured` is renamed to `OutputFormat::DocTags` (`"doctags"`) in every binding and the `output_format` config field. The rename shipped unannounced in 1.1.0; `"structured"` now resolves to an unregistered custom renderer.
- **Breaking:** The TypeScript and WebAssembly `OutputFormat` type is a string union again (`"plain" | "markdown" | ... | "doctags"`), matching the other surfaces; 1.1.0 published an object union.

### Fixed

- **(php):** PHP extension (PIE) archives are published; 1.1.0 had none.
- **(node, elixir):** HEIC and AVIF decode on the Linux glibc Node binding and Elixir `linux-gnu` NIF.
- **(elixir):** Elixir NIFs for `linux-gnu` and Windows are published (the Hex package was stuck at 1.0.14), and the `linux-gnu` NIF runs on glibc 2.28.
- **(elixir):** The Windows Hex package declares the `x86_64-pc-windows-msvc` target, so `mix deps.get` finds the precompiled NIF.

### Security

- Vendored native dependencies (`libde265`, `libheif`, ONNX Runtime) linked into the Linux Node and Elixir artifacts are verified against pinned SHA-256 checksums.

## [1.1.0] - 2026-09-06

### Added

- Per-page OCR confidence in `PageContent.ocr_confidence` as `PageOcrConfidence { score, word_count, backend }`; `score` is `None` for backends without a calibrated confidence ([#1568](https://github.com/xberg-io/xberg/issues/1568)).
- HWPX extraction in the WebAssembly package.
- Diagram recovery from flat OpenDocument drawings (`.fodg`), with content-based MIME detection (#1545 corpus fixture).
- Structural extraction for MyST Markdown and MyST text notebooks, including saved inline `{eval}` values in Jupyter markdown cells ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- Extraction of Jupytext percent- and light-format notebook scripts, with `text/x-python`, `text/x-r-source` and `text/x-julia` MIME aliases ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- Bounded, cancellable SQLite and GeoPackage table extraction, with GeoPackage detection and `.sqlite3` / `.gpkx` support (#1510).
- Configurable MIME inference policies (prefer content, trust extensions, or ignore extensions), with per-input overrides (#1509).
- Native KML and GeoJSON extraction (#1508).
- Rust `SUPPORTED_FORMAT_COUNT` and `SUPPORTED_EXTENSION_COUNT` constants (#1511).
- Reusable Rust PDF render sessions for page counts and multi-page rendering without reopening the document (#1485).
- Cooperative cancellation for single and batch extraction (#1476).
- Dynamic system linking of Tesseract and Leptonica via the `tesseract-dynamic` feature (#1407).
- Azure AD, Google Vertex AI and AWS STS credential providers, with credentials redacted from debug output.
- Reasoning-effort, provider-specific request-body and Bedrock configuration for LLM extraction.
- `xberg doctor` and the Rust `doctor()` API validate configuration and probe compiled capabilities without downloads or network calls; `xberg doctor --clean` removes stray files from Xberg-owned caches (#1347).
- Sceptre EasyOCR Gen2 backend for desktop, mobile and WebAssembly.
- Sparse and late-interaction embeddings in chunk output.
- Prometheus `/metrics` endpoint on the API server (#1391).
- CSV delimiters and comment-line prefixes via `CsvOptions`.
- `xberg tree-sitter` commands for downloading, listing and cleaning language assets.
- VLM extraction for complex PDF regions and LaTeX formula extraction from VLM OCR.
- Structural AsciiDoc and WebVTT extraction.
- Docling DocTags input and output, including tables and page geometry (#1383).
- Formula recognition for rasterized pages, with formulas exposed consistently across formats (#1385).
- MathML-to-LaTeX conversion for JATS, EPUB, ODT and ODP.
- Diagram recovery from SVG and PDF sources with Graphviz DOT output (#579).
- `SecurityLimits.max_pages` for PDF, presentations, Keynote and multi-frame TIFF (#1451).
- PDF backend selection via `PdfConfig.backend` and `--pdf-backend` (#1448).
- musllinux Python wheels and a Windows x86_64 Ruby gem.
- PDF and HTML extraction, layout and transcription types in the WebAssembly package.
- `--ocr-no-cache` to bypass the Tesseract result cache.
- `ContentFilterConfig.include_footnotes` to keep footnotes classified as page furniture.
- Public `render_heading_breadcrumb` helper (#1393).
- Structured-output merge, citation and vision-fallback helpers for Rust.
- A Tower-compatible extraction service, request type and builder for Rust.
- Typed configuration for the TrOCR, PaddleOCR-VL, GLM-OCR and DeepSeek-OCR backends.
- `classify_chunks_owned`.
- Chunk-classification and LLM concurrency, provider, cache, budget and rate-limit configuration types at the Rust crate root.
- `OcrConfig::security_limits` ([#1554](https://github.com/xberg-io/xberg/issues/1554)).
- `detected_language_confidences` with each detected language's confidence, proportion, script and reliability; `detected_languages` is unchanged ([#261](https://github.com/xberg-io/xberg/issues/261)).
- DOCX reviewer comments emit `NodeContent::Comment` nodes instead of footnotes.
- PDF annotations keep their subtype, author, date, colour, subject and QuadPoints, recover highlighted text, and are emitted by the Markdown, Djot, plain, HTML and JSON renderers ([#63](https://github.com/xberg-io/xberg/issues/63)).
- PDF extraction reads image alt text from the structure tree, falls back to XMP metadata, reports `/PageLabels` in `metadata.additional`, excludes default-off optional-content layers, renders filled AcroForm values, and emits a `ProcessingWarning` for image, annotation and form failures ([#62](https://github.com/xberg-io/xberg/issues/62), [#71](https://github.com/xberg-io/xberg/issues/71)).
- OOXML `DocSecurity` protection flags in `Metadata.additional` for DOCX, XLSX and PPTX.
- PaddleOCR on the tract backend, available on `wasm32` and the Android x86_64 emulator.
- `LlmConfig` `top_p`, `stop`, `seed`, `presence_penalty` and `frequency_penalty` are validated and sent to providers; they were previously dropped.
- `LlmConfig.max_concurrency` bounds VLM OCR and image-captioning requests independently of `max_threads` ([#1453](https://github.com/xberg-io/xberg/issues/1453)).
- Every error variant has a stable FFI error code, so typed errors work in Go (`errors.Is`), Java and Zig.
- `html_to_markdown_rs::ConversionOptions` is re-exported at the Rust crate root, and `DocumentNode`'s text and node-type accessors are public.
- `FormatMetadata::html()`.
- Opt-in Pdfium PDF backend behind the `pdf-pdfium` feature (`--pdf-backend pdfium`), with page count, per-page text and Info metadata only; results carry a `ProcessingWarning` naming the missing features. Not part of `full`.
- Scoop manifest in `xberg-io/scoop-bucket` (`scoop install xberg`).
- A `ProcessingWarning` when a document decodes lossily or content is skipped (archive, AsciiDoc, WebVTT, XML, plain text, ODT images and styles, LaTeX/Typst/RST/Org includes, OPML, truncated XML, URI cap, Tesseract word failures) ([#171](https://github.com/xberg-io/xberg/issues/171), [#133](https://github.com/xberg-io/xberg/issues/133)).
- A PDF page whose render is blank falls back to OCR of its embedded images, keeping tables, formulas and metadata the backend produced.

### Changed

- **Breaking (Python binding):** `ExtractionConfig` and `DoctorReport` are frozen dataclasses instead of `TypedDict`s. `extract()` still accepts a `dict` or JSON string, but a config object no longer supports mapping operations and is immutable.
- **Breaking (Rust source):** `validate_mime_type` requires an exact registered MIME type; unregistered `image/` subtypes are rejected as `UnsupportedFormat` (#1511).
- **Breaking (Rust source):** `ExtractionConfig` adds `apply_notebook_cell_tags`; notebook extraction honours MyST and Jupyter Book remove/hide cell tags by default ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- **Breaking (Rust source):** `OcrQualityThresholds` adds `discard_suspected_ocr_noise`.
- **Breaking:** Configuration deserialization rejects unknown fields in nested configuration tables.
- **Breaking:** The PDF backend is `"native"` / `PdfBackend::Native` instead of `"pdf_oxide"` / `PdfBackend::PdfOxide`; the default is unchanged.
- **Breaking:** `EmbeddingModelType::Llm` and `RerankerModelType::Llm` carry their model name.
- **Breaking:** `Formula.bbox` and `Formula.page` are optional.
- **Breaking:** Unknown multipart fields on extraction endpoints return an error.
- **Breaking:** The `ElementId` wrapper is removed; element identifiers are `String`.
- **Breaking:** Public tuple fields (ranges, coordinates, dimensions, links, code blocks, attributes) are named structs serialized as JSON objects; 1.0.x positional arrays are still accepted when parsing.
- **Breaking:** The duplicate `xberg::llm::region_extractor::RegionKind` is removed; `xberg::RegionKind` remains.
- Chunk `content` contains the exact source span; heading breadcrumbs are available separately.
- The CLI `all` feature includes audio transcription.
- `create_client_with_credential_provider` returns `ManagedClient`, and an LLM concurrency limit of zero is rejected.
- Native PDF pages expose their final reading order.
- WebVTT cue timing is optional.
- OpenDocument packages without `content.xml` return an extraction error.
- Invalid region, redaction and reranker values are rejected.
- CLI text output includes the extraction envelope (warnings, timings, metadata); CLI JSON output reports peak resident memory.
- Windows builds include the same feature set as other desktop builds.
- Corrected MIME and extension routing for DBF, YAML, reStructuredText, Org, Typst, XHTML, Djot, JPEG 2000, HEIC/HEIF, MP4 and MPEG.
- GeoJSON extraction returns a bounded summary by default; `geojson.include_full_coordinates = true` returns the full document.
- `quality_score` measures the cleanliness of retained text, not extraction completeness.
- `security_limits.max_table_cells` errors explain the limit and how to raise it.
- Recoverable PDF parse issues are logged at TRACE/DEBUG instead of WARN, with one DEBUG summary per document on the `xberg_native_pdf::recovery` target ([#1547](https://github.com/xberg-io/xberg/issues/1547)).
- `TesseractConfig.language_model_ngram_on` defaults to `true` for PDF and image OCR.
- Tesseract Markdown OCR drops lines that are mostly non-dictionary words and reports the count as a `ProcessingWarning`.
- OCR routing for undecodable text is decided per page instead of for the whole PDF.
- With `max_threads` unset, the thread budget is `min(num_cpus, 8)` (or the cgroup CPU quota) and caps Rayon, ONNX Runtime and batch workers; hosts with more than 8 cores log a one-time warning ([#1392](https://github.com/xberg-io/xberg/issues/1392)).
- PaddleOCR inference uses the thread budget instead of a single thread.

### Removed

- **Breaking:** Removed the inert `ChunkingConfig::prepend_heading_context`, `breadcrumb_target`, `BreadcrumbTarget` and their CLI and environment options.
- **Breaking:** Removed `OutputFormat::Structured` (renamed to `DocTags`; see 1.1.1).
- **Breaking:** Removed `ExtractedDocument.formatted_content` from the language bindings.
- **Breaking:** Removed the inert `Engine` structured-policy, preset-resolver, LLM-client and model-provider injection methods.
- **Breaking:** Removed the inert transcription field from `EnrichmentConfig`.
- **Breaking:** Embedding, reranking, sparse-embedding, late-interaction and preset APIs are exposed only when their features are enabled.
- **Breaking:** `core::batch_mode`, `core::formats` and `core::io` are crate-private, and `DocumentStructureBuilder` is removed.
- Removed advertised support for troff, mdoc, POD and DokuWiki, which had no structural extractors.
- Removed fabricated OCR `script_name` and `script_confidence` values.
- Removed the unused public `LanguageRegistry`, `BatchProcessor`, object-pooling APIs and unused tree-sitter re-exports.
- Removed the nonfunctional `wasm-threads` feature.
- Removed PDF writing, editing, building and XFA conversion APIs from the native PDF crate; read-only XFA analysis remains.

### Fixed

- **(pdf):** A ruled page mixing data rows with headings or lead-ins no longer collapses into one table that swallows its section headings ([#1565](https://github.com/xberg-io/xberg/issues/1565)).
- **(pdf):** A drawn table row with one wrapped cell is no longer split into extra rows ([#1555](https://github.com/xberg-io/xberg/issues/1555)).
- **(pdf):** A word split across two touching spans in different subset fonts is no longer rejoined with a space (`pri js`) ([#1566](https://github.com/xberg-io/xberg/issues/1566)).
- **(pdf):** Table reconstruction keeps surplus inferred header rows as data instead of dropping them ([#1558](https://github.com/xberg-io/xberg/issues/1558)).
- **(pdf):** Reading order no longer splits one visual table row and fuses neighbouring identifiers ([#1560](https://github.com/xberg-io/xberg/issues/1560)).
- **(pdf):** Suspended hyphens such as `vracht- en verzendkosten` are preserved across style boundaries ([#1561](https://github.com/xberg-io/xberg/issues/1561)).
- **(pdf):** A hyphen at a line break is kept when the same hyphenated compound appears elsewhere in the document (`price-determining`) ([#1543](https://github.com/xberg-io/xberg/issues/1543)).
- **(pdf):** Text extraction no longer rewrites `:` to `ti` and `M` to `tti` between lowercase letters ([#1556](https://github.com/xberg-io/xberg/issues/1556)).
- **(pdf):** Fonts such as "Monotype Corsiva" are no longer detected as monospace.
- **(pdf):** A standalone multi-line monospace paragraph is recognised as a code block ([#1557](https://github.com/xberg-io/xberg/issues/1557)).
- **(pdf):** All captured render warnings are reported as `ProcessingWarning`s ([#1548](https://github.com/xberg-io/xberg/issues/1548)).
- **(pdf):** A statistics table beside a prose column is emitted whole instead of interleaved with the prose ([#1545](https://github.com/xberg-io/xberg/issues/1545)).
- **(pdf):** A short `Tj` run between two `TJ` arrays no longer scrambles the text ([#1544](https://github.com/xberg-io/xberg/issues/1544)).
- **(pdf):** A font whose `/Widths` declares 0 for an ordinary glyph no longer collapses text on itself.
- **(pdf):** `pdf_options.hierarchy.enabled` produces a hierarchy without also setting `pages.extract_pages`.
- **(docx):** Page attribution no longer stays low when Word omits a page marker between stacked images ([#1559](https://github.com/xberg-io/xberg/issues/1559)).
- **(docx):** Text boxes keep character references such as `&amp;` and `&#8364;` ([#1562](https://github.com/xberg-io/xberg/issues/1562)).
- **(docx):** A cell merged across columns or rows is returned once instead of once per covered cell, and merged cells no longer shift header/footer table columns ([#1549](https://github.com/xberg-io/xberg/issues/1549)).
- **(docx):** Images report their actual `page_number` instead of always 1 ([#1546](https://github.com/xberg-io/xberg/issues/1546)).
- **(doc):** Legacy `.doc` headings use the document's heading styles and levels when present ([#1553](https://github.com/xberg-io/xberg/issues/1553)).
- **(doc):** Legacy `.doc` automatic list numbering produces `ListItem`s in list containers ([#1550](https://github.com/xberg-io/xberg/issues/1550)).
- **(doc):** Legacy `.doc` (Word97+) emits one element per Word paragraph instead of splitting on blank lines. **This changes element boundaries, counts and indices, and `content` line spacing, for most `.doc` documents.**
- **(doc):** Legacy `.doc` text is read through the piece table, fixing documents returned as garbled CJK-like text and adding footnote, header/footer, comment and text-box text ([#1551](https://github.com/xberg-io/xberg/issues/1551)).
- **(ocr):** OCR image decoding honours `ExtractionConfig::security_limits` on every route; PaddleOCR also honours `backend_options["security_limits"]` ([#1554](https://github.com/xberg-io/xberg/issues/1554)).
- **(ocr):** OCR backends registered with `register_ocr_backend` are no longer rejected by config validation.
- **(python, php):** Optional fields omitted from JSON payloads fall back to their defaults instead of failing.
- **(python):** Optional constructor arguments have precise keyword types for type checkers.
- **(langchain):** `XbergLoader` honours chunking and per-page splitting when given an `ExtractionConfig` object.
- **(ffi):** The C FFI library includes summarization, translation, analysis, HEIC, captioning, ML redaction and static embeddings, so the Java, Go, C#, Swift, Zig and C bindings have them.
- **(html):** HTML fetched over HTTP(S) keeps its format metadata (title, headings, Open Graph, Twitter card, links, structured data).
- **(url):** URL extraction reports the source MIME type instead of `text/markdown` for converted HTML.
- **(plugins):** `clear_post_processors` attempts every shutdown hook, returns the first error, and restores built-ins.
- **(ocr):** VLM concurrency limits no longer increase local OCR concurrency and raster memory (#1465).
- **(llm):** Structured extraction validates against the schema's declared JSON Schema draft instead of forcing 2020-12 ([#1539](https://github.com/xberg-io/xberg/issues/1539)).
- **(ocr):** Hybrid PDF OCR no longer drops prose around a table-bearing page.
- **(ocr):** The automatic PDF OCR fallback no longer reports empty success when OCR fails and no native text remains.
- **(ocr):** Degraded VLM fallback output no longer replaces denser OCR text.
- **(image):** Compressed images with oversized declared dimensions are rejected from the header against `security_limits.max_content_size` instead of exhausting memory.
- **(ocr):** Dot leaders and similar native content no longer suppress OCR of image-only PDF pages.
- **(pdf):** Glyph spacing and batch output no longer depend on document order and concurrency for fonts with indirect width tables.
- **(ocr):** Markdown OCR word counts and confidence statistics cover only text kept after dictionary filtering.
- **(pdf):** Repeated bold presenter labels and same-row legend keys are no longer promoted to headings.
- **(ocr):** Fragmented, low-confidence or dictionary-suspect PDF OCR text is kept with a processing warning instead of emptying the page; `ocr.quality_thresholds.discard_suspected_ocr_noise = true` restores the previous filtering.
- **(ocr):** System-linked Tesseract builds no longer crash at runtime.
- **(cli):** `xberg batch` emits every successful document and per-input error before returning a nonzero status on mixed results; TOON output uses the documented batch envelope.
- **(cli):** `xberg extract --ocr false` disables implicit OCR fallback and rejects contradictory OCR flags.
- **(ocr):** Tesseract preprocessing settings (deskew, denoise, contrast, Otsu/adaptive/Sauvola binarization, `none`/`off`) take effect on native and WebAssembly backends; WebAssembly Tesseract rejects images over 4096 x 4096 pixels.
- **(api):** The OpenAPI document no longer has a dangling Djot attribute reference (#1505).
- **(mime):** XML and JSON with unsupported specialized extensions use the generic extractor instead of failing MIME validation (#1507).
- **(mime):** Paths with an unknown or missing extension fall back to content sniffing (#1506).
- **(mime):** An `application/octet-stream` hint triggers MIME detection.
- **(epub):** EPUB extraction handles `text/html` spine items, named entities, non-UTF-8 encodings, navigation documents, SVG fallbacks, nested tables, MathML, headings, images and malformed HTML (#1486, #1488-#1494).
- **(epub):** A failing spine item produces a warning instead of failing the whole document (#1491).
- **(epub):** Fixed EPUB metadata, EPUB 2/3 cover selection, DRM detection and font obfuscation (#1492, #1494).
- **(pdf):** Fixed OCR and rendering for highly compressed scans, CCITT images, CFF fonts, maximum-size font tables, malformed fonts, rotated content, missing-glyph warnings and concurrent Pdfium extraction.
- **(pdf):** Corrupt optional content is logged as a recoverable warning; cross-reference failures log one error.
- **(pdf):** Visible FreeText annotations in annotation-only PDFs are recovered into page text; hidden or disabled annotations are not exposed.
- **(pdf):** Scanned PDFs: CCITT parameters align with their filter, JBIG2 masks reach OCR, and stencil masks render text opaque.
- **(pdf):** Reading order for dense two-column layouts, hanging clause numbers, split list markers and font-size changes on one baseline.
- **(pdf):** Repeated bold body-size section titles are recovered as headings; short bold labels stay body text (#1513).
- **(pdf):** Table extraction for multi-word cells, rule-less prose regions, OCR tables and page-local table failures (#688, #1358, #1542).
- **(pdf):** Markdown and Djot output keep native text when structured conversion is incomplete.
- **(pdf):** Metadata suppression and header/footer settings apply to every backend; invalid PDF and OCR settings return configuration errors.
- **(ocr):** OCR-backed PDFs preserve filtering, confidence thresholds, hierarchy, tables, formulas, lists, bounding boxes and page boundaries consistently across formats and backends (#1444).
- **(ocr):** Fixed Tesseract caching, configuration, preprocessing, page segmentation and font-size extraction.
- **(ocr):** Tesseract Markdown extraction reports a `ProcessingWarning` with the number of lines removed by dictionary filtering.
- **(ocr):** OCR element hierarchy honours `build_hierarchy` and contains only resolvable parent references.
- **(ocr):** Fixed Sceptre and PaddleOCR line grouping, region ordering, per-page resizing, table validation and font sizes.
- **(docx):** Fixed nested tables, VML images, text boxes, comments, fields, headings, hyperlinks, headers, footers, TOC entries, nested lists and page attribution; explicit page breaks are preserved (#1452, #1460, #1503).
- **(pptx):** Fixed malformed relationships, nested image paths, equations, fallback shapes, comments, metadata and security limits.
- **(spreadsheets):** Fixed hyperlinks, formulas, names, comments, hidden state, dates and OpenDocument metadata.
- Fixed ODT, ODP, iWork, HWP, DBF, RTF, email and PST extraction of nested content, metadata, binary data, folder traversal and repeated text.
- Fixed Markdown, MDX, RST, HTML, DocBook, JATS, FictionBook, Djot, Org, YAML frontmatter and Jupyter structure and content retention.
- `result.elements` headings report their level in `metadata.additional["heading_level"]` (#1504).
- Fixed CSV parsing of stray quotes and archive extraction order.
- HTML is detected before the generic XML fallback, and supported-format lists reflect the active extractor registry.
- Post-processing, chunking, enrichment, translation, NER, QR codes, captions and caching preserve extracted structure.
- Chunking presets apply the documented size and overlap without discarding other chunking settings.
- Timed-out extraction work is cancelled.
- Changing one CLI option no longer erases sibling configuration settings.
- **(api):** Multipart extraction accepts `json` and `doctags` for `output_format`.
- Cache keys include only settings that affect the result.
- OCR, embedding and reranking settings no longer reuse incompatible cached models.
- Batch extraction uses configured caches, and progress callbacks report completed items.
- Invalid nested configuration values, including OCR quality and scanned-page thresholds, are rejected by every entry point.
- Errors distinguish every documented extraction failure category.
- Built-in extraction always reports a recognised `extraction_method`.
- Owned document classification writes labels back to the returned document.
- `ContentFilterConfig.include_watermarks` keeps watermark content.
- `JsonExtractionConfig.flatten_nested_objects = false` preserves nested objects.
- Standalone-image and OCR-backed PDF results keep preprocessing scale, dimensions and DPI.
- Candle OCR backend options are validated and applied.
- PaddleOCR-VL honours the task chosen at construction unless a request overrides it.
- Keyword extraction returns an error for invalid n-gram ranges instead of empty results.
- `clear_ocr_backends()` and `clear_renderers()` re-register the built-ins instead of leaving the registries empty.
- `pages[N].content` renders nested lists correctly; figure alt text is kept alongside captions; the VLM OCR probe checks credentials; the PDF margin filter handles rotated text.
- **(packaging):** Fixed Node.js native-library loading, Swift iOS resolution, Windows DirectML packaging and `cargo install xberg-cli` (#1456).
- **(packaging):** Fixed Docker image builds; the CLI image contains only runtime dependencies.
- **(packaging):** Fixed API and packaging defects in the Python, PHP, Dart, Go, Java, C#, Kotlin, Elixir, Ruby, Zig and C packages.
- **(packaging):** Fixed Windows wheel and gem packaging, manylinux compatibility, musl builds and dynamic Tesseract builds (#1495, #1497).
- **(packaging):** The Windows Ruby gem builds.
- **(packaging):** The Swift package links on Linux.
- **(packaging):** SwiftPM no longer warns about a missing license file.
- **(packaging):** HEIC-enabled builds require libheif 1.19 instead of 1.21, so the PHP extension loads on Debian 13 ([#1541](https://github.com/xberg-io/xberg/issues/1541)).
- **(packaging):** The Elixir Hex package's vendored `Cargo.lock` resolves for `--locked` source builds.
- **(packaging):** Ruby source-package installation works with the gemspec's `rb_sys` range.
- **(build):** Source builds work on Windows without the MSVC developer environment, on stable Rust for Windows and Ruby packages, with GCC 12+ for WordPerfect support, and with only the `api` or `mcp` feature.
- **(wasm):** The `excel-wasm` feature builds, and unsupported managed credential providers are rejected.
- **(archives):** ZIP validation handles small compressed entries and impossible declared sizes (#1496).
- **(ocr):** Automatic PDF OCR no longer replaces healthy native text with a much poorer recognition.
- **(ocr):** OCR of a single detached page image is attributed to the correct page.
- **(ocr):** Image-level OCR no longer re-runs over a page-sized XObject on a page whose native text was already extracted, which duplicated content ([#1479](https://github.com/xberg-io/xberg/issues/1479)).
- **(ocr):** Rendered PDF pages are OCR'd at their real DPI instead of assuming 72 DPI; DPI normalisation also runs in candle-backend and VLM-only builds.
- **(ocr):** Layout detection no longer marks figure and drawing labels such as `SITE PLAN` and `LEGEND` as page furniture.
- **(ocr):** `candle-trocr` rejects input taller than a text line instead of returning invented text.
- **(ocr):** OCR handles pages with `/Rotate`, and auto-rotation no longer double-corrects the page hint.
- **(ocr):** OCR'd PDF pages report node, hierarchy, chunk and table bounding boxes in PDF points with a bottom-left origin, like digital pages ([#1423](https://github.com/xberg-io/xberg/issues/1423)).
- **(ocr):** An explicitly requested GPU execution provider (CUDA, TensorRT, CoreML) that fails to load returns an error instead of running on CPU; `Auto` still falls back.
- **(ocr):** Internal OCR keys no longer appear in document metadata.
- **(xml):** Elements nested more than 255 levels no longer wrap to a low heading level or panic before `max_xml_depth` applies ([#1474](https://github.com/xberg-io/xberg/issues/1474)).
- **(pdf):** XMP metadata keeps text around XML entity references ([#1475](https://github.com/xberg-io/xberg/issues/1475)).
- **(pdf):** A numbered subsection heading followed by same-style lines, and consecutive numbered headings, are no longer merged into one paragraph ([#1467](https://github.com/xberg-io/xberg/issues/1467), [#1386](https://github.com/xberg-io/xberg/issues/1386)).
- **(pdf):** List items keep their printed markers (`B.`, `(a)`) instead of being renumbered.
- **(pdf):** A `/ModDate` with invalid bytes no longer panics (aborting Go processes), and a rasterizer panic on one damaged page no longer loses every page's text ([#1422](https://github.com/xberg-io/xberg/issues/1422), [#1408](https://github.com/xberg-io/xberg/issues/1408)).
- **(pdf):** Page-number stripping no longer deletes short numeric table cells ([#1411](https://github.com/xberg-io/xberg/issues/1411)).
- **(pdf):** Paragraph breaks are detected on normally set pages instead of returning the page as one line.
- **(pdf):** Detected tables are no longer rendered on top of native text that already contains them.
- **(pdf):** Text and tables on rotated pages: a single rotated caption no longer disrupts the page, and table cells cluster on the table's own axes ([#1358](https://github.com/xberg-io/xberg/issues/1358)).
- **(docx, pptx):** Relationships targeting `../media/image1.png` resolve instead of being dropped as traversal.
- **(docx):** Documents with legacy VML pictures are no longer rejected as `NestingTooDeep`, and nesting depth is measured correctly everywhere ([#1395](https://github.com/xberg-io/xberg/issues/1395)).
- **(doc):** Legacy `.doc` no longer emits field instructions as prose, and keeps non-breaking hyphens.
- **(ppt):** Legacy `.ppt` slide numbers come from the slide containers, and images are extracted ([#1418](https://github.com/xberg-io/xberg/issues/1418), [#1417](https://github.com/xberg-io/xberg/issues/1417)).
- **(pptx):** Slides without a title keep their page number ([#1413](https://github.com/xberg-io/xberg/issues/1413)).
- **(html):** Inline `<svg>` elements are extracted when `extract_images` is enabled ([#745](https://github.com/xberg-io/xberg/issues/745)).
- **(html):** Styled HTML output escapes non-HTML raw blocks (ODP notes, Org source, script and style bodies, Djot raw blocks).
- **(api):** The Docling-compatible endpoint honours OpenWebUI's per-field extraction parameters ([#1462](https://github.com/xberg-io/xberg/issues/1462)).
- **(api):** The OpenAPI document includes nested component schemas, the PDF, office and transcription schemas, and the `415` and `429` responses ([#1424](https://github.com/xberg-io/xberg/issues/1424)).
- **(cli):** `--ocr-backend`, `--ocr-language`, `--ocr-auto-rotate` and `--ocr-backend-options` work without `--ocr true`, `--ocr-scanned-pages` alone runs OCR, and `--chunk-size` works without `--chunk true`.
- **(llm):** `XBERG_LLM_API_KEY` and `XBERG_LLM_BASE_URL` no longer create an empty structured-extraction config that failed every document ([#1421](https://github.com/xberg-io/xberg/issues/1421)).
- **(keywords):** Keyword extraction no longer panics on a language hint starting with a multi-byte character.
- **(url):** URL extraction reports crawled URLs, deduplicated in first-seen order.
- **(code):** `code_intelligence` is populated with metrics, imports, exports, comments, docstrings, symbols and diagnostics ([#259](https://github.com/xberg-io/xberg/issues/259)).
- **(transcription):** Whisper timestamp tokens no longer appear as text; transcripts are emitted as one paragraph per timed segment.
- **(build):** `cargo add xberg --features full` links on Windows MSVC ([#1389](https://github.com/xberg-io/xberg/issues/1389)).
- **(packaging):** musl (Alpine) Java, C#, Zig, C and Elixir artifacts bundle ONNX Runtime's dependencies and load; a host that already links libstdc++ needs libstdc++ 15 or newer.
- **(packaging):** PyPI `xberg-cli` wheels include their native libraries.
- **(embeddings):** `show_download_progress` takes effect on embedding, sparse-embedding, reranker and late-interaction models.
- **(ocr):** `target_dpi`, `max_image_dimension`, `auto_adjust_dpi`, `min_dpi` and `max_dpi` take effect ([#209](https://github.com/xberg-io/xberg/issues/209)).
- `split_and_extract` keeps keywords, entities, summaries, chunks, warnings and other enrichment, and chunks point at the correct images.
- **(telemetry):** Cache-hit, cache-miss and batch metrics, pipeline and batch operations, all pipeline stage spans, and extractor-priority and batch attributes are recorded ([#332](https://github.com/xberg-io/xberg/issues/332), [#282](https://github.com/xberg-io/xberg/issues/282)).
- An injected cache backend is consulted and progress events are emitted for single extraction.
- **(renderers):** JSON output includes page breaks, footnotes, citations, slides, definition terms, admonitions, raw blocks and metadata blocks; styled HTML closes slide sections and renders slide titles; formulas render as delimited display math.
- **(renderers):** Footnote definitions appear in JSON output, and unreferenced definitions are no longer dropped ([#68](https://github.com/xberg-io/xberg/issues/68)).
- **(plugins):** Plugin-produced documents keep `uris`, `children`, `annotations`, `processing_warnings`, `llm_usage`, `pages` and `ocr_elements`, native renderers produce output, and `pre_rendered_content` is used for HTML and JSON.
- CRLF documents (email, PST, OCR output, plain text, Djot) are no longer collapsed into one paragraph ([#227](https://github.com/xberg-io/xberg/issues/227)).
- **(mime):** `application/wordperfect`, `application/x-quarto` and four audio/video aliases route to their extractors instead of failing as `UnsupportedFormat`.

### Security

- DOCX image and iWork archive member reads are bounded instead of trusting the declared size, preventing memory exhaustion from forged ZIP entries. Reported by Syed Anas Mohiuddin ([GHSA-85w9-wqcq-x48r](https://github.com/xberg-io/xberg/security/advisories/GHSA-85w9-wqcq-x48r)).
- ZIP, TAR and 7z member reads (generic archives, ODT, ODP, EPUB, HWPX, PPTX, XLSX, OOXML embedded objects) are bounded by `SecurityLimits` with compression-ratio and aggregate-size checks; nested ZIPs no longer overflow the stack.
- ZIP accounting is hardened against overflow, impossible sizes and compression-ratio bypasses.
- `security_limits.max_files_in_archive` is enforced by every OOXML container ([#1449](https://github.com/xberg-io/xberg/issues/1449)).
- The native PDF engine no longer aborts, panics, hangs or allocates without bound on crafted documents (recursive name trees, deep nesting, malformed xref, CMap, font and image parameters, unguarded stream decoding).
- The native PDF reader's internal caches are bounded.
- Document-declared counts and offsets (HWP, HTML, EPUB, DOCX, PPTX, RST, JATS, hOCR) are clamped or rejected instead of panicking or aborting.
- DOCX, PPTX and EPUB relationship resolution rejects traversal, malformed UTF-8, NUL bytes, drive-letter and UNC paths, and symlink escapes.
- EPUB traversal and retained content are bounded; OPF nesting depth is enforced and external or amplified DTD entities are not resolved ([#1477](https://github.com/xberg-io/xberg/issues/1477), [#1478](https://github.com/xberg-io/xberg/issues/1478)).
- Structured extraction resolves JSON Schemas offline and rejects external HTTP and file references.
- REST and MCP requests can no longer override LLM credentials, provider registrations or other server-controlled settings.
- Cache namespaces are validated before directories are created.
- Redaction reports only content actually removed, never exposes pre-redaction text, and rejects invalid strategies.
- Native PDF tracing no longer logs document text or font names; failures log an `error_code` and byte offset, and font names are redacted from warnings.
- Downloaded Tesseract, Leptonica and English tessdata build inputs are pinned to verified SHA-256 digests.

## [1.0.14] - 2026-08-04

### Fixed

- **(api):** The OpenWebUI-compatible endpoints (`PUT /process`, `POST /v1/convert/file`) honour the server's configured defaults and per-request config (multipart `config`/`parameters` field or `X-Config` header), defaulting to Markdown only when no format is selected.
- **(docker):** Image captioning is included in the official Docker images, and a `ProcessingWarning` is emitted when `captioning` is configured but compiled out (#1382).

### Changed

- **(captioning):** Embedded-image captioning runs with bounded concurrency instead of one request at a time (#1378).

## [1.0.13] - 2026-08-04

### Fixed

- **(ocr):** OCR-backed PDF extraction keeps consecutive Tesseract paragraphs in the same hOCR text area together, including on mixed native/OCR pages.
- **(pdf):** Sparse, short-wide contact blocks are no longer detected as tables.
- **(ocr):** Standalone-image Tesseract OCR defaults to sparse-text segmentation, cropped layout regions use single-block segmentation, and vertical language packs such as `jpn_vert` use vertical-block segmentation; explicit settings are unchanged.
- **(ocr):** Standalone image extraction reports successful OCR via `metadata.ocr_used` and the OCR extraction method.
- **(ocr):** Tesseract's default preprocessing applies only to clean document pages; receipts and photos keep their source pixels.
- **(ocr):** Sparse, low-confidence standalone Tesseract results are retried with automatic segmentation and preprocessing, used only when confidence is consistently strong.
- **(csv):** CSV and TSV plain text uses the table renderer instead of `Row N` prose.
- **(email):** EML and MSG attachment text is included in the parent document; attachment children remain available.
- **(docx):** An in-run `<w:tab/>` is extracted as a tab instead of joining adjacent words (#1377).
- **(swift):** The Swift package builds and publishes again; broken in 1.0.12.
- **(dotnet):** The NuGet runtime packages for macOS and Linux publish at the current version (#1375).

## [1.0.12] - 2026-08-03

### Fixed

- **(mcp):** `xberg mcp` `extract` and `extract_batch` output matches the declared output schema, so MCP clients no longer reject it (#1372).
- **(install):** `install.sh` no longer creates a self-referential `xberg` symlink, and installs the glibc build on glibc Linux and the musl build on musl systems (#1371).
- **(csv):** An all-text first row such as `Name,City` is treated as the header; a numeric first row is still data (#1369).

## [1.0.11] - 2026-08-03

### Fixed

- **(server):** The HTTP server bounds in-flight requests (default `2 x CPU count` clamped to `[4, 32]`, set via `XBERG_MAX_CONCURRENT_REQUESTS`, `0` disables) so bursts of large uploads no longer exhaust memory (#1368).
- **(ocr):** PaddleOCR keeps consecutive lines in the same Markdown paragraph.
- **(ocr):** PaddleOCR and Tesseract automatic rotation use the orientation model correctly and recover sparse edge-aligned text.

## [1.0.10] - 2026-08-02

### Fixed

- **(cli):** `cargo install xberg-cli` succeeds on a stock Windows toolchain. HEIC is no longer a default CLI feature (`heic`); prebuilt binaries, the Docker `all` image and the Homebrew bottle still include it (#1361).
- **(cli):** The `cargo binstall xberg-cli` musl builds, missing from 1.0.9, are published.
- **(cli):** `brew install xberg-io/tap/xberg` installs a working binary instead of an empty bottle (#1356).
- **(dart):** Native-library loading reports download, filesystem and checksum failures instead of falling back to an unverified library path.
- **(ocr):** Concurrent PaddleOCR cold starts share one engine per model and accelerator, with separate entries per GPU device ID.

## [1.0.9] - 2026-08-02

### Added

- **(cli):** `cargo binstall xberg-cli` installs a fully static musl binary with no ONNX, Tesseract or libheif runtime dependencies. `x86_64-unknown-linux-musl` includes the Candle VLM OCR backends (TrOCR, PaddleOCR-VL); `aarch64-unknown-linux-musl` is extraction-only.

### Changed

- **(ocr):** PaddleOCR is exposed as `PaddleOcrEngine` with word-level quadrilaterals; `OcrLite` remains as a deprecated alias.
- Lower memory use and copying in XLSX extraction, layout-enabled image table recognition, multi-stage PDF OCR and batch DOCX extraction.

### Fixed

- **(cli):** The macOS CLI tarballs bundle libheif, so the binary no longer fails with `libheif.1.dylib` not loaded (#1357).
- **(cli):** PaddleOCR-only CLI builds without layout detection compile.
- **(ocr):** PaddleOCR keeps native resolution for 1024-pixel images by default.
- **(ocr):** PaddleOCR detection honours its DB threshold and matches upstream dilation, crop and line ordering.
- **(ocr):** PaddleOCR layout and table output uses word boxes without duplicating table text.
- **(ocr):** PaddleOCR reports processed image dimensions and orientation corrections.
- **(ocr):** PaddleOCR selects the Japanese model for vertical Japanese, prefers Korean or Japanese models for mixed Latin requests they cover, and uses the Korean recognizer for PP-OCRv6 Korean requests.
- **(ocr):** PaddleOCR keeps right-to-left column order in vertical Chinese and Japanese documents.
- **(ocr):** Image OCR keeps blank-line paragraph boundaries.
- **(ocr):** Tesseract vertical CJK OCR removes artificial spaces between characters.
- **(ocr):** Layout-enabled image OCR reuses whole-image text when structured assembly is unavailable instead of re-running region OCR.
- **(ocr):** Layout-enabled image extraction recognizes table structure from cached OCR elements without losing tokens.
- **(ocr):** TATR table reconstruction assigns each OCR word exactly once, preventing duplicated and dropped text.
- **(ocr):** Layout-enabled OCR keeps detected image headings and regroups adjacent PDF OCR lines without merging distant paragraphs.
- **(ocr):** Rotated PDF OCR re-runs layout on upright images instead of reusing display-coordinate rasters.
- **(iwork):** Keynote packages with only slide archives route to the Keynote extractor, and Numbers tables are reconstructed instead of raw protobuf fragments.
- **(mime):** AsciiDoc, NXML/JATS and WebVTT files route to their extractors instead of being unsupported.
- **(xlsx):** Standalone `excel` and `excel-wasm` feature builds include what XLSX extraction needs.
- **(org):** Org-mode tables distinguish header separators from headerless tables and keep every data row.
- **(epub):** `epub:switch` branches resolve per output renderer.
- **(typst):** Typst headings are marker-free and explicit table headers are distinguished from rows.
- **(msg):** MSG HTML-only message bodies are extracted.
- **(jupyter):** Notebook paths keep `application/x-ipynb+json` routing, and content no longer includes diagnostic cell/output markers; cell details remain in metadata.

## [1.0.7] - 2026-07-31

### Added

- **(ocr):** The Candle VLM OCR backends (`candle-trocr`, `candle-paddleocr-vl`, `candle-glm-ocr`, `candle-deepseek-ocr`) are included in the published packages for Python, Node, Go, Java, C#, Ruby, PHP, Elixir, Kotlin/JVM, Zig and the CLI / Docker image on Linux, macOS and Windows; weights download from Hugging Face on first use. Not available on WebAssembly, Android, iOS, Dart or Swift.

### Fixed

- **(ocr):** A `force_ocr` page that renders blank but has embedded images is OCR'd from the image bytes with a processing warning instead of returning a blank page (#1355).
- **(xlsx):** `.xlsx` files extract on Windows instead of returning `UnsupportedFormat`.
- **(swift):** The Swift binary bundle builds again for `x86_64-apple-darwin` and Linux, without HEIC.
- **(ocr):** Image layout extraction reuses whole-image OCR elements and falls back when region OCR loses text or quality.
- **(epub):** Duplicated MathML and media fallback content, and a doubled cover image, are removed.
- **(email):** Sender display names are kept alongside addresses, and attachments and nested messages are parsed once.
- **(mime):** FB2 and DocBook files route to their own extractors instead of the generic XML fallback.
- **(json):** Nested JSON objects and arrays render as Markdown headings and lists instead of compact JSON strings.

## [1.0.6] - 2026-07-31

### Fixed

- **(build):** Windows MSVC binding builds link zlib for the WordPerfect extractor instead of failing with `undefined symbol: inflate`.
- **(layout):** Automatic PDF layout inference retries on CPU only after runtime inference failures; explicitly selected providers and recognised `XBERG_ORT_EP` values are respected, blank or unrecognised values are ignored, and the effective provider is used for table reconstruction (#1344).
- **(tables):** Words narrowly crossing the outer edge of side-by-side PDF tables are kept in the outermost cell, preserving financial-table row prefixes.

### Changed

- Dependency upgrades: `crawlberg` 1.1.0, `html-to-markdown-rs` 3.10, `liter-llm` 1.12 (its spans are always emitted).
- The `otel` feature enables `crawlberg`'s and `liter-llm`'s OpenTelemetry integration.

## [1.0.5] - 2026-07-30

### Fixed

- **(build):** The WordPerfect extractor (`xberg-libwpd`) compiles and links on `x86_64-pc-windows-msvc`.
- **(pdf):** Sparse native two-column PDFs keep column order instead of interleaving lines across the gutter (#1345).
- **(ocr):** PaddleOCR emits a `ProcessingWarning` when requested languages are not covered by the selected model, and OCR metadata reports the model actually used (#1346).
- **(layout):** Layout inference no longer silently returns no-layout output when a hardware execution provider fails: macOS `auto` runs RT-DETR on CPU, an explicit provider that fails retries on CPU, and a `ProcessingWarning` reports the fallback or loss (#1344).
- **(tables):** TATR table reconstruction no longer writes cell content and coordinates to stderr (#1349).
- **(pdf):** The Markdown hierarchy no longer merges a distant header and footer into one block (#1350).
- **(node):** The Node package ships complete `index.d.ts` types instead of references to undefined `Js*` types (#1351).
- **(install):** The install script copies nested runtime library directories on Linux musl (#1353).

### Changed

- Diagnostics previously printed to stderr (binding FFI bridges, OCR gate decisions, GLM-OCR debug stats, the CLI `--output-format` deprecation notice) go through `tracing`, controlled by `RUST_LOG` / `--log-level`; `XBERG_DEBUG_OCR` and `XBERG_GLM_DEBUG` are no longer used.
- Repeated per-page warnings from OCR engines and layout models are de-duplicated, one per distinct problem.

## [1.0.4] - 2026-07-30

### Added

- **(mcp):** `extract`, `extract_batch` and `cache_warm` run as cancellable SEP-2663 tasks for clients that support tasks.
- **(mcp):** `cache_clear` and `cache_warm` return typed structured results.

### Changed

- Dependency upgrades: `crawlberg` 1.0.11, `tree-sitter-language-pack` 1.13.6.

### Fixed

- **(ocr):** Default `OcrStrategy::Auto` OCRs scanned PDFs with no text layer instead of returning empty content (#1338).
- **(ocr):** VLM fallback pipelines run for mixed native/OCR PDFs, keep stage diagnostics, and keep the last non-empty result when every stage scores below threshold (#1341).
- **(pdf):** Images and captions render at their reading-order position and column, consistently through chunking, translation and redaction (#1340).
- **(archives):** macOS and tooling metadata entries (`__MACOSX/`, `._*`, `.DS_Store`, `Thumbs.db`, `desktop.ini`, `__pycache__/`, `.pyc`/`.pyo`) are skipped with one aggregated warning, and unsniffable extensionless members are `application/octet-stream` (#1343).
- **(tables):** Short numeric grids with a nearly complete principal row are kept as tables (#1342).
- **(ocr):** Per-file OCR language overrides apply to explicit Tesseract pipeline stages.
- **(pdf):** Plain-text extraction repairs detached subscripts, phone suffixes and final glyphs while keeping RTL, rotated, vertical and math span order.
- **(pdf):** Markdown replaces side-by-side native table groups with validated layout tables as a whole, avoiding mixed grids.
- **(ocr):** OCR Markdown applies layout hints per line, keeping soft-wrapped paragraphs and merging multi-line headings, code, pictures and list items.
- **(ocr):** Tesseract OCR Markdown aligns layout hints and table cells with DPI-normalised, auto-rotated coordinates.
- **(ocr):** OCR Markdown recovers a missing ordered-list item only within a bounded three-item sequence.
- **(pdf):** Markdown keeps strong native headings over a low-confidence layout Code hint.
- **(ocr):** OCR Markdown recovers a title from a first-block logo/title pattern when layout finds no heading.
- **(pdf):** Markdown keeps native heading, list, code and formula semantics while using layout for reading order and tables, merges currency-affix columns, folds wrapped financial-table lines, and drops crop spill on table-dominant pages.
- **(pdf):** Markdown reconstructs paired wrapped financial tables as three-column grids and repairs merged numeric columns.
- **(pdf):** Markdown recognises repeated large-font heading tiers in sparse multi-page documents.

## [1.0.3] - 2026-07-29

### Changed

- **(mcp):** The MCP server uses `rmcp` 3.0.0; tools, prompts and resources are unchanged.
- **(ocr):** Runtime Tesseract language-pack downloads are logged (language, destination, each source URL, success).
- Dependency upgrades: `liter-llm` 1.11.4, `toml` 1.1.4.

### Fixed

- **(tables):** Numeric line-item tables with a trailing partial row stay tables (#1333).
- **(ocr):** Tesseract no longer creates cache directories when caching is disabled (#1336).
- **(ocr):** Light-on-dark scans are inverted before OCR, and `invert_colors` is honoured (`Some` forces, `None` auto-detects) (#1337).
- **(ocr):** NER and summarization are included in the container and CLI builds instead of their config being ignored; under `OcrStrategy::ScannedPages` a whole-document text failure OCRs every page (#1338).
- **(ocr):** VLM OCR uses `XBERG_LLM_*` credentials with a custom `base_url`, normalises openai.com model names, routes bare images through the OCR pipeline so `vlm_fallback` `on_low_quality` fires, and reports per-stage failures as warnings (#1339).
- **(ocr):** Per-file OCR language overrides reach nested Tesseract configurations.
- **(pdf):** Plain-text extraction no longer splits words at detached spans.
- **(pdf):** Plain-text extraction no longer renders native table text twice.
- **(pdf):** Label-heavy financial tables are recovered without merging independent aligned tables.
- **(pdf):** Markdown keeps explicit word boundaries and heading hierarchy.
- **(ocr):** OCR Markdown prefers validated semantic layout hints over broad text regions.

## [1.0.2] - 2026-07-28

Packaging release completing the 1.0.1 rollout; no core extraction changes.

### Added

- **Coding-agent plugin** for Claude Code, Codex, Cursor and OpenCode, plus a Hermes variant, with xberg extraction skills; published as `@xberg-io/opencode-xberg` (npm) and `xberg-hermes-plugin` (PyPI).

### Fixed

- **(php):** The PHP binding builds against `ort` 2.0.0-rc.13, so the Packagist package publishes again.

### Packaging

- Removed the never-built Node `@xberg-io/xberg-win32-arm64-msvc` optional dependency.
- Every binding is republished at 1.0.2, including PHP/Packagist.

## [1.0.1] - 2026-07-28

### Fixed

- **(pdf):** Borderless, text-heavy tables are recovered on pages that also contain an ML-detected table (#1321).
- **(rtf):** Hex byte escapes decode through the font's `\fcharsetN` charset, then `\ansicpgNNNN`, then Windows-1252, so Cyrillic and other non-ANSI text is no longer mojibake; adjacent escapes decode as one run (#1326, #1323).
- **(markdown):** Page markers appear verbatim in Markdown and Djot output instead of being escaped or dropped (#1328).

### Added

- **(layout):** `LayoutDetectionConfig.strategy` (`LayoutStrategy`, default `always`); `auto` runs the layout model only on PDF pages likely to benefit, reports skipped pages in `metadata.format.layout_gated_pages` and `layout_gate_reasons`, and is available as `--layout-strategy` ([#1322](https://github.com/xberg-io/xberg/issues/1322)).

### Packaging

- `xberg-libwpd` is republished with the zlib link fix, so `xberg-cli` builds from crates.io.

## [1.0.0] - 2026-07-27

xberg 1.0.0 is the first stable release of the engine previously developed as **Kreuzberg**, succeeding Kreuzberg v4.9. The Kreuzberg v4 line continues as LTS at [kreuzberg-dev/kreuzberg-lts](https://github.com/kreuzberg-dev/kreuzberg-lts). Changes below are relative to Kreuzberg v4.9.

### Migration from Kreuzberg v4

- Packages are renamed `kreuzberg` → `xberg` in every ecosystem (crates.io, PyPI, npm, Maven, NuGet, Composer, RubyGems, Hex, Go).
- The Rust error type `KreuzbergError` is now `XbergError`.
- Environment variables use the `XBERG_*` prefix instead of `KREUZBERG_*`, and config files are discovered as `xberg.{toml,yaml,yml,json}`.
- **Breaking API changes:** extracted URIs are `ExtractedUri` (formerly `Uri`); document metadata replaces the untyped `additional` bag with typed fields plus a `custom` map.
- The R binding, the EasyOCR backend and the bundled pdfium fork are removed. Existing Kreuzberg v4 installs keep working under their original names.

The full identifier mapping is in the [migration guide](/migration/from-kreuzberg-v4/).

### Added

- **OCR backends:** native PaddleOCR (PP-OCRv6, `medium`/`small`/`tiny` tiers) and a pure-Rust Candle OCR/VLM stack (TrOCR, GLM-OCR, GOT-OCR, DeepSeek-OCR, PaddleOCR-VL), alongside Tesseract; weights hosted on the `xberg-io` Hugging Face org.
- **tract inference path:** classifiers, layout detection (RT-DETR) and auto-rotation run without ONNX Runtime, enabling WASM and mobile inference.
- **Structured LLM extraction:** `extract_structured` and `split_and_extract` with rasterization, chunking, citations, caching and `CallMode` / `MergeMode` / VLM-fallback policies.
- **Audio and video transcription** (Whisper) for `.mp3`, `.wav`, `.m4a`, `.mp4` and `.webm`.
- **Named-entity recognition** (GLiNER2), including an in-browser WASM `NerModel`.
- **Retrieval:** sparse embeddings (SPLADE), ColBERT late interaction and cross-encoder reranking alongside dense embeddings, with sha256-pinned model presets.
- **Text intelligence:** reversible redaction, summarization, translation, VLM image captioning, QR-code detection, document diffing (`revisions` on `ExtractionResult`) and page/chunk classification.
- **URL ingestion:** `map_url` sitemap discovery and batched multi-URL crawling over the `ExtractInput` / `ExtractionOutput` envelope.
- **Formats (98 total):** WordPerfect `.wpd`/`.wp`/`.wp5`, HEIC/HEIF/AVIF, OpenDocument Presentation `.odp`, Quarto/R Markdown, configurable Jupyter cell rendering, and the audio/video formats above.
- **Bindings:** Dart/Flutter, Swift, Kotlin/Android and Zig (15 bindings in total), with Android/iOS cross-compilation.
- **Integrations:** LangChain.js, LlamaIndex, n8n, CrewAI and Spring AI.
- **API:** caller-supplied tokenizers, `TableChunkingMode::RepeatHeader`, heading-path breadcrumbs for RAG chunking, multi-label chunk classification, per-page spans with bounding boxes, `list_supported_formats()` in every binding, `pdf_page_count`, and `DELETE /jobs/{job_id}` on the API server.
- **Code intelligence:** tree-sitter coverage grows from 248 to 306 languages.

### Changed

- **PDF backend:** pdfium is replaced by the pure-Rust `pdf_oxide` engine.
- **Layout-aware PDF pipeline:** reading order from ONNX layout detection (PP-DocLayoutV3 / RT-DETR), per-page OCR of scanned pages, and extraction of AcroForm/XFA fields and outline headings.
- **Public API** stabilised for 1.0, with a Rust-only `Engine`.
- **Renamed** from Kreuzberg to xberg across packages, namespaces and the error type (see Migration).
- **Environment variables** use the `XBERG_` prefix; new layout, OCR model-tier, CoreML and ORT execution-provider variables.
- **Config discovery** accepts `.yml` and falls back to the XDG config directory.
- **Models and cache** live under the `xberg` cache segment and the `xberg-io` Hugging Face org.
- **Python** 3.10–3.14 supported; the SurrealDB connector moves to v3 (dropping `mem://`); default `extraction_timeout_secs` is 60 s.
- **License:** MIT (Kreuzberg 4.8/4.9 was Elastic License 2.0).

### Fixed

More than 150 bugs were fixed during the 1.0 cycle, including:

- **PDF text:** text in Marked-Content blocks is kept (#917); ligatures no longer map to control characters (#1135); glyph-spaced text no longer extracts one character per line (#962); spurious intra-word spaces (#1291, #1222); blank JPEG 2000 images breaking OCR (#1158); XML entity references preserved (#1242).
- **Tables:** bordered tables detected reliably (#964, #1097, #1213); rotated full-page tables (#1220, #1221); duplicate tables (#1288); fragmented per-row tables merged with their header (#1290, #1100); borderless grids keep row associations (#1316, #1319).
- **Reading order and structure:** two-column headings (#1170); panics and dropped documents from stale page boundaries (#1270, #1272); numbered and cover-page headings (#961, #966, #1096, #1098); form field placement (#1120).
- **OCR:** an explicit PaddleOCR backend no longer falls back to Tesseract (#801, #1071, #1088, #1102); consistent scanned-page text and provenance (#1095, #1110, #1281); no spurious auto-OCR on born-digital PDFs (#1176); SIGBUS and NaN-sort panic (#1057, #1179); Candle VLM backends stabilised (#1174, #1175, #1208–#1214); model downloads handle TLS-intercepting CAs, IPv6 blackholes and connect timeouts (#1146, #1249).
- **Chunking:** correct `firstPage`/`lastPage` and byte ranges (#1013, #1074, #1105, #1294); Markdown chunks keep Markdown (#1073, #1094); split-table chunks keep their header (#1100).
- **Bindings and packaging:** Go embed symbols, Java `UnsatisfiedLinkError`, missing C# config types, Node/PHP embedding shapes, Android `.so` loading, macOS wheel floors, musl/ONNX Runtime dependencies (#871, #965, #991, #998, #1008, #1055, #1131, #1257, #1304, #1307); Homebrew 404s, Docker stop signals and multi-arch `-core` images (#1081, #1147, #1247, #1315).
- **Formats:** EML HTML table bodies, DOCX hyperlink/bold overlap, archived Markdown/CSV escaping and Korean-charset EML detection (#942, #1086, #1212, #1237, #1278).
- **Config:** `extraction_timeout_secs` honoured on every path (#830, #911, #1273); `cancel_token`, custom LLM base URLs and page-classification config validate correctly (#937, #944, #1076).

### Removed

- **R binding** (the Kreuzberg v4 LTS line is the last to ship it).
- **EasyOCR backend.**
- **Hunyuan-OCR Candle backend.**
- **Bundled `pdfium-render` fork**, its `KREUZBERG_PDFIUM_BUNDLED_PATH` variable, and the standalone `@kreuzberg/core` npm package.

### Performance

- Lower OCR memory use: capped concurrent Tesseract sessions, early buffer release, reused RGB buffers and copy-free resizing.
- Faster layout inference: pooled sessions, balanced batch threads, no PNG round-trip.
- Leaner engine and batch processing: bounded scheduling, no per-item config clone, a single PDF parse per structured rasterization, streamed batch JSON and base64 hosted embeddings.
- Faster PDF and text processing: streamed RGB conversion, reused OCR render document, fewer redundant parses, and backtracking-free text cleanup.

### Security

- An untrusted RTF size field in the email extractor could allocate up to 4 GB; now bounded (#1058).
- A redaction path could leak PII across about a dozen output fields; fixed (#1223).
- PDF embedded streams are guarded by a decompression-ratio limit and per-file size caps, and a `SecurityBudget` applies to the PDF and email extractors.
- Excel DDE and external-call formulas raise warnings.
- FFI image and attachment buffers carry explicit lengths (#1056, #1059), and panics on malformed input are recoverable errors (#907, #1057, #1198).

### Packaging

- pdf_oxide replaces the pdfium native dependency; libheif (LGPL) and `xberg-libwpd` are vendored; model presets are hosted on `xberg-io` with sha256 manifests.
- Published for 15 targets to crates.io, PyPI, npm, Maven Central, NuGet, RubyGems, Packagist, Hex, pub.dev, Go, Swift Package Manager, Homebrew, Docker (`ghcr.io/xberg-io/xberg`) and a Helm chart.
