"""Build a synthetic multi-page PDF for #2694 and #2730.

Each page is a 300 dpi CCITT Group 4 scan behind a junk render-mode-3 numeric text layer.
Run: uv run --no-project --isolated --with pillow==11.3.0 python generate.py out.pdf [pages]
The page count is 4 (the default) or 7.
"""

import io
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

PAGE_WIDTH, PAGE_HEIGHT, DPI = 612, 792, 300
RASTER_WIDTH, RASTER_HEIGHT = 2550, 3300
FONT_PIXELS = 64
PAGES = [
    ["Harbor ledger one", "The lighthouse keeper recorded every vessel", "that crossed the northern channel today"],
    ["Harbor ledger two", "Volunteers carried the maps into the room", "and labeled each drawer beside the register"],
    ["Harbor ledger three", "Fishing boats arrived with silver baskets", "while neighbors delivered warm bread"],
    ["Harbor ledger four", "The evening tide rose gently over the dock", "and the lamps were counted twice"],
    ["Harbor ledger five", "Carpenters mended the pier before sunrise", "and stacked the planks beside the shed"],
    ["Harbor ledger six", "Children sketched the gulls above the bridge", "while a ferry crossed toward the island"],
    ["Harbor ledger seven", "Orchard growers weighed their crates today", "then shipped them upriver by barge"],
]
JUNK = "47110 20960 33850 18420 90210 55730 61240 70380"


def group4_strip(scan: Image.Image) -> bytes:
    buffer = io.BytesIO()
    scan.save(buffer, format="TIFF", compression="group4", tiffinfo={278: RASTER_HEIGHT})
    buffer.seek(0)
    with Image.open(buffer) as decoded:
        assert decoded.tag_v2[259] == 4
        (offset,) = decoded.tag_v2[273]
        (length,) = decoded.tag_v2[279]
        assert decoded.tobytes() == scan.tobytes()
    return buffer.getvalue()[offset : offset + length]


def scan_of(lines: list[str]) -> Image.Image:
    page = Image.new("L", (RASTER_WIDTH, RASTER_HEIGHT), 255)
    draw = ImageDraw.Draw(page)
    font = ImageFont.load_default(size=FONT_PIXELS)
    for row, line in enumerate(lines):
        draw.text((300, 400 + row * 180), line, fill=0, font=font)
    return page.point(lambda value: 255 if value >= 128 else 0).convert("1")


def stream(fields: str, data: bytes) -> bytes:
    return f"<< {fields} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"


def main() -> None:
    page_count = int(sys.argv[2]) if len(sys.argv) > 2 else 4
    assert page_count in (4, 7), "the page count is 4 or 7"
    pages = PAGES[:page_count]
    entries: list[bytes] = [b""] * (3 + 3 * len(pages))
    entries[0] = b"<< /Type /Catalog /Pages 2 0 R >>"
    kids = " ".join(f"{4 + 3 * index} 0 R" for index in range(len(pages)))
    entries[1] = f"<< /Type /Pages /Kids [{kids}] /Count {len(pages)} >>".encode()
    entries[2] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
    for index, lines in enumerate(pages):
        page_id, content_id, image_id = 4 + 3 * index, 5 + 3 * index, 6 + 3 * index
        rows = "".join(
            f"1 0 0 1 96 {PAGE_HEIGHT - (400 + row * 180) * 72 / DPI - 20:g} Tm ({JUNK}) Tj\n"
            for row in range(len(lines))
        )
        content = f"q {PAGE_WIDTH} 0 0 {PAGE_HEIGHT} 0 0 cm /Scan Do Q\nBT /F1 14 Tf 3 Tr\n{rows}ET\n"
        entries[page_id - 1] = (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] "
            f"/Resources << /Font << /F1 3 0 R >> /XObject << /Scan {image_id} 0 R >> >> "
            f"/Contents {content_id} 0 R >>"
        ).encode()
        entries[content_id - 1] = stream("", content.encode())
        entries[image_id - 1] = stream(
            f"/Type /XObject /Subtype /Image /ColorSpace /DeviceGray /Width {RASTER_WIDTH} "
            f"/Height {RASTER_HEIGHT} /BitsPerComponent 1 /Filter /CCITTFaxDecode "
            f"/DecodeParms << /K -1 /Columns {RASTER_WIDTH} /Rows {RASTER_HEIGHT} /BlackIs1 true >>",
            group4_strip(scan_of(lines)),
        )
    output = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for number, entry in enumerate(entries, 1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode() + entry + b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    output.extend("".join(f"{offset:010d} 00000 n \n" for offset in offsets[1:]).encode())
    output.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    Path(sys.argv[1]).write_bytes(output)


if __name__ == "__main__":
    main()
