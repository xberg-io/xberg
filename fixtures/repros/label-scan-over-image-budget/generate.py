"""usage: make_scan_pdf.py OUT.pdf WIDTH_IN HEIGHT_IN DPI [plan|prose]"""
import io, random, sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

out, w_in, h_in, dpi = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), int(sys.argv[4])
mode = sys.argv[5] if len(sys.argv) > 5 else "plan"
W, H = int(w_in * dpi), int(h_in * dpi)
s = dpi / 300.0
FONT = "/System/Library/Fonts/Supplemental/Arial.ttf"
def font(px):
    return ImageFont.truetype(FONT, max(8, int(px * s)))

img = Image.new("L", (W, H), 255)
d = ImageDraw.Draw(img)
NOTES = [
    "GENERAL NOTES",
    "1. All lot lines and easements are shown for planning purposes only.",
    "2. Street names are subject to approval by the planning department.",
    "3. Drainage areas will be maintained by the owners association.",
    "4. Minimum front building line is twenty five feet from the right of way.",
    "5. Open space and trails are shown as approximate locations.",
]
PROSE = ("The river runs past the old mill and into the lake below the town. "
         "Each spring the water rises and the farmers wait for the fields to dry. "
         "A small bridge carries the road over the creek near the school. ")
if mode == "plan":
    streets = ["OAK ST", "ELM DR", "PINE CT", "CEDAR LN", "MAPLE AVE", "BIRCH WAY", "ASH RD"]
    cell, lot = int(300 * s), 1
    for gy in range(int(500 * s), H - int(900 * s), cell * 3):
        for gx in range(int(400 * s), W - int(2600 * s), cell):
            for row in (0, 1):
                y0 = gy + row * cell
                d.rectangle([gx, y0, gx + cell, y0 + cell], outline=0, width=max(1, int(4 * s)))
                d.text((gx + cell // 3, y0 + cell // 3), str(lot), font=font(70), fill=0)
                if lot % 4 == 0:
                    d.text((gx + 20 * s, y0 + cell - 70 * s), "B" + str(lot % 9), font=font(40), fill=0)
                lot += 1
        d.text((int(500 * s), gy + 2 * cell + int(260 * s)), streets[lot % len(streets)], font=font(80), fill=0)
    x0 = W - int(2400 * s)
    d.rectangle([x0, int(400 * s), W - int(200 * s), int(2200 * s)], outline=0, width=max(1, int(6 * s)))
    for i, line in enumerate(NOTES):
        cut = line.rfind(" ", 0, 48) if len(line) > 48 else len(line)
        d.text((x0 + 60 * s, int(480 * s + i * 120 * s)), line[:cut], font=font(56), fill=0)
        if cut < len(line):
            d.text((x0 + 120 * s, int(540 * s + i * 120 * s)), line[cut + 1:], font=font(56), fill=0)
    d.text((x0 + 60 * s, H - int(600 * s)), "SITE PLAN  SCALE 1 IN = 200 FT", font=font(90), fill=0)
else:
    y = int(300 * s)
    while y < H - int(300 * s):
        d.text((int(300 * s), y), (PROSE * 3)[: int(W / (30 * s))], font=font(60), fill=0)
        y += int(110 * s)

# Scan look: slight blur, paper grain, gray cast, stored as RGB like a scanner writes it.
img = img.filter(ImageFilter.GaussianBlur(1.2 * s))
img = Image.blend(img, Image.effect_noise((W, H), 18), 0.12)
rgb = Image.merge("RGB", (img.point(lambda v: v * 0.96 + 6),) * 3)
buf = io.BytesIO()
rgb.save(buf, "JPEG", quality=80, dpi=(dpi, dpi))
jpeg = buf.getvalue()

pw, ph = w_in * 72, h_in * 72
content = f"q {pw:.2f} 0 0 {ph:.2f} 0 0 cm /Im0 Do Q".encode()
objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2f} {ph:.2f}] "
    f"/Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>".encode(),
    b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
    b"<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceRGB "
    b"/BitsPerComponent 8 /Filter /DCTDecode /Length %d >>\nstream\n" % (W, H, len(jpeg)) + jpeg + b"\nendstream",
]
pdf = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i, body in enumerate(objs, 1):
    offsets.append(len(pdf))
    pdf += b"%d 0 obj\n" % i + body + b"\nendobj\n"
xref = len(pdf)
pdf += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
pdf += b"".join(b"%010d 00000 n \n" % o for o in offsets)
pdf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
open(out, "wb").write(pdf)
