"""usage: generate.py OUT.pdf WIDTH_IN HEIGHT_IN DPI [--mixed]

Writes an image-only PDF of a site plan: mostly one or two character labels and a notes block,
stored as one DeviceRGB JPEG covering the page. With --mixed, a native text page precedes the scan.
"""

import hashlib
import io
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[3]
FONT = ROOT / "crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf"
FONT_SHA256 = "abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322"
SEED = 2832
NOTES = [
    "GENERAL NOTES",
    "1. All lot lines and easements are shown for planning purposes only.",
    "2. Street names are subject to approval by the planning department.",
    "3. Drainage areas will be maintained by the owners association.",
    "4. Minimum front building line is twenty five feet from the right of way.",
    "5. Open space and trails are shown as approximate locations.",
]
STREETS = ["OAK ST", "ELM DR", "PINE CT", "CEDAR LN", "MAPLE AVE", "BIRCH WAY", "ASH RD"]
NATIVE_TEXT = b"BT /F1 12 Tf 16 TL 72 720 Td " + b"(The native page remains unchanged.) ' " * 10 + b"ET"


def render(w_in: float, h_in: float, dpi: int) -> bytes:
    if hashlib.sha256(FONT.read_bytes()).hexdigest() != FONT_SHA256:
        raise SystemExit(f"{FONT} does not match the pinned font")
    width, height = int(w_in * dpi), int(h_in * dpi)
    s = dpi / 300.0

    def font(px: int) -> ImageFont.FreeTypeFont:
        return ImageFont.truetype(str(FONT), max(8, int(px * s)))

    img = Image.new("L", (width, height), 255)
    d = ImageDraw.Draw(img)
    cell, lot = int(300 * s), 1
    for gy in range(int(500 * s), height - int(900 * s), cell * 3):
        for gx in range(int(400 * s), width - int(2600 * s), cell):
            for row in (0, 1):
                y0 = gy + row * cell
                d.rectangle([gx, y0, gx + cell, y0 + cell], outline=0, width=max(1, int(4 * s)))
                d.text((gx + cell // 3, y0 + cell // 3), str(lot), font=font(70), fill=0)
                if lot % 4 == 0:
                    d.text((gx + 20 * s, y0 + cell - 70 * s), "B" + str(lot % 9), font=font(40), fill=0)
                lot += 1
        d.text((int(500 * s), gy + 2 * cell + int(260 * s)), STREETS[lot % len(STREETS)], font=font(80), fill=0)
    x0 = width - int(2400 * s)
    d.rectangle([x0, int(400 * s), width - int(200 * s), int(2200 * s)], outline=0, width=max(1, int(6 * s)))
    for i, line in enumerate(NOTES):
        cut = line.rfind(" ", 0, 48) if len(line) > 48 else len(line)
        d.text((x0 + 60 * s, int(480 * s + i * 120 * s)), line[:cut], font=font(56), fill=0)
        if cut < len(line):
            d.text((x0 + 120 * s, int(540 * s + i * 120 * s)), line[cut + 1 :], font=font(56), fill=0)
    d.text((x0 + 60 * s, height - int(600 * s)), "SITE PLAN  SCALE 1 IN = 200 FT", font=font(90), fill=0)

    # Scan look: slight blur, seeded paper grain, gray cast, stored as RGB like a scanner writes it.
    img = img.filter(ImageFilter.GaussianBlur(1.2 * s))
    grain = Image.frombytes("L", (width, height), random.Random(SEED).randbytes(width * height))
    img = Image.blend(img, grain, 0.025)
    rgb = Image.merge("RGB", (img.point(lambda v: v * 0.96 + 6),) * 3)
    buf = io.BytesIO()
    rgb.save(buf, "JPEG", quality=80, dpi=(dpi, dpi))
    return buf.getvalue()


def stream(fields: bytes, data: bytes) -> bytes:
    return b"<< " + fields + b" /Length %d >>\nstream\n" % len(data) + data + b"\nendstream"


def build(jpeg: bytes, w_in: float, h_in: float, dpi: int, mixed: bool) -> bytes:
    pw, ph = w_in * 72, h_in * 72
    width, height = int(w_in * dpi), int(h_in * dpi)
    paint = f"q {pw:.2f} 0 0 {ph:.2f} 0 0 cm /Im0 Do Q".encode()
    scan_page = (
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2f} {ph:.2f}] "
        f"/Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"
    ).encode()
    kids = b"[6 0 R 3 0 R]" if mixed else b"[3 0 R]"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids " + kids + b" /Count %d >>" % (2 if mixed else 1),
        scan_page,
        stream(b"", paint),
        stream(
            b"/Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceRGB "
            b"/BitsPerComponent 8 /Filter /DCTDecode" % (width, height),
            jpeg,
        ),
    ]
    if mixed:
        objs += [
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources << /Font << /F1 8 0 R >> >> /Contents 7 0 R >>",
            stream(b"", NATIVE_TEXT),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ]
    pdf = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objs, 1):
        offsets.append(len(pdf))
        pdf += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    xref = len(pdf)
    pdf += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    pdf += b"".join(b"%010d 00000 n \n" % offset for offset in offsets)
    pdf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(pdf)


def main() -> None:
    args = [arg for arg in sys.argv[1:] if arg != "--mixed"]
    if len(args) != 4:
        raise SystemExit(__doc__)
    out, w_in, h_in, dpi = args[0], float(args[1]), float(args[2]), int(args[3])
    Path(out).write_bytes(build(render(w_in, h_in, dpi), w_in, h_in, dpi, "--mixed" in sys.argv))


if __name__ == "__main__":
    main()
