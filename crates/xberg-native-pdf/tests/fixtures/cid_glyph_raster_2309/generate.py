"""Regenerate issue #2309 CID subset fixtures with the lock-pinned fonttools."""

import io
import sys
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable

SENT = [
    "The committee met on Tuesday to review the plan for the new library.",
    "Members asked about the schedule and the budget for the parking area.",
    "The architect said that building work would start after the winter.",
    "A vote on the final design is planned for the next regular meeting.",
]
LINES = [SENT[i % 4] for i in range(20)]
tt = TTFont(str(Path(__file__).resolve().parents[3] / "src/fonts/assets/DejaVuSans.ttf"), recalcTimestamp=False)
o = subset.Options()
o.layout_features = []
o.hinting = False
o.notdef_outline = True
s = subset.Subsetter(o)
s.populate(text="".join(SENT))
s.subset(tt)
order = tt.getGlyphOrder()
cm = tt.getBestCmap()
gid = {c: order.index(cm[ord(c)]) for c in set("".join(SENT))}
full_cmap = tt["cmap"]
t = newTable("cmap")
t.tableVersion = 0
st = CmapSubtable.newSubtable(4)
st.platformID, st.platEncID, st.language = 3, 1, 0
st.cmap = {0x20: order[gid[" "]], 0xA0: order[gid[" "]]}
t.tables = [st]
tt["cmap"] = t  # a two-entry Unicode cmap
mode = sys.argv[2] if len(sys.argv) > 2 else "nul"
if mode == "direct":
    del tt["cmap"]
elif mode == "custom":
    tt["cmap"] = full_cmap


def code(glyph):
    return glyph + 1000 if mode == "custom" else glyph


buf = io.BytesIO()
tt.save(buf)
prog = buf.getvalue()
w = tt["hmtx"].metrics
widths = [int(w[g][0] * 1000 / tt["head"].unitsPerEm) for g in order]
tu = (
    "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def "
    "/CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n"
    f"{0 if mode == 'missing' else len(gid)} beginbfchar\n"
)
if mode != "missing":
    for glyph, character in sorted((g, c) for c, g in gid.items()):
        mapped = mode in ("good", "direct") or (mode in ("partial", "custom") and character != "a")
        tu += f"<{code(glyph):04X}> <{ord(character) if mapped else 0:04X}>\n"
tu += "endbfchar endcmap end end"
ops = ["BT /F1 12 Tf 14 TL 72 740 Td"]
ops += ["<" + "".join(f"{code(gid[c]):04X}" for c in line) + "> Tj T*" for line in LINES]
ops += ["ET"]


def stream(data, extra=b""):
    return f"<< /Length {len(data)} ".encode() + extra + b" >>\nstream\n" + data + b"\nendstream"


objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 8 0 R >>",
    b"<< /Type /Font /Subtype /Type0 /BaseFont /ABCDEF+DejaVuSans /Encoding /Identity-H /DescendantFonts [5 0 R] /ToUnicode 7 0 R >>",
    b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ABCDEF+DejaVuSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 6 0 R /DW 600 /W [0 ["
    + " ".join(map(str, widths)).encode()
    + b"]] /CIDToGIDMap /Identity >>",
    b"<< /Type /FontDescriptor /FontName /ABCDEF+DejaVuSans /Flags 4 /FontBBox [-1000 -300 2000 1000] /ItalicAngle 0 /Ascent 928 /Descent -236 /CapHeight 729 /StemV 80 /FontFile2 9 0 R >>",
    stream(tu.encode()),
    stream("\n".join(ops).encode()),
    stream(prog, f"/Length1 {len(prog)}".encode()),
]
if mode == "custom":
    objs[3] = objs[3].replace(b"/Encoding /Identity-H", b"/Encoding 10 0 R")
    encoding = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap "
    encoding += "/CMapName /Private def /CMapType 1 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n"
    encoding += f"{len(gid)} begincidchar\n"
    encoding += "".join(f"<{code(g):04X}> {g}\n" for g in sorted(gid.values()))
    encoding += "endcidchar endcmap end end"
    objs.append(stream(encoding.encode()))
out = bytearray(b"%PDF-1.5\n")
offs = []
for i, b in enumerate(objs, 1):
    offs.append(len(out))
    out += f"{i} 0 obj\n".encode() + b + b"\nendobj\n"
x = len(out)
out += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
out += b"".join(f"{offset:010} 00000 n \n".encode() for offset in offs)
out += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode()
Path(sys.argv[1]).write_bytes(out)
