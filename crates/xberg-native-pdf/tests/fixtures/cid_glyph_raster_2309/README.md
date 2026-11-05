# CID raster regression (#2309)

Adapted from the public `g2309.py` recipe in
[issue #2309](https://github.com/xberg-io/xberg/issues/2309).
Each PDF has the posted 20 source lines (245 words), Identity-H CIDFontType2,
29 embedded DejaVu Sans glyphs, and a 612 by 792 point page. The source font is
`src/fonts/assets/DejaVuSans.ttf`; its license is `src/fonts/assets/LICENSE-DejaVu`.
The generator uses that bundled font rather than a host font, preserves its timestamp,
and uses fonttools 4.68.0 from `uv.lock`.

Run `python3 generate.py OUTPUT.pdf MODE` with that pinned fonttools available.
For the NUL mapping, use `python3 generate.py unicode_nul_mapping.pdf nul`;
the generator rejects Windows-reserved device filenames.
The supported modes are:

- `nul`: the posted two-entry Unicode font cmap and all ToUnicode values U+0000.
- `missing`: the same font and an empty ToUnicode map.
- `partial`: the same font, mapping every character except lowercase `a`. Every source run contains `a`.
- `good`: the same two-entry font cmap and complete correct ToUnicode for the native text control.
- `good-full`: the original complete embedded Unicode cmap and correct ToUnicode for raster and OCR controls without host font fallback.
- `direct`: the same outlines and advances, with no font cmap; the existing direct CID route is the raster oracle.
- `custom`: a full font cmap and a custom encoding mapping codes `1000 + GID` to CID `GID`.
  Its ToUnicode omits `a`; this guards against widening the new override beyond identity encoding.

All seven embedded programs have byte-identical compiled glyph outlines and identical
GID-ordered horizontal metrics. The compiled `glyf` SHA-256 is
`8564ec76447f72d0b715c4e729822e9ed04669c33de02710b24fd0764e17d4a1`.
The bundled input font SHA-256 is
`abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322`.
The fonttools source archive SHA-256 is
`73294e233070f71e2b3653b6bd8564af676d57f26d075761fb06d1ab2e4a7a5d`.

| File | SHA-256 |
| --- | --- |
| unicode_nul_mapping.pdf | df8db9722d21fbccbdea10a91d87d15c5189e520e5629d0c5917cab2acf2588c |
| missing.pdf | 9dc3fd580a22ac35f4616bc43affbf0767a374af4934c72a6021d628b9aee3fa |
| partial.pdf | 1653545e9bc23c8d459737eb84f1d3dfc69d7e6c256aafa0ac0c1e8e4893885a |
| good.pdf | e754acc67cc13002d46885df63a5da145f1abb9bbc3daa1be4d2ff41f3d4c3a6 |
| good_full.pdf | 391f7f2fede79ad81c2642f13b05267187fb026e28eea77714e426f353c7e9ec |
| direct.pdf | 173c809becb717e5a8a0a41cc0b9a2e71610454135647fa6ff6a858cb88a40ad |
| custom.pdf | 129cfb0836d0b9bd97150acd4d37b45a6bd000be9bc8069bec2166ca1fe0b60a |

The raster tests require actual ink in all 20 source line bands and exact decoded-pixel
agreement with the direct reference for lossy identity runs. The good mapping must retain
all 245 ordered source words; the custom encoding must retain its baseline raster.
These are recipe-derived fixtures, not byte-identical copies of an external customer PDF.

The NUL fixture is byte-identical to the output of the posted recipe with the
pinned bundled DejaVu Sans file and its saved font timestamp. The local-file OCR
test uses the same public input and JSON configuration as the CLI. It resolves
English via a subprocess-only `TESSDATA_PREFIX`, with no OCR config override.
