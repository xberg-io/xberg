"""Generate the image-only lexical-wrap regression page."""

import argparse
import zlib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

LINES = (
    "The printed paragraph ends with an open-",
    "source term and gives a clear example.",
    "The witness spells it opensource here.",
    "A separate open-source label is valid.",
)
WIDTH, HEIGHT = 875, 253
DPI = 96


def stream(payload: bytes, properties: str = "") -> bytes:
    """Wrap a PDF stream with an explicit length."""
    return f"<< /Length {len(payload)} {properties} >>\nstream\n".encode("ascii") + payload + b"\nendstream"


def render(font_path: Path) -> bytes:
    """Render the original spelling and physical line breaks into pixels."""
    image = Image.new("L", (WIDTH, HEIGHT), color=255)
    font = ImageFont.truetype(str(font_path), 30, layout_engine=ImageFont.Layout.BASIC)
    draw = ImageDraw.Draw(image)
    for index, line in enumerate(LINES):
        draw.text((38, 25 + 47 * index), line, fill=0, font=font, anchor="lt")
    return image.tobytes()


def pdf(raster: bytes) -> bytes:
    """Embed a single full-page image with no text or font objects."""
    page_width, page_height = WIDTH * 72 / DPI, HEIGHT * 72 / DPI
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width} {page_height}] "
        "/Resources << /XObject << /Scan 4 0 R >> >> /Contents 5 0 R >>".encode("ascii"),
        stream(
            zlib.compress(raster, level=9),
            f"/Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} "
            "/ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
        ),
        stream(f"q {page_width} 0 0 {page_height} 0 0 cm /Scan Do Q".encode("ascii")),
    )
    output = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode("ascii"))
        output.extend(body)
        output.extend(b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode("ascii"))
    for offset in offsets:
        output.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    output.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode("ascii"))
    return bytes(output)


def main() -> None:
    """Generate a PDF at the requested destination."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", type=Path)
    parser.add_argument("font", type=Path)
    args = parser.parse_args()
    args.target.write_bytes(pdf(render(args.font)))


if __name__ == "__main__":
    main()
