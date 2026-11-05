use super::{EncodedPage, PageRenderFailures, RenderedPages};

pub(crate) type PageRasterDpis = ahash::AHashMap<usize, crate::pdf::render::OcrRasterDpi>;
type PageImage = (usize, image::DynamicImage);
pub(super) type PageWithDpi = (PageImage, crate::pdf::render::OcrRasterDpi);
type PageRenderOutcome = (usize, crate::Result<PageWithDpi>);

pub(super) fn partition_raster_dpis(
    rendered: Vec<(EncodedPage, crate::pdf::render::OcrRasterDpi)>,
) -> (Vec<EncodedPage>, PageRasterDpis) {
    let mut pages = Vec::with_capacity(rendered.len());
    let mut dpis = PageRasterDpis::with_capacity(rendered.len());
    for (page, dpi) in rendered {
        dpis.insert(page.0, dpi);
        pages.push(page);
    }
    (pages, dpis)
}
pub(super) fn partition_page_render_outcomes(
    outcomes: Vec<PageRenderOutcome>,
) -> (RenderedPages, PageRenderFailures, PageRasterDpis) {
    let mut pages = Vec::with_capacity(outcomes.len());
    let mut failures = Vec::new();
    let mut dpis = PageRasterDpis::with_capacity(outcomes.len());
    for (page_index, outcome) in outcomes {
        match outcome {
            Ok((page, dpi)) => {
                dpis.insert(page_index, dpi);
                pages.push(page);
            }
            Err(error) => failures.push((page_index, error)),
        }
    }
    (pages, failures, dpis)
}
