import sys
from pathlib import Path

root = Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)
content = b"BT /F1 24 Tf 72 720 Td (Hello) Tj ET"
objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
padding = bytes(1048576)
for mib in (0, 40, 90):
    with (root / f"unref{mib}.pdf").open("wb") as pdf:
        pdf.write(b"%PDF-1.7\n")
        offsets = []
        for number, body in enumerate(objects, start=1):
            offsets.append(pdf.tell())
            pdf.write(b"%d 0 obj\n" % number + body + b"\nendobj\n")
        offsets.append(pdf.tell())
        pdf.write(b"6 0 obj\n<< /Length %d >>\nstream\n" % (mib * len(padding)))
        for _ in range(mib):
            pdf.write(padding)
        pdf.write(b"\nendstream\nendobj\n")
        xref = pdf.tell()
        pdf.write(b"xref\n0 7\n0000000000 65535 f \n")
        for offset in offsets:
            pdf.write(b"%010d 00000 n \n" % offset)
        pdf.write(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % xref)
