//! A `/PageLabels` number tree whose `/Kids` array names an ancestor must
//! terminate the walk, not the process.
//!
//! ISO 32000-1:2008 §7.9.7 describes a number tree as a tree, but nothing in
//! the file format prevents a `/Kids` entry from pointing back up. The walker
//! recursed with neither a depth cap nor a visited set, so a self-referential
//! node overflowed the stack — and a stack overflow is not a catchable panic:
//! with `panic = "abort"` in the release profile it takes the host process
//! down. The crate's two other tree walkers both carry these guards.
//!
//! Page labelling is an extraction feature, so a malformed tree degrades to
//! the ranges recovered so far with a warning rather than failing the open.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::PageLabelExtractor;

/// Assemble a one-page PDF whose catalog carries `/PageLabels 5 0 R`, with
/// objects 5.. supplied as raw dictionary bodies.
fn pdf_with_page_labels(label_objects: &[&str]) -> Vec<u8> {
    let total = 4 + label_objects.len();
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; total + 1];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(
        &mut buf,
        &mut off,
        1,
        "<< /Type /Catalog /Pages 2 0 R /PageLabels 5 0 R >>",
    );
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>",
    );
    let content = b"BT /F1 12 Tf 10 100 Td (x) Tj ET\n";
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    for (i, body) in label_objects.iter().enumerate() {
        obj(&mut buf, &mut off, 5 + i, body);
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total + 1).as_bytes());
    for offset in off.iter().take(total + 1).skip(1) {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n", total + 1).as_bytes());
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// A node whose `/Kids` names itself. Returning at all is the assertion.
#[test]
fn self_referential_number_tree_node_terminates() {
    let pdf = pdf_with_page_labels(&["<< /Kids [5 0 R] >>"]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let labels = PageLabelExtractor::extract(&doc).expect("extract must return, not abort");
    assert!(labels.is_empty(), "a cycle carries no ranges; got {labels:?}");
}

/// A two-node cycle: 5 -> 6 -> 5. A depth cap alone would still walk this a
/// long way, which is why the visited set is the load-bearing guard.
#[test]
fn two_node_number_tree_cycle_terminates() {
    let pdf = pdf_with_page_labels(&["<< /Kids [6 0 R] >>", "<< /Kids [5 0 R] >>"]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let labels = PageLabelExtractor::extract(&doc).expect("extract must return, not abort");
    assert!(labels.is_empty(), "a cycle carries no ranges; got {labels:?}");
}

/// A cycle must not discard the ranges recovered before it was reached.
#[test]
fn ranges_before_a_cycle_are_still_returned() {
    let pdf = pdf_with_page_labels(&[
        "<< /Kids [6 0 R 7 0 R] >>",
        "<< /Nums [0 << /S /r >>] >>",
        "<< /Kids [5 0 R] >>",
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let labels = PageLabelExtractor::extract(&doc).expect("extract must return, not abort");
    assert_eq!(labels.len(), 1, "the leaf before the cycle should survive");
    assert_eq!(labels[0].start_page, 0);
    assert_eq!(
        labels[0].style,
        xberg_native_pdf::extractors::PageLabelStyle::RomanLower
    );
}

/// The guards must not truncate a legal tree: a well-formed two-level tree
/// still yields every range.
#[test]
fn test_well_formed_nested_tree_is_walked_in_full() {
    let pdf = pdf_with_page_labels(&[
        "<< /Kids [6 0 R 7 0 R] >>",
        "<< /Nums [0 << /S /r >>] >>",
        "<< /Nums [4 << /S /D /St 1 >>] >>",
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let labels = PageLabelExtractor::extract(&doc).expect("extract");
    assert_eq!(labels.len(), 2, "both leaves should be walked; got {labels:?}");
    assert_eq!(labels[0].start_page, 0);
    assert_eq!(labels[1].start_page, 4);
    assert_eq!(PageLabelExtractor::get_label(&labels, 0), "i");
    assert_eq!(PageLabelExtractor::get_label(&labels, 4), "1");
}
