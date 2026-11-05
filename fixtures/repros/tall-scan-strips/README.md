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
