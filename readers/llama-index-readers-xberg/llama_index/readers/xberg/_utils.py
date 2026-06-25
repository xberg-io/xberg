"""Utility functions for XbergReader metadata and document construction."""

import base64
import hashlib
from pathlib import Path
from typing import Any

from llama_index.readers.xberg._types import (
    Annotation,
    DocumentMetadata,
    Keyword,
    ProcessingWarning,
)

from xberg import ExtractedImage, ExtractedTable, ExtractionResult


def serialize_images(images: list[ExtractedImage], page_number: int | None = None) -> list[dict[str, Any]]:
    """Serialize image objects to JSON-safe dicts, filtering by page when given."""
    serialized = []
    for img in images:
        if page_number is not None and img.get("page_number") != page_number:
            continue
        raw_data = img.get("data")
        entry: dict[str, Any] = {
            "format": img.get("format"),
            "image_index": img.get("image_index"),
            "page_number": img.get("page_number"),
            "width": img.get("width"),
            "height": img.get("height"),
            "colorspace": img.get("colorspace"),
            "bits_per_component": img.get("bits_per_component"),
            "is_mask": img.get("is_mask"),
            "description": img.get("description"),
            "data": base64.b64encode(raw_data).decode("ascii") if raw_data is not None else None,
        }
        bbox = img.get("bounding_box")
        if bbox is not None:
            entry["bounding_box"] = bbox
        ocr_result = img.get("ocr_result")
        if ocr_result is not None:
            entry["ocr_result"] = ocr_result.content if hasattr(ocr_result, "content") else str(ocr_result)
        serialized.append(entry)
    return serialized


def build_metadata(  # noqa: C901
    result: ExtractionResult,
    file_path: Path | None = None,
    source: str | None = None,
    extra_info: dict[str, Any] | None = None,
    page_number: int | None = None,
) -> DocumentMetadata:
    """Flatten ExtractionResult into a metadata dict."""
    meta: DocumentMetadata = {}  # type: ignore[assignment]

    if file_path is not None:
        meta["file_name"] = file_path.name
        meta["file_path"] = str(file_path)
    elif source is not None:
        meta["file_name"] = source
        meta["file_path"] = source

    meta["file_type"] = result.mime_type
    meta["total_pages"] = result.get_page_count()

    if page_number is not None:
        meta["page_number"] = page_number

    xberg_meta = result.metadata
    if isinstance(xberg_meta, dict):
        meta.update({k: v for k, v in xberg_meta.items() if v is not None})

    if result.quality_score is not None:
        meta["quality_score"] = result.quality_score
    if result.detected_languages is not None:
        meta["detected_languages"] = result.detected_languages
    meta["output_format"] = result.output_format
    if result.processing_warnings:
        meta["processing_warnings"] = [
            ProcessingWarning(source=w.source, message=w.message)
            for w in result.processing_warnings
            if hasattr(w, "source")
        ]
    if result.extracted_keywords:
        meta["extracted_keywords"] = [
            Keyword(text=kw.text, score=kw.score, algorithm=kw.algorithm)
            for kw in result.extracted_keywords
            if hasattr(kw, "text")
        ]
    if result.annotations:
        meta["annotations"] = [
            Annotation(
                annotation_type=a.annotation_type,
                content=a.content,
                page_number=a.page_number,
            )
            for a in result.annotations
            if hasattr(a, "annotation_type")
        ]
    if result.elements is not None:
        meta["_xberg_elements"] = result.elements
    if result.images:
        meta["images"] = serialize_images(result.images, page_number=page_number)

    if extra_info:
        meta.update(extra_info)

    return meta


def generate_doc_id(
    *,
    file_path: Path | None = None,
    data: bytes | None = None,
    page_number: int | None = None,
) -> str:
    """Generate a deterministic document ID via SHA-256."""
    if file_path is None and data is None:
        msg = "Either file_path or data must be provided"
        raise ValueError(msg)
    hasher = hashlib.sha256()
    if file_path is not None:
        hasher.update(str(file_path.resolve()).encode())
    elif data is not None:
        hasher.update(data)
    if page_number is not None:
        hasher.update(str(page_number).encode())
    return hasher.hexdigest()


def excluded_keys(meta: DocumentMetadata) -> list[str]:
    """Return metadata keys that should be excluded from LLM/embedding input."""
    keys: list[str] = []
    if "_xberg_elements" in meta:
        keys.append("_xberg_elements")
    if "images" in meta:
        keys.append("images")
    return keys


def append_tables(content: str, tables: list[ExtractedTable]) -> str:
    """Append table markdown to content when tables are not already included."""
    if not tables:
        return content
    for table in tables:
        table_md = table.markdown
        if table_md and table_md.strip() not in content:
            content = content.rstrip() + "\n\n" + table_md
    return content
