# usage: python3 -I g2327.py pdfs   (needs pillow and reportlab and a serif TrueType font)
# Writes six one-page Letter PDFs whose prose is a raster picture beside a heading-sized text layer.
import os
import sys
import textwrap

from PIL import Image, ImageDraw, ImageFont
from reportlab.lib.utils import ImageReader
from reportlab.pdfgen import canvas

PROSE = [
    (
        "The committee met on Tuesday to review the proposal for the new library building. "
        "Members asked about the schedule, the budget and the plan for the parking area."
    ),
    (
        "The architect explained that construction would begin after the winter season ends. "
        "A vote on the final design is planned for the next regular meeting of the council."
    ),
    (
        "Residents may send written comments to the office before the end of the month. "
        "The report also lists the staff needed to run the building during its first year."
    ),
]
FONTS = [
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
]


def font(size):
    for path in FONTS:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    raise SystemExit("no serif TrueType font found")


def prose_image(width, height, wrap, paragraphs=PROSE):
    img = Image.new("RGB", (width, height), "white")
    draw, f, y = ImageDraw.Draw(img), font(25), 40
    for paragraph in paragraphs:
        for line in textwrap.wrap(paragraph, wrap):
            draw.text((100 if width > 900 else 30, y), line, font=f, fill="black")
            y += 34
        y += 30
    return img


def photo(width, height):
    img = Image.new("RGB", (width, height))
    draw = ImageDraw.Draw(img)
    for y in range(height):
        draw.line([(0, y), (width, y)], fill=(40 + y * 120 // height, 90, 200 - y * 120 // height))
    draw.ellipse((width // 4, height // 4, width * 3 // 4, height * 3 // 4), fill=(200, 160, 60))
    return img


def write(path, pictures, texts):
    c = canvas.Canvas(path, pagesize=(612, 792), invariant=1)
    for img, box in pictures:
        c.drawImage(ImageReader(img), *box)
    c.setFont("Helvetica", 12)
    for x, y, s in texts:
        c.drawString(x, y, s)
    c.save()


LONG_HEADING = "Planning committee minutes: new library building proposal and parking plan"


def main(out):
    os.makedirs(out, exist_ok=True)
    half = (prose_image(1275, 825, 85), (0, 0, 612, 396))
    side = (prose_image(600, 1600, 38), (0, 0, 250, 667))
    pair = [
        (prose_image(1275, 400, 85, PROSE[:1]), (0, 0, 612, 192)),
        (prose_image(1275, 400, 85, PROSE[1:2]), (0, 250, 612, 192)),
    ]
    caption = (72, 410, "Figure 1. A caption beside the photograph.")
    short = [(72, 740, "Heading"), (280, 40, "Page 7")]
    long = [(72, 740, LONG_HEADING), (280, 40, "Page 7")]
    write(f"{out}/half_short.pdf", [half], short)
    write(f"{out}/half_long.pdf", [half], long)
    write(f"{out}/side_long.pdf", [side], [(300, 740, LONG_HEADING), (300, 40, "Page 7")])
    write(f"{out}/two_short.pdf", pair, short)
    write(f"{out}/photo_short.pdf", [(photo(612, 396), (0, 0, 612, 396))], [*short, caption])
    write(f"{out}/photo_long.pdf", [(photo(612, 396), (0, 0, 612, 396))], [*long, caption])


if __name__ == "__main__":
    main(sys.argv[1])
