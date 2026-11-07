The two one-page PDFs paint the same Khmer text with the same embedded Noto Sans Khmer subset.
The first maps the first glyph of each HarfBuzz cluster to its complete Unicode text. The
second maps only single-character clusters, so some native text is genuinely missing. Neither
maps the cluster continuation glyphs individually. The embedded font omits `cmap` and `post`
so the native decoder cannot supply additional characters through those fallbacks.

The source font is pinned by SHA-256 in the generator and redistributed under the accompanying
OFL. [Noto Sans Khmer source font](https://github.com/notofonts/noto-fonts/blob/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSansKhmer/NotoSansKhmer-Regular.ttf)

Run the generator with HarfBuzz 15.1.0 and the pinned source font. It prints the expected source
inventory and both PDF hashes.

```sh
python3 generate.py --font /path/to/NotoSansKhmer-Regular.ttf --out .
```

`khmer-then-scanned-english.pdf` is page 1 of `cluster-complete-mapped.pdf` followed by the
scanned English page `test_documents/pdf_scanned/memo_prose_scanned.pdf`:

```sh
qpdf --empty --pages cluster-complete-mapped.pdf 1 memo_prose_scanned.pdf 1 -- khmer-then-scanned-english.pdf
```
