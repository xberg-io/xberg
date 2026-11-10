"""Derive a Khmer page whose text layer is echoed from character codes, from cluster-complete-mapped.pdf (stdlib only).

The page keeps the font program of the source but drops its /ToUnicode map and sets the content stream to
show the code points of real Khmer text directly. The decoder has no mapping to read, so it echoes each code as
the character it spells: the layer is valid Khmer, and every span carries fallback provenance, which flags the
page for OCR on that ground alone.
"""

import re
import sys
from pathlib import Path

LINES = [
    "ខ្ញុំ ស្រលាញ់ ប្រទេស កម្ពុជា",
    "សួស្តី ភ្នំពេញ ក្រុង សៀវភៅ",
    "សាលារៀន សូម អរគុណ ខ្ញុំ",
    "ស្រលាញ់ ប្រទេស កម្ពុជា សួស្តី",
    "ភ្នំពេញ ក្រុង សៀវភៅ សាលារៀន",
    "សូម អរគុណ ខ្ញុំ ស្រលាញ់ ប្រទេស",
]
src = Path(sys.argv[1]).read_bytes()
objs = {int(n): body for n, body in re.findall(rb"(\d+) 0 obj\n(.*?)\nendobj\n", src, re.DOTALL)}


def st(d, extra=b""):
    return b"<< /Length %d " % len(d) + extra + b">>\nstream\n" + d + b"\nendstream"


ops = ["BT /F1 18 Tf"]
for row, line in enumerate(LINES):
    codes = "".join(f"{ord(c):04X}" for c in line if c != " ")
    ops.append(f"1 0 0 1 72 {740 - row * 43} Tm <{codes}> Tj")
ops.append("ET")


def replaced(body, old, new):
    if old not in body:
        sys.exit(f"pattern {old!r} not found: the source PDF is not the expected cluster-complete-mapped.pdf")
    return body.replace(old, new)


objs[4] = replaced(objs[4], b" /ToUnicode 7 0 R", b"")
objs[5] = replaced(objs[5], b" /CIDToGIDMap 10 0 R", b" /CIDToGIDMap /Identity")
objs[8] = st("\n".join(ops).encode())
for unused in (7, 10):
    if unused not in objs:
        sys.exit(f"object {unused} not found: the source PDF is not the expected cluster-complete-mapped.pdf")
    del objs[unused]
out = bytearray(b"%PDF-1.5\n")
offs = {}
for i in sorted(objs):
    offs[i] = len(out)
    out += b"%d 0 obj\n" % i + objs[i] + b"\nendobj\n"
size = max(objs) + 1
x = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % size
out += b"".join(b"%010d 00000 n \n" % offs[i] if i in offs else b"0000000000 65535 f \n" for i in range(1, size))
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (size, x)
Path(sys.argv[2]).write_bytes(out)
