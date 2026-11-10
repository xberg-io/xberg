# Source number in a Khmer scan

Both synthetic pages contain the same 150 DPI scan. The scan draws the word
`ប០ប` followed by six independent English prose lines. An invisible text
show at the source word's position reports `ប០ប` in `matching.pdf` and
`ប១ប` in `misleading.pdf`. At 150 DPI, Poppler emits identical grayscale
PGM bytes for both pages, SHA-256
`70fcfabe3153e465c58e0642dd31337f776b133bf8abb87c20eeebac16102241`.

Regenerate using ReportLab and Poppler with the hinted regular TTF from
[Noto Sans Khmer v2.004](https://github.com/notofonts/khmer/releases/tag/NotoSansKhmer-v2.004).
The generator checks its exact SHA-256. The embedded font is covered by
[NotoSansKhmer-OFL.txt](NotoSansKhmer-OFL.txt). Poppler and ReportLab are not
pinned, so regeneration is not guaranteed to reproduce byte-identical PDFs or
rasters; the checked-in pair and its independent hashes remain the qualification
inputs.

The pair is restored byte-for-byte from closed draft
[#2642](https://github.com/xberg-io/xberg/pull/2642), commit
`c762c94167395b764ae6399aab8480c406ef5a91`. It is public synthetic material,
not the reporter's original page for [#2408](https://github.com/xberg-io/xberg/issues/2408).

`pdf_khmer_number_2408.rs` guards the fixture identity, independent numeric
recovery and refusal of false hidden digits. Its ToUnicode control changes only
`<02> <17E0>` to `<02> <17E1>`: both painted and revealed-glyph rasters remain
identical, while the native text falsely claims one. Shape agreement alone
cannot authenticate the digit's Unicode value.

Recovery requires an already-provisioned `khm.traineddata` in the same tessdata
directory selected for the primary English read. The English OCR text is
unchanged when that model is absent; eligible default automatic pages carry
an OCR processing warning that the native Khmer numbers could not be verified.
Forced OCR, custom Tesseract options and layout paths do not receive this
numeric-recovery warning. Missing-model diagnostics may still be emitted before
the later Type0 ink-loss exclusion. The separate unreadable-script guard
uses letter and cluster evidence and does not cover every numeric-only layer.
Extraction never downloads or searches for an auxiliary model.

Each promoted numeric run must be independently read from its own painted crop
and have one owning OCR line. Comparison ignores only whitespace and Khmer
zero-width spaces; other Unicode differences still reject recovery. The source
number is inserted at its line, including an occurrence below the prose.
Recovery requires positive scale-only evidence bound to the selected OCR
geometry; deskewed reads and descending-x lines stay unchanged. Overlapping
claims cannot multiply one painted occurrence.

Insertion retains the original OCR tokens, including a misread `O`, `m` or `v`
next to a recovered number. Replacing them would require separate word-level
proof. New numeric segments keep neutral style flags and use their owning line's
positive finite font scale for structure heuristics, rather than copying a
neighbor's style or claiming measured source typography. Invalid line scales
decline recovery; no OCR confidence is invented. Forced OCR stays unchanged.

One lazy processor is retained for the OCR run. Its bounded native pool reuses
the initialized Khmer engine across pages, while crops remain serial with at
most 64 native numeric glyphs per page. Every crop still incurs recognition,
image/config hashing and model-context resolution, even with result caching
disabled. The independent numeral read is extra work on eligible pages only. Eligible
native Khmer-number pages bypass both the primary OCR cache and extraction-result
cache, with or without the auxiliary model, so installing it changes the result.

Run the mandatory model qualification explicitly:

```sh
XBERG_2408_KHM_MODEL=/path/to/khm.traineddata mise run rust:test:khmer-number-recovery
```

The qualification requires all twelve public issue tests and fifteen unit
controls to be collected and executed, including every external-model case; a
missing or altered model fails setup. The existing Rust CI workflow invokes it
through `.github/actions/qualify-ocr-orientation/action.yml`, which explicitly
provisions the pinned model. Ignored external-model cases are mandatory in that
qualification and cannot silently disappear from its inventory.
The ordinary English-only and false-digit controls run without that model.
No model asset is bundled. Provision the Apache-2.0
[tessdata_fast Khmer model][khm]
from revision `87416418657359cb625c412a48b6e1d6d41c29bd`, Git blob
`d466dc2a7e35bf90cb7b54364985703728398687`, size 1,446,926 bytes,
SHA-256 `47f110575341b322052f3becbefee61a3ecf1ef549352b5d4d33d28afe30d099`.
The upstream [license][lic]
applies to that model.

This is a bounded capability improvement for upright Khmer numeric runs. It is
not a general English-only recovery fix or evidence about the unavailable
reporter page. Rotated/cropped pages, custom OCR settings, layout/pipeline
routes and rows already containing OCR numbers remain unchanged. A Latin date or total
on the owning line also prevents recovery. Type0 rasters that lost glyph ink
remain unchanged: a numeral crop cannot corroborate the rest of that page.
Recovery also declines detached stages without positive raster-integrity
evidence; missing evidence is not proof that the raster retained every glyph.

[khm]: https://github.com/tesseract-ocr/tessdata_fast/blob/87416418657359cb625c412a48b6e1d6d41c29bd/khm.traineddata
[lic]: https://github.com/tesseract-ocr/tessdata_fast/blob/87416418657359cb625c412a48b6e1d6d41c29bd/LICENSE

When one OCR segment contains multiple words and overlaps a numeric source crop,
recovery leaves that segment unchanged: its word positions are unavailable, so
appending the number could move it past suffix prose. Single OCR tokens remain
intact and can still appear beside a recovered number. The ordering control
uses readable prose on both sides of the painted digit, compares the result with
the primary-only read, and retains the standalone top/bottom recovery positives.

Recovery requires complete located paragraph text and exactly one geometric
line owner. An overlapping line remains ambiguous even if its text, font scale
or ordering makes it ineligible for insertion.

Successfully recovered Plain pages take their text from the finalized document
through the existing page renderer, including list and document normalization.
Only recovered pages are synchronized; stages that defer assembly remain
unchanged. The positive control checks exact slices on one and two pages, with
and without adjacent real OCR lists.

Verified numerals do not count as evidence that OCR retained the native prose's
script. The existing native-script retry and retention check excludes numerals
from that evidence, including numeric-only reads in another script block.
