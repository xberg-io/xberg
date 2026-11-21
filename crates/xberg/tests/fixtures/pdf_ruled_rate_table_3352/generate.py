"""Write the synthetic #3352 page: a ruled rate table whose rules are painted bars.

The real page (ParseBench record `000999c16d74b494e344`) draws its row rules as
1 pt filled rectangles, not stroked segments. Counting only stroked lines makes
the table look rule-less, and the columnar-prose guard then drops it because its
numeric columns all have the same width.
"""

from __future__ import annotations

import gzip
from pathlib import Path

PAGE_HEIGHT = 842
LEFT = 60
RIGHT = 520
EDGES = (60, 175, 290, 405, 520)

HEADER = ("Attained Age", "Non-Tobacco", "Tobacco")
SUBHEADER = ("", "Female", "Male", "Female", "Male")
ROWS = tuple(
    (
        f"{age}",
        f"{2_797 + (age - 65) * 13:,}",
        f"{3_214 + (age - 65) * 13:,}",
        f"{3_108 + (age - 65) * 13:,}",
        f"{3_574 + (age - 65) * 13:,}",
    )
    for age in range(65, 96)
)


def text(x: int, y: int, value: str) -> str:
    return f"0 g BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({value}) Tj ET"


def bar(y: int) -> str:
    """A rule drawn as a filled, unstroked 1 pt-high rectangle."""
    return f"0.6 g {LEFT} {y} {RIGHT - LEFT} 1 re f"


def page() -> str:
    commands = ["0.6 g"]
    top = 700
    commands.append(bar(top))
    y = top - 8
    commands.append(text(70, y, HEADER[0]))
    for index, cell in enumerate(HEADER[1:]):
        commands.append(text(185 + index * 115, y, cell))
    commands.append(bar(y - 3))
    y -= 14
    for index, cell in enumerate(SUBHEADER[1:]):
        commands.append(text(EDGES[index + 1] + 10, y, cell))
    commands.append(bar(y - 3))
    y -= 12
    for row in ROWS:
        for index, cell in enumerate(row):
            commands.append(text(EDGES[index] + 10, y, cell))
        commands.append(bar(y - 3))
        y -= 10
    commands.append(bar(y - 3))
    commands.append(text(60, y - 24, "There is no modal loading."))
    return "\n".join(commands)


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
    folder = Path(__file__).resolve().parent
    compressed = bytearray(gzip.compress(pdf(page()), compresslevel=6, mtime=0))
    # Python writes OS 255; the committed file carries `gzip -n` output with OS 3. ~keep
    compressed[9] = 3
    (folder / "painted-rules.pdf.gz").write_bytes(compressed)


if __name__ == "__main__":
    main()
