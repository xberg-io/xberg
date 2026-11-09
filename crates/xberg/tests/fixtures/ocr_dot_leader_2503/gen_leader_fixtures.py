"""Writes 11.png, 11.tsv and toc.png (invented values only).

usage: uv run --no-project --with pillow python gen_leader_fixtures.py <DejaVuSans.ttf> <out dir>

11.png is img_11_plain_lead of the #2432 reporter's gen_table_fixtures.py (px 11, no tint, leaders,
no grid; its table_rows seed there is 15); draw_table and table_rows are copied from that script.
"""

import os
import random
import sys

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
    "Snow removal",
    "Field lights",
]


def amounts(rng, i):
    two = str(rng.randint(10, 99))
    big = f"{rng.randint(1, 999)},{rng.randint(0, 999):03d}"
    alt = [
        f"({rng.randint(100, 9999):,})",
        f"{rng.randint(10, 999)}.{rng.randint(0, 99):02d}",
        f"{rng.randint(1000, 99999):,}",
    ][i % 3]
    return [two, big, alt]


def table_rows(seed, n=12):
    rng = random.Random(seed)
    return [[LABELS[(seed + i) % len(LABELS)]] + amounts(rng, i) for i in range(n)]


def draw_table(font, rows, px):
    f = ImageFont.truetype(font, px)
    colw = [int(px * 13), int(px * 4.5), int(px * 6), int(px * 6)]
    rowh = int(px * 1.9)
    w, h = sum(colw) + 2 * px, rowh * (len(rows) + 1) + 2 * px
    im = Image.new("RGB", (w, h), (255, 255, 255))
    d = ImageDraw.Draw(im)
    for r, row in enumerate([["Item", "Count", "Budget", "Change"]] + rows):
        y = px + r * rowh
        x = px
        for c, cell in enumerate(row):
            if c == 0:
                d.text((x, y), cell, font=f, fill=(0, 0, 0))
                if r > 0:
                    lx = x + d.textlength(cell + " ", font=f)
                    while lx < x + colw[0] - px:
                        d.text((lx, y), ".", font=f, fill=(0, 0, 0))
                        lx += px * 0.55
            else:
                tw = d.textlength(cell, font=f)
                d.text((x + colw[c] - tw - px // 2, y), cell, font=f, fill=(0, 0, 0))
            x += colw[c]
    return im


def toc(font, px=20):
    """A table of contents: a title, a dot leader and a page number per line; two columns, no table."""
    f = ImageFont.truetype(font, px)
    entries = [
        ("Introduction", 1),
        ("Water service", 4),
        ("Street repair", 9),
        ("Library hours", 13),
        ("Fleet fuel", 18),
        ("Bridge paint", 22),
        ("Storm drains", 27),
        ("Transit passes", 31),
        ("Code review", 36),
        ("Tree planting", 40),
    ]
    width, rowh = px * 30, int(px * 1.9)
    im = Image.new("RGB", (width, rowh * (len(entries) + 1)), (255, 255, 255))
    d = ImageDraw.Draw(im)
    for i, (title, page) in enumerate(entries):
        y = px + i * rowh
        d.text((px, y), title, font=f, fill=(0, 0, 0))
        number = str(page)
        right = width - px
        d.text((right - d.textlength(number, font=f), y), number, font=f, fill=(0, 0, 0))
        lx = px + d.textlength(title + " ", font=f)
        while lx < right - d.textlength(number, font=f) - px:
            d.text((lx, y), ".", font=f, fill=(0, 0, 0))
            lx += px * 0.55
    return im


def main():
    font, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    rows = table_rows(15)
    draw_table(font, rows, 11).save(os.path.join(out, "11.png"), format="PNG")
    with open(os.path.join(out, "11.tsv"), "w") as tsv:
        tsv.writelines("\t".join(row) + "\n" for row in rows)
    toc(font).save(os.path.join(out, "toc.png"), format="PNG")


if __name__ == "__main__":
    main()
