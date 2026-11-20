"""Generate original synthetic transport fixtures using only the standard library."""

import argparse
import hashlib
import json
from pathlib import Path
from zipfile import ZIP_STORED, ZipFile, ZipInfo

MIB = 1024**2
MARKER = "Xberg transport acceptance 3152"
CASES = {"pdf-40.pdf": 40, "pdf-90.pdf": 90, "docx-90.docx": 90}
BLOCK = hashlib.shake_256(b"xberg-3152 synthetic padding v1").digest(MIB)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def pdf(path: Path, mib: int) -> None:
    page = f"BT /F1 18 Tf 50 740 Td ({MARKER}) Tj ET\n".encode()
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        f"<< /Length {len(page)} >>\nstream\n".encode() + page + b"endstream",
        None,
    ]
    with path.open("wb") as out:
        out.write(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
        offsets = []
        for index, body in enumerate(objects, 1):
            offsets.append(out.tell())
            out.write(f"{index} 0 obj\n".encode())
            if body is None:
                out.write(f"<< /Length {mib * MIB} >>\nstream\n".encode())
                for _ in range(mib):
                    out.write(BLOCK)
                out.write(b"\nendstream")
            else:
                out.write(body)
            out.write(b"\nendobj\n")
        xref = out.tell()
        out.write(b"xref\n0 7\n0000000000 65535 f \n")
        for offset in offsets:
            out.write(f"{offset:010d} 00000 n \n".encode())
        out.write(f"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())


def docx(path: Path, mib: int) -> None:
    parts = {
        "[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="bin" ContentType="application/octet-stream"/>'
        '<Override PartName="/word/document.xml" '
        'ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
        "</Types>",
        "_rels/.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/'
        'officeDocument" Target="word/document.xml"/></Relationships>',
        "word/document.xml": '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        f"<w:body><w:p><w:r><w:t>{MARKER}</w:t></w:r></w:p></w:body></w:document>",
    }

    def entry(name: str) -> ZipInfo:
        info = ZipInfo(name, (2000, 1, 1, 0, 0, 0))
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        info.compress_type = ZIP_STORED
        return info

    with ZipFile(path, "w") as archive:
        for name, xml in parts.items():
            archive.writestr(entry(name), xml.encode())
        padding = entry("word/media/transport-padding.bin")
        padding.file_size = mib * MIB
        with archive.open(padding, "w") as stream:
            for _ in range(mib):
                stream.write(BLOCK)


def generate(directory: Path) -> dict[str, dict[str, int | str]]:
    directory.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name, mib in CASES.items():
        path = directory / name
        (pdf if path.suffix == ".pdf" else docx)(path, mib)
        manifest[name] = {"bytes": path.stat().st_size, "sha256": digest(path)}
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    print(json.dumps(generate(args.directory), indent=2))
