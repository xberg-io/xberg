"""make_cid_page.py VARIANT OUT.pdf [FONT.ttf]: one page of neutral English prose in an embedded CIDFontType2 font of the
"keep glyph IDs" subset shape (DejaVu Sans without its cmap and post tables, CIDToGIDMap /Identity, Identity-H codes
equal to glyph IDs) under a Helvetica title. The letter "q" has no ToUnicode entry (one lost glyph).
  arabic:  the ToUnicode map sends each Latin letter to an Arabic code point.
  bengali: the ToUnicode map sends each Latin letter to a Bengali code point.
  genuine: the body is real Arabic text, its ToUnicode map is correct, and the letter qaf has no entry (control: a
           page whose ink really is Arabic).
Needs fontTools. FONT defaults to crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf.
"""

import io
import sys
from pathlib import Path
import zlib

from fontTools.ttLib import TTFont

variant, out = sys.argv[1:3]
font = TTFont(sys.argv[3] if len(sys.argv) > 3 else "crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf")
best = font.getBestCmap()
gid = {chr(c): font.getGlyphID(best[c]) for c in list(range(32, 127)) + list(range(0x0621, 0x064B)) if c in best}
upem = font["head"].unitsPerEm
widths = {g: font["hmtx"][font.getGlyphName(g)][0] * 1000 // upem for g in gid.values()}
del font["cmap"]
del font["post"]
buf = io.BytesIO()
font.save(buf)
program = buf.getvalue()

text = (
    "The committee met on Tuesday to review the plan for the new library. Members asked about the schedule and the "
    "budget for the parking area. The architect said that building work would start after the winter. A quiet queue "
    "of residents waited to speak about the design of the reading rooms and the garden. "
) * 4
ARABIC = (
    "اجتمعت اللجنة يوم الثلاثاء لمراجعة خطة المكتبة الجديدة وسأل الأعضاء عن الجدول الزمني وميزانية موقف السيارات "
    "وقال المهندس إن أعمال البناء ستبدأ بعد الشتاء وانتظر السكان للحديث عن تصميم قاعات القراءة والحديقة "
) * 4
if variant == "genuine":
    text = ARABIC
lines, cur = [], ""
for word in text.split():
    if len(cur) + len(word) + 1 > 95:
        lines.append(cur)
        cur = word
    else:
        cur = (cur + " " + word).strip()
lines.append(cur)

ops = ["BT /F1 14 Tf 50 750 Td (Library Planning Notes) Tj ET", "BT /F2 10 Tf 50 720 Td 13 TL"]
ops += ["<" + "".join(f"{gid[c]:04X}" for c in line) + "> Tj T*" for line in lines]
ops.append("ET")
content = "\n".join(ops).encode()

BASES = {"arabic": 0x0627, "bengali": 0x0985}


def unicode_of(c: str) -> int:
    """The code point the ToUnicode map gives a Latin character in this variant."""
    if variant in BASES and c.isalpha():
        return BASES[variant] + (ord(c.lower()) - 97) % 26
    return ord(c)


LOST = "\u0642" if variant == "genuine" else "q"
pairs = [(g, c) for c, g in sorted(gid.items(), key=lambda x: x[1]) if c != LOST]
cmap = [
    "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /U def",
    "1 begincodespacerange <0000> <FFFF> endcodespacerange",
    f"{len(pairs)} beginbfchar",
]
cmap += [f"<{g:04X}> <{unicode_of(c):04X}>" for g, c in pairs]
cmap += ["endbfchar", "endcmap CMapName currentdict /CMap defineresource pop end end"]
tounicode = "\n".join(cmap).encode()

w = " ".join(f"{g} [{a}]" for g, a in sorted(widths.items()))
resources = "/Font << /F1 5 0 R /F2 6 0 R >>"
objects = {
    1: b"<< /Type /Catalog /Pages 2 0 R >>",
    2: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    3: f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << {resources} >> /Contents 4 0 R >>".encode(),
    5: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    6: b"<< /Type /Font /Subtype /Type0 /BaseFont /ABCDEF+DejaVuSans /Encoding /Identity-H /DescendantFonts [7 0 R] "
    b"/ToUnicode 10 0 R >>",
    7: (
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ABCDEF+DejaVuSans /CIDSystemInfo << /Registry (Adobe) "
        f"/Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R /CIDToGIDMap /Identity /DW 1000 /W [{w}] >>"
    ).encode(),
    8: b"<< /Type /FontDescriptor /FontName /ABCDEF+DejaVuSans /Flags 4 /FontBBox [-1021 -463 1793 1232] /ItalicAngle 0 "
    b"/Ascent 928 /Descent -236 /CapHeight 729 /StemV 80 /FontFile2 9 0 R >>",
}
streams = {4: content, 9: program, 10: tounicode}
pdf = io.BytesIO()
pdf.write(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i in range(1, 11):
    offsets.append(pdf.tell())
    if i in streams:
        z = zlib.compress(streams[i])
        extra = b" /Length1 %d" % len(program) if i == 9 else b""
        pdf.write(
            b"%d 0 obj\n<< /Length %d /Filter /FlateDecode%s >>\nstream\n" % (i, len(z), extra)
            + z
            + b"\nendstream\nendobj\n"
        )
    else:
        pdf.write(b"%d 0 obj\n" % i + objects[i] + b"\nendobj\n")
xref = pdf.tell()
pdf.write(b"xref\n0 11\n0000000000 65535 f \n" + b"".join(b"%010d 00000 n \n" % o for o in offsets))
pdf.write(b"trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % xref)
Path(out).write_bytes(pdf.getvalue())
