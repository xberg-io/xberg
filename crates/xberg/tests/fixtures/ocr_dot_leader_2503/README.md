These synthetic PNGs and their ground-truth rows are from the public #2432
reporter's Pillow / DejaVu Sans recipe, using the plain 11, 14 and 20 pixel inputs
with dot leaders. The TSV values are the generator inputs, independently of OCR.
`gen_leader_fixtures.py` regenerates `11.png`, `11.tsv` and `toc.png` byte for byte (Pillow 12.3.0,
DejaVu Sans): `uv run --no-project --with pillow python gen_leader_fixtures.py DejaVuSans.ttf <out dir>`.

- `14.png` SHA-256 `89d90ef545561abf853817691e3082ee27e37cc72876a3419c37125371b6ace5`
- `11.png` SHA-256 `a89aab906e8e2af09070c86286088b966189ee4653597a0c981bcaa046dafec0` (the first read has no table, and the block read's table is dropped as prose while its rows stay whole)
- `20.png` SHA-256 `f1321bb2850e33e519ed02a8682df6b1d74c02106e256e52ae544361add769f2`

`toc.png` is a generated 20 px table of contents (title, dot leader, page number per line; invented titles), SHA-256 `5e27c8362d1eba6eae6a49cf36550436c267fcd410dab24ec94e9abadf19b601`. It has leaders but only two columns, so the re-read never admits it and it keeps its first read.

`two_column.png` is a generated two-column justified prose page (Pillow, DejaVu Sans 20 px, invented words), SHA-256 `c7d67a400423f94d2ec3b7617b98f9729b181b4f03cf3e6d37012416889e84f4`; it must never be re-read as a table.

`upside_down_table.png` is a generated 12-row, 5-column table page (invented item names and amounts) turned 180 degrees, 1275 x 1650 px at 150 dpi, SHA-256 `8243e7701e46ccc534c298bab12f1ce4ecaaf3a1ca801a73a5c7392711c868e0`. The geometry test admits it to a second read, and that read finds no table, so the first read must stand.
