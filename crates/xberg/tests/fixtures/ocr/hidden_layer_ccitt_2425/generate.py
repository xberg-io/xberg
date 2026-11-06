"""Regenerate the synthetic PDF fixtures from the independent source text."""

import argparse
import shutil
import subprocess
import tempfile
import zlib
from pathlib import Path

from PIL import Image

LINES_Y = (135, 245, 279, 313, 435, 469, 503, 625, 659, 693, 1215)
PARTIAL_ROWS = (0, 1, 2, 3, 10)
PAGE_WIDTH, PAGE_HEIGHT, DPI = 612, 792, 150
RASTER_WIDTH, RASTER_HEIGHT = 1275, 1650


def executable(name: str) -> Path:
    result = shutil.which(name)
    if result is None:
        raise FileNotFoundError(f"required fixture generator tool: {name}")
    return Path(result).resolve()


def operators(lines: list[str], indices: range | tuple[int, ...], *, hidden: bool) -> str:
    rows = "".join(
        f"1 0 0 1 67.2 {PAGE_HEIGHT - LINES_Y[index] * 72 / DPI:g} Tm ({lines[index]}) Tj\n" for index in indices
    )
    return f"BT /F1 12 Tf {3 if hidden else 0} Tr\n{rows}ET\n"


def stream(fields: str, data: bytes) -> bytes:
    return f"<< {fields} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"


def write_pdf(path: Path, content: str | tuple[str, str], image: bytes | None = None) -> None:
    resources = "/Font << /F1 4 0 R >>"
    if image is not None:
        resources += " /XObject << /Scan 6 0 R >>"
    streams = (content,) if isinstance(content, str) else content
    if len(streams) == 2:
        assert image is not None
    contents = "5 0 R" if len(streams) == 1 else "[5 0 R 7 0 R]"
    entries = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] "
        f"/Resources << {resources} >> /Contents {contents} >>".encode(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        stream("", streams[0].encode()),
    ]
    if image is not None:
        entries.append(image)
    if len(streams) == 2:
        entries.append(stream("", streams[1].encode()))
    output = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for index, entry in enumerate(entries, 1):
        offsets.append(len(output))
        output.extend(f"{index} 0 obj\n".encode() + entry + b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    output.extend("".join(f"{offset:010d} 00000 n \n" for offset in offsets[1:]).encode())
    output.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    path.write_bytes(output)


def encode_images(scratch: Path, lines: list[str]) -> tuple[bytes, bytes, bytes]:
    source = scratch / "source"
    write_pdf(source.with_suffix(".pdf"), operators(lines, range(len(lines)), hidden=False))
    subprocess.run(
        [
            executable("pdftoppm"),
            "-gray",
            "-r",
            str(DPI),
            "-f",
            "1",
            "-l",
            "1",
            "-singlefile",
            source.with_suffix(".pdf"),
            source,
        ],
        check=True,
    )
    with Image.open(source.with_suffix(".pgm")) as gray:
        bilevel = gray.point(lambda value: 255 if value >= 128 else 0).convert("1")
    assert bilevel.size == (RASTER_WIDTH, RASTER_HEIGHT)
    tiff = scratch / "bilevel.tif"
    group4 = scratch / "g4.tif"
    bilevel.save(tiff)
    subprocess.run([executable("tiffcp"), "-c", "g4", "-r", str(RASTER_HEIGHT), tiff, group4], check=True)
    with Image.open(group4) as decoded:
        assert decoded.tag_v2[259] == 4
        assert decoded.tag_v2[278] == RASTER_HEIGHT
        (offset,) = decoded.tag_v2[273]
        (length,) = decoded.tag_v2[279]
        assert decoded.tobytes() == bilevel.tobytes()
    encoded = group4.read_bytes()[offset : offset + length]
    assert len(encoded) == length
    pixels = bilevel.convert("L").tobytes()
    prefix = f"/Type /XObject /Subtype /Image /ColorSpace /DeviceGray /Width {RASTER_WIDTH} /Height {RASTER_HEIGHT}"
    flate_stream = stream(f"{prefix} /BitsPerComponent 8 /Filter /FlateDecode", zlib.compress(pixels))
    ccitt_stream = stream(
        f"{prefix} /BitsPerComponent 1 /Filter /CCITTFaxDecode "
        f"/DecodeParms << /K -1 /Columns {RASTER_WIDTH} /Rows {RASTER_HEIGHT} /BlackIs1 true >>",
        encoded,
    )
    return flate_stream, ccitt_stream, pixels


def verify_image(source: Path, scratch: Path, pixels: bytes) -> None:
    prefix = scratch / source.stem
    subprocess.run([executable("pdfimages"), "-png", source, prefix], check=True)
    with Image.open(scratch / f"{source.stem}-000.png") as decoded:
        assert decoded.convert("L").tobytes() == pixels


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path, help="new directory for the generated PDFs")
    output = parser.parse_args().output
    output.mkdir()
    lines = Path(__file__).with_name("source.txt").read_text().splitlines()
    assert len(lines) == len(LINES_Y)
    assert sum(len(line.split()) for line in lines) == 81
    with tempfile.TemporaryDirectory() as temporary:
        scratch = Path(temporary)
        flate, ccitt, pixels = encode_images(scratch, lines)
        for codec, image in (("flate8", flate), ("ccitt", ccitt)):
            for layer, indices in (("partial", PARTIAL_ROWS), ("complete", range(len(lines)))):
                path = output / f"{codec}-{layer}.pdf"
                paint = f"q {PAGE_WIDTH} 0 0 {PAGE_HEIGHT} 0 0 cm /Scan Do Q\n"
                hidden = operators(lines, indices, hidden=True)
                write_pdf(path, paint + hidden, image)
                verify_image(path, scratch, pixels)
                if codec == "ccitt":
                    split_path = output / f"{codec}-{layer}-split.pdf"
                    write_pdf(split_path, (paint, hidden), image)
                    verify_image(split_path, scratch, pixels)


if __name__ == "__main__":
    main()
