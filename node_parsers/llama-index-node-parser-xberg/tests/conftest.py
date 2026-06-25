"""Shared test fixtures for XbergNodeParser tests."""

from typing import Any

from llama_index.core.schema import Document


def make_element(
    element_type: str = "narrative_text",
    text: str = "Some narrative text.",
    element_id: str = "el-001",
    page_number: int | None = 1,
    element_index: int | None = 0,
) -> dict[str, Any]:
    """Create a xberg Element dict matching the Element TypedDict shape."""
    return {
        "element_id": element_id,
        "element_type": element_type,
        "text": text,
        "metadata": {
            "page_number": page_number,
            "filename": "test.pdf",
            "coordinates": None,
            "element_index": element_index,
        },
    }


def make_xberg_document(
    elements: list[dict[str, Any]] | None = None,
    text: str = "Full document text.",
    doc_id: str = "doc-001",
) -> Document:
    """Create a Document with _xberg_elements metadata, matching XbergReader output."""
    if elements is None:
        elements = [
            make_element(element_type="title", text="Document Title", element_id="el-001", element_index=0),
            make_element(element_type="narrative_text", text="First paragraph.", element_id="el-002", element_index=1),
            make_element(
                element_type="table",
                text="| A | B |\n| 1 | 2 |",
                element_id="el-003",
                page_number=2,
                element_index=2,
            ),
        ]
    return Document(
        text=text,
        id_=doc_id,
        metadata={
            "_xberg_elements": elements,
            "file_path": "/tmp/test.pdf",
            "mime_type": "application/pdf",
        },
        excluded_llm_metadata_keys=["_xberg_elements"],
        excluded_embed_metadata_keys=["_xberg_elements"],
    )
