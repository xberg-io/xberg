# Synthetic standalone photo-title controls

All five images are generated RGB artwork with no customer source. They use
a 1700 × 1100 synthetic background from Python's Random seed 7. Random RGB
tiles at scales 1, 4, 16 and 64 are resized to the canvas and successively
blended at 50%. The outlined title is drawn at (200, 400) with Liberation Sans
Bold 150, white fill and a four-pixel black stroke.

- title.png keeps the title pixels and repeats a 96 × 96 background tile.
- table.png uses a 128 × 128 tile. The primary read accepts two tables here
  even though the title pixels are identical.
- mixed-sign.png keeps the title pixels and adds a separate sign containing
  NORTH GATE 42 and AUTHORIZED ONLY. The remaining background is quantized
  to grayscale multiples of 32, or 12 inside the title's 400-pixel band.
- partial.png and numeric.png use an outlined NORTH GATE and NORTH GATE 42
  over a dark placard. Their backgrounds repeat 108 × 108 and 64 × 64 tiles
  respectively. The original OCR recognizes NORTH on the first and all three
  tokens on the second.

The primary output assertions use bundled English Tesseract. The tests check
the actual 1700 × 1100 processed frame, sparse PSM and original word counts.
