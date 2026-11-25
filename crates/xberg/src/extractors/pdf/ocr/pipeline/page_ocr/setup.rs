use super::super::*;
use super::*;
use crate::plugins::registry::get_ocr_backend_registry;

/// What one prepared run is: a document-level backend already returned the whole answer, or the
/// phase context every page batch runs through.
pub(super) enum PageOcrStart<'a> {
    DocumentLevel(Box<PageOcrOutput>),
    Pages(Box<PageOcrContext<'a>>),
}

/// Resolve the backend, the render plan and every accumulator the page batches write, or hand
/// back the document-level answer when the backend read the whole document itself.
#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare<'a>(
    content: Option<&'a [u8]>,
    images: Option<&'a [image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&'a [crate::layout::DetectionResult]>,
    config: &'a ExtractionConfig,
    path: Option<&'a std::path::Path>,
    page_rotation_override: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'a>>,
) -> OcrRunResult<PageOcrStart<'a>> {
    let xobject_document = detached_page.map(|source| source.document);
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let stored_scan = detached_page.and_then(|source| source.stored_scan);
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let has_stored_scan = stored_scan.is_some();
    #[cfg(not(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let has_stored_scan = false;
    #[cfg(feature = "layout-detection")]
    let has_layout_detections = layout_detections.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let has_layout_detections = false;

    // Re-seed the built-in backends if a prior `clear_ocr_backends()` call (a real user API,
    // also exercised by binding e2e suites sharing this process) left the registry without a
    // usable default. Cheap when the default is already present -- see
    // `ensure_ocr_backends_initialized`'s own doc comment. ~keep
    crate::plugins::ensure_ocr_backends_initialized();

    // Same slice as `ocr_points_per_pixel`'s suppression above: the only two readers of
    // `points_per_pixel_override` are compiled out, but every caller still passes it. ~keep
    #[cfg(all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")))]
    let _ = points_per_pixel_override;

    let resolved_ocr_config = pre_page_ocr_config(config);
    let base_ocr_config = resolved_ocr_config.as_ref();

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let can_retry_orientation;
    #[cfg(feature = "pdf")]
    let no_admitted_images = false;
    #[cfg(not(feature = "pdf"))]
    let no_admitted_images = false;
    #[cfg(all(feature = "pdf", not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))))]
    let can_prove_no_admitted_images = false;
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let can_prove_no_admitted_images;
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let forced_layout_frames: Option<&[bool]> = {
        #[cfg(feature = "pdf")]
        {
            page_ocr_hints
                .as_ref()
                .filter(|_| config.force_ocr && has_layout_detections && base_ocr_config.auto_rotate.is_none())
                .and_then(|hints| hints.layout_rasters_in_page_frame.as_deref())
        }
        #[cfg(not(feature = "pdf"))]
        {
            None
        }
    };
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let builtin_tesseract;
    let backend = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        let backend = registry.get(&base_ocr_config.backend)?;
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        if has_stored_scan && !registry.is_builtin_tesseract(&backend) {
            return Err(
                crate::XbergError::validation("Stored JPEG OCR requires the built-in Tesseract backend").into(),
            );
        }
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        {
            can_prove_no_admitted_images = detached_page.is_some()
                && images.is_some_and(|images| images.len() == 1)
                && registry.is_builtin_tesseract(&backend);
        }
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        {
            builtin_tesseract = registry.is_builtin_tesseract(&backend);
            can_retry_orientation = !has_layout_detections && builtin_tesseract;
        }
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        if !builtin_tesseract
            && page_ocr_hints
                .as_ref()
                .is_some_and(|hints| hints.sparse_glyph_trial.is_some())
        {
            return Err(sparse_glyph_read::declined());
        }
        backend
    };
    // Only `ConfidenceSemantics::Legibility` backends report a `mean_text_conf` whose scale
    // means legibility; anything else (`Uncalibrated`, `None`) must never be turned into a
    // normalized confidence, or a page-rejection gate downstream would compare it against an
    // absolute floor it was never calibrated against (see `resolve_confidence_semantics`).
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let tall_source_candidate =
        !has_layout_detections && !has_stored_scan && get_ocr_backend_registry().read().is_builtin_tesseract(&backend);
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let tall_source_unsupported = tall_source_candidate
        .then(|| super::tall_source::unsupported_reason_for(config, base_ocr_config, backend.as_ref()))
        .flatten();
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let try_tall_source = tall_source_candidate && tall_source_unsupported.is_none();
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let tall_ordered_primary = None;
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let force_khmer_retry = content.is_some()
        && detached_page.is_none()
        && !has_layout_detections
        && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
        && forced_khmer_retry_allowed(config, base_ocr_config);
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let forced_khmer_supplements = ahash::AHashMap::new();
    let backend_confidence_semantics = backend.confidence_semantics();
    // Owned up front: the per-page summaries below outlive the borrow-heavy OCR loop, and
    // the backend that actually produced a page is part of what `ocr_confidence` reports. ~keep
    let backend_name = backend.name().to_string();
    let backend_confidence_scale = match backend_confidence_semantics {
        crate::plugins::ConfidenceSemantics::Legibility { scale_max } if scale_max > 0.0 => Some(scale_max),
        _ => None,
    };
    // Only meaningful with the `pdf` feature: without it there is no `/Rotate` to correct for
    // (`page_rotation_degrees` is always `0` below), so nothing reads this in that build.
    #[cfg(feature = "pdf")]
    let orientation_handling = backend.page_orientation_handling();

    let structured_ocr_config;
    let ocr_config = {
        let cfg = ensure_elements_enabled(base_ocr_config);
        #[cfg(all(feature = "ocr", feature = "layout-detection"))]
        let cfg = if layout_detections.is_some() || backend.emits_structured_markdown() {
            inject_layout_config_to_backend(&cfg, config)
        } else {
            cfg
        };
        structured_ocr_config = cfg;
        &structured_ocr_config
    };

    #[cfg(not(feature = "layout-detection"))]
    let supports_doc = backend.supports_document_processing();
    #[cfg(feature = "layout-detection")]
    let supports_doc = backend.supports_document_processing() && layout_detections.is_none();

    #[cfg(feature = "pdf")]
    let page_margins = crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config));
    #[cfg(not(feature = "pdf"))]
    let use_document_processing = supports_doc && path.is_some();
    #[cfg(feature = "pdf")]
    let use_document_processing = should_use_document_processing(supports_doc, path.is_some(), page_margins);

    if let Some(doc_path) = path
        && use_document_processing
    {
        tracing::debug!(backend = %ocr_config.backend, "Using document-level OCR processing");
        let result = backend.process_document(doc_path, ocr_config).await?;
        let preprocessing = result
            .metadata
            .image_preprocessing
            .clone()
            .map(|metadata| ahash::AHashMap::from([(1, metadata)]))
            .unwrap_or_default();
        let mean_conf = backend_confidence_scale.and_then(|scale_max| {
            result
                .metadata
                .additional
                .get("mean_text_conf")
                .and_then(|v| v.as_f64())
                .map(|v| v / scale_max)
        });
        let backend_elements = result.ocr_elements.unwrap_or_default();
        let ocr_elements = filter_public_ocr_elements(&backend_elements, base_ocr_config);
        let llm_usage = result.llm_usage.unwrap_or_default();
        let formulas = result.formulas;
        let page_texts = if let Some(pages) = result.pages {
            pages.into_iter().map(|p| p.content).collect()
        } else {
            vec![result.content.clone()]
        };
        // #1575: this route returns `None` for the structured document, so a backend
        // warning (e.g. a document-processing capability notice) had nowhere to land and
        // was silently dropped. Only builds a document when there is a warning to carry --
        // `None` otherwise, matching this branch's pre-#1575 output byte-for-byte. Restricted
        // to the single-page case: `select_pdf_document`'s `ExtractionMethod::Ocr` arm uses
        // `ocr_doc` verbatim as *the* document when it is `Some`, and for `page_texts.len() >
        // 1` a document built from `result.content` alone would not reflect this route's own
        // per-page split (`page_texts`, above) -- risking a content regression worse than the
        // warning loss this fixes. `None` still falls through to the caller's own
        // `flat_pdf_document(text, ..)`, built from the correctly page-joined `text`, so the
        // warning is the only thing still missing for a multi-page document-level result.
        #[cfg(feature = "pdf")]
        let ocr_doc = if result.processing_warnings.is_empty() || page_texts.len() > 1 {
            None
        } else {
            let mut doc = super::document::flat_ocr_page_document(&result.content);
            doc.processing_warnings = result.processing_warnings.clone();
            Some(doc)
        };
        #[cfg(not(feature = "pdf"))]
        let ocr_doc = None;
        return Ok(PageOcrStart::DocumentLevel(Box::new((
            result.content,
            mean_conf,
            Vec::new(),
            ocr_elements,
            ocr_doc,
            llm_usage,
            page_texts,
            None,
            formulas,
            // The document-level backend route returns whole-document content, not per-page
            // paragraph geometry, so there is nothing for the document-global heuristic to
            // cluster over here.
            Vec::new(),
            preprocessing,
            // The backend judged the document as a whole, so there is no per-page confidence
            // to report. An empty map keeps every page's `ocr_confidence` absent rather than
            // pinning a document-wide number onto page 1 and leaving the rest bare (#1568). ~keep
            ahash::AHashMap::new(),
            // Same reasoning: this route never calls `accept_or_reject_ocr_page` per page.
            Vec::new(),
            OcrReadEvidence::default(),
        ))));
    }
    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let preprocessing_render_dpi = configured_preprocessing_render_dpi(
        config,
        base_ocr_config,
        get_ocr_backend_registry().read().is_builtin_tesseract(&backend),
    );
    #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let preprocessing_render_dpi: Option<i32> = None;
    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let captured_rasters: Vec<crate::types::ExtractedImage> = Vec::new();
    let preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata> = ahash::AHashMap::new();

    #[cfg(feature = "pdf")]
    let lazy_pdf_render_state = if !use_document_processing && images.is_none() {
        content.map(open_pdf_for_full_ocr).transpose()?
    } else {
        None
    };
    #[cfg(feature = "pdf")]
    let lazy_pdf_page_count = lazy_pdf_render_state
        .as_ref()
        .map_or(0, |(_, page_count, _)| *page_count);
    #[cfg(not(feature = "pdf"))]
    let lazy_pdf_page_count = 0;

    #[allow(unused_imports)]
    use std::sync::Arc;

    let configured_batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    let batch_size = if has_stored_scan {
        1
    } else if images.is_none() {
        adapt_batch_size_to_memory(
            configured_batch_size,
            content.map(|b| b.len()).unwrap_or(0),
            preprocessing_render_reservation(
                preprocessing_render_dpi,
                config
                    .security_limits
                    .as_ref()
                    .unwrap_or(&Default::default())
                    .max_content_size,
            ),
        )
        .await
    } else {
        configured_batch_size
    };

    if batch_size < configured_batch_size {
        tracing::info!(
            configured = configured_batch_size,
            adapted = batch_size,
            "Reduced OCR batch size to fit available memory"
        );
    }

    let mut ocr_config_owned = ocr_config_for_page_extraction(ocr_config);
    ocr_config_owned.acceleration = config.acceleration.clone();
    // Preserve OCR-specific settings while satisfying both configured byte caps (GH#1651). ~keep
    let mut limits = ocr_config_owned
        .security_limits
        .clone()
        .or_else(|| config.security_limits.clone())
        .unwrap_or_default();
    limits.max_content_size = pipeline_xobject_content_limit(config, &ocr_config_owned);
    ocr_config_owned.security_limits = Some(limits);
    let total_pages = if has_stored_scan {
        1
    } else if let Some(imgs) = images {
        imgs.len()
    } else {
        lazy_pdf_page_count
    };
    #[cfg(feature = "pdf")]
    let progress_total_pages = xobject_document
        .and_then(|document| document.page_count().ok())
        .unwrap_or(page_index_offset + total_pages);
    #[cfg(not(feature = "pdf"))]
    let progress_total_pages = page_index_offset + total_pages;

    // The layout-detection route hands in pre-rendered `images`, so `lazy_pdf_render_state`
    // above is never populated (it's gated on `images.is_none()`) and the page-rotation
    // lookups below would always fall back to their `unwrap_or(0)` default. When the
    // original PDF bytes are still available via `content`, read the same lightweight
    // `/Rotate` lookup the full-page and per-page routes use so this route's calls to
    // `ocr_config_with_page_rotation_hint` get a real hint too (see #530 / 972d2269f7).
    // `content` is `None` on some image-only callers (e.g. the image extractor's own OCR
    // path with no source PDF); there is genuinely no rotation to read in that case.
    // The same open also yields each page's MediaBox dimensions, which are what turns a
    // pre-rendered raster's pixel width into the `source_dpi` hint this route used to derive
    // only when it rendered the pages itself (#1753).
    #[cfg(feature = "pdf")]
    let external_image_page_geometry: Option<PreRenderedPageGeometry> = if images.is_some() {
        content.map(|c| pre_rendered_page_geometry(c, total_pages))
    } else {
        None
    };

    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let leading_khmer_rows = ahash::AHashMap::new();
    let page_texts = vec![String::new(); total_pages];
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let paint_retention = ahash::AHashMap::new();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let background_noise = ahash::AHashMap::<usize, super::symbol_noise_background::Evidence>::new();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let certified_background = content.is_some_and(super::symbol_noise_background::certified_source);
    #[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let certified_background = certified_background && !super::symbol_noise_lines::texture::replay_disabled();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let paint_work = super::hidden_layer::PreviewWorkBudget::new(config);
    // Which pages the quality gate rejected. Kept separately from `page_texts` because an
    // empty string is ambiguous -- a blank page produces one too -- and the structured
    // paragraphs of a rejected page must be dropped along with its text.
    let rejected_pages = vec![false; total_pages];
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let owned_scan_rows = None;
    let all_page_paragraphs: Vec<Option<Vec<crate::pdf::structure::types::PdfParagraph>>> = vec![None; total_pages];
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let column_lanes_by_page = (0..total_pages).map(|_| None).collect::<Vec<_>>();
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let ordered_layout_pages = vec![false; total_pages];
    // The factor from each page's OCR raster to points, recorded beside its paragraphs for the
    // layer merge. ~keep
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let page_ocr_scales: Vec<Option<(f32, f32)>> = vec![None; total_pages];
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let detached_page_scales = vec![None; total_pages];
    #[cfg(feature = "pdf")]
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let paragraph_rasters = vec![None; total_pages];
    let public_ocr_pages = ahash::AHashMap::new();
    // Written only by the two OCR-paragraph assembly blocks below (`layout-detection`
    // *with* `ocr`/`ocr-wasm`, or `not(layout-detection)`); still read unconditionally by
    // the document-global heuristic. `layout-detection` without either OCR frontend (the
    // `formula-recognition,pdf` CI leg) leaves it write-free. ~keep
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let ocr_page_heights = vec![0.0_f32; total_pages];
    // Written by the same two assembly blocks as `ocr_page_heights`, from each page backend
    // report that detected tables claimed all of its text (#1571).
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let pages_text_claimed_by_tables = vec![false; total_pages];
    // The embedded-image retry text of each page that adopted it, without the lines the retry's
    // tables carry (#2014). The page text keeps every recovered line, as a backend's own page
    // text does, so the quality checks and the pipeline score see the tables' text; the page's
    // paragraphs are built from this instead, beside the tables printed on their own.
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let retry_texts_outside_tables: Vec<Option<String>> = vec![None; total_pages];
    #[allow(unused_mut)]
    let mut collected_tables: Vec<crate::types::Table> = Vec::new();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let caption = None;
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let capture_caption =
        detached_page.is_some() && !has_layout_detections && super::caption_conversion::eligible(config);
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let detached_table_pair = false;
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    let collected_table_grids = Vec::new();
    let all_ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
    let accumulated_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
    let accumulated_formulas: Vec<crate::types::Formula> = Vec::new();
    let conf_sum: f64 = 0.0;
    let conf_count: usize = 0;
    // Per-page OCR confidence summaries, keyed by 1-based document page number (#1568).
    // Only pages this route actually OCR'd get an entry, so an absent key downstream means
    // "not OCR'd" rather than "OCR'd with nothing to report". ~keep
    let ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence> = ahash::AHashMap::new();
    // Warnings from the force_ocr image-XObject fallback (#1355): a page rendered
    // blank by xberg_native_pdf but carrying image XObjects the renderer couldn't paint.
    #[cfg(feature = "pdf")]
    let image_fallback_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    #[cfg(feature = "pdf")]
    let xobject_recovery_budget = crate::extractors::security::SecurityBudget::from_config(config);

    // #1444: a backend failure on one page used to propagate with `?`, aborting the whole
    // extraction and — crucially — never reaching the image-XObject fallback below, which is
    // exactly the recovery such a page needs. Failures are captured per page instead: the
    // page degrades to empty text (so the blank-page fallback fires for it), the failure is
    // surfaced as a warning, and only a document where *every* page failed *and* nothing was
    // recovered still returns an error.
    let page_backend_errors = Vec::new();
    let page_failure_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    let pending_page_failure_warnings = Vec::new();
    let page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
    // #1673: page numbers (1-based) whose embedded-image retry ran (attempted at least one
    // image) but the backend returned successfully with empty content on every one of them --
    // the shape every candle VLM backend takes when it silently produces nothing
    // (`tracing::warn!("... output is empty")`, `Ok` rather than `Err`). Distinguishes, on the
    // failure paths below, "the retry never ran" from "the retry ran and recovered nothing".
    let pages_with_empty_xobject_retry: Vec<u32> = Vec::new();
    // A retry can recover only structured payload (most importantly a table), leaving the flat
    // page text empty. That still prevents a wholesale failure and keeps the structured page. ~keep
    let pages_with_recovered_xobject_payload = vec![false; total_pages];
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let complete_native_words = false;
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let native_ink_owned = false;
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let native_placed_words = std::collections::HashSet::new();
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let matched_layer_occurrences = None;
    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let tall_source_used = false;

    #[cfg(feature = "pdf")]
    let margin_filter_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

    // #1575: the per-page backend result's own warnings (e.g. Tesseract's dictionary-filter
    // removal notice) and its `additional` metadata (psm, language, dict-invalid ratio) were
    // read for local bookkeeping but never forwarded to the caller -- unlike
    // `extract_mixed_ocr_native`, which already merges both via `dedup_extend_warnings`
    // (pipeline.rs:719-723). Collected here and folded into the returned document below,
    // mirroring that sibling route. Metadata is captured once, from the first page whose
    // backend result carries any -- psm/language are constant for the whole OCR run, and a
    // single representative page is a strict improvement over reporting none at all. ~keep
    #[cfg(feature = "pdf")]
    let render_warnings = Vec::new();
    #[cfg(feature = "pdf")]
    let raster_dpis = if images.is_some() {
        page_ocr_hints
            .as_ref()
            .and_then(|hints| hints.raster_dpis.as_deref())
            .cloned()
            .unwrap_or_default()
    } else {
        super::rendering::PageRasterDpis::new()
    };
    #[cfg_attr(
        not(feature = "layout-detection"),
        allow(unused_mut, reason = "only the TATR availability warning writes here")
    )]
    let mut backend_page_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let detached_number_source = page_ocr_hints.as_ref().and_then(|hints| hints.number_source.as_ref());
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let number_recovery = detached_number_source
        .map(|source| Arc::clone(&source.run))
        .unwrap_or_else(|| Arc::new(super::number_recovery::Run::default()));
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let number_primary_text = None;
    #[cfg(feature = "pdf")]
    let retained_hocr_read = None;
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let region_replay_raster = None;
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let number_recovered_pages = vec![false; total_pages];
    let backend_additional_metadata: Option<ahash::AHashMap<std::borrow::Cow<'static, str>, serde_json::Value>> = None;
    let page_segmentation_modes = std::collections::BTreeMap::<u32, i32>::new();

    // Opened on first blank page only; see `fallback_render_document`.
    #[cfg(feature = "pdf")]
    let fallback_pdf_state: Option<Option<xberg_native_pdf::PdfDocument>> = None;

    // Judged per page just before the OCR text is accepted, so a drawing page contributes
    // nothing instead of contributing invented words. See `is_ocr_recognition_noise`.
    let ocr_output_thresholds = base_ocr_config.quality_thresholds.clone().unwrap_or_default();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let orientation_retry = if can_retry_orientation || (builtin_tesseract && forced_layout_frames.is_some()) {
        detached_page
            .and_then(|source| source.orientation_retry.cloned())
            .or_else(|| {
                (super::orientation_retry::OrientationRetry::allows(config)
                    && content.is_some()
                    && (images.is_none() || forced_layout_frames.is_some()))
                .then(|| {
                    Arc::new(super::orientation_retry::OrientationRetry::new(
                        config,
                        ocr_output_thresholds.clone(),
                    ))
                })
            })
    } else {
        None
    };
    let recognition_noise_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    // Numeric evidence behind each fired `recognition_noise_warnings` entry, so a caller
    // above the accept/reject decision can observe the signal without recomputing stats
    // (see `OcrPageNoiseVerdict`). Populated exactly when a warning above is pushed. ~keep
    let recognition_noise_verdicts: Vec<OcrPageNoiseVerdict> = Vec::new();

    // #1812: `take_or_create_tatr` can block on a `std::sync::Condvar` (pool exhaustion, or a
    // competing default-acceleration model load) and, on a cache miss, performs synchronous
    // model-file I/O and ONNX Runtime session construction. None of that may run on a tokio
    // worker thread -- two concurrent layout-enabled extractions on a small worker pool could
    // starve every worker with nothing left to poll the task that would return a lease. Route
    // through `spawn_blocking`, mirroring the boundary already used for the RT-DETR layout
    // engine in `layout_runner::run_layout_for_pdf_pages_async`. ~keep
    #[cfg(feature = "layout-detection")]
    let tatr_model = if pdf_ocr_needs_table_model(config, layout_detections) {
        let tatr_acceleration = config.resolved_layout_acceleration().cloned();
        let tatr_thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _entered = span.entered();
            crate::layout::take_or_create_tatr(tatr_acceleration.as_ref(), tatr_thread_budget)
        })
        .await
        .map_err(|e| crate::XbergError::Plugin {
            message: format!("TATR model checkout panicked: {}", e),
            plugin_name: "layout".to_string(),
        })?
    } else {
        None
    };

    #[cfg(feature = "layout-detection")]
    if let Some(warning) = tatr_unavailable_warning(config, layout_detections, tatr_model.is_some()) {
        crate::core::diagnostics::push_warning_deduped(&mut backend_page_warnings, warning);
    }

    Ok(PageOcrStart::Pages(Box::new(PageOcrContext {
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        page_rotation_override,
        skip_document_global_heuristic,
        points_per_pixel_override,
        page_ocr_hints,
        page_index_offset,
        detached_page,
        xobject_document,
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        stored_scan,
        has_stored_scan,
        has_layout_detections,
        resolved_ocr_config,
        ocr_config_owned,
        ocr_output_thresholds,
        backend,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        builtin_tesseract,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        can_retry_orientation,
        can_prove_no_admitted_images,
        no_admitted_images,
        backend_confidence_semantics,
        backend_name,
        backend_confidence_scale,
        #[cfg(feature = "pdf")]
        orientation_handling,
        #[cfg(feature = "pdf")]
        page_margins,
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        tall_source_unsupported,
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        try_tall_source,
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        tall_ordered_primary,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        force_khmer_retry,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        forced_khmer_supplements,
        preprocessing_render_dpi,
        capture_rasters,
        #[cfg(feature = "pdf")]
        lazy_pdf_render_state,
        lazy_pdf_page_count,
        batch_size,
        total_pages,
        progress_total_pages,
        #[cfg(feature = "pdf")]
        external_image_page_geometry,
        #[cfg(feature = "pdf")]
        fallback_pdf_state,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        leading_khmer_rows,
        page_texts,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        paint_retention,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        background_noise,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        certified_background,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        paint_work,
        rejected_pages,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        owned_scan_rows,
        all_page_paragraphs,
        #[cfg(all(
            feature = "ocr",
            feature = "layout-detection",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        column_lanes_by_page,
        #[cfg(all(
            feature = "ocr",
            feature = "layout-detection",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        ordered_layout_pages,
        page_ocr_scales,
        detached_page_scales,
        #[cfg(feature = "pdf")]
        paragraph_rasters,
        public_ocr_pages,
        ocr_page_heights,
        pages_text_claimed_by_tables,
        retry_texts_outside_tables,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        caption,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        capture_caption,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        detached_table_pair,
        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
        collected_table_grids,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        orientation_retry,
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        matched_layer_occurrences,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        complete_native_words,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        native_ink_owned,
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        native_placed_words,
        #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        tall_source_used,
        #[cfg(feature = "pdf")]
        retained_hocr_read,
        #[cfg(all(
            test,
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        region_replay_raster,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        number_recovered_pages,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        number_recovery,
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        number_primary_text,
        page_segmentation_modes,
        backend_additional_metadata,
        collected_tables,
        all_ocr_elements,
        accumulated_llm_usage,
        accumulated_formulas,
        conf_sum,
        conf_count,
        ocr_confidence_by_page,
        captured_rasters,
        preprocessing_by_page,
        #[cfg(feature = "pdf")]
        image_fallback_warnings,
        #[cfg(feature = "pdf")]
        xobject_recovery_budget,
        page_backend_errors,
        page_failure_warnings,
        pending_page_failure_warnings,
        page_failures,
        pages_with_empty_xobject_retry,
        pages_with_recovered_xobject_payload,
        #[cfg(feature = "pdf")]
        margin_filter_warnings,
        #[cfg(feature = "pdf")]
        render_warnings,
        #[cfg(feature = "pdf")]
        raster_dpis,
        backend_page_warnings,
        recognition_noise_warnings,
        recognition_noise_verdicts,
        #[cfg(feature = "layout-detection")]
        tatr_model,
    })))
}
