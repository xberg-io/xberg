//! Page-tree walks must count each node once, order pages by /Kids even when
//! /Type is missing, and never mistake an annotation for a page.

mod common;

use std::sync::mpsc;
use std::time::Duration;
use xberg_native_pdf::PdfDocument;

fn within_ten_seconds<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("page-tree walk did not finish")
}

#[test]
fn self_referencing_pages_node_with_negative_count_is_counted_once_per_node() {
    let pdf = common::build_pdf_from_objects(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>".into()),
        (2, "<< /Type /Pages /Kids [2 0 R 2 0 R 3 0 R] /Count -1 >>".into()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".into()),
    ]);
    let count = within_ten_seconds(move || PdfDocument::from_bytes(pdf).expect("parse").page_count());
    assert_eq!(count.expect("page count"), 1);
}

#[test]
fn typeless_page_tree_keeps_kids_order_under_shuffled_object_numbers() {
    let pdf = common::build_pdf_from_objects(&[
        (1, "<< /Type /Catalog /Pages 9 0 R >>".into()),
        (9, "<< /Kids [7 0 R 5 0 R 8 0 R] /Count 3 >>".into()),
        (7, "<< /Parent 9 0 R /MediaBox [0 0 300 100] >>".into()),
        (5, "<< /Parent 9 0 R /MediaBox [0 0 200 100] >>".into()),
        (8, "<< /Parent 9 0 R /MediaBox [0 0 100 100] >>".into()),
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    assert_eq!(doc.page_count().expect("count"), 3);
    let widths: Vec<f32> = (0..3)
        .map(|i| doc.get_page_media_box(i).expect("media box").2)
        .collect();
    assert_eq!(widths, vec![300.0, 200.0, 100.0]);
}

#[test]
fn annotation_with_contents_is_not_a_page() {
    let pdf = common::build_pdf_from_objects(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>".into()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>".into(),
        ),
        (4, "<< /Subtype /Text /Contents (note) /Rect [0 0 1 1] >>".into()),
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    assert!(doc.get_page(0).is_ok());
    assert!(doc.get_page(1).is_err(), "annotation dict must not be served as page 1");
}
