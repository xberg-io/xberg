//! Two bounded source reads of an already certified region. Selection remains with the caller.

use super::{
    Attempt, STRIP_OCR, indexed::IndexedSource, ordered::BoxPx, region_reconciliation::RegionBox, wait_for_cancel,
};
use crate::core::config::OcrConfig;
use crate::plugins::OcrBackend;
use crate::types::OcrElementLevel;
use std::sync::Arc;

const PAD: u32 = 16;
const MAX_PAIR_PIXELS: u32 = 256 * 1024;
const MAX_READ_WORDS: usize = 64;

pub(super) struct RegionWord {
    pub(super) text: String,
    pub(super) bounds: RegionBox,
    pub(super) confidence: f64,
}

struct Crop {
    region: RegionBox,
    dimensions: (u32, u32),
}

impl Crop {
    fn new(region: RegionBox, column: RegionBox) -> Option<Self> {
        if !column.contains(region) {
            return None;
        }
        let width = (region.right - region.left).checked_add(PAD * 2)?;
        let height = (region.bottom - region.top).checked_add(PAD * 2)?;
        (width.checked_mul(height)?.checked_mul(2)? <= MAX_PAIR_PIXELS).then_some(Self {
            region,
            dimensions: (width, height),
        })
    }

    fn project(&self, bounds: BoxPx) -> Option<RegionBox> {
        let region = RegionBox {
            left: self.region.left.checked_add(bounds.x0.checked_sub(PAD)?)?,
            top: self.region.top.checked_add(bounds.y0.checked_sub(PAD)?)?,
            right: self.region.left.checked_add(bounds.x1.checked_sub(PAD)?)?,
            bottom: self.region.top.checked_add(bounds.y1.checked_sub(PAD)?)?,
        };
        self.region.contains(region).then_some(region)
    }
}

fn read_words(read: &crate::types::ExtractedDocument, crop: &Crop, psm: i32) -> Option<Vec<RegionWord>> {
    if read.metadata.additional.get("psm")?.as_str()?.parse::<i32>().ok()? != psm
        || super::super::document::processed_ocr_layout_dimensions(&read.metadata)? != crop.dimensions
        || !read.processing_warnings.is_empty()
        || !read.tables.is_empty()
        || !read.formulas.is_empty()
    {
        return None;
    }
    let words: Vec<_> = read
        .ocr_elements
        .as_ref()?
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .take(MAX_READ_WORDS + 1)
        .map(|word| {
            let bounds = crop.project(BoxPx::read(word, crop.dimensions)?)?;
            Some(RegionWord {
                text: word.text.clone(),
                bounds,
                confidence: word.confidence.recognition * 100.0,
            })
        })
        .collect::<Option<_>>()?;
    let count = usize::try_from(read.metadata.additional.get("word_count")?.as_u64()?).ok()?;
    (!words.is_empty() && words.len() <= MAX_READ_WORDS && words.len() == count).then_some(words)
}

fn probe_config(config: &OcrConfig, dpi: f64, limit: usize) -> OcrConfig {
    let mut probe = super::augmentation::tile_config(config, dpi);
    let mut tess = probe.tesseract_config.take().unwrap_or_default();
    tess.use_cache = false;
    tess.enable_table_detection = false;
    probe.auto_rotate = Some(false);
    tess.preprocessing = Some(crate::types::ImagePreprocessingConfig {
        target_dpi: dpi as i32,
        auto_rotate: false,
        deskew: false,
        ..Default::default()
    });
    probe.tesseract_config = Some(tess);
    let mut options = probe
        .backend_options
        .take()
        .and_then(|options| match options {
            serde_json::Value::Object(options) => Some(options),
            _ => None,
        })
        .unwrap_or_default();
    options.remove("source_region_reconciliation");
    options.insert("use_cache".into(), serde_json::Value::Bool(false));
    probe.backend_options = Some(serde_json::Value::Object(options));
    probe.security_limits.get_or_insert_default().max_content_size = limit;

    probe
}

fn agree(left: &[RegionWord], right: &[RegionWord]) -> bool {
    left.len() == right.len()
        && !left.is_empty()
        && left.iter().zip(right).all(|(a, b)| {
            a.text == b.text
                && a.bounds.intersects(b.bounds)
                && (85.0..=100.0).contains(&a.confidence)
                && (85.0..=100.0).contains(&b.confidence)
        })
}

pub(super) fn anchored_ink_edge(words: &[RegionBox], components: &[std::ops::Range<u32>]) -> bool {
    const MIN_EXTENSION: u32 = 8;
    let Some(last) = words.len().checked_sub(1).filter(|&last| last >= 2) else {
        return false;
    };
    let owned = |index: usize, component: &std::ops::Range<u32>| {
        let overlaps = |word: &RegionBox| component.start < word.right && word.left < component.end;
        overlaps(&words[index]) && words.iter().filter(|word| overlaps(word)).count() == 1
    };
    [(0, [1, 2]), (last, [last - 1, last - 2])]
        .into_iter()
        .any(|(edge, anchors)| {
            anchors
                .into_iter()
                .all(|anchor| components.iter().filter(|c| owned(anchor, c)).count() == 1)
                && components.iter().any(|component| {
                    owned(edge, component)
                        && (words[edge].left.saturating_sub(component.start) >= MIN_EXTENSION
                            || component.end.saturating_sub(words[edge].right) >= MIN_EXTENSION)
                })
        })
}

pub(super) fn discover_anchored_regions(
    source: &IndexedSource,
    words: &[RegionBox],
    protected: &[RegionBox],
    call_limit: usize,
) -> Option<Vec<(std::ops::Range<usize>, RegionBox)>> {
    if words.len() > 1024 || protected.len() > 1024 {
        return None;
    }
    let same_line = |a: RegionBox, b: RegionBox| {
        let height = (a.bottom - a.top).min(b.bottom - b.top);
        let shared = a.bottom.min(b.bottom).saturating_sub(a.top.max(b.top));
        height > 0 && shared >= height.div_ceil(2) && b.left >= a.right && b.left - a.right <= height.saturating_mul(3)
    };
    if words.iter().any(|word| {
        word.left >= word.right || word.top >= word.bottom || word.right > source.width || word.bottom > source.height
    }) {
        return None;
    }
    let mut candidates = Vec::new();
    let mut start = 0;
    for run in words.chunk_by(|a, b| same_line(*a, *b)) {
        let indices = start..start + run.len();
        start = indices.end;
        if run.len() < 3 {
            continue;
        }
        if let Some(region) = anchored_region(source, run, protected)? {
            candidates.push((indices, region));
            if candidates.len().checked_mul(2)? > call_limit {
                return None;
            }
        }
    }
    Some(candidates)
}

fn anchored_region(source: &IndexedSource, run: &[RegionBox], protected: &[RegionBox]) -> Option<Option<RegionBox>> {
    let first = *run.first()?;
    let mut hull = first;
    let (mut band_top, mut band_bottom) = (first.top, first.bottom);
    for &word in run {
        hull.left = hull.left.min(word.left);
        hull.right = hull.right.max(word.right);
        hull.top = hull.top.min(word.top);
        hull.bottom = hull.bottom.max(word.bottom);
        band_top = band_top.max(word.top);
        band_bottom = band_bottom.min(word.bottom);
    }
    if protected.iter().any(|&region| hull.intersects(region)) || band_top >= band_bottom {
        return Some(None);
    }
    let edge_ink = |word: RegionBox| -> Option<bool> {
        let left = RegionBox {
            left: word.left.saturating_sub(4),
            right: word.left,
            ..word
        };
        let right = RegionBox {
            left: word.right,
            right: word.right.saturating_add(4).min(source.width),
            ..word
        };
        for strip in [left, right] {
            if strip.left < strip.right && !source.line_ink_components(strip)?.is_empty() {
                return Some(true);
            }
        }
        Some(false)
    };
    if !edge_ink(first)? && !edge_ink(*run.last()?)? {
        return Some(None);
    }
    let strip = RegionBox {
        left: hull.left.saturating_sub(40),
        top: band_top.saturating_sub(2),
        right: hull.right.saturating_add(40).min(source.width),
        bottom: band_bottom.saturating_add(2).min(source.height),
    };
    let components = source.line_ink_components(strip)?;
    if !anchored_ink_edge(run, &components) {
        return Some(None);
    }
    for component in components {
        if run
            .iter()
            .filter(|word| component.start < word.right && word.left < component.end)
            .count()
            == 1
        {
            hull.left = hull.left.min(component.start);
            hull.right = hull.right.max(component.end);
        }
    }
    let crop = RegionBox {
        left: hull.left.checked_sub(1)?,
        top: hull.top.checked_sub(2)?,
        right: hull.right.checked_add(1)?,
        bottom: hull.bottom.checked_add(2)?,
    };
    if protected.iter().any(|&region| crop.intersects(region)) {
        return Some(None);
    }
    Some(Some(crop))
}

pub(super) async fn scan_selected_inventory(
    attempt: &Attempt<'_>,
    source: &IndexedSource,
    backend: &Arc<dyn OcrBackend>,
    ocr: &OcrConfig,
    selected: &crate::types::ExtractedDocument,
    read: &crate::types::formats::RetainedHocrRead,
) -> crate::Result<Option<(usize, String, String)>> {
    let native = |bounds| source.native_bounds(read.dimensions, bounds);
    let Some(words) = read
        .occurrences
        .iter()
        .map(|word| native(word.bounds))
        .collect::<Option<Vec<_>>>()
    else {
        tracing::trace!("anchored OCR discovery declined an unmappable selected word");
        return Ok(None);
    };
    let Some(tables) = selected
        .tables
        .iter()
        .map(|table| {
            let b = table.bounding_box.as_ref()?;
            let coordinates = [b.x0.floor(), b.y0.floor(), b.x1.ceil(), b.y1.ceil()];
            if coordinates
                .iter()
                .any(|&value| !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX))
            {
                return None;
            }
            native(coordinates.map(|value| value as u32))
        })
        .collect::<Option<Vec<_>>>()
    else {
        tracing::trace!("anchored OCR discovery declined an unmappable table");
        return Ok(None);
    };
    let Some(regions) = discover_anchored_regions(source, &words, &tables, attempt.max_strips) else {
        tracing::trace!(
            words = words.len(),
            "anchored OCR discovery could not enumerate within budget"
        );
        return Ok(None);
    };
    tracing::trace!(
        words = words.len(),
        regions = regions.len(),
        calls = regions.len() * 2,
        "anchored OCR discovery enumerated selected inventory"
    );
    let frame = RegionBox {
        left: 0,
        top: 0,
        right: source.width,
        bottom: source.height,
    };
    let mut replacement = None;
    let mut ambiguous = false;
    for (stock, region) in regions {
        let tighter = RegionBox {
            top: words[stock.clone()]
                .iter()
                .map(|word| word.top)
                .min()
                .unwrap_or(region.top),
            bottom: words[stock.clone()]
                .iter()
                .map(|word| word.bottom)
                .max()
                .unwrap_or(region.bottom),
            ..region
        };
        let region = source
            .certify_line_ink(region, frame)
            .or_else(|| source.certify_line_ink(tighter, frame));
        let Some(region) = region else {
            tracing::trace!(?stock, "anchored OCR discovery declined candidate perimeter");
            continue;
        };
        let Some(reads) = read_pair(attempt, source, backend, ocr, region, frame).await? else {
            continue;
        };
        let originals: Vec<_> = read.occurrences[stock.clone()]
            .iter()
            .zip(&words[stock.clone()])
            .filter_map(|(word, &bounds)| {
                Some(super::region_reconciliation::StockWord {
                    transcript_bytes: Some(word.transcript_bytes.clone()),
                    text: read.transcript.get(word.transcript_bytes.clone())?,
                    bounds,
                })
            })
            .collect();
        if originals.len() != stock.len() {
            continue;
        }
        let protected: Vec<_> = words
            .iter()
            .enumerate()
            .filter_map(|(index, &word)| (!stock.contains(&index)).then_some(word))
            .chain(tables.iter().copied())
            .collect();
        let [a, b] = reads.each_ref().map(|words| {
            words
                .iter()
                .map(|word| super::region_reconciliation::SourceWord {
                    text: word.text.as_str(),
                    bounds: word.bounds,
                    confidence: word.confidence,
                })
                .collect::<Vec<_>>()
        });
        let Some(edits) = super::region_reconciliation::plan_line_edits(
            &read.transcript,
            region,
            frame,
            &originals,
            [&a, &b],
            &protected,
        ) else {
            continue;
        };
        let [edit] = edits.as_slice() else { continue };
        if edit.selection_index != 0 && edit.selection_index + 1 != stock.len() {
            continue;
        }
        let id = read.occurrences[stock.start + edit.selection_index].id;
        if ![read.inline_page.as_ref(), read.plain_document.as_ref()]
            .into_iter()
            .all(|projection| {
                projection
                    .is_some_and(|projection| projection.ranges.iter().filter(|(owner, _)| *owner == id).count() == 1)
            })
        {
            continue;
        }
        if replacement
            .replace((id, edit.expected.to_owned(), edit.replacement.to_owned()))
            .is_some()
        {
            ambiguous = true;
        }
    }
    Ok(if ambiguous { None } else { replacement })
}

pub(super) async fn read_pair(
    attempt: &Attempt<'_>,
    source: &IndexedSource,
    backend: &Arc<dyn OcrBackend>,
    config: &OcrConfig,
    region: RegionBox,
    column: RegionBox,
) -> crate::Result<Option<[Vec<RegionWord>; 2]>> {
    if attempt.max_strips < 2
        || !crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .is_builtin_tesseract(backend)
    {
        return Ok(None);
    }
    let Some(crop) = Crop::new(region, column) else {
        return Ok(None);
    };
    let cancel = || attempt.cancel.is_some_and(|token| token.is_cancelled());
    let _permit = tokio::select! {
        permit = STRIP_OCR.acquire() => permit.expect("strip semaphore never closes"),
        () = wait_for_cancel(attempt.cancel) => return Err(crate::XbergError::Cancelled),
    };
    if cancel() {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(png) = source.encode_region(region, PAD) else {
        return Ok(None);
    };
    let Some(limit) = source.backend_limit(png.capacity()) else {
        return Ok(None);
    };
    let dpi = source.dpi;
    let png = Arc::new(png);
    let mut probe = probe_config(config, dpi, limit);

    let mut reads = [Vec::new(), Vec::new()];
    for (psm, slot) in [3, 11].into_iter().zip(&mut reads) {
        if cancel() {
            return Err(crate::XbergError::Cancelled);
        }
        probe.tesseract_config.as_mut().expect("configured above").psm = Some(psm);
        let read = match backend.process_image_owned(Arc::clone(&png), &probe).await {
            Ok(read) => read,
            Err(error @ crate::XbergError::Cancelled) => return Err(error),
            Err(error) => {
                crate::core::extractor::cacheability::invalidate_current();
                tracing::debug!(%error, "optional indexed region OCR failed");
                return Ok(None);
            }
        };
        if cancel() {
            return Err(crate::XbergError::Cancelled);
        }
        let Some(words) = read_words(&read, &crop, psm) else {
            return Ok(None);
        };
        *slot = words;
    }
    Ok(agree(&reads[0], &reads[1]).then_some(reads))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchored_edge_ink_refuses_a_component_shared_by_two_stock_words() {
        let words = [20..30, 44..54, 64..74].map(|x| RegionBox {
            left: x.start,
            right: x.end,
            top: 10,
            bottom: 20,
        });
        assert!(anchored_ink_edge(&words, &[20..40, 44..54, 64..74]));
        assert!(!anchored_ink_edge(&words, &[20..45, 46..54, 64..74]));
        assert!(!anchored_ink_edge(&words, &[20..40, 44..48, 50..54, 64..74]));
        assert!(!anchored_ink_edge(&words[..2], &[20..40, 44..54]));
    }

    #[test]
    fn crop_rejects_gutters_oversize_and_padding_claims() {
        let column = RegionBox {
            left: 10,
            top: 10,
            right: 800,
            bottom: 500,
        };
        let region = RegionBox {
            left: 20,
            top: 20,
            right: 120,
            bottom: 60,
        };
        let crop = Crop::new(region, column).unwrap();
        assert_eq!(crop.dimensions, (132, 72));
        let outside = BoxPx {
            x0: 15,
            y0: 16,
            x1: 30,
            y1: 40,
        };
        assert!(crop.project(outside).is_none());
        assert!(Crop::new(RegionBox { right: 801, ..region }, column).is_none());
        assert!(Crop::new(column, column).is_none());
    }
}
