"""Make vector_table.pdf: a heading in real text, a table whose text is drawn as filled glyph outlines."""

import sys
from pathlib import Path
import reportlab
from fontTools.pens.basePen import BasePen
from fontTools.ttLib import TTFont
from reportlab.lib.pagesizes import letter
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont as RLFont
from reportlab.pdfgen import canvas

FONT = str(Path(reportlab.__file__).parent / "fonts" / "Vera.ttf")
tt = TTFont(FONT)
gs, cmap, upm = tt.getGlyphSet(), tt.getBestCmap(), tt["head"].unitsPerEm


class _PathPen(BasePen):
    def __init__(self, glyphs, path, x, y, size):
        super().__init__(glyphs)
        self.p, self.x, self.y, self.k = path, x, y, size / upm
        self.cur = (0, 0)

    def _point(self, p):
        return self.x + p[0] * self.k, self.y + p[1] * self.k

    def _moveTo(self, p):  # noqa: N802
        self.p.moveTo(*self._point(p))
        self.cur = p

    def _lineTo(self, p):  # noqa: N802
        self.p.lineTo(*self._point(p))
        self.cur = p

    def _curveToOne(self, a, b, c):  # noqa: N802
        self.p.curveTo(*self._point(a), *self._point(b), *self._point(c))
        self.cur = c

    def _qCurveToOne(self, q, p):  # noqa: N802
        c0, c3 = self.cur, p
        a = (c0[0] + 2 / 3 * (q[0] - c0[0]), c0[1] + 2 / 3 * (q[1] - c0[1]))
        b = (c3[0] + 2 / 3 * (q[0] - c3[0]), c3[1] + 2 / 3 * (q[1] - c3[1]))
        self._curveToOne(a, b, c3)

    def _closePath(self):  # noqa: N802
        self.p.close()


def outline(c, text, x, y, size=10):
    path = c.beginPath()
    for ch in text:
        name = cmap.get(ord(ch))
        if name is None:
            continue
        gs[name].draw(_PathPen(gs, path, x, y, size))
        x += gs[name].width * size / upm
    c.drawPath(path, stroke=0, fill=1)


pdfmetrics.registerFont(RLFont("Vera", FONT))

MODE = sys.argv[1] if len(sys.argv) > 1 else "table"
SIZE = float(sys.argv[3]) if len(sys.argv) > 3 else 10
SHORT = len(sys.argv) > 4 and sys.argv[4] == "short"
c = canvas.Canvas(sys.argv[2] if len(sys.argv) > 2 else "vector_table.pdf", pagesize=letter, invariant=1)
if SHORT:
    c.setFont("Vera", 10)
    c.drawString(72, 750, "Header")
    c.drawString(500, 40, "Page 7")
elif MODE != "headless":
    c.setFont("Vera", 14)
    c.drawString(72, 720, "Water Department Fund Statement")
    c.setFont("Vera", 10)
    c.drawString(72, 700, "Annual budget summary, page 1")
rows = [
    ("Revenues", "2025", "2026", "2027"),
    ("Water sales", "296,345", "200,000", "245,000"),
    ("Connection fees", "34,051", "20,000", "20,000"),
    ("Total revenues", "330,396", "220,000", "265,000"),
    ("Personnel", "97,311", "102,515", "109,251"),
    ("Supplies", "28,659", "35,000", "31,000"),
    ("Total expenditures", "168,030", "237,623", "264,485"),
]
if MODE == "prose":
    lines = [
        "The committee met on Tuesday to review the plan for the new library building.",
        "Members asked about the schedule, the budget and the plan for the parking area.",
        "The architect said that building work would start after the winter season ends.",
        "A vote on the final design is planned for the next regular meeting of the council.",
        "Residents may send written comments to the office before the end of the month.",
    ]
    for i, line in enumerate(lines):
        outline(c, line, 72, 650 - i * 22, SIZE)
elif MODE == "geometry":
    for index, width in enumerate((190, 270, 225, 340, 285, 210, 310)):
        c.rect(72, 642 - index * 28, width, 19, stroke=0, fill=1)
else:
    for i, row in enumerate(rows):
        for j, cell in enumerate(row):
            outline(c, cell, 72 + (0, 190, 270, 350)[j], 650 - i * 22, SIZE)
c.save()
