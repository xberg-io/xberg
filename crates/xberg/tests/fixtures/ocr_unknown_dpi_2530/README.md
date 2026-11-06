# Unknown-DPI raster records

Synthetic input for [Xberg #2530](https://github.com/xberg-io/xberg/issues/2530).
The 4000 × 3000 RGB source draws 30 numbered lines of 48-pixel text on a
solid light green background. The PNG is the original diagnostic file.
The TIFF uses lossless Deflate compression to fit the fixture size limit
and decodes to the same RGB bytes as the original LZW TIFF. Both have
unknown physical resolution. The fixed encoded files have reproducible
workspace charges.

The decoded RGB SHA-256 is
`330d829ccd4aea982adb250e9d3215691b16fc2880617f79829e1c796ed5ff3e`.
The complete OCR oracle uses all 270 ordered lexical words from the source,
including the 30 distinct two-digit record numbers.

`large-scan.pdf` is a synthetic single-page PDF with one 4600 × 7100 grayscale
image on a 46 × 71 inch page. The image contains a large heading and 42 rows
of fine print. The 64 MiB regression asserts that OCR succeeds and recovers
the heading; it makes no assertion about the fine print at the reduced DPI.

`raw-records.png` keeps the 30 source lines and recolors the background to
RGB (236, 228, 210), below the automatic preprocessing brightness threshold.
It has no PNG physical-density chunk and exercises the raw downscale path
reported in #2618. The encoded SHA-256 is
`42fffe5932bddc2d0df54e5daff437c749fe11a49ff3ea61cd2ce3afe3393c08`.
