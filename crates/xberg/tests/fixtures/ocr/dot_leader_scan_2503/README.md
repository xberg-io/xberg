The reporter's synthetic tinted dot-leader PDF contains one 1700×2200
ICC RGB raster at 200 DPI. QPDF 12 losslessly compressed the original
unfiltered image with `--stream-data=compress --object-streams=disable`.
The decoded image SHA-256 stayed
`b1a58efafc22bcb0b694aa427726b6449b87b29738a89499dd1a2ba0fa444e6a`.
The decoded page operator SHA-256 stayed
`35aac8acdfce35da0c488e262235dd084c0faba86c2c0e735ee1aaeed1bcac59`.
The test also accepts the exact original file via `XBERG_2503_SCAN`.

`scan-200-dashes.pdf` is the same image with each leader dot redrawn as a
six-by-two pixel dash (127 dots in the 12 leader runs), wrapped by a minimal
uncompressed-structure PDF writer. It keeps the layout and tint, and the dash
aspect ratio fails the dot proof.

`scan-150-plain.pdf.gz` contains a synthetic 150-DPI scan from the #2432
numeric-table generator. It has 12 invented municipal budget rows, aligned
DejaVu Sans columns, and dot leaders on a white background. Its 1275×1650
RGB page is stored as an unfiltered PDF image. The decompressed PDF has
SHA-256 `9f427df6284fe295ea131e46211844b24b0a8693b2c36da1ce004439148abc6c`.

`scan-150-shifted-count-header.pdf.gz` preserves the source scan except for
the Count heading. Replace the pixels at x=[360,419), y=[138,166) with white,
then paste the captured rectangle at x=303, y=138. The shift leaves the
recognized lines intact and removes alignment with the Count values.
The decompressed control has SHA-256
`5d6a45358a6824f9003ead2d6bb0aed701e98a4e72826511848c38819bc56b60`.

Both files use `gzip -n -9` to retain the exact PDF bytes without storing
the unfiltered image streams in Git.
