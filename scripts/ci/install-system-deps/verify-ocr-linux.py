"""Verify actual selected native OCR libraries and their ELF loader closure."""

import ctypes
import hashlib
import json
import subprocess
import sys
from pathlib import Path


def verify_licenses(prefix: Path) -> list:
    """Check notices copied from the checksum-verified published source archives."""
    expected = {
        "tesseract-5.5.5/LICENSE": "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
        "tesseract-5.5.5/AUTHORS": "b3b7a07a14c060efe17574b2946a01ecaa0a5873a8f606239cc28c6d31300109",
        "leptonica-1.87.0/leptonica-license.txt": "87829abb5bbb00b55a107365da89e9a33f86c4250169e5a1e5588505be7d5806",
    }
    for name, digest in expected.items():
        path = prefix / "share/licenses/xberg-system-ocr" / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"upstream notice missing or changed: {path}")
    return list(expected)


def main() -> None:
    """Reject stale native APIs, foreign loaded libraries and missing loader dependencies."""
    prefix = Path(sys.argv[1]).resolve(strict=True)
    licenses = verify_licenses(prefix)
    rows = []
    for filename, symbol, expected in (
        ("libtesseract.so.5.5", "TessVersion", "5.5.5"),
        ("libleptonica.so.6", "getLeptonicaVersion", "leptonica-1.87.0"),
    ):
        path = (prefix / "lib" / filename).resolve(strict=True)
        library = ctypes.CDLL(str(path))
        function = getattr(library, symbol)
        function.argtypes = []
        function.restype = ctypes.c_char_p
        version = function().decode()
        if version != expected:
            raise RuntimeError(f"{path} reports {version}, expected {expected}")
        linkage = subprocess.check_output(["ldd", str(path)], universal_newlines=True)
        if "not found" in linkage:
            raise RuntimeError(f"incomplete native loader closure: {linkage}")
        codecs = ""
        if filename == "libleptonica.so.6":
            library.getImagelibVersions.argtypes = []
            library.getImagelibVersions.restype = ctypes.c_char_p
            codecs = library.getImagelibVersions().decode()
            for codec in ("libpng", "libjpeg", "libtiff", "zlib", "libwebp", "libgif", "libopenjp2"):
                if codec + " " not in codecs:
                    raise RuntimeError(f"missing compiled OCR image codec {codec}: {codecs}")
        elif f"libleptonica.so.6 => {prefix}/lib/libleptonica.so.6" not in linkage:
            raise RuntimeError(f"Tesseract loads a different Leptonica runtime: {linkage}")
        rows.append(
            {
                "path": str(path),
                "version": version,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "ldd": linkage,
                "image_codecs": codecs,
            }
        )
    loaded = {
        Path(line.split()[-1]).resolve()
        for line in Path("/proc/self/maps").read_text().splitlines()
        if "/" in line.split()[-1] and line.split()[-1].startswith("/")
    }
    for row in rows:
        if Path(row["path"]) not in loaded:
            raise RuntimeError("selected native library was not actually loaded")
    if len(rows) != 2:
        raise RuntimeError("expected exactly two native OCR libraries")
    sys.stdout.write(
        json.dumps(
            {
                "expected_notices": 3,
                "actual_notices": len(licenses),
                "expected_libraries": 2,
                "actual_libraries": len(rows),
                "libraries": rows,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
