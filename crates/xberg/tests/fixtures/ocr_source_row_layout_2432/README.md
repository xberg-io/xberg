Synthetic 150 ppi scan from the generator posted in [#2432](https://github.com/xberg-io/xberg/issues/2432).
The source's twelve records are in `rows.tsv`. The checked-in PDF uses lossless
stream compression; its 150 ppi rendering is byte-identical to the source PDF.
The PDF contains one raster image and no native text layer.

`image-11-no-leaders.png` is the reporter generator's 11 px DejaVu Sans
standalone plain image, without leaders or grid. SHA-256
`7b5e47e12dfacd331c17c0d2dc9e54f17ea1eede2ec8ea0ca7a890eb56e3de25`.
The twelve independent source rows are in `image-11-source-rows.tsv`.
The OCR regression expects the recognized `(1.463)` to remain literal in
the correct row. The source cell is `(1,463)`.
