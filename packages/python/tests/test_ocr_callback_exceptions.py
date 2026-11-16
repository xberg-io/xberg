"""Exercise Python OCR callback exceptions through native extraction."""

import asyncio
import unittest
from pathlib import Path

import xberg

_FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"


class _BackendCancelledError(RuntimeError):
    """Cancellation raised by the caller's own OCR backend."""


class _FailingOcrBackend:
    def __init__(self) -> None:
        self.failure = _BackendCancelledError("stopped by the caller")
        self.calls = 0

    def name(self) -> str:
        return "ocr-callback-exception-identity"

    def version(self) -> str:
        return "1.0.0"

    def supports_language(self, lang: str) -> bool:
        return lang == "eng"

    def backend_type(self) -> xberg.OcrBackendType:
        return xberg.OcrBackendType.custom

    def process_image(self, image_bytes: bytes, _config: xberg.OcrConfig) -> xberg.ExtractedDocument:
        assert image_bytes
        self.calls += 1
        raise self.failure


class OcrCallbackExceptionTests(unittest.IsolatedAsyncioTestCase):
    """Verify that extraction returns the callback's original exception object."""

    async def _expect_callback_exception(self, fixture: Path, mime_type: str, minimum_calls: int) -> None:
        backend = _FailingOcrBackend()
        image = xberg.ExtractInput(kind="bytes", bytes=await asyncio.to_thread(fixture.read_bytes), mime_type=mime_type)
        config = xberg.ExtractionConfig(
            use_cache=False,
            force_ocr=True,
            ocr=xberg.OcrConfig(backend=backend.name(), language=["eng"]),
        )
        caught = None
        xberg.register_ocr_backend(backend)
        try:
            try:
                await xberg.extract(image, config)
            except _BackendCancelledError as error:
                caught = error
            else:
                self.fail("extraction unexpectedly succeeded")
        finally:
            xberg.unregister_ocr_backend(backend.name())

        assert caught is backend.failure
        assert backend.calls >= minimum_calls

    async def test_png_returns_the_original_backend_exception(self) -> None:
        """Image extraction returns the callback exception directly."""
        await self._expect_callback_exception(_FIXTURES / "images/test_hello_world.png", "image/png", 1)

    async def test_pdf_keeps_the_original_exception_through_all_page_failure(self) -> None:
        """The aggregate retains the original exception from a failed scan page."""
        await self._expect_callback_exception(
            _FIXTURES / "repros/label-scan-over-image-budget/label-scan-two-pages.pdf", "application/pdf", 2
        )
