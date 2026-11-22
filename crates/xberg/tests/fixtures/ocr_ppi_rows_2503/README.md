# JPEG q45 table pictures at low resolution (#2503)

The JPEG arm of the #2503 low-ppi table cases: the issue keeps "the JPEG q45 and 7 px
standalone-picture cases" open because both reach OCR with small glyphs. These two rasters
are the committed generator layouts saved as JPEG quality 45. The 7 px picture is the
committed `../ocr_low_ppi_tinted_3337/tinted-100-7px.png` and is not duplicated here.

| file | layout | glyphs | paper |
| --- | --- | --- | --- |
| `six-column-150dpi-q45.jpg` | #2503 six-column generator, seed 21 (labels, dot leaders, five value columns) | 20 px at 150 dpi | white |
| `six-column-100dpi-9px-q45.jpg` | #3337 low-ppi grid (six number columns, 101..172) | 9 px at 100 dpi | white |

`rows.tsv` is the six-column generator's source rows (byte-identical to
`../ocr_six_column_rows_2503/six-column-150dpi.tsv`); `number-rows.tsv` is the grid's
101..172 source rows. Both are generator inputs, independent of OCR.

SHA-256:

- `six-column-150dpi-q45.jpg`: `d7b2f02032199e7d9506fe9036aca8613a60b4c4efd068d9727a54d7af1fa11e`
- `six-column-100dpi-9px-q45.jpg`: `fe21997298f943469ca774b3125136f767ca5bdf7b69a9214eab8795aedf36ff`
- `rows.tsv`: `433015462574b6a5e3f50dd78a04550729b29bf3ac788b7ff00cbda85ac40640`
- `number-rows.tsv`: `89966299bbb666f133e4b8a0a46b432f5cd79966b4a5742dfaf561ac44edbc96`

## Generator

`gen_ppi_rows.py` (Pillow, DejaVu Sans) draws both rasters and saves them at quality 45 with
their source density in the JFIF header. Regenerating produced byte-identical files for both
JPEGs:

```text
uv run --no-project --with pillow python gen_ppi_rows.py \
  ../../../../xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf .
```

## Measured rows kept on one line (main 281f71511c, reproduced at 26549dc005)

Twelve source rows per case; a row counts only when its cells appear in order on exactly one
output line. The lossless `../ocr_six_column_rows_2503/six-column-150dpi.png` is the
control.

| case | plain psm 6 | plain psm 11 | markdown psm 6 | markdown psm 11 |
| --- | ---: | ---: | ---: | ---: |
| six-column 150dpi lossless control | 12/12 | 12/12 | 12/12 | 12/12 |
| six-column 150dpi q45 | 12/12 | 10/12 | 12/12 | 10/12 |
| six-column 100dpi 9px q45 | 11/12 | 12/12 | 11/12 | 12/12 |
| tinted 100dpi 7px (#3337) | 5/12 | 11/12 | 5/12 | 11/12 |

Every missing row is a recognition loss, not a row split: its cells are still on one line with
one or two read wrong — `111` -> `211`, `155` -> `185`, `147`+`148` -> `147148`, `11` -> `Li`,
a leader -> `ee`. All four rasters take the default preprocessing and the 300 DPI resample
(scale 2.4 from the 150 dpi JPEG, 3.0 from the 100 dpi ones) and the 7 px grid still loses its
smallest glyphs; that belongs to the resample policy of #2327, not to row structure.

`crates/xberg/tests/ocr_ppi_rows_2503.rs` pins these counts as floors.
