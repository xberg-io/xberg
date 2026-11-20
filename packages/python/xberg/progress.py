"""Python progress callbacks for document extraction."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import TYPE_CHECKING

import xberg._xberg as _rust

from . import api as _api

if TYPE_CHECKING:
    from .options import ExtractInput, ExtractionConfig


@dataclass(frozen=True, slots=True)
class ProgressEvent:
    """The first completed OCR read of one page, reported once per page of an input.

    Attributes:
        stage: Always ``"ocr_page"``.
        page: The 1-based page number.
        total: The input's page count.
        completed: The number of distinct pages of the input reported so far, this one included.
        backend: The backend of the page's first OCR read. A later quality fallback may
            replace that read in the extraction result without a second event.
        input_index: The zero-based position in ``extract_batch``; ``None`` for ``extract``.
    """

    stage: str
    page: int
    total: int
    completed: int
    backend: str
    input_index: int | None = None


ProgressCallback = Callable[[ProgressEvent], None]


class _ProgressListener:
    def __init__(self, callback: ProgressCallback) -> None:
        self._callback = callback

    def on_progress(  # noqa: PLR0917
        self,
        stage: str,
        page: int | None,
        total: int | None,
        completed: int | None,
        backend: str | None,
        input_index: int | None,
    ) -> None:
        if stage != "ocr_page" or page is None or total is None or completed is None or backend is None:
            return
        self._callback(ProgressEvent(stage, page, total, completed, backend, input_index))


async def extract(
    input: ExtractInput | None = None,  # noqa: A002
    config: ExtractionConfig | None = None,
    on_progress: ProgressCallback | None = None,
) -> _api.ExtractionResult:
    """Extract one input, optionally reporting each OCR page once, when its first read completes."""
    if on_progress is None:
        return await _api.extract(input=input, config=config)
    rust_input = _api._to_rust_extract_input(input) if input is not None else _rust.ExtractInput()
    rust_config = _api._to_rust_extraction_config(config) if config is not None else _rust.ExtractionConfig()
    native_extract = _rust.__dict__["extract_with_progress"]
    return await native_extract(
        input=rust_input,
        config=rust_config,
        on_progress=_ProgressListener(on_progress),
    )


async def extract_batch(
    inputs: list[ExtractInput],
    config: ExtractionConfig | None = None,
    on_progress: ProgressCallback | None = None,
) -> _api.ExtractionResult:
    """Extract multiple inputs, optionally reporting each OCR page once, when its first read completes."""
    if on_progress is None:
        return await _api.extract_batch(inputs=inputs, config=config)
    rust_inputs = [_api._to_rust_extract_input(item) for item in inputs]
    rust_config = _api._to_rust_extraction_config(config) if config is not None else _rust.ExtractionConfig()
    native_extract_batch = _rust.__dict__["extract_batch_with_progress"]
    return await native_extract_batch(
        inputs=rust_inputs,
        config=rust_config,
        on_progress=_ProgressListener(on_progress),
    )
