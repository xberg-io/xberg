//! A tagged `/Table` with content on many pages is held once, not once per page.
//!
//! Peak heap is measured with a counting global allocator. This file is its own
//! test binary, and the tests share a lock so their measurements do not overlap.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use xberg_native_pdf::PdfDocument;

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call forwards to `System` with the caller's arguments unchanged;
// the counters only observe sizes and never affect the returned pointers.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let now = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

static SERIAL: Mutex<()> = Mutex::new(());

fn pdf(bodies: &[String]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    let n = bodies.len() + 1;
    out.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// A tagged document of `pages` pages, each drawing `cells_per_page` marked
/// runs, and one `/Table` whose rows hold one `/TD` per run — so the table has
/// content on every page.
fn table_across_pages(pages: usize, cells_per_page: usize) -> Vec<u8> {
    // Objects: 1 catalog, 2 pages, 3 font, 4 struct tree root, 5 table,
    // then per page: page, content, TR, and one TD per cell.
    let per_page = 3 + cells_per_page;
    let first = 6;
    let page_obj = |p: usize| first + p * per_page;
    let content_obj = |p: usize| page_obj(p) + 1;
    let tr_obj = |p: usize| page_obj(p) + 2;
    let td_obj = |p: usize, c: usize| page_obj(p) + 3 + c;

    let kids: Vec<String> = (0..pages).map(|p| format!("{} 0 R", page_obj(p))).collect();
    let rows: Vec<String> = (0..pages).map(|p| format!("{} 0 R", tr_obj(p))).collect();
    let mut bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /MarkInfo << /Marked true >> >>".to_string(),
        format!("<< /Type /Pages /Kids [{}] /Count {pages} >>", kids.join(" ")),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /StructTreeRoot /K 5 0 R >>".to_string(),
        format!("<< /Type /StructElem /S /Table /P 4 0 R /K [{}] >>", rows.join(" ")),
    ];
    for p in 0..pages {
        let mut content = String::new();
        for c in 0..cells_per_page {
            let x = 72 + 120 * c;
            content.push_str(&format!(
                "/TD << /MCID {c} >> BDC BT /F1 10 Tf {x} 750 Td (p{p}c{c}) Tj ET EMC\n"
            ));
        }
        let cells: Vec<String> = (0..cells_per_page).map(|c| format!("{} 0 R", td_obj(p, c))).collect();
        bodies.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R /StructParents {p} >>",
            content_obj(p)
        ));
        bodies.push(format!("<< /Length {} >>\nstream\n{content}endstream", content.len()));
        bodies.push(format!(
            "<< /Type /StructElem /S /TR /P 5 0 R /Pg {} 0 R /K [{}] >>",
            page_obj(p),
            cells.join(" ")
        ));
        for c in 0..cells_per_page {
            bodies.push(format!(
                "<< /Type /StructElem /S /TD /P {} 0 R /Pg {} 0 R /K {c} >>",
                tr_obj(p),
                page_obj(p)
            ));
        }
    }
    pdf(&bodies)
}

/// Peak heap growth, in bytes, while extracting the text of every page, and
/// the text of each page.
fn markdown_of_every_page(bytes: Vec<u8>) -> (usize, Vec<String>) {
    let doc = PdfDocument::from_bytes(bytes).expect("fixture parses");
    let pages = doc.page_count().expect("page count");
    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let markdown: Vec<String> = (0..pages)
        .map(|i| doc.extract_text(i).expect("page converts"))
        .collect();
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    drop(doc);
    (peak, markdown)
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// One table, one row per page, across 600 pages.
#[test]
fn test_tagged_table_across_many_pages_is_not_copied_per_page() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (peak, markdown) = markdown_of_every_page(table_across_pages(600, 1));
    assert_eq!(markdown.len(), 600);
    assert!(markdown[0].contains("p0c0"), "page 0 markdown: {:?}", markdown[0]);
    assert!(
        markdown[599].contains("p599c0"),
        "last page markdown: {:?}",
        markdown[599]
    );
    assert!(
        peak < 64 << 20,
        "peak heap growth while extracting was {:.1} MiB",
        mib(peak)
    );
}

/// Doubling the page count (and so the table) must not roughly quadruple peak
/// memory.
#[test]
fn test_peak_memory_grows_linearly_with_pages_of_a_tagged_table() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (small, small_md) = markdown_of_every_page(table_across_pages(200, 4));
    let (large, large_md) = markdown_of_every_page(table_across_pages(400, 4));
    assert!(
        small_md[199].contains("p199c3"),
        "last page markdown: {:?}",
        small_md[199]
    );
    assert!(
        large_md[399].contains("p399c3"),
        "last page markdown: {:?}",
        large_md[399]
    );
    assert!(
        large < small * 3,
        "peak heap went from {:.1} MiB at 200 pages to {:.1} MiB at 400 pages",
        mib(small),
        mib(large)
    );
}
