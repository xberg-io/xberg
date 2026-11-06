# Source number in a Khmer scan

Both synthetic pages contain the same 150 DPI scan. The scan draws the word
`ប០ប` followed by six independent English prose lines. An invisible text
show at the source word's position reports `ប០ប` in `matching.pdf` and
`ប១ប` in `misleading.pdf`. At 150 DPI, Poppler emits identical grayscale
PGM bytes for both pages, SHA-256
`70fcfabe3153e465c58e0642dd31337f776b133bf8abb87c20eeebac16102241`.

Regenerate using ReportLab and Poppler with the hinted regular TTF from
[Noto Sans Khmer v2.004](https://github.com/notofonts/khmer/releases/tag/NotoSansKhmer-v2.004).
The generator checks its exact SHA-256. The embedded font is covered by
[NotoSansKhmer-OFL.txt](NotoSansKhmer-OFL.txt).
