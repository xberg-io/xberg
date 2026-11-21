// Each phase binds the whole context by value and writes only the fields its own feature leg
// touches; which of those bindings need `mut` therefore differs per build. ~keep
#![allow(unused_mut)]

use super::super::*;
use super::*;
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use rayon::prelude::*;
use std::io::Cursor;
use std::sync::Arc;

impl<'a> PageOcrContext<'a> {
    /// Render this batch's pages, or encode the caller's own images, and report their bytes.
    pub(super) async fn render_batch(self, batch_start: usize) -> OcrRunResult<(Self, RenderedBatch<'a>)> {
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

        let batch_end = (batch_start + batch_size).min(total_pages);

        #[allow(unused_variables)]
        let (batch_slice, encoded) = if has_stored_scan {
            (None, Vec::new())
        } else if let Some(imgs) = images {
            let slice: Cow<'_, [image::DynamicImage]> = Cow::Borrowed(&imgs[batch_start..batch_end]);
            let default_security_limits = crate::extractors::security::SecurityLimits::default();
            let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
            // One page at a time. `max_content_size` bounds what a single page may cost to
            // render and encode; summing a whole batch against it made that per-image ceiling a
            // function of the thread budget. On the parallel encode path below, five US Letter
            // pages at the default 150 dpi already cross the default limit, so an eight-page
            // batch was refused whole and every page the caller had already rendered was
            // dropped (#1731). Matches the per-page check the mixed native-and-OCR route runs
            // at its own encode step. The batch's own footprint is bounded by `batch_size`,
            // which the thread budget decides; `max_content_size` does not bound it. ~keep
            validate_png_encode_pages_individually(slice.iter(), security_limits)?;
            #[allow(clippy::type_complexity)]
            #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .par_iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    let rgb_image = clone_rgb_for_png_encode(image, security_limits)?;
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            #[allow(clippy::type_complexity)]
            #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    let rgb_image = clone_rgb_for_png_encode(image, security_limits)?;
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            (Some(slice), encoded?)
        } else {
            #[cfg(feature = "pdf")]
            let encoded = {
                let (doc, _, page_rotations) =
                    lazy_pdf_render_state
                        .as_ref()
                        .ok_or_else(|| crate::XbergError::Parsing {
                            message: "PDF content is required for OCR rendering but was not provided".to_string(),
                            source: None,
                        })?;
                let default_security_limits = crate::extractors::security::SecurityLimits::default();
                let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                let rendered = crate::pdf::render::capture_render_warnings(|| {
                    render_full_pdf_ocr_batch_with_dpi(
                        doc,
                        page_rotations,
                        batch_start..batch_end,
                        security_limits,
                        config.images.as_ref(),
                        preprocessing_render_dpi,
                    )
                });
                let ((pages, batch_raster_dpis), warnings) = match rendered {
                    Ok(rendered) => rendered,
                    Err(error) => {
                        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                        let error = diagnose_djot_budget_refusal(
                            error,
                            doc,
                            config,
                            &backend,
                            &ocr_config_owned,
                            &mut xobject_recovery_budget,
                        )
                        .await?;
                        return Err(error.into());
                    }
                };
                crate::core::diagnostics::dedup_extend_warnings(&mut render_warnings, warnings);
                raster_dpis.extend(batch_raster_dpis);
                pages
            };
            #[cfg(not(feature = "pdf"))]
            let encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> = Vec::new();
            (None::<Cow<'_, [image::DynamicImage]>>, encoded)
        };

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
            RenderedBatch {
                slice: batch_slice,
                encoded,
            },
        ))
    }
}
