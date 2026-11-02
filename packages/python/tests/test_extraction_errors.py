"""Preserve native extraction error kinds across Python async and progress calls."""

import importlib.machinery
import sys
import unittest
from collections.abc import Awaitable
import xberg
import xberg._xberg as native

if sys.version_info >= (3, 12):
    from typing import override
else:
    from typing_extensions import override

PLAIN_TEXT = "Plain extraction remains available."
PARSING_ERROR_CODE = 1001
XML_LEVELS = 50
XML_DEPTH_LIMIT = 5


def _bad_pdf() -> xberg.ExtractInput:
    return xberg.ExtractInput(kind="bytes", bytes=b"not a pdf", mime_type="application/pdf")


def _plain_input() -> xberg.ExtractInput:
    return xberg.ExtractInput(kind="bytes", bytes=PLAIN_TEXT.encode(), mime_type="text/plain")


def _invalid_chunking() -> xberg.ExtractionConfig:
    return xberg.ExtractionConfig(chunking=xberg.ChunkingConfig(max_characters=10, overlap=20))


def _plain_config() -> xberg.ExtractionConfig:
    return xberg.ExtractionConfig(use_cache=False, output_format="plain")


class TypedExtractionErrorTests(unittest.IsolatedAsyncioTestCase):
    """Exercise real invalid inputs through the built native extension."""

    @override
    def setUp(self) -> None:
        """Require the real native module and its exported exception identities."""
        assert isinstance(native.__loader__, importlib.machinery.ExtensionFileLoader)
        assert any(native.__file__.endswith(s) for s in importlib.machinery.EXTENSION_SUFFIXES)
        for name in ("XbergError", "ParsingError", "SecurityError", "ValidationError", "UnsupportedFormatError"):
            assert getattr(xberg, name) is getattr(native, name)

    async def _expect_typed_error(
        self, operation: Awaitable[object], expected: type[Exception], rust_prefix: str, rust_context: str = ""
    ) -> None:
        caught = None
        try:
            await operation
        except Exception as error:
            caught = error
        assert caught is not None, "the real invalid input unexpectedly succeeded"
        message = str(caught)
        assert message.startswith(rust_prefix), f"expected the actual Rust error, received {caught!r}"
        assert len(message) > len(rust_prefix), "the Rust error must retain its context"
        if rust_context:
            assert rust_context in message
        assert type(caught) is expected
        assert isinstance(caught, xberg.XbergError)

    async def test_extract_invalid_pdf_preserves_parsing_error(self) -> None:
        """A malformed PDF raises the exported parsing exception."""
        await self._expect_typed_error(xberg.extract(_bad_pdf()), xberg.ParsingError, "Parsing error: ")

    async def test_extract_xml_depth_preserves_security_error(self) -> None:
        """The real XML depth limit raises the exported security exception."""
        document = xberg.ExtractInput(
            kind="bytes", bytes=b"<a>" * XML_LEVELS + b"</a>" * XML_LEVELS, mime_type="application/xml"
        )
        config = xberg.ExtractionConfig(security_limits=xberg.SecurityLimits(max_xml_depth=XML_DEPTH_LIMIT))
        await self._expect_typed_error(xberg.extract(document, config), xberg.SecurityError, "Security violation: ")

    async def test_extract_unsupported_mime_preserves_unsupported_format_error(self) -> None:
        """A declared unsupported MIME type retains its distinct error kind."""
        document = xberg.ExtractInput(
            kind="bytes", bytes=PLAIN_TEXT.encode(), mime_type="application/x-xberg-nonexistent"
        )
        await self._expect_typed_error(xberg.extract(document), xberg.UnsupportedFormatError, "Unsupported format: ")

    async def test_batch_global_validation_preserves_validation_error(self) -> None:
        """Invalid batch configuration raises before per-input processing."""
        await self._expect_typed_error(
            xberg.extract_batch([_plain_input()], _invalid_chunking()), xberg.ValidationError, "Validation error: "
        )

    async def test_external_redaction_bad_encoding_preserves_validation_error(self) -> None:
        """Invalid offset encoding is rejected before extraction starts."""
        operation = xberg.extract_with_external_redaction(
            _bad_pdf(), xberg.ExtractionConfig(), findings_json="[]", offset_encoding="not-an-encoding"
        )
        await self._expect_typed_error(operation, xberg.ValidationError, "Validation error: ")

    async def test_layout_uri_rejection_preserves_validation_error_before_io(self) -> None:
        """The bytes-only layout API rejects a URI before filesystem or model access."""
        document = xberg.ExtractInput(kind="uri", uri="file:///never-read-for-this-validation.pdf")
        operation = xberg.extract_with_layout_pages(document, xberg.ExtractionConfig(), pages=[])
        await self._expect_typed_error(operation, xberg.ValidationError, "Validation error: ")

    async def test_map_url_bad_crawl_config_preserves_validation_error_before_network(self) -> None:
        """Zero crawl concurrency fails configuration validation before network access."""
        config = xberg.UrlExtractionConfig(crawl=xberg.CrawlConfig(max_concurrent=0))
        operation = xberg.map_url("not a URL", config)
        await self._expect_typed_error(
            operation, xberg.ValidationError, "Validation error: ", "max_concurrent must be > 0"
        )

    async def test_progress_extract_preserves_parsing_error(self) -> None:
        """The progress adapter preserves the same parsing error as ordinary extraction."""
        operation = xberg.extract(_bad_pdf(), on_progress=lambda _event: None)
        await self._expect_typed_error(operation, xberg.ParsingError, "Parsing error: ")

    async def test_progress_batch_global_validation_preserves_validation_error(self) -> None:
        """The batch progress adapter preserves global configuration failures."""
        operation = xberg.extract_batch([_plain_input()], _invalid_chunking(), on_progress=lambda _event: None)
        await self._expect_typed_error(operation, xberg.ValidationError, "Validation error: ")

    async def test_sync_control_already_preserves_parsing_error(self) -> None:
        """The existing synchronous converter remains the typed control."""
        caught = None
        try:
            xberg.pdf_page_count(b"not a pdf")
        except Exception as error:
            caught = error
        assert caught is not None, "invalid PDF unexpectedly opened"
        assert str(caught).startswith("Parsing error: ")
        assert type(caught) is xberg.ParsingError
        assert isinstance(caught, xberg.XbergError)

    async def test_batch_per_input_errors_remain_structured(self) -> None:
        """One failed input remains in the result envelope beside a successful input."""
        result = await xberg.extract_batch([_bad_pdf(), _plain_input()], _plain_config())
        assert (result.summary.inputs, result.summary.results, result.summary.errors) == (2, 1, 1)
        assert (len(result.results), len(result.errors)) == (1, 1)
        assert result.results[0].content.strip() == PLAIN_TEXT
        error = result.errors[0]
        assert (error.index, error.code, error.error_type) == (0, PARSING_ERROR_CODE, "parsing")
        assert error.message.startswith("Parsing error: ")

    async def test_plain_extraction_remains_successful(self) -> None:
        """Successful plain extraction retains its content and result counts."""
        result = await xberg.extract(_plain_input(), _plain_config())
        assert (result.summary.inputs, result.summary.results, result.summary.errors) == (1, 1, 0)
        assert (len(result.results), len(result.errors)) == (1, 0)
        assert result.results[0].content.strip() == PLAIN_TEXT

    async def test_plain_progress_extraction_remains_successful(self) -> None:
        """Plain extraction with a progress callback succeeds without inventing OCR events."""
        events = []
        result = await xberg.extract(_plain_input(), _plain_config(), on_progress=events.append)
        assert (result.summary.inputs, result.summary.results, result.summary.errors) == (1, 1, 0)
        assert (len(result.results), len(result.errors)) == (1, 0)
        assert result.results[0].content.strip() == PLAIN_TEXT
        assert events == []


if __name__ == "__main__":
    unittest.main()
