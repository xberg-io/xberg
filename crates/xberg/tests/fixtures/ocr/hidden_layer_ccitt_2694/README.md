`four-page-junk-layer.pdf` is a synthetic four-page scan for #2694. Every page is a
2550x3300 (300 ppi) bilevel raster stored as a single CCITT Group 4 strip (`/K -1`,
`/BlackIs1 true`) painted over a US Letter page, with an invisible (render mode 3)
numeric text layer on top. The scan text is a different set of plain English lines on
each page; the layer repeats one string of numbers three times per page. A page costs
about 27 million CCITT preview work units. The file holds no customer data.

`seven-page-junk-layer.pdf` is the same scan with three more pages (#2730). It pins the
number of per-page budgets in the document ceiling: with six budgets the seventh page is
refused. Run `generate.py out.pdf 7` to rebuild it.

Run `generate.py` to rebuild it:
`uv run --no-project --isolated --with pillow==11.3.0 python generate.py out.pdf`.
The generator asserts that the Group 4 strip decodes back to the drawn raster. The
checked-in bytes depend on the Pillow release, the libtiff it bundles and the font
`ImageFont.load_default(size=...)` selects, so another Pillow version may produce a
different but equivalent PDF. The Rust tests pin the SHA-256 of each checked-in file.
