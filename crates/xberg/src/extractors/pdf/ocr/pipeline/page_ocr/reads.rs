// Each phase binds the whole context by value and writes only the fields its own feature leg
// touches; which of those bindings need `mut` therefore differs per build. ~keep
#![allow(unused_mut)]

use super::super::*;
use super::*;
use crate::plugins::registry::get_ocr_backend_registry;
use std::sync::Arc;
use tokio::task::JoinSet;

impl PageOcrContext<'_> {
    /// Read every page of one rendered batch, in parallel where OS threads exist.
    pub(super) async fn read_batch(
        self,
        batch_start: usize,
        rendered: &RenderedBatch<'_>,
    ) -> OcrRunResult<(Self, BatchReads)> {
        let Self {
            mut content,
            mut images,
            #[cfg(feature = "layout-detection")]
            mut layout_detections,
            mut config,
            mut page_rotation_override,
            mut skip_document_global_heuristic,
            mut points_per_pixel_override,
            mut page_ocr_hints,
            mut page_index_offset,
            mut detached_page,
            mut xobject_document,
            #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
            mut stored_scan,
            mut has_stored_scan,
            mut has_layout_detections,
            mut resolved_ocr_config,
            mut ocr_config_owned,
            mut ocr_output_thresholds,
            mut backend,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            mut builtin_tesseract,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            mut can_retry_orientation,
            mut can_prove_no_admitted_images,
            mut no_admitted_images,
            mut backend_confidence_semantics,
            mut backend_name,
            mut backend_confidence_scale,
            #[cfg(feature = "pdf")]
            mut orientation_handling,
            #[cfg(feature = "pdf")]
            mut page_margins,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut tall_source_unsupported,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut try_tall_source,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut tall_ordered_primary,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut force_khmer_retry,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut forced_khmer_supplements,
            mut preprocessing_render_dpi,
            mut capture_rasters,
            #[cfg(feature = "pdf")]
            mut lazy_pdf_render_state,
            mut lazy_pdf_page_count,
            mut batch_size,
            mut total_pages,
            mut progress_total_pages,
            #[cfg(feature = "pdf")]
            mut external_image_page_geometry,
            #[cfg(feature = "pdf")]
            mut fallback_pdf_state,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut leading_khmer_rows,
            mut page_texts,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut paint_retention,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut background_noise,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut certified_background,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut paint_work,
            mut rejected_pages,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut owned_scan_rows,
            mut all_page_paragraphs,
            #[cfg(all(
                feature = "ocr",
                feature = "layout-detection",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut column_lanes_by_page,
            #[cfg(all(
                feature = "ocr",
                feature = "layout-detection",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut ordered_layout_pages,
            mut page_ocr_scales,
            mut detached_page_scales,
            #[cfg(feature = "pdf")]
            mut paragraph_rasters,
            mut public_ocr_pages,
            mut ocr_page_heights,
            mut pages_text_claimed_by_tables,
            mut retry_texts_outside_tables,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut caption,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut capture_caption,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut detached_table_pair,
            #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
            mut collected_table_grids,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            mut orientation_retry,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mut matched_layer_occurrences,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut complete_native_words,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut native_ink_owned,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut native_placed_words,
            #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
            mut tall_source_used,
            #[cfg(feature = "pdf")]
            mut retained_hocr_read,
            #[cfg(all(
                test,
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut region_replay_raster,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut number_recovered_pages,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut number_recovery,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut number_primary_text,
            mut page_segmentation_modes,
            mut backend_additional_metadata,
            mut collected_tables,
            mut all_ocr_elements,
            mut accumulated_llm_usage,
            mut accumulated_formulas,
            mut conf_sum,
            mut conf_count,
            mut ocr_confidence_by_page,
            mut captured_rasters,
            mut preprocessing_by_page,
            #[cfg(feature = "pdf")]
            mut image_fallback_warnings,
            #[cfg(feature = "pdf")]
            mut xobject_recovery_budget,
            mut page_backend_errors,
            mut page_failure_warnings,
            mut pending_page_failure_warnings,
            mut page_failures,
            mut pages_with_empty_xobject_retry,
            mut pages_with_recovered_xobject_payload,
            #[cfg(feature = "pdf")]
            mut margin_filter_warnings,
            #[cfg(feature = "pdf")]
            mut render_warnings,
            #[cfg(feature = "pdf")]
            mut raster_dpis,
            mut backend_page_warnings,
            mut recognition_noise_warnings,
            mut recognition_noise_verdicts,
            #[cfg(feature = "layout-detection")]
            mut tatr_model,
        } = self;
        let base_ocr_config = resolved_ocr_config.as_ref();
        #[cfg(feature = "pdf")]
        let external_image_page_rotations: Option<&[u32]> = external_image_page_geometry
            .as_ref()
            .map(|geometry| geometry.rotations.as_slice());
        #[cfg(feature = "pdf")]
        let external_image_page_dimensions: Option<&[(f32, f32)]> = external_image_page_geometry
            .as_ref()
            .map(|geometry| geometry.dimensions_pt.as_slice());
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        let forced_layout_frames: Option<&[bool]> = page_ocr_hints
            .as_ref()
            .filter(|_| config.force_ocr && has_layout_detections && base_ocr_config.auto_rotate.is_none())
            .and_then(|hints| hints.layout_rasters_in_page_frame.as_deref());
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
        let number_config = detached_number_source.map_or(config, |source| &source.config);
        let batch_slice = rendered.slice.as_deref();
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        let encoded = if let Some(scan) = stored_scan {
            vec![(0, Arc::clone(&scan.data), scan.render.0, scan.render.1)]
        } else {
            rendered.encoded.clone()
        };
        #[cfg(not(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
        let encoded = rendered.encoded.clone();
        let batch_count = encoded.len();
        #[cfg(all(
            feature = "ocr",
            feature = "layout-detection",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let mut batch_column_orders: Vec<Option<super::column_retry::LineOrder>> =
            (0..batch_count).map(|_| None).collect();
        let mut batch_ocr_results: Vec<Option<crate::types::ExtractedDocument>> = vec![None; batch_count];
        // Backend error and its display text for a page whose OCR call failed, if any (#1444). The page's
        // entry in `batch_ocr_results` is an empty document in that case, so the blank-page
        // image-XObject fallback below gets its chance before the failure is reported.
        let mut batch_page_errors: Vec<_> = (0..batch_count).map(|_| None).collect();
        // (correction_degrees, upright_width, upright_height) applied by
        // `upright_raster_for_backend` before this page's backend call, so the matching
        // `undo_upright_raster_correction` below maps the result back correctly (#643).
        let mut batch_upright_correction: Vec<(u32, u32, u32)> = vec![(0, 0, 0); batch_count];
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let mut detached_frame_reads = vec![false; batch_count];
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let mut number_frame_reads = vec![false; batch_count];
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let mut paint_frames = vec![false; batch_count];

        // See the sibling JoinSet block above: `Send` futures aren't available on wasm32. ~keep
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        {
            let mut join_set = JoinSet::new();
            for (page_idx, image_data, width, height) in &encoded {
                let idx = *page_idx;
                let cancel_token = config.cancel_token.clone();
                if cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
                    return Err(crate::XbergError::Cancelled.into());
                }
                let backend_clone = std::sync::Arc::clone(&backend);
                #[cfg(feature = "pdf")]
                let page_rotation_degrees = if page_rotation_override != 0 {
                    page_rotation_override
                } else {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(_, _, rotations)| rotations.get(*page_idx))
                        .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(*page_idx)))
                        .copied()
                        .unwrap_or(0)
                };
                #[cfg(not(feature = "pdf"))]
                let page_rotation_degrees: u32 = 0;
                // `lazy_pdf_render_state` marks the branch that rendered the pages itself (it is
                // only opened when `images.is_none()`, see its `let` above). Pre-rendered images
                // reach the same answer from the source document's MediaBox instead, and stay
                // hint-free when they are not a whole-page render of it (#1753).
                #[cfg(feature = "pdf")]
                let source_dpi = page_ocr_hints.as_ref().and_then(|hints| hints.source_dpi).or_else(|| {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(doc, _, _)| rendered_page_source_dpi(doc, *page_idx, *width))
                        .or_else(|| {
                            external_image_page_dimensions
                                .and_then(|dimensions| dimensions.get(*page_idx))
                                .and_then(|dimensions| pre_rendered_page_source_dpi(*dimensions, *width, *height))
                        })
                });
                #[cfg(not(feature = "pdf"))]
                let source_dpi: Option<f64> = None;
                #[cfg(feature = "pdf")]
                let whole_page_raster = page_ocr_hints
                    .as_ref()
                    .filter(|hints| hints.known_full_page_scan)
                    .map_or_else(
                        || {
                            whole_page_raster_for_ocr_page(
                                lazy_pdf_render_state.as_ref(),
                                &mut fallback_pdf_state,
                                content,
                                *page_idx,
                            )
                        },
                        |_| true,
                    );
                #[cfg(not(feature = "pdf"))]
                let whole_page_raster = false;
                #[cfg(feature = "pdf")]
                let prefer_single_block = single_block_for_ocr_page(
                    page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.single_block_pages.as_deref()),
                    page_index_offset + *page_idx + 1,
                    lazy_pdf_render_state.as_ref(),
                    &mut fallback_pdf_state,
                    content,
                    *page_idx,
                );
                #[cfg(not(feature = "pdf"))]
                let prefer_single_block = false;
                let config_clone = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    prefer_single_block,
                    has_layout_detections,
                )
                .into_owned();
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let (config_clone, observe_number_frame) =
                    if get_ocr_backend_registry().read().is_builtin_tesseract(&backend) {
                        super::number_recovery::prepare_read(
                            config_clone,
                            number_config,
                            page_index_offset + *page_idx,
                            if detached_number_source.is_some() {
                                0
                            } else {
                                page_index_offset
                            },
                            detached_number_source
                                .and_then(|source| source.layer((page_index_offset + *page_idx + 1) as u32))
                                .or_else(|| {
                                    page_ocr_hints.as_ref().and_then(|hints| {
                                        hints.native_layer((page_index_offset + *page_idx + 1) as u32)
                                    })
                                }),
                            &number_recovery,
                        )
                    } else {
                        (config_clone, false)
                    };
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let config_clone = if ocr_output_thresholds.prune_symbol_noise_lines
                    && lazy_pdf_render_state.is_some()
                    && !has_layout_detections
                    && !has_stored_scan
                    && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
                {
                    let mut config = ensure_elements_enabled(&config_clone);
                    if let Some(elements) = config.element_config.as_mut() {
                        elements.min_confidence = 0.0;
                    }
                    config
                } else {
                    config_clone
                };
                let original_config = ocr_config_owned.clone();
                // No PDF `/Rotate` is ever known without the `pdf` feature (`page_rotation_degrees`
                // is always `0` above in that build), so there is nothing to correct upright.
                #[cfg(feature = "pdf")]
                let (upright_data, upright_width, upright_height, correction_degrees) = upright_raster_for_backend(
                    image_data,
                    *width,
                    *height,
                    page_rotation_degrees,
                    orientation_handling,
                    config.security_limits.as_ref(),
                )?;
                #[cfg(not(feature = "pdf"))]
                let (upright_data, upright_width, upright_height, correction_degrees) =
                    (Arc::clone(image_data), *width, *height, 0u32);
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let retry = orientation_retry
                    .as_ref()
                    .filter(|_| {
                        orientation_retry_page_eligible(
                            whole_page_raster,
                            has_layout_detections,
                            forced_layout_frames,
                            *page_idx,
                        )
                    })
                    .map(|retry| (Arc::clone(retry), Arc::clone(&upright_data)));
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let mut recognition_recovery = if images.is_none() {
                    super::orientation_retry::RecognitionRecovery::TableOrProse
                } else {
                    page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.detached_recognition_recovery)
                        .unwrap_or(super::orientation_retry::RecognitionRecovery::Table)
                };
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if matches!(
                    recognition_recovery,
                    super::orientation_retry::RecognitionRecovery::MixedPage { .. }
                ) && !get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
                {
                    recognition_recovery = super::orientation_retry::RecognitionRecovery::Table;
                }
                #[cfg(all(feature = "ocr", feature = "layout-detection", not(target_arch = "wasm32")))]
                if matches!(
                    recognition_recovery,
                    super::orientation_retry::RecognitionRecovery::Table
                ) && tatr_model.is_some()
                    && layout_detections
                        .and_then(|pages| pages.get(*page_idx))
                        .is_some_and(|page| {
                            page.detections
                                .iter()
                                .any(crate::ocr::layout_assembly::can_recognize_table_region)
                        })
                {
                    recognition_recovery = super::orientation_retry::RecognitionRecovery::LayoutTable;
                }
                #[cfg(feature = "ocr")]
                let caption_raster =
                    (capture_caption && retry.is_some() && page_rotation_degrees == 0 && correction_degrees == 0).then(
                        || {
                            (
                                Arc::clone(&upright_data),
                                super::caption_conversion::proof_limits(config, base_ocr_config),
                            )
                        },
                    );
                let retained_pixels = batch_slice
                    .as_ref()
                    .and_then(|images| images.get(idx - batch_start))
                    .map_or(0, |image| image.as_bytes().len());
                let retained_source_bytes = retained_ocr_source_bytes(image_data, &upright_data, retained_pixels);
                #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
                let ordered_raster = (try_tall_source
                    && super::tall_source::augmentation::supports_run(base_ocr_config, *height)
                    && total_pages == 1
                    && page_index_offset == 0
                    && idx == 0
                    && correction_degrees == 0
                    && detached_page.is_none()
                    && lazy_pdf_render_state.is_some()
                    && config.pages.as_ref().is_none_or(|pages| !pages.insert_page_markers))
                .then_some((*width, *height, image_data.capacity()));
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let observe_detached_frame = detached_page
                    .is_some_and(|source| detached_mixed_frame::supports(config, source.document.page_count().ok()))
                    && can_retry_orientation
                    && !has_stored_scan
                    && whole_page_raster
                    && page_rotation_degrees == 0
                    && correction_degrees == 0
                    && base_ocr_config.auto_rotate.is_none()
                    && base_ocr_config.tesseract_config.is_none()
                    && base_ocr_config.effective_tesseract_language().as_slice() == ["eng"];
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let observe_paint_frame = ocr_output_thresholds.prune_symbol_noise_lines
                    && lazy_pdf_render_state.is_some()
                    && !has_layout_detections
                    && !has_stored_scan
                    && correction_degrees == 0
                    && get_ocr_backend_registry().read().is_builtin_tesseract(&backend);
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let probe_reads = page_ocr_hints.as_ref().and_then(|hints| hints.probe_reads.clone());
                #[cfg(all(
                    feature = "pdf",
                    feature = "ocr",
                    feature = "layout-detection",
                    not(target_arch = "wasm32")
                ))]
                let column_raster = (has_layout_detections
                    && whole_page_raster
                    && progress_total_pages == 1
                    && !has_stored_scan
                    && page_rotation_degrees == 0
                    && correction_degrees == 0
                    && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
                    && !config.force_ocr
                    && config.force_ocr_pages.is_none()
                    && base_ocr_config.auto_rotate.is_none()
                    && base_ocr_config.pipeline.is_none()
                    && base_ocr_config.tesseract_config.as_ref().and_then(|t| t.psm).is_none()
                    && base_ocr_config.effective_tesseract_language().as_slice() == ["eng"]
                    && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
                    && xobject_document.is_none_or(|pdf| pdf.page_count().ok() == Some(1)))
                .then(|| Arc::clone(&upright_data));
                let task = crate::engine::seams::inherit_progress(async move {
                    if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                        return PageRead::failed(
                            idx,
                            correction_degrees,
                            (upright_width, upright_height),
                            crate::XbergError::Cancelled,
                        );
                    }
                    let (original_config, config_clone) = match crate::extractors::image::reserve_retained_ocr_raster(
                        &backend_clone,
                        (original_config, config_clone),
                        retained_source_bytes,
                    ) {
                        Ok(configs) => configs,
                        Err(error) => {
                            return PageRead::failed(idx, correction_degrees, (upright_width, upright_height), error);
                        }
                    };
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let primary_config = retry
                        .as_ref()
                        .and_then(|(retry, _)| retry.primary_config(&config_clone));
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let read_config = primary_config.as_ref().unwrap_or(&config_clone);
                    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                    let read_config = &config_clone;
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let read = super::read_reuse::read(
                        probe_reads
                            .as_ref()
                            .filter(|_| primary_config.is_none() && !force_khmer_retry),
                        super::read_reuse::ReadIdentity {
                            page: (page_index_offset + idx + 1) as u32,
                            backend: &backend_clone,
                            configs: (&original_config, read_config),
                            whole_page_raster,
                            correction_degrees,
                        },
                        upright_data,
                        cancel_token.as_ref(),
                        |data| {
                            crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                                &backend_clone,
                                data,
                                &original_config,
                                read_config,
                                crate::extractors::image::SparseRetryPolicy {
                                    whole_page_raster,
                                    allow_standard_retry: false,
                                },
                                cancel_token.as_ref(),
                            )
                        },
                    );
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let read = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                        &backend_clone,
                        upright_data,
                        &original_config,
                        read_config,
                        crate::extractors::image::SparseRetryPolicy {
                            whole_page_raster,
                            allow_standard_retry: false,
                        },
                        cancel_token.as_ref(),
                    );
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let primary = async {
                        if let Some((retry, _)) = &retry {
                            retry.read_primary(&config_clone, read).await
                        } else {
                            read.await
                        }
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let primary = async {
                        if force_khmer_retry {
                            crate::ocr::processor::script_retry::MIXED_PDF_KHMER_RETRY
                                .scope(true, primary)
                                .await
                        } else {
                            primary.await
                        }
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let (result, scale_only, inventory) = if caption_raster.is_some() {
                        crate::ocr::preprocessing::geometry_evidence::observe_with_word_inventory(primary).await
                    } else if ordered_raster.is_some()
                        || observe_detached_frame
                        || observe_number_frame
                        || observe_paint_frame
                    {
                        let (result, scale_only) = crate::ocr::preprocessing::geometry_evidence::observe(primary).await;
                        (result, scale_only, None)
                    } else {
                        (primary.await, false, None)
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let paint_read = (observe_paint_frame && scale_only)
                        .then(|| result.as_ref().ok().map(|read| blake3::hash(read.content.as_bytes())))
                        .flatten();
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    let background = (certified_background && scale_only && observe_paint_frame)
                        .then(|| {
                            super::symbol_noise_background::Evidence::capture(
                                result.as_ref().ok()?,
                                page_index_offset + idx,
                            )
                        })
                        .flatten();
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let number_proof = observe_number_frame
                        .then(|| {
                            result
                                .as_ref()
                                .ok()
                                .and_then(|read| super::number_recovery::ReadProof::capture(read, scale_only))
                        })
                        .flatten();
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let detached_frame_read = observe_detached_frame
                        && scale_only
                        && result.as_ref().is_ok_and(detached_mixed_frame::stock_mixed_read);
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let detached_frame_read = false;
                    // Stock orientation preserves table-bearing primaries but later restores
                    // the caller's public element policy. Capture the complete internal read here.
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let ordered_proof = ordered_raster
                        .filter(|_| scale_only)
                        .zip(result.as_ref().ok())
                        .and_then(|(raster, document)| {
                            super::tall_source::augmentation::PrimaryProof::capture(
                                document,
                                document.ocr_elements.as_deref()?,
                                raster,
                            )
                        });
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let (result, ordered_proof) = (read.await, None::<()>);
                    // ~keep A table makes the recognition retry keep the original attempt.
                    #[cfg(feature = "ocr")]
                    let captured_caption = if let Some((raster, mut limits)) = caption_raster
                        && primary_config.is_some()
                        && scale_only
                    {
                        let candidate = inventory.zip(result.as_ref().ok()).and_then(|(inventory, result)| {
                            super::caption_conversion::capture(
                                result,
                                (upright_width, upright_height),
                                &mut limits,
                                inventory,
                            )
                        });
                        super::caption_conversion::verify_paint(
                            candidate,
                            raster,
                            retained_source_bytes,
                            limits,
                            cancel_token.clone(),
                        )
                        .await
                    } else {
                        None
                    };
                    #[cfg(not(feature = "ocr"))]
                    let captured_caption = None::<()>;
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let result = if let Some((retry, raster)) = retry {
                        retry
                            .after_ocr(
                                &backend_clone,
                                raster,
                                &config_clone,
                                result,
                                cancel_token.as_ref(),
                                recognition_recovery,
                            )
                            .await
                    } else {
                        result
                    };
                    #[cfg(all(
                        feature = "pdf",
                        feature = "ocr",
                        feature = "layout-detection",
                        not(target_arch = "wasm32")
                    ))]
                    let (result, column_order) = match (column_raster, result) {
                        (Some(raster), Ok(baseline)) if !detached_frame_read => super::column_retry::retry(
                            baseline,
                            &backend_clone,
                            raster,
                            &config_clone,
                            cancel_token.as_ref(),
                        )
                        .await
                        .map_or_else(|error| (Err(error), None), |(primary, order)| (Ok(primary), order)),
                        (_, result) => (result, None),
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let number_frame =
                        number_proof.is_some_and(|proof| result.as_ref().is_ok_and(|read| proof.matches(read)));
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let number_frame = false;
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let ordered_proof =
                        ordered_proof.filter(|proof| result.as_ref().is_ok_and(|selected| proof.matches(selected)));
                    #[cfg(feature = "ocr")]
                    let captured_caption = captured_caption
                        .filter(|proof| result.as_ref().is_ok_and(|selected| proof.matches_selected(selected)));
                    #[cfg(feature = "ocr")]
                    let paint_frame = paint_read.is_some_and(|hash| {
                        result
                            .as_ref()
                            .is_ok_and(|read| hash == blake3::hash(read.content.as_bytes()))
                    });
                    PageRead {
                        page_idx: idx,
                        correction_degrees,
                        upright_width,
                        upright_height,
                        #[cfg(all(
                            feature = "ocr",
                            feature = "layout-detection",
                            feature = "tokio-runtime",
                            not(target_arch = "wasm32")
                        ))]
                        column_order,
                        result,
                        ordered_proof,
                        detached_frame_read,
                        captured_caption,
                        number_frame,
                        #[cfg(feature = "ocr")]
                        paint_frame,
                        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                        background,
                    }
                });
                join_set.spawn(crate::core::extractor::cacheability::inherit(task));
            }
            while let Some(join_result) = join_set.join_next().await {
                let PageRead {
                    page_idx,
                    correction_degrees,
                    upright_width,
                    upright_height,
                    #[cfg(all(
                        feature = "ocr",
                        feature = "layout-detection",
                        feature = "tokio-runtime",
                        not(target_arch = "wasm32")
                    ))]
                    column_order,
                    result: ocr_result,
                    ordered_proof,
                    detached_frame_read,
                    captured_caption,
                    number_frame,
                    #[cfg(feature = "ocr")]
                    paint_frame,
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    background,
                } = join_result.map_err(|e| crate::XbergError::Plugin {
                    message: format!("OCR task panicked: {}", e),
                    plugin_name: "ocr".to_string(),
                })?;
                if matches!(&ocr_result, Err(crate::XbergError::Cancelled)) {
                    return Err(crate::XbergError::Cancelled.into());
                }
                #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
                let _ = (ordered_proof, detached_frame_read, number_frame);
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                {
                    detached_frame_reads[page_idx - batch_start] = detached_frame_read;
                    number_frame_reads[page_idx - batch_start] = number_frame;
                    paint_frames[page_idx - batch_start] = paint_frame;
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    if let Some(evidence) = background {
                        background_noise.insert(page_idx, evidence);
                    }
                }
                #[cfg(all(
                    feature = "ocr",
                    feature = "pdf",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let ocr_result = {
                    let mut ocr_result = ocr_result;
                    if let Some(proof) = ordered_proof {
                        tall_ordered_primary = Some(proof);
                    }
                    if try_tall_source
                        && correction_degrees == 0
                        && let Ok(document) = &mut ocr_result
                        && let Some(source) = lazy_pdf_render_state
                            .as_ref()
                            .map(|(pdf, _, _)| (pdf.as_ref(), page_idx))
                            .or_else(|| xobject_document.map(|pdf| (pdf, page_index_offset + page_idx)))
                    {
                        tall_source_used |= super::tall_source::try_replace(
                            document,
                            &backend,
                            config,
                            base_ocr_config,
                            source,
                            &encoded[page_idx - batch_start],
                        )
                        .await?;
                    } else if let Some(reason) = tall_source_unsupported
                        && correction_degrees == 0
                        && let Ok(document) = &mut ocr_result
                        && let Some(source) = lazy_pdf_render_state
                            .as_ref()
                            .map(|(pdf, _, _)| (pdf.as_ref(), page_idx))
                            .or_else(|| xobject_document.map(|pdf| (pdf, page_index_offset + page_idx)))
                    {
                        let (_, _, raster_width, raster_height) = &encoded[page_idx - batch_start];
                        super::tall_source::note_unrecovered(
                            document,
                            base_ocr_config,
                            source,
                            (*raster_width, *raster_height),
                            super::tall_source::Unrecovered::Config(reason),
                        );
                    }
                    ocr_result
                };
                crate::engine::seams::emit_ocr_page(
                    page_index_offset + page_idx + 1,
                    progress_total_pages,
                    backend_name.as_str(),
                );
                batch_upright_correction[page_idx - batch_start] = (correction_degrees, upright_width, upright_height);
                #[cfg(feature = "ocr")]
                if ocr_result.is_ok() && captured_caption.is_some() {
                    caption = captured_caption;
                }
                #[cfg(not(feature = "ocr"))]
                let _ = captured_caption;
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                {
                    batch_column_orders[page_idx - batch_start] = column_order;
                }
                match ocr_result {
                    Ok(document) => batch_ocr_results[page_idx - batch_start] = Some(document),
                    Err(error) => {
                        let described = describe_page_failure(&error);
                        tracing::warn!(
                            page = page_index_offset + page_idx + 1,
                            error = %error.pdf_ocr_trace_category(),
                            "OCR backend failed for page"
                        );
                        batch_page_errors[page_idx - batch_start] = Some((error, described));
                        batch_ocr_results[page_idx - batch_start] = Some(crate::types::ExtractedDocument::default());
                    }
                }
            }
        }
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        {
            for (page_idx, image_data, width, height) in &encoded {
                #[cfg(feature = "pdf")]
                let page_rotation_degrees = if page_rotation_override != 0 {
                    page_rotation_override
                } else {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(_, _, rotations)| rotations.get(*page_idx))
                        .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(*page_idx)))
                        .copied()
                        .unwrap_or(0)
                };
                #[cfg(not(feature = "pdf"))]
                let page_rotation_degrees: u32 = 0;
                // See the JoinSet branch above for both derivations.
                #[cfg(feature = "pdf")]
                let source_dpi = page_ocr_hints.as_ref().and_then(|hints| hints.source_dpi).or_else(|| {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(doc, _, _)| rendered_page_source_dpi(doc, *page_idx, *width))
                        .or_else(|| {
                            external_image_page_dimensions
                                .and_then(|dimensions| dimensions.get(*page_idx))
                                .and_then(|dimensions| pre_rendered_page_source_dpi(*dimensions, *width, *height))
                        })
                });
                #[cfg(not(feature = "pdf"))]
                let source_dpi: Option<f64> = None;
                #[cfg(feature = "pdf")]
                let whole_page_raster = page_ocr_hints
                    .as_ref()
                    .filter(|hints| hints.known_full_page_scan)
                    .map_or_else(
                        || {
                            whole_page_raster_for_ocr_page(
                                lazy_pdf_render_state.as_ref(),
                                &mut fallback_pdf_state,
                                content,
                                *page_idx,
                            )
                        },
                        |_| true,
                    );
                #[cfg(not(feature = "pdf"))]
                let whole_page_raster = false;
                #[cfg(feature = "pdf")]
                let prefer_single_block = single_block_for_ocr_page(
                    page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.single_block_pages.as_deref()),
                    page_index_offset + *page_idx + 1,
                    lazy_pdf_render_state.as_ref(),
                    &mut fallback_pdf_state,
                    content,
                    *page_idx,
                );
                #[cfg(not(feature = "pdf"))]
                let prefer_single_block = false;
                let config_for_page = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    prefer_single_block,
                    has_layout_detections,
                );
                #[cfg(feature = "pdf")]
                let (upright_data, upright_width, upright_height, correction_degrees) = upright_raster_for_backend(
                    image_data,
                    *width,
                    *height,
                    page_rotation_degrees,
                    orientation_handling,
                    config.security_limits.as_ref(),
                )?;
                #[cfg(not(feature = "pdf"))]
                let (upright_data, upright_width, upright_height, correction_degrees) =
                    (Arc::clone(image_data), *width, *height, 0u32);
                // Route token cancellation through the same terminal result check as backend cancellation. ~keep
                let ocr_result = if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    Err(crate::XbergError::Cancelled)
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        async {
                            let retained_pixels = batch_slice
                                .as_ref()
                                .and_then(|images| images.get(*page_idx - batch_start))
                                .map_or(0, |image| image.as_bytes().len());
                            let retained = retained_ocr_source_bytes(image_data, &upright_data, retained_pixels);
                            let (original, effective) = crate::extractors::image::reserve_retained_ocr_raster(
                                &backend,
                                (ocr_config_owned.clone(), config_for_page.as_ref().clone()),
                                retained,
                            )?;
                            crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                                &backend,
                                upright_data,
                                &original,
                                &effective,
                                crate::extractors::image::SparseRetryPolicy {
                                    whole_page_raster,
                                    allow_standard_retry: false,
                                },
                                config.cancel_token.as_ref(),
                            )
                            .await
                        }
                        .await
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        backend
                            .process_image(upright_data.as_slice(), config_for_page.as_ref())
                            .await
                    }
                };
                if matches!(&ocr_result, Err(crate::XbergError::Cancelled)) {
                    return Err(crate::XbergError::Cancelled.into());
                }
                crate::engine::seams::emit_ocr_page(
                    page_index_offset + *page_idx + 1,
                    progress_total_pages,
                    backend_name.as_str(),
                );
                batch_upright_correction[page_idx - batch_start] = (correction_degrees, upright_width, upright_height);
                match ocr_result {
                    Ok(document) => batch_ocr_results[page_idx - batch_start] = Some(document),
                    Err(error) => {
                        let described = describe_page_failure(&error);
                        tracing::warn!(
                            page = page_index_offset + page_idx + 1,
                            error = %error.pdf_ocr_trace_category(),
                            "OCR backend failed for page"
                        );
                        batch_page_errors[page_idx - batch_start] = Some((error, described));
                        batch_ocr_results[page_idx - batch_start] = Some(crate::types::ExtractedDocument::default());
                    }
                }
            }
        }
        Ok((
            Self {
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
            },
            BatchReads {
                upright_correction: batch_upright_correction,
                results: batch_ocr_results,
                errors: batch_page_errors,
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                column_orders: batch_column_orders,
                #[cfg(all(
                    feature = "pdf",
                    feature = "ocr",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                detached_frames: detached_frame_reads,
                #[cfg(all(
                    feature = "pdf",
                    feature = "ocr",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                number_frames: number_frame_reads,
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                paint_frames,
            },
        ))
    }
}
