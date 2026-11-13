from pathlib import Path
import re, subprocess, sys, textwrap

source, english_script, out = map(Path, sys.argv[1:4])
variant = sys.argv[4]
control = sys.argv[5] if len(sys.argv) > 5 else ""
namespace = {"__name__": "fixture"}
exec(english_script.read_text(), namespace)
original = source.read_bytes()
prior = int(re.findall(rb"startxref\s+(\d+)", original)[-1])
khmer = subprocess.check_output(["mutool", "show", "-b", source, "8"])
# Reuse the public fixture's embedded Khmer font and text, then add independent
# visible Helvetica text in two columns below and beside the smaller Khmer block.
latin = " ".join(namespace["LINES"] * (2 if control in ("ordered", "overlap", "mixed-map") else 3))
spacing = {"clean": 0, "tc1": 1, "tc2": 2, "tc3": 3, "tc4": 4, "tc6": 6, "fragmented": 0, "garbled": 0}[variant]
if variant == "fragmented":
    latin = " ".join(" ".join(word[i : i + 2] for i in range(0, len(word), 2)) for word in latin.split())
lines = textwrap.wrap(latin, width=38 if spacing or control in ("ordered", "overlap", "mixed-map") else 62)
commands = []
for i, line in enumerate(lines):
    ordered_fixture = control in ("ordered", "overlap", "mixed-map")
    x = 40 + (280 if not ordered_fixture else 185) * (i // 26 if ordered_fixture else i // 32)
    y = (410 - 14 * (i % 26)) if ordered_fixture else (470 - 13 * (i % 32))
    text = line.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
    commands.append(f"1 0 0 1 {x} {y} Tm ({text}) Tj")
content = (
    b"q 0.75 0 0 0.75 0 218 cm\n"
    + khmer
    + f"\nQ\nBT /EN 9 Tf {spacing} Tc\n".encode()
    + "\n".join(commands).encode()
    + b"\nET\n"
)
if variant == "garbled":
    content += b"BT /BAD 9 Tf 1 0 0 1 50 35 Tm (NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN) Tj ET\n"
if control == "hidden":
    content = content.replace(b"BT /F1 18 Tf", b"BT /F1 18 Tf 3 Tr", 1)
    content += b"BT /EN 11 Tf 1 0 0 1 54 770 Tm (Community water volunteers collect supplies at the library.) Tj ET\n"
if control in ("ordered", "overlap", "mixed-map"):
    content = content.replace(b"0 218 cm", b"0 168 cm")
    content += b"BT /EN 12 Tf 1 0 0 1 54 770 Tm (Morning volunteers prepare the auditorium.) Tj 1 0 0 1 54 28 Tm (Evening visitors collect supplies at reception.) Tj ET\n"
if control == "overlap":
    content = content.replace(b"BT /F1 18 Tf", b"BT /F1 18 Tf 1 g", 1)
    content = content.replace(b"BT /EN 9 Tf", b"BT 0 g /EN 9 Tf", 1)
    content += b"BT /EN 12 Tf 1 0 0 1 54 724 Tm (Visible library volunteers prepare supplies.) Tj ET\n"
if control == "mixed-map":
    content = content.replace(
        b"(Evening visitors collect supplies at reception.) Tj",
        b"(Evening visitors collect supplies at reception. Show your ) Tj /BAD 12 Tf (id) Tj /EN 12 Tf ( card.) Tj",
    )
    content += (
        b"BT /BAD 12 Tf 1 0 0 1 54 48 Tm (Volunteers deliver library supplies before the community meeting.) Tj ET\n"
    )
if control == "wrong-map":
    content = content[content.index(b"\nQ\n") + 3 :]
    content += b"BT /BAD 12 Tf 1 0 0 1 54 760 Tm (Volunteers deliver library supplies before the community meeting.) Tj 1 0 0 1 54 735 Tm (Families can collect the remaining equipment at the reception desk.) Tj ET\n"
objects = {
    3: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R /EN 11 0 R /BAD 12 0 R >> >> /Contents 8 0 R >>",
    8: f"<< /Length {len(content)} >>\nstream\n".encode() + content + b"endstream",
    11: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
}
cmap = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /Control def /CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar <4E> <FFFD> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end"
if control in ("wrong-map", "mixed-map"):
    letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    sample = ["ស្រ", "លា", "ញ់", "ប្រ", "ទេ", "សា", "ខ្ញុំ"]
    label_map = {ord("i"): "សាលា", ord("d"): "រៀន"} if control == "mixed-map" else {}
    mapping = " ".join(
        f"<{byte:02X}> <{label_map.get(byte, sample[i % len(sample)]).encode('utf-16-be').hex()}>"
        for i, byte in enumerate(letters)
    ).encode()
    cmap = cmap.replace(b"1 beginbfchar <4E> <FFFD>", b"52 beginbfchar " + mapping)
objects[12] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 13 0 R >>"
objects[13] = f"<< /Length {len(cmap)} >>\nstream\n".encode() + cmap + b"\nendstream"
data = bytearray(original + b"\n")
offsets = {}
for num, obj in objects.items():
    offsets[num] = len(data)
    data.extend(f"{num} 0 obj\n".encode() + obj + b"\nendobj\n")
xref = len(data)
data.extend(b"xref\n")
for num, offset in offsets.items():
    data.extend(f"{num} 1\n{offset:010d} 00000 n \n".encode())
data.extend(f"trailer\n<< /Size 14 /Root 1 0 R /Prev {prior} >>\nstartxref\n{xref}\n%%EOF\n".encode())
out.write_bytes(data)
