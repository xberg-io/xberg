# Label scan over the image budget

`label-scan.pdf` is a one-page 12 × 8.5 in image-only PDF holding a single 3600 × 2550 px
300 dpi RGB JPEG of a site plan: mostly one or two character labels plus a notes block.
`label-scan-mixed.pdf` puts a native text page before the same scan. With
`security_limits.max_content_size` at 80 MiB (83 886 080 bytes) the page renders at 261 dpi: the
renderer decodes the baseline JPEG straight into its RGBA raster and lets tiny-skia scale it while
drawing, 77 141 340 bytes in all. The issue (#2832) hit the refusal path with a 34 × 24 in sheet at
the 512 MiB default.

The `-progressive` copies hold the same scans as progressive JPEGs: the same quantized coefficients,
so they decode to the same pixels, but the renderer can decode them neither at a reduced size nor in
place. `label-scan-progressive.pdf` at 80 MiB needs 92 857 078 bytes to render and is refused (up to
about 97 MB), so OCR retries on the embedded image at its proven 300 dpi, unshrunk; that read needs
74 242 138 bytes.

`label-scan-downscaled.pdf` is the same sheet at 15 × 10 in (4500 × 3000 px); at 112 MiB it renders
(108 985 353 bytes). Its progressive copy needs 134 611 970 bytes and is refused (up to about 143 MB),
and the retry reads the image downscaled to 4096 px (scale 0.91, 273 dpi), not clamped; that read's
resize needs 112 931 273 bytes.

`label-scan-low-dpi.pdf` is a 42 x 28 in sheet holding a 4200 x 2800 px image, with two letter lot labels
(`--lettered`) so that a read is mostly short words. With `max_content_size` at 128 MiB the render is
refused and the retry shrinks the image to 4096 px, which is 97 dpi on the sheet. The test reads it on
both routes and requires the same verdict from each.

`label-scan-two-pages.pdf` is two 12 × 8.5 in scan pages, each with its own copy of the image. At
80 MiB the progressive copy's renders are both refused and both pages are read from their embedded images.

Regenerate with Pillow 13.0.0 and the vendored DejaVu Sans (SHA-256 checked); the grain is seeded.
Byte-identical output only holds on macOS, where the committed PDFs were generated: on Linux the
same commands produced PDFs 302 bytes smaller, with 395537 of 9180000 pixels different, so a
regeneration there is not drop-in.

```sh
uv run --no-project --with pillow==13.0.0 python generate.py label-scan.pdf 12 8.5 300
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-mixed.pdf 12 8.5 300 --mixed
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-downscaled.pdf 15 10 300
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-low-dpi.pdf 42 28 100 --lettered
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-two-pages.pdf 12 8.5 300 --twice
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-progressive.pdf 12 8.5 300 --progressive
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-mixed-progressive.pdf 12 8.5 300 --mixed --progressive
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-two-pages-progressive.pdf 12 8.5 300 --twice --progressive
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-downscaled-progressive.pdf 15 10 300 --progressive
```

The real-Tesseract tests in `pdf_label_scan_over_image_budget.rs` read the progressive copies for
every refusal: direct and explicit-pipeline retention and discard, a downscaled but legible read (scale 0.91, 273 dpi), the per-page warnings of a
two-scan document, and the shrunk 97 dpi read on both routes. One test pins that the baseline
`label-scan.pdf` now renders and is not retried. The clamped case is pinned by
`pdf_tall_scan_strips.rs` and the unit tests in `proven_read_tests.rs`.
`pdf_cross_image_retry_2836.rs` separately
checks the retry policy with a controlled backend: an unclamped, proven-DPI JPEG read must survive an
empty, unproven, clamped auxiliary image. Those two tests check provenance pairing, not OCR
recognition accuracy.
