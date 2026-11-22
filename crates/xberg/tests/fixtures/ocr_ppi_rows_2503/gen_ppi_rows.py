"""Writes the JPEG q45 six-column table fixtures for #2503's low-ppi row cases.

usage: uv run --no-project --with pillow python gen_ppi_rows.py <DejaVuSans.ttf> <out dir>

Two rasters, both twelve rows of six columns, both JPEG quality 45:

- `six-column-150dpi-q45.jpg` is the committed #2503 six-column generator layout
  (`../ocr_six_column_rows_2503/gen_six_column_fixture.py`, seed 21, DejaVu Sans 20 px at
  150 dpi) saved as JPEG q45. Labels, dot leaders and five value columns.
- `six-column-100dpi-9px-q45.jpg` is the #3337 low-ppi layout (A4 at 100 dpi, digit height
  9 px, six number columns, values 101..172) on plain paper saved as JPEG q45. The committed
  #3337 fixture of the same layout is tinted and lossless; this is the JPEG arm the issue
  names ("a tinted or JPEG picture with small glyphs").

`rows.tsv` and `number-rows.tsv` are the generator inputs, independent of OCR.
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
JPEG_QUALITY = 45


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


def draw_six_column(font_path, table_rows, px=20):
    font = ImageFont.truetype(font_path, px)
    label_width = int(px * 9)
    value_width = int(px * 5.5)
    row_height = int(px * 1.9)
    width = label_width + value_width * 5 + 2 * px
    height = row_height * (len(table_rows) + 1) + 2 * px
    image = Image.new("RGB", (width, height), (255, 255, 255))
    draw_ctx = ImageDraw.Draw(image)
    for index, row in enumerate([["Item", *HEADERS], *table_rows]):
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


def draw_number_grid(font_path, dpi, digit_px):
    """The #3337 layout: an A4 page of twelve rows by six three-digit numbers, 101..172."""
    width_px = round(210.0 / 25.4 * dpi)
    height_px = round(297.0 / 25.4 * dpi)
    em_px = digit_px / 0.716
    font = ImageFont.truetype(font_path, round(em_px))
    image = Image.new("RGB", (width_px, height_px), (255, 255, 255))
    draw_ctx = ImageDraw.Draw(image)
    left_margin = em_px * 2.0
    top_margin = em_px * 3.0
    column_pitch = em_px * 3.1
    row_pitch = em_px * 1.5
    for row in range(12):
        for column in range(6):
            value = 101 + row * 6 + column
            draw_ctx.text(
                (left_margin + column * column_pitch, top_margin + row * row_pitch),
                str(value),
                font=font,
                fill=(0, 0, 0),
            )
    return image


def main():
    font, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)

    table_rows = rows(seed=21)
    table = draw_six_column(font, table_rows)
    table.save(out / "six-column-150dpi-q45.jpg", format="JPEG", quality=JPEG_QUALITY, dpi=(DPI, DPI))
    with (out / "rows.tsv").open("w") as tsv:
        tsv.writelines("\t".join(row) + "\n" for row in table_rows)

    grid = draw_number_grid(font, dpi=100, digit_px=9)
    grid.save(out / "six-column-100dpi-9px-q45.jpg", format="JPEG", quality=JPEG_QUALITY, dpi=(100, 100))
    with (out / "number-rows.tsv").open("w") as tsv:
        for row in range(12):
            tsv.write("\t".join(str(101 + row * 6 + column) for column in range(6)) + "\n")


if __name__ == "__main__":
    main()
