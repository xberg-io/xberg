use crate::types::ProcessingWarning;

#[derive(Clone, Debug)]
pub(crate) struct LayoutPageSelection {
    pages: std::collections::BTreeSet<u32>,
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub(crate) region_text: Option<crate::engine::layout_region_text::RegionTextCollector>,
}

#[cfg_attr(not(all(feature = "pdf", feature = "layout-detection")), allow(dead_code))]
impl LayoutPageSelection {
    pub(crate) fn new(pages: Vec<u32>) -> Self {
        Self {
            pages: pages.into_iter().collect(),
            #[cfg(all(feature = "pdf", feature = "layout-detection"))]
            region_text: None,
        }
    }

    pub(crate) fn contains_index(&self, index: usize) -> bool {
        u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(1))
            .is_some_and(|page| self.pages.contains(&page))
    }

    pub(crate) fn warnings(&self, page_count: usize) -> Vec<ProcessingWarning> {
        self.pages
            .iter()
            .copied()
            .filter(|page| *page == 0 || (*page as usize) > page_count)
            .map(|page| ProcessingWarning {
                source: std::borrow::Cow::Borrowed("layout-pages"),
                message: std::borrow::Cow::Owned(format!(
                    "layout page {page} is outside the 1-based range 1..={page_count}; ignored"
                )),
            })
            .collect()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub(crate) trait LayoutPagesExtractor: Send + Sync {
    async fn extract_with_layout_pages(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &crate::core::config::ExtractionConfig,
        pages: &LayoutPageSelection,
    ) -> crate::Result<crate::types::internal::InternalDocument>;
}

#[cfg(test)]
mod tests {
    use super::LayoutPageSelection;

    #[test]
    fn should_normalize_duplicate_one_based_layout_pages() {
        let pages = LayoutPageSelection::new(vec![3, 1, 3, 0, 4]);
        let selected: Vec<_> = (0..3).filter(|index| pages.contains_index(*index)).collect();
        assert_eq!(selected, vec![0, 2]);
        let warnings = pages.warnings(3);
        assert_eq!(warnings.len(), 2);
        assert_eq!(
            warnings[0].message,
            "layout page 0 is outside the 1-based range 1..=3; ignored"
        );
        assert_eq!(
            warnings[1].message,
            "layout page 4 is outside the 1-based range 1..=3; ignored"
        );
    }

    #[test]
    fn should_skip_every_page_when_explicit_layout_selection_is_empty() {
        let pages = LayoutPageSelection::new(Vec::new());
        assert_eq!((0..3).filter(|index| pages.contains_index(*index)).count(), 0);
        assert_eq!(pages.warnings(3).len(), 0);
    }

    #[test]
    fn should_reject_indices_that_cannot_represent_a_one_based_page() {
        let pages = LayoutPageSelection::new(vec![u32::MAX]);
        assert!(!pages.contains_index(usize::MAX));
        assert!(!pages.contains_index(u32::MAX as usize));
    }
}
