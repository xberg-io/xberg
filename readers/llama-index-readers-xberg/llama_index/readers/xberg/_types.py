"""Internal type definitions for XbergReader metadata."""

from typing import Any, TypedDict

from xberg import ExtractedImage, ExtractedTable, Metadata


class ProcessingWarning(TypedDict):
    source: str
    message: str


class Keyword(TypedDict):
    text: str
    score: float
    algorithm: str


class Annotation(TypedDict):
    annotation_type: str
    content: str | None
    page_number: int


class PageContent(TypedDict):
    page_number: int
    content: str
    tables: list[ExtractedTable]
    images: list[ExtractedImage]
    is_blank: bool


class DocumentMetadata(Metadata, total=False):  # type: ignore[misc,call-arg]
    file_name: str
    file_path: str
    file_type: str
    total_pages: int
    page_number: int
    quality_score: float | None
    detected_languages: list[str] | None
    output_format: str
    processing_warnings: list[ProcessingWarning] | None
    extracted_keywords: list[Keyword] | None
    annotations: list[Annotation] | None
    images: list[dict[str, Any]] | None
    _xberg_elements: list[Any] | None
