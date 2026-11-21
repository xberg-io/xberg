# Six-column dot-leader table at 150 dpi (#2503)

`gen_six_column_fixture.py` draws twelve invented rows — one label column and five
numeric columns, dot leaders between the label and the count — with DejaVu Sans 20 px
at 150 dpi. It writes:

- `six-column-150dpi.tsv` — the generator's source rows, independent of OCR.
- `six-column-150dpi.png` — the table raster, 770 × 534 px at 150 dpi.
- `six-column-150dpi.pdf` — a one-page 150 dpi scan whose page raster is the table,
  with no margins. A page with margins reads this table as prose at PSM 6, so the
  scan route needs the table to fill its page.

Regenerate with Pillow 13 and the repository's DejaVu Sans:

```text
uv run --no-project --with pillow python gen_six_column_fixture.py \
  ../../../../xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf .
```

SHA-256:

- `six-column-150dpi.png` `b369ad154e8ea715b10db8e23cc6ced72820cef0a0d623f5aa46190ffe9eb803`
- `six-column-150dpi.pdf` `98fa2e76586868873167a0b5137743f43323c84dca9f843176315aa76fb287b1`
- `six-column-150dpi.tsv` `433015462574b6a5e3f50dd78a04550729b29bf3ac788b7ff00cbda85ac40640`
