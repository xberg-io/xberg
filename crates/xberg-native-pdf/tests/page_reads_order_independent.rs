//! A document's text is a property of the document, not of the schedule that
//! read it.
//!
//! Two callers extracting the same PDF must get the same text, so reading the
//! pages concurrently must produce exactly what reading them in order produces.
//! The fixture is a 7-page report whose body is CJK text in embedded CID font
//! subsets: its objects are large enough that a cold load of one page overlaps
//! a cold load of another, which is the window these tests aim at.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg_native_pdf::document::PdfDocument;

const FIXTURE: &str = "tests/fixtures/1.pdf";
const THREADS: usize = 8;

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(FIXTURE).expect("read fixture")
}

fn digest(parts: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Read every page in index order through one handle.
fn read_in_order(bytes: &[u8], pages: usize) -> Vec<String> {
    let doc = PdfDocument::from_bytes(bytes.to_vec()).expect("open");
    (0..pages)
        .map(|page| doc.extract_text(page).unwrap_or_default())
        .collect()
}

/// Read every page through one shared handle, with `THREADS` workers claiming
/// page indices from a shared counter.
fn read_concurrently(bytes: &[u8], pages: usize) -> Vec<String> {
    let doc = Arc::new(PdfDocument::from_bytes(bytes.to_vec()).expect("open"));
    let slots: Arc<Vec<std::sync::Mutex<String>>> =
        Arc::new((0..pages).map(|_| std::sync::Mutex::new(String::new())).collect());
    let next = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(std::sync::Barrier::new(THREADS));

    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let doc = Arc::clone(&doc);
            let slots = Arc::clone(&slots);
            let next = Arc::clone(&next);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                loop {
                    let page = next.fetch_add(1, Ordering::SeqCst);
                    if page >= slots.len() {
                        break;
                    }
                    let text = doc.extract_text(page).unwrap_or_default();
                    *slots[page].lock().expect("slot") = text;
                }
            })
        })
        .collect();

    for worker in workers {
        worker.join().expect("worker panicked");
    }

    slots.iter().map(|slot| slot.lock().expect("slot").clone()).collect()
}

/// A page-parallel read is byte-identical to a sequential one.
#[test]
fn concurrent_page_reads_match_sequential_text() {
    const REPETITIONS: usize = 80;

    let bytes = fixture_bytes();
    let pages = PdfDocument::from_bytes(bytes.clone())
        .expect("open")
        .page_count()
        .expect("page count");
    assert_eq!(pages, 7, "fixture page count changed");

    let expected = read_in_order(&bytes, pages);
    let expected_len: usize = expected.iter().map(|page| page.len()).sum();
    assert!(expected_len > 10_000, "fixture yielded {expected_len} bytes of text");

    for repetition in 0..REPETITIONS {
        let actual = read_concurrently(&bytes, pages);
        if actual != expected {
            let differing: Vec<usize> = (0..pages).filter(|&page| actual[page] != expected[page]).collect();
            panic!(
                "repetition {repetition}: concurrent read differs from sequential read\n  \
                 sequential digest {} ({expected_len} bytes)\n  \
                 concurrent digest {} ({} bytes)\n  \
                 differing pages {differing:?}",
                digest(&expected),
                digest(&actual),
                actual.iter().map(|page| page.len()).sum::<usize>(),
            );
        }
    }
}

/// A second shape of the same invariant: many threads reading the SAME page
/// through one cold handle. Nothing here partitions the work, so every worker
/// races for the same objects, and each must still get the sequential text.
#[test]
fn concurrent_reads_of_one_page_match_sequential_text() {
    const REPETITIONS: usize = 60;
    const PAGE: usize = 1;

    let bytes = fixture_bytes();
    let expected = {
        let doc = PdfDocument::from_bytes(bytes.clone()).expect("open");
        doc.extract_text(PAGE).expect("extract")
    };
    assert!(!expected.is_empty(), "fixture page {PAGE} yielded no text");

    for repetition in 0..REPETITIONS {
        let doc = Arc::new(PdfDocument::from_bytes(bytes.clone()).expect("open"));
        let start = Arc::new(std::sync::Barrier::new(THREADS));
        let workers: Vec<_> = (0..THREADS)
            .map(|_| {
                let doc = Arc::clone(&doc);
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    start.wait();
                    doc.extract_text(PAGE).unwrap_or_default()
                })
            })
            .collect();

        for worker in workers {
            let actual = worker.join().expect("worker panicked");
            assert_eq!(
                actual.len(),
                expected.len(),
                "repetition {repetition}: page {PAGE} read concurrently gave {} bytes, sequential read gave {} bytes",
                actual.len(),
                expected.len()
            );
            assert!(
                actual == expected,
                "repetition {repetition}: page {PAGE} text differs from the sequential read"
            );
        }
    }
}

/// Assemble a PDF with a correct xref from raw object bodies.
/// `objects[i]` is the body of object i+1 (no "N 0 obj"/"endobj" wrapper).
fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

/// Two pages whose `/Font` dictionaries are written inline under the same
/// resource name `/F1`, each remapping code 65 (`A`) to a different glyph.
/// Both pages draw the single byte `A`, so page 1 reads `a` and page 2 reads
/// `b` only if each page decodes through its own font. An inline dictionary
/// has no object id, so the only thing that can key a shared entry for it is
/// the resource name, which both pages share.
fn two_pages_with_conflicting_inline_fonts() -> Vec<u8> {
    let page = |glyph: &str, content_id: usize| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
             /Encoding << /Type /Encoding /Differences [65 /{glyph}] >> >> >> >> \
             /Contents {content_id} 0 R >>"
        )
        .into_bytes()
    };
    let content = b"BT /F1 12 Tf 20 100 Td (A) Tj ET";
    let stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    let mut content_obj = stream.clone();
    content_obj.extend_from_slice(content);
    content_obj.extend_from_slice(b"\nendstream");

    build_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        page("a", 5),
        page("b", 6),
        content_obj.clone(),
        content_obj,
    ])
}

/// Single-threaded, the text of a page does not depend on which page was read
/// before it: front-to-back and back-to-front give the same per-page text, and
/// each page decodes through the font its own resources name.
#[test]
fn page_text_is_the_same_read_forward_and_backward() {
    let bytes = two_pages_with_conflicting_inline_fonts();

    let forward = read_in_order(&bytes, 2);

    let backward = {
        let doc = PdfDocument::from_bytes(bytes.clone()).expect("open");
        let second = doc.extract_text(1).expect("page 2");
        let first = doc.extract_text(0).expect("page 1");
        vec![first, second]
    };

    let trimmed = |pages: &[String]| pages.iter().map(|page| page.trim().to_string()).collect::<Vec<_>>();
    assert_eq!(trimmed(&forward), vec!["a", "b"], "forward read: page 2 must decode through its own font");
    assert_eq!(trimmed(&backward), vec!["a", "b"], "backward read: page 1 must decode through its own font");
    assert_eq!(forward, backward, "page text depends on the order the pages were read");
}
