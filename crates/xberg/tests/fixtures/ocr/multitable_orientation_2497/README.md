These synthetic one-page PDFs have one RGB raster and no native text layer.
Each page has two separate 20×4 tables and 200 source tokens. The fixtures
store the same raster at 0°, 90° and 270° clockwise, wrapped at 150 ppi.

The cell oracle was generated before rendering or OCR. The source uses
Pillow 13.0.0 and the bundled DejaVuSans at 20 px. The independent random
number generators for the upper and lower grids are seeded with 3 and 5,
and labels use `Cash Rent Travel Repairs Supplies Taxes Payroll Sales`
indexed by row IDs 100–119 and 300–319 modulo eight. Each row has its label
and ID in the first cell and three independently generated comma-separated
integers from 10000 to 99999. The canvas is 1276×1650, columns begin at
80, 360, 640 and 920 pixels, and rows start at 100 and 900 with 30 px
spacing. The hashes identify the exact checked fixtures.

The `strict-two-tables` fixtures are a separate, previously generated
source control. They use the same 1276×1650 frame at 150 ppi, with two
20×4 tables drawn in Arial at 21 px and converted to grayscale. The two
tables start at source y coordinates 120 and 890 with 31 px row spacing.
The columns are x coordinates 60, 260, 380, and 500. A Python random
generator seeded with 3 produced the three comma-formatted values per
record before rendering. The generated exact cell data is in
`strict-cells.json`. The 90° fixture's raw and retained primary word
counts agree, while the original RGB pair intentionally exercises refusal.
