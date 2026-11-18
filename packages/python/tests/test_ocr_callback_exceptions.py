"""Exercise Python callback error compatibility through native extraction."""

import asyncio
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import pytest
import xberg

_FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"


class _BackendCancelledError(RuntimeError):
    """Cancellation raised by the caller's own OCR backend."""


class _FailingOcrBackend:
    def __init__(self) -> None:
        self.raised: list[_BackendCancelledError] = []

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
        failure = _BackendCancelledError(f"stopped by the caller on read {len(self.raised) + 1}")
        self.raised.append(failure)
        raise failure


class OcrCallbackExceptionTests(unittest.IsolatedAsyncioTestCase):
    """Verify that extraction returns the callback's original exception object."""

    async def _expect_callback_exception(self, fixture: Path, mime_type: str, minimum_calls: int) -> None:
        backend = _FailingOcrBackend()
        image = xberg.ExtractInput(kind="bytes", bytes=await asyncio.to_thread(fixture.read_bytes), mime_type=mime_type)
        config = xberg.ExtractionConfig(
            use_cache=False,
            force_ocr=True,
            # One page per batch keeps OCR callback order aligned with PDF page order.
            concurrency=xberg.ConcurrencyConfig(max_threads=1),
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

        assert len(backend.raised) >= minimum_calls
        assert caught is backend.raised[0]

    async def test_png_returns_the_original_backend_exception(self) -> None:
        """Image extraction returns the callback exception directly."""
        await self._expect_callback_exception(_FIXTURES / "images/test_hello_world.png", "image/png", 1)

    async def test_pdf_keeps_the_first_exception_through_all_page_failure(self) -> None:
        """The aggregate retains the first exception when every page fails."""
        await self._expect_callback_exception(
            _FIXTURES / "repros/label-scan-over-image-budget/label-scan-two-pages.pdf", "application/pdf", 2
        )

    async def test_native_pdf_ocr_failure_has_no_python_cause(self) -> None:
        """Native page errors produce an OcrError without a Python exception source."""
        fixture = _FIXTURES / "repros/label-scan-over-image-budget/label-scan-two-pages.pdf"
        document = xberg.ExtractInput(
            kind="bytes", bytes=await asyncio.to_thread(fixture.read_bytes), mime_type="application/pdf"
        )
        with TemporaryDirectory() as tessdata:
            # A present but invalid language pack avoids downloads and makes the native OCR reads fail.
            (Path(tessdata) / "eng.traineddata").write_bytes(b"invalid Tesseract model")
            config = xberg.ExtractionConfig(
                use_cache=False,
                force_ocr=True,
                # Registry lookup is case-insensitive. Only lowercase "tesseract" opts into implicit Paddle fallback.
                ocr=xberg.OcrConfig(backend="TESSERACT", language=["eng"], tessdata_path=tessdata),
            )
            with pytest.raises(xberg.OcrError, match=r"OCR failed on all 2 page\(s\)") as caught:
                await xberg.extract(document, config)
        assert caught.value.__cause__ is None


class _FailingValidator:
    def name(self) -> str:
        return "callback-error-classification-validator"

    def validate(self, _document: xberg.ExtractedDocument, _config: xberg.ExtractionConfig) -> None:
        raise RuntimeError("validator failure")


class NonOcrCallbackCompatibilityTests(unittest.IsolatedAsyncioTestCase):
    """Verify the existing classification of callbacks outside OCR."""

    async def test_validator_keeps_the_batch_other_classification(self) -> None:
        """Batch extraction still reports validator failures as Other."""
        validator = _FailingValidator()
        xberg.register_validator(validator)
        self.addCleanup(xberg.unregister_validator, validator.name())

        document = xberg.ExtractInput(kind="bytes", bytes=b"Hello world", mime_type="text/plain")
        batch = await xberg.extract_batch([document], xberg.ExtractionConfig(use_cache=False))

        assert len(batch.errors) == 1
        error = batch.errors[0]
        assert (error.error_type, error.code) == ("other", 1017)
        assert "validator failure" in error.message
