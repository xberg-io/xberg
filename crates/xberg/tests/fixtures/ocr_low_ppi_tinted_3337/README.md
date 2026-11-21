# Low-ppi tinted table scans (#3337)

Synthetic six-column by twelve-row tables of three-digit numbers, 101 to 172, rendered on the
tint of the #2432/#2503 reporter scans: paper `(214,224,238)`, ink `(40,52,70)`. The same layout
is rendered at three resolutions so the resample decision can be compared against a source that
needs no resample.

| file | page | glyphs | paper |
| --- | --- | --- | --- |
| `tinted-100-9px.png` | 827x1169 px, 100 DPI | 9 px digits | tinted |
| `tinted-100-7px.png` | 827x1169 px, 100 DPI | 7 px digits | tinted |
| `tinted-300-9px.png` | 2480x3508 px, 300 DPI | 27 px digits (9 px at 100 DPI) | tinted |

SHA-256:

- `tinted-100-9px.png`: `aee252d18330c1827e95e0616c08088c5b46f626596cbe00dcdf205bd31bf832`
- `tinted-100-7px.png`: `5af0e10e0a92130a61534e15ccd42f85aa052b3726bdfd09004a3c9b37e334e3`
- `tinted-300-9px.png`: `8df42b2796c2e6ffa4cf02f38890ba47755a811c8ecf06d8e552791d6e2abb97`

The paper averages 225 of 255, 88% of the channel range: about two points below the 90% near-white
level the OCR preprocessing gate used before #3337, the dimmest channel (214) six points below it.
That is why these rasters used to reach Tesseract at their own resolution.

## Generator

`generate.swift` (CoreGraphics/CoreText, run with `xcrun swift`) draws an A4 page with `Arial`,
left margin 2 em, top margin 3 em, column pitch 3.1 em, row pitch 1.5 em, em = digit height /
0.716. The PNG carries a `pHYs` density chunk, so the extractor knows the source resolution.
Regenerating produced byte-identical files for all three committed PNGs:

```
xcrun swift generate.swift 100 9 tinted tinted-100-9px.png
xcrun swift generate.swift 100 7 tinted tinted-100-7px.png
xcrun swift generate.swift 300 27 tinted tinted-300-9px.png
```

Font rendering is generation time only; the committed PNGs are the fixtures.
