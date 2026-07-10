//! PDF output quality integration tests.
//!
//! Regression tests verifying that extraction output is clean and free of
//! common noise patterns (figure-internal text, arXiv watermarks, reference
//! entries misclassified as headings, repeating conference headers).
//!
//! Benchmark documents:
//! - `docling.pdf` — academic paper with figures, tables, arXiv sidebar
//! - `multi_page.pdf` — clean multi-page document (no noise expected)

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_uri_document_blocking;

use helpers::*;
use xberg::core::config::{ExtractionConfig, OutputFormat};

fn extract_markdown(relative_path: &str) -> String {
    let pdf_path = get_test_file_path(relative_path);
    if !pdf_path.exists() {
        panic!("Test document not found: {}", relative_path);
    }
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    extract_uri_document_blocking(&pdf_path, None, &config)
        .expect("extraction should succeed")
        .content
}

#[cfg(feature = "layout-detection")]
fn extract_markdown_with_layout(relative_path: &str) -> String {
    use xberg::core::config::layout::LayoutDetectionConfig;

    let pdf_path = get_test_file_path(relative_path);
    if !pdf_path.exists() {
        panic!("Test document not found: {}", relative_path);
    }
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig::default()),
        ..Default::default()
    };
    extract_uri_document_blocking(&pdf_path, None, &config)
        .expect("layout extraction should succeed")
        .content
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_docling_no_figure_text_as_headings() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown_with_layout("pdf/docling.pdf");

    for line in content.lines() {
        if line.starts_with('#') {
            assert!(
                !line.contains("{;}"),
                "Figure diagram text promoted to heading: {}",
                line
            );
            assert!(
                !line.contains("Parse PDF pages Table Structure OCR"),
                "Figure diagram text promoted to heading: {}",
                line
            );
        }
    }
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_docling_no_arxiv_watermark() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown_with_layout("pdf/docling.pdf");

    assert!(
        !content.contains("arXiv:2408.09869"),
        "arXiv watermark identifier not stripped from output"
    );
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_docling_references_not_headings() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown_with_layout("pdf/docling.pdf");

    let heading_lines: Vec<&str> = content.lines().filter(|l| l.starts_with("## ")).collect();
    for h in &heading_lines {
        assert!(
            !h.contains("PyPDFium2"),
            "Reference entry misclassified as heading: {}",
            h
        );
        assert!(
            !h.contains("LlamaIndex"),
            "Reference entry misclassified as heading: {}",
            h
        );
        assert!(
            !h.contains("PyttiuPDF"),
            "Reference entry misclassified as heading: {}",
            h
        );
    }
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_docling_key_content_preserved() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown_with_layout("pdf/docling.pdf");

    assert!(
        content.contains("Docling Technical Report"),
        "Title not found in output"
    );
    assert!(
        content.contains("Processing pipeline") || content.contains("processing pipeline"),
        "Section 'Processing pipeline' not found"
    );
    assert!(content.contains("TableFormer"), "'TableFormer' not found");
    assert!(
        content.contains("PDF backend") || content.contains("PDF backends"),
        "'PDF backends' section not found"
    );
}

#[test]
fn test_multipage_clean_output() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown("pdf/multi_page.pdf");

    assert!(content.contains("Evolution of the Word Processor"), "Title not found");
    assert!(
        content.contains("Pre-Digital Era"),
        "Section 'Pre-Digital Era' not found"
    );
    assert!(content.contains("IBM MT/ST"), "'IBM MT/ST' not found");
}

#[test]
fn test_multipage_no_noise() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown("pdf/multi_page.pdf");

    assert!(
        !content.contains("arXiv:"),
        "multipage.pdf should have no arXiv identifiers"
    );
}

/// Regression test: nougat_014.pdf (105 pages) must complete layout extraction
/// without error or panic.  Previously failed in CI because the single-shot
/// N=105 ONNX batch tensor exceeded available memory on constrained runners.
#[cfg(feature = "layout-detection")]
#[test]
fn test_nougat_014_layout_extraction_completes() {
    if !test_documents_available() {
        return;
    }
    let path = get_test_file_path("pdf/nougat_014.pdf");
    if !path.exists() {
        return;
    }
    let content = extract_markdown_with_layout("pdf/nougat_014.pdf");
    assert!(
        content.len() > 500,
        "nougat_014.pdf layout extraction produced unexpectedly short output ({} chars)",
        content.len()
    );
}

/// Regression test: iso_21111_10 (214 pages) must complete layout extraction
/// without error or panic.  Previously failed in CI because the single-shot
/// N=214 ONNX batch tensor (~1 GB) was too large for constrained runners.
#[cfg(feature = "layout-detection")]
#[test]
fn test_iso_21111_10_layout_extraction_completes() {
    if !test_documents_available() {
        return;
    }
    let path = get_test_file_path("pdf/iso_21111_10_2021_road_vehicles_in_vehicle_ethernet_conformance_test_plans.pdf");
    if !path.exists() {
        return;
    }
    let content = extract_markdown_with_layout(
        "pdf/iso_21111_10_2021_road_vehicles_in_vehicle_ethernet_conformance_test_plans.pdf",
    );
    assert!(
        content.len() > 500,
        "iso_21111_10 layout extraction produced unexpectedly short output ({} chars)",
        content.len()
    );
}

/// Regression test for reading-order inversion caused by coordinate system mismatch.
///
/// pdf_oxide returns bounding boxes in PDF coordinates (y=0 at bottom of page, y increases
/// upward). Storing these directly in SegmentData.baseline_y and then sorting descending
/// in `assemble_page_elements_with_tables` produces correct top-to-bottom reading order.
///
/// The bug was an erroneous conversion `page_height - bbox.y - bbox.height` that turned
/// PDF coordinates into screen coordinates (y=0 at top). The descending sort then placed
/// bottom-of-page content first, completely reversing the output.
#[test]
fn test_pdf_structure_reading_order() {
    if !test_documents_available() {
        return;
    }
    let content = extract_markdown("vendored/pdfplumber/pdf/pdf_structure.pdf");

    let title_pos = content.find("Titre du document").expect("title not found in output");
    let tableau_pos = content.find("Tableau").expect("'Tableau' section not found in output");
    assert!(
        title_pos < tableau_pos,
        "'Titre du document' (top of page) must appear before 'Tableau' (bottom of page); \
         got title at byte {title_pos}, Tableau at byte {tableau_pos}. \
         This indicates a reading-order inversion — check PDF coordinate handling in \
         crates/xberg/src/pdf/oxide/hierarchy.rs"
    );
}
