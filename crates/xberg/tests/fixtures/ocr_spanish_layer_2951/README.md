# Spanish OCR layer fixture

`spanish.pdf` is invented text from the [public #2951 reproducer](https://github.com/xberg-io/xberg/issues/2951#issuecomment-6787752220).
It was generated in Spanish mode with fontTools 4.63.0 and the repository's
`crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf`. The Type0 header maps
to the Private Use Area and triggers automatic OCR. The Helvetica/WinAnsi body
contains the accented layer words. Inspect it with `pdftotext -raw spanish.pdf -`.

`position-control.pdf` appends a visible Helvetica span `Control visible ano`
below the body. `invisible-control.pdf` adds the native-only word
`invisibilización` in PDF text rendering mode 3. Both were derived by
appending these operators to object 8's content stream and rebuilding the xref.
The two controls rasterize identically at 72 DPI with Poppler. Native text
extraction exposes the extra word only in the invisible control.

The reporter replay asserts the complete body, including words the historical
read split or shortened and lines the original report says were absent.
Capture the actual base OCR words and positions under the matched configuration
before attributing a transcript failure to any of those mechanisms. The stored
warmed baseline already differs from the original report on which lines survive.

The replay checks the full body on both public text surfaces for all three
fixtures. It also checks that native extraction sees the hidden word and the
painting-only transcript excludes it. On the two controls, the OCR word boxes
are converted from processed pixels to top-left page points. The body `año`
is near (109, 383), and the separate `ano` is near (175, 583). The windows
allow for differences in glyph fitting while keeping the two occurrences
disjoint. Both controls must recover the painted body and preserve the
separate spelling. The invisible word must stay absent from text and OCR
elements.
