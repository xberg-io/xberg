import argparse
import io
import zlib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser()
parser.add_argument("target", type=Path)
parser.add_argument("font", type=Path)
variants = parser.add_mutually_exclusive_group()
variants.add_argument("--bilevel-only", action="store_true")
variants.add_argument("--neutral-rgb-only", action="store_true")
variants.add_argument("--jpeg-only", action="store_true")
variants.add_argument("--predictor-indexed-only", action="store_true")
args = parser.parse_args()
target, font_path = args.target, args.font.resolve(strict=True)
phrase = "Source scan keeps all words from this repeated line safely today"
tags = [
    "apple",
    "orange",
    "banana",
    "cherry",
    "peach",
    "lemon",
    "melon",
    "grape",
    "mango",
    "plum",
    "almond",
    "walnut",
    "carrot",
    "celery",
    "radish",
    "turnip",
    "olive",
    "pepper",
    "garlic",
    "potato",
    "spinach",
    "cabbage",
    "pumpkin",
    "coconut",
    "raisin",
    "papaya",
    "beetroot",
    "parsnip",
    "tomato",
    "broccoli",
    "cucumber",
    "apricot",
    "pineapple",
    "blueberry",
    "strawberry",
    "raspberry",
    "lettuce",
    "nectarine",
    "fig",
    "date",
]
lines = [phrase.rsplit(" ", 1)[0] + " " + tag for tag in tags]
target.joinpath("expected.txt").write_text("\n".join(lines) + "\n")


def _stream(data: bytes, fields: str = "") -> bytes:
    return f"<< /Length {len(data)} {fields} >>\nstream\n".encode() + data + b"\nendstream"


def _raster(height: int, packing: str) -> bytes:
    width = 1275
    image = Image.new("L", (width, height), 255)
    font = ImageFont.truetype(str(font_path), 20 if packing in {"predictor", "indexed"} else 18)
    draw = ImageDraw.Draw(image)
    for line in range(40):
        draw.text((95, 180 + line * ((height - 350) // 40)), lines[line], fill=0, font=font)
    if packing in {"predictor", "indexed"}:
        return _predictor_or_indexed(image, packing)
    if packing == "bilevel":
        image = image.convert("1")
    elif packing == "neutral-rgb":
        image = image.convert("RGB")
    if packing == "jpeg":
        buffer = io.BytesIO()
        image.save(buffer, format="JPEG", quality=88, optimize=True)
        compressed, codec = buffer.getvalue(), "DCTDecode"
    else:
        compressed, codec = zlib.compress(image.tobytes()), "FlateDecode"
    depth = 1 if packing == "bilevel" else 8
    color_space = "DeviceRGB" if packing == "neutral-rgb" else "DeviceGray"
    return _stream(
        compressed,
        f"/Type /XObject /Subtype /Image /Width {width} /Height {height} "
        f"/ColorSpace /{color_space} /BitsPerComponent {depth} /Filter /{codec}",
    )


def _predictor_or_indexed(image: Image.Image, packing: str) -> bytes:
    width, height = image.size
    samples = image.tobytes()
    prefix = f"/Type /XObject /Subtype /Image /Width {width} /Height {height}"
    if packing == "predictor":
        previous = bytes(width)
        encoded = bytearray()
        for i in range(height):
            row = samples[i * width : (i + 1) * width]
            encoded.append(2 if i % 2 else 0)
            encoded.extend(bytes((a - b) & 255 for a, b in zip(row, previous, strict=True)) if i % 2 else row)
            previous = row
        params = f"/DecodeParms << /Predictor 12 /Columns {width} /Colors 1 /BitsPerComponent 8 >>"
        fields = f"{prefix} /BitsPerComponent 8 /Filter /FlateDecode /ColorSpace /DeviceGray {params}"
    else:
        stride = (width + 7) // 8
        encoded = bytearray()
        for i in range(height):
            packed = bytearray(stride)
            for col, value in enumerate(samples[i * width : (i + 1) * width]):
                if value < 128:
                    packed[col // 8] |= 1 << (7 - col % 8)
            packed[-1] |= (1 << (stride * 8 - width)) - 1
            encoded.extend(packed)
        fields = f"{prefix} /BitsPerComponent 1 /Filter /FlateDecode /ColorSpace [/Indexed /DeviceGray 1 <FF00>]"
    return _stream(zlib.compress(encoded), fields)


def _pdf(name: str, height: int, *, mixed: bool = False, packing: str = "gray") -> None:
    pts = height * 0.48
    entries = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 {pts:g}] /Resources << /XObject << /Scan 5 0 R >> >> /Contents 4 0 R >>".encode(),
        _stream(f"q 612 0 0 {pts:g} 0 0 cm /Scan Do Q".encode()),
        _raster(height, packing),
    ]
    if mixed:
        entries[1] = b"<< /Type /Pages /Kids [6 0 R 3 0 R] /Count 2 >>"
        entries.extend(
            [
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 8 0 R >> >> /Contents 7 0 R >>",
                _stream(
                    b"BT /F1 14 Tf 1 0 0 1 72 730 Tm 25 TL (The native page remains unchanged.) Tj "
                    + b"T* (The native page remains unchanged.) Tj " * 9
                    + b"ET"
                ),
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            ]
        )
    data = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for i, entry in enumerate(entries, 1):
        offsets.append(len(data))
        data.extend(f"{i} 0 obj\n".encode() + entry + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for off in offsets[1:]:
        data.extend(f"{off:010d} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    target.joinpath(name).write_bytes(data)


if args.bilevel_only:
    _pdf("bilevel-tall.pdf", 12000, packing="bilevel")
elif args.neutral_rgb_only:
    _pdf("neutral-rgb-tall.pdf", 12000, packing="neutral-rgb")
elif args.jpeg_only:
    _pdf("jpeg-tall.pdf", 12000, packing="jpeg")
elif args.predictor_indexed_only:
    _pdf("predictor-mixed.pdf", 12000, packing="predictor")
    _pdf("indexed-inverted.pdf", 12000, packing="indexed")
else:
    _pdf("strict-tall.pdf", 9000)
    _pdf("strict-short.pdf", 3600)
    _pdf("strict-mixed.pdf", 9000, mixed=True)
