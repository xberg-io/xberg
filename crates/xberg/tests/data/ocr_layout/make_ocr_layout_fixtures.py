"""Generate the synthetic pages for the OCR false-table regression tests.

Run: python -I make_ocr_layout_fixtures.py <output-directory>

Writes, with invented text only:
  cover_scattered.png    a cover page with a few large text blocks placed far apart
  map_loose_grid.png     a diagram whose labels sit in a loose grid, with no rules
  ruled_table.png        a ruled table that must stay a table
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

FONT_DIR = Path("/usr/share/fonts/truetype/dejavu")


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    name = "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf"
    return ImageFont.truetype(str(FONT_DIR / name), size)


def save(image: Image.Image, path: Path, dpi: tuple[int, int]) -> None:
    image.convert("L").save(path, dpi=dpi, optimize=True)


def page(width: int, height: int) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    image = Image.new("RGB", (width, height), "white")
    return image, ImageDraw.Draw(image)


def cover_scattered(out: Path) -> None:
    image, draw = page(1700, 2200)
    draw.text((180, 260), "HARBOR CITY", font=font(150, True), fill="black")
    draw.text((620, 520), "Annual Review", font=font(96), fill="black")
    draw.text((1000, 760), "Seventh Edition", font=font(72), fill="black")
    draw.text((150, 1350), "Prepared for the council", font=font(64), fill="black")
    draw.text((700, 1650), "Office of Planning", font=font(64), fill="black")
    draw.text((250, 1950), "Spring Volume Two", font=font(56), fill="black")
    save(image, out / "cover_scattered.png", dpi=(300, 300))


def map_loose_grid(out: Path) -> None:
    image, draw = page(1700, 1100)
    labels = [
        ["North Harbor", "Pine Ridge", "Stone Bridge"],
        ["Cedar Point", "Lake Mirror", "Fox Hollow"],
        ["Quarry Hill", "Salt Marsh", "Bell Tower"],
    ]
    x_offsets = [(120, 560, 1100), (160, 640, 1060), (100, 600, 1140)]
    for row, line in enumerate(labels):
        for column, label in enumerate(line):
            draw.text((x_offsets[row][column], 160 + row * 300), label, font=font(52), fill="black")
    save(image, out / "map_loose_grid.png", dpi=(300, 300))


def cover_centered(out: Path) -> None:
    image, draw = page(1700, 2200)
    lines = [
        (300, "FISCAL YEAR 2027", 110, True),
        (480, "ADOPTED BUDGET", 110, True),
        (700, "CITY OF HARBOR", 80, False),
        (780, "Department of Finance", 52, False),
        (1900, "Published March 2027", 48, False),
        (1990, "Harbor City Hall, Room 12", 48, False),
    ]
    for y, text, size, bold in lines:
        width = draw.textlength(text, font=font(size, bold))
        draw.text(((1700 - width) / 2, y), text, font=font(size, bold), fill="black")
    save(image, out / "cover_centered.png", dpi=(300, 300))


def map_ragged(out: Path) -> None:
    image, draw = page(1700, 1300)
    labels = [
        ["Harbor", "Ridge", "Bridge"],
        ["Point", "Mirror", "Hollow"],
        ["Quarry", "Marsh", "Tower"],
        ["Meadow", "Orchard", "Depot"],
    ]
    offsets = [(120, 650, 1250), (300, 620, 1100), (90, 760, 1300), (240, 700, 1180)]
    for row, line in enumerate(labels):
        for column, label in enumerate(line):
            draw.text((offsets[row][column], 150 + row * 105), label, font=font(46), fill="black")
    save(image, out / "map_ragged.png", dpi=(300, 300))


def ruled_table(out: Path) -> None:
    image, draw = page(1700, 900)
    left, top, row_height = 150, 150, 110
    columns = [left, left + 600, left + 900, left + 1200, left + 1500]
    rows = [
        ["Item", "Qty", "Price", "Total"],
        ["Anvil", "14", "12.50", "50.00"],
        ["Bucket", "10", "3.20", "32.00"],
        ["Chisel", "27", "5.75", "40.25"],
        ["Ladder", "32", "48.00", "96.00"],
    ]
    bottom = top + row_height * len(rows)
    for index in range(len(rows) + 1):
        y = top + index * row_height
        draw.line([(columns[0], y), (columns[-1], y)], fill="black", width=3)
    for x in columns:
        draw.line([(x, top), (x, bottom)], fill="black", width=3)
    for row_index, row in enumerate(rows):
        for column_index, cell in enumerate(row):
            draw.text(
                (columns[column_index] + 24, top + row_index * row_height + 26),
                cell,
                font=font(48, row_index == 0),
                fill="black",
            )
    save(image, out / "ruled_table.png", dpi=(300, 300))


def ledger_plain(out: Path) -> None:
    image, draw = page(1400, 700)
    x_debit, x_credit, top, step = 200, 700, 150, 110
    rows = [("Debit", "Credit"), ("100", ""), ("", "200"), ("300", ""), ("", "400")]
    for index, (debit, credit) in enumerate(rows):
        y = top + index * step
        draw.text((x_debit, y), debit, font=font(48, index == 0), fill="black")
        draw.text((x_credit, y), credit, font=font(48, index == 0), fill="black")
    save(image, out / "ledger_plain.png", dpi=(300, 300))


if __name__ == "__main__":
    target = Path(sys.argv[1])
    target.mkdir(parents=True, exist_ok=True)
    cover_scattered(target)
    map_loose_grid(target)
    cover_centered(target)
    map_ragged(target)
    ruled_table(target)
    ledger_plain(target)
