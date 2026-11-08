"""Regenerate the issue #2322 ligature-advance fixtures with the lock-pinned fonttools.

Usage: python3 generate.py OUTPUT.pdf MODE, MODE one of direct, letters, codepoint, plain, plain_direct.
"""

import io
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

output = Path(sys.argv[1])
mode = sys.argv[2]
if mode not in ("direct", "letters", "codepoint", "plain", "plain_direct"):
    raise ValueError(f"unknown mode {mode}")

SENT = [
    "The financial plan shows a careful reserve for the first year.",
    "Each office reflects the effort of the staff and the final review.",
    "The influence of the flow on the efficient system is specific.",
]
LIGS = [("ffi", "ﬃ"), ("ff", "ﬀ"), ("fi", "ﬁ"), ("fl", "ﬂ")]
LETTERS = {lig: seq for seq, lig in LIGS}
ligated = mode in ("direct", "letters", "codepoint")


def ligate(s):
    for seq, lig in LIGS:
        s = s.replace(seq, lig)
    return s


def pieces(s):
    """The show strings of one TJ line: each string ends after a ligature (or after its letters)."""
    out, cur, i = [], "", 0
    while i < len(s):
        for seq, lig in LIGS:
            token = lig if ligated else seq
            if s.startswith(token, i):
                out.append(cur + token)
                cur, i = "", i + len(token)
                break
        else:
            cur += s[i]
            i += 1
    return out + ([cur] if cur else [])


lines = [ligate(s) if ligated else s for s in SENT]
tt = TTFont(str(Path(__file__).resolve().parents[3] / "src/fonts/assets/DejaVuSans.ttf"), recalcTimestamp=False)
o = subset.Options()
o.layout_features = ["liga"]
o.hinting = False
o.notdef_outline = True
s = subset.Subsetter(o)
s.populate(text="".join(SENT) + "".join(lig for _, lig in LIGS))
s.subset(tt)
order = tt.getGlyphOrder()
cm = tt.getBestCmap()
used = sorted(set("".join(lines)))
gid = {c: order.index(cm[ord(c)]) for c in used}
if mode in ("direct", "plain_direct"):
    del tt["cmap"]  # no font cmap: the direct CID route paints every run, the reference raster
buf = io.BytesIO()
tt.save(buf)
prog = buf.getvalue()
w = tt["hmtx"].metrics
widths = [int(w[g][0] * 1000 / tt["head"].unitsPerEm) for g in order]


def unicode_of(c):
    if c in LETTERS and mode != "codepoint":
        return "".join(f"{ord(x):04X}" for x in LETTERS[c])
    return f"{ord(c):04X}"


tu = (
    "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def "
    "/CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n"
    f"{len(gid)} beginbfchar\n"
)
tu += "".join(f"<{g:04X}> <{unicode_of(c)}>\n" for g, c in sorted((g, c) for c, g in gid.items()))
tu += "endbfchar endcmap end end"


def hexs(text):
    return "<" + "".join(f"{gid[c]:04X}" for c in text) + ">"


ops = ["BT /F1 12 Tf 20 TL 72 720 Td"]
ops += [hexs(line) + " Tj T*" for line in lines]
ops += ["[" + " ".join(hexs(p) for p in pieces(line)) + "] TJ T*" for line in lines]
ops += ["ET"]
# the size in the text matrix, not in Tf, with horizontal scaling and character spacing ~keep
ops += ["BT /F1 1 Tf 90 Tz 0.02 Tc 12 0 0 12 72 600 Tm"]
ops += ["[" + " ".join(hexs(p) for p in pieces(lines[1])) + "] TJ", "ET"]


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
out = bytearray(b"%PDF-1.5\n")
offs = []
for i, b in enumerate(objs, 1):
    offs.append(len(out))
    out += f"{i} 0 obj\n".encode() + b + b"\nendobj\n"
x = len(out)
out += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
out += b"".join(f"{offset:010} 00000 n \n".encode() for offset in offs)
out += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode()
output.write_bytes(out)
