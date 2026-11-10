use super::super::is_khmer_digit;
use crate::core::config::ExtractionConfig;
use crate::ocr::processor::validation::{TessdataEnv, existing_tessdata_dir_for};
use crate::ocr::processor::{ImageOcrPolicy, OcrProcessor};
use crate::ocr::types::TesseractConfig;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use xberg_native_pdf::{PdfDocument, geometry::Rect, layout::TextChar};

const POINTS_PER_INCH: f32 = 72.0;
const MAX_NUMBER_GLYPHS: usize = 64;
const MAX_NATIVE_CHARS: usize = 32_768;
const MAX_CROP_PIXELS: u64 = 1024 * 1024;
const CROP_BORDER: u32 = 8;

mod budget;
mod read_proof;
mod shape_proof;
pub(super) use read_proof::ReadProof;
use shape_proof::Preparation;

#[derive(Clone)]
struct Number {
    text: String,
    bbox: Rect,
    baseline: f32,
}

struct SourceProof {
    media: Rect,
    numbers: Vec<Number>,
    shape: shape_proof::ShapeProof,
}

#[derive(Default)]
pub(super) struct Run {
    processor: once_cell::sync::OnceCell<OcrProcessor>,
    budget: budget::Budget,
    worker: parking_lot::Mutex<()>,
    budget_warning: std::sync::atomic::AtomicBool,
}

#[derive(Clone, Copy)]
pub(super) struct PagePolicy<'a> {
    pub(super) page_index_offset: usize,
    pub(super) accepted: bool,
    pub(super) scale_only: bool,
    pub(super) deferred_structure: bool,
    pub(super) layout: bool,
    pub(super) transformed: bool,
    pub(super) raster: Option<&'a crate::pdf::render::OcrRasterDpi>,
    pub(super) structured: bool,
    pub(super) builtin: bool,
}

pub(super) fn applicable(policy: PagePolicy<'_>) -> bool {
    policy.page_index_offset == 0
        && policy.accepted
        && policy.scale_only
        && !policy.deferred_structure
        && !policy.layout
        && !policy.transformed
        && policy.raster.is_some_and(|raster| !raster.type0_ink_loss)
        && !policy.structured
        && policy.builtin
}

pub(super) struct PageSource<'a> {
    pub(super) document: std::sync::Arc<PdfDocument>,
    pub(super) index: usize,
    pub(super) layer: &'a str,
    pub(super) raster: std::sync::Arc<Vec<u8>>,
    pub(super) dimensions: (u32, u32),
    pub(super) metadata: &'a crate::types::Metadata,
}

pub(super) struct PreparedPage<'a> {
    pub(super) policy: PagePolicy<'a>,
    pub(super) source: Option<PageSource<'a>>,
    pub(super) config: &'a ExtractionConfig,
    pub(super) run: &'a std::sync::Arc<Run>,
}

pub(super) struct AcceptedRead<'a> {
    pub(super) accepted: bool,
    pub(super) paragraphs: Option<&'a mut [PdfParagraph]>,
    pub(super) text: &'a mut String,
    pub(super) warnings: &'a mut Vec<crate::types::ProcessingWarning>,
}

impl PreparedPage<'_> {
    pub(super) async fn apply(
        mut self,
        read: AcceptedRead<'_>,
        tables: &[crate::types::Table],
        formulas: &[crate::types::Formula],
    ) -> bool {
        let Some(source) = self.source else { return false };
        let Ok(page) = u32::try_from(source.index.saturating_add(1)) else {
            return false;
        };
        self.policy.accepted = read.accepted;
        self.policy.structured = tables.iter().any(|table| table.page_number == page)
            || formulas.iter().any(|formula| formula.page == Some(page));
        let verified_frame = applicable(self.policy);
        self.policy.scale_only = true;
        if !applicable(self.policy) || !eligible(self.config, source.index) || !has_bounded_numbers(source.layer) {
            return false;
        }
        if model_directory(self.config).is_none() {
            missing_model_warning(read.warnings, source.index);
            return false;
        }
        if !self.run.budget.page_available(source.index, self.config) {
            budget_warning(self.run, read.warnings, source.index);
            return false;
        }
        let Some(paragraphs) = read.paragraphs else {
            unproved_read_warning(read.warnings, source.index);
            return false;
        };
        if !verified_frame && model_directory(self.config).is_some() {
            unproved_read_warning(read.warnings, source.index);
            return false;
        }
        recover(source, paragraphs, read.text, self.config, self.run, read.warnings).await
    }
}

fn has_bounded_numbers(layer: &str) -> bool {
    let count = layer
        .chars()
        .filter(|character| is_khmer_digit(*character))
        .take(MAX_NUMBER_GLYPHS + 1)
        .count();
    (1..=MAX_NUMBER_GLYPHS).contains(&count)
}

pub(super) async fn recover(
    source: PageSource<'_>,
    paragraphs: &mut [PdfParagraph],
    read: &mut String,
    config: &ExtractionConfig,
    run: &std::sync::Arc<Run>,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> bool {
    let Some((directory, frame)) = recovery_context(&source, paragraphs, read, config, warnings) else {
        return false;
    };
    if !run.budget.begin_page(source.index, config) {
        budget_warning(run, warnings, source.index);
        return false;
    }
    let index = source.index;
    let Some((numbers, media)) = recognize(Verification {
        document: source.document,
        index,
        raster: source.raster,
        directory,
        config: config.clone(),
        run: run.clone(),
    })
    .await
    else {
        if run.budget.limited() {
            budget_warning(run, warnings, index);
        } else if !cancelled(config) {
            unproved_read_warning(warnings, index);
        }
        return false;
    };
    if run.budget.limited() {
        budget_warning(run, warnings, index);
    }
    if cancelled(config) {
        return false;
    }
    let scale = (media.width / frame.0 as f32, media.height / frame.1 as f32);
    insert_into_read(numbers, paragraphs, read, scale, proof_limits(config).max_iterations)
}

fn recovery_context(
    source: &PageSource<'_>,
    paragraphs: &[PdfParagraph],
    read: &str,
    config: &ExtractionConfig,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> Option<(String, (u32, u32))> {
    if !eligible(config, source.index)
        || super::document::resolved_ocr_correction_degrees(source.metadata).is_some_and(|angle| angle != 0)
        || !has_bounded_numbers(source.layer)
    {
        return None;
    }
    crate::core::extractor::cacheability::invalidate_current();
    let Some(directory) = model_directory(config) else {
        missing_model_warning(warnings, source.index);
        return None;
    };
    if !super::document::ocr_paragraphs_plain_text(paragraphs)
        .split_whitespace()
        .eq(read.split_whitespace())
    {
        unproved_read_warning(warnings, source.index);
        return None;
    }
    let frame =
        super::document::resolved_ocr_layout_dimensions(source.metadata, source.dimensions.0, source.dimensions.1);
    if frame.0 == 0 || frame.1 == 0 {
        unproved_read_warning(warnings, source.index);
        return None;
    }
    Some((directory, frame))
}

pub(super) fn synchronize_plain_pages(
    document: &crate::types::internal::InternalDocument,
    recovered: &[bool],
    page_texts: &mut [String],
    format: &crate::OutputFormat,
) -> bool {
    if *format != crate::OutputFormat::Plain {
        return false;
    }
    let requested: std::collections::BTreeSet<u32> = recovered
        .iter()
        .enumerate()
        .filter(|(_, changed)| **changed)
        .filter_map(|(index, _)| u32::try_from(index + 1).ok())
        .collect();
    if requested.is_empty() {
        return false;
    }
    let mut rendered = crate::extraction::derive::render_plain_selected_pages(document, &requested);
    let mut changed = false;
    for ((index, read), recovered) in page_texts.iter_mut().enumerate().zip(recovered) {
        if *recovered
            && let Ok(page) = u32::try_from(index + 1)
            && let Some(rendered) = rendered.remove(&page)
            && *read != rendered
        {
            *read = rendered;
            changed = true;
        }
    }
    changed
}

fn source_numbers(
    document: &PdfDocument,
    index: usize,
    retained: usize,
    config: &ExtractionConfig,
) -> crate::Result<Preparation<SourceProof>> {
    let info = document.get_page_info(index).map_err(proof_error)?;
    let media = info.media_box;
    if info.rotation != 0 || media.x != 0.0 || media.y != 0.0 || info.crop_box.is_some_and(|crop| crop != media) {
        return Ok(Preparation::UnsupportedBeforeWork);
    }
    let limits = proof_limits(config);
    let budget = limits.max_content_size.saturating_sub(retained);
    Ok(
        match shape_proof::prepare_source_accounted(document, index, budget, limits.max_iterations)? {
            Preparation::Ready((numbers, shape)) => Preparation::Ready(SourceProof { media, numbers, shape }),
            Preparation::UnsupportedBeforeWork => Preparation::UnsupportedBeforeWork,
            Preparation::UnmeasuredDecline => Preparation::UnmeasuredDecline,
        },
    )
}

fn cancelled(config: &ExtractionConfig) -> bool {
    config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
}

struct Verification {
    document: std::sync::Arc<PdfDocument>,
    index: usize,
    raster: std::sync::Arc<Vec<u8>>,
    directory: String,
    config: ExtractionConfig,
    run: std::sync::Arc<Run>,
}

fn verify_page(work: Verification) -> crate::Result<Option<(Vec<Number>, Rect)>> {
    let Verification {
        document,
        index,
        raster,
        directory,
        config,
        run,
    } = work;
    let _serial = run.worker.lock();
    if cancelled(&config) {
        return Ok(None);
    }
    if !run.budget.crop_available(&config) {
        return Ok(None);
    }
    let Some(allowance) = run.budget.reserve_work(&config) else {
        return Ok(None);
    };
    let mut limits = proof_limits(&config);
    limits.max_content_size = allowance;
    let mut bounded = config.clone();
    bounded.security_limits = Some(limits);
    let proof = match source_numbers(&document, index, raster.capacity(), &bounded) {
        Ok(Preparation::Ready(proof)) => proof,
        Ok(Preparation::UnsupportedBeforeWork) => {
            run.budget.settle_work(allowance, 0);
            return Ok(None);
        }
        Ok(Preparation::UnmeasuredDecline) => {
            run.budget.retain_failed_work();
            return Ok(None);
        }
        Err(error) => {
            run.budget.retain_failed_work();
            return Err(error);
        }
    };
    if cancelled(&config) {
        return Ok(None);
    }
    let media = proof.media;
    match verify_source(raster, &proof, &directory, &bounded, &run, Some(allowance)) {
        Ok(numbers) => Ok(Some((numbers, media))),
        Err(error) => {
            run.budget.retain_failed_work();
            Err(error)
        }
    }
}

async fn recognize(work: Verification) -> Option<(Vec<Number>, Rect)> {
    let dispatcher = tracing::dispatcher::get_default(Clone::clone);
    let worker = crate::core::extractor::cacheability::blocking(move || {
        tracing::dispatcher::with_default(&dispatcher, || verify_page(work))
    });
    match tokio::task::spawn_blocking(worker).await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            tracing::warn!(%error, "Unable to verify native numeric source crops");
            None
        }
        Err(error) => {
            tracing::warn!(%error, "Native numeric source verification task failed");
            None
        }
    }
}

fn budget_warning(run: &Run, warnings: &mut Vec<crate::types::ProcessingWarning>, index: usize) {
    if !run.budget_warning.swap(true, std::sync::atomic::Ordering::AcqRel) {
        recovery_warning(warnings, index.saturating_add(1), "Native Khmer number verification reached its document budget. Remaining accepted OCR reads are retained; some numbers may be missing.".into());
    }
}

fn eligible(config: &ExtractionConfig, index: usize) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    let Ok(page) = u32::try_from(index.saturating_add(1)) else {
        return false;
    };
    super::document::native_preservation_is_automatic(config, page)
        && config.redaction.is_none()
        && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
        && matches!(
            config.output_format,
            crate::OutputFormat::Plain | crate::OutputFormat::Markdown
        )
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.effective_tesseract_language()
                .iter()
                .map(String::as_str)
                .eq(["eng"])
                && ocr.tesseract_config.is_none()
                && ocr.element_config.is_none()
                && ocr.backend_options.is_none()
                && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
        })
}

fn model_directory(config: &ExtractionConfig) -> Option<String> {
    let ocr = config.ocr.as_ref()?;
    let directory = existing_tessdata_dir_for(
        &ocr.effective_tesseract_language(),
        ocr.tessdata_path.as_deref(),
        &TessdataEnv::from_process(),
    )?;
    // An auxiliary model must accompany the selected primary model; never search or download it. ~keep
    std::path::Path::new(&directory)
        .join("khm.traineddata")
        .is_file()
        .then_some(directory)
}

pub(super) fn prepare_read(
    mut primary: crate::core::config::OcrConfig,
    config: &ExtractionConfig,
    page: usize,
    page_index_offset: usize,
    layer: Option<&str>,
    run: &Run,
) -> (crate::core::config::OcrConfig, bool) {
    if page_index_offset != 0 || !eligible(config, page) || !layer.is_some_and(has_bounded_numbers) {
        return (primary, false);
    }
    crate::core::extractor::cacheability::invalidate_current();
    if model_directory(config).is_none() {
        return (primary, false);
    }
    if !run.budget.page_available(page, config) {
        return (primary, false);
    }
    let options = primary.backend_options.get_or_insert_with(|| serde_json::json!({}));
    let Some(options) = options.as_object_mut() else {
        return (primary, false);
    };
    // A cached read cannot certify whether deskew changed this raster. ~keep
    options.insert("use_cache".into(), false.into());
    (primary, true)
}

fn missing_model_warning(warnings: &mut Vec<crate::types::ProcessingWarning>, index: usize) {
    let page = index.saturating_add(1);
    let message = format!(
        "The native text on PDF page {page} contains Khmer-script numbers, but no khm.traineddata accompanies \
         the selected OCR model. Those numbers cannot be verified and may be missing from the OCR text."
    );
    recovery_warning(warnings, page, message);
}

fn unproved_read_warning(warnings: &mut Vec<crate::types::ProcessingWarning>, index: usize) {
    let page = index.saturating_add(1);
    recovery_warning(
        warnings,
        page,
        format!(
            "Native Khmer numbers on PDF page {page} were not recovered because the accepted OCR text and geometry \
         could not be bound to the original raster within the numeric recovery proof limits. The OCR read is retained."
        ),
    );
}

fn recovery_warning(warnings: &mut Vec<crate::types::ProcessingWarning>, page: usize, message: String) {
    let warning = crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Owned(message),
    };
    if !warnings
        .iter()
        .any(|existing| existing.source == warning.source && existing.message == warning.message)
    {
        tracing::warn!(page, script = "Khmer", message = %warning.message);
        crate::core::diagnostics::push_warning_deduped(warnings, warning);
    }
}

fn proof_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::XbergError {
    crate::XbergError::Ocr {
        message: "Unable to recognize a native number in its painted source crop".into(),
        source: Some(Box::new(error)),
    }
}

fn candidates(chars: &[TextChar]) -> Vec<Number> {
    let mut numbers: Vec<Number> = Vec::new();
    let mut adjacent = false;
    for character in chars {
        if !is_khmer_digit(character.char) || character.rotation_degrees != 0.0 {
            adjacent = false;
            continue;
        }
        if adjacent
            && let Some(last) = numbers.last_mut()
            && last.baseline == character.origin_y
            && character.bbox.x >= last.bbox.x
            && character.bbox.x <= last.bbox.x + last.bbox.width + character.advance_width
        {
            last.text.push(character.char);
            last.bbox = last.bbox.union(&character.bbox);
        } else {
            numbers.push(Number {
                text: character.char.to_string(),
                bbox: character.bbox,
                baseline: character.origin_y,
            });
        }
        adjacent = true;
    }
    numbers
}

fn verify_source(
    encoded: std::sync::Arc<Vec<u8>>,
    proof: &SourceProof,
    directory: &str,
    config: &ExtractionConfig,
    run: &Run,
    reserved: Option<usize>,
) -> crate::Result<Vec<Number>> {
    if cancelled(config) || !run.budget.crop_available(config) {
        return Ok(Vec::new());
    }
    let SourceProof { media, numbers, shape } = proof;
    let mut limits = proof_limits(config);
    limits.max_content_size = limits
        .max_content_size
        .min(run.budget.remaining(config).saturating_add(reserved.unwrap_or(0)));
    if numbers.is_empty()
        || numbers.iter().map(|number| number.text.chars().count()).sum::<usize>() > MAX_NUMBER_GLYPHS
        || numbers.len() > limits.max_iterations
    {
        return Ok(Vec::new());
    }
    let raster = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        &encoded,
        &limits,
        u64::try_from(encoded.capacity().saturating_add(shape.retained_bytes())).unwrap_or(u64::MAX),
    )?;
    let raster = image::DynamicImage::ImageRgb8(raster);
    let measured = encoded
        .capacity()
        .saturating_add(shape.retained_bytes())
        .saturating_add(raster.as_bytes().len());
    if cancelled(config) || (reserved.is_none() && !run.budget.charge(measured, config)) {
        return Ok(Vec::new());
    }
    let processor = run
        .processor
        .get_or_try_init(|| OcrProcessor::new(None).map_err(proof_error))?;
    let source_retained = encoded.capacity().saturating_add(raster.as_bytes().len());
    let retained = source_retained.saturating_add(shape.retained_bytes());
    let mut recognition = recognition_config(
        directory,
        config,
        f64::from(raster.width()) / f64::from(media.width) * f64::from(POINTS_PER_INCH),
    );
    let budget = CropBudget {
        limits: &limits,
        retained,
        cancel_token: config.cancel_token.as_ref(),
    };
    let numbers = verify_numbers(proof, &raster, processor, &mut recognition, &budget, (config, run))?;
    if cancelled(config) {
        return Ok(Vec::new());
    }
    if reserved.is_some_and(|allowance| !run.budget.settle_work(allowance, measured)) {
        return Ok(Vec::new());
    }
    Ok(numbers)
}

fn verify_numbers(
    proof: &SourceProof,
    raster: &image::DynamicImage,
    processor: &OcrProcessor,
    recognition: &mut TesseractConfig,
    budget: &CropBudget<'_>,
    context: (&ExtractionConfig, &Run),
) -> crate::Result<Vec<Number>> {
    let (config, run) = context;
    let limits = budget.limits;
    let source_retained = budget.retained.saturating_sub(proof.shape.retained_bytes());
    let mut verified = Vec::new();
    for (index, number) in proof.numbers.iter().enumerate() {
        if cancelled(config) || !run.budget.take_crop(config) {
            break;
        }
        if !proof.shape.corroborates(
            index,
            raster,
            proof.media,
            number,
            limits.max_content_size.saturating_sub(source_retained),
        )? {
            continue;
        }
        if cancelled(config) {
            break;
        }
        if let Some(read) = recognize_crop(raster, proof.media, number.bbox, processor, recognition, budget)?
            && recognized_number_matches(&read.content, &number.text)
        {
            verified.push(number.clone());
        }
    }
    Ok(verified)
}

fn recognized_number_matches(read: &str, number: &str) -> bool {
    read.chars()
        .filter(|character| !character.is_whitespace() && *character != '\u{200b}')
        .eq(number.chars())
}

struct CropBudget<'a> {
    limits: &'a crate::extractors::security::SecurityLimits,
    retained: usize,
    cancel_token: Option<&'a crate::cancellation::CancellationToken>,
}

fn recognize_crop(
    raster: &image::DynamicImage,
    media: Rect,
    bbox: Rect,
    processor: &OcrProcessor,
    recognition: &mut TesseractConfig,
    budget: &CropBudget<'_>,
) -> crate::Result<Option<crate::types::OcrExtractionResult>> {
    if budget
        .cancel_token
        .is_some_and(crate::cancellation::CancellationToken::is_cancelled)
    {
        return Ok(None);
    }
    let limits = budget.limits;
    let retained = budget.retained;
    let Some(crop) = crop_number(raster, media, bbox, limits.max_content_size.saturating_sub(retained)) else {
        return Ok(None);
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    crop.write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(proof_error)?;
    drop(crop);
    let Some(remaining) = limits
        .max_content_size
        .checked_sub(retained.saturating_add(encoded.get_ref().capacity()))
    else {
        return Ok(None);
    };
    let mut crop_limits = limits.clone();
    crop_limits.max_content_size = remaining;
    recognition.security_limits = Some(crop_limits);
    if budget
        .cancel_token
        .is_some_and(crate::cancellation::CancellationToken::is_cancelled)
    {
        return Ok(None);
    }
    processor
        .process_image_with_policy(
            encoded.get_ref(),
            recognition,
            None,
            ImageOcrPolicy::ProvisionedModelsOnly,
        )
        .map(Some)
        .map_err(proof_error)
}

fn proof_limits(config: &ExtractionConfig) -> crate::extractors::security::SecurityLimits {
    let mut limits = config.security_limits.clone().unwrap_or_default();
    if let Some(nested) = config.ocr.as_ref().and_then(|ocr| ocr.security_limits.as_ref()) {
        limits.max_content_size = limits.max_content_size.min(nested.max_content_size);
        limits.max_iterations = limits.max_iterations.min(nested.max_iterations);
    }
    limits
}

fn recognition_config(directory: &str, config: &ExtractionConfig, dpi: f64) -> TesseractConfig {
    TesseractConfig {
        language: "khm".into(),
        tessdata_path: Some(directory.into()),
        psm: 8,
        output_format: "text".into(),
        use_cache: false,
        enable_table_detection: false,
        source_dpi: Some(dpi),
        security_limits: Some(proof_limits(config)),
        ..Default::default()
    }
}

fn crop_number(raster: &image::DynamicImage, media: Rect, bbox: Rect, budget: usize) -> Option<image::DynamicImage> {
    let sx = raster.width() as f32 / media.width;
    let sy = raster.height() as f32 / media.height;
    let bounds = [
        bbox.x * sx,
        (media.height - bbox.y - bbox.height) * sy,
        bbox.width * sx,
        bbox.height * sy,
    ];
    if bounds.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return None;
    }
    let x = bounds[0].floor() as u32;
    let y = bounds[1].floor() as u32;
    let width = (bounds[0] + bounds[2]).ceil() as u32 - x;
    let height = (bounds[1] + bounds[3]).ceil() as u32 - y;
    if width == 0
        || height == 0
        || x.checked_add(width)? > raster.width()
        || y.checked_add(height)? > raster.height()
        || u64::from(width) * u64::from(height) > MAX_CROP_PIXELS
    {
        return None;
    }
    let padded_pixels = u64::from(width + 2 * CROP_BORDER) * u64::from(height + 2 * CROP_BORDER);
    if 3 * (u64::from(width) * u64::from(height) + padded_pixels) > budget as u64 {
        return None;
    }
    let crop = raster.crop_imm(x, y, width, height).into_rgb8();
    let mut padded =
        image::RgbImage::from_pixel(width + 2 * CROP_BORDER, height + 2 * CROP_BORDER, image::Rgb([255; 3]));
    image::imageops::replace(&mut padded, &crop, i64::from(CROP_BORDER), i64::from(CROP_BORDER));
    Some(image::DynamicImage::ImageRgb8(padded))
}

#[cfg(test)]
fn insert(numbers: Vec<Number>, paragraphs: &mut [PdfParagraph], scale: (f32, f32), max_work: usize) -> bool {
    let mut read = super::document::ocr_paragraphs_plain_text(paragraphs);
    insert_into_read(numbers, paragraphs, &mut read, scale, max_work)
}

fn insert_into_read(
    numbers: Vec<Number>,
    paragraphs: &mut [PdfParagraph],
    read: &mut String,
    scale: (f32, f32),
    max_work: usize,
) -> bool {
    if !scale.0.is_finite()
        || !scale.1.is_finite()
        || scale.0 <= 0.0
        || scale.1 <= 0.0
        || paragraphs
            .iter()
            .any(|paragraph| !located_text_covers_paragraph(paragraph))
    {
        return false;
    }
    let lines = paragraphs.iter().map(|paragraph| paragraph.lines.len()).sum::<usize>();
    if !bounded_unique_crops(&numbers, lines, max_work) {
        return false;
    }
    let planned = numbers
        .into_iter()
        .filter_map(|number| {
            let bbox = Rect::new(
                number.bbox.x / scale.0,
                number.bbox.y / scale.1,
                number.bbox.width / scale.0,
                number.bbox.height / scale.1,
            );
            let mut owners =
                paragraphs.iter().enumerate().flat_map(|(paragraph, value)| {
                    value.lines.iter().enumerate().filter_map(move |(line, value)| {
                        line_geometrically_owns(value, bbox).then_some((paragraph, line))
                    })
                });
            let (paragraph, line) = owners.next()?;
            if owners.next().is_some() || !line_accepts_number(&paragraphs[paragraph].lines[line], bbox) {
                return None;
            }
            Some((paragraph, line, number, bbox))
        })
        .collect::<Vec<_>>();
    let mut changed = false;
    for (paragraph, line, number, bbox) in planned {
        let preceding = paragraphs[..paragraph]
            .iter()
            .flat_map(|paragraph| &paragraph.lines)
            .flat_map(|line| &line.segments)
            .map(|segment| segment.text.split_whitespace().count())
            .sum::<usize>();
        let paragraph = &mut paragraphs[paragraph];
        let position = paragraph.lines[line]
            .segments
            .partition_point(|existing| existing.x <= bbox.x);
        if !splice_paragraph_number(paragraph, line, position, &number.text, (read, preceding)) {
            continue;
        }
        let line = &mut paragraph.lines[line];
        let segment = numeric_segment(number, bbox, scale.1, line.dominant_font_size);
        line.segments.insert(position, segment);
        paragraph.word_count = PdfParagraph::compute_word_count(&paragraph.text, &paragraph.lines);
        changed = true;
    }
    changed
}

fn splice_paragraph_number(
    paragraph: &mut PdfParagraph,
    line: usize,
    position: usize,
    number: &str,
    (read, page_words): (&mut String, usize),
) -> bool {
    let preceding = paragraph.lines[..line]
        .iter()
        .flat_map(|line| &line.segments)
        .chain(paragraph.lines[line].segments[..position].iter())
        .map(|segment| segment.text.split_whitespace().count())
        .sum::<usize>();
    let before_word = paragraph.lines[line].segments[position..]
        .iter()
        .any(|segment| !segment.text.trim().is_empty());
    let Some(page_offset) = number_offset(read, page_words + preceding, before_word) else {
        return false;
    };
    let local_offset = number_offset(&paragraph.text, preceding, before_word);
    if !paragraph.text.is_empty() && local_offset.is_none() {
        return false;
    }
    let addition = if before_word {
        format!("{number} ")
    } else {
        format!(" {number}")
    };
    if let Some(offset) = local_offset {
        paragraph.text.insert_str(offset, &addition);
    }
    read.insert_str(page_offset, &addition);
    true
}

fn number_offset(text: &str, preceding: usize, before_word: bool) -> Option<usize> {
    let index = if before_word {
        preceding
    } else {
        preceding.checked_sub(1)?
    };
    let range = word_byte_range(text, index)?;
    Some(if before_word { range.start } else { range.end })
}

fn word_byte_range(text: &str, index: usize) -> Option<std::ops::Range<usize>> {
    let mut offset = 0;
    text.split_inclusive(char::is_whitespace)
        .filter_map(|part| {
            let start = offset;
            offset += part.len();
            let word = part.trim_end_matches(char::is_whitespace);
            (!word.is_empty()).then_some(start..start + word.len())
        })
        .nth(index)
}

fn numeric_segment(number: Number, bbox: Rect, scale: f32, font_size: f32) -> crate::pdf::hierarchy::SegmentData {
    crate::pdf::hierarchy::SegmentData {
        text: number.text,
        x: bbox.x,
        y: bbox.y,
        width: bbox.width,
        height: bbox.height,
        baseline_y: number.baseline / scale,
        font_size,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn bounded_unique_crops(numbers: &[Number], lines: usize, max_work: usize) -> bool {
    numbers.len() <= MAX_NUMBER_GLYPHS
        && numbers
            .len()
            .checked_mul(lines.saturating_add(numbers.len()))
            .is_some_and(|work| work <= max_work)
        && numbers.iter().enumerate().all(|(index, number)| {
            numbers[..index]
                .iter()
                .all(|other| !number.bbox.intersects(&other.bbox))
        })
}

fn located_text_covers_paragraph(paragraph: &PdfParagraph) -> bool {
    paragraph.text.is_empty()
        || paragraph.text.split_whitespace().eq(paragraph
            .lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .flat_map(|segment| segment.text.split_whitespace()))
}

fn line_accepts_number(line: &PdfLine, bbox: Rect) -> bool {
    if !line.dominant_font_size.is_finite() || line.dominant_font_size <= 0.0 {
        return false;
    }
    if !line.segments.windows(2).all(|pair| pair[0].x <= pair[1].x) {
        return false;
    }
    if line.segments.iter().any(|segment| {
        segment.text.chars().any(char::is_numeric)
            || (segment.text.split_whitespace().nth(1).is_some()
                && segment.x < bbox.x + bbox.width
                && bbox.x < segment.x + segment.width)
    }) {
        return false;
    }
    true
}

fn line_geometrically_owns(line: &PdfLine, bbox: Rect) -> bool {
    let Some(first) = line.segments.first() else {
        return false;
    };
    let bounds = line.segments.iter().fold(
        Rect::new(first.x, first.y, first.width, first.height),
        |bounds, segment| bounds.union(&Rect::new(segment.x, segment.y, segment.width, segment.height)),
    );
    let line_box = |rect: Rect| super::document::LineBox {
        left: rect.x,
        right: rect.x + rect.width,
        top: -rect.y - rect.height,
        bottom: -rect.y,
    };
    line_box(bounds).owns(&line_box(bbox))
}

#[cfg(test)]
mod page_ranges_tests;

#[cfg(test)]
mod tests;
