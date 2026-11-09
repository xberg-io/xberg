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
[NotoSansKhmer-OFL.txt](NotoSansKhmer-OFL.txt).


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
directory selected for the primary English read. English-only extraction is
unchanged. Extraction never downloads or searches for an auxiliary language
model. Each promoted numeric run must be independently read, with exactly the
same Unicode text, from its own painted crop and have one owning OCR line.
The OCR prose is retained; numbers are inserted at that line, including a source
occurrence below the prose. Recovery requires a positively observed scale-only primary read bound to the
selected OCR geometry; deskewed reads and descending-x lines stay unchanged.
Forced OCR is unchanged. Overlapping native claims cannot multiply a painted
occurrence; ambiguous overlapping claims leave the OCR read unchanged.

Run the mandatory model qualification explicitly:

```sh
XBERG_2408_KHM_MODEL=/path/to/khm.traineddata mise run rust:test:khmer-number-recovery
```

The qualification requires all twelve issue tests to be collected and executed,
including the external-model cases; a missing or altered model fails setup.
The ordinary English-only and false-digit controls run without that model.
No model asset is bundled. Provision the Apache-2.0
[tessdata_fast Khmer model](https://github.com/tesseract-ocr/tessdata_fast/blob/87416418657359cb625c412a48b6e1d6d41c29bd/khm.traineddata)
from revision `87416418657359cb625c412a48b6e1d6d41c29bd`, Git blob
`d466dc2a7e35bf90cb7b54364985703728398687`, size 1,446,926 bytes,
SHA-256 `47f110575341b322052f3becbefee61a3ecf1ef549352b5d4d33d28afe30d099`.
The upstream [license](https://github.com/tesseract-ocr/tessdata_fast/blob/87416418657359cb625c412a48b6e1d6d41c29bd/LICENSE)
applies to that model.

This is a bounded capability improvement for upright Khmer numeric runs. It is
not a general English-only recovery fix or evidence about the unavailable
reporter page. Rotated/cropped pages, custom OCR settings, layout/pipeline
routes and rows already containing OCR numbers remain unchanged.
