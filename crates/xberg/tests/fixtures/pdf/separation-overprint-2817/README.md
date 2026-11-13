# Separation Black overprint regression

The two PDFs use the synthetic library-planning text and embedded CIDFont
construction from the [public #2817 reproducer][issue]. The body uses a
Separation Black paint with an intentionally incomplete ToUnicode map. The
overprint variant selects the graphics state with `/op true` and `/OPM 1`.
The control omits that selection. Both include a synthetic native second page.

The archived fixtures were generated with FontTools 4.60.1 and the
[DejaVu Sans font bundled by Matplotlib v3.10.7][font]. FontTools timestamp
recalculation was disabled. See [the DejaVu license](DEJAVU-LICENSE).

- `sepop-mixed.pdf` SHA-256 `9d8ab4769237e62dcdfc72bd93c56ecbbbac73534b9421113f13d4a06b477c9a`
- `separation-no-overprint-mixed.pdf` SHA-256 `50672da3d74946b8664240eb141f677402bdc8d976e52f6e10ea86dc8e98f65c`
- `sepop.ocr.txt` is the independent English transcript from a 150 DPI Poppler
  render using Tesseract 5.5.4. Its 219 tokens also match the synthetic source prose.

[issue]: https://github.com/xberg-io/xberg/issues/2817#issuecomment-6761729366
[font]: https://github.com/matplotlib/matplotlib/blob/v3.10.7/lib/matplotlib/mpl-data/fonts/ttf/DejaVuSans.ttf

## Single-page reporter folio

The three folio PDFs come from the [follow-up public synthetic recipe][folio].
The generator block ends after writing the PDF to a buffer. The fixture
generation added only the final write to the output path. The body is the same
216-token synthetic prose already tracked in `sepop.ocr.txt`, followed by the
native `Page 12` folio. The fixtures contain no material from the private PDF
mentioned in the issue.

FontTools 4.60.1 used the repository's DejaVu Sans font at Git blob
`5789a29d6552ed8c7939190ba8899d748da7ea4b`. The embedded font program omits
the cmap and post tables as specified by the recipe. Its DejaVu name and
[license notice](DEJAVU-LICENSE) cover redistribution with these fixtures.

- `folio-sepop.pdf` SHA-256 `f2e5c1d73cd8467cf218d42ff0938e0f6031fc32c1a325a09c0a3c053d931bb6`
- `folio-black.pdf` SHA-256 `d480c5b265b1f153105997645c72db2b02c0d457357e226fcc601537d1041bd2`
- `folio-mapped.pdf` SHA-256 `63fda270a0516617ef3cfaf4c19e7b5e7008d3114e087b2b75307bb040e8230f`

[folio]: https://github.com/xberg-io/xberg/issues/2817#issuecomment-6845799306

`folio-mixed-size.pdf` is derived from `folio-black.pdf` by changing the first
body line containing “quiet queue” from 10 pt to 8 pt and restoring 10 pt for
the next line. The font program, ToUnicode map, and expected transcript are
the same.

- SHA-256 `c00fd330a01fd657c0aa3eacc93b15e6f824f99df33e887b645a06e1b95a7a14`
