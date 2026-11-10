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
objs[4] = objs[4].replace(b" /ToUnicode 7 0 R", b"")
objs[5] = objs[5].replace(b" /CIDToGIDMap 10 0 R", b" /CIDToGIDMap /Identity")
objs[7] = st(b"% unused")
objs[8] = st("\n".join(ops).encode())
objs.pop(10)
out = bytearray(b"%PDF-1.5\n")
offs = []
for i in sorted(objs):
    offs.append(len(out))
    out += b"%d 0 obj\n" % i + objs[i] + b"\nendobj\n"
x = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offs)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
Path(sys.argv[2]).write_bytes(out)
