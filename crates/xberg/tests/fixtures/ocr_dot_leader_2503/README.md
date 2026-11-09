These two synthetic PNGs and their ground-truth rows are from the public #2432
reporter's Pillow / DejaVu Sans recipe, using the plain 14 and 20 pixel inputs
with dot leaders. The TSV values are the generator inputs, independently of OCR.

- `14.png` SHA-256 `89d90ef545561abf853817691e3082ee27e37cc72876a3419c37125371b6ace5`
- `20.png` SHA-256 `f1321bb2850e33e519ed02a8682df6b1d74c02106e256e52ae544361add769f2`

`two_column.png` is a generated two-column justified prose page (Pillow, DejaVu Sans 20 px, invented words), SHA-256 `c7d67a400423f94d2ec3b7617b98f9729b181b4f03cf3e6d37012416889e84f4`; it must never be re-read as a table.

`upside_down_table.png` is a generated 12-row, 5-column table page (invented item names and amounts) turned 180 degrees, 1275 x 1650 px at 150 dpi, SHA-256 `8243e7701e46ccc534c298bab12f1ce4ecaaf3a1ca801a73a5c7392711c868e0`. The geometry test admits it to a second read, and that read finds no table, so the first read must stand.
