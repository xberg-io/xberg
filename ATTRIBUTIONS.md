# Attributions

This document acknowledges sources of vendored code, runtime artifacts, test documents, and baseline data used in the Xberg project.

## Licensing principle

Xberg's own code is MIT-licensed (`LICENSE`; the workspace manifest declares `license = "MIT"`; `xberg-gliner` is the one crate that declares Apache-2.0, see its entry). Third-party and ported code keeps its original licence: files derived from Apache-2.0 sources stay Apache-2.0, with the licence text, upstream copyright and NOTICE content retained and the changes listed in the entry's Modifications section; files derived from MIT sources stay MIT, with the upstream copyright and permission notice retained verbatim. Licence texts are reproduced in `crates/xberg/LICENSE-THIRD-PARTY` or in the `LICENSE-*` file of the crate that carries the code, as each entry states. Runtime dependencies are covered by `THIRD_PARTY_LICENSES.md`. Code moved from Xberg's own sibling projects (for example html-to-markdown) is Xberg's own and is not listed as third-party.

## Pandoc Test Suite

Test documents and reference baseline outputs derived from the Pandoc test suite:

- **Source**: <https://github.com/jgm/pandoc>
- **License**: GPL-2.0-or-later
- **Usage**: Test documents and reference baselines only (no code copied from Pandoc)
- **Attribution**: John MacFarlane and Pandoc contributors
- **Purpose**: Baseline reference testing - used to validate our native Rust extractors work correctly on the same documents that Pandoc processes

### Test Documents from Pandoc

The following test documents were copied from the Pandoc repository to `/test_documents/`:

#### Org Mode

- `org-select-tags.org` - SELECT_TAGS and EXCLUDE_TAGS testing
- `pandoc-tables.org` - Org Mode table formats
- `pandoc-writer.org` - Comprehensive Pandoc test suite in Org Mode format

#### Typst

- `typst-reader.typ` - Fibonacci sequence with mathematical formulas
- `undergradmath.typ` - Comprehensive undergraduate mathematics document (16KB)

#### DocBook

- `docbook-chapter.docbook` - Recursive section hierarchy (7 nested levels)
- `docbook-reader.docbook` - Comprehensive DocBook 4.4 test suite (36KB, 1704 lines)
- `docbook-xref.docbook` - Cross-reference (xref) functionality testing

#### JATS

- `jats-reader.xml` - Comprehensive JATS (Z39.96) Journal Archiving test document (38KB, 1460 lines)

#### FictionBook

- `test_documents/fictionbook/pandoc/` - 13 FictionBook test files including:
  - `basic.fb2` - Basic FictionBook structure
  - `images-embedded.fb2` - Embedded base64 images
  - `math.fb2` - Mathematical content
  - `meta.fb2` - Document metadata testing
  - `reader/emphasis.fb2` - Text emphasis testing
  - `reader/epigraph.fb2` - Epigraph/quote elements
  - `reader/meta.fb2` - Document metadata and title info
  - `reader/notes.fb2` - Footnotes/endnotes with cross-references
  - `reader/poem.fb2` - Poem/verse structure
  - `reader/titles.fb2` - Section titles and heading hierarchy
  - And others

#### OPML

- `opml-reader.opml` - OPML 2.0 outline structure (US states example)
- `pandoc-writer.opml` - Comprehensive Pandoc test suite in OPML format

### Baseline Outputs Generated

For each test document listed above, three baseline outputs were generated using Pandoc 3.8.3:

1. **Plain Text** (`*_pandoc_baseline.txt`) - Raw text content extraction
2. **JSON Metadata** (`*_pandoc_meta.json`) - Full Pandoc AST with document structure and metadata
3. **Markdown** (`*_pandoc_markdown.md`) - Markdown representation for format comparison

**Total**: 132 baseline files for 44 documents across 6 formats

### GPL Compliance Statement

We acknowledge that Pandoc is licensed under GPL-2.0-or-later. We have:

- ✓ Used Pandoc's test documents (test data is allowed under GPL)
- ✓ Generated baseline outputs using Pandoc for comparison purposes
- ✓ NOT copied any Pandoc source code
- ✓ Implemented our extractors independently in Rust
- ✓ Used Pandoc only as a behavioral baseline for testing

Our Rust extractors are independently implemented and do not contain any GPL-licensed code from Pandoc.

### Verification

Test documents and baselines can be regenerated at any time using:

```bash
./generate_pandoc_baselines.sh
```

This script processes all test documents and generates fresh baselines using the installed version of Pandoc.

## docx-lite

DOCX XML parser vendored into `crates/xberg/src/extraction/docx/parser.rs`:

- **Source**: <https://github.com/v-lawyer/docx-lite>
- **License**: MIT OR Apache-2.0
- **Authors**: V-Lawyer Team
- **Copyright**: Copyright (c) 2024 V-Lawyer Team (`LICENSE-MIT` in the docx-lite 0.2.0 crates.io package, identical to `LICENSE-MIT` at <https://github.com/v-lawyer/docx-lite> commit `5913cb58c65d6a095e29e4cd92b3a05cd5903bd7`; the upstream repository has no release tags). Used under the MIT option; the license text is reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Version**: 0.2.0 (vendored with modifications)
- **Usage**: DOCX text extraction parser inlined into xberg core
- **Modifications**:
  - Fixed `Paragraph::to_text()` joining text runs without whitespace (#359)
  - Adapted to xberg's `quick-xml` v0.39 and `zip` v7.x APIs
  - Removed file-path based APIs (only bytes/reader needed)

---

## xberg-gliner

Span-mode GLiNER preprocessing, prompt construction, tensor construction, and decoding logic vendored into
`crates/xberg-gliner/`:

- **Source lineage**: `gline-rs` → private fork `xberg-io/gline-rs-fork` → `xberg-gliner`; Candle backend: `fastino/gliner2` reference ← `SemplificaAI/gliner2-rs` ← `anno` → `xberg-gliner`
- **License**: crate is Apache-2.0; upstream licenses and notices are in `crates/xberg-gliner/LICENSE` and `crates/xberg-gliner/NOTICE`
- **Usage**: Local named-entity recognition backends for xberg (ONNX Runtime span-mode and GLiNER2, and an optional Candle GLiNER2 backend behind the `candle` feature)

### gline-rs (ONNX span-mode backend)

- **Source**: <https://github.com/fbilhaut/gline-rs>; intermediate private fork <https://github.com/xberg-io/gline-rs-fork>
- **Base version**: UNRECORDED; the fork (`xberg-io/gline-rs-fork`) is private and was not accessible to verify. Upstream has eight tags: `v0.9.0` (2025-01-07) through `v0.9.4` (2025-03-30), `v1.0.0` (2025-06-22, commit `30c17183bc121d18b55c80e1ad9603225c9f4c47`), `v1.0.1` (2026-01-10, `6df04c1b233faea256c38501ad4ba06a9d68d96a`) and `v1.1.0` (2026-08-05, `f1f8923a7af972855909302a843063d958f69d95`). `src/` is identical between `v0.9.4` and `v1.0.0`; `v1.0.1` changes only `TextInput` derives, relation output and memory profiling; `v1.1.0` makes `GreedySearch::search` fallible (`Result<Vec<Span>>`) and adds `IndexError`. xberg-gliner's `greedy_search` is infallible, consistent with a base at or before `v1.0.1`, but the code was rewritten (at most 17% of normalised lines match any single upstream file, e.g. 16 of 135 in `decode.rs` against `src/text/span.rs`; the decision logic in `accept_span` is the same as upstream's `GreedySearch::accept`), so the base tag cannot be determined from the code. Earlier text citing "gline-rs 0.2.1" was wrong: no such release exists, and the earlier "v0.9.4 or later on the 1.0.x line" had no supporting evidence
- **License**: Apache-2.0
- **Author**: Frédérik Bilhaut (per upstream `Cargo.toml`); upstream ships no NOTICE file, and its `LICENSE.txt` (checked at `v1.1.0`/HEAD `f1f8923a7af972855909302a843063d958f69d95`) is the Apache-2.0 text with the unfilled appendix line `Copyright [yyyy] [name of copyright owner]`, identical to `crates/xberg-gliner/LICENSE`
- **Scope**: Span-mode text splitting, zero-shot prompt construction, Hugging Face tokenizer adapter, GLiNER tensor construction, logits decoding, and greedy overlap filtering (`src/*.rs` outside `src/candle/` and `src/v2/`)
- **Excluded**: ORP pipeline code, examples, CSV helpers, relation extraction, token-mode runtime, memory profiling, and generic pipeline abstractions
- **Modifications**:
  - Adapted crate identity to `xberg-gliner`
  - Removed ORP dependency and replaced the pipeline wrapper with direct `ort` session creation and synchronized inference
  - Span / byte-offset decode output (`Span`) and input validation caps
  - Integrated with xberg's model catalog/cache and `xberg-io/gliner-models` artifact layout
  - Added focused unit tests for validation, splitting, prompt construction, tensor shapes, decoding, and overlap filtering

### anno (Candle GLiNER2 backend)

- **Source**: <https://github.com/arclabs561/anno>, modules `backends::gliner2_fastino_candle` and `backends::gliner2_fastino` (decode)
- **License**: MIT OR Apache-2.0 (xberg-gliner uses the Apache-2.0 option)
- **Copyright**: "Copyright (c) 2025 Arc" (anno `LICENSE-MIT`)
- **Version**: the ported commit was not recorded; best evidence: `xberg-gliner`'s Candle backend landed in `ea06025c6e` (2026-07-17). anno's `gliner2_fastino` and `gliner2_fastino_candle` modules were added in a single commit `036e102a93` (2026-06-10, "anno: add gliner2-fastino + heuristic-fr backends, ...") and no commit touched those paths between that date and 2026-07-18. The newest anno tag at the port date was `v0.11.0` (`8df359f317`, 2026-06-11, 12 commits after `036e102a93`) and anno's HEAD on 2026-07-14 was `b6f2d5468c1798508274b5302b1f959bcd570193`, so the ported sources are those of `036e102a93`, present in `v0.11.0` onward. `LICENSE-MIT` is identical at `b6f2d54` and current HEAD (`852df845e26ec7215acccb5ac0413363d741d2fc`); `LICENSE-APACHE` is the unfilled Apache-2.0 template. The earlier "anno tags 0.12.0 to 0.14.0" note referred to later releases
- **Location**: `crates/xberg-gliner/src/candle/` (`mod.rs`, `encoder.rs`, `decode.rs`, `model.rs`, `pipeline.rs`, `heads/`, `lora.rs`, `streaming_load.rs`); doc comments say "Ported from `anno::backends::gliner2_fastino_candle`"
- **Upstream lineage**: anno's `gliner2_fastino` modules are marked "Adapted from SemplificaAI/gliner2-rs (Apache-2.0)" with the line "Original: Copyright 2026 Dario Finardi, Semplifica s.r.l." (headers of `processor.rs` lines 1-3, `sessions.rs` lines 4-6 and `pipeline.rs` lines 4-7 at anno `852df845e2`; the module doc of `gliner2_fastino/mod.rs` line 100 repeats "adapted from SemplificaAI/gliner2-rs (Apache-2.0)"); the Candle modules wrap `candle-transformers` DeBERTa-v2. The two names for the gliner2-rs holder are both authoritative for their date: gliner2-rs's current `NOTICE` and per-file headers (commit `7626d237d8c58f0fcfadd094e8360e7d5ae9c8ee`, 2026-08-29) read "Copyright 2026 Dario Finardi. Published by Jugaad s.r.l. — Apache-2.0", and its `RELEASE_NOTES.md` (lines 316-320) states "Every reference to Semplifica removed" and that the HuggingFace model ids moved from `SemplificaAI/...` to `jugaadsrl/...` "following the repositories transferred". So "Semplifica s.r.l." is the name in anno's headers, which pre-date or ignore the rename, and "Jugaad s.r.l." is the publisher named by gliner2-rs now; `Cargo.toml` lists `authors = ["Dario Finardi <df@dariofinardi.it>"]`. The same `NOTICE` also says "This is an independent Rust implementation, not a port of existing code", which is a statement about gliner2-rs against its own upstreams and does not alter anno's adaptation notice. The upstream gliner2-rs NOTICE is reproduced in `crates/xberg-gliner/NOTICE`
- **Reference model**: `fastino/gliner2` (Fastino; <https://huggingface.co/fastino>). Weights are not distributed with this crate
- **Modifications**:
  - Decode targets `crate::Span` (byte offsets) instead of anno's character offsets
  - Tokenization and prompt construction reuse the crate's own V2 schema-prompt encoder (`src/v2/`) instead of anno's separate processor
  - Streaming safetensors loader (`streaming_load.rs`) and `wasm32` support
  - PEFT LoRA adapter merge-at-load (`lora.rs`, native targets only)
  - anno's `classifier` head is not ported; scope is trimmed to entity extraction
  - Validation caps on inputs and tensor sizes

### GLiNER2 ONNX engine (`src/v2/`)

The GLiNER2 ONNX engine (`src/v2/*`, originally `src/v2_*.rs`, added in commit `ea06025c6e` on 2026-07-17, the same commit that added the Candle backend ported from anno) builds the `fastino/gliner2` schema prompt (`[P]`, `[E]`, `[SEP_TEXT]` tokens). Comparison against anno `gliner2_fastino*` (commit `852df845e2`) and gliner2-rs (`7626d237d8`): no line-level derivation was found. For each `src/v2/*.rs` file the best single-file match is 4 of 77 (`decode.rs`), 3 of 84 (`preprocess.rs`), 3 of 27 (`splitter.rs`), 3 of 30 (`tensor.rs`) normalised lines, all of them one-line constants or the regex below. The structure differs: xberg has `encode_v2`/`build_schema_tokens` over a `PretokenizingTokenizer` trait, a dense `build_span_idx` with a size cap, and a single-task entity-only prompt, while anno and gliner2-rs use a multi-task `SchemaTransformer`/`SchemaTask`/`ProcessedRecord` with `[C]`/`[L]`/`[R]`/`[DESCRIPTION]` tokens. The one non-trivial shared element is the word-splitter pattern (URLs, emails, `@mentions`, `\w+(?:[-_]\w+)*`, `\S`), which appears identically in anno, in gliner2-rs and in the Python reference `gliner2/processing/word_splitter.py` (`WhitespaceTokenSplitter._PATTERN`) of <https://github.com/fastino-ai/GLiNER2> (Apache-2.0, HEAD `ed9963f2463609634e42116ba2b137d2bbfede04`; its `LICENSE` is the unfilled template). xberg's `V2Splitter` lowercases the text before matching, whereas the Python reference and gliner2-rs match the original text and lowercase only the token value (anno also matches the original text), so the splitter was not copied from any of them as written. Conclusion from the evidence: no line-level or structural derivation from anno or gliner2-rs was found; `src/v2/*` implements the public `fastino/gliner2` prompt format and reuses that reference's splitter pattern. UNRECORDED: because the files were written in the same commit as the anno port, derivation at the design level (prompt layout, token names, span-index layout) cannot be excluded by history alone, and whether the pattern and prompt layout, which are functional elements of the model format, need an Apache-2.0 attribution to fastino-ai/GLiNER2 is left for legal review.

## GLiNER Models

GLiNER ONNX runtime artifacts consumed by xberg are exported and governed through `xberg-io/gliner-models`.
Their source model lineage is the `gliner-community` Hugging Face organization.

- **Upstream source**: <https://huggingface.co/gliner-community>
- **Runtime artifact repository**: <https://huggingface.co/xberg-io/gliner-models>
- **Default license**: Apache-2.0 unless a model manifest declares otherwise
- **Original GLiNER authors**: Urchade Zaratiana, Nadi Tomeh, Pierre Holat, Thierry Charnois
- **Usage**: ONNX weights, tokenizer files, manifests, checksums, and notices for xberg NER
- **Policy**: Model weights are not committed to this repository; verified artifacts are downloaded into the xberg model cache

---

## hwpers

Vendored HWP text extraction code from the hwpers crate:

- **Source**: <https://github.com/Indosaram/hwpers>
- **License**: MIT OR Apache-2.0
- **Authors**: HWP Parser Contributors
- **Copyright**: Copyright (c) 2024 HWP Parser Contributors (`LICENSE` at <https://github.com/Indosaram/hwpers> tag `v0.5.0`, identical to the file at commit `4097f71829b3726ebccc33bbc55decdc1a8a8e34`). Used under the MIT option; the license text is reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Vendored Version**: 0.5.0
- **Location**: `crates/xberg/src/extraction/hwp/`
- **Purpose**: Text extraction from Korean Hangul Word Processor (.hwp) files
- **Scope**: Minimal subset — CFB reader, binary record parser, text extraction only
- **Excluded**: HWPX (XML/ZIP), writer, renderer, crypto, preview modules

---

## paddle-ocr-rs

Vendored source code from the paddle-ocr-rs crate for PaddleOCR via ONNX Runtime integration:

- **Source**: <https://github.com/mg-chao/paddle-ocr-rs>
- **Original License**: Apache-2.0
- **Author**: mg-chao (<chao@mgchao.top>)
- **Vendored Version**: 0.6.1 (crates.io, published 2025-08-31, `license = "Apache-2.0"`, `authors = ["mg-chao <chao@mgchao.top>"]`; the upstream repository has no release tags)
- **Copyright**: the `LICENSE` in the paddle-ocr-rs 0.6.1 package is the Apache-2.0 text with the appendix line `Copyright 2024 paddle-ocr-rust Contributors`; the current upstream `LICENSE` (<https://github.com/mg-chao/paddle-ocr-rs> commit `8d4a04c413074fb1457b8e506d4cc005cc6d32c3`) reads `Copyright 2026 rapid-ocr-rust Contributors`. Upstream ships no NOTICE file. The 0.6.1 text is preserved verbatim at `crates/xberg-paddle-ocr/LICENSE-paddle-ocr-rs`
- **Location**: `crates/xberg-paddle-ocr/`
- **Purpose**: Text detection and recognition using PaddlePaddle's OCR models via ONNX Runtime

### Vendored Files

The following source files were vendored from paddle-ocr-rs:

- `ocr_lite.rs` - Core OCR pipeline and high-level API
- `db_net.rs` - DBNet text detection network
- `crnn_net.rs` - CRNN text recognition network
- `angle_net.rs` - Text angle detection network
- `base_net.rs` - Base network trait
- `ocr_utils.rs` - Image preprocessing utilities
- `ocr_result.rs` - Result type definitions
- `scale_param.rs` - Scaling parameter calculations
- `ocr_error.rs` - Error type definitions

### Modifications

The vendored code has been modified for Xberg integration:

- Updated to Rust 2024 edition
- Aligned with Xberg workspace dependencies
- Crate packaged as `xberg-paddle-ocr` with Xberg's MIT-licensed additions (see Licensing below; the derived files stay Apache-2.0)

### Licensing

The files derived from paddle-ocr-rs remain under Apache-2.0, with the Apache-2.0 text, the upstream copyright and this statement of changes (the Modifications list above) retained. Xberg's own modifications and additions are MIT.

The crate's `LICENSE` contains the MIT text with "Copyright (c) 2025-2026 Kreuzberg, Inc." for Xberg's own code, and `Cargo.toml` declares `license = "MIT"` for it. The upstream Apache-2.0 text and copyright line are carried beside it in `LICENSE-paddle-ocr-rs` (matched by the crate's `include = ["/LICENSE*"]`) and apply to the derived files listed above. Upstream ships no NOTICE file, so there is no NOTICE content to retain.

---

## fastembed-rs

Text embedding inference pipeline vendored into `crates/xberg/src/embeddings/engine.rs`:

- **Source**: <https://github.com/Anush008/fastembed-rs>
- **License**: Apache-2.0
- **Author**: Anush008 and contributors
- **Vendored Version**: not recorded in the file. Best evidence: `fastembed` 5.12.1 was the locked dependency (`Cargo.lock`, `fastembed = "5.12"`) in the commit immediately before `2c189df8ae` ("chore: vendor fastembed", 2026-03-15) removed it. The earlier "0.2.x" was wrong. The current `engine.rs` shares almost no lines with fastembed 5.12.1 `src/` (at most 16 of 256 normalised lines with any single upstream file), so it has been largely rewritten and the extent of derivation cannot be verified by diff
- **Copyright**: the `LICENSE` of fastembed 5.12.1 is the unfilled Apache-2.0 template (`Copyright [yyyy] [name of copyright owner]`); upstream's current `LICENSE` (<https://github.com/Anush008/fastembed-rs> commit `1cde83b4426c1afe9ae4af22a02833e7ec723184`) reads `Copyright 2026 Anush008`. The crates.io `authors` list is Anush008, Josh Niemelä, GrisiaEvy, George MacKerron, Timon Vonk, Luya Wang, Tri, Denny Wong, Alex Rozgo
- **Header status**: the vendored `engine.rs` does not retain an upstream copyright header. Its header names fastembed-rs and its copyright holder and carries the statement of changes, so both ship inside the published `xberg` crate (this file does not); the Apache-2.0 text is reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Location**: `crates/xberg/src/embeddings/engine.rs`
- **Purpose**: ONNX-based text embedding inference with thread-safe concurrent embedding generation

### Modifications

The vendored code has been modified from the original fastembed-rs:

- Changed `embed()` method signature from `&mut self` to `&self` for thread-safe concurrent inference without mutex contention
- Adapted to Xberg's ONNX Runtime integration and error handling
- Integrated with Xberg's embedding configuration and model management

### License Compatibility

The upstream code is Apache-2.0. Redistribution in Xberg requires the Apache-2.0 text, upstream copyright and attribution notices and a statement of changes (above) to accompany the derived file. The engine header names the upstream project and states the changes, so the statement of changes travels with the published crate; the notice is also recorded in this entry and in `crates/xberg/LICENSE-THIRD-PARTY`. The derived file stays Apache-2.0.

---

## numbers-parser Test Fixtures

Test documents derived from the `numbers-parser` test suite:

- **Source**: <https://github.com/masaccio/numbers-parser>
- **License**: MIT
- **Author**: Jon Connell (masaccio)
- **Usage**: Test documents and reference baselines only (no code copied)
- **Modifications**: Fixtures downloaded directly for integration testing.
- **Location**: `test_documents/iwork/`

---

---

## yake-rust

YAKE keyword extraction algorithm vendored into xberg:

- **Source**: <https://github.com/quesurifn/yake-rust>
- **License**: MIT
- **Authors**: Kyle Fahey, Anton Vikstrom, Igor Strebz
- **Copyright**: Copyright (c) 2022 Kyle Fahey (`LICENSE` at <https://github.com/quesurifn/yake-rust> commit `af346ef7c7336e899c6fec43e6448c34c928bd2f`; the repository has no release tags and the 1.0.3 crates.io package ships no license file, so the repository file is the source). Reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Vendored Version**: 1.0.3
- **Location**: `crates/xberg/src/keywords/yake/`
- **Purpose**: YAKE (Yet Another Keyword Extractor) statistical keyword extraction

### Modifications

- Replaced segtok dependency with custom memchr-based sentence splitter (fixes #676 BacktrackLimitExceeded on large files)
- Integrated with xberg's stopwords module (64 languages vs original 34)
- Replaced hashbrown with ahash, inlined streaming-stats and levenshtein
- Optimized punctuation checks with byte lookup tables
- Removed itertools dependency (manual dedup)

### Licensing

The files derived from yake-rust stay MIT; the upstream copyright and permission notice are reproduced verbatim in `crates/xberg/LICENSE-THIRD-PARTY`.

## text-splitter (inlined)

The chunking submodule `crates/xberg/src/chunking/text_splitter/` is a trimmed inline copy of [text-splitter](https://github.com/benbrandt/text-splitter) v0.30.1 by Benjamin Brandt. We inlined it because upstream pins `tokenizers = "0.22"`, which conflicts with xberg's direct `tokenizers 0.23` dependency and pulls a duplicate copy of `tokenizers` into the build graph (breaking the `Tokenizer: ChunkSizer` bound in `chunking::core`).

- **Source**: <https://github.com/benbrandt/text-splitter> @ v0.30.1
- **License**: MIT
- **Copyright**: © 2023 Benjamin Brandt <benjamin.j.brandt@gmail.com>. Verified against <https://github.com/benbrandt/text-splitter> tag `v0.30.1` (commit `b181fc924245d7b832e40d955f2b62676f4a4db2`): its `LICENSE.txt` is byte-identical to the text below and reads `Copyright (c) 2023 Benjamin Brandt`; `Cargo.toml` at the tag gives `authors = ["Ben Brandt <benjamin.j.brandt@gmail.com>"]`, `license = "MIT"`. The text is also reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Location**: `crates/xberg/src/chunking/text_splitter/`

### Modifications

- Dropped the `code` (tree-sitter) splitter — xberg has its own tree-sitter integration and does not use the upstream code splitter.
- Dropped the `tiktoken-rs` sizer — unused.
- Rebuilt against `tokenizers 0.23`.
- Renamed feature gate `tokenizers` → `chunking-tokenizers`; the `markdown` splitter is always available because `pulldown-cmark` is already a non-optional xberg dependency.
- Tightened visibility on internal types to `pub(crate)`.
- Path rewiring: upstream `crate::*` paths inside the inlined module rewritten relative to the new submodule root.

### Licensing

The inlined files stay MIT. The full upstream license text is reproduced below:

```text
MIT License

Copyright (c) 2023 Benjamin Brandt

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Test Documents from text-splitter

The following test inputs were copied from the text-splitter repository to `/test_documents/text_splitter/`:

- `text/romeo_and_juliet.txt` — Shakespeare, public domain (Project Gutenberg)
- `text/room_with_a_view.txt` — E. M. Forster, public domain (Project Gutenberg)
- `markdown/commonmark_spec.md` — CommonMark spec, CC-BY-SA-4.0
- `markdown/github_flavored.md` — GitHub Flavored Markdown spec, CC-BY-4.0

---

## hOCR parsing (moved from html-to-markdown)

`crates/xberg/src/ocr/hocr_parser.rs` and `crates/xberg/src/table_core.rs` contain hOCR parsing code moved from the `hocr` module of Xberg's own [html-to-markdown](https://github.com/Goldziher/html-to-markdown) project (`html-to-markdown-rs`, removed upstream in v3). It is Xberg's own code, MIT-licensed under `LICENSE`, and carries no third-party licence entry.

---

## sysinfo (vendored patch)

Patched copy of `sysinfo` used through a Cargo path override:

- **Source**: <https://github.com/GuillaumeGomez/sysinfo> v0.39.6, commit `1bc6ed4f7d0dd03606de572a1531a165438b63ec`
- **License**: MIT, Copyright (c) 2015 Guillaume Gomez (`vendored/sysinfo/LICENSE`, byte-identical to `LICENSE` in the upstream repository; commit `1bc6ed4f7d0dd03606de572a1531a165438b63ec` is "Update crate version to `0.39.6`" dated 2026-07-09, tag `v0.39.6`, `Cargo.toml` version 0.39.6; verified via the GitHub API)
- **Location**: `vendored/sysinfo/`
- **Modifications**: iOS network backport and `try_update` for two atomic updates; details, patch checksum and removal condition in `vendored/sysinfo/XBERG-PATCH.md`

---

## libheif-rs

Safe Rust bindings around `libheif-sys` for decoding HEIF / HEIC / AVIF
containers, vendored as the `xberg-libheif` crate:

- **Source**: <https://github.com/Cykooz/libheif-rs>
- **License**: MIT
- **Author(s)**: Kirill Kuzminykh (Cykooz) and contributors
- **Copyright**: Copyright (c) 2019 Kirill Kuzminykh (`LICENSE` at <https://github.com/Cykooz/libheif-rs> tag `v2.7.0`, unchanged at current HEAD `004a2a2fc16010b3e206c1c2d55f1bc175800219`). The upstream text is preserved at `crates/xberg-libheif/LICENSE-libheif-rs`
- **Vendored Version**: 2.7.0
- **Location**: `crates/xberg-libheif/`
- **Purpose**: Decode HEIF-family containers (HEIC, HEIF, AVIF, HEICS, AVCS) to
  interleaved RGBA pixels and expose EXIF / XMP metadata blocks for xberg's
  image-extraction and OCR pipeline.

### Vendored Files / Scope

- Full upstream `src/` tree (LibHeif, HeifContext, ImageHandle, Image, decoder,
  encoder, color profile, regions, metadata, reader, security limits, track,
  utils, and the optional `image`-crate integration module).
- Vendored verbatim from upstream v2.7.0 so the public API mirrors `libheif-rs`
  exactly; we continue to depend on upstream `libheif-sys` from crates.io for
  the underlying C bindings.

### Modifications

- Workspace dependency alignment (`libc`, `image` pinned via the workspace).
- Rust 2024 edition (upstream is 2021).
- Replaced upstream `include_str!("../README.md")` doc shim with an inline
  module-level vendoring header pointing at this file.
- Workspace clippy / rust lints applied via `[lints] workspace = true`.

### Licensing

The vendored files stay MIT. The upstream MIT license text is preserved
verbatim at `crates/xberg-libheif/LICENSE-libheif-rs`. The crate's own
`LICENSE` is a different file: it carries only "Copyright (c) 2025-2026
Kreuzberg, Inc." for Xberg's own code.

### System Library Requirement

`libheif-rs` is a safe wrapper around the C library `libheif`, which in turn
requires `libde265` (HEVC) and `libaom` (AV1). These must be available at build
and runtime. `libheif` is LGPL-licensed and is linked dynamically everywhere —
the release CLI archives ship the shared libraries (on Windows the ZIP carries
`heif.dll`, `libde265.dll` and `aom.dll` plus their source offer, while the
`-no-gpl-lgpl` Windows ZIP compiles the codecs out and carries none of them)
— except in the macOS PHP extension, which links `libheif`, `libde265` and
`libaom` statically because PIE installs a single file (see
`THIRD_PARTY_LICENSES.md`). Pixel
decoding is therefore native-only — the `heic` feature is excluded from
xberg's `wasm-target` and `android-target` aggregate features. EXIF
extraction via `nom-exif` is pure Rust and works on every target.

---

## tesseract-rs

Rust bindings and build scripts for Tesseract and Leptonica, forked as the `xberg-tesseract` crate:

- **Source**: <https://github.com/cafercangundogdu/tesseract-rs>
- **License**: MIT
- **Author**: Cafer Can Gündoğdu
- **Copyright**: Copyright (c) 2024 Cafer Can Gündoğdu (`LICENSE`, identical at tags `v0.1.20` and `v0.4.0`; HEAD `e56a268e17e1a499189d31c8e6af72420c5132fd`). The upstream text is preserved at `crates/xberg-tesseract/LICENSE-tesseract-rs`; the crate's own `LICENSE` carries only "Copyright (c) 2025-2026 Kreuzberg, Inc." and is not the upstream text
- **Forked Version**: not recorded. Upstream releases on crates.io include 0.1.13 to 0.1.20 (2024-10 to 2025-07-27), 0.2.0 (2026-03-23), 0.3.0, 0.3.1 and 0.4.0 (2026-07); the fork's base tag cannot be determined from the crate's files
- **Location**: `crates/xberg-tesseract/`
- **Modifications**: as listed in `crates/xberg-tesseract/README.md` (C++17 for Tesseract 5.5.1, cross-compilation CXX detection, architecture validation of cached libraries, MSVC static linking, `OUT_DIR`-based build cache, MinGW support)
- **Related**: the bundled Tesseract and Leptonica license texts are in `crates/xberg-tesseract/licenses/`

---

## pdfium-render

High-level idiomatic Rust wrapper around Pdfium, forked and vendored for xberg:

- **Source**: <https://github.com/ajrcarey/pdfium-render>
- **License**: MIT OR Apache-2.0 upstream (`license` in `Cargo.toml`); the fork takes the MIT option and is redistributed under MIT.
- **Author**: Alastair Carey (<alastair@alastaircarey.com>), per upstream `Cargo.toml` `authors`.
- **Copyright**: no copyright line exists upstream. The repository's only license file is `LICENSE.md` (checked at HEAD `79164d0d2837a2d6edef8f5a3487cc1f8017b9cd` and in the 0.8.37 crates.io package; the repository has never had a `LICENSE-MIT` file), a pointer reading "Licensed under either of Apache License, Version 2.0 (LICENSE-APACHE ...) or MIT License (LICENSE-MIT ...) at your option"; the `LICENSE-APACHE` and `LICENSE-MIT` files it names are absent from the HEAD tree. There is therefore no upstream copyright notice or license text to reproduce. An earlier "Copyright (c) 2022-2026" line here had no source in upstream and was removed
- **Forked Version**: upstream 0.9.0 development line at or after commit 0dbad90 (2025-12-13), with bindings generated against pdfium 7678 (`src/bindings/version.rs`,
  `src/bindgen/pdfium_7678.rs`). The runtime binary staged by `scripts/download_pdfium_runtime.sh`, the
  two system installers and the Docker images is the same 7678, because the bindings are the ABI
  contract and only one binding set exists in the tree
- **Location**: `crates/xberg-pdfium-render/`
- **Purpose**: PDF structure-tree and content-mark access, text extraction and rendering via
  Google's Pdfium library

### Modifications

- Updated to Rust 2024 edition and workspace conventions
- Added musl static linking support in `build.rs`
- Removed examples and test data (kept in the upstream repo)
- **Added** struct-tree and content-mark wrappers (~1,036 lines across `struct_element.rs`,
  `struct_tree.rs`, `content_mark.rs`, `content_marks.rs`) that upstream does not provide —
  upstream's `PdfPageStructureTree` exposes no public constructor and has no `PdfStructElement`
  type at all

### License Compatibility

The upstream MIT OR Apache-2.0 dual license permits redistribution under MIT alone. The upstream
author is recorded in this file, which is distributed with the source. Because upstream ships no
copyright line or MIT text, none can be reproduced; `crates/xberg-pdfium-render/LICENSE` carries only
Xberg's copyright; the author attribution is recorded here.

---

## PDFOxide

Pure-Rust PDF parsing, extraction and rendering — xberg's default PDF engine, forked and vendored:

- **Source**: <https://github.com/yfedoseev/pdf_oxide>
- **License**: MIT OR Apache-2.0 upstream; this fork elects the MIT arm and is redistributed
  under MIT, retaining the upstream copyright notice below.
- **Author**: Yury Fedoseev, Copyright (c) 2025-present
- **Forked Version**: forked pre-1.0, published independently as `xberg-pdf-oxide` 1.0.1 on
  crates.io, then vendored here from that repository's `development` branch at commit
  `196ec71a1cfd6e818e1498053584ff93c794bde1`. No git history was carried across, so that SHA
  is the provenance link; the source repository has since been retired.
- **Location**: `crates/xberg-native-pdf/` (crate `xberg-native-pdf`, lib `xberg_native_pdf`)
- **Purpose**: text extraction, reading order, tables, forms, annotations, XFA, encryption and
  page rendering — everything behind `PdfBackend::Native`

### Modifications

- Renamed to `xberg-native-pdf` on vendoring. Upstream's trademark policy permits describing
  this crate as a fork of PDFOxide but not using upstream's names as the name of a
  distribution, so the rename is a compliance improvement as well as a naming one. This file,
  `crates/xberg-native-pdf/ATTRIBUTIONS.md`, and `crates/xberg-native-pdf/CHANGELOG.md` are the
  places that document the upstream project by design. Beyond those, the identifier
  `pdf_oxide` (or a variant spelling) survives, deliberately, in a small number of live,
  commented, load-bearing spots — confirm with `grep -rlniE 'pdf[_-]?oxide' crates/`:
  - `crates/xberg/src/core/config/pdf.rs` (`PdfBackend`'s rejection test): the old spelling
    must appear verbatim so the test can assert it is rejected, not silently aliased.
  - `crates/xberg-native-pdf/src/document.rs` (a `/Producer` allowlist entry): matches PDFs
    already written by the `pdf_oxide`/`xberg-pdf-oxide` lineages, which are still arriving in
    the wild, so recognizing them as trusted authoring tools is intentional.
  - `crates/xberg-native-pdf/src/decoders/flate.rs` (an environment-variable
    back-compatibility chain): `PDF_OXIDE_MAX_DECOMPRESS_MB` and
    `XBERG_PDF_OXIDE_MAX_DECOMPRESS_MB` keep older deployments' env vars working.
  - `crates/xberg-native-pdf/tests/test_signed_pdf_opens_and_renders.rs`: a verbatim quotation
    of a third-party signing tool's own error message (`PdfOxide.Exceptions.SignatureException`),
    not a reference to this codebase.

  The same grep also turns up `crates/xberg-php/` and `crates/xberg-node/` (alef-generated
  binding sources) and every `docs-site/src/content/docs/reference/api-*.md` /
  `cli.md`/`configuration.md`/`types.md` page: these are mechanically generated from the Rust
  doc comments above and were stale (pre-dating the `xberg-native-pdf` rename in the comments
  they mirror) at the time of writing; they carry no independent attribution and refresh on the
  next `alef generate`, not by hand-editing. Historical entries in the root `CHANGELOG.md` (and
  its `docs-site/src/content/docs/changelog.md` mirror) also name `pdf_oxide`/`xberg-pdf-oxide`;
  those describe releases from when the crate really was named that, and changelogs are an
  append-only historical record, so they are left as written rather than retroactively edited.
- Stripped upstream's language bindings, CLI, MCP server, and its OCR, ML, digital-signature,
  WASM, barcode, HTML/CSS-to-PDF and PDF/A subsystems
- Replaced `ttf-parser` with the fontations/`skrifa` stack
- Migrated diagnostics from the `log` facade to `tracing`; the target root is exported as
  `xberg_native_pdf::LOG_TARGET_ROOT` so consumers filter on a value rather than a literal
- Rust 2024 edition and workspace conventions
- Per-release detail in `crates/xberg-native-pdf/CHANGELOG.md`

### Vendored and bundled third-party code

`crates/xberg-native-pdf/ATTRIBUTIONS.md` is the authoritative record and ships with the crate.
It covers `src/vendor/fontdb/` (derived from fontdb 0.24.0, Yevhenii Reizner, MIT) and the fonts
embedded via `include_bytes!` under `src/fonts/assets/`: DejaVuSans and DejaVuSans-Bold
(Bitstream Vera / DejaVu licence), DroidSansFallbackFull.ttf (Apache-2.0) and
NotoEmoji-Regular.ttf (SIL OFL 1.1), the latter two behind the `cjk-form-fonts` feature. The
DejaVu licence requires renaming modified fonts; these are unmodified and must stay so.

### License Compatibility

Electing the MIT arm of an MIT-OR-Apache-2.0 dual offer is permitted by both licences. The
upstream copyright notice is preserved verbatim at `crates/xberg-native-pdf/LICENSE`, which is
distributed with the source. No endorsement by or affiliation with the upstream project is
implied.

---

## libwpd + librevenge

Native WordPerfect document libraries built from source and statically linked
into the `xberg-libwpd` crate:

- **Source**: <https://libwpd.sourceforge.net/>
- **License**: MPL-2.0 OR LGPL-2.1+ (xberg builds under the MPL-2.0 arm)
- **Authors**: Document Liberation Project (LibreOffice/AbiWord ecosystem) contributors
- **Versions**: librevenge 0.0.6, libwpd 0.10.3
- **Location**: vendored as committed `.tar.gz` archives under `crates/xberg-libwpd/vendor/`, decompressed and compiled by `crates/xberg-libwpd/build.rs` (checksum-verified at extract time); consumed via `crates/xberg-libwpd/src/shim.cpp`
- **Purpose**: WordPerfect (.wpd) document extraction
- **Modifications**: none; both libraries are built from unmodified upstream source

### License Compatibility

See `THIRD_PARTY_LICENSES.md` for the full MPL-2.0 vs LGPL-2.1+ linking analysis. In short: xberg statically links under the MPL-2.0 arm (file-level copyleft, static linking explicitly permitted), so no copyleft obligation reaches xberg's MIT-licensed core or language bindings.

---

## Boost C++ Libraries (build-time, xberg-libwpd)

A header-only subset of Boost, used at build time by the `xberg-libwpd` C++ shim:

- **Source**: <https://www.boost.org/>
- **License**: BSL-1.0 (Boost Software License 1.0)
- **Author**: Boost contributors
- **Location**: `bcp`-extracted subset vendored as `crates/xberg-libwpd/vendor/boost-subset.tar.gz`, decompressed by `crates/xberg-libwpd/build.rs`
- **Purpose**: header-only dependency of libwpd/librevenge (build-time only; no Boost code is redistributed in xberg binaries)
- **Modifications**: none; subset extracted verbatim with `bcp`

---

## jhqxxx/aha

Rust-native VLM-OCR backends vendored into `crates/xberg-candle-ocr/`:

- **Source**: <https://github.com/jhqxxx/aha>
- **License**: Apache-2.0
- **Author**: jhqxxx
- **Copyright**: upstream's `LICENSE` is the Apache-2.0 text whose appendix line is the unfilled template `Copyright [jhq] [name of copyright owner]`; there is no other copyright line and no NOTICE file in the repository at the vendored commit or at HEAD. Preserved verbatim at `crates/xberg-candle-ocr/LICENSE-aha`
- **Vendored Version**: `e29ddc589d089042afd66ab8ea76409d8d33f701` (committed 2026-06-07; recorded as HEAD at the time of vendoring on 2026-06-17, and still upstream HEAD on 2026-11-22)
- **Location**: `crates/xberg-candle-ocr/src/vendor/aha/` (shared infra), `crates/xberg-candle-ocr/src/models/deepseek_ocr/` (DeepSeek-OCR + Qwen2 decoder), `crates/xberg-candle-ocr/src/models/paddleocr_vl/` (PaddleOCR-VL 1.5 upgrade)
- **Purpose**: Rust-native VLM-OCR backends (PaddleOCR-VL 1.5, DeepSeek-OCR) including their Qwen2 decoder dependency and shared SigLIP/CLIP/MRoPE infrastructure subsets.

### Vendored Files

| File (under `crates/xberg-candle-ocr/src/`) | Upstream (jhqxxx/aha @ `e29ddc589d`) | Normalised-line overlap | Notes |
| ---- | ------ | ---- | ----- |
| `vendor/aha/image.rs` | `src/utils/img_utils.rs`, `src/utils/video_utils.rs`, `src/utils/interpolate.rs` (subset) | 31% with `img_utils.rs`, 25% with `interpolate.rs` | Image loading, patch-grid helpers, normalisation, smart-resize. HTTP/HTTPS image URLs are not supported |
| `vendor/aha/modules.rs` | `src/models/common/modules.rs` | 62% | Attention/MLP primitives (`NaiveAttention`, `QKVCatAttention`, `GateUpDownMLP`, `TwoLinearMLP`, layer-norm/conv helpers) |
| `vendor/aha/qwen2.rs` | `src/models/qwen2/mod.rs` | 69% | `forward_no_cache` prefill path for DeepSeek-OCR |
| `vendor/aha/rope.rs` | `src/position_embed/rope.rs`, `src/utils/tensor_utils.rs` (`split_tensor`, inlined) | 56% | 1D, 2D and M-RoPE position embeddings |
| `vendor/aha/mod.rs` | `src/models/common/mod.rs` (`InferenceModel` / `MultiModalData` surface) | 52% | Subset tailored to xberg; `forward_step_with_position_ids` is an xberg addition |
| `models/deepseek_ocr/config.rs` | `src/models/deepseek_ocr/config.rs` | 95% | |
| `models/deepseek_ocr/model.rs` | `src/models/deepseek_ocr/model.rs` | 72% | Header: "Vendored from jhqxxx/aha (Apache-2.0)" |
| `models/deepseek_ocr/processor.rs` | `src/models/deepseek_ocr/processor.rs` | 56% (25 lines) | |
| `models/deepseek_ocr/utils.rs` | no matching upstream file (best 3%) | 3% | Header says "Vendored from jhqxxx/aha (Apache-2.0)"; tensor helpers (interpolation, top-k, masked scatter) are not recognisable as copies of any aha file at line level |
| `models/deepseek_ocr/{engine,mod}.rs` | none (aha uses `deepseek_ocr/generate.rs`; best 4%) | 4% | xberg generation-loop integration; not derived at line level |
| `models/paddleocr_vl/config.rs` | `src/models/paddleocr_vl/config.rs` | 98% | Header: "Adapted from aha's paddleocr_vl module" |
| `models/paddleocr_vl/model.rs` | `src/models/paddleocr_vl/model.rs` | 27% | Substantially rewritten (see Modifications) |
| `models/paddleocr_vl/processor.rs` | `src/models/paddleocr_vl/processor.rs` | 30% | |
| `models/paddleocr_vl/{engine,mod}.rs` | none (best 2%) | 2% | xberg generation loop (see Modifications); not derived at line level |

The overlap column is the share of a local file's non-comment lines (longer than 12 characters, whitespace-normalised) that also appear verbatim in the named upstream file. It is a coarse lower bound: reformatting and the `anyhow` to `CandleOcrError` rewrite lower it, so a low figure does not prove independence and a file with a vendoring header should still be treated as derived. The mapping was produced by this comparison against the upstream tree at the vendored commit, not from a recorded diff.

### Modifications

- `anyhow::Result<T>` → `Result<T, CandleOcrError>` throughout
- `assert!`/`assert_eq!`/`panic!`/`unwrap()` on user-controlled values → typed `CandleOcrError` returns
- Candle 0.9.2 → Candle 0.10 API migration
- Generation loop routed through `xberg_candle_ocr::models::glm_ocr::mtp::generate_mrope` (with a new `forward_step_with_position_ids` trait extension per A3) — NOT aha's `common/generate.rs`
- rustdoc on every `pub` item
- Dead code blocks dropped (specific examples: aha `modules.rs:214-230, 265-266`)
- `paddleocr_vl::model::get_rope_index`: vision-block position ids restored to aha's (t, h, w) mrope layout after the vendoring rewrite transposed the h index and dropped the t row (crashed on non-square grids); images always carry a single frame so the t row is constant
- `paddleocr_vl::model::forward`: restored aha's seqlen_offset-based position continuation for cache_position-less decode steps (dropped during vendoring, failing every decode step with a dtype mismatch)
- `paddleocr_vl::engine::generate`: rewritten from a full-sequence re-feed (which double-appended the KV cache each step and read the argmax at the wrong rank) to aha's prefill + cached single-token step loop
- `paddleocr_vl::model::Ernie4_5Model::forward`: restored aha's causal attention mask for the multi-token prefill (dropped during vendoring; bidirectional prefill degenerated generation into single-token repetition)

### License Compatibility

The vendored files stay Apache-2.0. Each derived file carries a "Vendored from jhqxxx/aha (Apache-2.0)" or "Adapted from aha" header and a notice that Xberg modified it, and the changes are listed under Modifications above. The crate ships the upstream Apache-2.0 text as `LICENSE-aha` (upstream has no NOTICE file and no filled-in copyright line, so there is nothing further to reproduce).

**Last Updated**: 2026-11-22

---

## candle (huggingface/candle)

Code derived from the Hugging Face Candle project (<https://github.com/huggingface/candle>, `MIT OR Apache-2.0`), used under the Apache-2.0 option. Copyright holder line: upstream has none. `LICENSE-APACHE` is the Apache-2.0 text with the unfilled appendix line `Copyright [yyyy] [name of copyright owner]`, `LICENSE-MIT` begins at "Permission is hereby granted" with no copyright line, and there is no NOTICE file (checked at tag `0.10.2`, commit `7c7a8c570e6a16ea24cc30a7501c8ffbbdb51680`, 2026-04-01, identical at HEAD `1a49dc27c1da7671e0ee12279ddf4ba619412856`). The Apache-2.0 text is preserved at `crates/xberg-candle-ocr/LICENSE-candle-APACHE`.

- `crates/xberg-candle-ocr/src/models/image_processor.rs`: ported from `candle-examples/examples/trocr/image_processor.rs`. That upstream file was last changed in commit `6056fd5c90` (2024-07-23, "onnx: fix pad, unsqueeze (#2317)") and is blob `3571d6d3c409e0343674c558e0af4abb2cbf2bfb` at `0.10.2`, so every candle release from that commit through 0.10.2 carries the same content; the exact release ported is not recorded. Modifications: bytes-in API instead of file paths, `CandleOcrError` return type, decode-size budget (`DEFAULT_IMAGE_DECODE_BUDGET`), excluded on `wasm32`. Only 2% of the local file's normalised non-comment lines (5 of 202) appear verbatim in the upstream file: the module was largely rewritten around the ported `ViTImageProcessor` / `ProcessorConfig` shape, so the header's "Ported from" claim is the only evidence of lineage.
- `crates/xberg-candle-ocr/src/models/glm_ocr/decoder.rs`: vendored from `candle-transformers` 0.10.2, `src/models/glm4.rs` (blob `969325f2c9ec342336ccc2703141cd465e19c552` at `0.10.2`; that file was last changed in commit `317a3aecce`, 2025-07-07, "Support new arch of GLM4 models (#2991)", so any release from that commit on is identical). The module header's "commit hash TBD" is superseded by this entry. Modifications: added `forward_embeds()` (embedding-level entry that skips token lookup), `embed_tokens()` and `clear_kv_cache()`; config reads nested `text_config.rope_parameters` (`rope_theta`, `mrope_section`). Weight quantization, chat templates and tokenization are not vendored. Only 5% of the local file's normalised non-comment lines (20 of 389) appear verbatim in upstream `glm4.rs` (the local `DecoderConfig`, M-RoPE and decoder are substantially restructured); treat the file as derived on the strength of its header. The vision tower (`glm_ocr/vision.rs`) follows the Hugging Face `transformers` `Glm4vVisionModel` modeling code (Apache-2.0) and is not copied from Candle.

---

## Docling reading order

Rust port of the Docling reading-order predictor in `crates/xberg/src/extractors/pdf/reading_order.rs`:

- **Source**: `BoundingBox::is_strictly_above` and `overlaps_horizontally` port `docling_core/types/doc/base.py` in <https://github.com/docling-project/docling-core> (formerly DS4SD/docling-core). `ReadingOrderPredictor._predict_page`, `_init_ud_maps`, `_has_sequence_interruption`, `_depth_first_search_upwards`/`_downwards`, `_do_horizontal_dilation` and `PageElement.__lt__` port `docling_ibm_models/reading_order/reading_order_rb.py` in <https://github.com/docling-project/docling-ibm-models> (formerly DS4SD/docling-ibm-models). Both confirmed by reading the upstream files. The predictor has since moved: ibm-models v3.15.0 removed the models, and the current predictor is `docling/models/postprocessing/reading_order_rb.py` in <https://github.com/docling-project/docling> (also MIT). The `docling` repository is not the source of the port, which used the ibm-models layout
- **License**: MIT, both repositories
- **Copyright**: `Copyright (c) 2024 International Business Machines` in both `LICENSE` files (docling-core tag `v2.108.0`, commit `5b767924cced614163e893d87612f4f674fc99d6`; docling-ibm-models tag `v3.13.3`, commit `c655e0c9f12c3bed85f3c2226932f5788d2abbf4`; identical to current HEAD). Neither file's source carries a per-file copyright header in the versions read. Both texts are reproduced in `crates/xberg/LICENSE-THIRD-PARTY`
- **Version/commit**: not recorded. The port landed in commit `97c46e9815` (2026-07-13). At that date the latest docling-ibm-models tag was `v3.13.3` (2026-06-04), which contains `reading_order_rb.py` with all the ported functions; that is the best-evidence version, not a confirmed one
- **Modifications**: PDF-point coordinates with bottom-left origin, `f32` geometry, iterative (explicit-stack) depth-first searches, horizontal dilation capped at 0.15 of page width, integration with xberg layout hints and span projection

---

## Self-hosted retrieval models

xberg re-hosts ONNX weights for 17 upstream retrieval models across four `xberg-io/*` Hugging Face
repos and serves them as embedding/reranker/sparse/late-interaction presets. Weights are re-hosted
unmodified (unchanged upstream weights, xberg-added ONNX export/manifest/tokenizer artifacts only)
under the `xberg-io` org. All upstream licenses are permissive (Apache-2.0 or MIT), consistent with
xberg's permissive-only dependency policy.

### xberg-io/embedding-models

Dense sentence-embedding presets:

- **Runtime artifact repository**: <https://huggingface.co/xberg-io/embedding-models>
- **Usage**: ONNX weights, tokenizer files, and manifests for xberg's embeddings engine

| Upstream model | License | Preset |
| --- | --- | --- |
| [sentence-transformers/all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) | Apache-2.0 | `fast` |
| [BAAI/bge-base-en-v1.5](https://huggingface.co/BAAI/bge-base-en-v1.5) | MIT | `balanced` |
| [BAAI/bge-large-en-v1.5](https://huggingface.co/BAAI/bge-large-en-v1.5) | MIT | `quality` |
| [intfloat/multilingual-e5-base](https://huggingface.co/intfloat/multilingual-e5-base) | MIT | `multilingual` |
| [Alibaba-NLP/gte-modernbert-base](https://huggingface.co/Alibaba-NLP/gte-modernbert-base) | Apache-2.0 | `gte-modernbert-base` (default) |
| [minishlab/potion-base-8M](https://huggingface.co/minishlab/potion-base-8M) | MIT | `lightweight` (model2vec static) |
| [Snowflake/snowflake-arctic-embed-m-v2.0](https://huggingface.co/Snowflake/snowflake-arctic-embed-m-v2.0) | Apache-2.0 | `arctic-embed-m-v2.0` |
| [Qwen/Qwen3-Embedding-0.6B](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B) | Apache-2.0 | `qwen3-embedding-0.6b` |

### xberg-io/reranker-models

Cross-encoder reranker presets:

- **Runtime artifact repository**: <https://huggingface.co/xberg-io/reranker-models>
- **Usage**: ONNX weights, tokenizer files, and manifests for xberg's reranking engine

| Upstream model | License | Preset |
| --- | --- | --- |
| [BAAI/bge-reranker-base](https://huggingface.co/BAAI/bge-reranker-base) | MIT | — |
| [BAAI/bge-reranker-v2-m3](https://huggingface.co/BAAI/bge-reranker-v2-m3) | Apache-2.0 | — |
| [jinaai/jina-reranker-v1-turbo-en](https://huggingface.co/jinaai/jina-reranker-v1-turbo-en) | Apache-2.0 | — |
| [Qwen/Qwen3-Reranker-0.6B](https://huggingface.co/Qwen/Qwen3-Reranker-0.6B) | Apache-2.0 | — |
| [cross-encoder/ettin-reranker-150m-v1](https://huggingface.co/cross-encoder/ettin-reranker-150m-v1) | Apache-2.0 | `balanced` (default alias) |

### xberg-io/sparse-embeddings

SPLADE-family sparse embedding presets:

- **Runtime artifact repository**: <https://huggingface.co/xberg-io/sparse-embeddings>
- **Usage**: ONNX weights, tokenizer files, and manifests for xberg's sparse embeddings engine

| Upstream model | License | Preset |
| --- | --- | --- |
| [prithivida/Splade_PP_en_v1](https://huggingface.co/prithivida/Splade_PP_en_v1) | Apache-2.0 | `splade` |
| [opensearch-project/opensearch-neural-sparse-encoding-doc-v3-distill](https://huggingface.co/opensearch-project/opensearch-neural-sparse-encoding-doc-v3-distill) | Apache-2.0 | `opensearch-v3-distill` (default) |

### xberg-io/late-interaction-models

ColBERT-family late-interaction presets:

- **Runtime artifact repository**: <https://huggingface.co/xberg-io/late-interaction-models>
- **Usage**: ONNX weights, tokenizer files, and manifests for xberg's late-interaction engine

| Upstream model | License | Preset |
| --- | --- | --- |
| [answerdotai/answerai-colbert-small-v1](https://huggingface.co/answerdotai/answerai-colbert-small-v1) | Apache-2.0 | `colbert` |
| [lightonai/GTE-ModernColBERT-v1](https://huggingface.co/lightonai/GTE-ModernColBERT-v1) | Apache-2.0 | `gte-moderncolbert` (default) |

### Policy

Model weights are not committed to this repository; verified artifacts are downloaded into the
xberg model cache at runtime, matching the GLiNER Models policy above.

**Last Updated**: 2026-07-08

---

**Last Updated**: November 22, 2026
**Pandoc Version Used**: 3.8.3
**Baseline Generation Date**: December 6, 2025
