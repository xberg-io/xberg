use super::super::*;
use super::*;
use crate::plugins::registry::get_ocr_backend_registry;

impl PageOcrContext<'_> {
    /// Assemble the document the page batches read, dropping what the quality gates rejected.
    pub(super) async fn finish(self) -> OcrRunResult<PageOcrOutput> {
        let Self {
            content,
            images,
            #[cfg(feature = "layout-detection")]
            mut layout_detections,
            config,
            page_rotation_override,
            skip_document_global_heuristic,
            points_per_pixel_override: _,
            page_ocr_hints,
            page_index_offset,
            detached_page,
            xobject_document,
            #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
            mut stored_scan,
            has_stored_scan: _,
            has_layout_detections,
            resolved_ocr_config,
            ocr_config_owned: _,
            ocr_output_thresholds,
            backend,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                builtin_tesseract: _,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            can_retry_orientation,
            can_prove_no_admitted_images: _,
            no_admitted_images,
            backend_confidence_semantics: _,
            backend_name: _,
            backend_confidence_scale,
            #[cfg(feature = "pdf")]
                orientation_handling: _,
            #[cfg(feature = "pdf")]
                page_margins: _,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
                tall_source_unsupported: _,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
                try_tall_source: _,
            #[cfg(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            tall_ordered_primary,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                force_khmer_retry: _,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            mut forced_khmer_supplements,
            preprocessing_render_dpi: _,
            capture_rasters,
            #[cfg(feature = "pdf")]
            lazy_pdf_render_state,
            lazy_pdf_page_count: _,
            batch_size: _,
            total_pages,
            progress_total_pages: _,
            #[cfg(feature = "pdf")]
            external_image_page_geometry,
            #[cfg(feature = "pdf")]
                fallback_pdf_state: _,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            mut leading_khmer_rows,
            mut page_texts,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            paint_retention,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            background_noise,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                certified_background: _,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                paint_work: _,
            rejected_pages,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            owned_scan_rows,
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
            page_ocr_scales,
            detached_page_scales,
            #[cfg(feature = "pdf")]
            paragraph_rasters,
            mut public_ocr_pages,
            ocr_page_heights,
            mut pages_text_claimed_by_tables,
            retry_texts_outside_tables,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            caption,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                capture_caption: _,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            detached_table_pair,
            #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
            mut collected_table_grids,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                orientation_retry: _,
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
            mut region_replay_raster,
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
                number_recovery: _,
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            number_primary_text,
            page_segmentation_modes,
            backend_additional_metadata,
            mut collected_tables,
            mut all_ocr_elements,
            accumulated_llm_usage,
            mut accumulated_formulas,
            conf_sum,
            conf_count,
            ocr_confidence_by_page,
            captured_rasters,
            preprocessing_by_page,
            #[cfg(feature = "pdf")]
            image_fallback_warnings,
            #[cfg(feature = "pdf")]
                xobject_recovery_budget: _,
            page_backend_errors,
            mut page_failure_warnings,
            pending_page_failure_warnings,
            mut page_failures,
            pages_with_empty_xobject_retry,
            pages_with_recovered_xobject_payload,
            #[cfg(feature = "pdf")]
            margin_filter_warnings,
            #[cfg(feature = "pdf")]
            render_warnings,
            #[cfg(feature = "pdf")]
            raster_dpis,
            mut backend_page_warnings,
            mut recognition_noise_warnings,
            recognition_noise_verdicts,
            #[cfg(feature = "layout-detection")]
            mut tatr_model,
        } = self;
        let base_ocr_config = resolved_ocr_config.as_ref();
        #[cfg(feature = "pdf")]
        let external_image_page_rotations: Option<&[u32]> = external_image_page_geometry
            .as_ref()
            .map(|geometry| geometry.rotations.as_slice());

        for (page_idx, recovered, warning) in pending_page_failure_warnings {
            if recovered && rejected_pages[page_idx] {
                let page = (page_index_offset + page_idx + 1) as u32;
                for failure in page_failures.iter_mut().filter(|failure| failure.page == page) {
                    failure.recovered = false;
                    page_failure_warnings.push(discarded_xobject_retry_warning(failure, false));
                }
            } else {
                page_failure_warnings.push(warning);
            }
        }

        if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
            return Err(crate::XbergError::Cancelled.into());
        }

        // Degrading a per-page failure to a warning must not turn a wholesale OCR failure into a
        // silently empty document: when every page failed *and* nothing was recovered from any
        // page's embedded images, there is no partial result to preserve, so report it (#1444).
        if !page_backend_errors.is_empty()
            && page_backend_errors.len() == total_pages
            && page_texts.iter().all(|text| text.trim().is_empty())
            && pages_with_recovered_xobject_payload.iter().all(|recovered| !recovered)
        {
            let first_error = page_backend_errors
                .into_iter()
                .next()
                .expect("at least one page failed");
            let first_failure = describe_page_failure(&first_error);
            let headline = format!("OCR failed on all {total_pages} page(s)");
            let (message, cli_message) = if pages_with_empty_xobject_retry.is_empty() {
                let summary = format!("{headline} and no text could be recovered from the pages' embedded images");
                (format!("{summary}; first failure: {first_failure}"), summary)
            } else {
                let retry = format!(
                    "The embedded-image retry ran on {} of these page(s) and also returned no text.",
                    pages_with_empty_xobject_retry.len()
                );
                (
                    format!("{headline}; first failure: {first_failure}. {retry}"),
                    format!("{headline}. {retry}"),
                )
            };
            let error = crate::XbergError::all_page_ocr_failure(message, cli_message, first_error);
            return Err(FailedOcrRun {
                error,
                page_failures,
                unrecovered_glyphs_on_page_one: 0,
            });
        }

        let mean_text_conf = match (conf_count > 0, backend_confidence_scale) {
            (true, Some(scale_max)) => Some((conf_sum / conf_count as f64) / scale_max),
            _ => None,
        };

        // Pages the verdict flagged but kept lose their symbol-noise lines only now, so the
        // verdict, backend choice and table acceptance above judged the unpruned text (#2312).
        // The pruned text is what this function returns; pipeline stage scoring reads
        // `OcrReadEvidence::unpruned_text` instead, so pruning never changes stage selection.
        let mut unpruned_page_texts: Vec<(usize, String)> = Vec::new();
        for verdict in recognition_noise_verdicts.iter().filter(|verdict| !verdict.discarded) {
            let Some(page_idx) = verdict.page_index.checked_sub(page_index_offset) else {
                continue;
            };
            let page_number = (verdict.page_index + 1) as u32;
            if page_idx >= page_texts.len()
                || rejected_pages[page_idx]
                || collected_tables.iter().any(|table| table.page_number == page_number)
            {
                continue;
            }
            if ocr_output_thresholds.prune_symbol_noise_lines
                && super::symbol_noise_lines::page_is_dominated_by_noise_lines(page_texts[page_idx].lines())
            {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let retained = paint_retention.get(&page_idx).and_then(|evidence| {
                    evidence.reconcile(&page_texts[page_idx]).filter(|word| {
                        all_page_paragraphs[page_idx].as_ref().is_none_or(|paragraphs| {
                            super::symbol_noise_lines::paint::paragraphs_own_line(paragraphs, word)
                        })
                    })
                });
                #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
                let retained = None;
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                if !config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
                    && public_ocr_pages.get(&page_number).is_none_or(|doc| {
                        doc.tables.is_empty() && super::symbol_noise_lines::elements_are_prunable(doc)
                    })
                    && let Some(word) = background_noise
                        .get(&page_idx)
                        .and_then(|evidence| evidence.reconcile(&page_texts[page_idx]))
                {
                    let original = page_texts[page_idx].clone();
                    let (removed, background_removed) = super::symbol_noise_lines::texture::prune(
                        &mut page_texts[page_idx],
                        None,
                        all_page_paragraphs[page_idx].as_mut(),
                        public_ocr_pages.get_mut(&page_number),
                        &[word],
                        retained,
                    );
                    unpruned_page_texts.push((page_idx, original));
                    recognition_noise_warnings.push(super::symbol_noise_lines::pruned_lines_warning(
                        verdict.page_index + 1,
                        removed,
                    ));
                    if background_removed != 0 {
                        recognition_noise_warnings
                            .push(super::symbol_noise_background::warning(page_number, background_removed));
                    }
                    continue;
                }
                let (pruned, removed) =
                    super::symbol_noise_lines::prune_text_with_retention(&page_texts[page_idx], retained);
                unpruned_page_texts.push((page_idx, std::mem::replace(&mut page_texts[page_idx], pruned)));
                if let Some(paragraphs) = all_page_paragraphs[page_idx].as_mut() {
                    super::symbol_noise_lines::prune_paragraphs_with_retention(paragraphs, retained);
                }
                recognition_noise_warnings.push(super::symbol_noise_lines::pruned_lines_warning(
                    verdict.page_index + 1,
                    removed,
                ));
            }
        }

        let marker_format = super::super::super::super::ocr_page_marker_format(config);
        let join_pages = |texts: &[String]| super::document::join_run_pages(texts, marker_format, page_index_offset);
        #[cfg_attr(
            not(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            )),
            allow(unused_mut)
        )]
        let mut result = join_pages(&page_texts);
        #[cfg_attr(
            not(all(
                feature = "ocr",
                feature = "pdf",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            )),
            allow(unused_mut)
        )]
        let mut unpruned_text = (!unpruned_page_texts.is_empty()).then(|| {
            let mut texts = page_texts.clone();
            for (page_idx, original) in &unpruned_page_texts {
                texts[*page_idx] = original.clone();
            }
            join_pages(&texts)
        });

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if let Some(primary) = &number_primary_text {
            unpruned_text = Some(join_pages(std::slice::from_ref(primary)));
        }

        // A page the quality gate rejected must not keep its structured paragraphs either.
        //
        // The paragraphs and the page text are two renderings of the same OCR output, built by
        // different assemblies: `all_page_paragraphs` above, `page_texts` from
        // `accept_or_reject_ocr_page`. Only the text was ever judged, and the rendered document
        // is built exclusively from the paragraphs -- so a scanned survey plat lost its text,
        // logged its warning, and rendered its garbage anyway. The mixed route already couples
        // the two (`ocr_results.retain` feeding `merge_structured_ocr_pages_into_internal_document`);
        // this is that same coupling for the force-ocr route, which never had it.
        //
        // Clearing before `fill_unstructured_ocr_pages` is deliberate and safe: that function
        // only rebuilds paragraphs from `page_texts[page_index]`, which is empty for exactly
        // these pages, so it cannot resurrect them.
        for (page_idx, page_rejected) in rejected_pages.iter().enumerate() {
            if *page_rejected {
                all_page_paragraphs[page_idx] = None;
            }
        }
        discard_ocr_elements_from_rejected_pages(&mut all_ocr_elements, &rejected_pages, page_index_offset);
        discard_rejected_ocr_page_payloads(
            &mut collected_tables,
            &mut accumulated_formulas,
            &rejected_pages,
            page_index_offset,
        );

        #[cfg(feature = "pdf")]
        {
            let retained_payload_pages: ahash::AHashSet<u32> = collected_tables
                .iter()
                .map(|table| table.page_number)
                .chain(accumulated_formulas.iter().filter_map(|formula| formula.page))
                .collect();
            let retained_noise_pages: ahash::AHashSet<_> = recognition_noise_verdicts
                .iter()
                .filter(|verdict| !verdict.discarded)
                .map(|verdict| verdict.page_index)
                .collect();
            for (page_idx, text) in page_texts.iter().enumerate() {
                let document_page_index = page_index_offset + page_idx;
                if (text.trim().is_empty() || retained_noise_pages.contains(&document_page_index))
                    && !retained_payload_pages.contains(&((document_page_index + 1) as u32))
                    && let Some(dpi) = raster_dpis.get(&page_idx)
                    && let Some(warning) = dpi.empty_text_warning(document_page_index)
                {
                    crate::core::diagnostics::push_warning_deduped(&mut backend_page_warnings, warning);
                }
            }
        }

        // A page that adopted the retry text is refilled from the retry lines no recovered table
        // carries, and is claimed by its tables when they carry every line (#2014).
        let mut refill_texts = std::borrow::Cow::Borrowed(page_texts.as_slice());
        for (page_idx, text_outside_tables) in retry_texts_outside_tables.into_iter().enumerate() {
            if let Some(text_outside_tables) = text_outside_tables
                && !rejected_pages[page_idx]
            {
                pages_text_claimed_by_tables[page_idx] = text_outside_tables.trim().is_empty();
                refill_texts.to_mut()[page_idx] = text_outside_tables;
            }
        }
        // ~keep Detached PDF pages retain their PDF document; only standalone images reach
        // this fallback without either PDF source. Remove legacy table transcripts before
        // paragraph synthesis, preserving the raw OCR text and authoritative public nodes.
        if content.is_none() && xobject_document.is_none() {
            for (page_idx, paragraphs) in all_page_paragraphs.iter().enumerate() {
                let page = (page_index_offset + page_idx + 1) as u32;
                if paragraphs.as_ref().is_some_and(|paragraphs| !paragraphs.is_empty())
                    || public_ocr_pages.contains_key(&page)
                    || rejected_pages[page_idx]
                {
                    continue;
                }
                let text = &refill_texts[page_idx];
                let prose = super::strip_embedded_gfm_tables(
                    text,
                    collected_tables.iter().filter(|table| table.page_number == page),
                );
                if prose != *text {
                    pages_text_claimed_by_tables[page_idx] = prose.trim().is_empty();
                    refill_texts.to_mut()[page_idx] = prose;
                }
            }
        }
        // ~keep A standalone image has no later native-PDF table handoff. Complete cell-only
        // sidecars before assembly so every accepted table receives its canonical IR node.
        // This follows page rejection and transcript stripping, and precedes normalization
        // and authoritative public-node replacement (#2226).
        let standalone_image = content.is_none() && xobject_document.is_none();
        if standalone_image {
            for table in &mut collected_tables {
                if table.markdown.trim().is_empty() {
                    table.markdown = crate::rendering::common::render_table_markdown(&table.cells);
                }
            }
        }
        fill_unstructured_ocr_pages(&mut all_page_paragraphs, &refill_texts, &pages_text_claimed_by_tables);

        let (ocr_doc, mut raw_page_paragraphs) = {
            let has_structured =
                all_page_paragraphs
                    .iter()
                    .zip(&pages_text_claimed_by_tables)
                    .any(|(paragraphs, claimed_by_tables)| {
                        paragraphs.as_ref().is_some_and(|paragraphs| !paragraphs.is_empty()) || *claimed_by_tables
                    })
                    || (standalone_image && collected_tables.iter().any(|table| !table.markdown.trim().is_empty()));
            if has_structured {
                let pages: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> = all_page_paragraphs
                    .into_iter()
                    .map(|opt| opt.unwrap_or_default())
                    .collect();
                #[cfg(feature = "layout-detection")]
                let pages = {
                    let mut pages = pages;
                    crate::pdf::structure::adapters::promote_anchored_ordered_list_sequences(&mut pages);
                    pages
                };
                // Document-global heading/list heuristic: `ocr_doc_to_paragraphs` /
                // `ocr_doc_to_layout_paragraphs` (above) build `pages` one OCR page at a
                // time and never call it, which is why every non-layout OCR route has
                // emitted zero headings and zero list items regardless of output format.
                // `None` (Plain output, or pages ML layout already classified) falls
                // through to the pre-existing assembly unchanged. Skipped entirely when
                // `skip_document_global_heuristic` is set -- see this function's own doc
                // comment: the caller wants the *bare* `pages` back to run this pass
                // itself, document-wide, and a heuristic run here first would either be
                // wasted (too few pages to cluster) or would mark `pages` "already
                // structured" and block that wider pass from doing anything.
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let detached_table_document = (detached_table_pair
                    && pages.as_slice().first().is_some_and(Vec::is_empty)
                    && public_ocr_pages.is_empty()
                    && pages_with_recovered_xobject_payload == [false])
                .then(|| detached_table_pair::assemble(&collected_tables))
                .flatten();
                #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                let detached_table_document: Option<crate::types::internal::InternalDocument> = None;
                let doc = if detached_table_document.is_some() {
                    detached_table_document
                } else if skip_document_global_heuristic || {
                    #[cfg(all(
                        feature = "ocr",
                        feature = "layout-detection",
                        feature = "tokio-runtime",
                        not(target_arch = "wasm32")
                    ))]
                    {
                        ordered_layout_pages.iter().any(|&ordered| ordered)
                    }
                    #[cfg(not(all(
                        feature = "ocr",
                        feature = "layout-detection",
                        feature = "tokio-runtime",
                        not(target_arch = "wasm32")
                    )))]
                    {
                        false
                    }
                } {
                    Some(crate::pdf::structure::assemble_internal_document(
                        pages.clone(),
                        &collected_tables,
                        None,
                        &[],
                        &Default::default(),
                    ))
                } else {
                    match heuristically_restructured_ocr_pages(&pages, &ocr_page_heights, &collected_tables, config) {
                        Some(doc) => Some(doc),
                        None => {
                            // The document-global heading/list heuristic declined to run (Plain
                            // output, or a paragraph was already ML-layout-classified) or found
                            // nothing. Apply a text-marker-only list pass to the bare pages before
                            // falling back to plain assembly, so a non-layout OCR document that
                            // never reaches the heuristic still gets *some* list-item recovery
                            // (#713) -- this never touches `heading_level`, so it cannot regress
                            // heading detection the way pre-classifying before the heuristic call
                            // would (see `apply_ocr_text_list_fallback`'s own doc comment on the
                            // "already structured" gate this must not trip).
                            let mut fallback_pages = pages.clone();
                            for (page_idx, page) in fallback_pages.iter_mut().enumerate() {
                                apply_ocr_text_list_fallback(page);
                                // #729: recover a bare marker no ML hint ever classified --
                                // see `adapters::reattach_detached_ocr_list_markers`'s doc
                                // comment.
                                // #760: same rotation lookup as `assemble_ocr_page_paragraphs`'s
                                // call site above -- an explicit override wins, otherwise this
                                // page's own known PDF `/Rotate`, defaulting to `0`.
                                #[cfg(feature = "pdf")]
                                let page_rotation_degrees = if page_rotation_override != 0 {
                                    page_rotation_override
                                } else {
                                    lazy_pdf_render_state
                                        .as_ref()
                                        .and_then(|(_, _, rotations)| rotations.get(page_idx))
                                        .or_else(|| {
                                            external_image_page_rotations.as_ref().and_then(|r| r.get(page_idx))
                                        })
                                        .copied()
                                        .unwrap_or(0)
                                };
                                #[cfg(not(feature = "pdf"))]
                                let page_rotation_degrees: u32 = 0;
                                crate::pdf::structure::adapters::reattach_detached_ocr_list_markers(
                                    page,
                                    page_rotation_degrees,
                                );
                            }
                            // The document-global pass declined, so nothing above marked running
                            // furniture; the pages still need the cross-page pass the native route
                            // always runs, or a scanned header stays in the body.
                            let filter = config.content_filter.clone().unwrap_or_default();
                            if filter.strip_repeating_text {
                                crate::pdf::structure::pipeline::strip_repeating_text_from_pages(
                                    &mut fallback_pages,
                                    &ocr_page_heights,
                                    filter.include_headers,
                                    filter.include_footers,
                                );
                            }
                            Some(crate::pdf::structure::assemble_internal_document(
                                fallback_pages,
                                &collected_tables,
                                None,
                                &[],
                                &Default::default(),
                            ))
                        }
                    }
                };
                let pages = pages
                    .into_iter()
                    .zip(page_ocr_scales)
                    .zip(detached_page_scales)
                    .enumerate()
                    .map(
                        |(_page_index, ((paragraphs, ocr_scale), detached_ocr_scale))| RunPageParagraphs {
                            #[cfg(feature = "pdf")]
                            raster_px: paragraph_rasters[_page_index],
                            #[cfg(all(
                                feature = "pdf",
                                feature = "ocr",
                                feature = "tokio-runtime",
                                not(target_arch = "wasm32")
                            ))]
                            leading_khmer_rows: leading_khmer_rows.remove(&_page_index),
                            #[cfg(all(
                                feature = "ocr",
                                feature = "layout-detection",
                                feature = "tokio-runtime",
                                not(target_arch = "wasm32")
                            ))]
                            column_lanes: column_lanes_by_page[_page_index].take(),
                            preserve_ordered_layout: {
                                #[cfg(all(
                                    feature = "ocr",
                                    feature = "layout-detection",
                                    feature = "tokio-runtime",
                                    not(target_arch = "wasm32")
                                ))]
                                {
                                    ordered_layout_pages[_page_index]
                                }
                                #[cfg(not(all(
                                    feature = "ocr",
                                    feature = "layout-detection",
                                    feature = "tokio-runtime",
                                    not(target_arch = "wasm32")
                                )))]
                                {
                                    false
                                }
                            },
                            paragraphs,
                            ocr_scale,
                            detached_ocr_scale,
                        },
                    )
                    .collect();
                (doc, pages)
            } else {
                (None, Vec::new())
            }
        };

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let ocr_doc = {
            let mut ocr_doc = ocr_doc;
            if rejected_pages == [false]
                && public_ocr_pages.is_empty()
                && let (Some(doc), Some(rows)) = (ocr_doc.as_mut(), owned_scan_rows.as_deref())
            {
                super::document::certify_scan_html_paragraph(doc, rows);
            }
            ocr_doc
        };

        // Public nodes are already semantic structure. Do not offer flattened copies
        // to the mixed route's document-global paragraph reconstruction pass.
        for page in public_ocr_pages.keys() {
            if let Some(index) = (*page as usize).checked_sub(page_index_offset + 1)
                && let Some(page) = raw_page_paragraphs.get_mut(index)
            {
                page.paragraphs.clear();
            }
        }
        let ocr_doc = if public_ocr_pages.is_empty() {
            ocr_doc
        } else {
            let accepted: ahash::AHashMap<u32, String> = page_texts
                .iter()
                .enumerate()
                .filter(|(index, _)| !rejected_pages[*index])
                .filter_map(|(index, text)| {
                    let page = (page_index_offset + index + 1) as u32;
                    public_ocr_pages.contains_key(&page).then(|| (page, text.clone()))
                })
                .collect();
            let mut document = ocr_doc.unwrap_or_else(|| crate::types::internal::InternalDocument::new("pdf"));
            super::document::merge_structured_ocr_pages_into_internal_document(
                &mut document,
                &accepted,
                &public_ocr_pages,
            );
            Some(document)
        };

        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
        let ocr_doc = match ocr_doc {
            Some(mut document) => {
                crate::types::internal::table_grid::attach_recognized_table_grids(
                    &mut document,
                    &collected_table_grids,
                )
                .map_err(|error| crate::XbergError::Parsing {
                    message: "Failed to retain recognized PDF OCR table geometry".to_owned(),
                    source: Some(Box::new(error)),
                })?;
                Some(document)
            }
            None => None,
        };

        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let ocr_doc = {
            let mut ocr_doc = ocr_doc;
            if let (Some(proof), Some(document), [page], Some((pdf, _, _))) = (
                tall_ordered_primary,
                ocr_doc.as_mut(),
                page_texts.as_mut_slice(),
                lazy_pdf_render_state.as_ref(),
            ) && *page == result
                && super::tall_source::augmentation::augment(
                    proof,
                    (document, page),
                    &backend,
                    config,
                    base_ocr_config,
                    pdf,
                )
                .await?
            {
                result.clone_from(page);
                unpruned_text = None;
            }
            ocr_doc
        };

        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if let Some(native) = page_ocr_hints
            .as_ref()
            .and_then(|hints| hints.single_native_type0_page.as_deref())
            && let [read] = page_texts.as_slice()
            && page_index_offset == 0
            && images.is_none()
            && !has_layout_detections
            && raster_dpis.get(&0).is_some_and(|raster| raster.type0_ink_loss)
            && super::document::native_preservation_is_automatic(config, 1)
            && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
            && !tall_source_used
            && pages_with_recovered_xobject_payload == [false]
            && public_ocr_pages.is_empty()
            && accumulated_formulas.is_empty()
            && accumulated_llm_usage.is_empty()
            && super::native_word_coverage::single_page_type0_structure_can_yield_to_native(
                ocr_doc.as_ref(),
                read,
                &collected_tables,
            )
            && super::document::type0_native_is_corroborated(
                native,
                read,
                &ocr_output_thresholds,
                super::document::Type0ReadEvidence {
                    ink_owned: native_ink_owned,
                    words_covered: complete_native_words,
                    placed_words: &native_placed_words,
                },
            )
        {
            for warning in render_warnings {
                crate::pdf::render::record_render_warning(warning);
            }
            return Err(type0_ocr_rejection());
        }

        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        if native_ink_owned
            && detached_page.is_none()
            && images.is_none()
            && page_index_offset == 0
            && !has_layout_detections
            && !tall_source_used
            && let [read] = page_texts.as_slice()
            && !read.trim().is_empty()
            && pages_with_recovered_xobject_payload == [false]
            && public_ocr_pages.is_empty()
            && collected_tables.is_empty()
            && accumulated_formulas.is_empty()
            && accumulated_llm_usage.is_empty()
            && ocr_doc
                .as_ref()
                .is_none_or(|doc| super::native_word_coverage::only_segments_certified_tesseract_text(doc, read))
        {
            for warning in render_warnings {
                crate::pdf::render::record_render_warning(warning);
            }
            let lost = page_ocr_hints
                .as_ref()
                .and_then(|hints| hints.single_native_type0_page.as_deref())
                .map_or(0, |native| {
                    native
                        .matches(super::super::super::super::unmapped_glyphs::is_lost_glyph)
                        .count()
                });
            return Err(lost_glyph_ocr_rejection(lost));
        }

        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        let discarded_visible_native_page = matches!(recognition_noise_verdicts.as_slice(), [verdict] if verdict.page_index == 0 && verdict.discarded)
            && page_index_offset == 0
            && matches!(page_texts.as_slice(), [text] if text.trim().is_empty())
            && result.trim().is_empty()
            && ocr_doc.is_none()
            && !has_layout_detections
            && !capture_rasters
            && collected_tables.is_empty()
            && all_ocr_elements.is_empty()
            && accumulated_formulas.is_empty()
            && accumulated_llm_usage.is_empty()
            && page_failures.is_empty()
            && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
            && super::document::native_preservation_is_automatic(config, 1)
            && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
            && certify_discarded_native_layer(lazy_pdf_render_state, page_ocr_hints.as_ref(), config).await?;
        #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
        let discarded_visible_native_page = false;

        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if let Some(document) = ocr_doc.as_ref()
            && super::number_recovery::synchronize_plain_pages(
                document,
                &number_recovered_pages,
                &mut page_texts,
                &config.output_format,
            )
        {
            result = join_pages(&page_texts);
        }

        #[cfg(feature = "pdf")]
        let ocr_doc = {
            let mut warnings = image_fallback_warnings;
            warnings.extend(recognition_noise_warnings);
            warnings.extend(page_failure_warnings);
            warnings.extend(margin_filter_warnings);
            warnings.extend(backend_page_warnings);
            let mut ocr_doc = attach_ocr_fallback_warnings(ocr_doc, &result, warnings);
            if let Some(doc) = ocr_doc.as_mut() {
                doc.ocr_page_failures = page_failures;
            }
            // #1575: only merged when a document already exists (or the warnings step above just
            // built one from `text`) -- forcing an empty document into existence here for the sole
            // purpose of carrying metadata would replace `select_pdf_document`'s `flat_pdf_document`
            // fallback with an empty one, silently deleting the page text it currently preserves.
            if let (Some(doc), Some(additional)) = (ocr_doc.as_mut(), backend_additional_metadata) {
                doc.metadata.additional.extend(additional);
            }
            if let Some(doc) = ocr_doc.as_mut() {
                let accepted_modes = page_segmentation_modes.into_iter().filter(|(page_number, _)| {
                    let local_index = (*page_number as usize).saturating_sub(page_index_offset + 1);
                    !rejected_pages.get(local_index).copied().unwrap_or(true)
                        && page_texts.get(local_index).is_some_and(|text| !text.trim().is_empty())
                });
                attach_page_segmentation_modes(doc, accepted_modes);
            }
            ocr_doc
        };
        #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
        let ocr_doc = if forced_khmer_supplements.is_empty() {
            ocr_doc
        } else {
            forced_khmer_supplements
                .retain(|&index, _| !rejected_pages[index] && !pages_with_recovered_xobject_payload[index]);
            let mut document = ocr_doc;
            if let Some(document) = document.as_mut() {
                append_forced_script_paragraphs(document, &forced_khmer_supplements, page_index_offset);
            }
            for (index, supplement) in forced_khmer_supplements {
                if !page_texts[index].is_empty() {
                    page_texts[index].push_str("\n\n");
                }
                page_texts[index].push_str(&supplement);
            }
            result = join_pages(&page_texts);
            document
        };
        // Without `pdf` there is no page renderer, so no page-level OCR runs and the vector is
        // always empty; bind it so the non-pdf build does not warn about an unused value.
        #[cfg(not(feature = "pdf"))]
        let _ = (
            recognition_noise_warnings,
            page_failure_warnings,
            backend_page_warnings,
            backend_additional_metadata,
            page_segmentation_modes,
        );

        #[cfg(feature = "pdf")]
        for warning in render_warnings {
            crate::pdf::render::record_render_warning(warning);
        }

        Ok((
            result,
            mean_text_conf,
            collected_tables,
            all_ocr_elements,
            ocr_doc,
            accumulated_llm_usage,
            page_texts,
            if capture_rasters { Some(captured_rasters) } else { None },
            accumulated_formulas,
            raw_page_paragraphs,
            preprocessing_by_page,
            ocr_confidence_by_page,
            recognition_noise_verdicts,
            OcrReadEvidence {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                caption,
                no_admitted_images,
                unpruned_text,
                discarded_visible_native_page,
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                complete_native_words,
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                native_ink_owned,
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                native_placed_words,
                #[cfg(all(
                    feature = "pdf",
                    feature = "ocr",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                matched_layer_occurrences: matched_layer_occurrences
                    .filter(|_| !tall_source_used && pages_with_recovered_xobject_payload == [false]),
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                number_primary_text,
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
                #[cfg(not(feature = "pdf"))]
                retained_hocr_read: None,
                original_tesseract_raster: {
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    {
                        detached_page.is_some()
                            && images.is_some_and(|images| images.len() == 1)
                            && can_retry_orientation
                            && !tall_source_used
                            && pages_with_recovered_xobject_payload == [false]
                    }
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    {
                        false
                    }
                },
            },
        ))
    }
}
