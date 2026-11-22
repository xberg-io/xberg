//! Outline, named-destination and embedded-file tree walks must visit each
//! indirect node once and finish promptly on cyclic or shared-kid trees.

mod common;

use std::sync::mpsc;
use std::time::Duration;
use xberg_native_pdf::PdfDocument;

fn within<T: Send + 'static>(seconds: u64, work: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let _ = tx.send(work());
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(seconds))
        .expect("walk did not finish")
}

fn pdf(catalog_extra: &str, objects: &[(u32, &str)]) -> Vec<u8> {
    let mut all: Vec<(u32, String)> = vec![
        (1, format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>")),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_string()),
    ];
    all.extend(objects.iter().map(|(id, body)| (*id, body.to_string())));
    common::build_pdf_from_objects(&all)
}

#[test]
fn outline_next_self_loop_yields_one_item() {
    let bytes = pdf(
        "/Outlines 7 0 R",
        &[
            (7, "<< /Type /Outlines /First 8 0 R >>"),
            (8, "<< /Title (a) /Next 8 0 R >>"),
        ],
    );
    let outline = within(3, move || {
        PdfDocument::from_bytes(bytes).unwrap().get_outline().unwrap()
    });
    assert_eq!(outline.expect("outline").len(), 1);
}

#[test]
fn outline_first_cycle_terminates() {
    let bytes = pdf(
        "/Outlines 7 0 R",
        &[
            (7, "<< /Type /Outlines /First 8 0 R >>"),
            (8, "<< /Title (a) /First 9 0 R >>"),
            (9, "<< /Title (b) /First 8 0 R >>"),
        ],
    );
    let outline = within(10, move || {
        PdfDocument::from_bytes(bytes).unwrap().get_outline().unwrap()
    });
    let outline = outline.expect("outline");
    assert_eq!(outline.len(), 1);
    assert_eq!(outline[0].children.len(), 1);
    assert!(outline[0].children[0].children.is_empty());
}

#[test]
fn named_destination_tree_with_repeated_kid_is_walked_once() {
    let bytes = pdf("/Names << /Dests 7 0 R >>", &[(7, "<< /Kids [7 0 R 7 0 R] >>")]);
    let found = within(5, move || {
        PdfDocument::from_bytes(bytes)
            .unwrap()
            .resolve_named_destination("missing")
            .unwrap()
    });
    assert_eq!(found, None);
}

#[test]
fn embedded_files_tree_with_repeated_kid_is_walked_once() {
    let bytes = pdf("/Names << /EmbeddedFiles 7 0 R >>", &[(7, "<< /Kids [7 0 R 7 0 R] >>")]);
    let files = within(5, move || {
        PdfDocument::from_bytes(bytes)
            .unwrap()
            .extract_embedded_files()
            .unwrap()
    });
    assert!(files.is_empty());
}
