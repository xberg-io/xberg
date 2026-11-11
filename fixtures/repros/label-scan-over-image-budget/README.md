# Label scan over the image budget

`label-scan.pdf` is a one-page 12 × 8.5 in image-only PDF holding a single 3600 × 2550 px
300 dpi RGB JPEG of a site plan: mostly one or two character labels plus a notes block.
`label-scan-mixed.pdf` puts a native text page before the same scan. With
`security_limits.max_content_size` at 80 MiB the page render needs about 93 MB of image working
memory and is refused, so OCR retries on the embedded image at its proven 300 dpi, unshrunk.
The issue (#2832) hit the same path with a 34 × 24 in sheet at the 512 MiB default.

`label-scan-downscaled.pdf` is the same sheet at 15 × 10 in (4500 × 3000 px). With `max_content_size` at 112 MiB the render is refused and the retry reads the image downscaled to 4096 px (scale 0.91, 273 dpi), not clamped.

`label-scan-low-dpi.pdf` is a 42 x 28 in sheet holding a 4200 x 2800 px image, with two letter lot labels (`--lettered`) so that a read is mostly short words. With `max_content_size` at 128 MiB the render is refused and the retry shrinks the image to 4096 px, which is 97 dpi on the sheet. Below 100 dpi a shrunk read that trips the noise check is rejected, so the page stays unread.

Regenerate with Pillow 13.0.0 and the vendored DejaVu Sans (SHA-256 checked); the grain is seeded:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py label-scan.pdf 12 8.5 300
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-mixed.pdf 12 8.5 300 --mixed
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-downscaled.pdf 15 10 300
uv run --no-project --with pillow==13.0.0 python generate.py label-scan-low-dpi.pdf 42 28 100 --lettered
```

The real-Tesseract tests in `pdf_label_scan_over_image_budget.rs` cover direct and explicit-pipeline
retention and discard, and a downscaled but legible read (scale 0.91, 273 dpi). A real-Tesseract test drops a shrunk read below 100 dpi on both routes (`label-scan-low-dpi.pdf`); the clamped case is pinned by `pdf_tall_scan_strips.rs` and the unit tests in `proven_read_tests.rs`. `pdf_cross_image_retry_2836.rs` separately
checks the retry policy with a controlled backend: an unclamped, proven-DPI JPEG read must survive an
empty, unproven, clamped auxiliary image. Those two tests check provenance pairing, not OCR
recognition accuracy.
