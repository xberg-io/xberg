"""Write the synthetic #3352 ruled non-table control.

Two page regions that a rule count must not turn into tables:

1. Three-column justified prose inside a painted box (top and bottom rules span
   the text, so the ruled branch of the borderless gate applies).
2. A chart-like block of short painted axis ticks and numeric labels; its rules
   are shorter than the text, so they are not the region's own.
"""

from __future__ import annotations

import gzip
from pathlib import Path

PAGE_HEIGHT = 842

PROSE = (
    ("The", "quick", "brown"),
    ("fox", "jumps", "over"),
    ("the", "lazy", "dog"),
    ("and", "then", "continues"),
    ("across", "the", "green"),
    ("field", "toward", "hills"),
)
COLUMNS = (60, 220, 380)

TICKS = 14

TICKS = 14

SHORT_WORDS = (
    ("one", "two", "three"),
    ("four", "five", "six"),
    ("ace", "bee", "sea"),
    ("red", "sky", "sun"),
)


def text(x: int, y: int, value: str) -> str:
    return f"0 g BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({value}) Tj ET"


def bar(x: int, y: int, width: int, height: int = 1, grey: str = "0.6") -> str:
    return f"{grey} g {x} {y} {width} {height} re f"


def boxed_prose(commands: list[str], top: int) -> None:
    commands.append(bar(50, top + 6, 470, grey="0.6"))
    for index, words in enumerate(PROSE):
        y = top - 12 * index
        for x, word in zip(COLUMNS, words, strict=True):
            commands.append(text(x, y, word))
    # Two full-width rules whose centres lie inside the prose text band, not only
    # above and below it: the candidate must reach the ruled branch of the
    # borderless gate and still be refused by the structural prose guards. ~keep
    commands.append(bar(50, top - 18, 470, grey="0.6"))
    commands.append(bar(50, top - 42, 470, grey="0.6"))
    bottom = top - 12 * len(PROSE)
    commands.append(bar(50, bottom - 6, 470, grey="0.6"))
    commands.append(bar(50, bottom - 6, 1, top + 7 - bottom, grey="0.6"))
    commands.append(bar(519, bottom - 6, 1, top + 7 - bottom, grey="0.6"))


def axis_block(commands: list[str], top: int) -> None:
    for index in range(TICKS):
        y = top - 10 * index
        commands.append(bar(100, y, 50))
        commands.append(text(160, y + 2, f"{index * 10}"))
    for index in range(8):
        x = 100 + 7 * index
        commands.append(bar(x, top - 10 * (TICKS - 1), 1, 10 * (TICKS - 1)))


def pdf(commands: str) -> bytes:
    stream = commands.encode("ascii")
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources "
            b"<< /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        f"<< /Length {len(stream)} >>\nstream\n".encode() + stream + b"\nendstream",
    )
    raw = bytearray(b"%PDF-1.4\n")
    offsets = []
    for index, obj in enumerate(objects, 1):
        offsets.append(len(raw))
        raw.extend(f"{index} 0 obj\n".encode() + obj + b"\nendobj\n")
    start = len(raw)
    raw.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode())
    for offset in offsets:
        raw.extend(f"{offset:010d} 00000 n \n".encode())
    raw.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n".encode())
    return bytes(raw)


def main() -> None:
    commands: list[str] = []
    boxed_prose(commands, 760)
    axis_block(commands, 560)
    folder = Path(__file__).resolve().parent
    compressed = bytearray(gzip.compress(pdf("\n".join(commands)), compresslevel=6, mtime=0))
    # Python writes OS 255; the committed file carries `gzip -n` output with OS 3. ~keep
    compressed[9] = 3
    (folder / "boxed-prose-and-axis.pdf.gz").write_bytes(compressed)


if __name__ == "__main__":
    main()
