import sys

LINES = [
    "The committee met on Tuesday to review the annual budget and the staffing plan for the library.",
    "Members agreed to publish the minutes, to extend the opening hours on weekends, and to order new books.",
    "The treasurer reported that the reading room repairs were finished on time and under the planned cost.",
    "A volunteer programme for schoolchildren will begin in March, with weekly sessions led by two teachers.",
    "The next meeting is set for the first Tuesday of the following month at the town hall at seven o'clock.",
    "Visitors are reminded that the archive remains closed on public holidays and during the annual inventory.",
    "Questions about membership, donations or loans can be sent to the secretary by post or by electronic mail.",
]


def build() -> bytes:
    text = "".join("({}) Tj T*\n".format(line.replace("'", "\\'")) for line in LINES * 3)
    content = f"BT /F1 11 Tf 14 TL 50 740 Td\n{text}ET"
    page = (
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
    )
    objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page,
        f"<< /Length {len(content)} >>\nstream\n{content}\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out = b"%PDF-1.4\n"
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{number} 0 obj\n{body}\nendobj\n".encode()
    xref = len(out)
    out += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    for offset in offsets:
        out += f"{offset:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return out


if __name__ == "__main__":
    with open(sys.argv[1], "wb") as handle:
        handle.write(build())
