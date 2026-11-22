// Each phase binds the whole context by value and writes only the fields its own feature leg
// touches; which of those bindings need `mut` therefore differs per build. ~keep
#![allow(unused_mut)]

use super::super::*;
use super::*;
use crate::plugins::registry::get_ocr_backend_registry;
use std::sync::Arc;

impl PageOcrContext<'_> {
    /// Post-process every page read of one batch.
    pub(super) async fn process_batch(
        self,
        batch_start: usize,
        rendered: &RenderedBatch<'_>,
        reads: &mut BatchReads,
    ) -> OcrRunResult<Self> {
        let count = reads.results.len();
        let mut this = self;
        for offset in 0..count {
            this = this.process_page(batch_start, offset, rendered, reads).await?;
        }
        Ok(this)
    }

    /// Post-process one page: recover what the read lost, then classify its paragraphs and
    /// accept or reject its text.
    async fn process_page(
        self,
        batch_start: usize,
        offset: usize,
        rendered: &RenderedBatch<'_>,
        reads: &mut BatchReads,
    ) -> OcrRunResult<Self> {
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
        #[allow(unused_variables)]
        let external_image_page_rotations: Option<&[u32]> = external_image_page_geometry
            .as_ref()
            .map(|geometry| geometry.rotations.as_slice());
        #[cfg(feature = "pdf")]
        #[allow(unused_variables)]
        let external_image_page_dimensions: Option<&[(f32, f32)]> = external_image_page_geometry
            .as_ref()
            .map(|geometry| geometry.dimensions_pt.as_slice());
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
        let encoded_batch = rendered.encoded.as_slice();
        #[allow(unused_variables)]
        let batch_slice = rendered.slice.as_deref();
        let page_idx = batch_start + offset;
        let document_page_idx = page_index_offset + page_idx;
        let document_page_number = (document_page_idx + 1) as u32;
        let ocr_result = reads.results[offset].take().expect("OCR result missing for page");
        #[cfg(all(
            test,
            paddle_ocr,
            feature = "ocr",
            feature = "bundle-tessdata-eng",
            not(target_arch = "wasm32")
        ))]
        let mut ocr_result = sparse_glyph_read::test_implicit_attempt(
            ocr_result,
            base_ocr_config,
            encoded_batch[offset].2,
            encoded_batch[offset].3,
            page_ocr_hints
                .as_ref()
                .and_then(|hints| hints.sparse_glyph_trial.as_ref()),
        );
        #[cfg(not(all(
            test,
            paddle_ocr,
            feature = "ocr",
            feature = "bundle-tessdata-eng",
            not(target_arch = "wasm32")
        )))]
        let mut ocr_result = ocr_result;
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        if let Some(words) = page_ocr_hints
            .as_ref()
            .and_then(|hints| hints.sparse_glyph_trial.as_ref())
            && !sparse_glyph_read::accepted_pipeline_read(
                words,
                &ocr_result,
                (encoded_batch[offset].2, encoded_batch[offset].3),
                &base_ocr_config.effective_thresholds(),
                backend.confidence_semantics(),
            )
        {
            return Err(sparse_glyph_read::declined());
        }
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        // The complete source supplies this count. A detached page counts the document it came
        // from: a build with an automatic fallback stage reads a one-page scan through the
        // per-page pipeline, which detaches it. Stored-scan and layout-guided calls have zero. ~keep
        if let Some(rows) = ocr_result.ocr_internal_document.as_ref().and_then(|doc| {
            let source_page_count = match detached_page {
                _ if lazy_pdf_page_count != 0 => lazy_pdf_page_count,
                Some(page) if !has_stored_scan && !has_layout_detections => {
                    page.document.page_count().unwrap_or_default()
                }
                _ => 0,
            };
            super::document::certified_scan_html_rows(config, source_page_count, builtin_tesseract, doc)
        }) {
            owned_scan_rows = Some(rows.to_owned());
        }

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if !has_stored_scan
            && !tall_source_used
            && reads.upright_correction[offset].0 == 0
            && let Some(source) = detached_page
                .map(|page| (page.document, document_page_idx))
                .or_else(|| {
                    lazy_pdf_render_state
                        .as_ref()
                        .map(|(pdf, _, _)| (pdf.as_ref(), page_idx))
                })
        {
            super::tall_source::probe_anchored_occurrences(
                &mut ocr_result,
                &backend,
                config,
                base_ocr_config,
                source,
                &encoded_batch[offset],
            )
            .await?;
        }

        #[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if let Some(read) = ocr_result.retained_hocr_read.as_ref() {
            tracing::trace!(total_pages, has_stored_scan, tall_source_used,
                raw_frame = ocr_result.raw_ocr_source_frame.is_some(),
                frame = ?super::document::processed_ocr_layout_dimensions(&ocr_result.metadata),
                read_frame = ?read.dimensions,
                owners = ocr_result.ocr_internal_document.as_ref()
                    .is_some_and(|doc| read.matches(&read.transcript, read.dimensions, doc)),
                "selected OCR batch receipt gates");
        }

        #[cfg(feature = "pdf")]
        if total_pages == 1
            && ocr_result.raw_ocr_source_frame.is_none()
            && let Some(read) = ocr_result.retained_hocr_read.take()
            && let Some(frame) = super::document::processed_ocr_layout_dimensions(&ocr_result.metadata)
            && let Some(doc) = ocr_result.ocr_internal_document.as_ref()
            && read.matches(&read.transcript, frame, doc)
        {
            #[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            if !has_stored_scan && !tall_source_used {
                region_replay_raster = super::tall_source::region_replay::certified_raster(
                    &read,
                    &ocr_result,
                    &encoded_batch[offset],
                    config,
                    (base_ocr_config, &backend),
                    reads.upright_correction[offset].0,
                );
            }
            retained_hocr_read = Some(read);
        }

        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if force_khmer_retry
            && let Some(serde_json::Value::String(text)) = ocr_result
                .metadata
                .additional
                .remove(crate::ocr::processor::script_retry::KHMER_SUPPLEMENT_KEY)
        {
            forced_khmer_supplements.insert(page_idx, text);
        }
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if can_retry_orientation && reads.upright_correction[offset].0 == 0 {
            let coverage_layer = page_ocr_hints
                .as_ref()
                .and_then(|hints| hints.native_coverage_layers.as_ref())
                .and_then(|layers| layers.get(&document_page_number))
                .map(String::as_str);
            let lost_glyph_layer = page_ocr_hints
                .as_ref()
                .and_then(|hints| hints.single_native_type0_page.as_deref())
                .filter(|_| document_page_idx == 0 && super::document::native_preservation_is_automatic(config, 1));
            let pdf = detached_page
                .map(|source| source.document)
                .or_else(|| lazy_pdf_render_state.as_ref().map(|(pdf, _, _)| pdf.as_ref()));
            if let (Some(pdf), Some(layer)) = (pdf, coverage_layer.filter(|_| detached_page.is_some())) {
                complete_native_words = super::native_word_coverage::covers_complete_native_page(
                    pdf,
                    document_page_idx,
                    layer,
                    &ocr_result,
                );
            }
            if let (Some(pdf), Some(layer)) = (
                pdf,
                coverage_layer.filter(|_| detached_page.is_some()).or(lost_glyph_layer),
            ) {
                native_placed_words =
                    super::native_word_coverage::read_words_at_native_place(pdf, document_page_idx, &ocr_result);
                native_ink_owned = !complete_native_words
                    && super::document::native_preservation_is_automatic(config, document_page_number)
                    && super::native_word_coverage::native_text_paints_all_read_ink(
                        pdf,
                        document_page_idx,
                        layer,
                        &ocr_result,
                        super::native_word_coverage::InkProofLimits {
                            thresholds: &ocr_output_thresholds,
                            max_working_bytes: super::hidden_layer::preview_byte_limit(config),
                            cancelled: &|| config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()),
                        },
                    );
            }
        }
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if can_retry_orientation
            && reads.upright_correction[offset].0 == 0
            && let Some(source) = detached_page.filter(|_| !has_stored_scan)
            && let Some(layer) = page_ocr_hints
                .as_ref()
                .and_then(|hints| hints.decision_layer(document_page_number))
        {
            matched_layer_occurrences = super::document::MatchedLayerOccurrences::capture(
                source.document,
                document_page_idx,
                layer,
                &ocr_result,
            );
        }
        crate::extraction::derive::adopt_public_ocr_document(
            &mut ocr_result,
            config.security_limits.as_ref().unwrap_or(&Default::default()),
        )?;
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if reads.upright_correction[offset].0 == 0
            && !tall_source_used
            && !pages_with_recovered_xobject_payload[page_idx]
            && !forced_khmer_supplements.contains_key(&page_idx)
            && !has_layout_detections
            && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
            && let Some(pdf) = detached_page
                .map(|source| source.document)
                .or_else(|| lazy_pdf_render_state.as_ref().map(|(pdf, _, _)| pdf.as_ref()))
        {
            recover_automatic_latin_rows(
                pdf,
                (document_page_idx, document_page_number),
                &mut ocr_result,
                page_ocr_hints
                    .as_ref()
                    .and_then(|hints| hints.decision_layer(document_page_number)),
                config,
                &ocr_output_thresholds,
            );
        }
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if reads.upright_correction[offset].0 == 0
            && !complete_native_words
            && !native_ink_owned
            && !pages_with_recovered_xobject_payload[page_idx]
            && super::document::resolved_ocr_correction_degrees(&ocr_result.metadata).is_none_or(|angle| angle == 0)
            && super::document::native_preservation_is_automatic(config, document_page_number)
            && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
            && let Some(pdf) = detached_page
                .map(|source| source.document)
                .or_else(|| lazy_pdf_render_state.as_ref().map(|(pdf, _, _)| pdf.as_ref()))
        {
            let limits = super::native_word_coverage::InkProofLimits {
                thresholds: &ocr_output_thresholds,
                max_working_bytes: super::hidden_layer::preview_byte_limit(config),
                cancelled: &|| config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()),
            };
            let additions = if config.ocr_strategy == crate::core::config::OcrStrategy::Auto
                && config
                    .pdf_options
                    .as_ref()
                    .is_none_or(|pdf| !pdf.has_nonzero_raw_margins())
                && !forced_khmer_supplements.contains_key(&page_idx)
                && super::layer_token_recovery::is_english_read(&ocr_result)
            {
                super::layer_token_recovery::plan_minority_khmer(pdf, document_page_idx, &ocr_result, &limits)
            } else {
                None
            };
            super::layer_token_recovery::recover_page(
                pdf,
                document_page_idx,
                &mut ocr_result,
                page_ocr_hints
                    .as_ref()
                    .and_then(|hints| hints.decision_layer(document_page_number)),
                super::native_word_coverage::InkProofLimits { ..limits },
            );
            if let Some((additions, leading)) = additions {
                let before = ocr_result.content.len();
                if let Some(rows) = super::layer_token_recovery::apply_minority_khmer(
                    &mut ocr_result,
                    &additions,
                    leading,
                    limits.cancelled,
                ) {
                    leading_khmer_rows.insert(page_idx, rows);
                }
                if detached_page.is_some() && ocr_result.content.len() > before {
                    tracing::debug!(page = document_page_number, "Detached Tesseract read recovered Khmer");
                }
            }
        }
        if let Some(psm) = effective_tesseract_psm(&ocr_result.metadata, &backend_name) {
            page_segmentation_modes.insert(document_page_number, psm);
        }
        if let Some(metadata) = ocr_result.metadata.image_preprocessing.clone() {
            preprocessing_by_page.insert(document_page_number, metadata);
        }
        crate::core::diagnostics::dedup_extend_warnings(
            &mut backend_page_warnings,
            std::mem::take(&mut ocr_result.processing_warnings),
        );
        // ~keep Whether a detached-stage normalizer below already put this page's backend
        // tables in the render frame the page owner scales to points.
        #[cfg(feature = "pdf")]
        #[cfg_attr(
            not(all(feature = "ocr", not(target_arch = "wasm32"))),
            expect(unused_mut, reason = "only the Tesseract normalizers below set it")
        )]
        let mut tables_in_render = false;
        #[cfg(feature = "pdf")]
        {
            let (correction_degrees, upright_width, upright_height) = reads.upright_correction[offset];
            #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
            let normalize_recovered_table = xobject_document.is_some()
                && (orientation_retry.is_some() || base_ocr_config.auto_rotate == Some(true))
                && !ocr_result.tables.is_empty()
                && super::document::resolved_ocr_correction_degrees(&ocr_result.metadata)
                    .is_some_and(|angle| angle != 0);
            undo_upright_raster_correction(&mut ocr_result, correction_degrees, upright_width, upright_height);
            #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
            if let Some(scan) = stored_scan {
                stored_scan_retry::place_in_render_frame(&mut ocr_result, scan)?;
            }
            let render = (encoded_batch[offset].2, encoded_batch[offset].3);
            let normalized_raw = (xobject_document.is_some() || lazy_pdf_render_state.is_some())
                && raw_workspace_geometry::normalize(&mut ocr_result, render, matches!(correction_degrees, 90 | 270));
            #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
            if normalize_recovered_table && !normalized_raw {
                // The detached stage's owner will place raw tables using the render raster. ~keep
                tables_in_render = super::orientation_retry::scale_recovered_table_to_raster(&mut ocr_result, render);
            }
            #[cfg(not(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32"))))]
            let _ = normalized_raw;
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            if detached_page.is_some()
                && can_retry_orientation
                && !has_stored_scan
                && total_pages == 1
                && ocr_result.document.is_none()
                && ocr_result.formulas.is_empty()
                && ocr_result.ocr_internal_document.as_ref().is_some_and(|doc| {
                    doc.ocr_text_claimed_by_tables && doc.elements.is_empty() && doc.tables.is_empty()
                })
                && let Some(processed) = super::document::processed_ocr_layout_dimensions(&ocr_result.metadata)
            {
                // The raw path rewrites the processed dimensions to render size. The certified
                // orientation path already rescales its tables but leaves the text frame alone. ~keep
                #[cfg(auto_rotate)]
                let processed = if normalize_recovered_table { render } else { processed };
                detached_table_pair = detached_table_pair::normalize(&mut ocr_result.tables, processed, render);
                tables_in_render |= detached_table_pair;
            }
        }
        // #1575: capture once, from the first page that has any -- see this function's
        // `backend_additional_metadata` declaration for why a single representative page
        // is the right granularity here.
        if backend_additional_metadata.is_none() && !ocr_result.metadata.additional.is_empty() {
            backend_additional_metadata = Some(ocr_result.metadata.additional.clone());
        }
        #[cfg(feature = "layout-detection")]
        let _height = encoded_batch[offset].3;

        if let Some(conf_val) = ocr_result
            .metadata
            .additional
            .get("mean_text_conf")
            .and_then(|v| v.as_i64())
        {
            conf_sum += conf_val as f64;
            conf_count += 1;
        }

        if let Some(usage) = ocr_result.llm_usage.take() {
            accumulated_llm_usage.extend(usage);
        }

        // ~keep Only the renderer can certify the page frame. Aspect ratio alone
        // cannot distinguish a display-oriented raster from an aligned raster.
        #[cfg(feature = "pdf")]
        let layout_page_points = external_image_page_dimensions
            .filter(|_| {
                xobject_document.is_none()
                    && page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.layout_rasters_in_page_frame.as_deref())
                        .and_then(|pages| pages.get(page_idx))
                        .copied()
                        .unwrap_or(false)
            })
            .and_then(|dimensions| dimensions.get(page_idx).copied())
            .filter(|&(width, height)| width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0);
        if let Some(document) = ocr_result
            .ocr_internal_document
            .as_mut()
            .filter(|_| ocr_result.document.is_some())
        {
            let (width, height) =
                resolved_ocr_layout_dimensions(&ocr_result.metadata, encoded_batch[offset].2, encoded_batch[offset].3);
            let outcome =
                super::document::filter_structured_ocr_page_by_page_margins(document, height as f32, page_margins);
            if outcome.removed {
                ocr_result.content = crate::rendering::render_plain(document);
            }
            if outcome.missing_geometry {
                crate::core::diagnostics::push_warning_deduped(
                    &mut margin_filter_warnings,
                    ocr_margin_filter_capability_warning(),
                );
            }
            let mut document = document.clone();
            use super::document::PublicOcrOutputFrame;
            let frame = if let Some((pdf, _, _)) = lazy_pdf_render_state.as_ref() {
                PublicOcrOutputFrame::PdfPage {
                    raster: (width, height),
                    points: page_dimensions_pt(pdf, page_idx),
                }
            } else if xobject_document.is_some() {
                // ~keep A detached stage has no PDF bytes/lazy render state, but its
                // explicit PDF owner still requires the canonical bottom-left
                // raster frame. build_pipeline_ocr_page_document scales it once.
                PublicOcrOutputFrame::PdfRaster {
                    processed: (width, height),
                    raster: (encoded_batch[offset].2, encoded_batch[offset].3),
                }
            } else if let Some(points) = layout_page_points {
                PublicOcrOutputFrame::PdfPage {
                    raster: (width, height),
                    points,
                }
            } else {
                PublicOcrOutputFrame::ImagePixels
            };
            super::document::place_public_ocr_bboxes(&mut document, frame);
            super::document::normalize_mixed_ocr_document_page(&mut document, document_page_number);
            public_ocr_pages.insert(document_page_number, document);
        }
        // An upright, scale-only stock table is kept by implicit orientation selection.
        // Preserve its source dimensions for paragraphs while putting the table in the detached owner's render frame.
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let detached_paragraph_scale = if reads.detached_frames[offset]
            && !tall_source_used
            && detached_mixed_frame::stock_mixed_read(&ocr_result)
            && let Some(source) = detached_page
        {
            detached_mixed_frame::place_table_in_render(
                &mut ocr_result,
                (encoded_batch[offset].2, encoded_batch[offset].3),
                page_dimensions_pt(source.document, document_page_idx),
            )
        } else {
            None
        };
        #[cfg(all(
            any(not(feature = "layout-detection"), feature = "ocr", feature = "ocr-wasm"),
            not(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))
        ))]
        let detached_paragraph_scale: Option<(f32, f32)> = None;
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let tables_in_render = tables_in_render || detached_paragraph_scale.is_some();
        let mut backend_tables = std::mem::take(&mut ocr_result.tables);
        for table in &mut backend_tables {
            table.page_number = document_page_number;
        }
        #[cfg(feature = "pdf")]
        if xobject_document.is_some() && !tables_in_render {
            // ~keep The detached page owner scales table rectangles from the render raster
            // (`build_pipeline_ocr_page_document`), but recognition may have read a resampled
            // image (GH#1895): bring the rectangles from that frame to the render's.
            let render = (encoded_batch[offset].2, encoded_batch[offset].3);
            let processed = resolved_ocr_layout_dimensions(&ocr_result.metadata, render.0, render.1);
            detached_mixed_frame::scale_tables_into_render(&mut backend_tables, processed, render);
        }
        #[cfg(feature = "pdf")]
        if let Some((page_width_pt, page_height_pt)) = lazy_pdf_render_state
            .as_ref()
            .map(|(doc, _, _)| page_dimensions_pt(doc, page_idx))
            .or(layout_page_points)
        {
            // ~keep A backend table's rect is in the pixel space the backend OCR'd, which
            // preprocessing may have resized away from the render raster -- Tesseract
            // normalizes to `target_dpi` by default. Passing the render dims here scaled
            // every TSV table's page-space bbox by the processed/render ratio, while the
            // formula bbox eleven lines below already resolved the same dims from metadata
            // (xberg-io/xberg#1813).
            let (table_px_width, table_px_height) =
                resolved_ocr_layout_dimensions(&ocr_result.metadata, encoded_batch[offset].2, encoded_batch[offset].3);
            rescale_ocr_bboxes_to_page_points(
                None,
                &mut backend_tables,
                table_px_width,
                table_px_height,
                page_width_pt,
                page_height_pt,
            );
        }
        collected_tables.append(&mut backend_tables);

        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if page_rotation_override != 0
            || !reads.paint_frames[offset]
            || tall_source_used
            || pages_with_recovered_xobject_payload[page_idx]
            || forced_khmer_supplements.contains_key(&page_idx)
        {
            background_noise.remove(&page_idx);
        }

        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if reads.paint_frames[offset]
            && !tall_source_used
            && !pages_with_recovered_xobject_payload[page_idx]
            && !forced_khmer_supplements.contains_key(&page_idx)
            && ocr_result.tables.is_empty()
            && let Some((pdf, _, _)) = lazy_pdf_render_state.as_ref()
            && let Some(planned) = paint_work.probe_page(|work| {
                super::symbol_noise_lines::paint::plan(
                    pdf,
                    document_page_idx,
                    &ocr_result,
                    config,
                    &ocr_config_owned,
                    work,
                )
            })
            && let Some(retained) = planned.verify(Arc::clone(pdf), config, &ocr_config_owned).await
        {
            paint_retention.insert(page_idx, retained);
        }

        if let Some(ref mut elems) = ocr_result.ocr_elements {
            #[cfg(feature = "pdf")]
            let public_elements = {
                let (_, layout_height) = resolved_ocr_layout_dimensions(
                    &ocr_result.metadata,
                    encoded_batch[offset].2,
                    encoded_batch[offset].3,
                );
                let (public_elements, outcome) = public_ocr_elements_for_pdf_page(
                    elems,
                    base_ocr_config,
                    document_page_number,
                    layout_height,
                    page_margins,
                );
                if outcome.missing_geometry && !public_elements.is_empty() {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut margin_filter_warnings,
                        ocr_margin_filter_capability_warning(),
                    );
                }
                public_elements
            };
            #[cfg(not(feature = "pdf"))]
            let public_elements = {
                for elem in elems.iter_mut() {
                    elem.page_number = document_page_number;
                }
                filter_public_ocr_elements(elems, base_ocr_config)
            };
            all_ocr_elements.extend(public_elements);
        }

        for mut formula in ocr_result.formulas {
            formula.page = Some(document_page_number);
            #[cfg(feature = "pdf")]
            if let Some((doc, _, _)) = lazy_pdf_render_state.as_ref() {
                let (w, h) = (encoded_batch[offset].2, encoded_batch[offset].3);
                formula_bbox_to_page_points(&mut formula, doc, page_idx, Some(&ocr_result.metadata), w, h);
            }
            accumulated_formulas.push(formula);
        }

        // force_ocr image-XObject fallback (#1355): xberg_native_pdf can catch an
        // image-decode error internally and substitute a blank white bitmap for
        // the whole-page render, so the page comes back from OCR as blank with no
        // indication anything was wrong. When that happens and the page actually
        // carries image XObjects, retry OCR directly on the embedded image bytes
        // (decoded pixels re-encoded to PNG, or the raw JPEG/JP2 stream) and always
        // surface a warning so the silent drop becomes visible.
        //
        // #1444 widened when this runs: a page whose backend call *failed* arrives here
        // as an empty document (see `reads.errors`), and a page the backend merely
        // *described* as blank is caught by the ink probe in
        // `page_needs_xobject_fallback` -- both used to sail past this block.
        #[cfg(feature = "pdf")]
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        // Set when the embedded-image retry ran (attempted at least one image) but the
        // backend returned successfully with empty content on every one of them (#1673).
        let mut xobject_retry_ran_empty = false;
        let xobject_trigger = if has_stored_scan {
            super::rendering::XObjectFallbackTrigger {
                needs_fallback: false,
                draw_failed: false,
            }
        } else {
            page_needs_xobject_fallback(&ocr_result.content, encoded_batch[offset].1.as_slice(), security_limits)
        };
        if xobject_trigger.needs_fallback {
            // The layout-detection route hands in pre-rendered `images`, which leaves
            // `lazy_pdf_render_state` unopened; that used to disable this fallback
            // entirely for exactly the scanned documents most likely to need it. The
            // fallback needs nothing from the render state but the page's XObject
            // table, so it opens its own handle rather than perturbing the rotation and
            // points-per-pixel lookups that state is indexed for (#1444).
            let render_doc = match lazy_pdf_render_state.as_ref() {
                Some((doc, _, _)) => Some(doc.as_ref()),
                None => xobject_document.or_else(|| fallback_render_document(&mut fallback_pdf_state, content)),
            };
            if let Some(render_doc) = render_doc
                && let Some(recovery) = recover_page_text_from_image_xobjects(
                    &backend,
                    render_doc,
                    document_page_idx,
                    progress_total_pages,
                    &ocr_config_owned,
                    XObjectRecoveryContext {
                        budget: &mut xobject_recovery_budget,
                        blank_raster_warnings: xobject_trigger.draw_failed.then_some(&mut image_fallback_warnings),
                        no_admitted_images: can_prove_no_admitted_images.then_some(&mut no_admitted_images),
                        stored_jpegs: None,
                        cancel_token: config.cancel_token.as_ref(),
                        use_proven_source_dpi: false,
                        proven_read: None,
                        emit_progress: true,
                    },
                )
                .await?
            {
                let XObjectRecoveryOutcome {
                    text,
                    text_outside_tables,
                    attempted,
                    images,
                    mut llm_usage,
                    mut tables,
                    mut formulas,
                    image_preprocessing,
                    tesseract_psm: _,
                    failure_causes: _,
                    warnings: _,
                } = recovery;
                let recovered_tables = !tables.is_empty();
                let recovered_payload = !text.trim().is_empty() || recovered_tables || !formulas.is_empty();
                if should_adopt_xobject_retry_text(&ocr_result.content, &text) {
                    ocr_result.content = text;
                    retry_texts_outside_tables[page_idx] = Some(text_outside_tables);
                    // The page text is now the retry text, not the lines the tables of this
                    // pass claimed.
                    if let Some(document) = ocr_result.ocr_internal_document.as_mut() {
                        document.ocr_text_claimed_by_tables = false;
                    }
                } else if recovered_tables {
                    retry_texts_outside_tables[page_idx] = Some(text_outside_tables);
                } else if attempted > 0 && !recovered_payload {
                    xobject_retry_ran_empty = true;
                }
                pages_with_recovered_xobject_payload[page_idx] = recovered_payload;
                accumulated_llm_usage.append(&mut llm_usage);
                collected_tables.append(&mut tables);
                accumulated_formulas.append(&mut formulas);
                if let Some(metadata) = image_preprocessing {
                    preprocessing_by_page.insert(document_page_number, metadata);
                }
                if capture_rasters {
                    captured_rasters.extend(images);
                }
                image_fallback_warnings.push(xobject_fallback_warning(
                    document_page_idx,
                    attempted,
                    xobject_trigger.draw_failed,
                ));
            }
        }

        // Report a page whose backend call failed, now that the fallback above has had
        // its chance to recover it (#1444). Either way the failure is visible: a page
        // that vanishes silently is the defect this replaces.
        if let Some((backend_error, error)) = reads.errors[offset].take() {
            let recovered = !ocr_result.content.trim().is_empty() || pages_with_recovered_xobject_payload[page_idx];
            if !recovered && xobject_retry_ran_empty {
                pages_with_empty_xobject_retry.push(document_page_number);
            }
            let warning = crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(if recovered {
                    let recovered_kind = if ocr_result.content.trim().is_empty() {
                        "content"
                    } else {
                        "text"
                    };
                    format!(
                        "OCR of page {} failed ({error}); its {recovered_kind} was recovered from the page's \
                         embedded image XObjects instead.",
                        document_page_number
                    )
                } else {
                    unread_page_failure_message(document_page_number, &error, xobject_retry_ran_empty)
                }),
            };
            pending_page_failure_warnings.push((page_idx, recovered, warning));
            page_failures.push(crate::types::OcrPageFailure {
                page: document_page_number,
                error,
                recovered,
            });
            page_backend_errors.push(backend_error);
        }

        // Both are read by the `pdf` margin-warning/page-content code below in every
        // feature set, but only written inside the two assembly blocks -- so with
        // `layout-detection` and neither OCR frontend they stay at their initializers.
        // ~keep
        #[cfg(feature = "pdf")]
        #[cfg_attr(
            all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
            allow(unused_mut)
        )]
        let mut margin_filtered_content: Option<String> = None;
        #[cfg(feature = "pdf")]
        #[cfg_attr(
            all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
            allow(unused_mut)
        )]
        let mut margin_filter_complete = false;

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let number_page = super::number_recovery::PreparedPage {
            policy: super::number_recovery::PagePolicy {
                page_index_offset: if detached_number_source.is_some() {
                    0
                } else {
                    page_index_offset
                },
                accepted: false,
                scale_only: reads.number_frames[offset],
                deferred_structure: skip_document_global_heuristic && detached_number_source.is_none(),
                raster: detached_number_source
                    .and_then(|source| source.raster.as_ref())
                    .or_else(|| raster_dpis.get(&page_idx)),
                layout: has_layout_detections,
                transformed: tall_source_used
                    || reads.upright_correction[offset].0 != 0
                    || pages_with_recovered_xobject_payload[page_idx],
                structured: false,
                builtin: get_ocr_backend_registry().read().is_builtin_tesseract(&backend),
            },
            source: detached_number_source
                .and_then(|source| {
                    source
                        .layer(document_page_number)
                        .map(|layer| (Arc::clone(&source.document), layer))
                })
                .or_else(|| {
                    lazy_pdf_render_state.as_ref().map(|(pdf, _, _)| Arc::clone(pdf)).zip(
                        page_ocr_hints
                            .as_ref()
                            .and_then(|hints| hints.native_layer(document_page_number)),
                    )
                })
                .map(|(document, layer)| super::number_recovery::PageSource {
                    document,
                    index: document_page_idx,
                    layer,
                    raster: Arc::clone(&encoded_batch[offset].1),
                    dimensions: (encoded_batch[offset].2, encoded_batch[offset].3),
                    metadata: &ocr_result.metadata,
                }),
            config: number_config,
            run: &number_recovery,
        };

        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
        if ocr_result.ocr_internal_document.is_some()
            || ocr_result
                .ocr_elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty())
        {
            let elements = ocr_result.ocr_elements.as_deref().unwrap_or_default();
            let detection = layout_detections.and_then(|detections| detections.get(page_idx));

            let ocr_render_width = encoded_batch[offset].2;
            let ocr_render_height = encoded_batch[offset].3;
            let render_scaled_detection =
                detection.map(|det| scale_detection_to_dimensions(det, ocr_render_width, ocr_render_height));
            let (ocr_layout_width, ocr_layout_height) =
                resolved_ocr_layout_dimensions(&ocr_result.metadata, ocr_render_width, ocr_render_height);
            ocr_page_heights[page_idx] = ocr_layout_height as f32;
            let layout_paragraph_scale = layout_page_points
                .filter(|_| ocr_layout_width > 0 && ocr_layout_height > 0)
                .map(|(width, height)| (width / ocr_layout_width as f32, height / ocr_layout_height as f32));
            let points_per_pixel = layout_paragraph_scale
                .or(detached_paragraph_scale)
                .map(|(_, sy)| sy)
                .or(points_per_pixel_override)
                .unwrap_or_else(|| {
                    ocr_points_per_pixel(
                        #[cfg(feature = "pdf")]
                        lazy_pdf_render_state.as_ref(),
                        page_idx,
                        ocr_layout_height,
                    )
                });
            let ocr_scaled_detection = detection.map(|det| {
                scale_detection_to_ocr_coordinates(det, &ocr_result.metadata, ocr_render_width, ocr_render_height)
            });
            let render_ocr_elements = transform_ocr_elements_to_render_space(
                elements,
                &ocr_result.metadata,
                ocr_render_width,
                ocr_render_height,
            );
            // ~keep A `None` transform means the word boxes cannot be placed in the raster
            // TATR crops from. Running TATR anyway loses every word to the cell-overlap
            // filter and publishes a table holding none of the page's values, so skip the
            // table and let the words stay in the page's paragraph stream
            // (xberg-io/xberg#1813).
            if render_ocr_elements.is_none() && !elements.is_empty() {
                crate::core::diagnostics::push_warning_deduped(
                    &mut backend_page_warnings,
                    crate::types::ProcessingWarning {
                        source: std::borrow::Cow::Borrowed("ocr"),
                        message: std::borrow::Cow::Borrowed(
                            "OCR word geometry could not be mapped onto the rendered page, so layout-based table \
                             recognition was skipped for a page; its text is reported as paragraphs instead.",
                        ),
                    },
                );
            }

            let recognized_tables = if let (Some(scaled_det), Some(render_ocr_elements), true) =
                (render_scaled_detection, render_ocr_elements, tatr_model.is_some())
            {
                let rgb = if let Some(ref slice) = batch_slice {
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    crate::extraction::image_decode::clone_dynamic_image_to_rgb8_with_security_limits(
                        &slice[offset],
                        security_limits,
                    )?
                } else {
                    let png_data = &encoded_batch[offset].1;
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(
                        png_data,
                        security_limits,
                    )
                    .map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to decode PNG for TATR: {}", e),
                        source: None,
                    })?
                };
                let mut model = tatr_model.take().expect("checked tatr_model.is_some() above");
                // #1812: TATR inference is synchronous ONNX Runtime CPU work with no await
                // points; running it inline would occupy a tokio worker thread for the
                // call's full duration on every table-bearing page. Route through
                // `spawn_blocking` and hand the lease back through the join so the next
                // page reuses it instead of it sitting checked out on a parked task. ~keep
                let span = tracing::Span::current();
                let (model, outcome) = tokio::task::spawn_blocking(move || {
                    let _entered = span.entered();
                    let outcome = crate::ocr::layout_assembly::recognize_page_tables_with_fallback(
                        &rgb,
                        &scaled_det,
                        &render_ocr_elements,
                        &mut model,
                    );
                    (model, outcome)
                })
                .await
                .map_err(|e| crate::XbergError::Plugin {
                    message: format!("TATR table recognition panicked: {}", e),
                    plugin_name: "layout".to_string(),
                })?;
                tatr_model = Some(model);
                outcome
            } else {
                crate::ocr::layout_assembly::RecognizedTablesOutcome::default()
            };

            let table_frame = if let Some((pdf, _, _)) = lazy_pdf_render_state.as_ref() {
                RecognizedTableFrame::PdfPage {
                    raster: (ocr_render_width, ocr_render_height),
                    points: page_dimensions_pt(pdf, page_idx),
                }
            } else if xobject_document.is_some() {
                RecognizedTableFrame::DetachedPdfRaster {
                    height: ocr_render_height,
                }
            } else if let Some(points) = layout_page_points {
                RecognizedTableFrame::PdfPage {
                    raster: (ocr_render_width, ocr_render_height),
                    points,
                }
            } else {
                RecognizedTableFrame::ImagePixels
            };
            append_recognized_tables(
                &recognized_tables,
                document_page_number,
                table_frame,
                &mut collected_tables,
                &mut collected_table_grids,
            );

            if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
                // Same lookup as the per-page `page_rotation_degrees` computed above for
                // the backend call (#760) -- that one is scoped to the join_set/backend
                // loop and does not survive to here, so it is recomputed identically:
                // an explicit override wins, otherwise the PDF's own known `/Rotate` for
                // this page (falling back to the image-derived rotation when the render
                // state was never opened), defaulting to `0` when nothing is known.
                #[cfg(feature = "pdf")]
                let page_rotation_degrees = if page_rotation_override != 0 {
                    page_rotation_override
                } else {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(_, _, rotations)| rotations.get(page_idx))
                        .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(page_idx)))
                        .copied()
                        .unwrap_or(0)
                };
                #[cfg(not(feature = "pdf"))]
                let page_rotation_degrees: u32 = 0;
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let mut column_order = reads.column_orders[offset].take();
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let mut ordered_early = false;
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let mut order_lines = |lines: &[crate::pdf::structure::types::PdfParagraph]| {
                    let cancel = config.cancel_token.as_ref();
                    if page_rotation_degrees != 0
                        || page_margins.top != 0.0
                        || page_margins.bottom != 0.0
                        || !recognized_tables.unrecognized_table_text.is_empty()
                        || config
                            .content_filter
                            .as_ref()
                            .is_some_and(|filter| filter.strip_repeating_text)
                        || cancel.is_some_and(|token| token.is_cancelled() || token.time_limit_reached())
                    {
                        return None;
                    }
                    let order = column_order.as_ref()?.before_grouping(
                        lines,
                        &ocr_result.metadata,
                        (ocr_layout_width, ocr_layout_height),
                    )?;
                    ordered_early = true;
                    Some(order)
                };
                let mut paragraphs = assemble_ocr_page_paragraphs(
                    ocr_doc,
                    ocr_layout_height,
                    ocr_scaled_detection.as_ref(),
                    points_per_pixel,
                    page_rotation_degrees,
                    #[cfg(all(
                        feature = "ocr",
                        feature = "layout-detection",
                        feature = "tokio-runtime",
                        not(target_arch = "wasm32")
                    ))]
                    Some(&mut order_lines),
                    #[cfg(all(feature = "ocr", any(not(feature = "tokio-runtime"), target_arch = "wasm32")))]
                    None,
                );
                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                if ordered_early {
                    let cancel = config.cancel_token.as_ref();
                    if cancel.is_some_and(|token| token.is_cancelled()) {
                        return Err(crate::XbergError::Cancelled.into());
                    }
                    // Reordering can change list detection, which is a gate for the
                    // document-global pass that rebuilds paragraphs without layout hints.
                    let mut original = assemble_ocr_page_paragraphs(
                        ocr_doc,
                        ocr_layout_height,
                        ocr_scaled_detection.as_ref(),
                        points_per_pixel,
                        page_rotation_degrees,
                        None,
                    );
                    if cancel.is_some_and(|token| token.is_cancelled()) {
                        return Err(crate::XbergError::Cancelled.into());
                    }
                    if cancel.is_some_and(|token| token.time_limit_reached()) {
                        paragraphs = original;
                        super::column_retry::note_deadline(&mut backend_page_warnings, cancel);
                    } else {
                        ordered_layout_pages[page_idx] =
                            original_ocr_layout_blocks_reconstruction(&mut original, config);
                        column_lanes_by_page[page_idx] = column_order.as_mut().and_then(|order| order.lanes.take());
                    }
                    column_order = None;
                }
                append_unrecognized_table_fallback_paragraphs(
                    &mut paragraphs,
                    &recognized_tables.unrecognized_table_text,
                );
                apply_ocr_layout_content_filter(&mut paragraphs, config);
                #[cfg(feature = "pdf")]
                {
                    let outcome =
                        filter_ocr_paragraphs_by_page_margins(&mut paragraphs, ocr_layout_height as f32, page_margins);
                    margin_filter_complete = !outcome.missing_geometry;
                    // #1574: `page_margins` is now non-zero only when the caller set
                    // `top_margin_fraction`/`bottom_margin_fraction` explicitly (defaults
                    // resolve to 0.0, so this branch cannot fire on a default config). A
                    // successful removal here is therefore the caller's own documented
                    // filter working as configured -- the native-PDF span filter
                    // (`extraction.rs`'s use of the same `PageMarginFractions`) emits no
                    // warning for the identical case, and `diagnostics.rs`'s house rule is
                    // to stay silent for a deliberate, documented filter unless the caller
                    // could plausibly believe the content was extracted. It was not: the
                    // caller asked for it to be removed. `outcome.missing_geometry` (the
                    // capability gap where the filter could NOT run) is the one case that
                    // still warns, below. ~keep
                    if outcome.removed {
                        margin_filtered_content = Some(ocr_paragraphs_plain_text(&paragraphs));
                    }
                }

                #[cfg(all(
                    feature = "ocr",
                    feature = "layout-detection",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                if let Some(order) = column_order
                    && recognized_tables.unrecognized_table_text.is_empty()
                {
                    let cancel = config.cancel_token.as_ref();
                    let reordered = cancel
                        .is_none_or(|token| !token.is_cancelled() && !token.time_limit_reached())
                        .then(|| {
                            order.reorder(&paragraphs, &ocr_result.metadata, (ocr_layout_width, ocr_layout_height))
                        })
                        .flatten();
                    if cancel.is_some_and(|token| token.is_cancelled()) {
                        return Err(crate::XbergError::Cancelled.into());
                    }
                    if cancel.is_some_and(|token| token.time_limit_reached()) {
                        let mut warnings = Vec::new();
                        super::column_retry::note_deadline(&mut warnings, cancel);
                        crate::core::diagnostics::dedup_extend_warnings(&mut backend_page_warnings, warnings);
                    } else if let Some(reordered) = reordered {
                        paragraphs = reordered;
                    }
                }

                tracing::debug!(
                    page = document_page_number,
                    paragraphs = paragraphs.len(),
                    raw_content_len = ocr_result.content.len(),
                    "OCR page layout classification complete"
                );

                // ~keep Margin filtering uses OCR pixels. Assembly compares paragraph bounds
                // directly with the table bounds, which are already in MediaBox points.
                if let Some(scale) = layout_paragraph_scale {
                    detached_mixed_frame::scale_paragraph_geometry(&mut paragraphs, scale);
                    ocr_page_heights[page_idx] *= scale.1;
                }
                #[cfg(feature = "pdf")]
                {
                    paragraph_rasters[page_idx] = layout_paragraph_scale
                        .is_none()
                        .then_some((ocr_layout_width, ocr_layout_height));
                }
                all_page_paragraphs[page_idx] = Some(paragraphs);
                detached_page_scales[page_idx] = detached_paragraph_scale;
                page_ocr_scales[page_idx] = layout_paragraph_scale.map(|_| (1.0, 1.0)).or_else(|| {
                    run_page_ocr_scale(
                        #[cfg(feature = "pdf")]
                        lazy_pdf_render_state.as_ref(),
                        page_idx,
                        (ocr_render_width, ocr_render_height),
                    )
                });
                pages_text_claimed_by_tables[page_idx] = ocr_doc.ocr_text_claimed_by_tables;
            }

            if capture_rasters {
                let (_, png_arc, w, h) = &encoded_batch[offset];
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                captured_rasters.push(build_page_raster_image(document_page_idx, png_bytes, *w, *h));
            }
            let dict_invalid_word_ratio = ocr_result
                .metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
                .and_then(|v| v.as_f64());
            let confidence = mean_text_conf_of(&ocr_result.metadata.additional);
            if let Some(summary) = page_ocr_confidence(
                backend_confidence_semantics,
                confidence,
                word_count_of(&ocr_result.metadata.additional).unwrap_or(0),
                &backend_name,
            ) {
                ocr_confidence_by_page.insert(document_page_number, summary);
            }
            #[cfg(feature = "pdf")]
            if (page_margins.top != 0.0 || page_margins.bottom != 0.0)
                && !margin_filter_complete
                && !ocr_result.content.trim().is_empty()
            {
                crate::core::diagnostics::push_warning_deduped(
                    &mut margin_filter_warnings,
                    ocr_margin_filter_capability_warning(),
                );
            }
            #[cfg(feature = "pdf")]
            let page_content = margin_filtered_content.unwrap_or(ocr_result.content);
            #[cfg(not(feature = "pdf"))]
            let page_content = ocr_result.content;
            let acceptance = accept_or_reject_ocr_page(
                document_page_idx,
                page_content,
                &ocr_output_thresholds,
                &mut recognition_noise_warnings,
                dict_invalid_word_ratio,
                backend_confidence_semantics,
                confidence,
            );
            page_texts[page_idx] = acceptance.content;
            rejected_pages[page_idx] = acceptance.discarded;
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            {
                let original = detached_number_source.is_some().then(|| page_texts[page_idx].clone());
                number_recovered_pages[page_idx] = number_page
                    .apply(
                        super::number_recovery::AcceptedRead {
                            accepted: !acceptance.discarded && acceptance.verdict.is_none(),
                            paragraphs: all_page_paragraphs[page_idx].as_deref_mut(),
                            text: &mut page_texts[page_idx],
                            warnings: &mut backend_page_warnings,
                        },
                        &collected_tables,
                        &accumulated_formulas,
                    )
                    .await;
                if number_recovered_pages[page_idx] && original.is_some() {
                    number_primary_text = original;
                }
            }

            if let Some(verdict) = acceptance.verdict {
                recognition_noise_verdicts.push(verdict);
            }
            return Ok(Self {
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
            });
        }

        #[cfg(not(feature = "layout-detection"))]
        if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
            let ocr_render_width = encoded_batch[offset].2;
            let ocr_render_height = encoded_batch[offset].3;
            let raster_px = resolved_ocr_layout_dimensions(&ocr_result.metadata, ocr_render_width, ocr_render_height);
            let ocr_layout_height = raster_px.1;
            ocr_page_heights[page_idx] = ocr_layout_height as f32;
            let points_per_pixel = detached_paragraph_scale
                .map(|(_, sy)| sy)
                .or(points_per_pixel_override)
                .unwrap_or_else(|| {
                    ocr_points_per_pixel(
                        #[cfg(feature = "pdf")]
                        lazy_pdf_render_state.as_ref(),
                        page_idx,
                        ocr_layout_height,
                    )
                });
            // `ocr_doc`'s bbox AND ocr_geometry are both still raw OCR raster pixels here
            // (same pure-OCR-route reasoning as `assemble_ocr_page_paragraphs` above).
            let font_size_scale = crate::pdf::structure::adapters::OcrFontSizeScale::uniform(points_per_pixel);
            let mut paragraphs =
                crate::pdf::structure::adapters::ocr_doc_to_paragraphs(ocr_doc, ocr_layout_height, font_size_scale);
            apply_ocr_layout_content_filter(&mut paragraphs, config);
            #[cfg(feature = "pdf")]
            {
                let outcome =
                    filter_ocr_paragraphs_by_page_margins(&mut paragraphs, ocr_layout_height as f32, page_margins);
                margin_filter_complete = !outcome.missing_geometry;
                if outcome.removed {
                    margin_filtered_content = Some(ocr_paragraphs_plain_text(&paragraphs));
                }
            }
            #[cfg(feature = "pdf")]
            {
                paragraph_rasters[page_idx] = Some(raster_px);
            }
            all_page_paragraphs[page_idx] = Some(paragraphs);
            detached_page_scales[page_idx] = detached_paragraph_scale;
            page_ocr_scales[page_idx] = run_page_ocr_scale(
                #[cfg(feature = "pdf")]
                lazy_pdf_render_state.as_ref(),
                page_idx,
                (ocr_render_width, ocr_render_height),
            );
            pages_text_claimed_by_tables[page_idx] = ocr_doc.ocr_text_claimed_by_tables;
        }

        let _ = page_idx;
        if capture_rasters {
            let (_, png_arc, w, h) = &encoded_batch[offset];
            let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
            captured_rasters.push(build_page_raster_image(document_page_idx, png_bytes, *w, *h));
        }
        let dict_invalid_word_ratio = ocr_result
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
            .and_then(|v| v.as_f64());
        let confidence = mean_text_conf_of(&ocr_result.metadata.additional);
        if let Some(summary) = page_ocr_confidence(
            backend_confidence_semantics,
            confidence,
            word_count_of(&ocr_result.metadata.additional).unwrap_or(0),
            &backend_name,
        ) {
            ocr_confidence_by_page.insert(document_page_number, summary);
        }
        #[cfg(feature = "pdf")]
        if (page_margins.top != 0.0 || page_margins.bottom != 0.0)
            && !margin_filter_complete
            && !ocr_result.content.trim().is_empty()
        {
            crate::core::diagnostics::push_warning_deduped(
                &mut margin_filter_warnings,
                ocr_margin_filter_capability_warning(),
            );
        }
        #[cfg(feature = "pdf")]
        let page_content = margin_filtered_content.unwrap_or(ocr_result.content);
        #[cfg(not(feature = "pdf"))]
        let page_content = ocr_result.content;
        let acceptance = accept_or_reject_ocr_page(
            document_page_idx,
            page_content,
            &ocr_output_thresholds,
            &mut recognition_noise_warnings,
            dict_invalid_word_ratio,
            backend_confidence_semantics,
            confidence,
        );
        page_texts[page_idx] = acceptance.content;
        rejected_pages[page_idx] = acceptance.discarded;
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        {
            let original = detached_number_source.is_some().then(|| page_texts[page_idx].clone());
            number_recovered_pages[page_idx] = number_page
                .apply(
                    super::number_recovery::AcceptedRead {
                        accepted: !acceptance.discarded && acceptance.verdict.is_none(),
                        paragraphs: all_page_paragraphs[page_idx].as_deref_mut(),
                        text: &mut page_texts[page_idx],
                        warnings: &mut backend_page_warnings,
                    },
                    &collected_tables,
                    &accumulated_formulas,
                )
                .await;
            if number_recovered_pages[page_idx] && original.is_some() {
                number_primary_text = original;
            }
        }

        if let Some(verdict) = acceptance.verdict {
            recognition_noise_verdicts.push(verdict);
        }

        Ok(Self {
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
        })
    }
}
