# Scanned lexical hyphen regression

The fixture is a synthetic, image-only PDF. Its 875 × 253 grayscale image
fills one 96 DPI page. The first physical line ends in “open-” and the
second begins with “source”. The remaining lines show the closed spelling
“opensource” and the hyphenated spelling “open-source”. Xberg's static
compound list does not include this pair. The PDF contains no source text layer.

Regenerate from the repository root with Python 3.13, Pillow 11.3.0, and the
repository's DejaVu Sans font.

```sh
uv run --no-project --python 3.13 --with pillow==11.3.0 python \
  fixtures/repros/scanned-dehyphenation/generate.py \
  fixtures/repros/scanned-dehyphenation/lexical-wrap.pdf \
  crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf
```
