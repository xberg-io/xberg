//! The automatic OCR route of the native PDF path: the per-page quality gate and the fallbacks it selects.

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use super::ink_probe;
#[cfg(all(
    feature = "layout-detection",
    any(feature = "formula-recognition", feature = "liter-llm")
))]
use super::native_layout_readers_configured;
use super::native_routing::{OcrRouteInputs, OcrRoutingState};
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use super::picture_layer;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use super::unreadable_script;
use super::{
    PageOcrLayoutContext, WholePageOcrOutput, automatic_page_acceptance_supported, dropped_script_warnings,
    ensure_pdf_not_cancelled, every_page_dropped_script, extraction_method_after_mixed_ocr,
    failed_ocr_fallback_is_total_loss, loses_whole_read, ocr, pages_that_dropped_script, run_ocr_with_layout_pages,
    unmapped_glyphs, whole_document_ocr_hints, with_native_page_acceptance,
};
use crate::Result;
use crate::types::ExtractionMethod;

// The result of a targeted per-page run, shared by the `RunFallbackOnPages` arm and the
// whole-document arm's retry of the pages whose native script the whole read dropped. ~keep
macro_rules! apply_targeted_mixed_run {
    ($routing:ident, $mixed_run:expr) => {{
        let mixed_run = $mixed_run;
        #[cfg(feature = "formula-recognition")]
        {
            $routing.mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
        }
        let (
            mixed,
            results_map,
            mixed_structured_pages,
            mixed_llm_usage,
            mixed_rstrs,
            mixed_formulas,
            mixed_preprocessing,
            mixed_ocr_confidence,
            mixed_warnings,
            mixed_page_failures,
        ) = mixed_run.output;
        let extraction_method = extraction_method_after_mixed_ocr(&results_map);
        $routing.llm_usage = mixed_llm_usage;
        $routing.results_map = Some(results_map);
        $routing.structured_pages = Some(mixed_structured_pages);
        $routing.page_rasters = mixed_rstrs;
        if !mixed_formulas.is_empty() {
            $routing.formulas = mixed_formulas;
        }
        $routing.preprocessing_by_page.extend(mixed_preprocessing);
        $routing.confidence_by_page.extend(mixed_ocr_confidence);
        $routing.page_failures.extend(mixed_page_failures);
        $routing.fallback_warnings.extend(mixed_warnings);
        (mixed, extraction_method)
    }};
}

struct AutoGate {
    failing_pages: Vec<u32>,
    fallback: bool,
    native_script_pages: Vec<u32>,
    total_chars: usize,
    alnum_ws_chars: usize,
    alnum_ws_ratio: f64,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    probe_reads: Option<std::sync::Arc<ocr::read_reuse::ProbeReads>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    picture_candidates: Vec<u32>,
}

pub(super) async fn route_auto_fallback(
    route: &mut OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
) -> Result<(String, ExtractionMethod)> {
    let (outcome, gate) = evaluate_auto_gate(route, routing, &native_text).await?;
    let (total_chars, alnum_ws_chars, alnum_ws_ratio) = (gate.total_chars, gate.alnum_ws_chars, gate.alnum_ws_ratio);
    let routed = match outcome {
        ocr::OcrGateOutcome::SkipNonText => {
            tracing::debug!(
                target: "xberg::extractors::pdf",
                alnum_ws_ratio,
                total_chars,
                alnum_ws_chars,
                "Skipping OCR: content is non-textual and pre-rendered structured doc available"
            );
            (native_text, ExtractionMethod::Native)
        }
        ocr::OcrGateOutcome::SkipSubstantive => {
            tracing::debug!(
                target: "xberg::extractors::pdf",
                total_chars,
                alnum_ws_ratio,
                ocr_fallback = gate.fallback,
                "Skipping OCR: pre-rendered structured doc available with substantive native text"
            );
            (native_text, ExtractionMethod::Native)
        }
        ocr::OcrGateOutcome::RunFallback => run_whole_document_fallback(route, routing, native_text, &gate).await?,
        ocr::OcrGateOutcome::RunFallbackOnPages(pages) => {
            run_selected_pages_fallback(route, routing, native_text, pages, &gate).await?
        }
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        ocr::OcrGateOutcome::UseNative => (native_text, ExtractionMethod::Native),
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        ocr::OcrGateOutcome::UseNative => probe_hidden_layers(route, routing, native_text).await?,
    };
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let routed = read_picture_pages(route, routing, routed, &gate).await?;
    Ok(routed)
}

async fn evaluate_auto_gate(
    route: &mut OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: &str,
) -> Result<(ocr::OcrGateOutcome, AutoGate)> {
    let (content, config, layout_pages, pdf_metadata, boundaries) = (
        route.content,
        route.config,
        route.layout_pages,
        route.pdf_metadata,
        route.boundaries,
    );
    let outlined_ocr_pages = &mut *route.outlined_ocr_pages;
    // Under `Auto`, a PDF with NO native text at all (a scan / missing text layer)
    // must reach OCR even when no explicit `ocr` config was given — the default
    // `ocr: None` ("OCR disabled") otherwise silently discarded the detected scan
    // and returned an empty result (#1338). This is gated on genuinely-absent text
    // a legitimately sparse/short PDF must stay native under a default config, and
    // heuristic quality failures still require an explicit `ocr` config to trigger
    // OCR. An explicit `ocr` config keeps its full per-page gate behavior.
    // `ocr_near_empty_fallback` overrides that derivation in either direction
    // (GH#1752); see `near_empty_ocr_fallback_applies`. ~keep
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
    let thresholds = ocr_config.effective_thresholds();
    let mut decision = ocr::evaluate_per_page_ocr(
        &native_text,
        boundaries,
        pdf_metadata.pdf_specific.page_count,
        &thresholds,
    );
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    let native_script_pages: Vec<u32> = Vec::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut picture_candidates = Vec::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let probe_reads = {
        let layer = unreadable_script::NativeLayer {
            text: &native_text,
            boundaries,
            page_count: pdf_metadata.pdf_specific.page_count,
        };
        let ink = ink_probe::contradicted_pages(&decision, layer, content, config, layout_pages).await?;
        let reads = ink.reads.filter(|reads| reads.finish(&ink.contradicted));
        picture_candidates.clone_from(&decision.failing_pages);
        crate::core::diagnostics::dedup_extend_warnings(&mut routing.fallback_warnings, ink.warnings);
        crate::core::diagnostics::dedup_extend_warnings(
            &mut routing.fallback_warnings,
            unreadable_script::retain_unreadable_script_pages(&mut decision, layer, content, config, &ink.contradicted),
        );
        reads
    };
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let native_script_pages =
        unreadable_script::native_script_pages(&decision, &native_text, boundaries, content, config);

    // `NativeTextStats`' character-class checks cannot see a font whose mapping
    // legitimately (per the §9.10.2 cascade) resolves to the wrong-but-ordinary
    // letters and punctuation (issue #1667): the text is structurally clean. The
    // provenance signal computed above (issue #1254) is a fact about how the text
    // was derived, not a shape guess, and `ScannedPages` already routes it via
    // `scanned_pages_to_ocr`; `Auto` must consult it too rather than silently
    // discarding it. ~keep
    let has_boundaries = boundaries.is_some_and(|b| !b.is_empty());
    ocr::apply_flagged_pages(
        &mut decision,
        pdf_metadata
            .pdf_specific
            .fabricated_text_pages
            .as_deref()
            .unwrap_or(&[]),
        has_boundaries,
        pdf_metadata.pdf_specific.page_count,
    );

    // A `/ToUnicode` CMap that resolves every glyph to *a* character, but
    // consistently the WRONG one, is file-backed exactly like a correct mapping --
    // `fabricated_text_pages`' provenance check cannot see it either (issue #1696).
    // `record_implausible_text_pages` already computed and merged this into
    // `scanned_pages` above; `Auto` reads it here the same way it reads the
    // provenance signal, so both share one union step. ~keep
    ocr::apply_flagged_pages(
        &mut decision,
        pdf_metadata
            .pdf_specific
            .implausible_text_pages
            .as_deref()
            .unwrap_or(&[]),
        has_boundaries,
        pdf_metadata.pdf_specific.page_count,
    );

    if decision.whole_doc_failure {
        outlined_ocr_pages.clear();
    } else {
        outlined_ocr_pages.retain(|page| !decision.failing_pages.contains(page));
        // Keep the selected-page acceptance check even when every page qualifies. ~keep
        ocr::apply_flagged_pages(&mut decision, &outlined_ocr_pages, has_boundaries, None);
    }

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    picture_candidates.retain(|page| !decision.failing_pages.contains(page));

    tracing::debug!(
        target: "xberg::pdf::ocr",
        fallback = decision.fallback,
        non_whitespace = decision.stats.non_whitespace,
        alnum = decision.stats.alnum,
        meaningful_words = decision.stats.meaningful_words,
        avg_non_whitespace = decision.avg_non_whitespace,
        avg_alnum = decision.avg_alnum,
        alnum_ratio = decision.stats.alnum_ratio,
        fragmented_word_ratio = decision.stats.fragmented_word_ratio,
        avg_word_length = decision.stats.avg_word_length,
        word_count = decision.stats.word_count,
        consecutive_repeat_ratio = decision.stats.consecutive_repeat_ratio,
        "per-page OCR gate decision",
    );

    let total_chars = native_text.chars().count();
    let alnum_ws_chars = native_text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .count();
    let alnum_ws_ratio = if total_chars > 0 {
        alnum_ws_chars as f64 / total_chars as f64
    } else {
        1.0
    };

    let outcome = ocr::evaluate_ocr_skip_gate(
        route.has_pre_rendered_doc,
        total_chars,
        alnum_ws_ratio,
        &decision,
        &thresholds,
    );
    let outcome = if automatic_page_acceptance_supported(config) {
        with_native_page_acceptance(
            outcome,
            &decision.failing_pages,
            boundaries,
            &native_text,
            pdf_metadata.pdf_specific.page_count,
            pdf_metadata
                .pdf_specific
                .fabricated_text_pages
                .as_deref()
                .unwrap_or(&[]),
        )
    } else {
        outcome
    };
    Ok((
        outcome,
        AutoGate {
            failing_pages: decision.failing_pages,
            fallback: decision.fallback,
            native_script_pages,
            total_chars,
            alnum_ws_chars,
            alnum_ws_ratio,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            picture_candidates,
        },
    ))
}

async fn run_whole_document_fallback(
    route: &mut OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    gate: &AutoGate,
) -> Result<(String, ExtractionMethod)> {
    let (content, config, path, layout_pages, pdf_metadata, boundaries) = (
        route.content,
        route.config,
        route.path,
        route.layout_pages,
        route.pdf_metadata,
        route.boundaries,
    );
    // With marked pages the retry below may keep some pages native, and the formula and
    // region-VLM stages read their rasters, so the inputs are held until the retry is
    // decided. The whole read consumes the reusable Markdown inputs, so a clone is held
    // when a native reader is configured; the clone holds every page raster a second time
    // until the retry is decided. ~keep
    #[cfg(all(
        feature = "layout-detection",
        any(feature = "formula-recognition", feature = "liter-llm")
    ))]
    let mut held_markdown_inputs = if gate.native_script_pages.is_empty() {
        drop(route.layout.native_only.take());
        None
    } else if route.layout.native_only.is_none() && native_layout_readers_configured(config) {
        Some((route.layout.images.clone(), route.layout.detections.clone()))
    } else {
        None
    };
    // `images.run_ocr_on_images` controls whether extracted images are
    // separately OCR'd (`core/pipeline/mod.rs`); it does not gate this
    // document-level page OCR fallback. Reading it here made any
    // `ImageExtractionConfig` (whose `run_ocr_on_images` defaults to `true`)
    // silently suppress page OCR on scanned PDFs, leaving `pages[].content`
    // empty (#1576). `RunFallbackOnPages` below has never read this field;
    // this now matches it. ~keep
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let release_reads = gate
        .probe_reads
        .as_ref()
        .and_then(|reads| reads.release_after_final_read());
    let whole_read = run_ocr_with_layout_pages(
        content,
        config,
        path,
        PageOcrLayoutContext {
            pages: layout_pages,
            #[cfg(all(feature = "pdf", feature = "layout-detection"))]
            rendered_layout_pages: std::mem::take(route.layout.rendered_pages),
            hints: {
                #[allow(unused_mut)]
                let mut hints = whole_document_ocr_hints(&pdf_metadata, &native_text, boundaries, config);
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if let Some(reads) = &gate.probe_reads {
                    hints.get_or_insert_with(Default::default).probe_reads = Some(std::sync::Arc::clone(reads));
                }
                hints
            },
        },
        #[cfg(feature = "layout-detection")]
        route.layout.images.take(),
        #[cfg(feature = "layout-detection")]
        route.layout.detections.take(),
        #[cfg(feature = "layout-detection")]
        route.layout.acceleration_override.take(),
    )
    .await;
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    drop(release_reads);
    let retried = retry_dropped_script_pages(route, routing, &native_text, gate, &whole_read).await?;
    #[cfg(all(
        feature = "layout-detection",
        any(feature = "formula-recognition", feature = "liter-llm")
    ))]
    if retried.is_some() {
        *route.layout.native_only = route.layout.native_only.take().or(held_markdown_inputs.take());
    } else {
        drop(route.layout.native_only.take());
        drop(held_markdown_inputs.take());
    }
    if let Some(retried) = retried {
        Ok(retried)
    } else {
        settle_whole_read(route, routing, native_text, gate, whole_read)
    }
}

async fn retry_dropped_script_pages(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: &str,
    gate: &AutoGate,
    whole_read: &ocr::OcrRunResult<WholePageOcrOutput>,
) -> Result<Option<(String, ExtractionMethod)>> {
    let (content, config, layout_pages, pdf_metadata, boundaries) = (
        route.content,
        route.config,
        route.layout_pages,
        route.pdf_metadata,
        route.boundaries,
    );
    let outlined_ocr_pages = &*route.outlined_ocr_pages;
    // Some marked pages lost their script to the whole read while others did not
    // (a scan beside a native page): the pages go through the per-page route, which
    // keeps the native text of the marked ones. That is a second OCR run over the
    // document, and the per-page route reports its own warnings and page failures, so
    // those of the discarded whole read are dropped with it. When the retry fails the
    // whole read stays the fallback. ~keep
    let dropped_pages = match (whole_read, boundaries) {
        (Ok(read), Some(bounds)) if !bounds.is_empty() => {
            let read_pages = &read.5;
            if every_page_dropped_script(&gate.native_script_pages, &native_text, Some(bounds), read_pages) {
                Vec::new()
            } else {
                pages_that_dropped_script(&gate.native_script_pages, &native_text, Some(bounds), read_pages)
            }
        }
        _ => Vec::new(),
    };
    let partial_script_drop = !dropped_pages.is_empty();
    let targeted_retry = match boundaries {
        Some(bounds) if partial_script_drop => Some(
            ocr::extract_mixed_ocr_native_with_layout_pages(
                &native_text,
                bounds,
                (
                    &gate.failing_pages,
                    pdf_metadata
                        .pdf_specific
                        .fabricated_text_pages
                        .as_deref()
                        .unwrap_or(&[]),
                ),
                content,
                config,
                layout_pages,
                (&outlined_ocr_pages, &gate.native_script_pages),
            )
            .await,
        ),
        _ => None,
    };
    let retried = match targeted_retry {
        Some(Ok(mixed_run)) => {
            // A page the retry could not read that the whole read did read would come out
            // empty, so the retry is discarded and the whole read used. ~keep
            let whole_read_loses = loses_whole_read(
                whole_read.as_ref().ok().map(|read| read.5.as_slice()),
                &mixed_run.output.9,
                &dropped_pages,
            );
            if whole_read_loses {
                tracing::warn!(
                    target: "xberg::extractors::pdf",
                    failing_pages = ?gate.failing_pages,
                    "Targeted OCR could not read a page the whole-document read had read; \
                    using the whole-document read"
                );
                routing.fallback_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("ocr"),
                    message: std::borrow::Cow::Owned(format!(
                        "Pages {dropped_pages:?} are in a script that the OCR read did not \
                                         produce, and the targeted OCR that keeps their native text could not \
                                         read every other page; their native text was replaced by the \
                                         whole-document OCR read."
                    )),
                });
                None
            } else {
                Some(apply_targeted_mixed_run!(routing, mixed_run))
            }
        }
        Some(Err(error @ crate::XbergError::Cancelled)) => return Err(error),
        Some(Err(error)) => {
            tracing::warn!(
                target: "xberg::extractors::pdf",
                error = %error.pdf_ocr_trace_category(),
                failing_pages = ?gate.failing_pages,
                "Targeted OCR of the pages that dropped their native script failed; \
                using the whole-document read"
            );
            ensure_pdf_not_cancelled(config)?;
            routing.fallback_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "Pages {dropped_pages:?} are in a script that the OCR read did not produce, \
                                     and the targeted OCR that keeps their native text failed ({error}); \
                                     their native text was replaced by the whole-document OCR read."
                )),
            });
            None
        }
        None => None,
    };
    Ok(retried)
}

fn settle_whole_read(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    gate: &AutoGate,
    whole_read: ocr::OcrRunResult<WholePageOcrOutput>,
) -> Result<(String, ExtractionMethod)> {
    let (config, pdf_metadata, boundaries) = (route.config, route.pdf_metadata, route.boundaries);
    match whole_read {
        Ok(read) if !every_page_dropped_script(&gate.native_script_pages, &native_text, boundaries, &read.5) => {
            Ok(adopt_whole_read(routing, pdf_metadata, native_text, read))
        }
        Ok(read) => Ok(discard_whole_read(routing, boundaries, native_text, gate, read)),
        Err(failed_run) => {
            if matches!(failed_run.error, crate::XbergError::Cancelled) {
                return Err(failed_run.error);
            }
            tracing::warn!(
                target: "xberg::extractors::pdf",
                error = %failed_run.error.pdf_ocr_trace_category(),
                "OCR fallback failed; using native text extraction result"
            );
            if failed_ocr_fallback_is_total_loss(&native_text) {
                return Err(failed_run.error);
            }
            if ensure_pdf_not_cancelled(config).is_ok() {
                routing.page_failures.extend(ocr::failed_run_page_failures(
                    &failed_run,
                    pdf_metadata.pdf_specific.page_count.unwrap_or(0),
                    &native_text,
                    boundaries.unwrap_or(&[]),
                ));
            }
            if let Some(warning) =
                unmapped_glyphs::unrecovered_page_glyph_warning(1, failed_run.unrecovered_glyphs_on_page_one)
            {
                crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
            }
            let e = &failed_run.error;
            routing.fallback_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "OCR fallback failed ({e}); returning native text that was below the \
                                         quality threshold which triggered OCR. Extracted content may be empty \
                                         or incomplete."
                )),
            });
            Ok((native_text, ExtractionMethod::Native))
        }
    }
}

fn adopt_whole_read(
    routing: &mut OcrRoutingState,
    pdf_metadata: &crate::pdf::metadata::PdfExtractionMetadata,
    native_text: String,
    read: WholePageOcrOutput,
) -> (String, ExtractionMethod) {
    let (
        ocr_text,
        ocr_tbls,
        ocr_elems,
        ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rstrs,
        formulas,
        preprocessing,
        page_ocr_confidence,
        gate_audit,
        layout_warning,
        layout_glyph_drop_warnings,
        discarded_visible_native_page,
    ) = read;
    if let Some(warning) = layout_warning {
        crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
    }
    for warning in layout_glyph_drop_warnings {
        crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
    }
    let preserve_native_after_discard = discarded_visible_native_page
        && pdf_metadata
            .pdf_specific
            .fabricated_text_pages
            .as_ref()
            .is_some_and(Vec::is_empty)
        && pdf_metadata
            .pdf_specific
            .implausible_text_pages
            .as_ref()
            .is_some_and(Vec::is_empty);
    if preserve_native_after_discard {
        if let Some(warning) = unmapped_glyphs::unrecovered_page_glyph_warning(
            1,
            native_text.matches(unmapped_glyphs::is_lost_glyph).count(),
        ) {
            crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
        }
        let (warnings, failures) = ocr_doc
            .map(|doc| (doc.processing_warnings, doc.ocr_page_failures))
            .unwrap_or_default();
        crate::core::diagnostics::dedup_extend_warnings(&mut routing.fallback_warnings, warnings);
        routing.page_failures.extend(failures);
        (native_text, ExtractionMethod::Native)
    } else {
        routing.layout_gate_audit = gate_audit;
        routing.tables = ocr_tbls;
        routing.elements = ocr_elems;
        routing.internal_doc = ocr_doc;
        routing.llm_usage = llm_usage;
        routing.page_texts = Some(ocr_pts);
        routing.page_rasters = ocr_rstrs;
        routing.formulas = formulas;
        routing.preprocessing_by_page = preprocessing;
        routing.confidence_by_page = page_ocr_confidence;
        (ocr_text, ExtractionMethod::Ocr)
    }
}

fn discard_whole_read(
    routing: &mut OcrRoutingState,
    boundaries: Option<&[crate::types::PageBoundary]>,
    native_text: String,
    gate: &AutoGate,
    read: WholePageOcrOutput,
) -> (String, ExtractionMethod) {
    let (_, _, _, ocr_doc, _, _, _, _, _, _, _, _, _, _) = read;
    let (warnings, failures) = ocr_doc
        .map(|doc| (doc.processing_warnings, doc.ocr_page_failures))
        .unwrap_or_default();
    // The discarded read's text is not in the result: its failures were not
    // recovered by it, and layout inputs and llm_usage go with it, as in the
    // sibling arms.
    crate::core::diagnostics::dedup_extend_warnings(
        &mut routing.fallback_warnings,
        warnings
            .into_iter()
            .filter(|warning| !warning.message.contains("was recovered from"))
            .collect(),
    );
    routing
        .page_failures
        .extend(failures.into_iter().map(|failure| crate::types::OcrPageFailure {
            recovered: false,
            ..failure
        }));
    crate::core::diagnostics::dedup_extend_warnings(
        &mut routing.fallback_warnings,
        dropped_script_warnings(&gate.native_script_pages, &native_text, boundaries),
    );
    (native_text, ExtractionMethod::Native)
}

async fn run_selected_pages_fallback(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    pages: Vec<u32>,
    gate: &AutoGate,
) -> Result<(String, ExtractionMethod)> {
    let (content, config, layout_pages, pdf_metadata, boundaries) = (
        route.content,
        route.config,
        route.layout_pages,
        route.pdf_metadata,
        route.boundaries,
    );
    let outlined_ocr_pages = &*route.outlined_ocr_pages;
    Ok(match boundaries {
        Some(bounds) if !bounds.is_empty() => {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let selected_run = ocr::extract_mixed_ocr_native_with_probe_reads(
                &native_text,
                bounds,
                (
                    &pages,
                    pdf_metadata
                        .pdf_specific
                        .fabricated_text_pages
                        .as_deref()
                        .unwrap_or(&[]),
                ),
                content,
                config,
                layout_pages,
                (&outlined_ocr_pages, &gate.native_script_pages, gate.probe_reads.clone()),
            )
            .await;
            #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
            let selected_run = ocr::extract_mixed_ocr_native_with_layout_pages(
                &native_text,
                bounds,
                (
                    &pages,
                    pdf_metadata
                        .pdf_specific
                        .fabricated_text_pages
                        .as_deref()
                        .unwrap_or(&[]),
                ),
                content,
                config,
                layout_pages,
                (&outlined_ocr_pages, &gate.native_script_pages),
            )
            .await;
            match selected_run {
                Ok(mixed_run) => apply_targeted_mixed_run!(routing, mixed_run),
                Err(e @ crate::XbergError::Cancelled) => return Err(e),
                Err(e) => {
                    tracing::warn!(
                        target: "xberg::extractors::pdf",
                        error = %e.pdf_ocr_trace_category(),
                        failing_pages = ?pages,
                        "Targeted OCR fallback failed; using native text extraction result"
                    );
                    if failed_ocr_fallback_is_total_loss(&native_text) {
                        return Err(e);
                    }
                    routing
                        .page_failures
                        .extend(ocr::route_level_ocr_page_failures(&pages, &e, &native_text, bounds));
                    routing.fallback_warnings.push(crate::types::ProcessingWarning {
                        source: std::borrow::Cow::Borrowed("ocr"),
                        message: std::borrow::Cow::Owned(format!(
                            "Targeted OCR fallback failed ({e}) for pages {pages:?}; those pages \
                                         retain native text that was below the OCR-trigger quality threshold \
                                         and may be empty or incomplete."
                        )),
                    });
                    (native_text, ExtractionMethod::Native)
                }
            }
        }
        _ => {
            tracing::warn!(
                target: "xberg::extractors::pdf",
                failing_pages = ?pages,
                "Targeted OCR requested but no page boundaries available; using native text"
            );
            if failed_ocr_fallback_is_total_loss(&native_text) {
                return Err(crate::XbergError::Plugin {
                    message: format!(
                        "Targeted OCR was required for pages {pages:?} but no page boundaries \
                                     were available, and no native text could be recovered"
                    ),
                    plugin_name: "ocr".to_string(),
                });
            }
            routing.fallback_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "Targeted OCR was required for pages {pages:?} but no page boundaries were \
                                 available; those pages retain native text that was below the OCR-trigger \
                                 quality threshold and may be empty or incomplete."
                )),
            });
            (native_text, ExtractionMethod::Native)
        }
    })
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
async fn probe_hidden_layers(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
) -> Result<(String, ExtractionMethod)> {
    let (content, config, pdf_metadata, boundaries) =
        (route.content, route.config, route.pdf_metadata, route.boundaries);
    let probe = ocr::probe_incomplete_hidden_layers(
        &native_text,
        boundaries.unwrap_or(&[]),
        pdf_metadata.pdf_specific.scanned_pages.as_deref().unwrap_or(&[]),
        content,
        config,
    )
    .await?;
    Ok(if let Some(mixed_run) = probe {
        let (text, replacements, _, _, _, _, preprocessing, confidence, warnings, page_failures) = mixed_run.output;
        let method = extraction_method_after_mixed_ocr(&replacements);
        routing.preprocessing_by_page.extend(
            preprocessing
                .into_iter()
                .filter(|(page, _)| replacements.contains_key(page)),
        );
        routing.confidence_by_page.extend(
            confidence
                .into_iter()
                .filter(|(page, _)| replacements.contains_key(page)),
        );
        routing.fallback_warnings.extend(warnings);
        routing.page_failures.extend(page_failures);
        routing.results_map = Some(replacements);
        (text, method)
    } else {
        (native_text, ExtractionMethod::Native)
    })
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
async fn read_picture_pages(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    routed: (String, ExtractionMethod),
    gate: &AutoGate,
) -> Result<(String, ExtractionMethod)> {
    let (content, config, boundaries) = (route.content, route.config, route.boundaries);
    let (mut text, mut method) = routed;
    if method != ExtractionMethod::Ocr
        && !gate.picture_candidates.is_empty()
        && let Some(bounds) = boundaries
    {
        let current_bounds = routing.results_map.as_ref().map_or_else(
            || bounds.to_vec(),
            |accepted| ocr::boundaries_after_replacements(bounds, accepted),
        );
        routing.picture_pages = picture_layer::read_retained_picture_pages(
            content,
            &text,
            &current_bounds,
            &gate.picture_candidates,
            routing.results_map.as_ref(),
            config,
        )
        .await?;
        if !routing.picture_pages.is_empty() {
            let additions = routing
                .picture_pages
                .iter()
                .map(|(&page, read)| (page, read.transcript()))
                .collect();
            text = ocr::apply_ocr_page_replacements(&text, &current_bounds, &additions);
            routing
                .results_map
                .get_or_insert_with(Default::default)
                .extend(additions);
            if routing.picture_pages.values().any(|page| page.raster.is_some()) {
                method = ExtractionMethod::Mixed;
            }
        }
    }
    Ok((text, method))
}
