# Source number in a Khmer scan

Both synthetic pages contain the same 150 DPI scan. The scan draws the word
`ប០ប` followed by six independent English prose lines. An invisible text
show at the source word's position reports `ប០ប` in `matching.pdf` and
`ប១ប` in `misleading.pdf`. At 150 DPI, Poppler emits identical grayscale
PGM bytes for both pages, SHA-256
`70fcfabe3153e465c58e0642dd31337f776b133bf8abb87c20eeebac16102241`.

Regenerate using ReportLab and Poppler with the hinted regular TTF from
[Noto Sans Khmer v2.004](https://github.com/notofonts/khmer/releases/tag/NotoSansKhmer-v2.004).
Use the source TTF whose SHA-256 is
`e66675f2082788f0511a714bef5a1748928294b38c8e286a96ea73a864b5e605`:

```sh
python3 generate.py /path/to/NotoSansKhmer-Regular.ttf /path/to/output-directory
python3 generate.py /path/to/NotoSansKhmer-Regular.ttf /path/to/output-directory --lookalikes-only
```

The second command regenerates only the painted Latin `O` and ASCII `0` controls,
leaving the original pair untouched. It also requires the repository's
`crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf`, SHA-256
`abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322`.
The generator checks both font hashes. The embedded font is covered by
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

The original pair and both look-alike controls use physical character code
`0x01` → glyph ID 2 and `0x02` → glyph ID 3. The matching/control ToUnicode
map claims `0x01` → U+1794 and `0x02` → U+17E0; misleading.pdf claims
U+17E1 for `0x02`. Glyph IDs are local to each embedded font, not universal
identifiers. The matching/control embedded font SHA-256 is
`351cdb7a62fc16c2192da074d491f432a8dd4665589e6032992dfa574fe87db5`;
the misleading font is
`adbfe7b7371a58a0d1fe92c0c902f477be26c3a1f88054394a089ba46ee1bbee`.
The look-alike PDF hashes are
`2589b3d11be6e45c9e287830c5f08050201a97403a2f691651490157b394ae1e`
(painted Latin O) and
`1f1a0ed8d95f3446bad5d2452267bb4cbe049c203480c5f193bedc9634f436f6`
(painted ASCII zero). Their hidden layer still claims Khmer zero. The Khmer
model itself misreads painted Latin O as Khmer zero, so recognition agreement
alone is insufficient.

Activation is automatic when the selected English-primary directory contains
both models; there is no separate configuration opt-in. All-model deployments
can therefore receive changed output and extra work. Primary languages other
than exactly `eng`, including `deu`, `khm` and `eng+khm`, retain their ordinary
OCR policy and do not enter this recovery feature. See
[ADR 0014](../../../../../adrs/0014-gate-khmer-numeric-recovery-on-locally-provisioned-models.md).

Recovery requires an already-provisioned `khm.traineddata` in the same tessdata
directory selected for the primary English read. The English OCR text is
unchanged when that model is absent; eligible default automatic pages carry
an OCR processing warning that the native Khmer numbers could not be verified.
Forced OCR, custom Tesseract options and layout paths do not receive this
numeric-recovery warning. Warnings are deduplicated and emitted only after an otherwise eligible primary
page is accepted; Type0 ink-loss and nonzero-offset stages are excluded. The separate unreadable-script guard
uses letter and cluster evidence and does not cover every numeric-only layer.
Auxiliary verification never downloads, materializes or searches another root
for a model, including when the selected file disappears after preparation.

Each promoted numeric run must be independently read from its own painted crop
and have one owning OCR line. It must also match independently rendered native
glyph outlines in both ink directions, allowing only a one-pixel rasterization
margin. This initial shape verifier admits simple embedded TrueType fonts with
a byte-indexed MacRoman cmap, a unique physical code for the native claim and
untransformed glyph coordinates. Admitted fonts have at most 256 glyphs,
1,024 points per simple glyph and 4,096 total points. Composite outlines, Forms,
non-BMP claims, variable/color/CFF tables, Type0, Unicode-shaping fonts, ambiguous codes,
unsupported font encodings and uncertain shapes decline recovery. Font objects,
stream expansion, retained bytes and cumulative crop pixels have explicit
budgets, including a 4 MiB reservation per font for native caches and a 2 MiB
font working reservation per render. This is deliberately narrower than general Khmer OCR support.

Source admission permits at most 16 KiB of cumulative raw/decoded page content,
4,096 conservatively counted text bytes, 1,024 operators and 4,096 lexical
tokens. Font names and aliases are limited to 128 bytes. Tagged/ActualText
content, property/ExtGState resources, marked content, lexical comments,
inline images and unknown
operators decline before character extraction. The admitted text bounds reserve
character/string/parser work before emission and deduplication. One owned
character buffer is reused for candidates and glyph preparation; this auxiliary
path does not populate native page-character or content caches. These limits
apply to the auxiliary verifier's additional work.
 Comparison ignores only whitespace and Khmer
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

The selected primary read must fit a 256 KiB serialized content/geometry proof.
Exceeding that cap retains the accepted OCR read and emits a verification-gap
warning. Existing paragraph bytes, including multiline whitespace, are retained;
only the verified number and its separator are inserted.

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

The qualification requires every explicitly named public, recovery-unit and
glyph-shape control to be collected and executed, including every external-model case; a
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
A missing final rendered page preserves the earlier page read. Only recovered pages are synchronized; stages that defer assembly remain
unchanged. The positive control checks exact slices on one and two pages, with
and without adjacent real OCR lists.

Verified numerals do not count as evidence that OCR retained the native prose's
script. The existing native-script retry and retention check excludes numerals
from that evidence across all supported scripts, including numeric-only reads
in another script block. This global letter-evidence correction is separate
from the English-primary Khmer numeric recovery capability.
