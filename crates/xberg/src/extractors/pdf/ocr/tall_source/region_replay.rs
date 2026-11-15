use std::{ops::Range, sync::Arc};

use super::{
    Attempt, OcrConfig,
    indexed::IndexedSource,
    region_probe::read_pair,
    region_reconciliation::{RegionBox, SourceWord, StockWord, plan_line_edits},
};
use crate::types::formats::RetainedHocrRead;

type Raster = (u32, u32, usize);

#[derive(Default)]
struct ReplayCapture {
    count: usize,
    first: Option<(RetainedHocrRead, Raster)>,
}

static CAPTURE: parking_lot::Mutex<Option<ReplayCapture>> = parking_lot::Mutex::new(None);

pub(in crate::extractors::pdf::ocr) fn certified_raster(
    read: &RetainedHocrRead,
    output: &crate::types::ExtractedDocument,
    (_, png, width, height): &super::super::rendering::EncodedPage,
    config: &crate::ExtractionConfig,
    (ocr, backend): (&OcrConfig, &Arc<dyn crate::plugins::OcrBackend>),
    correction: u32,
) -> Option<(u32, u32, usize)> {
    let builtin = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .is_builtin_tesseract(backend);
    let deskew_disabled = ocr
        .tesseract_config
        .as_ref()
        .and_then(|tess| tess.preprocessing.as_ref())
        .is_some_and(|settings| !settings.deskew);
    tracing::trace!(
        %config.output_format, builtin, deskew_disabled,
        correction, raster = ?(*width, *height), processed = ?read.dimensions,
        backend_correction = ?super::super::document::resolved_ocr_correction_degrees(&output.metadata),
        raw_frame = output.raw_ocr_source_frame.is_some(),
        owner_present = output.ocr_internal_document.is_some(),
        transcript_matches = output.content == read.transcript,
        scale_only = super::read_geometry::scale_only((*width, *height), read.dimensions, output.metadata.image_preprocessing.as_ref()),
        "checking selected hOCR receipt"
    );
    if correction != 0
        || config.output_format != crate::core::config::OutputFormat::Plain
        || !builtin
        || !deskew_disabled
        || output.raw_ocr_source_frame.is_some()
        || super::super::document::resolved_ocr_correction_degrees(&output.metadata).is_some_and(|angle| angle != 0)
        || !super::read_geometry::scale_only(
            (*width, *height),
            read.dimensions,
            output.metadata.image_preprocessing.as_ref(),
        )
        || !output
            .ocr_internal_document
            .as_ref()
            .is_some_and(|doc| read.matches(&read.transcript, read.dimensions, doc))
    {
        return None;
    }
    Some((*width, *height, png.len()))
}

pub(in crate::extractors::pdf::ocr) fn record_selected(read: &RetainedHocrRead, raster: Raster) {
    if let Some(capture) = CAPTURE.lock().as_mut() {
        capture.count += 1;
        if capture.first.is_none() {
            capture.first = Some((read.clone(), raster));
        }
    }
}

fn tokens(text: &str) -> Vec<(&str, Range<usize>)> {
    let base = text.as_ptr() as usize;
    text.split_whitespace()
        .map(|token| {
            let start = token.as_ptr() as usize - base;
            (token, start..start + token.len())
        })
        .collect()
}

fn project<'a>(stock: &str, view: &'a str, words: &[StockWord<'_>]) -> Option<Vec<StockWord<'a>>> {
    if stock.len().max(view.len()) > 16 * 1024 {
        return None;
    }
    let (stock, view) = (tokens(stock), tokens(view));
    if !stock.iter().map(|(text, _)| text).eq(view.iter().map(|(text, _)| text)) {
        return None;
    }
    words
        .iter()
        .map(|word| {
            let span = word.transcript_bytes.as_ref()?;
            let index = stock.binary_search_by_key(&span.start, |(_, range)| range.start).ok()?;
            if &stock[index].1 != span || stock[index].0 != word.text {
                return None;
            }
            Some(StockWord {
                transcript_bytes: Some(view[index].1.clone()),
                text: view[index].0,
                bounds: word.bounds,
            })
        })
        .collect()
}

struct Selection<'a> {
    words: Vec<StockWord<'a>>,
    protected: Vec<RegionBox>,
    region: RegionBox,
}

fn select<'a>(read: &'a RetainedHocrRead, source: &IndexedSource, witness: RegionBox) -> Option<Selection<'a>> {
    let mut words = Vec::new();
    let mut protected = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for occurrence in &read.occurrences {
        if !ids.insert(occurrence.id) {
            return None;
        }
        let bounds = source.native_bounds(read.dimensions, occurrence.bounds)?;
        if !bounds.intersects(witness) {
            protected.push(bounds);
            continue;
        }
        if !witness.contains(bounds) {
            return None;
        }
        let bytes = occurrence.transcript_bytes.clone();
        words.push(StockWord {
            text: read.transcript.get(bytes.clone())?,
            transcript_bytes: Some(bytes),
            bounds,
        });
    }
    let [first, second] = words.as_slice() else { return None };
    let region = RegionBox {
        left: first.bounds.left.min(second.bounds.left),
        top: first.bounds.top.min(second.bounds.top),
        right: first.bounds.right.max(second.bounds.right),
        bottom: first.bounds.bottom.max(second.bounds.bottom),
    };
    if protected.iter().any(|&bounds| region.intersects(bounds)) {
        return None;
    }
    Some(Selection {
        words,
        protected,
        region,
    })
}

fn revise(
    view: &str,
    stock: &str,
    selection: &Selection<'_>,
    column: RegionBox,
    sources: [&[SourceWord<'_>]; 2],
) -> Option<String> {
    let words = project(stock, view, &selection.words)?;
    let edits = plan_line_edits(view, selection.region, column, &words, sources, &selection.protected)?;
    let mut revised = view.to_owned();
    for edit in edits.into_iter().rev() {
        revised.replace_range(edit.bytes, edit.replacement);
    }
    Some(revised)
}

async fn recover(
    attempt: &Attempt<'_>,
    backend: &Arc<dyn crate::plugins::OcrBackend>,
    ocr: &OcrConfig,
    read: &RetainedHocrRead,
    witness: RegionBox,
    (document, page): (&str, &str),
) -> crate::Result<Option<(String, String)>> {
    let Some(source) = IndexedSource::load(attempt) else {
        tracing::trace!("archived replay declined Indexed source admission");
        return Ok(None);
    };
    let Some(selection) = select(read, &source, witness) else {
        tracing::trace!("archived replay declined selected word ownership");
        return Ok(None);
    };
    if source.certify_line_ink(selection.region, witness).is_none() {
        tracing::trace!("archived replay declined source ink at the crop perimeter");
        return Ok(None);
    }
    if project(&read.transcript, document, &selection.words).is_none()
        || project(&read.transcript, page, &selection.words).is_none()
    {
        tracing::trace!("archived replay declined final text projections");
        return Ok(None);
    }
    tracing::trace!(bounds = ?(selection.region.left, selection.region.top, selection.region.right, selection.region.bottom), "archived replay probing selected source crop");
    let Some(reads) = read_pair(attempt, &source, backend, ocr, selection.region, witness, None).await? else {
        tracing::trace!("archived replay declined bounded source read pair");
        return Ok(None);
    };
    let [first, second] = reads.each_ref().map(|words| {
        words
            .iter()
            .map(|word| SourceWord {
                text: &word.text,
                bounds: word.bounds,
                confidence: word.confidence,
            })
            .collect::<Vec<_>>()
    });
    let revise_view = |view| revise(view, &read.transcript, &selection, witness, [&first, &second]);
    Ok(revise_view(document).zip(revise_view(page)))
}

fn publish_verified_pair(
    (document, page): (&mut String, &mut String),
    (document_bytes, page_bytes): (Range<usize>, Range<usize>),
    expected: &str,
    replacement: &str,
) -> Option<()> {
    if expected.is_empty()
        || document.get(document_bytes.clone())? != expected
        || page.get(page_bytes.clone())? != expected
    {
        return None;
    }
    let mut revised_document = document.clone();
    let mut revised_page = page.clone();
    revised_document.replace_range(document_bytes, replacement);
    revised_page.replace_range(page_bytes, replacement);
    *document = revised_document;
    *page = revised_page;
    Some(())
}

#[cfg(feature = "bundle-tessdata-eng")]
#[path = "region_replay_tests.rs"]
mod tests;
