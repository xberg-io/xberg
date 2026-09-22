//! Guards for the page dispatch in `extract_images_with_data` (issue #1732).
//!
//! Lives beside `images.rs` rather than in its `tests` module so that file stays inside the
//! project's file-length limit.

use super::{extract_images_with_data, page_call_thread_names};

/// The thread-name prefix this file gives every pool it builds. `record_page_thread` stores
/// the running thread's name, so a guard can count the threads of its own pool and ignore any
/// other test that reaches the same pass while it runs. ~keep
const POOL_PREFIX: &str = "xberg-image-pass-guard";

/// A `page_count`-page PDF carrying one uncompressed RGB image XObject per page.
///
/// Each image is filled with a per-page byte pattern, so a result whose pages came back in
/// the wrong order is detectable from the image bytes alone rather than only from a count.
/// Built as a hand-written object graph with its own xref table, the same way
/// `build_minimal_multi_page_pdf` in `extractors/pdf/ocr/tests.rs` builds its page-only
/// version, so the fixture needs no file on disk.
fn build_pdf_with_one_image_per_page(page_count: usize, side: u32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    offsets.push(buf.len());
    let kids: String = (0..page_count).map(|i| format!("{} 0 R ", 3 + i * 3)).collect();
    buf.extend_from_slice(
        format!(
            "2 0 obj\n<</Type /Pages /Kids [{}] /Count {}>>\nendobj\n",
            kids.trim_end(),
            page_count
        )
        .as_bytes(),
    );

    let content = b"q 150 0 0 150 25 25 cm /Im0 Do Q";
    let pixel_count = (side as usize) * (side as usize) * 3;

    for page_idx in 0..page_count {
        let page_obj = 3 + page_idx * 3;
        let content_obj = page_obj + 1;
        let image_obj = page_obj + 2;

        offsets.push(buf.len());
        buf.extend_from_slice(
            format!(
                "{page_obj} 0 obj\n<</Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
                 /Resources <</XObject <</Im0 {image_obj} 0 R>>>> /Contents {content_obj} 0 R>>\nendobj\n"
            )
            .as_bytes(),
        );

        offsets.push(buf.len());
        buf.extend_from_slice(format!("{content_obj} 0 obj\n<</Length {}>>\nstream\n", content.len()).as_bytes());
        buf.extend_from_slice(content);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            format!(
                "{image_obj} 0 obj\n<</Type /XObject /Subtype /Image /Width {side} /Height {side} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {pixel_count}>>\nstream\n"
            )
            .as_bytes(),
        );
        // A per-page pattern that is neither uniform nor shared between pages: the PNG
        // re-encode has real work to do and every page's bytes differ from its neighbours'. ~keep
        buf.extend((0..pixel_count).map(|byte_idx| ((page_idx * 37 + byte_idx * 11 + byte_idx / 97) % 251) as u8));
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    let xref_offset = buf.len();
    let total_objs = 2 + page_count * 3 + 1;
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("0 {}\n", total_objs).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", total_objs).as_bytes());
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());
    buf
}

/// The thread-name prefix `extract_in_pool` gives the pool it builds for `tag`.
fn pool_thread_prefix(tag: &str) -> String {
    format!("{POOL_PREFIX}-{tag}")
}

/// Extract every image in `pdf` inside a pool of exactly `threads` threads, whose threads are
/// named after `POOL_PREFIX` and `tag` so a caller can tell them from every other thread.
fn extract_in_pool(pdf: &[u8], threads: usize, tag: &str) -> Vec<crate::types::ExtractedImage> {
    let prefix = pool_thread_prefix(tag);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(move |index| format!("{prefix}-{index}"))
        .build()
        .expect("building a dedicated pool must succeed");
    // Wake every worker before the pass runs. A pool built a moment ago has all its workers but
    // the one that takes the job asleep, and rayon wakes a sleeper through a futex: the thread
    // running `install` gets through the whole short range before any wake lands, so a genuinely
    // parallel pass is observed on a single thread. Measured on a 32-core box, 2026-09-21: the
    // guard below saw one thread in 30 runs of 30 without this line and four in 30 of 30 with it.
    // `broadcast` returns only once every worker has run it, and it leaves the pool in the warm
    // state a real extraction finds, because there the pool is process-wide and long-lived. ~keep
    pool.broadcast(|_| ());
    pool.install(|| {
        let mut doc = crate::pdf::native::NativeDocument::open_bytes(pdf).expect("the fixture must open");
        let (images, warnings) = extract_images_with_data(&mut doc, None, None).expect("extraction must not error");
        assert!(
            warnings.is_empty(),
            "the fixture's images must all re-encode; got {:?}",
            warnings
        );
        images
    })
}

/// #1732: the embedded-image pass must dispatch pages across the thread pool, not merely be
/// fast. A wall-clock threshold on a shared build box flakes under load; this asserts the
/// MECHANISM, the same way `parallel_render_dispatches_across_more_than_one_thread` does for
/// page rendering, so it either used more than one OS thread or it did not.
///
/// The pass runs in a pool this test builds itself (4 threads, not the ambient global pool),
/// so the assertion never depends on how many CPUs the host reports. 24 pages against 4 pool
/// threads leaves no reasonable path for rayon to keep every page on the calling thread:
/// `into_par_iter()` on a range is an `IndexedParallelIterator` whose recursive `join`-based
/// splitting hands half the remaining range to a stolen thread whenever one is idle, and
/// every page here carries a real 256x256 decode and PNG re-encode.
///
/// The page-order assertions are part of the same test on purpose: parallel dispatch that
/// reorders the returned collection is a regression, and a test that only counts images
/// would not see it.
#[test]
#[serial_test::serial]
fn image_pass_dispatches_pages_across_more_than_one_thread() {
    let page_count = 24;
    let pdf = build_pdf_with_one_image_per_page(page_count, 256);
    let prefix = pool_thread_prefix("dispatch");

    page_call_thread_names()
        .lock()
        .expect("page-thread record must not be poisoned")
        .clear();

    let images = extract_in_pool(&pdf, 4, "dispatch");

    assert_eq!(
        images.len(),
        page_count,
        "every page's image must come back; got {}",
        images.len()
    );
    for (position, image) in images.iter().enumerate() {
        assert_eq!(
            image.page_number,
            Some(position as u32 + 1),
            "images must stay in page order; entry {position} came from page {:?}",
            image.page_number
        );
        assert_eq!(
            image.image_index, position as u32,
            "image_index must stay the position in the returned collection"
        );
    }

    // Copied out before the assertion so a failure here reports rather than poisoning the
    // shared record for the next test in the file. Only this pool's own threads count: the
    // record is process-global and a test running beside this one can reach the same pass. ~keep
    let recorded = page_call_thread_names()
        .lock()
        .expect("page-thread record must not be poisoned")
        .clone();
    let observed: std::collections::BTreeSet<&String> =
        recorded.iter().filter(|name| name.starts_with(&prefix)).collect();
    assert!(
        observed.len() > 1,
        "expected the image pass to be observed on more than one thread of the pool named \
         {prefix} (mechanism proof that pages dispatched in parallel), got {} of the pool's \
         threads: {:?}; every thread recorded in this process: {:?}",
        observed.len(),
        observed,
        recorded
    );
}

/// #1732: widening the thread budget must not change a single output byte. Page order, the
/// document-global `image_index`, and the re-encoded PNG bytes all have to match what one
/// thread produces.
#[test]
#[serial_test::serial]
fn image_pass_output_is_identical_at_one_thread_and_four() {
    let page_count = 12;
    let pdf = build_pdf_with_one_image_per_page(page_count, 128);

    let single = extract_in_pool(&pdf, 1, "identical-one");
    let wide = extract_in_pool(&pdf, 4, "identical-four");

    assert_eq!(single.len(), page_count, "the one-thread arm must extract every page");
    assert_eq!(
        single.len(),
        wide.len(),
        "the thread budget must not change how many images come back"
    );
    for (position, (one, four)) in single.iter().zip(wide.iter()).enumerate() {
        assert_eq!(
            one.page_number, four.page_number,
            "page order differs at entry {position}"
        );
        assert_eq!(
            one.image_index, four.image_index,
            "image_index differs at entry {position}"
        );
        assert_eq!(one.format, four.format, "format differs at entry {position}");
        assert_eq!(one.width, four.width, "width differs at entry {position}");
        assert_eq!(one.height, four.height, "height differs at entry {position}");
        assert_eq!(
            one.colorspace, four.colorspace,
            "colorspace differs at entry {position}"
        );
        assert_eq!(
            one.description, four.description,
            "alt text differs at entry {position}"
        );
        assert_eq!(
            one.bounding_box, four.bounding_box,
            "bounding box differs at entry {position}"
        );
        assert_eq!(
            one.data, four.data,
            "the image bytes at entry {position} must not depend on the thread budget"
        );
    }
}
