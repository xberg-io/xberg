# Ligature advance regression (#2322)

Each PDF has seven lines of neutral text in an Identity-H CIDFontType2 subset of the bundled
`src/fonts/assets/DejaVuSans.ttf` (license `src/fonts/assets/LICENSE-DejaVu`), with its GSUB `liga`
feature kept except in `accent` and `fewer`. The first three lines are one show string each. The next three
are TJ arrays in which a string ends after each ligature, the shape that adds a blank gap after the ligature.
The seventh repeats the second TJ line with the size in the text matrix, 90 percent horizontal scaling and
character spacing.

Run `python3 generate.py OUTPUT.pdf MODE` with fonttools 4.68.0 from `uv.lock`. The modes are:

- `direct`: ligature codes (ff, fi, fl, ffi), no font cmap. The CID route paints it: the reference raster.
- `letters`: the same codes and a full font cmap. ToUnicode maps each ligature code to its letters.
- `codepoint`: the same as `letters`, but ToUnicode maps each ligature code to its U+FB0x code point.
- `plain`: separate letter codes, a full font cmap. The control for the Unicode shaping route.
- `plain_direct`: the same codes as `plain`, no font cmap.
- `accent`: the same codes as `plain`. ToUnicode maps the code of `e` to U+0065 U+0301, a letter and a combining
  accent. No ligature code is present, and the font has no GSUB ligatures, so a run without `e` shapes to the
  glyphs the CID route paints.
- `fewer`: the same codes as `plain`. The font cmap maps U+0001 to the glyph of `e` and ToUnicode lacks the code of
  `e`, so the reverse cmap decodes it to a control character that the decoder drops: one code, no characters.
  The font has no GSUB ligatures, as in `accent`.

| File | SHA-256 |
| --- | --- |
| direct.pdf | ee38d165db188ffaa9a1f6e846b6d76488cf54474b3edf0b6004f35abb06fa8b |
| letters.pdf | b16844ccfa86668ee216faad3438d9b12bbf192ebd176d1a816df26c3941d037 |
| codepoint.pdf | 942e329df0e11268ec853cc3ce67278acbacd20e17667f18ff0892d81aa375c3 |
| plain.pdf | 805e8011e94cb855002714a913b14964c3411d4786d65f2405521a12e59730f8 |
| plain_direct.pdf | 3b2dc89dfd042fe4024d34496d25c0ce59d325f9865b7ecff3c87ba876c6b069 |
| accent.pdf | 60b0fe61008c793fbe827721e188fdeec15caa3109296fc931b3946ae2fb3829 |
| fewer.pdf | 409e1f4b60276d926e6bbbd0ef0b1d398b34b98574b3c25a189b934e4aa105e9 |
