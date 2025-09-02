from __future__ import annotations

import builtins
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pandas as pd
import pytest

from kreuzberg._document_classification import (
    DOCUMENT_CLASSIFIERS,
    auto_detect_document_type,
    classify_document,
    classify_document_from_layout,
)
from kreuzberg._types import ExtractionConfig, ExtractionResult
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.mark.anyio
@pytest.mark.parametrize(
    ("doc_type", "file_name"),
    [
        ("invoice", "invoice_test.txt"),
        ("receipt", "receipt_test.txt"),
        ("contract", "contract_test.txt"),
        ("report", "report_test.txt"),
        ("form", "form_test.txt"),
    ],
)
async def test_extract_file_with_all_document_types(doc_type: str, file_name: str, test_files_path: Path) -> None:
    from kreuzberg import extract_file

    test_file = test_files_path / file_name
    config = ExtractionConfig(auto_detect_document_type=True)
    result = await extract_file(test_file, config=config)
    assert result.document_type == doc_type
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


def test_classify_document_low_confidence() -> None:
    result = ExtractionResult(
        content="Some random text that doesn't match any patterns",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(document_type_confidence_threshold=0.9)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_high_confidence() -> None:
    result = ExtractionResult(
        content="INVOICE #12345 Total: $100.00 Due Date: 01/01/2024",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.1)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.1


def test_document_classifiers_available() -> None:
    expected_types = {"invoice", "receipt", "contract", "report", "form"}
    assert set(DOCUMENT_CLASSIFIERS.keys()) == expected_types

    for pattern_list in DOCUMENT_CLASSIFIERS.values():
        assert isinstance(pattern_list, list)
        assert len(pattern_list) > 0


def test_document_classifiers_patterns() -> None:
    for patterns in DOCUMENT_CLASSIFIERS.values():
        assert isinstance(patterns, list)
        assert len(patterns) > 0

        for pattern in patterns:
            assert isinstance(pattern, str)
            assert len(pattern) > 0


def test_document_classifiers_keywords() -> None:
    invoice_patterns = DOCUMENT_CLASSIFIERS["invoice"]
    assert any("invoice" in pattern.lower() for pattern in invoice_patterns)

    receipt_patterns = DOCUMENT_CLASSIFIERS["receipt"]
    assert any("receipt" in pattern.lower() for pattern in receipt_patterns)


def test_classify_document_with_metadata() -> None:
    result = ExtractionResult(
        content="Regular content",
        mime_type="text/plain",
        metadata={"title": "Invoice #12345", "subject": "Payment Due"},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    assert confidence is not None


def test_classify_document_disabled() -> None:
    result = ExtractionResult(
        content="INVOICE #12345 Total: $100.00",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=False)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_empty_content() -> None:
    result = ExtractionResult(
        content="",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_with_exclusions() -> None:
    result = ExtractionResult(
        content="CONTRACT AGREEMENT INVOICE #12345 Total: $100.00",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "contract"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_from_layout_basic() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["INVOICE", "#12345", "Total:", "$100.00"],
            "top": [10, 30, 100, 120],
            "height": [20, 15, 15, 15],
        }
    )

    result = ExtractionResult(
        content="INVOICE #12345 Total: $100.00",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_from_layout_no_layout() -> None:
    result = ExtractionResult(
        content="INVOICE #12345",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_empty_layout() -> None:
    layout_df = pd.DataFrame()

    result = ExtractionResult(
        content="INVOICE #12345",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_missing_columns() -> None:
    layout_df = pd.DataFrame({"text": ["Test"], "missing_columns": [1]})

    result = ExtractionResult(
        content="Test content",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_no_pattern_matches() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["Generic text", "No patterns here", "Just regular content"],
            "top": [10, 50, 100],
            "height": [20, 20, 20],
        }
    )

    result = ExtractionResult(
        content="Generic text No patterns here Just regular content",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_header_patterns() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["INVOICE", "Company Name", "Item description", "Total: $100"],
            "top": [10, 40, 200, 250],
            "height": [30, 20, 15, 15],
        }
    )

    result = ExtractionResult(
        content="INVOICE Company Name Item description Total: $100",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.6


def test_classify_document_from_layout_position_scoring() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["receipt", "store info", "items", "total"],
            "top": [5, 30, 200, 300],
            "height": [15, 15, 15, 15],
        }
    )

    result = ExtractionResult(
        content="receipt store info items total",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "receipt"
    assert confidence is not None


def test_auto_detect_document_type_from_content() -> None:
    result = ExtractionResult(
        content="INVOICE #12345 Amount Due: $500.00 Payment Terms: Net 30",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    detection_result = auto_detect_document_type(result, config)

    assert detection_result.document_type == "invoice"
    assert detection_result.document_type_confidence is not None
    assert detection_result.document_type_confidence >= 0.5


def test_auto_detect_document_type_from_layout() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["RECEIPT", "Store: ABC Shop", "Item: Coffee", "Total: $5.00"],
            "top": [10, 30, 100, 120],
            "height": [25, 15, 15, 15],
        }
    )

    result = ExtractionResult(
        content="Generic text without strong patterns",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    detection_result = auto_detect_document_type(result, config)

    assert detection_result.document_type == "receipt"
    assert detection_result.document_type_confidence is not None


def test_auto_detect_document_type_disabled() -> None:
    result = ExtractionResult(
        content="INVOICE #12345",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=False)

    detection_result = auto_detect_document_type(result, config)

    assert detection_result.document_type is None
    assert detection_result.document_type_confidence is None


def test_auto_detect_document_type_no_matches() -> None:
    result = ExtractionResult(
        content="Random text with no document indicators",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    detection_result = auto_detect_document_type(result, config)

    assert detection_result.document_type is None
    assert detection_result.document_type_confidence is None


def test_auto_detect_document_type_confidence_threshold() -> None:
    result = ExtractionResult(
        content="Maybe invoice payment receipt unclear document",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(document_type_confidence_threshold=0.9)

    detection_result = auto_detect_document_type(result, config)

    assert detection_result.document_type is None
    assert detection_result.document_type_confidence is None


@pytest.mark.anyio
async def test_document_classification_integration_invoice(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    invoice_file = test_files_path / "invoice_test.txt"
    config = ExtractionConfig(auto_detect_document_type=True)

    result = await extract_file(invoice_file, config=config)

    assert result.document_type == "invoice"
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


@pytest.mark.anyio
async def test_document_classification_integration_receipt(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    receipt_file = test_files_path / "receipt_test.txt"
    config = ExtractionConfig(auto_detect_document_type=True)

    result = await extract_file(receipt_file, config=config)

    assert result.document_type == "receipt"
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


@pytest.mark.anyio
async def test_document_classification_integration_contract(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    contract_file = test_files_path / "contract_test.txt"
    config = ExtractionConfig(auto_detect_document_type=True)

    result = await extract_file(contract_file, config=config)

    assert result.document_type == "contract"
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


@pytest.mark.anyio
async def test_document_classification_integration_report(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    report_file = test_files_path / "report_test.txt"
    config = ExtractionConfig(auto_detect_document_type=True)

    result = await extract_file(report_file, config=config)

    assert result.document_type == "report"
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


@pytest.mark.anyio
async def test_document_classification_integration_form(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    form_file = test_files_path / "form_test.txt"
    config = ExtractionConfig(auto_detect_document_type=True)

    result = await extract_file(form_file, config=config)

    assert result.document_type == "form"
    assert result.document_type_confidence is not None
    assert result.document_type_confidence > 0.5


@pytest.mark.anyio
async def test_document_classification_integration_disabled(test_files_path: Path) -> None:
    from kreuzberg import extract_file

    invoice_file = test_files_path / "invoice_test.txt"
    config = ExtractionConfig(auto_detect_document_type=False)

    result = await extract_file(invoice_file, config=config)

    assert result.document_type is None
    assert result.document_type_confidence is None


def test_classify_document_invoice(mocker: MockerFixture) -> None:
    mock_translate = mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="this is an invoice with invoice number 12345 and total amount $100",
    )

    result = ExtractionResult(
        content="This is an invoice with invoice number 12345 and total amount $100",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.3
    mock_translate.assert_called_once_with(result)


def test_classify_document_receipt(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="cash receipt subtotal $50 total due $55",
    )

    result = ExtractionResult(content="Cash receipt subtotal $50 total due $55", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "receipt"
    assert confidence is not None
    assert confidence > 0.3


def test_classify_document_contract(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="this agreement between party a and party b includes terms and conditions",
    )

    result = ExtractionResult(
        content="This agreement between party A and party B includes terms and conditions",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "contract"
    assert confidence is not None
    assert confidence > 0.3


def test_classify_document_no_match(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="this is just some random text with no specific document indicators",
    )

    result = ExtractionResult(
        content="This is just some random text with no specific document indicators",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_comprehensive_low_confidence_detailed(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="random text without clear patterns",
    )

    result = ExtractionResult(content="random text", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.8)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_comprehensive_with_metadata_detailed(mocker: MockerFixture) -> None:
    mock_translate = mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="invoice document title invoice number 12345",
    )

    result = ExtractionResult(content="Document content", mime_type="text/plain", metadata={"title": "Invoice"})
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    mock_translate.assert_called_once_with(result)


def test_classify_document_from_layout_disabled() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=False)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_comprehensive_no_layout_detailed() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=None)
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_comprehensive_empty_layout_detailed() -> None:
    empty_df = pd.DataFrame()
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=empty_df)
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_comprehensive_missing_columns_detailed() -> None:
    layout_df = pd.DataFrame({"text": ["some text"], "left": [10]})
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=layout_df)
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_invoice(mocker: MockerFixture) -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["INVOICE", "Invoice Number: 12345", "Total Amount: $500"],
            "top": [10, 50, 100],
            "height": [20, 15, 15],
        }
    )

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=layout_df)
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    mock_translator = mocker.Mock()
    mock_translator.translate.return_value = "invoice invoice number 12345 total amount $500"
    mocker.patch("deep_translator.GoogleTranslator", return_value=mock_translator)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.3


def test_classify_document_from_layout_translation_error(mocker: MockerFixture) -> None:
    layout_df = pd.DataFrame({"text": ["INVOICE", "Total Amount: $500"], "top": [10, 50], "height": [20, 15]})

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=layout_df)
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    mocker.patch("deep_translator.GoogleTranslator", side_effect=Exception("Translation failed"))

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.3


def test_classify_document_from_layout_header_bonus() -> None:
    layout_df = pd.DataFrame(
        {
            "text": ["INVOICE"],
            "top": [5],
            "height": [20],
        }
    )

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, layout=layout_df)
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.3)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None


def test_get_translated_text_basic(mocker: MockerFixture) -> None:
    from kreuzberg._document_classification import _get_translated_text

    mock_translator = mocker.Mock()
    mock_translator.translate.return_value = "This is translated text"
    mocker.patch("deep_translator.GoogleTranslator", return_value=mock_translator)

    result = ExtractionResult(content="Original text", mime_type="text/plain", metadata={})

    translated = _get_translated_text(result)

    assert translated == "this is translated text"
    mock_translator.translate.assert_called_once_with("Original text")


def test_get_translated_text_with_metadata(mocker: MockerFixture) -> None:
    from kreuzberg._document_classification import _get_translated_text

    mock_translator = mocker.Mock()
    mock_translator.translate.return_value = "Translated content with metadata"
    mocker.patch("deep_translator.GoogleTranslator", return_value=mock_translator)

    result = ExtractionResult(content="Original content", mime_type="text/plain", metadata={"title": "Document Title"})

    translated = _get_translated_text(result)

    assert translated == "translated content with metadata"
    expected_call = "Original content Document Title"
    mock_translator.translate.assert_called_once_with(expected_call)


def test_get_translated_text_translation_error(mocker: MockerFixture) -> None:
    from kreuzberg._document_classification import _get_translated_text

    mocker.patch("deep_translator.GoogleTranslator", side_effect=Exception("Translation API error"))

    result = ExtractionResult(content="Original Text", mime_type="text/plain", metadata={})

    translated = _get_translated_text(result)

    assert translated == "original text"


def test_auto_detect_document_type_text_mode(mocker: MockerFixture) -> None:
    mock_classify = mocker.patch("kreuzberg._document_classification.classify_document", return_value=("invoice", 0.8))

    result = ExtractionResult(content="Invoice content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_classification_mode="text")

    updated_result = auto_detect_document_type(result, config)

    assert updated_result.document_type == "invoice"
    assert updated_result.document_type_confidence == 0.8
    mock_classify.assert_called_once_with(result, config)


def test_auto_detect_document_type_vision_mode_with_file(mocker: MockerFixture) -> None:
    mock_ocr_result = ExtractionResult(
        content="OCR extracted text",
        mime_type="text/plain",
        metadata={},
        layout=pd.DataFrame({"text": ["INVOICE", "Total: $100"], "top": [10, 50], "height": [20, 15]}),
    )

    mock_ocr_backend = mocker.Mock()
    mock_ocr_backend.process_file_sync.return_value = mock_ocr_result
    mocker.patch("kreuzberg._document_classification.get_ocr_backend", return_value=mock_ocr_backend)

    mock_classify_layout = mocker.patch(
        "kreuzberg._document_classification.classify_document_from_layout", return_value=("invoice", 0.9)
    )

    result = ExtractionResult(content="Original content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_classification_mode="vision")
    file_path = Path("/path/to/test.pdf")

    updated_result = auto_detect_document_type(result, config, file_path)

    assert updated_result.document_type == "invoice"
    assert updated_result.document_type_confidence == 0.9
    mock_classify_layout.assert_called_once_with(mock_ocr_result, config)


def test_auto_detect_document_type_vision_mode_no_file(mocker: MockerFixture) -> None:
    mock_classify = mocker.patch("kreuzberg._document_classification.classify_document", return_value=("report", 0.7))

    result = ExtractionResult(content="Report content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_classification_mode="vision")

    updated_result = auto_detect_document_type(result, config, file_path=None)

    assert updated_result.document_type == "report"
    assert updated_result.document_type_confidence == 0.7
    mock_classify.assert_called_once_with(result, config)


def test_auto_detect_document_type_with_existing_layout(mocker: MockerFixture) -> None:
    layout_df = pd.DataFrame({"text": ["CONTRACT", "Agreement between parties"], "top": [10, 50], "height": [20, 15]})

    result = ExtractionResult(content="Contract content", mime_type="text/plain", metadata={}, layout=layout_df)

    mock_classify_layout = mocker.patch(
        "kreuzberg._document_classification.classify_document_from_layout", return_value=("contract", 0.85)
    )

    config = ExtractionConfig(auto_detect_document_type=True)

    updated_result = auto_detect_document_type(result, config)

    assert updated_result.document_type == "contract"
    assert updated_result.document_type_confidence == 0.85
    mock_classify_layout.assert_called_once_with(result, config)


def test_auto_detect_document_type_mixed_patterns(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="invoice receipt contract report form",
    )

    result = ExtractionResult(content="Mixed document with various keywords", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.15)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is not None
    assert confidence is not None
    assert confidence >= 0.15


def test_classify_document_confidence_calculation(mocker: MockerFixture) -> None:
    mocker.patch(
        "kreuzberg._document_classification._get_translated_text",
        return_value="invoice invoice number total amount",
    )

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})
    config = ExtractionConfig(auto_detect_document_type=True, document_type_confidence_threshold=0.5)

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    assert confidence == 1.0


def test_missing_deep_translator_import_error(mocker: MockerFixture) -> None:
    original_module = sys.modules.pop("deep_translator", None)

    try:

        def mock_import(name: str, *args: Any, **kwargs: Any) -> Any:
            if name == "deep_translator":
                raise ImportError("No module named 'deep_translator'")
            return original_import(name, *args, **kwargs)

        original_import = builtins.__import__
        mocker.patch("builtins.__import__", side_effect=mock_import)

        from kreuzberg._document_classification import _get_translated_text

        result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={})

        with pytest.raises(MissingDependencyError) as exc_info:
            _get_translated_text(result)

        assert "deep-translator" in str(exc_info.value)
        assert "pip install 'kreuzberg[document-classification]'" in str(exc_info.value)
    finally:
        if original_module is not None:
            sys.modules["deep_translator"] = original_module
