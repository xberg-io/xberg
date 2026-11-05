//! The whole-document OCR route's page window (#2428): pages start as slots free, come back in page order, and
//! the window holds no more pages than the free-memory estimate allows.

use super::pipeline::*;
#[cfg(feature = "pdf")]
use super::tests::tests::PinnedAvailableMemory;
use crate::core::config::ExtractionConfig;

/// A backend for the #2428 tests: it reads the page index from the page's aspect ratio (`10 + index` wide
/// for every 10 high, at any render DPI), awaits
/// `answer(index)` and answers with its text, or with `page <index>` when it returns `None`.
type PageAnswer = std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>>;

struct PageIndexBackend {
    name: &'static str,
    answer: fn(usize) -> PageAnswer,
}

#[async_trait::async_trait]
impl crate::plugins::OcrBackend for PageIndexBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        image_bytes: &[u8],
        _: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let page = image::load_from_memory(image_bytes).expect("a PNG page");
        let index = (10.0 * f64::from(page.width()) / f64::from(page.height()) - 10.0).round() as usize;
        let content = (self.answer)(index).await.unwrap_or_else(|| format!("page {index}"));
        Ok(crate::types::ExtractedDocument {
            content,
            ..Default::default()
        })
    }
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Uncalibrated
    }
}

impl crate::plugins::Plugin for PageIndexBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

/// Runs `extract_with_ocr_for_page` over `pages` pre-rendered pages with a thread budget of `threads` and
/// the backend above, and returns the page texts.
async fn page_texts_with_page_index_backend(backend: PageIndexBackend, pages: u32, threads: usize) -> Vec<String> {
    let images: Vec<_> = (0..pages).map(|i| image::DynamicImage::new_rgb8(10 + i, 10)).collect();
    page_texts_with_page_index_backend_from(backend, None, Some(&images), threads).await
}

/// The same run over a PDF of `pages` pages that the route renders itself, `10 + index` wide for every 10
/// high, so the window's size comes from the free-memory estimate.
#[cfg(feature = "pdf")]
async fn pdf_page_texts_with_page_index_backend(
    backend: PageIndexBackend,
    pdf: &[u8],
    threads: usize,
) -> Vec<String> {
    page_texts_with_page_index_backend_from(backend, Some(pdf), None, threads).await
}

#[cfg(feature = "pdf")]
fn page_index_pdf(pages: u32) -> Vec<u8> {
    let media_boxes: Vec<(u32, u32)> = (0..pages).map(|i| ((10 + i) * 20, 200)).collect();
    build_minimal_pdf_with_media_boxes(&media_boxes)
}

async fn page_texts_with_page_index_backend_from(
    backend: PageIndexBackend,
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    threads: usize,
) -> Vec<String> {
    use crate::core::config::{ConcurrencyConfig, OcrConfig};
    let name = backend.name;
    crate::plugins::register_ocr_backend(std::sync::Arc::new(backend)).unwrap();
    #[allow(clippy::needless_update)]
    let config = ExtractionConfig {
        use_cache: false,
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(threads),
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: name.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_with_ocr_for_page(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        0,
        false,
        None,
        None,
        0,
        None,
    )
    .await;
    crate::plugins::unregister_ocr_backend(name).unwrap();
    result.expect("OCR run").6
}

/// #2428: with a thread budget of two, page 2 must start while page 0 still recognizes. Lockstep
/// batches start page 2 only after the whole first batch (pages 0 and 1) finished, so page 0 waits
/// out its timeout and answers `lockstep`.
#[tokio::test]
#[serial_test::serial]
async fn extract_with_ocr_for_page_starts_a_page_while_an_earlier_page_still_recognizes() {
    static PAGE_TWO_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    PAGE_TWO_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);
    let backend = PageIndexBackend {
        name: "page-window-overlap-test-backend",
        answer: |index| {
            Box::pin(async move {
                if index == 2 {
                    PAGE_TWO_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                if index != 0 {
                    return None;
                }
                for _ in 0..300 {
                    if PAGE_TWO_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
                        return Some("streamed".to_string());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Some("lockstep".to_string())
            })
        },
    };
    let texts = page_texts_with_page_index_backend(backend, 3, 2).await;
    assert_eq!(texts, vec!["streamed", "page 1", "page 2"]);
}

/// #2428: pages that finish out of order still come back in page order. Page 0 is the slowest and
/// page 3 the fastest.
#[tokio::test]
#[serial_test::serial]
async fn extract_with_ocr_for_page_keeps_page_order_when_pages_finish_out_of_order() {
    let backend = PageIndexBackend {
        name: "page-window-order-test-backend",
        answer: |index| {
            Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(40 * (4 - index.min(4)) as u64)).await;
                None
            })
        },
    };
    let texts = page_texts_with_page_index_backend(backend, 6, 4).await;
    let expected: Vec<String> = (0..6).map(|i| format!("page {i}")).collect();
    assert_eq!(texts, expected);
}

/// Page 0's answer for the #2428 memory tests: `streamed` when page 2 starts while page 0 still
/// recognizes, `held` when page 2 has not started after three seconds.
#[cfg(feature = "pdf")]
static MEMORY_TEST_PAGE_TWO_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "pdf")]
fn memory_test_backend(name: &'static str) -> PageIndexBackend {
    MEMORY_TEST_PAGE_TWO_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);
    PageIndexBackend {
        name,
        answer: |index| {
            Box::pin(async move {
                if index == 2 {
                    MEMORY_TEST_PAGE_TWO_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                if index != 0 {
                    return None;
                }
                for _ in 0..300 {
                    if MEMORY_TEST_PAGE_TWO_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
                        return Some("streamed".to_string());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Some("held".to_string())
            })
        },
    }
}

/// #2428 review: every page the window holds counts as one page of the free-memory estimate that sizes
/// the batch. With free memory for two pages and a thread budget of two, pages 0 and 1 fill the window,
/// so page 2 cannot render or start until page 0 has left it.
#[cfg(feature = "pdf")]
#[tokio::test]
#[serial_test::serial]
async fn extract_with_ocr_for_page_holds_no_more_pages_than_free_memory_allows() {
    let pdf = page_index_pdf(4);
    let _memory =
        PinnedAvailableMemory::set(512 * 1024 * 1024 + pdf.len() + 2 * UNMEASURED_OCR_PAGE_WORKING_SET_BYTES);
    let backend = memory_test_backend("page-window-memory-bound-test-backend");
    let texts = pdf_page_texts_with_page_index_backend(backend, &pdf, 2).await;
    assert_eq!(texts, vec!["held", "page 1", "page 2", "page 3"]);
}

/// The other side of the test above: with free memory for four pages, the same run renders page 2 ahead
/// and starts it as soon as page 1 frees its slot.
#[cfg(feature = "pdf")]
#[tokio::test]
#[serial_test::serial]
async fn extract_with_ocr_for_page_renders_ahead_when_free_memory_has_room() {
    let pdf = page_index_pdf(4);
    let _memory =
        PinnedAvailableMemory::set(512 * 1024 * 1024 + pdf.len() + 4 * UNMEASURED_OCR_PAGE_WORKING_SET_BYTES);
    let backend = memory_test_backend("page-window-memory-room-test-backend");
    let texts = pdf_page_texts_with_page_index_backend(backend, &pdf, 2).await;
    assert_eq!(texts, vec!["streamed", "page 1", "page 2", "page 3"]);
}
