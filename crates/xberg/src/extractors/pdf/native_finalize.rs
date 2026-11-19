//! Stages that finish the `InternalDocument` built by `PdfExtractor::extract_core_native_with_layout_pages`.

use crate::types::PageContent;
use crate::types::internal::{InternalDocument, PdfPageCoordinateFrame};

pub(super) fn push_link_annotation_uris(doc: &mut InternalDocument) {
    use crate::types::annotations::PdfAnnotationType;
    use crate::types::uri::{ExtractedUri, UriKind};

    let uris: Vec<ExtractedUri> = doc
        .annotations
        .as_ref()
        .map(|annotations| {
            annotations
                .iter()
                .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                .filter_map(|a| {
                    a.content.as_ref().map(|url| {
                        let kind = if url.starts_with('#') {
                            UriKind::Anchor
                        } else if url.starts_with("mailto:") {
                            UriKind::Email
                        } else {
                            UriKind::Hyperlink
                        };
                        ExtractedUri {
                            url: url.clone(),
                            label: Some(url.clone()),
                            page: Some(a.page_number),
                            kind,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    for uri in uris {
        doc.push_uri(uri);
    }
}

// GH#1653 + GH#1654: one raw-MediaBox coordinate frame per page that ended up with
// hierarchy blocks, once `assign_hierarchy_to_pages` has populated `PageContent::hierarchy`.
// One shared record covers both issues: a consumer needs the MediaBox origin (which can be
// non-zero and negative) and the page rotation together to place a page's raw-space geometry.
// Rides in `additional` rather than a new public binding type, same as the `page_labels` key
// (issue #66) and `ocr_page_coordinate_frames` (GH#1645). ~keep
pub(super) fn attach_page_coordinate_frames(
    doc: &mut InternalDocument,
    pages: &[PageContent],
    pdf_page_coordinate_frames: Vec<PdfPageCoordinateFrame>,
) {
    let pages_with_hierarchy: std::collections::HashSet<u32> = pages
        .iter()
        .filter(|page| {
            page.hierarchy
                .as_ref()
                .is_some_and(|hierarchy| !hierarchy.blocks.is_empty())
        })
        .map(|page| page.page_number)
        .collect();
    let pdf_page_coordinate_frames: Vec<_> = pdf_page_coordinate_frames
        .into_iter()
        .filter(|frame| pages_with_hierarchy.contains(&frame.page_number))
        .collect();
    if !pdf_page_coordinate_frames.is_empty() {
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("pdf_page_coordinate_frames"),
            serde_json::json!(pdf_page_coordinate_frames),
        );
    }
}
