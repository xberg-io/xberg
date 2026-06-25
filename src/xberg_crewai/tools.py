"""Xberg document extraction tools for CrewAI agents."""

from typing import Literal

from crewai.tools import BaseTool
from xberg import ExtractionConfig, extract_file_sync
from pydantic import BaseModel, Field


class ExtractInput(BaseModel):
    """Input schema for XbergExtractTool."""

    file_path: str = Field(..., description="Path to the document file to extract text from")
    output_format: Literal["plain", "markdown", "html"] = Field(
        default="markdown",
        description="Output format: plain, markdown, or html",
    )


class MetadataInput(BaseModel):
    """Input schema for XbergExtractMetadataTool."""

    file_path: str = Field(..., description="Path to the document file to extract metadata from")


class XbergExtractTool(BaseTool):
    """Extract text content from a document file.

    Supports 88+ formats including PDF, DOCX, XLSX, HTML, images with OCR, and more.
    The agent chooses the output format (plain, markdown, or html) per extraction.
    """

    name: str = "Extract Document"
    description: str = (
        "Extract text content from a document file. Supports 88+ formats "
        "including PDF, DOCX, XLSX, HTML, images with OCR, and more. "
        "Returns the extracted text content."
    )
    args_schema: type[BaseModel] = ExtractInput

    def _run(self, file_path: str, output_format: Literal["plain", "markdown", "html"] = "markdown") -> str:
        config = ExtractionConfig(output_format=output_format)
        result = extract_file_sync(file_path, config=config)
        content: str = result.content
        return content


class XbergExtractMetadataTool(BaseTool):
    """Extract metadata from a document file.

    Returns metadata such as title, authors, dates, page count, and format-specific
    details as a formatted string. Supports 88+ formats.
    """

    name: str = "Extract Document Metadata"
    description: str = (
        "Extract metadata from a document file such as title, authors, "
        "dates, page count, and format-specific details. "
        "Supports 88+ formats including PDF, DOCX, XLSX, HTML, images, and more."
    )
    args_schema: type[BaseModel] = MetadataInput

    def _run(self, file_path: str) -> str:
        result = extract_file_sync(file_path)
        metadata = result.metadata
        lines = [f"{key}: {value}" for key, value in metadata.items() if value is not None]
        return "\n".join(lines) if lines else "No metadata found."
