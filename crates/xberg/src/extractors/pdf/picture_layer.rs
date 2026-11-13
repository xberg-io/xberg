//! The ink a text layer drawn over a full-page picture leaves, in the page and on blank paper (#2826).

use crate::core::config::ExtractionConfig;

/// Resolution of the renders that look for the layer's ink.
const PICTURE_LAYER_DPI: u32 = 150;

/// The layer's glyphs ink a pixel of the page render when they change it by more than this many
/// grey levels.
const MIN_GLYPH_CONTRAST: u8 = 40;

/// Luminance under which a pixel of the render without pictures is ink.
const MAX_INK_LUMINANCE: u8 = 127;

/// Ink pixels per glyph, at [`PICTURE_LAYER_DPI`], that a layer a reader sees leaves in each render.
/// A layer printed on a picture leaves several times this; a layer drawn invisibly by render mode,
/// opacity 0, an empty clip, a fill the colour of the picture or under the picture leaves none in
/// one of the two. ~keep
const MIN_INK_PER_GLYPH: f64 = 5.0;

/// Whether the text layer of a page with a full-page picture is drawn over it where a reader sees
/// it: the glyphs drawn visibly after the picture (see
/// [`crate::pdf::scan_detect::glyph_share_over_full_page_picture`]) leave [`MIN_INK_PER_GLYPH`]
/// both where they change the page render and in a render without pictures.
/// A render that fails or is cancelled sees nothing.
pub(super) fn visible_text_over_picture(
    document: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    config: &ExtractionConfig,
) -> Option<String> {
    let share = crate::pdf::scan_detect::glyph_share_over_full_page_picture(document, page_index)?;
    let painting = document
        .extract_text_without_nonpainting_glyphs(page_index)
        .inspect_err(|error| {
            tracing::debug!(%error, page_index, "could not extract the native overlay separately");
        })
        .ok()?;
    let glyphs = painting.chars().filter(|c| !c.is_whitespace()).count() as f64 * share;
    if glyphs <= 0.0 {
        return None;
    }
    let budget = super::ocr::preview_byte_limit(config);
    let cancelled = || config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled());
    let Ok(Some(rasters)) = xberg_native_pdf::rendering::render_text_ink_rasters(
        document,
        page_index,
        PICTURE_LAYER_DPI,
        budget,
        cancelled,
    ) else {
        return None;
    };
    let glyph_ink = rasters
        .drawn
        .iter()
        .zip(&rasters.without_glyphs)
        .filter(|(drawn, without)| drawn.abs_diff(**without) > MIN_GLYPH_CONTRAST)
        .count();
    if (glyph_ink as f64) < MIN_INK_PER_GLYPH * glyphs || cancelled() {
        return None;
    }
    let Ok((_, _, without_pictures)) =
        xberg_native_pdf::rendering::render_page_without_images(document, page_index, PICTURE_LAYER_DPI, budget)
    else {
        return None;
    };
    let ink = without_pictures
        .iter()
        .filter(|luminance| **luminance <= MAX_INK_LUMINANCE)
        .count();
    (ink as f64 >= MIN_INK_PER_GLYPH * glyphs).then_some(painting)
}

/// The page rendered without its pictures, as an image to read: the text and vector art on blank
/// paper. `None` when the render fails.
pub(super) fn page_without_pictures(
    document: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    config: &ExtractionConfig,
) -> Option<image::DynamicImage> {
    let (width, height, luminance) = xberg_native_pdf::rendering::render_page_without_images(
        document,
        page_index,
        PICTURE_LAYER_DPI,
        super::ocr::preview_byte_limit(config),
    )
    .ok()?;
    image::GrayImage::from_raw(width, height, luminance).map(image::DynamicImage::ImageLuma8)
}

pub(super) struct RetainedPicturePage {
    pub(super) visible: String,
    pub(super) raster: Option<String>,
    words: Vec<xberg_native_pdf::layout::Word>,
}

impl RetainedPicturePage {
    pub(super) fn transcript(&self) -> String {
        self.raster.as_ref().map_or_else(
            || self.visible.clone(),
            |raster| format!("{}\n\n{raster}", self.visible),
        )
    }
}

/// Read the picture on pages the script gate kept for their visible native text. ~keep
pub(super) async fn read_retained_picture_pages(
    content: &[u8],
    text: &str,
    boundaries: &[crate::types::PageBoundary],
    candidates: &[u32],
    replacements: Option<&ahash::AHashMap<u32, String>>,
    config: &ExtractionConfig,
) -> crate::Result<ahash::AHashMap<u32, RetainedPicturePage>> {
    let mut additions = ahash::AHashMap::new();
    let mut eligible = candidates
        .iter()
        .copied()
        .filter(|&page| {
            super::ocr::native_preservation_is_automatic(config, page)
                && replacements.is_none_or(|replaced| !replaced.contains_key(&page))
        })
        .peekable();
    if eligible.peek().is_none() {
        return Ok(additions);
    }
    let document = match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(document) => document,
        Err(error) => {
            tracing::debug!(%error, "could not open PDF for picture-layer OCR");
            return Ok(additions);
        }
    };
    for page in eligible {
        super::ensure_pdf_not_cancelled(config)?;
        let Some(layer) = super::unreadable_script::page_text(text, boundaries, page) else {
            continue;
        };
        let Some(index) = page.checked_sub(1).map(|index| index as usize) else {
            continue;
        };
        if visible_text_over_picture(&document, index, config).is_none() {
            continue;
        }
        let Ok((mut visible, spans)) = document.extract_painting_text_and_spans(index).inspect_err(|error| {
            tracing::debug!(%error, index, "could not recover positioned native overlay");
        }) else {
            continue;
        };
        let words = painting_words(&document, index, spans).unwrap_or_else(|error| {
            tracing::debug!(%error, index, "could not assemble positioned painting words");
            Vec::new()
        });
        super::unmapped_glyphs::strip_native_fragment(&mut visible);
        let visible = visible.trim();
        if visible.is_empty() {
            continue;
        }
        let raster = read_picture(&document, index, config).await?;
        if raster.is_some() || !layer.split_whitespace().eq(visible.split_whitespace()) {
            additions.insert(
                page,
                RetainedPicturePage {
                    visible: visible.to_owned(),
                    raster,
                    words,
                },
            );
        }
    }
    Ok(additions)
}

fn painting_words(
    document: &xberg_native_pdf::PdfDocument,
    index: usize,
    spans: Vec<xberg_native_pdf::layout::TextSpan>,
) -> xberg_native_pdf::Result<Vec<xberg_native_pdf::layout::Word>> {
    let (x0, y0, x1, y1) = document.get_page_media_box(index)?;
    let ordered = xberg_native_pdf::pipeline::order_page_spans(document, index, spans)?;
    let spans: Vec<_> = ordered.into_iter().map(|ordered| ordered.span).collect();
    xberg_native_pdf::PdfDocument::assemble_positioned_words(
        &spans,
        xberg_native_pdf::geometry::Rect::new(x0, y0, x1, y1),
        None,
    )
    .map(|(words, _, _)| words)
}

/// Read the picture with its native glyphs suppressed. ~keep
async fn read_picture(
    document: &xberg_native_pdf::PdfDocument,
    index: usize,
    config: &ExtractionConfig,
) -> crate::Result<Option<String>> {
    let budget = super::ocr::preview_byte_limit(config);
    let raster = xberg_native_pdf::rendering::render_page_without_glyphs(document, index, PICTURE_LAYER_DPI, budget);
    let (width, height, luminance) = match raster {
        Ok(raster) => raster,
        Err(error) => {
            tracing::debug!(%error, index, "could not render the picture without its native glyphs");
            return Ok(None);
        }
    };
    let Some(image) = image::GrayImage::from_raw(width, height, luminance) else {
        return Ok(None);
    };
    super::ensure_pdf_not_cancelled(config)?;
    let read = super::ocr::read_rendered_page(image::DynamicImage::ImageLuma8(image), config).await;
    super::ensure_pdf_not_cancelled(config)?;
    let (read, confidence) = match read {
        Ok(read) => read,
        Err(error) => {
            tracing::debug!(%error, index, "could not read the picture beneath the native overlay");
            return Ok(None);
        }
    };
    let thresholds = config
        .ocr
        .as_ref()
        .map(|ocr| ocr.effective_thresholds())
        .unwrap_or_default();
    if read.trim().is_empty()
        || !confidence.is_some_and(|confidence| {
            confidence.word_count as usize >= thresholds.min_meaningful_words
                && confidence
                    .score
                    .is_some_and(|score| score >= thresholds.min_ocr_mean_confidence / 100.0)
        })
    {
        return Ok(None);
    }
    Ok(Some(read.trim().to_owned()))
}

pub(super) fn unapplied_replacements(
    document: Option<&mut crate::types::internal::InternalDocument>,
    pages: &ahash::AHashMap<u32, RetainedPicturePage>,
    replacements: Option<&ahash::AHashMap<u32, String>>,
) -> Option<ahash::AHashMap<u32, String>> {
    let applied = document.map_or_else(ahash::AHashSet::new, |document| {
        merge_retained_picture_pages(document, pages)
    });
    replacements.map(|replacements| {
        replacements
            .iter()
            .filter(|(page, _)| !applied.contains(*page))
            .map(|(&page, text)| (page, text.clone()))
            .collect()
    })
}

/// Keep the native nodes in place and append the raster transcript before the page ends. ~keep
pub(super) fn merge_retained_picture_pages(
    document: &mut crate::types::internal::InternalDocument,
    pages: &ahash::AHashMap<u32, RetainedPicturePage>,
) -> ahash::AHashSet<u32> {
    use crate::types::internal::{ElementKind, InternalElement};

    let mut last = ahash::AHashMap::new();
    let mut previous_page = None;
    for (index, element) in document.elements.iter().enumerate() {
        if element.kind == ElementKind::PageBreak {
            previous_page = None;
            continue;
        }
        if element.page.is_some() {
            previous_page = element.page;
        }
        let page = element
            .page
            .or_else(|| element.kind.is_container_end().then_some(previous_page).flatten());
        if let Some(page) = page.filter(|page| pages.contains_key(page)) {
            last.insert(page, index);
        }
    }
    let mut edits = ahash::AHashMap::new();
    let mut retained = ahash::AHashMap::<u32, String>::new();
    let mut ambiguous_pages = ahash::AHashSet::new();
    for (index, element) in document.elements.iter().enumerate() {
        let Some(page) = element.page.filter(|page| last.contains_key(page)) else {
            continue;
        };
        if element.text.trim().is_empty()
            || matches!(element.kind, ElementKind::Table { .. } | ElementKind::Image { .. })
        {
            continue;
        }
        let mut candidate = element.clone();
        match retain_painting_text(&mut candidate, &pages[&page].words) {
            PaintingMatch::Retained => {
                retained.entry(page).or_default().push_str(&candidate.text);
                edits.insert(index, Some(candidate));
            }
            PaintingMatch::Absent => {
                edits.insert(index, None);
            }
            PaintingMatch::Ambiguous => {
                ambiguous_pages.insert(page);
            }
        }
    }
    let applied: ahash::AHashSet<_> = retained
        .into_iter()
        .filter(|(page, _)| !ambiguous_pages.contains(page))
        .filter_map(|(page, text)| {
            text.chars()
                .filter(|ch| !ch.is_whitespace())
                .eq(pages[&page].visible.chars().filter(|ch| !ch.is_whitespace()))
                .then_some(page)
        })
        .collect();
    let insertions: ahash::AHashMap<_, _> = last
        .into_iter()
        .filter(|(page, _)| applied.contains(page))
        .map(|(page, index)| (index, page))
        .collect();
    let old = std::mem::take(&mut document.elements);
    let mut remap = vec![None; old.len()];
    for (index, element) in old.into_iter().enumerate() {
        let edit = element
            .page
            .filter(|page| applied.contains(page))
            .and_then(|_| edits.remove(&index));
        if let Some(element) = edit.unwrap_or(Some(element)) {
            remap[index] = Some(document.elements.len() as u32);
            document.elements.push(element);
        }
        if let Some(&page) = insertions.get(&index)
            && let Some(raster) = &pages[&page].raster
        {
            document.push_element(InternalElement::text(ElementKind::Paragraph, raster.as_str(), 0).with_page(page));
        }
    }
    super::ocr::remap_relationships(&mut document.relationships, &remap, &document.elements);
    applied
}

enum PaintingMatch {
    Retained,
    Absent,
    Ambiguous,
}

fn retain_painting_text(
    element: &mut crate::types::internal::InternalElement,
    words: &[xberg_native_pdf::layout::Word],
) -> PaintingMatch {
    let Some(bounds) = element.bbox else {
        return PaintingMatch::Absent;
    };
    let mut visible = Vec::new();
    for word in words.iter().filter(|word| {
        let x = f64::from(word.bbox.x + word.bbox.width / 2.0);
        let y = f64::from(word.bbox.y + word.bbox.height / 2.0);
        bounds.x0 <= x && x <= bounds.x1 && bounds.y0 <= y && y <= bounds.y1
    }) {
        visible.extend(word.text.split_whitespace());
    }
    let words = element.text.split_whitespace();
    if visible
        .windows(words.clone().count())
        .any(|window| words.clone().eq(window.iter().copied()))
    {
        return PaintingMatch::Retained;
    }

    // Keep original byte intervals only when complete painting tokens witness them. ~keep
    let mut counts = ahash::AHashMap::<&str, usize>::new();
    for word in &visible {
        *counts.entry(word).or_default() += 1;
    }
    let mut start = 0;
    let mut retained = Vec::new();
    let mut skipped = Vec::new();
    for word in words {
        let offset = start + element.text[start..].find(word).expect("word came from this element");
        start = offset + word.len();
        let count = counts.entry(word).or_default();
        if *count > 0 {
            *count -= 1;
            retained.push(offset..start);
        } else {
            skipped.push(word);
        }
    }
    // A split spelling can compete with a whole token for the same painted occurrence.
    // Likewise, equal spellings can refer to different annotation ranges. ~keep
    if skipped.iter().any(|word| {
        visible
            .iter()
            .any(|painted| painted.contains(word) || word.contains(painted))
    }) {
        return PaintingMatch::Ambiguous;
    }
    if retained.is_empty() {
        return PaintingMatch::Absent;
    }
    rewrite_element_text(element, &retained);
    PaintingMatch::Retained
}

fn rewrite_element_text(element: &mut crate::types::internal::InternalElement, retained: &[std::ops::Range<usize>]) {
    let mut text = String::new();
    let mut annotations = Vec::new();
    for range in retained {
        if !text.is_empty() {
            text.push(' ');
        }
        let offset = text.len();
        text.push_str(&element.text[range.clone()]);
        for annotation in &element.annotations {
            let start = range.start.max(annotation.start as usize);
            let end = range.end.min(annotation.end as usize);
            if start < end {
                let mut annotation = annotation.clone();
                annotation.start = (offset + start - range.start) as u32;
                annotation.end = (offset + end - range.start) as u32;
                annotations.push(annotation);
            }
        }
    }
    element.text = text;
    element.annotations = annotations;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    fn bounds(y: f64) -> crate::types::BoundingBox {
        crate::types::BoundingBox {
            x0: 0.0,
            y0: y,
            x1: 100.0,
            y1: y + 5.0,
        }
    }

    fn span(text: &str, y: f32) -> xberg_native_pdf::layout::TextSpan {
        xberg_native_pdf::layout::TextSpan {
            text: text.into(),
            bbox: xberg_native_pdf::geometry::Rect::new(0.0, y, 50.0, 5.0),
            ..Default::default()
        }
    }

    fn positioned_words(spans: &[xberg_native_pdf::layout::TextSpan]) -> Vec<xberg_native_pdf::layout::Word> {
        xberg_native_pdf::PdfDocument::assemble_positioned_words(
            spans,
            xberg_native_pdf::geometry::Rect::new(0.0, 0.0, 612.0, 792.0),
            None,
        )
        .unwrap()
        .0
    }

    #[test]
    fn reversed_painting_spans_keep_the_native_word_and_annotation() {
        let mut left = span("Bud", 10.0);
        left.bbox.width = 15.0;
        let mut right = span("get", 10.0);
        right.bbox.x = 15.0;
        right.bbox.width = 15.0;
        right.sequence = 0;
        left.sequence = 1;
        let source = include_bytes!("../../../tests/fixtures/pdf/khmer-cluster-2536/scanned-english-khmer-overlay.pdf");
        let pdf = xberg_native_pdf::PdfDocument::from_bytes(source.to_vec()).unwrap();
        let words = painting_words(&pdf, 0, vec![right, left]).unwrap();
        assert_eq!(
            words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["Budget"]
        );

        let mut doc = InternalDocument::new("pdf");
        let mut element = InternalElement::text(ElementKind::Heading { level: 2 }, "Budget", 0).with_page(1);
        element.bbox = Some(bounds(10.0));
        let annotation = TextAnnotation {
            start: 0,
            end: 6,
            kind: AnnotationKind::Italic,
        };
        element.annotations.push(annotation.clone());
        doc.push_element(element);
        let read = RetainedPicturePage {
            visible: "Budget".into(),
            raster: Some("Raster-only body".into()),
            words,
        };
        let applied = merge_retained_picture_pages(&mut doc, &[(1, read)].into_iter().collect());
        assert_eq!(applied, [1].into_iter().collect());
        assert_eq!(doc.elements[0].kind, ElementKind::Heading { level: 2 });
        assert_eq!(doc.elements[0].annotations, [annotation]);
        assert_eq!(doc.elements[1].text, "Raster-only body");
    }

    #[test]
    fn picture_replacement_survives_missing_native_document_or_page_anchor() {
        let read = RetainedPicturePage {
            visible: "Budget".into(),
            raster: Some("Raster-only body".into()),
            words: positioned_words(&[span("Budget", 10.0)]),
        };
        let replacements = [(1, read.transcript())].into_iter().collect();
        let reads = [(1, read)].into_iter().collect();
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Neighbor", 0).with_page(2));
        for document in [None, Some(&mut doc)] {
            assert_eq!(
                unapplied_replacements(document, &reads, Some(&replacements)),
                Some(replacements.clone())
            );
        }
        assert_eq!(doc.elements[0].text, "Neighbor");
    }

    #[test]
    fn ambiguous_annotated_occurrences_use_the_corrected_transcript() {
        let mut doc = InternalDocument::new("pdf");
        let mut element = InternalElement::text(ElementKind::Paragraph, "Budget Budget", 0).with_page(1);
        element.bbox = Some(bounds(10.0));
        element.annotations = vec![
            TextAnnotation {
                start: 0,
                end: 6,
                kind: AnnotationKind::Bold,
            },
            TextAnnotation {
                start: 7,
                end: 13,
                kind: AnnotationKind::Italic,
            },
        ];
        doc.push_element(element);
        let read = RetainedPicturePage {
            visible: "Budget".into(),
            raster: Some("Raster-only body".into()),
            words: positioned_words(&[span("Budget", 10.0)]),
        };
        let replacements = [(1, read.transcript())].into_iter().collect();
        let reads = [(1, read)].into_iter().collect();
        let fallback = unapplied_replacements(Some(&mut doc), &reads, Some(&replacements));
        assert_eq!(
            fallback,
            Some(replacements),
            "the hidden bold occurrence cannot claim the painted italic occurrence"
        );
        super::super::ocr::merge_ocr_pages_into_internal_document(&mut doc, &fallback.unwrap());
        assert_eq!(crate::rendering::render_plain(&doc), "Budget\n\nRaster-only body");
        assert!(doc.elements.iter().all(|element| element.annotations.is_empty()));
    }

    #[test]
    fn split_hidden_token_cannot_claim_a_painted_token_or_its_annotations() {
        for source in ["Bud get", "Bud get Budget"] {
            let mut doc = InternalDocument::new("pdf");
            let mut element = InternalElement::text(ElementKind::Paragraph, source, 0).with_page(1);
            element.bbox = Some(bounds(10.0));
            element.annotations = vec![
                TextAnnotation {
                    start: 0,
                    end: 3,
                    kind: AnnotationKind::Bold,
                },
                TextAnnotation {
                    start: 4,
                    end: 7,
                    kind: AnnotationKind::Italic,
                },
            ];
            if source.ends_with(" Budget") {
                element.annotations.push(TextAnnotation {
                    start: 8,
                    end: 14,
                    kind: AnnotationKind::Underline,
                });
            }
            doc.push_element(element);
            let read = RetainedPicturePage {
                visible: "Budget".into(),
                raster: Some("Raster-only body".into()),
                words: positioned_words(&[span("Budget", 10.0)]),
            };
            let replacements = [(1, read.transcript())].into_iter().collect();
            let reads = [(1, read)].into_iter().collect();
            let fallback = unapplied_replacements(Some(&mut doc), &reads, Some(&replacements));
            assert_eq!(
                fallback,
                Some(replacements),
                "{source}: ambiguous token boundaries require fallback"
            );
            super::super::ocr::merge_ocr_pages_into_internal_document(&mut doc, &fallback.unwrap());
            assert_eq!(crate::rendering::render_plain(&doc), "Budget\n\nRaster-only body");
            assert!(doc.elements.iter().all(|element| element.annotations.is_empty()));
        }
    }

    #[test]
    fn raster_text_joins_the_native_heading_and_list_without_flattening_them() {
        let mut document = InternalDocument::new("pdf");
        let mut heading =
            InternalElement::text(ElementKind::Heading { level: 2 }, "Visible stale heading", 0).with_page(1);
        heading.bbox = Some(bounds(10.0));
        heading.annotations.push(TextAnnotation {
            start: 14,
            end: 21,
            kind: AnnotationKind::Bold,
        });
        document.push_element(heading);
        document.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
        let mut item = InternalElement::text(ElementKind::ListItem { ordered: false }, "Visible item", 1).with_page(1);
        item.bbox = Some(bounds(20.0));
        document.push_element(item);
        document.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
        let mut hidden = InternalElement::text(ElementKind::Heading { level: 1 }, "Visible item", 0).with_page(1);
        hidden.bbox = Some(bounds(30.0));
        document.push_element(hidden);
        document.push_element(InternalElement::text(ElementKind::Paragraph, "Neighbor", 0).with_page(2));

        merge_retained_picture_pages(
            &mut document,
            &[(
                1,
                RetainedPicturePage {
                    visible: "Visible heading\nVisible item".into(),
                    raster: Some("Raster-only body".into()),
                    words: positioned_words(&[span("Visible heading", 10.0), span("Visible item", 20.0)]),
                },
            )]
            .into_iter()
            .collect(),
        );

        let elements = &document.elements;
        assert_eq!(elements[0].kind, ElementKind::Heading { level: 2 });
        assert_eq!(elements[0].text, "Visible heading");
        assert_eq!(
            (elements[0].annotations[0].start, elements[0].annotations[0].end),
            (8, 15)
        );
        assert_eq!(elements[2].kind, ElementKind::ListItem { ordered: false });
        assert_eq!(elements[3].kind, ElementKind::ListEnd);
        assert_eq!(elements[4].page, Some(1));
        assert_eq!(elements[4].text, "Raster-only body");
        assert_eq!(elements[5].text, "Neighbor");
    }
}
