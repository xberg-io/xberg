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

`scanned-english-khmer-overlay.pdf` is the scanned page with a scaled-down copy of the Khmer page drawn over its lower right (the stamp is mirrored by the overlay matrix), so the text layer is Khmer only and the English body exists only in the picture.

`khmer-text-with-figure.pdf` is `cluster-complete-mapped.pdf` with its text drawn twice (about 440 readable glyphs) and the scanned English page scaled to 60% over it as a figure that covers about 36% of the page.

The short fully mapped page is derived with `python3 -I derive_fully_mapped.py cluster-complete-mapped.pdf khmer-short-fully-mapped.pdf`: it keeps the first 14 glyphs and maps each continuation glyph to U+200B, so the layer has no U+FFFD marker.

`khmer-among-english-pages.pdf` is a readable English text page, the Khmer page, the same English page and the scanned English page, so the text layer passes the quality check on pages 1 and 3 and only pages 2 and 4 are sent to OCR. Page 2 is the only one a first-stage Khmer model claims:

```sh
python3 -I english_page.py english.pdf
qpdf --empty --pages english.pdf 1 cluster-complete-mapped.pdf 1 english.pdf 1 khmer-then-scanned-english.pdf 2 -- khmer-among-english-pages.pdf
```

`echoed-khmer.pdf` is a Khmer page whose font has no `/ToUnicode` map and whose content stream shows the code points of real Khmer text, so the decoder echoes each code and every span has fallback provenance (144 Khmer characters, valid clusters, no U+FFFD). The provenance check flags the page for OCR, which must win over the script rule that would keep it native:

```sh
python3 -I derive_echoed_khmer.py cluster-complete-mapped.pdf echoed-khmer.pdf
```
