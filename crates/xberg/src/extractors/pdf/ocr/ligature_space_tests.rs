use super::document::{heuristically_restructured_ocr_pages, ocr_structure_heuristic_char_retention};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::pdf::structure::adapters::{OcrFontSizeScale, ocr_doc_to_paragraphs};
use crate::pdf::structure::types::PdfParagraph;
use crate::types::extraction::BoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::OcrElementLevel;

fn block(text: &str, font_size_pt: &str, y0: f64, y1: f64) -> InternalElement {
    let mut element = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Block,
        },
        text,
        0,
    );
    element.bbox = Some(BoundingBox {
        x0: 10.0,
        y0,
        x1: 500.0,
        y1,
    });
    element.attributes = Some(
        [("x_fsize".to_string(), font_size_pt.to_string())]
            .into_iter()
            .collect(),
    );
    element
}

fn paragraphs(blocks: Vec<InternalElement>) -> Vec<PdfParagraph> {
    let mut page = InternalDocument::new("test");
    for element in blocks {
        page.push_element(element);
    }
    ocr_doc_to_paragraphs(&page, 1000, OcrFontSizeScale::uniform(1.0))
}

/// OCR text splits a ligature from its word with a space ("\u{FB01} eld"), and the restructured
/// route must absorb it. The extractor-geometry route keeps that space, so this pins the
/// `ocr_text: true` set at the call site in `heuristically_restructured_ocr_pages`.
#[test]
fn heuristically_restructured_ocr_pages_absorbs_the_space_after_a_ligature() {
    let page_one = paragraphs(vec![
        block("ANNUAL REPORT OVERVIEW", "28", 10.0, 50.0),
        block(
            "This document summarizes the annual results for the reporting period in detail.",
            "11",
            60.0,
            90.0,
        ),
        block("1. First item in the numbered list of findings", "11", 100.0, 130.0),
        block(
            "2. Second item continuing the numbered list of findings",
            "11",
            140.0,
            170.0,
        ),
        block(
            "Additional narrative text follows describing the broader context for readers.",
            "11",
            180.0,
            210.0,
        ),
        block(
            "The \u{FB01} eld notes describe the e\u{FB00} ort and the \u{FB02} oor plan.",
            "11",
            220.0,
            250.0,
        ),
    ]);
    let page_two = paragraphs(vec![
        block(
            "Further discussion continues here with more explanatory prose for the reader.",
            "11",
            10.0,
            40.0,
        ),
        block(
            "Another paragraph of body text appears on the second page of the document.",
            "11",
            50.0,
            80.0,
        ),
    ]);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let doc = heuristically_restructured_ocr_pages(&[page_one, page_two], &[1000.0, 1000.0], &[], &config)
        .expect("the page restructures");
    let text: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();

    assert!(
        text.iter()
            .any(|t| t.contains("The field notes describe the effort and the floor plan.")),
        "OCR ligature spaces must be absorbed, got: {text:?}"
    );
}

/// The retention gate must read each ligature code point as the letters it stands for, checked
/// against hand-written spellings and not against the table the gate itself calls (#2689).
#[test]
fn ocr_retention_gate_reads_each_ligature_as_its_letters() {
    for (ligature, spelled) in [
        ('\u{FB00}', "ff"),
        ('\u{FB01}', "fi"),
        ('\u{FB02}', "fl"),
        ('\u{FB03}', "ffi"),
        ('\u{FB04}', "ffl"),
        ('\u{FB05}', "st"),
        ('\u{FB06}', "st"),
    ] {
        let glyph = format!("o{ligature}ce");
        let letters = format!("o{spelled}ce");
        for (baseline, restructured) in [(&glyph, &letters), (&letters, &glyph)] {
            assert_eq!(
                ocr_structure_heuristic_char_retention(restructured, baseline),
                1.0,
                "U+{:04X}: {baseline:?} against {restructured:?}",
                ligature as u32
            );
        }
    }
}
