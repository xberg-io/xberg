"""Xberg-backed document extraction pipeline."""

from typing import TypedDict

from xberg import ExtractionConfig, extract_file_sync


class DocumentMetadata(TypedDict):
    """Metadata extracted from a single document."""

    source: str
    mime_type: str
    title: str | None
    page_count: int | None


class ExtractionDocument(TypedDict):
    """A single document extraction result."""

    content: str
    metadata: DocumentMetadata


class XbergPipeline:
    """Xberg-backed document extraction pipeline.

    A plain callable class that accepts one or more document paths and
    returns structured extraction results suitable for any downstream
    pipeline framework — txtai workflows, LangChain loaders, or direct
    use with embeddings indices.
    """

    def __init__(self, config: ExtractionConfig | None = None) -> None:
        """Initialize the pipeline.

        Args:
            config: A Xberg ``ExtractionConfig``. Pass one to control
                output format, OCR backend and language, ``force_ocr``, and
                every other Xberg knob — they are all fields on
                ``ExtractionConfig`` (OCR settings live on the nested
                ``OcrConfig``). When omitted, Xberg's defaults apply.

                Example::

                    from xberg import ExtractionConfig, OcrConfig

                    config = ExtractionConfig(
                        output_format="markdown",
                        ocr=OcrConfig(backend="tesseract", language="eng"),
                        force_ocr=True,
                    )
                    pipeline = XbergPipeline(config=config)

        """
        self._config = config

    def __call__(self, documents: str | list[str]) -> list[ExtractionDocument]:
        """Extract text and metadata from one or more documents.

        Args:
            documents: A single file path, or a list of file paths.

        Returns:
            A list of :class:`ExtractionDocument` dicts. The list has one
            element per input path, in input order. A single-string input
            still returns a one-element list.

        """
        paths = [documents] if isinstance(documents, str) else list(documents)
        return [self._extract_one(path) for path in paths]

    def _extract_one(self, path: str) -> ExtractionDocument:
        result = extract_file_sync(path, config=self._config)
        metadata = result.metadata or {}
        return ExtractionDocument(
            content=result.content,
            metadata=DocumentMetadata(
                source=path,
                mime_type=result.mime_type,
                title=metadata.get("title"),
                page_count=metadata.get("page_count"),
            ),
        )
