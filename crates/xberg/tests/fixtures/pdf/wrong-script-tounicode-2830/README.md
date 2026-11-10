One-page PDFs in an embedded CIDFontType2 subset of DejaVu Sans (no `cmap` or `post`, Identity-H codes equal to glyph IDs) with one glyph missing from its `ToUnicode` map, so each layer holds a lost-glyph marker. `arabic.pdf` and `bengali.pdf` print English while their `ToUnicode` maps send each Latin letter to an Arabic or Bengali code point; `genuine.pdf` prints real Arabic with a correct map.

```sh
for v in arabic bengali genuine; do python3 -I make_cid_page.py $v $v.pdf /path/to/DejaVuSans.ttf; done
```

Needs fontTools. DejaVu Sans is redistributed under `crates/xberg-native-pdf/src/fonts/assets/LICENSE-DejaVu`.
