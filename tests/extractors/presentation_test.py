from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


@pytest.fixture
def extractor() -> PresentationExtractor:
    return PresentationExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=DEFAULT_CONFIG
    )


@pytest.mark.anyio
async def test_extract_pptx_with_notes(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_notes_slide = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()

    mock_presentation.slides = [mock_slide]
    mock_slide.has_notes_slide = True
    mock_slide.notes_slide = mock_notes_slide
    mock_notes_slide.notes_text_frame = mock_text_frame
    mock_text_frame.text = "Test note content"
    mock_slide.shapes = []

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = await extractor.extract_bytes_async(b"mock pptx content")

    assert "Test note content" in result.content
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_path_async(mocker: MockerFixture, extractor: PresentationExtractor, tmp_path: Path) -> None:
    """Test async path extraction - covers lines 69-70."""

    mock_presentation = mocker.MagicMock()
    mock_presentation.slides = []
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    test_file = tmp_path / "test.pptx"
    test_file.write_bytes(b"mock pptx content")

    result = await extractor.extract_path_async(test_file)

    assert result.mime_type == "text/markdown"


def test_extract_bytes_sync(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test sync bytes extraction - covers line 82."""
    mock_presentation = mocker.MagicMock()
    mock_presentation.slides = []
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert result.mime_type == "text/markdown"


def test_extract_path_sync(mocker: MockerFixture, extractor: PresentationExtractor, tmp_path: Path) -> None:
    """Test sync path extraction - covers lines 94-95."""

    mock_presentation = mocker.MagicMock()
    mock_presentation.slides = []
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    test_file = tmp_path / "test.pptx"
    test_file.write_bytes(b"mock pptx content")

    result = extractor.extract_path_sync(test_file)

    assert result.mime_type == "text/markdown"


def test_extract_pptx_with_slide_title(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with slide title - covers line 131."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_title_shape = mocker.MagicMock()
    mock_shapes = mocker.MagicMock()

    mock_shapes.title = mock_title_shape
    mock_slide.shapes = mock_shapes
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mock_shapes.__iter__ = mocker.Mock(return_value=iter([mock_title_shape]))
    mock_title_shape.shape_type = mocker.MagicMock()
    mock_title_shape.has_text_frame = True
    mock_title_shape.text = "Test Title"

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "# Test Title" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_pptx_with_shapes_no_shape_type(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with shapes that don't have shape_type - covers lines 134-135."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_shape_no_type = mocker.MagicMock()

    del mock_shape_no_type.shape_type

    mock_slide.shapes = [mock_shape_no_type]
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert result.mime_type == "text/markdown"


def test_extract_pptx_with_picture_shape(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with picture shapes - covers lines 137-145."""
    from pptx.enum.shapes import MSO_SHAPE_TYPE

    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_picture_shape = mocker.MagicMock()

    mock_picture_shape.shape_type = MSO_SHAPE_TYPE.PICTURE
    mock_picture_shape.name = "test_image"

    mock_element = mocker.MagicMock()
    mock_nvXxPr = mocker.MagicMock()
    mock_cNvPr = mocker.MagicMock()
    mock_cNvPr.attrib = {"descr": "Test alt text"}
    mock_nvXxPr.cNvPr = mock_cNvPr
    mock_element._nvXxPr = mock_nvXxPr
    mock_picture_shape._element = mock_element

    mock_slide.shapes = [mock_picture_shape]
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "![Test alt text](test_image.jpg)" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_pptx_with_table_shape(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with table shapes - covers lines 147-162."""
    from pptx.enum.shapes import MSO_SHAPE_TYPE

    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_table_shape = mocker.MagicMock()

    mock_table_shape.shape_type = MSO_SHAPE_TYPE.TABLE

    mock_table = mocker.MagicMock()
    mock_row1 = mocker.MagicMock()
    mock_row2 = mocker.MagicMock()

    mock_cell1 = mocker.MagicMock()
    mock_cell1.text = "Header1"
    mock_cell2 = mocker.MagicMock()
    mock_cell2.text = "Header2"
    mock_cell3 = mocker.MagicMock()
    mock_cell3.text = "Data1"
    mock_cell4 = mocker.MagicMock()
    mock_cell4.text = "Data2"

    mock_row1.cells = [mock_cell1, mock_cell2]
    mock_row2.cells = [mock_cell3, mock_cell4]
    mock_table.rows = [mock_row1, mock_row2]
    mock_table_shape.table = mock_table

    mock_slide.shapes = [mock_table_shape]
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "<table>" in result.content
    assert "<th>Header1</th>" in result.content
    assert "<td>Data1</td>" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_pptx_with_text_frame_shape(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with text frame shapes - covers lines 164-165."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_text_shape = mocker.MagicMock()

    mock_text_shape.shape_type = mocker.MagicMock()
    mock_text_shape.has_text_frame = True
    mock_text_shape.text = "  Some text content"

    mock_slide.shapes = [mock_text_shape]
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "Some text content" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_presentation_metadata(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test metadata extraction - covers lines 210-231."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()

    mock_core_properties = mocker.MagicMock()
    mock_core_properties.author = "Test Author"
    mock_core_properties.title = "Test Title"
    mock_core_properties.subject = "Test Subject"
    mock_core_properties.language = "en-US"
    mock_core_properties.category = "Test Category"
    mock_core_properties.comments = None
    mock_core_properties.content_status = None
    mock_core_properties.created = None
    mock_core_properties.identifier = None
    mock_core_properties.keywords = None
    mock_core_properties.last_modified_by = None
    mock_core_properties.modified = None
    mock_core_properties.revision = None
    mock_core_properties.version = None

    mock_presentation.core_properties = mock_core_properties

    mock_shape = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()
    mock_paragraph = mocker.MagicMock()
    mock_run = mocker.MagicMock()
    mock_font = mocker.MagicMock()
    mock_font.name = "Arial"
    mock_run.font = mock_font
    mock_paragraph.runs = [mock_run]
    mock_text_frame.paragraphs = [mock_paragraph]
    mock_shape.text_frame = mock_text_frame

    mock_slide.shapes = [mock_shape]
    mock_presentation.slides = [mock_slide]

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert result.metadata["authors"] == "Test Author"  # type: ignore[comparison-overlap]
    assert result.metadata["title"] == "Test Title"
    assert result.metadata["subject"] == "Test Subject"
    assert result.metadata["languages"] == ["en-US"]
    assert result.metadata["categories"] == ["Test Category"]
    assert result.metadata["fonts"] == ["Arial"]
    assert result.mime_type == "text/markdown"


def test_extract_presentation_metadata_no_text_frame(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test metadata extraction with shapes that don't have text_frame - covers lines 222-223."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()

    mock_core_properties = mocker.MagicMock()
    mock_core_properties.author = None
    mock_core_properties.title = None
    mock_core_properties.language = None
    mock_core_properties.category = None
    mock_presentation.core_properties = mock_core_properties

    mock_shape = mocker.MagicMock()
    del mock_shape.text_frame

    mock_slide.shapes = [mock_shape]
    mock_presentation.slides = [mock_slide]

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "fonts" not in result.metadata
    assert result.mime_type == "text/markdown"


def test_extract_pptx_shape_without_text_frame(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    """Test extraction with shapes that don't have text_frame - covers branch 164->133."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_shape = mocker.MagicMock()

    mock_shape.shape_type = mocker.MagicMock()
    mock_shape.has_text_frame = False

    mock_slide.shapes = [mock_shape]
    mock_slide.has_notes_slide = False
    mock_presentation.slides = [mock_slide]
    mock_presentation.core_properties = mocker.MagicMock()
    mock_presentation.core_properties.author = None

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert result.mime_type == "text/markdown"


def test_extract_presentation_metadata_run_without_font(
    mocker: MockerFixture, extractor: PresentationExtractor
) -> None:
    """Test metadata extraction with runs that don't have font - covers branch 227->226."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()

    mock_core_properties = mocker.MagicMock()
    mock_core_properties.author = None
    mock_core_properties.language = None
    mock_core_properties.category = None
    mock_presentation.core_properties = mock_core_properties

    mock_shape = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()
    mock_paragraph = mocker.MagicMock()
    mock_run_no_font = mocker.MagicMock()

    del mock_run_no_font.font

    mock_paragraph.runs = [mock_run_no_font]
    mock_text_frame.paragraphs = [mock_paragraph]
    mock_shape.text_frame = mock_text_frame

    mock_slide.shapes = [mock_shape]
    mock_presentation.slides = [mock_slide]

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "fonts" not in result.metadata
    assert result.mime_type == "text/markdown"


def test_extract_presentation_metadata_font_without_name(
    mocker: MockerFixture, extractor: PresentationExtractor
) -> None:
    """Test metadata extraction with fonts that don't have name - covers branch 227->226."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()

    mock_core_properties = mocker.MagicMock()
    mock_core_properties.author = None
    mock_core_properties.language = None
    mock_core_properties.category = None
    mock_presentation.core_properties = mock_core_properties

    mock_shape = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()
    mock_paragraph = mocker.MagicMock()
    mock_run = mocker.MagicMock()
    mock_font = mocker.MagicMock()
    mock_font.name = None
    mock_run.font = mock_font
    mock_paragraph.runs = [mock_run]
    mock_text_frame.paragraphs = [mock_paragraph]
    mock_shape.text_frame = mock_text_frame

    mock_slide.shapes = [mock_shape]
    mock_presentation.slides = [mock_slide]

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = extractor.extract_bytes_sync(b"mock pptx content")

    assert "fonts" not in result.metadata
    assert result.mime_type == "text/markdown"
