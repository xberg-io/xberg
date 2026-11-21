"""Write the synthetic #3351 page: one borderless two-column definition grid.

The real page (ParseBench record `0580ae4eb20f312e0b8b`) carries several
`Factor`/value grids like this one. Their values are single letters and the last
value is `=`, so the value column reads as a sentence continuation and the
whole grid is lost as prose. There are no rules anywhere on the page.

The fused side-by-side band the same page presents is covered by the
`definition_grid` unit tests, which can place cells at exact coordinates.
"""

from __future__ import annotations

import gzip
from pathlib import Path

PAGE_HEIGHT = 842

GRID = (
    ("Factor", ""),
    ("MeanDebt", "XX"),
    ("Credit", "x"),
    ("UWTier", "x"),
    ("FDLFact", "x"),
    ("UnityOne", "1"),
    ("Install", "x"),
    ("ISITier", "x"),
    ("Downpay", "x"),
    ("DebtFact", "="),
)

PROSE = (
    "There is no modal loading and no separate policy fee.",
    "The rates above apply to the standard plan with no riders.",
)


def text(x: int, y: int, value: str, bold: bool = False) -> str:
    font = "F2" if bold else "F1"
    return f"0 g BT /{font} 8 Tf 1 0 0 1 {x} {y} Tm ({value}) Tj ET"


def grid(commands: list[str], left: int, top: int) -> None:
    for row, (label, value) in enumerate(GRID):
        y = top - 12 * row
        commands.append(text(left, y, label, bold=row == 0))
        if value:
            commands.append(text(left + 70, y, value))


def pdf(commands: str) -> bytes:
    stream = commands.encode("ascii")
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources "
            b"<< /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold >>",
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
    commands: list[str] = [text(45, PAGE_HEIGHT - 60, "Proposed Calculation of Bad Debt Factor", bold=True)]
    grid(commands, 45, PAGE_HEIGHT - 100)
    for index, line in enumerate(PROSE):
        commands.append(text(45, PAGE_HEIGHT - 260 - 14 * index, line))
    raw = pdf("\n".join(commands))
    folder = Path(__file__).resolve().parent
    compressed = bytearray(gzip.compress(raw, compresslevel=6, mtime=0))
    # Python writes OS 255; the committed file carries `gzip -n` output with OS 3. ~keep
    compressed[9] = 3
    (folder / "definition-grid.pdf.gz").write_bytes(compressed)


if __name__ == "__main__":
    main()
