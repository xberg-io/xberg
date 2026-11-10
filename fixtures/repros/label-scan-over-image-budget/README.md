# Label scan over the image budget

`label-scan.pdf` is a one-page 12 × 8.5 in image-only PDF holding a single 3600 × 2550 px
300 dpi RGB JPEG of a site plan: mostly one or two character labels plus a notes block. With
`security_limits.max_content_size` at 80 MiB the page render needs about 93 MB of image working
memory and is refused, so OCR retries on the embedded image at its proven 300 dpi, unclamped.
The issue (#2832) hit the same path with a 34 × 24 in sheet at the 512 MiB default.

Regenerate with Pillow and the macOS Arial path (change `FONT` on other systems):

```sh
uv run --no-project --with pillow python generate.py label-scan.pdf 12 8.5 300 plan
```
