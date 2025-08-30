"""Unit tests for GMFT isolated process execution."""

from __future__ import annotations

import contextlib
import io
import multiprocessing as mp
import os
import queue
import signal
import sys
from dataclasses import asdict
from typing import TYPE_CHECKING, Any
from unittest.mock import MagicMock, patch

import pytest
from PIL import Image

from kreuzberg._gmft import (
    GMFTConfig,
    _extract_tables_in_process,
)
from kreuzberg._gmft import (
    _extract_tables_isolated as extract_tables_isolated,
)
from kreuzberg._gmft import (
    _extract_tables_isolated_async as extract_tables_isolated_async,
)
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from collections.abc import Generator
    from pathlib import Path


@pytest.fixture
def sample_pdf(tmp_path: Path) -> Path:
    """Create a simple valid PDF file."""
    pdf_file = tmp_path / "sample.pdf"

    pdf_content = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
217
%%EOF"""
    pdf_file.write_bytes(pdf_content)
    return pdf_file


@pytest.fixture
def mock_gmft_modules() -> Generator[None, None, None]:
    """Mock GMFT modules for testing."""
    with patch.dict(
        "sys.modules",
        {
            "gmft": MagicMock(),
            "gmft.auto": MagicMock(),
            "gmft.detectors": MagicMock(),
            "gmft.detectors.tatr": MagicMock(),
            "gmft.formatters": MagicMock(),
            "gmft.formatters.tatr": MagicMock(),
            "gmft.pdf_bindings": MagicMock(),
            "gmft.pdf_bindings.pdfium": MagicMock(),
        },
    ):
        yield


@pytest.mark.xfail(
    sys.version_info < (3, 11) and os.environ.get("CI") == "true",
    reason="Mock patching issues with multiprocessing on Python 3.10 in CI",
)
def test_extract_tables_in_process_success(sample_pdf: Path, mock_gmft_modules: None) -> None:
    """Test successful table extraction in isolated process."""
    config = GMFTConfig()
    config_dict = asdict(config).copy()
    result_queue: Any = mp.Queue()

    import pandas as pd

    mock_df = pd.DataFrame({"col1": [1, 2], "col2": [3, 4]})

    mock_page = MagicMock()
    mock_page.page_number = 1

    mock_cropped_table = MagicMock()
    mock_cropped_table.page = mock_page
    mock_image = Image.new("RGB", (100, 100), color="white")
    mock_cropped_table.image.return_value = mock_image

    mock_formatted_table = MagicMock()
    mock_formatted_table.df.return_value = mock_df

    mock_doc = MagicMock()
    mock_doc.__iter__.return_value = [mock_page]
    mock_doc.close = MagicMock()

    mock_detector = MagicMock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = MagicMock()
    mock_formatter.extract.return_value = mock_formatted_table

    with (
        patch("gmft.auto.AutoTableDetector", return_value=mock_detector),
        patch("gmft.auto.AutoTableFormatter", return_value=mock_formatter),
        patch("gmft.pdf_bindings.pdfium.PyPDFium2Document", return_value=mock_doc),
        patch("gmft.detectors.tatr.TATRDetectorConfig"),
        patch("gmft.formatters.tatr.TATRFormatConfig"),
    ):
        _extract_tables_in_process(str(sample_pdf), config_dict, result_queue)

        success, result = result_queue.get(timeout=1)
        assert success is True
        assert len(result) == 1
        assert result[0]["page_number"] == 1
        assert "col1" in result[0]["text"]
        assert isinstance(result[0]["cropped_image_bytes"], bytes)
        assert isinstance(result[0]["df_csv"], str)


def test_extract_tables_in_process_exception(sample_pdf: Path) -> None:
    """Test exception handling in isolated process."""
    config = GMFTConfig()
    config_dict = asdict(config).copy()
    result_queue: Any = mp.Queue()

    with patch("gmft.auto.AutoTableDetector", side_effect=ImportError("GMFT not installed")):
        _extract_tables_in_process(str(sample_pdf), config_dict, result_queue)

        success, error_info = result_queue.get(timeout=1)
        assert success is False
        assert error_info["type"] == "ImportError"
        assert "GMFT not installed" in error_info["error"]
        assert "traceback" in error_info


def test_extract_tables_isolated_timeout(sample_pdf: Path) -> None:
    """Test timeout handling in extract_tables_isolated."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="timed out"):
            extract_tables_isolated(str(sample_pdf), config, timeout=0.1)

        mock_process.terminate.assert_called_once()
        mock_process.join.assert_called()


def test_extract_tables_isolated_segfault(sample_pdf: Path) -> None:
    """Test segmentation fault handling."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, False, False]
        mock_process.exitcode = -signal.SIGSEGV
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="segmentation fault"):
            extract_tables_isolated(str(sample_pdf), config)

        assert mock_process.start.called


def test_extract_tables_isolated_unexpected_death(sample_pdf: Path) -> None:
    """Test handling of unexpected process death."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, False, False]
        mock_process.exitcode = -9
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="died unexpectedly with exit code -9"):
            extract_tables_isolated(str(sample_pdf), config)


def test_extract_tables_isolated_error_result(sample_pdf: Path) -> None:
    """Test handling of error result from subprocess."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        error_info = {"error": "Table extraction failed", "type": "RuntimeError", "traceback": "Traceback..."}
        mock_queue.get_nowait.return_value = (False, error_info)
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="Table extraction failed"):
            extract_tables_isolated(str(sample_pdf), config)


def test_extract_tables_isolated_success(sample_pdf: Path) -> None:
    """Test successful table extraction."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        import pandas as pd

        df = pd.DataFrame({"col1": [1, 2], "col2": [3, 4]})

        img = Image.new("RGB", (100, 100), color="white")
        img_buffer = io.BytesIO()
        img.save(img_buffer, format="PNG")
        img_bytes = img_buffer.getvalue()

        result = [
            {
                "cropped_image_bytes": img_bytes,
                "page_number": 1,
                "text": df.to_markdown(),
                "df_csv": df.to_csv(index=False),
            }
        ]

        mock_queue = MagicMock()
        mock_queue.get_nowait.return_value = (True, result)
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        tables = extract_tables_isolated(str(sample_pdf), config)

        assert len(tables) == 1
        assert isinstance(tables[0], dict)
        assert tables[0]["page_number"] == 1
        assert "col1" in tables[0]["text"]


def test_extract_tables_isolated_process_cleanup_timeout(sample_pdf: Path) -> None:
    """Test process cleanup when termination times out."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, True, True, True]
        mock_ctx.Process.return_value = mock_process

        with contextlib.suppress(ParsingError):
            extract_tables_isolated(str(sample_pdf), config, timeout=0.1)

        mock_process.terminate.assert_called_once()
        mock_process.kill.assert_called_once()
        assert mock_process.join.call_count == 2


@pytest.mark.anyio
async def test_extract_tables_isolated_async_success(sample_pdf: Path) -> None:
    """Test successful async table extraction."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        import pandas as pd

        df = pd.DataFrame({"col1": [1, 2], "col2": [3, 4]})

        img = Image.new("RGB", (100, 100), color="white")
        img_buffer = io.BytesIO()
        img.save(img_buffer, format="PNG")
        img_bytes = img_buffer.getvalue()

        result = [
            {
                "cropped_image_bytes": img_bytes,
                "page_number": 1,
                "text": df.to_markdown(),
                "df_csv": df.to_csv(index=False),
            }
        ]

        mock_queue = MagicMock()
        mock_queue.get_nowait.return_value = (True, result)
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        tables = await extract_tables_isolated_async(str(sample_pdf), config)

        assert len(tables) == 1
        assert isinstance(tables[0], dict)
        assert tables[0]["page_number"] == 1


@pytest.mark.anyio
async def test_extract_tables_isolated_async_timeout(sample_pdf: Path) -> None:
    """Test async timeout handling."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="timed out"):
            await extract_tables_isolated_async(str(sample_pdf), config, timeout=0.1)


@pytest.mark.anyio
async def test_extract_tables_isolated_async_segfault(sample_pdf: Path) -> None:
    """Test async segmentation fault handling."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, False, False, False]
        mock_process.exitcode = -signal.SIGSEGV
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="segmentation fault"):
            await extract_tables_isolated_async(str(sample_pdf), config)


@pytest.mark.anyio
async def test_extract_tables_isolated_async_unexpected_death(sample_pdf: Path) -> None:
    """Test async handling of unexpected process death."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, False, False, False]
        mock_process.exitcode = -15
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="died unexpectedly with exit code -15"):
            await extract_tables_isolated_async(str(sample_pdf), config)


@pytest.mark.anyio
async def test_extract_tables_isolated_async_error_result(sample_pdf: Path) -> None:
    """Test async handling of error result from subprocess."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        error_info = {"error": "Async table extraction failed", "type": "ValueError", "traceback": "Traceback..."}
        mock_queue.get_nowait.return_value = (False, error_info)
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.return_value = True
        mock_ctx.Process.return_value = mock_process

        with pytest.raises(ParsingError, match="Async table extraction failed"):
            await extract_tables_isolated_async(str(sample_pdf), config)


@pytest.mark.anyio
async def test_extract_tables_isolated_async_process_cleanup(sample_pdf: Path) -> None:
    """Test async process cleanup when termination times out."""
    config = GMFTConfig()

    with patch("multiprocessing.get_context") as mock_get_context:
        mock_ctx = MagicMock()
        mock_get_context.return_value = mock_ctx

        mock_queue = MagicMock()
        mock_queue.get_nowait.side_effect = queue.Empty
        mock_ctx.Queue.return_value = mock_queue

        mock_process = MagicMock()
        mock_process.is_alive.side_effect = [True, True, True, True]
        mock_ctx.Process.return_value = mock_process

        with contextlib.suppress(ParsingError):
            await extract_tables_isolated_async(str(sample_pdf), config, timeout=0.1)

        mock_process.terminate.assert_called_once()
        mock_process.kill.assert_called_once()


def test_signal_handling() -> None:
    """Test signal handling in process."""
    with patch("signal.signal") as mock_signal:
        config = GMFTConfig()
        config_dict = asdict(config).copy()
        result_queue: Any = mp.Queue()

        with patch("gmft.auto.AutoTableDetector", side_effect=ImportError("Test")):
            _extract_tables_in_process("dummy.pdf", config_dict, result_queue)

        mock_signal.assert_called_once_with(signal.SIGINT, signal.SIG_IGN)
