# PDF with a title over a noisy photograph

`photo-title.pdf` follows the synthetic #2312 generator. It places four
blended scales of seeded RGB noise beneath a native Helvetica title on a
612 × 792 point page. It uses JPEG quality 43 to keep the fixture compact.
The only text operation paints the literal `SECTION TITLE`. SHA-256 is
`d9c21380343a4c9d512eb590f67958e3ff16098edf18b9d9b9f8e84a174639b0`.

The image contains no source text. On the built-in direct Tesseract route, its
recognized noise is discarded with the explicit quality setting. The
independent OCR-disabled result supplies the native text and structure control.

`same-colour-title.pdf` uses the same source text on a light patch in the
noise raster. The title fill matches the patch. A Poppler 100 DPI render is
byte-identical after suppressing the title's sole show operation. SHA-256 is
`3ff8d471039963ace93f29358ca95f5ed289e944a3336e6c13a9102bb685b5f9`.
