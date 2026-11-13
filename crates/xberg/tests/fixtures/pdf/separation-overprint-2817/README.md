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
