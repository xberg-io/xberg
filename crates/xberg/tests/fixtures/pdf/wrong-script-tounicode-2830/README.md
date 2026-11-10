One-page PDFs in an embedded CIDFontType2 subset of DejaVu Sans (no `cmap` or `post`, Identity-H codes equal to glyph IDs) with one glyph missing from its `ToUnicode` map, so each layer holds a lost-glyph marker. `arabic.pdf` and `bengali.pdf` print English while their `ToUnicode` maps send each Latin letter to an Arabic or Bengali code point; `genuine.pdf` prints real Arabic with a correct map.

```sh
for v in arabic bengali genuine hebrew bilingual; do python3 -I make_cid_page.py $v $v.pdf /path/to/DejaVuSans.ttf; done
```

Needs fontTools. DejaVu Sans is redistributed under `crates/xberg-native-pdf/src/fonts/assets/LICENSE-DejaVu`.

`hebrew.pdf` (real Hebrew) and `bilingual.pdf` (real Arabic paragraphs and one English paragraph) are controls whose ink matches their layer. The two composites are built with qpdf: `arabic-then-scanned-english.pdf` is `arabic.pdf` page 1 and `test_documents/pdf_scanned/memo_prose_scanned.pdf` page 1, and `arabic-and-bengali.pdf` is page 1 of each of the two wrong-script pages.

```sh
qpdf --empty --pages arabic.pdf 1 memo_prose_scanned.pdf 1 -- arabic-then-scanned-english.pdf
qpdf --empty --pages arabic.pdf 1 bengali.pdf 1 -- arabic-and-bengali.pdf
```
