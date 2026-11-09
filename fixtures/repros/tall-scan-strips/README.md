# Tall scan strip regression

The fixtures contain generated grayscale scans with forty distinct text lines.
The tall and short rasters contain the same text. The mixed document puts a
native text page before the tall scan. The page size and resolution trigger
the render allocation cap only for the tall raster.

Regenerate with Pillow 13.0.0 and Arial Regular.

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . /path/to/Arial.ttf
```

The bilevel fixture has the same 40 source lines in a 1275 × 12000 raster.
Each packed DeviceGray row ends with five padding bits.
Regenerate it with the bundled DejaVu Sans font:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --bilevel-only
```

The neutral RGB variant stores the same 12,000-pixel grayscale raster in
three identical channels. Use the same font and `--neutral-rgb-only`.

The grayscale JPEG fixture uses the same 1275 × 12000 source grid and 150 DPI
as the bilevel fixture. It uses sequential 8-bit JPEG with quality 88 and
optimized entropy coding. Generate it with the bundled DejaVu Sans font and
Pillow 11.3.0.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --jpeg-only
```

The mixed JPEG fixture puts the native page of the mixed document before the same
grayscale JPEG scan. Generate it with the same font and Pillow version and
`--jpeg-mixed-only`.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --jpeg-mixed-only
```

The predictor and indexed fixtures use the same 40 lines at 20 px. The
DeviceGray predictor declares PNG Up and alternates actual None and Up row
tags. The one-bit Indexed image maps index zero to white and sets all five
padding bits on each row to the black index.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --predictor-indexed-only
```

The 120 DPI variants (bilevel, neutral RGB, Indexed and grayscale JPEG) are rendered
at 1020 × 9600 px with the same 40 lines at 0.8 of the 150 DPI glyph size, on a
612 × 5760 pt page. Regenerate them with Pillow 13.0.0 and the bundled DejaVu Sans font:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --lower-density-only
```

Of these, only the Indexed scan reads back as all 440 source words; the others pin the 150 DPI floor.

## Source geometry at different densities

`large-text-150.pdf` and `large-text-300.pdf` contain the same physical scan on
216 × 5760 pt pages. The 450 × 12000 grayscale raster is enlarged by exact
2 × 2 pixel replication to 900 × 24000; no new glyphs or OCR-derived text
are introduced. A 64 px source line precedes forty 20 px prose lines.
`large-text-expected.txt` comes directly from the strings drawn by the generator.
The source font is the repository's DejaVu Sans, checked by SHA-256 in the script.

```sh
mise exec -- uv run --no-project --with pillow==13.0.0 python generate-scaled.py
```

The public API tests also losslessly compress the existing neutral RGB fixture's
page-paint stream, verifying identical decoded instructions before extraction.
An additional-drawing variant exercises refusal of pages that are not one scan.

Successful source-strip recovery retains the original rendered and processed frames
in `ocr_source_strip_original_frames`, ordered by accepted document page number.
Detached-stage page IDs are rebound to those document pages. The existing
`ocr_source_strip_original_frame` object remains a compatibility view of the first
entry. These historical dimensions survive both whole-document and automatic
per-page OCR projection; they are not the recovered text's current coordinate frame.
The public tests check both keys and the actual 154 × 4096 processed frame against
the retained native render dimensions at both source densities. They exercise raw, Flate and
ASCII85+Flate page-paint streams, including separately serialized extra-paint
controls that must report an uncertified source. The low-density geometry tests
retain the pre-existing 80-row inclusive and 48-column exclusive limits at 120 DPI.
