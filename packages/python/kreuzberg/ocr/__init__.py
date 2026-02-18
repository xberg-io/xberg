"""Python OCR backend implementations.

These backends can be imported and manually registered, or they will be
auto-registered when kreuzberg is imported (if their dependencies are installed).

Each backend has a separate optional dependency group:
- EasyOCR: pip install "kreuzberg[easyocr]"
- RapidOCR: pip install "kreuzberg[rapidocr]"

Note: PaddleOCR is now a native Rust backend available in all non-WASM bindings
via the 'paddle-ocr' feature flag. No Python dependency is required.
"""

from __future__ import annotations

__all__ = ["EasyOCRBackend", "OcrBackendProtocol", "RapidOCRBackend"]

from kreuzberg.ocr.protocol import OcrBackendProtocol

try:
    from kreuzberg.ocr.easyocr import EasyOCRBackend
except ImportError:
    EasyOCRBackend = None  # type: ignore[assignment,misc]

try:
    from kreuzberg.ocr.rapidocr import RapidOCRBackend
except ImportError:
    RapidOCRBackend = None  # type: ignore[assignment,misc]
