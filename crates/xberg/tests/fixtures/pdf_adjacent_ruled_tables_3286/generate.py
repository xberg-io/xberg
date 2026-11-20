"""Write the three gzip-compressed #3286 ruled-grid PDFs next to this script."""

import gzip
from pathlib import Path

TOP = (
    ("Service", "Zone", "Period", "Capacity"),
    ("Iris", "North", "Winter", "350"),
    ("Hazel", "East", "Autumn", "470"),
    ("Linden", "South", "Summer", "590"),
)
BOTTOM = (
    ("Review", "Cadence"),
    ("Clock", "Week one"),
    ("Signal", "Week two"),
    ("Meter", "Week three"),
    ("Beacon", "Week four"),
)


def grid(rows: tuple[tuple[str, ...], ...], top: int) -> str:
    edges = (55, 166, 300, 398, 525) if len(rows[0]) == 4 else (55, 221, 525)
    height = 26
    commands = []
    for i, cells in enumerate(rows):
        y = top - height * (i + 1)
        fill = "0.78 0.84 0.92" if i == 0 else "0.95 0.96 0.97" if i % 2 else "1 1 1"
        commands.append(f"{fill} rg 55 {y} 470 {height} re f")
        for x, cell in zip(edges[:-1], cells, strict=True):
            font = "F2" if i == 0 else "F1"
            commands.append(f"0 g BT /{font} 10 Tf 1 0 0 1 {x + 9} {y + 9} Tm ({cell}) Tj ET")
    bottom = top - height * len(rows)
    commands.append("0 G 0.8 w")
    for x in edges:
        commands.append(f"{x} {bottom} m {x} {top} l S")
    for y in range(bottom, top + 1, height):
        commands.append(f"55 {y} m 525 {y} l S")
    return "\n".join(commands)


def pdf(commands: str) -> bytes:
    stream = commands.encode("ascii")
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources "
            b"<< /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>",
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


def fixtures() -> dict[str, bytes]:
    heading = "0 g BT /F2 13 Tf 1 0 0 1 55 602 Tm (Equipment Reference) Tj ET"
    top, bottom = grid(TOP, 560), grid(BOTTOM, 449)
    continuous = grid((*TOP, ("Juniper", "West", "Spring", "610"), ("Aster", "Central", "Year", "730")), 560)
    return {
        "two-grids": pdf("\n".join((heading, top, bottom))),
        "reverse-paint": pdf("\n".join((heading, bottom, top))),
        "continuous-grid": pdf("\n".join((heading, continuous))),
    }


def main() -> None:
    folder = Path(__file__).resolve().parent
    for name, raw in fixtures().items():
        compressed = bytearray(gzip.compress(raw, compresslevel=6, mtime=0))
        # Python writes OS 255; the committed files carry `gzip -n` output with OS 3 (Unix). ~keep
        compressed[9] = 3
        (folder / f"{name}.pdf.gz").write_bytes(compressed)


if __name__ == "__main__":
    main()
