# Federal Register document-type corpus (2026-09-01 .. 2026-11-20)

408 documents from the [Federal Register](https://www.federalregister.gov) public API, published
between 2026-09-01 and 2026-11-20. The Office of the Federal Register publishes the document type
as metadata, so the gold label never comes from the filename, the directory or the text.

- **Task:** three-class document type — `final rule`, `proposed rule`, `public notice` — plus
  presidential documents as an unknown class that is outside the vocabulary.
- **Source:** `https://www.federalregister.gov/api/v1/documents.json`, raw text from the pinned
  `raw_text_url` of each document.
- **License:** US-PD (17 U.S.C. §105). See `LICENSE-AUDIT.md`.
- **Pins:** `source_sha256` is the SHA-256 of the exact fetched response bytes, `text_sha256` the
  SHA-256 of the normalized text that reaches the classifier. `manifest.digest` covers the document
  list; the file SHA-256 is what the preregistration pins.

## Selection

- Types fetched: `RULE`, `PRORULE`, `NOTICE`, `PRESDOCU`.
- Cap: 120 documents per known class, 48 for the unknown class, at most 12 per known-class family.
- Within a family, documents are taken in ascending document-number order. Within a class the cap
  is applied in ascending family-slug order. The acquisition is deterministic for a fixed API
  snapshot.
- **Family** is the top-level issuing agency slug. A family is assigned to exactly one split
  because an agency reuses its own headings and boilerplate.
- **Unknown class:** presidential documents are a single template family
  (`executive-office-of-the-president`) reserved for the held-out split, so unknown-class rejection
  is measured on 48 documents instead of a handful.

## Splits

| split | documents | families | known | unknown |
| --- | --- | --- | --- | --- |
| training | 135 | 24 | 135 | 0 |
| calibration | 87 | 12 | 87 | 0 |
| held_out | 186 | 25 | 138 | 48 |

The zero-shot candidate is not trained, so the training split is reserved for a future trained
head and does not enter the reported run. The calibration split is scored for calibration
behaviour but never enters the go/no-go decision. The plan contains the calibration and held-out
documents: 273 × 1 model × 1 arm × 3 repeats = 819 inferences.

## Text normalization

The raw Federal Register text is an HTML page wrapping a `<pre>` block. Normalization removes the
HTML wrapper, then drops the lines that state the type as metadata: the bracketed banner
(`[Proposed Rules]`, `[Notices]`, ...), the `[Federal Register Volume ...]`, `From the Federal
Register Online ...` and `[FR Doc No: ...]` lines, and the `ACTION:` block up to the next blank
line. At most the first 60,000 UTF-8 bytes are kept at acquisition.

A second, token-bounded freeze is applied because the NLI adapter **refuses** an over-long premise
instead of truncating it. `onnx-qualification truncate` re-encodes each document with the pinned
candidate tokenizer and keeps the largest prefix that fits the 512-token pair window once the
hypothesis is counted: a budget of 493 premise tokens with a round-trip margin. 395 of 408
documents are cut; the median document is 1,991 bytes. Every document keeps `full_text_sha256` and
`full_text_bytes`, so the untruncated text is still identified even though it is not what the model
sees. The next paragraph states what that costs.

**Cost of the window:** the model sees roughly the first 500 tokens — the agency header, docket,
title and `SUMMARY` — not the whole document. That is exactly the window a caller with this adapter
would get, and long-document degradation beyond it is therefore not measured. `SUMMARY` is part of
the document, not metadata, and it is frequently where a rule says what it does.

What remains is still the document's own prose. Many documents say "this final rule", "we
propose" or "this notice"; those are semantic statements in the body, not metadata, and removing
them would rewrite the documents. The audit quantifies them and the scorer reports a cue-free
sensitivity stratum rather than hiding the reliance.

## Reproduce

```sh
python3 tools/benchmark-harness/onnx_qualification/scripts/acquire_classification_corpus.py \
  --start 2026-09-01 --end 2026-11-20 --cap 120 --unknown-cap 48 \
  --out /path/to/text-cache --manifest qualification/classification/federal-register-2026/manifest.json
```

The command re-fetches current API results, so it reproduces the *procedure*, not this frozen
document set. The manifest pins the frozen set.

## Frozen bytes

`manifest.json` carries two independent pins: `digest` covers the canonical document list (sorted
keys, no whitespace), and the file SHA-256 is pinned in `qualification/preregistration.json` and in
every plan's `provenance.corpus_receipt_sha256`. Reformatting the file leaves `digest` unchanged and
changes the file pin, so the two must be updated together. Any regeneration produces a new file and
requires a new preregistration.
