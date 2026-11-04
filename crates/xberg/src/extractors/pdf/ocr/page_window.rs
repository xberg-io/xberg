//! The page window of a full-document OCR run: pages render ahead, a fixed number recognize at once, and
//! finished pages come back in page order.

use std::collections::VecDeque;
use std::ops::Range;

use super::rendering::EncodedPage;

/// Pages the window holds per recognition slot: one recognizing and one rendered ahead or finished behind a
/// slower earlier page. ~keep
const HELD_PAGES_PER_SLOT: usize = 2;

/// What recognition returned for one page.
pub(super) struct PageRecognition {
    /// `(correction_degrees, upright_width, upright_height)` applied before the backend call.
    pub(super) upright_correction: (u32, u32, u32),
    /// The backend result, or an empty document when the call failed.
    pub(super) document: crate::types::ExtractedDocument,
    /// The described backend failure, if the call failed.
    pub(super) error: Option<String>,
}

/// Consecutive finished pages, starting at `first_page`, in page order.
pub(super) struct ReadyPages {
    pub(super) first_page: usize,
    pub(super) encoded: Vec<EncodedPage>,
    pub(super) documents: Vec<Option<crate::types::ExtractedDocument>>,
    pub(super) errors: Vec<Option<String>>,
    pub(super) upright_corrections: Vec<(u32, u32, u32)>,
}

/// Which pages to render, which to recognize, and which are ready, for one OCR run.
pub(super) struct PageWindow {
    total_pages: usize,
    recognition_slots: usize,
    first_held: usize,
    held: VecDeque<(EncodedPage, Option<PageRecognition>)>,
    next_to_recognize: usize,
    in_flight: usize,
    render_error: Option<crate::XbergError>,
}

impl PageWindow {
    pub(super) fn new(total_pages: usize, recognition_slots: usize) -> Self {
        Self {
            total_pages,
            recognition_slots: recognition_slots.max(1),
            first_held: 0,
            held: VecDeque::new(),
            next_to_recognize: 0,
            in_flight: 0,
            render_error: None,
        }
    }

    /// The pages to render next: at most one slot count at a time, none past the hold limit or after a
    /// render error.
    pub(super) fn pages_to_render(&self) -> Range<usize> {
        let start = self.first_held + self.held.len();
        if self.render_error.is_some() {
            return start..start;
        }
        let end = (start + self.recognition_slots)
            .min(self.first_held + HELD_PAGES_PER_SLOT * self.recognition_slots)
            .min(self.total_pages);
        start..end.max(start)
    }

    /// Store the pages a render of [`Self::pages_to_render`] returned. A render that returns a different
    /// number of pages becomes a render error, because a held page's position is its page number.
    pub(super) fn push_rendered(&mut self, pages: Vec<EncodedPage>) {
        let asked = self.pages_to_render().len();
        if pages.len() != asked {
            self.render_error = Some(crate::XbergError::Parsing {
                message: format!("OCR page rendering returned {} of {asked} pages", pages.len()),
                source: None,
            });
            return;
        }
        self.held.extend(pages.into_iter().map(|page| (page, None)));
    }

    /// Keep a render failure until every earlier page is out of the window; see [`Self::take_render_error`].
    pub(super) fn hold_render_error(&mut self, error: crate::XbergError) {
        self.render_error = Some(error);
    }

    /// The next rendered page to recognize, when a recognition slot is free.
    pub(super) fn next_to_recognize(&mut self) -> Option<EncodedPage> {
        let position = self.next_to_recognize - self.first_held;
        if self.in_flight >= self.recognition_slots || position >= self.held.len() {
            return None;
        }
        self.next_to_recognize += 1;
        self.in_flight += 1;
        Some(self.held[position].0.clone())
    }

    /// The rendered page `page_idx`, which the window holds from its render until it leaves in a ready batch.
    #[cfg(all(feature = "ocr", feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    pub(super) fn held_page(&self, page_idx: usize) -> &EncodedPage {
        &self.held[page_idx - self.first_held].0
    }

    /// Record the recognition of `page_idx` and free its slot.
    pub(super) fn finish(&mut self, page_idx: usize, recognition: PageRecognition) {
        self.in_flight -= 1;
        self.held[page_idx - self.first_held].1 = Some(recognition);
    }

    /// Remove and return the finished pages at the front of the window, if any.
    pub(super) fn take_ready(&mut self) -> Option<ReadyPages> {
        let count = self.held.iter().take_while(|(_, done)| done.is_some()).count();
        if count == 0 {
            return None;
        }
        let mut ready = ReadyPages {
            first_page: self.first_held,
            encoded: Vec::with_capacity(count),
            documents: Vec::with_capacity(count),
            errors: Vec::with_capacity(count),
            upright_corrections: Vec::with_capacity(count),
        };
        for (page, recognition) in self.held.drain(..count) {
            let recognition = recognition.expect("counted as finished above");
            ready.encoded.push(page);
            ready.documents.push(Some(recognition.document));
            ready.errors.push(recognition.error);
            ready.upright_corrections.push(recognition.upright_correction);
        }
        self.first_held += count;
        Some(ready)
    }

    /// The held render failure, once every page before it has left the window.
    pub(super) fn take_render_error(&mut self) -> Option<crate::XbergError> {
        if self.held.is_empty() {
            self.render_error.take()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn rendered(pages: Range<usize>) -> Vec<EncodedPage> {
        pages.map(|idx| (idx, Arc::new(Vec::new()), 10, 10)).collect()
    }

    fn recognized(text: &str) -> PageRecognition {
        PageRecognition {
            upright_correction: (0, 10, 10),
            document: crate::types::ExtractedDocument {
                content: text.to_string(),
                ..Default::default()
            },
            error: None,
        }
    }

    fn launch_all(window: &mut PageWindow) -> Vec<usize> {
        std::iter::from_fn(|| window.next_to_recognize().map(|page| page.0)).collect()
    }

    #[test]
    fn a_finished_page_frees_its_slot_for_the_next_page_before_the_others_finish() {
        let mut window = PageWindow::new(6, 2);
        window.push_rendered(rendered(window.pages_to_render()));
        assert_eq!(launch_all(&mut window), vec![0, 1]);
        window.push_rendered(rendered(window.pages_to_render()));
        assert_eq!(launch_all(&mut window), Vec::<usize>::new(), "both slots are busy");

        window.finish(1, recognized("one"));
        assert_eq!(
            launch_all(&mut window),
            vec![2],
            "page 2 must start while page 0 still recognizes"
        );
        assert!(window.take_ready().is_none(), "page 1 waits behind page 0");
    }

    #[test]
    fn ready_pages_come_back_in_page_order_when_recognition_finishes_out_of_order() {
        let mut window = PageWindow::new(3, 3);
        window.push_rendered(rendered(window.pages_to_render()));
        assert_eq!(launch_all(&mut window), vec![0, 1, 2]);
        window.finish(2, recognized("two"));
        window.finish(1, recognized("one"));
        assert!(window.take_ready().is_none());
        window.finish(0, recognized("zero"));

        let ready = window.take_ready().expect("pages 0 to 2 are finished");
        assert_eq!(ready.first_page, 0);
        let pages: Vec<usize> = ready.encoded.iter().map(|page| page.0).collect();
        assert_eq!(pages, vec![0, 1, 2]);
        let texts: Vec<&str> = ready
            .documents
            .iter()
            .map(|document| document.as_ref().expect("finished").content.as_str())
            .collect();
        assert_eq!(texts, vec!["zero", "one", "two"]);
        assert_eq!(window.in_flight, 0);
        assert!(window.pages_to_render().is_empty(), "every page is rendered");
    }

    #[test]
    fn the_window_holds_at_most_two_pages_per_slot_and_renders_one_slot_count_at_a_time() {
        let mut window = PageWindow::new(100, 4);
        assert_eq!(window.pages_to_render(), 0..4);
        window.push_rendered(rendered(0..4));
        assert_eq!(window.pages_to_render(), 4..8);
        window.push_rendered(rendered(4..8));
        assert!(window.pages_to_render().is_empty(), "eight pages held for four slots");

        assert_eq!(launch_all(&mut window), vec![0, 1, 2, 3]);
        window.finish(0, recognized("zero"));
        assert_eq!(window.take_ready().expect("page 0").encoded.len(), 1);
        assert_eq!(window.pages_to_render(), 8..9);
    }

    #[test]
    fn a_render_error_waits_until_every_earlier_page_has_left_the_window() {
        let mut window = PageWindow::new(4, 2);
        window.push_rendered(rendered(window.pages_to_render()));
        window.hold_render_error(crate::XbergError::Parsing {
            message: "render failed".to_string(),
            source: None,
        });
        assert!(window.pages_to_render().is_empty(), "no render after a failure");
        assert_eq!(launch_all(&mut window), vec![0, 1]);
        assert!(window.take_render_error().is_none(), "pages 0 and 1 are still held");

        window.finish(0, recognized("zero"));
        window.finish(1, recognized("one"));
        assert_eq!(window.take_ready().expect("pages 0 and 1").encoded.len(), 2);
        let error = window.take_render_error().expect("the held failure");
        assert!(error.to_string().contains("render failed"), "{error}");
    }

    #[test]
    fn a_render_that_returns_fewer_pages_than_asked_becomes_a_render_error() {
        let mut window = PageWindow::new(4, 2);
        window.push_rendered(rendered(0..1));
        assert!(window.pages_to_render().is_empty(), "no render after a failure");
        assert_eq!(launch_all(&mut window), Vec::<usize>::new(), "no short render is held");
        let error = window.take_render_error().expect("the short render");
        assert!(error.to_string().contains("returned 1 of 2 pages"), "{error}");
    }
}
