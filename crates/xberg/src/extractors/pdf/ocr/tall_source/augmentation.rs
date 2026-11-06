//! Add independently ordered source occurrences only at certified projection cuts.

use super::indexed::IndexedSource;
use super::ordered::{BoxPx, right_column};
use super::{Attempt, MAX_TILE_SIDE, STRIP_OCR, wait_for_cancel};
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::plugins::OcrBackend;
use crate::table_core::HocrWord;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractedDocument, OcrElementLevel, OcrTable, OcrTableBoundingBox};
use std::ops::Range;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

const MAX_ATTEMPTS: usize = 4;
const MAX_OWNERS: usize = 1024;
const MAX_PRIMARY_BYTES: usize = 16 * 1024;

pub(in super::super) fn supports_raster_height(height: u32) -> bool {
    height > MAX_TILE_SIDE
}

#[derive(Clone, Copy)]
struct Band {
    top: u32,
    bottom: u32,
}

struct Owner {
    bytes: Range<usize>,
    band: Band,
}

pub(in super::super) struct PrimaryProof {
    frame: (u32, u32),
    raster: (u32, u32, usize),
    protected: Vec<Band>,
    page_owners: Vec<Owner>,
    original_page: String,
    original_table: crate::types::Table,
    table: Band,
}

fn metadata_u32(result: &ExtractedDocument, key: &str) -> Option<u32> {
    u32::try_from(result.metadata.additional.get(key)?.as_u64()?).ok()
}

fn processed_frame(result: &ExtractedDocument) -> Option<(u32, u32)> {
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
    Some((
        metadata_u32(result, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?,
        metadata_u32(result, OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?,
    ))
}

fn accepted_raw_read(result: &ExtractedDocument) -> bool {
    result.metadata.additional.get("psm").and_then(|value| value.as_str()) == Some("11")
        && result.processing_warnings.is_empty()
        && result.formulas.is_empty()
        && result.document.is_none()
        && result
            .metadata
            .additional
            .get("word_iterator_skipped_count")
            .is_none_or(|count| count.as_u64() == Some(0))
}

impl PrimaryProof {
    pub(in super::super) fn capture(
        primary: &ExtractedDocument,
        elements: &[crate::types::OcrElement],
        raster: (u32, u32, usize),
    ) -> Option<Self> {
        if !accepted_raw_read(primary) || primary.content.len() > MAX_PRIMARY_BYTES {
            return None;
        }
        let [table] = primary.tables.as_slice() else {
            return None;
        };
        let frame = processed_frame(primary)?;
        let bounds = table.bounding_box?;
        let bounds = OcrTableBoundingBox {
            left: exact_pixel(bounds.x0, frame.0)?,
            top: exact_pixel(bounds.y0, frame.1)?,
            right: exact_pixel(bounds.x1, frame.0)?,
            bottom: exact_pixel(bounds.y1, frame.1)?,
        };
        let band = Band {
            top: bounds.top,
            bottom: bounds.bottom,
        };
        if bounds.left >= bounds.right
            || bounds.top >= bounds.bottom
            || table.markdown.len() > MAX_PRIMARY_BYTES
            || table.cells.len() > MAX_OWNERS
            || table.cells.iter().map(Vec::len).sum::<usize>() > MAX_OWNERS
            || table.cells.iter().flatten().map(String::len).sum::<usize>() > MAX_PRIMARY_BYTES
        {
            return None;
        }
        let mut protected = vec![band];
        let raw = primary.ocr_internal_document.as_ref()?;
        if elements.len().checked_add(raw.elements.len())? > MAX_OWNERS || !raw.relationships.is_empty() {
            return None;
        }
        let words = capture_words(elements, frame, &mut protected)?;
        if metadata_u32(primary, "word_count")? as usize != words.len() {
            return None;
        }
        for block in &raw.elements {
            let bounds = block.bbox?;
            protected.push(Band {
                top: exact_pixel(bounds.y0, frame.1)?,
                bottom: exact_pixel(bounds.y1, frame.1)?,
            });
        }
        let page_owners = page_owners(primary, table.markdown.clone(), bounds, &words)?;
        Some(Self {
            frame,
            raster,
            protected,
            page_owners,
            original_page: primary.content.clone(),
            original_table: table.clone(),
            table: band,
        })
    }
    pub(in super::super) fn matches(&self, selected: &ExtractedDocument) -> bool {
        selected.content == self.original_page
            && processed_frame(selected) == Some(self.frame)
            && accepted_raw_read(selected)
            && matches!(selected.tables.as_slice(), [table]
                if table.bounding_box == self.original_table.bounding_box
                    && table.markdown == self.original_table.markdown
                    && table.cells == self.original_table.cells)
    }
}

fn exact_pixel(value: f64, max: u32) -> Option<u32> {
    (value >= 0.0 && value <= f64::from(max) && value == f64::from(value as u32)).then_some(value as u32)
}

fn capture_words(
    elements: &[crate::types::OcrElement],
    frame: (u32, u32),
    protected: &mut Vec<Band>,
) -> Option<Vec<HocrWord>> {
    let mut words = Vec::new();
    let mut text_bytes = 0usize;
    for element in elements {
        let bounds = BoxPx::read(element, frame)?;
        protected.push(Band {
            top: bounds.y0,
            bottom: bounds.y1,
        });
        if element.level == OcrElementLevel::Word {
            text_bytes = text_bytes.checked_add(element.text.len())?;
            if text_bytes > MAX_PRIMARY_BYTES || element.text.split_whitespace().next() != Some(&element.text) {
                return None;
            }
            words.push(HocrWord {
                text: element.text.clone(),
                left: bounds.x0,
                top: bounds.y0,
                width: bounds.x1 - bounds.x0,
                height: bounds.y1 - bounds.y0,
                confidence: 0.0,
            });
        }
    }
    Some(words)
}

fn page_owners(
    primary: &ExtractedDocument,
    markdown: String,
    bounds: OcrTableBoundingBox,
    words: &[HocrWord],
) -> Option<Vec<Owner>> {
    let table = OcrTable {
        markdown,
        cells: Vec::new(),
        page_number: 1,
        bounding_box: Some(bounds),
    };
    let mut owners = Vec::new();
    let mut complete = true;
    let replay = crate::ocr::processor::render_inline_ocr_table_content(
        words,
        &[table],
        Some(&mut |bytes, band| {
            if let Some((top, bottom)) = band {
                owners.push(Owner {
                    bytes,
                    band: Band { top, bottom },
                });
            } else {
                complete = false;
            }
        }),
    );
    (complete && replay == primary.content && owners.len() >= 2).then_some(owners)
}

fn projection_cut(
    bands: impl Iterator<Item = Band>,
    tile: &Range<u32>,
    source_y: impl Fn(u32) -> Option<f64>,
) -> Option<usize> {
    let mut before = 0;
    let mut after = false;
    for band in bands {
        if band.top >= band.bottom {
            return None;
        }
        let top = source_y(band.top)?;
        let bottom = source_y(band.bottom)?;
        if bottom < f64::from(tile.start) && !after {
            before += 1;
        } else if top > f64::from(tile.end) {
            after = true;
        } else {
            return None;
        }
    }
    (before > 0 && after).then_some(before)
}

fn raw_tile_is_empty(proof: &PrimaryProof, source: &IndexedSource, rows: &Range<u32>) -> bool {
    proof.protected.iter().all(|band| {
        band.top < band.bottom
            && (source
                .source_y(band.bottom, proof.frame.1)
                .is_some_and(|y| y < f64::from(rows.start))
                || source
                    .source_y(band.top, proof.frame.1)
                    .is_some_and(|y| y > f64::from(rows.end)))
    })
}

fn source_tiles(
    proof: &PrimaryProof,
    source: &IndexedSource,
    max_attempts: usize,
    cancel: impl Fn() -> bool,
) -> Option<Vec<(Range<u32>, Range<u32>)>> {
    let seams = source.seams(&cancel)?;
    let mut candidates = Vec::new();
    for pair in seams.windows(2) {
        if cancel() {
            return None;
        }
        let rows = pair[0]..pair[1];
        if rows.len() <= source.max_tile_rows()
            && raw_tile_is_empty(proof, source, &rows)
            && let Some(separator) = source.right_separator(rows.clone())
        {
            if candidates.len() == max_attempts {
                return None;
            }
            candidates.push((rows, separator));
        }
    }
    Some(candidates)
}

fn tile_config(config: &OcrConfig, dpi: f64) -> OcrConfig {
    let mut config = super::ocr_config_with_page_rotation_hint(config, 0, Some(dpi), true, false, false).into_owned();
    config.element_config = Some(crate::types::OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        ..Default::default()
    });
    if let Some(tesseract) = config.tesseract_config.as_mut() {
        tesseract.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            target_dpi: dpi as i32,
            ..Default::default()
        });
    }
    if let Some(options) = config.backend_options.as_mut() {
        options["use_cache"] = serde_json::Value::Bool(false);
    }
    config
}

async fn read_tiles(
    proof: &PrimaryProof,
    source: &IndexedSource,
    backend: &Arc<dyn OcrBackend>,
    config: &OcrConfig,
    attempts: usize,
    cancel: impl Fn() -> bool,
) -> Option<(Range<u32>, Vec<String>)> {
    let mut config = tile_config(config, source.dpi);
    let mut range = None::<Range<u32>>;
    let mut text = Vec::new();
    let mut text_bytes = 0usize;
    for (rows, separator) in source_tiles(proof, source, attempts, &cancel)? {
        if cancel() {
            return None;
        }
        let png = source.encode(rows.clone())?;
        config.security_limits.get_or_insert_default().max_content_size = source.backend_limit(png.capacity())?;
        let result = backend.process_image_owned(Arc::new(png), &config).await.ok()?;
        let frame = (source.width, rows.end - rows.start);
        if !accepted_raw_read(&result) || processed_frame(&result)? != frame || !result.tables.is_empty() {
            return None;
        }
        let elements = result.ocr_elements.as_ref()?;
        if metadata_u32(&result, "word_count")? as usize
            != elements
                .iter()
                .filter(|element| element.level == OcrElementLevel::Word)
                .count()
        {
            return None;
        }
        let Some(lines) = right_column(elements, frame, separator) else {
            continue;
        };
        if let Some(range) = range.as_mut() {
            if range.end != rows.start {
                return None;
            }
            range.end = rows.end;
        } else {
            range = Some(rows);
        }
        for (line, _) in lines {
            text_bytes = text_bytes.checked_add(line.len())?;
            if text_bytes > MAX_PRIMARY_BYTES {
                return None;
            }
            text.push(line.to_owned());
        }
    }
    Some((range?, text))
}

fn typed_bands(doc: &InternalDocument, proof: &PrimaryProof) -> Option<Vec<(usize, Band)>> {
    if doc.elements.len() > MAX_OWNERS
        || !doc.relationships.is_empty()
        || doc.tables.len() != 1
        || !doc.images.is_empty()
        || !doc.formulas.is_empty()
        || !doc.form_fields.is_empty()
        || doc.annotations.as_ref().is_some_and(|entries| !entries.is_empty())
    {
        return None;
    }
    let mut owners = Vec::new();
    let mut containers = 0usize;
    let mut saw_table = false;
    for (index, element) in doc.elements.iter().enumerate() {
        if element.anchor.is_some() || !element.annotations.is_empty() {
            return None;
        }
        let band = match element.kind {
            ElementKind::ListStart { .. } => {
                containers += 1;
                continue;
            }
            ElementKind::ListEnd => {
                containers = containers.checked_sub(1)?;
                continue;
            }
            ElementKind::Table { table_index: 0 } if !saw_table => {
                saw_table = true;
                proof.table
            }
            ElementKind::Paragraph | ElementKind::ListItem { .. } => {
                let bounds = element.bbox?;
                Band {
                    top: proof.frame.1 - exact_pixel(bounds.y1, proof.frame.1)?,
                    bottom: proof.frame.1 - exact_pixel(bounds.y0, proof.frame.1)?,
                }
            }
            _ => return None,
        };
        if element.page != Some(1) {
            return None;
        }
        owners.push((index, band));
    }
    (containers == 0 && saw_table).then_some(owners)
}

fn apply(
    proof: &PrimaryProof,
    source: &IndexedSource,
    doc: &mut InternalDocument,
    page: &mut String,
    rows: Range<u32>,
    lines: Vec<String>,
) -> Option<()> {
    if *page != proof.original_page {
        return None;
    }
    let source_y = |y| source.source_y(y, proof.frame.1);
    let page_owner = projection_cut(proof.page_owners.iter().map(|owner| owner.band), &rows, source_y)?;
    let page_at = proof.page_owners.get(page_owner - 1)?.bytes.end;
    if page.get(page_at..page_at.checked_add(2)?)? != "\n\n" {
        return None;
    }
    let owners = typed_bands(doc, proof)?;
    let typed_owner = projection_cut(owners.iter().map(|(_, band)| *band), &rows, source_y)?;
    let element_at = owners.get(typed_owner)?.0;
    let mut suffix = String::new();
    for element in doc.elements.get(element_at..)? {
        if element.kind != ElementKind::Paragraph || element.depth != 0 || element.text.is_empty() {
            return None;
        }
        suffix.push_str("\n\n");
        suffix.push_str(&element.text);
    }
    let old = crate::rendering::render_plain(doc);
    let byte_at = old.strip_suffix(&suffix)?.len();
    let inserted = format!("\n\n{}", lines.join("\n"));
    let element = InternalElement::text(ElementKind::Paragraph, lines.join("\n"), 0)
        .with_page(1)
        .with_index(u32::try_from(doc.elements.len()).ok()?);
    doc.elements.insert(element_at, element);
    let new = crate::rendering::render_plain(doc);
    let end = byte_at + inserted.len();
    if new.get(..byte_at) != old.get(..byte_at)
        || new.get(byte_at..end) != Some(&inserted)
        || new.get(end..) != old.get(byte_at..)
    {
        doc.elements.remove(element_at);
        return None;
    }
    page.insert_str(page_at, &inserted);
    Some(())
}

pub(in super::super) async fn augment(
    proof: PrimaryProof,
    (doc, page): (&mut InternalDocument, &mut String),
    backend: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    pdf: &PdfDocument,
) -> crate::Result<bool> {
    let attempt = Attempt::new(pdf, 0, Some(proof.raster), config, ocr);
    let cancel = || attempt.cancel.is_some_and(|token| token.is_cancelled());
    let _permit = tokio::select! {
        permit = STRIP_OCR.acquire() => permit.expect("strip semaphore never closes"),
        () = wait_for_cancel(attempt.cancel) => return Err(crate::XbergError::Cancelled),
    };
    if cancel() {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(source) = IndexedSource::load(&attempt) else {
        return Ok(false);
    };
    let result = read_tiles(
        &proof,
        &source,
        backend,
        ocr,
        MAX_ATTEMPTS.min(attempt.max_strips),
        cancel,
    )
    .await;
    if cancel() {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(result
        .and_then(|(rows, text)| apply(&proof, &source, doc, page, rows, text))
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_cut_neighbors_must_be_wholly_outside_the_inserted_source_band() {
        let body = Band { top: 12, bottom: 20 };
        let table = Band { top: 25, bottom: 30 };
        let after = Band { top: 90, bottom: 99 };
        let cut = |owners: [Band; 3]| projection_cut(owners.into_iter(), &(40..80), |y| Some(f64::from(y)));
        assert_eq!(cut([table, body, after]), Some(2));
        let crossing = Band { top: 25, bottom: 45 };
        assert_eq!(cut([crossing, body, after]), None);
        let touching = Band { top: 80, bottom: 99 };
        assert_eq!(cut([table, body, touching]), None);
        assert_eq!(cut([table, after, body]), None);
    }
}
