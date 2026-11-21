"""Write the deterministic GH#2332 table-boundary PDFs next to this script.

Two unambiguous boundaries:

- ``side-by-side-gutter``: two independent ruled grids separated by a wide blank
  gutter that no rule crosses, so they are two source tables.
- ``continuous-grid-blank-band``: one ruled grid whose outer vertical rules and
  horizontal row rules run through an empty middle band, so it is one source
  table even though the text has a large gap there.
"""

import gzip
from pathlib import Path

LEFT = (
    ("Service", "Zone", "Capacity"),
    ("Iris", "North", "350"),
    ("Hazel", "East", "470"),
    ("Linden", "South", "590"),
)
RIGHT = (
    ("Review", "Cadence", "Owner"),
    ("Clock", "Week one", "Ada"),
    ("Signal", "Week two", "Bo"),
    ("Meter", "Week three", "Cy"),
)
BANDED = (
    ("Service", "Zone", "Capacity"),
    ("Iris", "North", "350"),
    ("Hazel", "East", "470"),
    ("", "", ""),
    ("", "", ""),
    ("Linden", "South", "590"),
    ("Aster", "West", "610"),
)


def grid(rows, left, top, edges, height=26):
    width = edges[-1] - left
    commands = []
    for i, cells in enumerate(rows):
        y = top - height * (i + 1)
        fill = "0.78 0.84 0.92" if i == 0 else "0.95 0.96 0.97" if i % 2 else "1 1 1"
        commands.append(f"{fill} rg {left} {y} {width} {height} re f")
        for x, cell in zip(edges[:-1], cells, strict=True):
            if not cell:
                continue
            font = "F2" if i == 0 else "F1"
            commands.append(f"0 g BT /{font} 10 Tf 1 0 0 1 {x + 9} {y + 9} Tm ({cell}) Tj ET")
    bottom = top - height * len(rows)
    commands.append("0 G 0.8 w")
    for x in edges:
        commands.append(f"{x} {bottom} m {x} {top} l S")
    for y in range(bottom, top + 1, height):
        commands.append(f"{left} {y} m {edges[-1]} {y} l S")
    return "\n".join(commands)


def pdf(commands):
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


def fixtures():
    heading = "0 g BT /F2 12 Tf 1 0 0 1 55 720 Tm (Schedule) Tj ET"
    left = grid(LEFT, 55, 680, (55, 140, 215, 285))
    right = grid(RIGHT, 345, 680, (345, 430, 505, 575))
    banded = grid(BANDED, 55, 680, (55, 140, 215, 285))
    return {
        "side-by-side-gutter": pdf("\n".join((heading, left, right))),
        "continuous-grid-blank-band": pdf("\n".join((heading, banded))),
    }


def main():
    folder = Path(__file__).resolve().parent
    for name, raw in fixtures().items():
        with (
            (folder / f"{name}.pdf.gz").open("wb") as handle,
            gzip.GzipFile(fileobj=handle, mode="wb", mtime=0) as stream,
        ):
            stream.write(raw)


if __name__ == "__main__":
    main()
