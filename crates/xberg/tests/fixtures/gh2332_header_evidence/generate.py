"""Write the deterministic GH#2332 header-evidence fixtures next to this script.

``retained-role-tables.json`` holds two retained grids over identical dense cells.
One marks row 0 as a header, one does not, so the renderers can be checked against the
source role record instead of the dense ``columns`` projection.

``uniform-rules-body-labels.pdf.gz`` is one ruled grid whose first band carries labels
but no header evidence: no shading, no bold, no separating rule weight, no tagged
structure. HTML reports every cell as body; the ignored test pins that Markdown must
agree once the native detector records the abstention.
"""

import gzip
import json
from pathlib import Path

ROWS = (
    ("Region", "Code", "Rate", "Note"),
    ("North", "A1", "0.924", "base"),
    ("East", "B2", "0.930", "peak"),
    ("South", "C3", "0.937", "base"),
)
EDGES = (40, 120, 200, 280, 360)
TOP = 260
HEIGHT = 24


def pdf_body() -> bytes:
    commands = ["0.5 G 0.7 w"]
    for i, cells in enumerate(ROWS):
        y = TOP - HEIGHT * (i + 1)
        for x, cell in zip(EDGES[:-1], cells, strict=True):
            commands.append(f"0 g BT /F1 9 Tf 1 0 0 1 {x + 8} {y + 8} Tm ({cell}) Tj ET")
    bottom = TOP - HEIGHT * len(ROWS)
    for x in EDGES:
        commands.append(f"{x} {bottom} m {x} {TOP} l S")
    for y in range(bottom, TOP + 1, HEIGHT):
        commands.append(f"{EDGES[0]} {y} m {EDGES[-1]} {y} l S")
    return "\n".join(commands).encode("ascii")


def pdf(stream: bytes) -> bytes:
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Resources "
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


def retained_role_tables() -> str:
    """Two retained grids over the same dense cells: one marks row 0 a header, one does not."""
    columns = ["Region", "Code", "Rate", "Note"]
    records = [
        ["North", "A1", "0.924", "base"],
        ["East", "B2", "0.930", "peak"],
    ]
    cells = [columns, *records]

    def grid(header: bool) -> dict:
        entries = []
        for row, values in enumerate(cells):
            for col, value in enumerate(values):
                entries.append(
                    {
                        "row": row,
                        "col": col,
                        "row_span": 1,
                        "col_span": 1,
                        "is_header": header and row == 0,
                        "content": value,
                    }
                )
        return {"rows": len(cells), "cols": len(columns), "cells": entries}

    def table() -> dict:
        return {
            "cells": cells,
            "markdown": "",
            "page_number": 1,
            "columns": columns,
        }

    return (
        json.dumps(
            {
                "header_roles": {"table": table(), "grid": grid(True)},
                "no_header_roles": {"table": table(), "grid": grid(False)},
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )


def main() -> None:
    folder = Path(__file__).resolve().parent
    (folder / "retained-role-tables.json").write_text(retained_role_tables(), encoding="utf-8", newline="\n")
    with (
        (folder / "uniform-rules-body-labels.pdf.gz").open("wb") as handle,
        gzip.GzipFile(fileobj=handle, mode="wb", mtime=0) as stream,
    ):
        stream.write(pdf(pdf_body()))


if __name__ == "__main__":
    main()
