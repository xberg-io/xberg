"""Writes the six-column 150 dpi dot-leader table used by the HTML row-preservation test (#2503).

usage: uv run --no-project --with pillow python gen_six_column_fixture.py <DejaVuSans.ttf> <out dir>

Twelve invented rows, one label column and five numeric columns, dot leaders between the label
and the first number. `six-column-150dpi.png` is the table raster; `six-column-150dpi.pdf` is a
one-page 150 dpi scan whose page raster is that table, with no margins, the route whose HTML
output used to print one row beside the table it also carried. Both files are deterministic for
a given Pillow version.
"""

import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

LABELS = [
    "Parks and trails",
    "Water service",
    "Street repair",
    "Library hours",
    "Fleet fuel",
    "Bridge paint",
    "Storm drains",
    "Transit passes",
    "Code review",
    "Tree planting",
    "Signal timing",
    "Pool repairs",
]

HEADERS = ["Count", "Budget", "Change", "Actual", "Variance"]
DPI = 150


def values(rng, i):
    return [
        str(rng.randint(10, 99)),
        f"{rng.randint(100, 999)},{rng.randint(0, 999):03d}",
        [f"({rng.randint(100, 9999):,})", f"{rng.randint(10, 999)}.{rng.randint(0, 99):02d}"][i % 2],
        f"{rng.randint(1000, 99999):,}",
        f"({rng.randint(10, 999):,})" if i % 3 == 0 else f"{rng.randint(100, 999)}.{rng.randint(0, 99):02d}",
    ]


def rows(seed):
    rng = random.Random(seed)
    return [[LABELS[i], *values(rng, i)] for i in range(len(LABELS))]


def draw(font_path, rows, px=20):
    font = ImageFont.truetype(font_path, px)
    label_width = int(px * 9)
    value_width = int(px * 5.5)
    row_height = int(px * 1.9)
    width = label_width + value_width * 5 + 2 * px
    height = row_height * (len(rows) + 1) + 2 * px
    image = Image.new("RGB", (width, height), (255, 255, 255))
    draw_ctx = ImageDraw.Draw(image)
    for index, row in enumerate([["Item", *HEADERS], *rows]):
        y = px + index * row_height
        x = px
        draw_ctx.text((x, y), row[0], font=font, fill=(0, 0, 0))
        if index > 0:
            leader_x = x + draw_ctx.textlength(row[0] + " ", font=font)
            while leader_x < x + label_width - px:
                draw_ctx.text((leader_x, y), ".", font=font, fill=(0, 0, 0))
                leader_x += px * 0.55
        x += label_width
        for cell in row[1:]:
            cell_width = draw_ctx.textlength(cell, font=font)
            draw_ctx.text((x + value_width - cell_width - px // 2, y), cell, font=font, fill=(0, 0, 0))
            x += value_width
    return image


def main():
    font, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    table_rows = rows(seed=21)
    table = draw(font, table_rows)
    table.save(out / "six-column-150dpi.png", format="PNG", dpi=(DPI, DPI))
    with (out / "six-column-150dpi.tsv").open("w") as tsv:
        tsv.writelines("\t".join(row) + "\n" for row in table_rows)
    # The page is the table raster at 150 dpi. A letter-size page with margins reads the same
    # table as prose at PSM 6, so the scan route needs the table to fill its page.
    table.save(out / "six-column-150dpi.pdf", format="PDF", resolution=DPI)


if __name__ == "__main__":
    main()
