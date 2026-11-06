# PDF OCR workspace fixture

`title-layer-scan.pdf` is a synthetic one-page PDF with a 46 × 71 inch MediaBox.
Its only native text is the four-word title “Signed Native Source Title.”
A 4,600 × 7,100 grayscale raster contains the heading “Quarterly Accounts Report”
and 30 records numbered 01 through 30. The raster is inset on a gray page to
exercise the built-in Tesseract path without scan preprocessing.

At a 64 MiB limit, the renderer reduces the page to 41 DPI. The default OCR
pipeline keeps that raster available to its stages, so Tesseract has less
workspace than the renderer had. The regression test identifies the producing
backend and checks the native title, raster heading, and record numbers.
