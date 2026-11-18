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


The companion integration test uses the corpus-pinned Docling scan at
`pdf_scanned/docling_scanned.pdf`. On page 2, the list item ends with
“struc-” on a full-width line and “tures” alone on its final line. The same
page independently spells “structures”. Sparse OCR puts the final fragment in
a separate block. The join checks the near-aligned left edges and gap, a
detached list marker, and surrounding body lines that corroborate the lane.

This scanned-text path compares lowercase Unicode alphabetic spellings within
the selected Tesseract hOCR page. The word boxes can confirm that an
unbroken token exists independently of the two fragments. The comparison
retains accents and uses the ASCII hyphen at the line break. Native PDF text
has document-wide segments and uses a broader compound collector. The scanned
path leaves cross-page spellings, ambiguous word boundaries, and soft hyphens
unchanged.
