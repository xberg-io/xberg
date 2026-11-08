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
