use crate::bindgen::{FPDF_DOCUMENT, FPDF_PAGE};
use crate::pdf::document::page::PdfPageContentRegenerationStrategy;
use crate::pdf::document::pages::PdfPageIndex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// A cache of [PdfPageIndex] indices for all open [PdfPage] objects.
/// We keep track of these so that we can return accurate [PdfPageIndex] values to
/// the object copying functions in [PdfPageObjectGroup], some of which depend upon
/// accurate source page indices.
static PAGE_INDEX_CACHE: Lazy<Mutex<PdfPageIndexCache>> = Lazy::new(|| Mutex::new(PdfPageIndexCache::new()));

struct PdfPageCachedProperties {
    index: PdfPageIndex,
    content_regeneration_strategy: PdfPageContentRegenerationStrategy,
}

pub(crate) struct PdfPageIndexCache {
    pages_by_index: HashMap<(FPDF_DOCUMENT, FPDF_PAGE), PdfPageCachedProperties>,
    indices_by_page: HashMap<(FPDF_DOCUMENT, PdfPageIndex), FPDF_PAGE>,
    documents_by_maximum_index: HashMap<FPDF_DOCUMENT, PdfPageIndex>,
}

impl PdfPageIndexCache {
    #[inline]
    fn new() -> Self {
        Self {
            pages_by_index: HashMap::new(),
            indices_by_page: HashMap::new(),
            documents_by_maximum_index: HashMap::new(),
        }
    }

    /// Returns the currently cached properties for the given raw document and page handles, if any.
    #[inline]
    fn get(&self, document: FPDF_DOCUMENT, page: FPDF_PAGE) -> Option<&PdfPageCachedProperties> {
        self.pages_by_index.get(&(document, page))
    }

    /// Sets the currently cached properties for the given raw document and page handles.
    #[inline]
    fn set(&mut self, document: FPDF_DOCUMENT, page: FPDF_PAGE, props: PdfPageCachedProperties) {
        // Keep track of the maximum page index for this document. We'll need to know this
        // if we have to shuffle indices to accommodate page insertions or deletions.

        match self.documents_by_maximum_index.get(&document).copied() {
            Some(maximum) => {
                if props.index > maximum {
                    self.documents_by_maximum_index.insert(document, props.index);
                }
            }
            None => {
                self.documents_by_maximum_index.insert(document, props.index);
            }
        }

        self.indices_by_page.insert((document, props.index), page);
        self.pages_by_index.insert((document, page), props);
    }

    /// Removes the cached [PdfPageIndex] value for the given raw document and page handles.
    #[inline]
    fn remove(&mut self, document: FPDF_DOCUMENT, page: FPDF_PAGE) -> Option<PdfPageCachedProperties> {
        let props = self.pages_by_index.remove(&(document, page));

        if let Some(props) = props.as_ref() {
            self.indices_by_page.remove(&(document, props.index));

            if self.documents_by_maximum_index.get(&document).copied() == Some(props.index) {
                // This page had the maximum page index for this document. Now that it's been removed
                // from the cache, we need to find the new maximum page index for this document.

                let keys = self.indices_by_page.keys();

                if keys.len() == 0 {
                    // There's no longer any page indices cached for this document.

                    self.documents_by_maximum_index.remove(&document);
                } else {
                    let mut maximum = 0;

                    for (key, index) in keys {
                        if *key == document {
                            let index = *index;

                            maximum = index.max(maximum);
                        }
                    }

                    self.documents_by_maximum_index.insert(document, maximum);
                }
            }
        }

        props
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// an insertion of the given number of pages at the given index position.
    #[inline]
    fn insert(&mut self, document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        match self.documents_by_maximum_index.get(&document).copied() {
            Some(maximum_index_for_document) => {
                if maximum_index_for_document > index {
                    // Shuffle down all page indices in the document after the given index position.

                    for index in (index..=maximum_index_for_document).rev() {
                        if let Some(page) = self.indices_by_page.get(&(document, index)).copied() {
                            // Update the indices of this page.

                            let props = self.remove(document, page);

                            let content_regeneration_strategy = if let Some(props) = props {
                                props.content_regeneration_strategy
                            } else {
                                PdfPageContentRegenerationStrategy::AutomaticOnEveryChange
                            };

                            self.set(
                                document,
                                page,
                                PdfPageCachedProperties {
                                    index: index + count,
                                    content_regeneration_strategy,
                                },
                            );
                        }
                    }
                }

                self.documents_by_maximum_index
                    .insert(document, maximum_index_for_document + count);
            }
            None => {
                // This is the first page index we're caching for this document.

                self.documents_by_maximum_index.insert(document, index + count - 1);
            }
        }
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// a deletion of the given number of pages at the given index position.
    #[inline]
    fn delete(&mut self, document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        // Shuffle up all page indices in the document after the given index position.

        let mut maximum_index_for_document = self.documents_by_maximum_index.get(&document).copied().unwrap_or(0);

        // Remove the deleted pages from the cache.

        for index in index..index + count {
            if let Some(page) = self.indices_by_page.get(&(document, index)).copied() {
                self.remove(document, page);
            }
        }

        if maximum_index_for_document > index {
            // Shuffle up all page indices in the document after the given index position.

            for index in index + 1..=maximum_index_for_document {
                if let Some(page) = self.indices_by_page.get(&(document, index)).copied() {
                    // Update the indices of this page.

                    let props = self.remove(document, page);

                    let content_regeneration_strategy = if let Some(props) = props {
                        props.content_regeneration_strategy
                    } else {
                        PdfPageContentRegenerationStrategy::AutomaticOnEveryChange
                    };

                    self.set(
                        document,
                        page,
                        PdfPageCachedProperties {
                            index: index - count,
                            content_regeneration_strategy,
                        },
                    );
                }
            }
        } else {
            maximum_index_for_document = index;
        }

        // Update the maximum index position for this document.

        if maximum_index_for_document >= count {
            self.documents_by_maximum_index
                .insert(document, maximum_index_for_document - count);
        } else {
            // There's no longer any page indices cached for this document.

            self.documents_by_maximum_index.remove(&document);
        }
    }

    #[inline]
    fn lock() -> MutexGuard<'static, PdfPageIndexCache> {
        PAGE_INDEX_CACHE.lock().unwrap()
    }

    // The remaining methods in this implementation take care of thread-safe locking.
    // These methods form the public API of the cache.

    /// Caches the given properties for the given raw document and page handles.
    #[inline]
    pub(crate) fn cache_props_for_page(
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        index: PdfPageIndex,
        content_regeneration_strategy: PdfPageContentRegenerationStrategy,
    ) {
        Self::lock().set(
            document,
            page,
            PdfPageCachedProperties {
                index,
                content_regeneration_strategy,
            },
        )
    }

    /// Returns the current [PdfPageIndex] value for the given raw document and page handles, if any.
    #[inline]
    pub(crate) fn get_index_for_page(document: FPDF_DOCUMENT, page: FPDF_PAGE) -> Option<PdfPageIndex> {
        Self::lock().get(document, page).map(|props| props.index)
    }

    /// Returns the current [PdfPageContentRegenerationStrategy] value for the given raw document
    /// and page handles, if any.
    #[inline]
    pub(crate) fn get_content_regeneration_strategy_for_page(
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
    ) -> Option<PdfPageContentRegenerationStrategy> {
        Self::lock()
            .get(document, page)
            .map(|props| props.content_regeneration_strategy)
    }

    /// Removes the cached [PdfPageIndex] value for the given raw document and page handles.
    #[inline]
    pub(crate) fn remove_index_for_page(document: FPDF_DOCUMENT, page: FPDF_PAGE) {
        Self::lock().remove(document, page);
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// an insertion of the given number of pages at the given index position.
    #[inline]
    pub(crate) fn insert_pages_at_index(document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        Self::lock().insert(document, index, count);
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// a deletion of the given number of pages at the given index position.
    #[inline]
    pub(crate) fn delete_pages_at_index(document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        Self::lock().delete(document, index, count);
    }

    /// Clears all cached entries for the given document handle.
    ///
    /// This should be called when a document is closed to prevent stale cache entries
    /// from persisting if Pdfium reuses the document handle for a different document.
    #[inline]
    pub(crate) fn clear_document(document: FPDF_DOCUMENT) {
        let mut cache = Self::lock();

        // Collect all page handles for this document
        let page_handles: Vec<FPDF_PAGE> = cache
            .pages_by_index
            .keys()
            .filter(|(doc, _)| *doc == document)
            .map(|(_, page)| *page)
            .collect();

        // Remove all entries for this document
        for page_handle in page_handles {
            cache.remove(document, page_handle);
        }

        // Remove document from maximum index tracking
        cache.documents_by_maximum_index.remove(&document);
    }
}

unsafe impl Send for PdfPageIndexCache {}

unsafe impl Sync for PdfPageIndexCache {}

#[cfg(test)]
mod tests {
    use crate::pdf::document::page::index_cache::PdfPageIndexCache;
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_cache_instantiation() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        {
            // Now let's create a blank page and get a handle to it...

            let _page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            // ... and confirm the cache updated.

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);
        }

        // The page has dropped out of scope. Confirm the cache got cleaned up.

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        // Get a new handle to the page...

        let _page = document.pages().first();

        // ... and confirm the cache updated.

        assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_and_set_index_for_page() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document_0 = pdfium.create_new_pdf()?;

        {
            // Create three blank pages.

            for _ in 1..=3 {
                document_0.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
            }

            // Since we haven't retrieved any references to these pages, the index cache
            // should be empty.

            assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

            // Check that the cache gets populated as we retrieve references to pages.

            let document_0_page_0 = document_0.pages().get(0)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);

            let document_0_page_1 = document_0.pages().get(1)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 2);

            let document_0_page_2 = document_0.pages().get(2)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);

            // Check the cached indices are correct.

            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_0.page_handle())
                    .is_some()
            );
            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_0.page_handle())
                    .unwrap()
                    .index
                    == 0
            );

            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_1.page_handle())
                    .is_some()
            );
            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_1.page_handle())
                    .unwrap()
                    .index
                    == 1
            );

            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_2.page_handle())
                    .is_some()
            );
            assert!(
                PdfPageIndexCache::lock()
                    .get(document_0.handle(), document_0_page_2.page_handle())
                    .unwrap()
                    .index
                    == 2
            );

            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document_0.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document_0.handle())
                    .copied()
                    .unwrap(),
                2
            );

            // Now, while we still have references to those pages, let's create a second document
            // and make sure that references to the second document are also stored correctly.

            let mut document_1 = pdfium.create_new_pdf()?;

            {
                // Create four blank pages.

                for _ in 1..=4 {
                    document_1.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
                }

                // Since we haven't retrieved any references to these pages, the index cache
                // should only contain the references to the pages from the first document.

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);

                // Check that the cache gets populated as we retrieve references to pages.

                let document_1_page_0 = document_1.pages().get(0)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 4);

                let document_1_page_1 = document_1.pages().get(1)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 5);

                let document_1_page_2 = document_1.pages().get(2)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 6);

                let document_1_page_3 = document_1.pages().get(3)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 7);

                // Check the cached indices are correct.

                assert!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_0.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_0.page_handle())
                        .unwrap()
                        .index,
                    0
                );

                assert!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_1.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_1.page_handle())
                        .unwrap()
                        .index,
                    1
                );

                assert!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_2.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_2.page_handle())
                        .unwrap()
                        .index,
                    2
                );

                assert!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_3.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document_1.handle(), document_1_page_3.page_handle())
                        .unwrap()
                        .index,
                    3
                );

                assert!(
                    PdfPageIndexCache::lock()
                        .documents_by_maximum_index
                        .contains_key(&document_1.handle())
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .documents_by_maximum_index
                        .get(&document_1.handle())
                        .copied()
                        .unwrap(),
                    3
                );
            }

            // At this point, the pages from document_1 have been dropped. Those pages should
            // have been removed from the cache.

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);
        }

        // At this point, the pages from document_0 have been dropped. Those pages should
        // have been removed from the cache; the cache should now be empty.

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        Ok(())
    }

    #[test]
    fn test_get_invalid_page() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let page_handle = {
            // Create a new page...

            let page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            // ... confirm the index of the page is cached...

            assert!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), page.page_handle())
                    .is_some()
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), page.page_handle())
                    .unwrap()
                    .index,
                0
            );

            // ... and return the handle of the page.

            page.page_handle()
        };

        // At this point, the page itself has been dropped, so the page handle is no longer valid.
        // Attempting to retrieve the cached index for the page should return None.

        assert!(PdfPageIndexCache::lock().get(document.handle(), page_handle).is_none());

        Ok(())
    }

    #[test]
    fn test_insert_pages_at_index() -> Result<(), PdfiumError> {
        // Create a document with 100 pages, caching the index position of each page.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        // To cache the index position of each page, we have to hold a reference to each page.
        // We use a Vec to do this. Create the Vec inside a sub-scope, to ensure its lifetime
        // is shorter than document and pdfium.

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?);
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                99
            );

            for (index, page) in pages.iter().enumerate() {
                assert!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .unwrap()
                        .index,
                    index as PdfPageIndex
                );
            }

            // Our cache now holds 100 index positions. Insert a new page at the start of the document...

            let inserted = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 101);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                100
            );

            assert!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), inserted.page_handle())
                    .is_some()
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), inserted.page_handle())
                    .unwrap()
                    .index,
                0
            );

            // ... and check that the index positions for all other pages have correctly shuffled down.

            for (index, page) in pages.iter().enumerate() {
                assert!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .unwrap()
                        .index,
                    index as PdfPageIndex + 1
                );
            }

            // Our cache now holds 101 index positions. Insert a new page at position 50...

            let inserted = document.pages_mut().create_page_at_index(PdfPagePaperSize::a4(), 50)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 102);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                101
            );

            assert!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), inserted.page_handle())
                    .is_some()
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .get(document.handle(), inserted.page_handle())
                    .unwrap()
                    .index,
                50
            );

            // ... and check that the index positions for pages before position 50 _haven't_ changed,
            // while the index positions for pages _after_ position 50 _have_ shuffled down.

            for (index, page) in pages.iter().enumerate() {
                // We compare against an index position of 49 rather than 50 because we've already
                // inserted one page at the beginning of the document. This insertion at index position
                // 50 is our _second_ insertion into the page sequence.

                if index < 49 {
                    assert!(
                        PdfPageIndexCache::lock()
                            .get(document.handle(), page.page_handle())
                            .is_some()
                    );
                    assert_eq!(
                        PdfPageIndexCache::lock()
                            .get(document.handle(), page.page_handle())
                            .unwrap()
                            .index,
                        index as PdfPageIndex + 1
                    );
                }

                if index > 49 {
                    assert!(
                        PdfPageIndexCache::lock()
                            .get(document.handle(), page.page_handle())
                            .is_some()
                    );
                    assert_eq!(
                        PdfPageIndexCache::lock()
                            .get(document.handle(), page.page_handle())
                            .unwrap()
                            .index,
                        index as PdfPageIndex + 2
                    );
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_delete_pages_at_index() -> Result<(), PdfiumError> {
        // Create a document with 100 pages, caching the index position of each page.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        // To cache the index position of each page, we have to hold a reference to each page.
        // We use a Vec to do this. Create the Vec inside a sub-scope, to ensure its lifetime
        // is shorter than document and pdfium.

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(Some(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?));
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                99
            );

            for (index, page) in pages.iter().enumerate() {
                assert!(page.is_some());

                let document = document.handle();
                let page = page.as_ref().unwrap().page_handle();

                assert!(PdfPageIndexCache::lock().get(document, page).is_some());
                assert_eq!(
                    PdfPageIndexCache::lock().get(document, page).unwrap().index,
                    index as PdfPageIndex
                );
            }

            // Our cache now holds 100 index positions. Delete the page at the start of the document...

            pages.first_mut().unwrap().take().unwrap().delete()?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 99);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                98
            );

            // ... and check that the index positions for all other pages have correctly shuffled up.

            for (index, page) in pages.iter().enumerate() {
                if index == 0 {
                    // This page no longer exists.

                    assert!(page.is_none());
                } else {
                    assert!(page.is_some());

                    let document = document.handle();
                    let page = page.as_ref().unwrap().page_handle();

                    assert!(PdfPageIndexCache::lock().get(document, page).is_some());
                    assert_eq!(
                        PdfPageIndexCache::lock().get(document, page).unwrap().index,
                        index as PdfPageIndex - 1
                    );
                }
            }

            // Our cache now holds 99 index positions. Delete the page at index position 50...

            pages.get_mut(50).unwrap().take().unwrap().delete()?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 98);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                97
            );

            // ... and check that the index positions for pages before position 50 _haven't_ changed,
            // while the index positions for pages _after_ position 50 _have_ shuffled up.

            for (index, page) in pages.iter().enumerate() {
                if index == 0 || index == 50 {
                    // This page no longer exists.

                    assert!(page.is_none());
                } else if index < 50 {
                    assert!(page.is_some());

                    let document = document.handle();
                    let page = page.as_ref().unwrap().page_handle();

                    assert!(PdfPageIndexCache::lock().get(document, page).is_some());
                    assert_eq!(
                        PdfPageIndexCache::lock().get(document, page).unwrap().index,
                        index as PdfPageIndex - 1
                    );
                } else if index > 50 {
                    assert!(page.is_some());

                    let document = document.handle();
                    let page = page.as_ref().unwrap().page_handle();

                    assert!(PdfPageIndexCache::lock().get(document, page).is_some());
                    assert_eq!(
                        PdfPageIndexCache::lock().get(document, page).unwrap().index,
                        index as PdfPageIndex - 2
                    );
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_pathological_delete_all_pages() -> Result<(), PdfiumError> {
        // Create a document with 100 pages, caching the index position of each page,
        // then delete all one hundred pages, testing the cached maximum page index
        // after each deletion.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        // To cache the index position of each page, we have to hold a reference to each page.
        // We use a Vec to do this. Create the Vec inside a sub-scope, to ensure its lifetime
        // is shorter than document and pdfium.

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?);
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
            assert_eq!(
                PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .get(&document.handle())
                    .copied()
                    .unwrap(),
                99
            );

            for (index, page) in pages.iter().enumerate() {
                assert!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .is_some()
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .get(document.handle(), page.page_handle())
                        .unwrap()
                        .index,
                    index as PdfPageIndex
                );
            }

            // Our cache now holds 100 index positions. Delete all 100 pages.

            for index in (0..100).rev() {
                assert!(
                    PdfPageIndexCache::lock()
                        .documents_by_maximum_index
                        .contains_key(&document.handle())
                );
                assert_eq!(
                    PdfPageIndexCache::lock()
                        .documents_by_maximum_index
                        .get(&document.handle())
                        .copied()
                        .unwrap(),
                    index
                );

                PdfPageIndexCache::lock().delete(document.handle(), index, 1);

                if index > 0 {
                    assert!(
                        PdfPageIndexCache::lock()
                            .documents_by_maximum_index
                            .contains_key(&document.handle())
                    );
                    assert_eq!(
                        PdfPageIndexCache::lock()
                            .documents_by_maximum_index
                            .get(&document.handle())
                            .copied()
                            .unwrap(),
                        index - 1
                    );
                }
            }

            // All pages are now deleted.

            assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());
            assert!(
                !PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
        }

        Ok(())
    }
}
