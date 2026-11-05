import argparse
import hashlib
import json
import shutil
import struct
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory

p = argparse.ArgumentParser()
p.add_argument("--font", type=Path, required=True)
p.add_argument("--out", type=Path, required=True)
args = p.parse_args()
font = args.font.read_bytes()
assert hashlib.sha256(font).hexdigest() == "5c1ec068352f1e1fe8e7a3218230360d054e104d377ef40c0423b17c34c3c259"
words = ["ខ្ញុំ", "ស្រលាញ់", "ប្រទេស", "កម្ពុជា", "សួស្តី", "ភ្នំពេញ", "ក្រុង", "សៀវភៅ", "សាលារៀន", "សូម", "អរគុណ"]
lines = [" ".join(words[(i * 3 + j) % len(words)] for j in range(4)) for i in range(8)]
# #2536 gives the mapping rule and count without a transcript. These lines bring the source to 222.
lines.extend(["ខ្ញុំ ខ្ញុំ ខ្ញុំ ខ្ញុំ"] * 2)
shape = shutil.which("hb-shape")
subset_font = shutil.which("hb-subset")
if not shape or not subset_font:
    p.error("HarfBuzz hb-shape and hb-subset are required")
shape = str(Path(shape).resolve(strict=True))
subset_font = str(Path(subset_font).resolve(strict=True))
ops = ["BT /F1 18 Tf"]
gids = [0]
cluster_map = {}
partial_map = {}
for row, line in enumerate(lines):
    glyphs = json.loads(
        subprocess.check_output([shape, "--no-glyph-names", "-O", "json", str(args.font), line], text=True)
    )
    cluster_starts = sorted({g["cl"] for g in glyphs})
    next_cluster = dict(zip(cluster_starts, [*cluster_starts[1:], len(line)], strict=True))
    first = {}
    x, y = 72, 740 - row * 43
    for g in glyphs:
        cid = len(gids)
        gids.append(g["g"])
        if g["cl"] not in first:
            first[g["cl"]] = cid
            text = line[g["cl"] : next_cluster[g["cl"]]]
            cluster_map[cid] = text
            if len(text) == 1:
                partial_map[cid] = text
        ops.append(f"1 0 0 1 {x + g['dx'] * 0.018:.2f} {y + g['dy'] * 0.018:.2f} Tm <{cid:04X}> Tj")
        x += g["ax"] * 0.018
ops.append("ET")
with TemporaryDirectory() as temp:
    subset = Path(temp) / "embedded-no-cmap.ttf"
    subprocess.run(
        [
            subset_font,
            str(args.font),
            "--retain-gids",
            "--gids=" + ",".join(map(str, sorted(set(gids)))),
            "--drop-tables=cmap,post",
            "--no-hinting",
            "--no-layout-closure",
            "-o",
            str(subset),
        ],
        check=True,
    )
    embedded = subset.read_bytes()


def st(d, extra=b""):
    return b"<< /Length %d " % len(d) + extra + b">>\nstream\n" + d + b"\nendstream"


def build(path, mapping):
    cm = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n"
    entries = list(mapping.items())
    for i in range(0, len(entries), 100):
        batch = entries[i : i + 100]
        cm += f"{len(batch)} beginbfchar\n"
        cm += "".join(f"<{cid:04X}> <{text.encode('utf-16-be').hex().upper()}>\n" for cid, text in batch)
        cm += "endbfchar\n"
    cm += "endcmap end end"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 8 0 R >>",
        b"<< /Type /Font /Subtype /Type0 /BaseFont /NotoSansKhmer-Regular /Encoding /Identity-H /DescendantFonts [5 0 R] /ToUnicode 7 0 R >>",
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /NotoSansKhmer-Regular /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 6 0 R /DW 600 /CIDToGIDMap 10 0 R >>",
        b"<< /Type /FontDescriptor /FontName /NotoSansKhmer-Regular /Flags 4 /FontBBox [-500 -500 2000 1500] /ItalicAngle 0 /Ascent 1000 /Descent -300 /CapHeight 700 /StemV 80 /FontFile2 9 0 R >>",
        st(cm.encode()),
        st("\n".join(ops).encode()),
        st(embedded, f"/Length1 {len(embedded)} ".encode()),
        st(struct.pack(f">{len(gids)}H", *gids)),
    ]
    out = bytearray(b"%PDF-1.5\n")
    offs = []
    for i, obj in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + obj + b"\nendobj\n"
    x = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offs)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, x)
    path.write_bytes(out)
    return hashlib.sha256(out).hexdigest()


args.out.mkdir(parents=True, exist_ok=True)
proof = {
    "font_sha": hashlib.sha256(font).hexdigest(),
    "glyphs": len(gids) - 1,
    "mapped_clusters": len(cluster_map),
    "unmapped_cluster_continuations": len(gids) - 1 - len(cluster_map),
    "mapped_khmer_count": sum("\u1780" <= c <= "\u17ff" for w in cluster_map.values() for c in w),
    "literal_lines": lines,
    "partial_mapped_count": len(partial_map),
}
proof["cluster_pdf_sha"] = build(args.out / "cluster-complete-mapped.pdf", cluster_map)
proof["partial_pdf_sha"] = build(args.out / "genuinely-partial-mapped.pdf", partial_map)
print(json.dumps(proof, ensure_ascii=False, indent=2))
