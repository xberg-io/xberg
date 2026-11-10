# Label scan over the image budget

`label-scan.pdf` is a one-page 12 × 8.5 in image-only PDF holding a single 3600 × 2550 px
300 dpi RGB JPEG of a site plan: mostly one or two character labels plus a notes block.
`label-scan-mixed.pdf` puts a native text page before the same scan. With
`security_limits.max_content_size` at 80 MiB the page render needs about 93 MB of image working
memory and is refused, so OCR retries on the embedded image at its proven 300 dpi, unshrunk.
The issue (#2832) hit the same path with a 34 × 24 in sheet at the 512 MiB default.

Regenerate with Pillow 13.0.0 and the vendored DejaVu Sans (SHA-256 checked); the grain is seeded:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py label-scan.pdf 12 8.5 300
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-mixed.pdf 12 8.5 300 --mixed
```
