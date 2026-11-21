"""Synthetic pages for xberg-io/xberg#3348.

The pinned records are ParseBench `0dc78d10374b01a67aee`
(`FBLB-134215544_page88`, PDF SHA-256
`a2947a029921a904ca829c3c6d322bc1ae4e26328c67f94294e0d9e79aca3976`) and
`078d0b5457f6a070d206` (`SERFF_TX_random_pages_1_page810`), both at ParseBench
revision `2805a1d940f95a203e0ae4b88be9934f7765b3fc`.

Their individual PDF rights are unreviewed, so the fixtures print the same
*shape* at the same geometry instead of copying the pages:

- four ruled tables stacked on one page, each opening with its own full-width
  title band and followed by a fresh header band. The titled rows carry no
  internal vertical rules, so a ruled detector reads each title as one cell
  spanning the table and joins the four tables into one;
- three borderless tables side by side, each opening with a centered title over
  its own block of six columns, at the pinned page's own column positions.

Two negative controls must not split: a spanning row inside a body, and a
repeated header band with no title above it.
"""

from __future__ import annotations

import gzip
from pathlib import Path

# --- first record: stacked ruled tables -------------------------------------

PAGE_WIDTH = 612
PAGE_HEIGHT = 792
TABLE_LEFT = 64
TABLE_RIGHT = 474
COLUMN_LEFT = (64, 150, 205, 260, 315, 370, 425, 480, 530)
COLUMN_RULE = (*COLUMN_LEFT, 580)
ROW_PITCH = 6
SECTION_ROWS = (
    (
        "Alpha Construction Factors",
        None,
        ("Dwelling Construction Year", "Peril 1", "Peril 2", "Peril 3", "Peril 4", "Peril 5", "Peril 6", "Peril 7", "Peril 8"),
        (
            ("1950 & Prior", "1.000", "1.000", "1.000", "1.000", "1.000", "1.000", "1.000", "1.000"),
            ("1951-1970", "1.100", "1.000", "1.000", "1.100", "1.000", "1.000", "1.100", "1.100"),
            ("1971-1980", "1.220", "1.000", "1.000", "1.220", "1.000", "1.000", "1.220", "1.220"),
        ),
    ),
    (
        "Beta Services Factor",
        "DW, MH",
        ("Financial Product", "Peril 1", "Peril 2", "Peril 3", "Peril 4", "Peril 5", "Peril 6", "Peril 7", "Peril 8"),
        (
            ("Max Life Policy - FV $10,000", "2.0%", "2.0%", "2.0%", "2.0%", "2.0%", "2.0%", "2.0%", "2.0%"),
            ("Max Life Policy - FV $25,000", "3.0%", "3.0%", "3.0%", "3.0%", "3.0%", "3.0%", "3.0%", "3.0%"),
        ),
    ),
    (
        "Gamma Products",
        None,
        ("Financial Product", "Peril 1", "Peril 2", "Peril 3", "Peril 4", "Peril 5", "Peril 6", "Peril 7", "Peril 8"),
        (
            ("Condo", "0.110", "0.220", "0.330", "0.440", "0.550", "0.660", "0.770", "NA"),
            ("Tenant", "0.125", "0.250", "0.375", "0.500", "0.625", "0.750", "0.875", "NA"),
        ),
    ),
    (
        "Delta Coverage Factors",
        None,
        ("Coverage", "Peril 1", "Peril 2", "Peril 3", "Peril 4", "Peril 5", "Peril 6", "Peril 7", "Peril 8"),
        (
            ("HPP, HPPO", "0.595", "1.000", "0.652", "1.000", "1.000", "1.000", "0.686", "NA"),
            ("MHPP, MHPPO", "0.595", "1.000", "0.652", "1.000", "1.000", "1.000", "0.686", "NA"),
        ),
    ),
)
# --- second record: side-by-side borderless blocks ---------------------------

WIDE_WIDTH = 792
WIDE_HEIGHT = 612
BLOCK_STARTS = (8, 261, 528)
BLOCK_OFFSETS = (0, 44, 84, 128, 164, 206)
BLOCK_TITLES = ("Zips All Others", "Zips 750-753, 760-762", "Zips 770-775")
BLOCK_AGES = ((65, 76), (65, 76), (65, 76))
WIDE_TOP = 612 - 57 - 5


def text(x: float, y: float, value: str, size: float = 5.5) -> str:
    escaped = value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
    return f"0 g BT /F1 {size} Tf 1 0 0 1 {x:.2f} {y:.2f} Tm ({escaped}) Tj ET"


def rule(x0: float, y0: float, x1: float, y1: float) -> str:
    return f"0.4 w 0 g {x0:.2f} {y0:.2f} m {x1:.2f} {y1:.2f} l S"


def section_rows() -> list[tuple[str, ...]]:
    rows: list[tuple[str, ...]] = []
    for title, subtitle, header, body in SECTION_ROWS:
        rows.append((title,))
        if subtitle is not None:
            rows.append((subtitle,))
        rows.extend([header, *body])
    return rows


def titled_sections_page() -> str:
    """Four stacked, titled borderless tables: #3348's first record's shape."""
    rows = section_rows()
    top = PAGE_HEIGHT - 74
    commands: list[str] = []
    y = top
    for row in rows:
        for column, cell in zip(COLUMN_LEFT, row):
            if cell:
                commands.append(text(column + 1.5, y, cell, 5.0))
        y -= 10
    return "\n".join(commands)


def side_by_side_page() -> str:
    commands: list[str] = []
    modes = ("Annual", "Semi-Annual", "Quarterly", "Month Bank Draft")
    for block, (title, start, ages) in enumerate(zip(BLOCK_TITLES, BLOCK_STARTS, BLOCK_AGES)):
        span = BLOCK_OFFSETS[-1] + 42
        commands.append(text(start + span // 2 - len(title) * 1.4, WIDE_TOP, title))
        header_y = WIDE_TOP - 9
        for offset, label in (
            (BLOCK_OFFSETS[0], "Attained"),
            (BLOCK_OFFSETS[2], "Plan F"),
            (BLOCK_OFFSETS[3], "Attained"),
            (BLOCK_OFFSETS[5], "Plan F"),
        ):
            commands.append(text(start + offset + 1, header_y, label))
        for offset, label in zip(BLOCK_OFFSETS, ("Age", "Mode", "9743F", "Age", "Mode", "9743F")):
            commands.append(text(start + offset + 1, header_y - 9, label))
        for row in range(10):
            y = header_y - 20 - row * 9.2
            annual = row % 4 == 0
            age = str(ages[0] + row // 4) if annual else ""
            other = str(ages[1] + row // 4) if annual else ""
            mode = modes[row % 4]
            base = 4000 + block * 700 + row * 11
            other_base = 6000 + block * 900 + row * 11
            for offset, label in zip(
                BLOCK_OFFSETS,
                (
                    age,
                    mode,
                    f"${base}.00",
                    other,
                    mode,
                    f"${other_base}.00",
                ),
            ):
                if label:
                    commands.append(text(start + offset + 1, y, label))
    return "\n".join(commands)


def single_ruled_page(rows: tuple[tuple[str, ...], ...]) -> str:
    columns = (64, 144, 186, 225, 258)
    top = PAGE_HEIGHT - 74
    commands: list[str] = []
    for index in range(len(rows) + 1):
        y = top - index * ROW_PITCH
        commands.append(rule(columns[0], y, columns[-1], y))
    for index in range(len(rows)):
        y = top - index * ROW_PITCH
        for x in columns:
            commands.append(rule(x, y, x, y - ROW_PITCH))
    y = top - ROW_PITCH + 1.5
    for row in rows:
        for column, cell in zip(columns, row):
            commands.append(text(column + 1.5, y, cell))
        y -= ROW_PITCH
    return "\n".join(commands)


def spanning_row_page() -> str:
    """One ruled table with a full-width subtotal inside its body (no split)."""
    return single_ruled_page(
        (
            ("Factor Schedule",),
            ("Product", "One", "Two", "Three"),
            ("Alpha", "1.000", "2.000", "3.000"),
            ("Beta", "4.000", "5.000", "6.000"),
            ("Subtotal",),
            ("Gamma", "7.000", "8.000", "9.000"),
            ("Delta", "1.100", "2.200", "3.300"),
        )
    )


def repeated_header_page() -> str:
    """One ruled table whose header band repeats mid-body (no split)."""
    return single_ruled_page(
        (
            ("Factor Schedule",),
            ("Product", "One", "Two", "Three"),
            ("Alpha", "1.000", "2.000", "3.000"),
            ("Beta", "4.000", "5.000", "6.000"),
            ("Product", "One", "Two", "Three"),
            ("Gamma", "7.000", "8.000", "9.000"),
            ("Delta", "1.100", "2.200", "3.300"),
        )
    )


def pdf(commands: str, width: int, height: int) -> bytes:
    stream = commands.encode("ascii")
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources "
            "<< /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ).encode(),
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


def write(folder: Path, name: str, commands: str, width: int, height: int) -> None:
    compressed = bytearray(gzip.compress(pdf(commands, width, height), compresslevel=6, mtime=0))
    # Python writes OS 255; the committed file carries `gzip -n` output with OS 3. ~keep
    compressed[9] = 3
    (folder / name).write_bytes(compressed)


def main() -> None:
    folder = Path(__file__).resolve().parent
    write(folder, "stacked-titled.pdf.gz", titled_sections_page(), PAGE_WIDTH, PAGE_HEIGHT)
    write(folder, "side-by-side-titled.pdf.gz", side_by_side_page(), WIDE_WIDTH, WIDE_HEIGHT)
    write(folder, "spanning-body-row.pdf.gz", spanning_row_page(), PAGE_WIDTH, PAGE_HEIGHT)
    write(folder, "repeated-header-band.pdf.gz", repeated_header_page(), PAGE_WIDTH, PAGE_HEIGHT)


if __name__ == "__main__":
    main()
