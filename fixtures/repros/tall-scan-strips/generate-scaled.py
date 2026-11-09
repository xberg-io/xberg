"""Generate identical physical scans at 150 and 300 DPI for line-geometry regression."""

import hashlib
import zlib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
TARGET = Path(__file__).resolve().parent
FONT = ROOT / "crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf"
FONT_SHA256 = "abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322"


def _stream(data: bytes, fields: str = "") -> bytes:
    return f"<< /Length {len(data)} {fields} >>\nstream\n".encode() + data + b"\nendstream"


def _write_pdf(image: Image.Image, dpi: int) -> None:
    width, height = image.size
    paint = b"q 216 0 0 5760 0 0 cm /Scan Do Q"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 216 5760] "
            b"/Resources << /XObject << /Scan 5 0 R >> >> /Contents 4 0 R >>"
        ),
        _stream(paint),
        _stream(
            zlib.compress(image.tobytes()),
            f"/Type /XObject /Subtype /Image /Width {width} /Height {height} "
            "/ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
        ),
    ]
    output = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for number, obj in enumerate(objects, 1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode() + obj + b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        output.extend(f"{offset:010d} 00000 n \n".encode())
    output.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    (TARGET / f"large-text-{dpi}.pdf").write_bytes(output)


def _generate() -> None:
    if hashlib.sha256(FONT.read_bytes()).hexdigest() != FONT_SHA256:
        raise ValueError("The source font does not match the pinned DejaVu Sans font")
    tags = [line.split()[-1] for line in (TARGET / "expected.txt").read_text().splitlines()]
    lines = ["Public copy", *[f"Source text stays here for {tag}" for tag in tags]]
    image = Image.new("L", (450, 12000), 255)
    draw = ImageDraw.Draw(image)
    draw.text((20, 100), lines[0], font=ImageFont.truetype(str(FONT), 64), fill=0)
    body_font = ImageFont.truetype(str(FONT), 20)
    for index, line in enumerate(lines[1:]):
        if draw.textlength(line, font=body_font) > image.width - 40:
            raise ValueError(f"Source line does not fit inside the scan: {line}")
        draw.text((20, 500 + index * 280), line, font=body_font, fill=0)
    _write_pdf(image, 150)
    _write_pdf(image.resize((900, 24000), Image.Resampling.NEAREST), 300)
    (TARGET / "large-text-expected.txt").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    _generate()
