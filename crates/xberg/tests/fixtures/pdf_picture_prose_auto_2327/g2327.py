# usage: python3 -I g2327.py OUTDIR   (needs pillow and reportlab)
# Writes one-page Letter PDFs whose prose is a raster picture beside a heading-sized text layer.
import os
import sys
import textwrap

from PIL import Image, ImageDraw, ImageFont
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
    c = canvas.Canvas(path, pagesize=(612, 792))
    for i, (img, box) in enumerate(pictures):
        png = f"{path}.{i}.png"
        img.save(png)
        c.drawImage(png, *box)
        os.remove(png)
    c.setFont("Helvetica", 12)
    for x, y, s in texts:
        c.drawString(x, y, s)
    c.save()


LONG_HEADING = "Planning committee minutes: new library building proposal and parking plan"


def main(out):
    os.makedirs(out, exist_ok=True)
    half = (prose_image(1275, 825, 85), (0, 0, 612, 396))
    top = (prose_image(1275, 825, 85), (0, 396, 612, 396))
    side = (prose_image(600, 1600, 38), (0, 0, 250, 667))
    pair = [
        (prose_image(1275, 400, 85, PROSE[:1]), (0, 0, 612, 192)),
        (prose_image(1275, 400, 85, PROSE[1:2]), (0, 250, 612, 192)),
    ]
    # `short` is the issue's text layer; `long` is a running header of one line that still
    # covers far less than a fifth of the page but is enough text for the native page to read
    # as clean prose.
    for kind, heading in (("short", "Heading"), ("long", LONG_HEADING)):
        low = [(72, 740, heading), (280, 40, "Page 7")]
        upper = [(72, 380, heading), (280, 40, "Page 7")]
        beside = [(300, 740, heading), (300, 40, "Page 7")]
        write(f"{out}/half_{kind}.pdf", [half], low)
        write(f"{out}/top_{kind}.pdf", [top], upper)
        write(f"{out}/side_{kind}.pdf", [side], beside)
        write(f"{out}/two_{kind}.pdf", pair, low)
        write(
            f"{out}/photo_{kind}.pdf",
            [(photo(612, 396), (0, 0, 612, 396))],
            low + [(72, 410, "Figure 1. A caption beside the photograph.")],
        )


def write_variants(out):
    """Stored-picture shapes: encoding, nesting, size and page orientation."""
    low = [(72, 740, LONG_HEADING), (280, 40, "Page 7")]
    img = prose_image(1275, 825, 85)

    def one(name, draw, rotate=0):
        path = f"{out}/{name}.pdf"
        c = canvas.Canvas(path, pagesize=(612, 792))
        if rotate:
            c.setPageRotation(rotate)
        draw(c, path)
        c.setFont("Helvetica", 12)
        for x, y, s in low:
            c.drawString(x, y, s)
        c.save()

    def jpeg(c, path):
        img.save(path + ".jpg", quality=92)
        c.drawImage(path + ".jpg", 0, 0, 612, 396)
        os.remove(path + ".jpg")

    def gray(c, path):
        img.convert("L").save(path + ".png")
        c.drawImage(path + ".png", 0, 0, 612, 396)
        os.remove(path + ".png")

    def smask(c, path):
        rgba = img.convert("RGBA")
        rgba.save(path + ".png")
        c.drawImage(path + ".png", 0, 0, 612, 396, mask="auto")
        os.remove(path + ".png")

    def form(c, path):
        img.save(path + ".png")
        c.beginForm("panel")
        c.drawImage(path + ".png", 0, 0, 612, 396)
        c.endForm()
        c.doForm("panel")
        os.remove(path + ".png")

    def small(c, path):
        crop = prose_image(1275, 300, 85, PROSE[:1])
        crop.save(path + ".png")
        c.drawImage(path + ".png", 0, 0, 612, 144)
        os.remove(path + ".png")

    def sideways(c, path):
        img.save(path + ".png")
        c.saveState()
        c.translate(0, 0)
        c.rotate(90)
        c.drawImage(path + ".png", 0, -300, 700, 340)
        c.restoreState()
        os.remove(path + ".png")

    one("jpeg_long", jpeg)
    one("gray_long", gray)
    one("smask_long", smask)
    one("form_long", form)
    one("small_long", small)
    one("sideways_long", sideways)
    one("rotated_page_long", jpeg, rotate=90)


if __name__ == "__main__":
    main(sys.argv[1])
    write_variants(sys.argv[1])
