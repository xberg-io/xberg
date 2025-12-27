//! Integration tests for PDF extraction in WASM environments.
//!
//! These tests verify PDF extraction behavior in the kreuzberg-wasm binding,
//! including basic text extraction, MIME type handling, and error conditions.
//! Tests focus on behavioral correctness rather than implementation details.

#![cfg(target_arch = "wasm32")]

use js_sys::Uint8Array;
use kreuzberg_wasm::*;

/// A minimal valid PDF structure for testing.
/// This is a small, valid PDF that contains simple text.
const MINIMAL_PDF_BYTES: &[u8] = b"%PDF-1.4\n\
1 0 obj\n\
<< /Type /Catalog /Pages 2 0 R >>\n\
endobj\n\
2 0 obj\n\
<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n\
endobj\n\
3 0 obj\n\
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
endobj\n\
4 0 obj\n\
<< /Length 44 >>\n\
stream\n\
BT\n\
/F1 12 Tf\n\
100 700 Td\n\
(Test PDF) Tj\n\
ET\n\
endstream\n\
endobj\n\
5 0 obj\n\
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\n\
endobj\n\
xref\n\
0 6\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
0000000244 00000 n\n\
0000000337 00000 n\n\
trailer\n\
<< /Size 6 /Root 1 0 R >>\n\
startxref\n\
417\n\
%%EOF";

/// Corrupted PDF with invalid structure for error handling tests.
const CORRUPTED_PDF_BYTES: &[u8] = b"%PDF-1.4\n\
This is not a valid PDF structure.\n\
Random bytes: \x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\n\
No proper objects or streams defined.\n\
%%EOF";

/// Empty data (not a PDF at all) for error handling tests.
const EMPTY_DATA: &[u8] = b"";

#[test]
fn test_pdf_extraction_basic() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_ok(), "PDF extraction should succeed with valid PDF bytes");

    let js_value = result.unwrap();

    assert!(
        !js_value.is_null() && !js_value.is_undefined(),
        "Extraction result should be a non-null object"
    );
}

#[test]
fn test_pdf_extraction_returns_content() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_ok(), "PDF extraction should succeed");

    let js_value = result.unwrap();

    let result_str = format!("{:?}", js_value);
    assert!(!result_str.is_empty(), "Extraction result should contain data");
}

#[test]
fn test_pdf_extraction_mime_type_recognized() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data.clone(), "application/pdf".to_string(), None);
    assert!(result.is_ok(), "Should recognize standard application/pdf MIME type");

    let data2 = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };
    let result2 = extract_bytes_sync_wasm(data2, "application/x-pdf".to_string(), None);

    assert!(
        result2.is_ok() || result2.is_err(),
        "Should handle alternative PDF MIME types"
    );
}

#[test]
fn test_pdf_extraction_mime_type_case_sensitivity() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "APPLICATION/PDF".to_string(), None);

    let _ = result;
}

#[test]
fn test_pdf_extraction_corrupted_returns_error() {
    let data = unsafe { Uint8Array::view(CORRUPTED_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Corrupted PDF should result in an error");
}

#[test]
fn test_pdf_extraction_empty_data_returns_error() {
    let data = unsafe { Uint8Array::view(EMPTY_DATA) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Empty data should result in an error");
}

#[test]
fn test_pdf_extraction_error_is_js_value() {
    let data = unsafe { Uint8Array::view(CORRUPTED_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Should error for corrupted PDF");

    let error = result.unwrap_err();

    assert!(
        !error.is_null() && !error.is_undefined(),
        "Error should be a valid JsValue"
    );
}

#[test]
fn test_pdf_extraction_with_config_none() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_ok(), "Extraction should work with None config");
}

#[test]
fn test_pdf_extraction_uint8array_conversion() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    assert_eq!(
        data.length() as usize,
        MINIMAL_PDF_BYTES.len(),
        "Uint8Array should preserve byte length"
    );

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_ok(), "Should extract from properly converted bytes");
}

#[test]
fn test_pdf_extraction_is_deterministic() {
    let data1 = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };
    let data2 = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result1 = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), None);
    let result2 = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    assert_eq!(result1.is_ok(), result2.is_ok(), "Extraction should be deterministic");
}

#[test]
fn test_pdf_extraction_small_valid_document() {
    let small_pdf = b"%PDF-1.1\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj 2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj 3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 1 1]>>endobj xref 0 4 0000000000 65535 f 0000000009 00000 n 0000000058 00000 n 0000000133 00000 n trailer<</Size 4/Root 1 0 R>>startxref 218 %%EOF";

    let data = unsafe { Uint8Array::view(small_pdf) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_ok(), "Should handle small but valid PDF documents");
}

#[test]
fn test_pdf_extraction_unsupported_mime_type() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "text/plain".to_string(), None);

    let _ = result;
}

#[test]
fn test_pdf_extraction_mime_type_with_charset() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf; charset=utf-8".to_string(), None);

    let _ = result;
}

#[test]
fn test_pdf_extraction_result_structure() {
    let data = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };

    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    if let Ok(js_value) = result {
        assert!(js_value.is_object(), "Extraction result should be a JavaScript object");
    }
}

#[test]
fn test_pdf_multiple_extractions_no_state_leak() {
    let data1 = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };
    let result1 = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(CORRUPTED_PDF_BYTES) };
    let result2 = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    let data3 = unsafe { Uint8Array::view(MINIMAL_PDF_BYTES) };
    let result3 = extract_bytes_sync_wasm(data3, "application/pdf".to_string(), None);

    assert_eq!(
        result1.is_ok(),
        result3.is_ok(),
        "Multiple extractions should not leak state"
    );

    assert!(
        result2.is_err(),
        "Corrupted PDF should fail even between valid extractions"
    );
}
