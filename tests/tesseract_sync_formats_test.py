from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig, TesseractConfig, extract_file_sync
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import OutputFormatType
from kreuzberg._utils._cache import clear_all_caches


class TestTesseractSyncFormats:
    @pytest.fixture(autouse=True)
    def setup_method(self) -> None:
        clear_all_caches()

    @pytest.fixture
    def test_image_path(self) -> Path:
        return Path("tests/test_source_files/ocr-image.jpg")

    @pytest.fixture
    def tesseract_backend(self) -> TesseractBackend:
        return TesseractBackend()

    def test_text_format_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="text", language="eng")
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/plain"
        assert len(result.content) > 0
        assert isinstance(result.content, str)
        assert "<" not in result.content
        assert "<?xml" not in result.content

    def test_hocr_format_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="hocr", language="eng")
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/html"
        assert len(result.content) > 1000
        assert result.content.startswith('<?xml version="1.0"')
        assert "ocrx_word" in result.content
        assert "DOCTYPE html" in result.content

    def test_markdown_format_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="markdown", language="eng")
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/markdown"
        assert len(result.content) > 0
        assert "---" in result.content
        assert "<?xml" not in result.content
        assert 'class="ocrx_word"' not in result.content

    def test_tsv_format_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="tsv", language="eng")
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/plain"
        assert len(result.content) > 0
        assert "\t" not in result.content
        assert isinstance(result.content, str)

    def test_tsv_with_table_detection_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="tsv", enable_table_detection=True, language="eng")
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/plain"
        assert len(result.content) > 0
        assert isinstance(result.content, str)

    def test_format_differences_sync(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        formats: list[OutputFormatType] = ["text", "hocr", "markdown", "tsv"]
        results = {}

        for fmt in formats:
            config = TesseractConfig(output_format=fmt, language="eng")
            result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())
            results[fmt] = result

        assert len(results["hocr"].content) > len(results["text"].content) * 10
        assert results["hocr"].content != results["text"].content
        assert results["hocr"].mime_type != results["text"].mime_type

        assert results["markdown"].content != results["text"].content
        assert results["markdown"].mime_type == "text/markdown"

        base_text = "Nasdaq"
        for result in results.values():
            assert base_text in result.content or base_text.lower() in result.content.lower()

    def test_sync_format_via_extract_file_sync(self, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        clear_all_caches()

        text_config = ExtractionConfig(
            force_ocr=True, ocr_backend="tesseract", ocr_config=TesseractConfig(output_format="text", language="eng")
        )
        text_result = extract_file_sync(test_image_path, config=text_config)
        assert text_result.mime_type == "text/plain"
        assert len(text_result.content) > 0

        markdown_config = ExtractionConfig(
            force_ocr=True,
            ocr_backend="tesseract",
            ocr_config=TesseractConfig(output_format="markdown", language="eng"),
        )
        markdown_result = extract_file_sync(test_image_path, config=markdown_config)
        assert len(markdown_result.content) > 0

    def test_auto_switch_text_to_tsv_with_table_detection(
        self, tesseract_backend: TesseractBackend, test_image_path: Path
    ) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="text", enable_table_detection=True, language="eng")

        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/plain"
        assert len(result.content) > 0

    @pytest.mark.parametrize("output_format", ["text", "hocr", "markdown", "tsv"])
    def test_cache_isolation_by_format(
        self, tesseract_backend: TesseractBackend, test_image_path: Path, output_format: OutputFormatType
    ) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format=output_format, language="eng")

        clear_all_caches()
        result1 = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        result2 = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result1.content == result2.content
        assert result1.mime_type == result2.mime_type

    def test_error_handling_invalid_format(self, tesseract_backend: TesseractBackend, test_image_path: Path) -> None:
        if not test_image_path.exists():
            pytest.skip("Test image not found")

        config = TesseractConfig(output_format="invalid_format", language="eng")  # type: ignore[arg-type]
        result = tesseract_backend.process_file_sync(test_image_path, **config.to_dict())

        assert result.mime_type == "text/plain"
        assert len(result.content) > 0
        assert "<" not in result.content
