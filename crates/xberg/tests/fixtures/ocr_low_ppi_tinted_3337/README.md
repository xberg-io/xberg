# Low-ppi tinted table scans (#3337)

Synthetic six-column by twelve-row tables of three-digit numbers, 101 to 172, rendered on the
tint of the #2432/#2503 reporter scans: paper `(214,224,238)`, ink `(40,52,70)`. The same layout
is rendered at three resolutions so the resample decision can be compared against a source that
needs no resample, and the two 100 DPI rasters are also wrapped into one-page PDF scans.

| file | page | glyphs | paper | route |
| --- | --- | --- | --- | --- |
| `tinted-100-9px.png` | 827x1169 px, 100 DPI | 9 px digits | tinted | PNG -> OCR |
| `tinted-100-7px.png` | 827x1169 px, 100 DPI | 7 px digits | tinted | PNG -> OCR |
| `tinted-300-9px.png` | 2480x3508 px, 300 DPI | 27 px digits (9 px at 100 DPI) | tinted | PNG -> OCR |
| `tinted-100-9px.pdf` | 595.4x841.7 pt, 100 DPI page raster | 9 px digits in the page raster | tinted | PDF raster -> OCR |
| `tinted-100-7px.pdf` | 595.4x841.7 pt, 100 DPI page raster | 7 px digits in the page raster | tinted | PDF raster -> OCR |

SHA-256:

- `tinted-100-9px.png`: `aee252d18330c1827e95e0616c08088c5b46f626596cbe00dcdf205bd31bf832`
- `tinted-100-7px.png`: `5af0e10e0a92130a61534e15ccd42f85aa052b3726bdfd09004a3c9b37e334e3`
- `tinted-300-9px.png`: `8df42b2796c2e6ffa4cf02f38890ba47755a811c8ecf06d8e552791d6e2abb97`
- `tinted-100-9px.pdf`: `1ed4461fede583c8cd030faeb5dc2e24979f91f224f755409c6e85cd1bfca81f`
- `tinted-100-7px.pdf`: `79e795ac533bd27156e59870dfaca2d02aa7ba1917384b2fb763d4de2f6dadf3`

The paper averages 225 of 255, 88% of the channel range: about two points below the 90% near-white
level the OCR preprocessing gate used before #3337, the dimmest channel (214) six points below it.
That is why these rasters used to reach Tesseract at their own resolution.

## Generator

`generate.swift` (CoreGraphics/CoreText, run with `xcrun swift`) draws an A4 page with `Arial`,
left margin 2 em, top margin 3 em, column pitch 3.1 em, row pitch 1.5 em, em = digit height /
0.716. The PNG carries a `pHYs` density chunk, so the extractor knows the source resolution.
A fifth argument wraps the PNG just written into a one-page PDF: the page is the raster's own
size in points at its own density (827x1169 px over 595.4x841.7 pt at 100 DPI), and the PDF
draws that PNG, so the scan route receives the committed raster pixel for pixel. Regenerating
produced byte-identical files for all three committed PNGs:

```text
xcrun swift generate.swift 100 9 tinted tinted-100-9px.png tinted-100-9px.pdf
xcrun swift generate.swift 100 7 tinted tinted-100-7px.png tinted-100-7px.pdf
xcrun swift generate.swift 300 27 tinted tinted-300-9px.png
```

The PDFs carry no creation date and no producer string; CoreGraphics still writes a random
trailer `/ID`, so a regenerated PDF differs from the committed one in that value alone. Compare
the page raster, not the PDF bytes. Font rendering is generation time only; the committed
rasters are the fixtures.

## Local baseline

Measured with the vendored Tesseract 5.5.4 on macOS arm64, at PSM 11, with the default OCR
preprocessing. The PNG fixtures are asserted on their number count. The scan fixtures are
asserted on row preservation: a row counts only when one output line holds all six of its
source numbers in order, and the count is reported next to the number count.

| fixture | route | measured | floor in test |
| --- | --- | --- | --- |
| `tinted-100-9px.png` | PNG | 72 of 72 numbers | 68 numbers |
| `tinted-100-7px.png` | PNG | 71 of 72 numbers | 60 numbers |
| `tinted-300-9px.png` | PNG | 72 of 72 numbers | 68 numbers |
| `tinted-100-9px.pdf` | scan | 12 of 12 rows, 72 of 72 numbers | 12 rows |
| `tinted-100-7px.pdf` | scan | 7 of 12 rows, 66 of 72 numbers | 7 rows |

The PNG floors sit between the pre-#3358 result (65 and 49 numbers) and the current one, so a
reverted resample gate fails while a recognizer that reads one or two numbers differently does
not. `ocr_low_ppi_tinted_resample_3337` asserts them.

The scan floors are the measured rows themselves. A 100 DPI page is rendered by xberg at the
150 DPI default render floor before the OCR preprocessor resamples it to 300, so its 7 px glyphs
reach Tesseract through 1.5x then 2x interpolation: one or two digits of a row come back wrong
while the row itself stays a line, which is why that scan keeps 7 of 12 rows instead of 12. The
9 px scan keeps every row, and it represents the reporter's 100 DPI scans.
`ocr_low_ppi_scan_replay_3337` asserts the row floors and prints the per-scan result under
`--nocapture`, so the CI host records its own counts.

## The reporter's scans

The four 100 DPI tinted scans behind #3337 (`scan_100_tint_lead` and its siblings) kept 11, 10,
0 and 11 of 12 rows on the CI host. They are not in this repository, and the replay that
produced those counts was never committed, so their numbers are not reproduced here. These
fixtures use the same table shape and tint, not the reporter's pixels; their baselines are
measured independently and are not a restatement of the reporter's counts.
