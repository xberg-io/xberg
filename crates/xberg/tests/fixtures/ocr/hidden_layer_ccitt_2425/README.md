These four synthetic scans use the same 1275×1650 bilevel raster at 150 ppi.
The source was rasterized with Poppler and thresholded at 128. The Flate
variant stores 8-bit gray samples. The CCITT variant contains a single
6,676-byte Group 4 strip made by libtiff, with explicit columns, rows and
black-sample polarity. Poppler independently decoded every image to the same
2,103,750 luma samples.

The partial hidden layer contains source lines 1–4 and 11. The complete
hidden layer contains all eleven lines. The source text was written before
rendering and is stored separately as the 81-token OCR oracle.

Run `generate.py` with Pillow 11.3.0, Poppler 26.10.0, and libtiff 4.7.2
to reproduce the fixtures in a new directory. The generator checks each
decoded image against the original luma bytes. The public Rust regression
pins all four PDF hashes.
