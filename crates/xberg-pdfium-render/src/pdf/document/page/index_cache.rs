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

        let Some(removed_index) = props.as_ref().map(|props| props.index) else {
            return props;
        };

        self.indices_by_page.remove(&(document, removed_index));

        if self.documents_by_maximum_index.get(&document).copied() == Some(removed_index) {
            self.recompute_maximum_index(document);
        }

        props
    }

    /// Recomputes and caches the highest remaining [PdfPageIndex] for `document` after its
    /// previously-cached maximum index was removed.
    #[inline]
    fn recompute_maximum_index(&mut self, document: FPDF_DOCUMENT) {
        let maximum = self
            .indices_by_page
            .keys()
            .filter(|(doc, _)| *doc == document)
            .map(|(_, index)| *index)
            .max();

        match maximum {
            Some(maximum) => {
                self.documents_by_maximum_index.insert(document, maximum);
            }
            None => {
                self.documents_by_maximum_index.remove(&document);
            }
        }
    }

    /// Removes and re-caches the page at `old_index` in `document` under `new_index`, preserving
    /// its content regeneration strategy. No-op if no page is cached at `old_index`.
    #[inline]
    fn reindex_page_if_present(&mut self, document: FPDF_DOCUMENT, old_index: PdfPageIndex, new_index: PdfPageIndex) {
        let Some(page) = self.indices_by_page.get(&(document, old_index)).copied() else {
            return;
        };

        let content_regeneration_strategy = self
            .remove(document, page)
            .map(|props| props.content_regeneration_strategy)
            .unwrap_or(PdfPageContentRegenerationStrategy::AutomaticOnEveryChange);

        self.set(
            document,
            page,
            PdfPageCachedProperties {
                index: new_index,
                content_regeneration_strategy,
            },
        );
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// an insertion of the given number of pages at the given index position.
    #[inline]
    fn insert(&mut self, document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        match self.documents_by_maximum_index.get(&document).copied() {
            Some(maximum_index_for_document) => {
                if maximum_index_for_document > index {
                    for index in (index..=maximum_index_for_document).rev() {
                        self.reindex_page_if_present(document, index, index + count);
                    }
                }

                self.documents_by_maximum_index
                    .insert(document, maximum_index_for_document + count);
            }
            None => {
                self.documents_by_maximum_index.insert(document, index + count - 1);
            }
        }
    }

    /// Adjusts all cached [PdfPageIndex] values for the given document as necessary to accommodate
    /// a deletion of the given number of pages at the given index position.
    #[inline]
    fn delete(&mut self, document: FPDF_DOCUMENT, index: PdfPageIndex, count: PdfPageIndex) {
        let mut maximum_index_for_document = self.documents_by_maximum_index.get(&document).copied().unwrap_or(0);

        for index in index..index + count {
            if let Some(page) = self.indices_by_page.get(&(document, index)).copied() {
                self.remove(document, page);
            }
        }

        if maximum_index_for_document > index {
            for index in index + 1..=maximum_index_for_document {
                self.reindex_page_if_present(document, index, index - count);
            }
        } else {
            maximum_index_for_document = index;
        }

        if maximum_index_for_document >= count {
            self.documents_by_maximum_index
                .insert(document, maximum_index_for_document - count);
        } else {
            self.documents_by_maximum_index.remove(&document);
        }
    }

    #[inline]
    fn lock() -> MutexGuard<'static, PdfPageIndexCache> {
        PAGE_INDEX_CACHE.lock().unwrap()
    }

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

        let page_handles: Vec<FPDF_PAGE> = cache
            .pages_by_index
            .keys()
            .filter(|(doc, _)| *doc == document)
            .map(|(_, page)| *page)
            .collect();

        for page_handle in page_handles {
            cache.remove(document, page_handle);
        }

        cache.documents_by_maximum_index.remove(&document);
    }
}

unsafe impl Send for PdfPageIndexCache {}

unsafe impl Sync for PdfPageIndexCache {}

#[cfg(test)]
mod tests {
    use super::{FPDF_DOCUMENT, FPDF_PAGE};
    use crate::pdf::document::page::index_cache::PdfPageIndexCache;
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    /// Asserts that `document`/`page` currently has a cached index equal to `expected_index`.
    fn assert_cached_index(document: FPDF_DOCUMENT, page: FPDF_PAGE, expected_index: PdfPageIndex) {
        let cache = PdfPageIndexCache::lock();
        let props = cache.get(document, page);

        assert!(props.is_some());
        assert_eq!(props.unwrap().index, expected_index);
    }

    /// Asserts that `document` currently has a cached maximum index equal to `expected_maximum`.
    fn assert_maximum_index(document: FPDF_DOCUMENT, expected_maximum: PdfPageIndex) {
        let cache = PdfPageIndexCache::lock();

        assert!(cache.documents_by_maximum_index.contains_key(&document));
        assert_eq!(
            cache.documents_by_maximum_index.get(&document).copied().unwrap(),
            expected_maximum
        );
    }

    #[test]
    fn test_cache_instantiation() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        {
            let _page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);
        }

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        let _page = document.pages().first();

        assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_and_set_index_for_page() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document_0 = pdfium.create_new_pdf()?;

        {
            for _ in 1..=3 {
                document_0.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
            }

            assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

            let document_0_page_0 = document_0.pages().get(0)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 1);

            let document_0_page_1 = document_0.pages().get(1)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 2);

            let document_0_page_2 = document_0.pages().get(2)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);

            assert_cached_index(document_0.handle(), document_0_page_0.page_handle(), 0);
            assert_cached_index(document_0.handle(), document_0_page_1.page_handle(), 1);
            assert_cached_index(document_0.handle(), document_0_page_2.page_handle(), 2);

            assert_maximum_index(document_0.handle(), 2);

            let mut document_1 = pdfium.create_new_pdf()?;

            {
                for _ in 1..=4 {
                    document_1.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
                }

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);

                let document_1_page_0 = document_1.pages().get(0)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 4);

                let document_1_page_1 = document_1.pages().get(1)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 5);

                let document_1_page_2 = document_1.pages().get(2)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 6);

                let document_1_page_3 = document_1.pages().get(3)?;

                assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 7);

                assert_cached_index(document_1.handle(), document_1_page_0.page_handle(), 0);
                assert_cached_index(document_1.handle(), document_1_page_1.page_handle(), 1);
                assert_cached_index(document_1.handle(), document_1_page_2.page_handle(), 2);
                assert_cached_index(document_1.handle(), document_1_page_3.page_handle(), 3);

                assert_maximum_index(document_1.handle(), 3);
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 3);
        }

        assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());

        Ok(())
    }

    #[test]
    fn test_get_invalid_page() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let page_handle = {
            let page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            assert_cached_index(document.handle(), page.page_handle(), 0);

            page.page_handle()
        };

        assert!(PdfPageIndexCache::lock().get(document.handle(), page_handle).is_none());

        Ok(())
    }

    #[test]
    fn test_insert_pages_at_index() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?);
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert_maximum_index(document.handle(), 99);

            for (index, page) in pages.iter().enumerate() {
                assert_cached_index(document.handle(), page.page_handle(), index as PdfPageIndex);
            }

            let inserted = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 101);
            assert_maximum_index(document.handle(), 100);
            assert_cached_index(document.handle(), inserted.page_handle(), 0);

            for (index, page) in pages.iter().enumerate() {
                assert_cached_index(document.handle(), page.page_handle(), index as PdfPageIndex + 1);
            }

            let inserted = document.pages_mut().create_page_at_index(PdfPagePaperSize::a4(), 50)?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 102);
            assert_maximum_index(document.handle(), 101);
            assert_cached_index(document.handle(), inserted.page_handle(), 50);

            for (index, page) in pages.iter().enumerate() {
                if index < 49 {
                    assert_cached_index(document.handle(), page.page_handle(), index as PdfPageIndex + 1);
                }

                if index > 49 {
                    assert_cached_index(document.handle(), page.page_handle(), index as PdfPageIndex + 2);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_delete_pages_at_index() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(Some(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?));
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert_maximum_index(document.handle(), 99);

            for (index, page) in pages.iter().enumerate() {
                assert!(page.is_some());

                assert_cached_index(
                    document.handle(),
                    page.as_ref().unwrap().page_handle(),
                    index as PdfPageIndex,
                );
            }

            pages.first_mut().unwrap().take().unwrap().delete()?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 99);
            assert_maximum_index(document.handle(), 98);

            for (index, page) in pages.iter().enumerate() {
                if index == 0 {
                    assert!(page.is_none());
                } else {
                    assert!(page.is_some());

                    assert_cached_index(
                        document.handle(),
                        page.as_ref().unwrap().page_handle(),
                        index as PdfPageIndex - 1,
                    );
                }
            }

            pages.get_mut(50).unwrap().take().unwrap().delete()?;

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 98);
            assert_maximum_index(document.handle(), 97);

            for (index, page) in pages.iter().enumerate() {
                if index == 0 || index == 50 {
                    assert!(page.is_none());
                } else if index < 50 {
                    assert!(page.is_some());

                    assert_cached_index(
                        document.handle(),
                        page.as_ref().unwrap().page_handle(),
                        index as PdfPageIndex - 1,
                    );
                } else if index > 50 {
                    assert!(page.is_some());

                    assert_cached_index(
                        document.handle(),
                        page.as_ref().unwrap().page_handle(),
                        index as PdfPageIndex - 2,
                    );
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_pathological_delete_all_pages() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        {
            let mut pages = Vec::new();

            for _ in 1..=100 {
                pages.push(document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?);
            }

            assert_eq!(PdfPageIndexCache::lock().pages_by_index.len(), 100);
            assert_maximum_index(document.handle(), 99);

            for (index, page) in pages.iter().enumerate() {
                assert_cached_index(document.handle(), page.page_handle(), index as PdfPageIndex);
            }

            for index in (0..100).rev() {
                assert_maximum_index(document.handle(), index);

                PdfPageIndexCache::lock().delete(document.handle(), index, 1);

                if index > 0 {
                    assert_maximum_index(document.handle(), index - 1);
                }
            }

            assert!(PdfPageIndexCache::lock().pages_by_index.is_empty());
            assert!(
                !PdfPageIndexCache::lock()
                    .documents_by_maximum_index
                    .contains_key(&document.handle())
            );
        }

        Ok(())
    }

    #[test]
    fn removing_a_documents_last_page_clears_its_maximum_index() {
        use crate::pdf::document::page::PdfPageContentRegenerationStrategy;
        use crate::pdf::document::page::index_cache::PdfPageCachedProperties;

        let document_a = 0xD000_0000usize as FPDF_DOCUMENT;
        let document_b = 0xE000_0000usize as FPDF_DOCUMENT;
        let a_page = 0xD000_0001usize as FPDF_PAGE;
        let b_page = 0xE000_0001usize as FPDF_PAGE;

        // A private cache instance keeps this test independent of the shared global cache.
        let mut cache = PdfPageIndexCache::new();

        for (document, page) in [(document_a, a_page), (document_b, b_page)] {
            cache.set(
                document,
                page,
                PdfPageCachedProperties {
                    index: 0,
                    content_regeneration_strategy: PdfPageContentRegenerationStrategy::AutomaticOnEveryChange,
                },
            );
        }

        cache.remove(document_a, a_page);

        assert_eq!(cache.documents_by_maximum_index.get(&document_a), None);
        assert_eq!(cache.documents_by_maximum_index.get(&document_b).copied(), Some(0));
    }
}
