"""Derive the short, fully mapped Khmer page from cluster-complete-mapped.pdf (stdlib only).

Keeps the first GLYPHS glyphs of the page and maps every continuation glyph to U+200B, so the
text layer carries no U+FFFD marker.
"""

import re
import sys
from pathlib import Path

GLYPHS = 14
src = Path(sys.argv[1]).read_bytes()
objs = {int(n): body for n, body in re.findall(rb"(\d+) 0 obj\n(.*?)\nendobj\n", src, re.DOTALL)}


def stream_of(body):
    return body.split(b"stream\n", 1)[1].rsplit(b"\nendstream", 1)[0]


def st(d, extra=b""):
    return b"<< /Length %d " % len(d) + extra + b">>\nstream\n" + d + b"\nendstream"


ops = stream_of(objs[8]).decode().split("\n")
tj = [o for o in ops if o.endswith("Tj")][:GLYPHS]
content = "\n".join(["BT /F1 18 Tf", *tj, "ET"])
mapped = {int(c, 16): t for c, t in re.findall(r"<([0-9A-F]{4})> <([0-9A-F]+)>", stream_of(objs[7]).decode())}
cm = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n"
cm += f"{GLYPHS} beginbfchar\n"
for cid in range(1, GLYPHS + 1):
    text = bytes.fromhex(mapped[cid]).decode("utf-16-be") if cid in mapped else "\u200b"
    cm += f"<{cid:04X}> <{text.encode('utf-16-be').hex().upper()}>\n"
cm += "endbfchar\nendcmap end end"
objs[7] = st(cm.encode())
objs[8] = st(content.encode())
out = bytearray(b"%PDF-1.5\n")
offs = []
for i in sorted(objs):
    offs.append(len(out))
    out += b"%d 0 obj\n" % i + objs[i] + b"\nendobj\n"
x = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offs)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
Path(sys.argv[2]).write_bytes(out)
