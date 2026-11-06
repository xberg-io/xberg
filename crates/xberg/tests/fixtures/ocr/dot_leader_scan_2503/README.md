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
