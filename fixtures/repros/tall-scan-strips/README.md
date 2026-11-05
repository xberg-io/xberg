# Tall scan strip regression

The fixtures contain generated grayscale scans with forty distinct text lines.
The tall and short rasters contain the same text. The mixed document puts a
native text page before the tall scan. The page size and resolution trigger
the render allocation cap only for the tall raster.

Regenerate with Pillow 13.0.0 and Arial Regular.

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . /path/to/Arial.ttf
```
