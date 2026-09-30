//! Public per-page extraction surfaces for spans, chars, words, and lines.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Internal helper: extract raw (unsorted) text spans from a page.
    ///
    /// This is the common extraction logic shared by `extract_spans`
    /// `extract_spans_with_reading_order`. Spans are returned without any
    /// sorting or erase-region filtering applied.
    pub(super) fn extract_spans_raw(&self, page_index: usize) -> Result<Vec<crate::layout::TextSpan>> {
        self.extract_spans_raw_with_extraction_config(page_index, crate::extractors::TextExtractionConfig::default())
    }

    /// Internal helper: extract raw text spans using a specific extraction config.
    ///
    /// This allows callers to provide a [`TextExtractionConfig`] (optionally
    /// configured with an [`ExtractionProfile`]) to control TJ offset thresholds
    /// and word boundary detection during span extraction.
    fn extract_spans_raw_with_extraction_config(
        &self,
        page_index: usize,
        config: crate::extractors::TextExtractionConfig,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        self.extract_spans_impl(page_index, config, HashSet::new(), HashSet::new())
    }

    pub(super) fn extract_spans_raw_filtered(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        self.extract_spans_impl(
            page_index,
            crate::extractors::TextExtractionConfig::default(),
            excluded_layers,
            excluded_inks,
        )
    }

    fn extract_spans_impl(
        &self,
        page_index: usize,
        config: crate::extractors::TextExtractionConfig,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        if self.is_encrypted_unreadable() {
            tracing::warn!(target: LOG_TARGET, "PDF is encrypted and could not be decrypted; returning no spans");
            return Ok(Vec::new());
        }
        use crate::extractors::TextExtractor;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        if self.page_cannot_have_text(page_dict) {
            // A whole page discarded with no log line is why GH#1754 needed a downstream
            // consumer to find it. Both this guard and `may_contain_text` below drop every
            // span on the page, so both say which page and why. ~keep
            tracing::debug!(
                target: LOG_TARGET,
                page_index,
                reason = "no /Font resources and no Form XObjects",
                "skipping page text extraction"
            );
            return Ok(Vec::new());
        }

        let content_data = match self.get_page_content_data(page_index) {
            Ok(data) => data,
            Err(error) => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "decode_page_content",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "returning empty page content after decode failure"
                );
                return Ok(Vec::new());
            }
        };

        if !Self::may_contain_text(&content_data) {
            tracing::debug!(
                target: LOG_TARGET,
                page_index,
                content_bytes = content_data.len(),
                reason = "content stream has no BT and no Do operator",
                "skipping page text extraction"
            );
            return Ok(Vec::new());
        }

        let mut extractor = TextExtractor::with_config(config);
        // Stamp the page index so spans carry McidScope::Page(page_index)
        // by default; Form XObject Do invocations push their own scope
        // on top of the stack inside the extractor. ~keep ~keep
        extractor.set_page_index(page_index as u32);
        if !excluded_layers.is_empty() {
            extractor.set_excluded_layers(excluded_layers);
        }
        if !excluded_inks.is_empty() {
            extractor.set_excluded_inks(excluded_inks);
        }
        if let Some(resources) = page_dict.get("Resources") {
            extractor.set_resources(resources.clone());
            extractor.set_document(self);
            if let Err(error) = self.load_fonts(resources, &mut extractor) {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "load_page_fonts",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "using fallback font encoding"
                );
            }
        }

        let spans = extractor.extract_text_spans(&content_data)?;
        // Drain MCIDs whose in-stream /ActualText was applied during
        // extraction and stash on the document so the struct-tree-
        // scope applier honours MC-scope-wins precedence (§14.9.4).
        //
        // The per-page entry is REPLACED, not extended: every
        // `extract_spans_impl` call is a self-contained per-page
        // extraction and its own MC-scope detections must be
        // authoritative. Accumulating would make stale results from
        // an earlier filter-set leak into a later, differently-
        // filtered call. ~keep ~keep
        let mc_set = extractor.take_mc_actualtext_mcids();
        let mut guard = self.mc_actualtext_mcids.lock_or_recover();
        if mc_set.is_empty() {
            guard.remove(&page_index);
        } else {
            guard.insert(page_index, mc_set);
        }
        Ok(spans)
    }

    /// Extract text from a page, excluding content from specified layers and inks.
    ///
    /// Uses the same full text assembly pipeline as [`extract_text`](Self::extract_text)
    /// (structure-tree ordering, table detection, column detection), but with
    /// layer/ink-excluded spans removed before assembly.
    ///
    /// **Ink filtering note:** For DeviceN color spaces, text is suppressed if
    /// ANY ink in the DeviceN array matches an excluded ink name. Tint values
    /// are not evaluated — this is an all-or-nothing match.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `excluded_layers` - OCG layer names to suppress (empty = no layer filtering)
    /// * `excluded_inks` - Separation/DeviceN ink names to suppress (empty = no ink filtering)
    pub fn extract_text_filtered(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<String> {
        if excluded_layers.is_empty() && excluded_inks.is_empty() {
            return self.extract_text(page_index);
        }

        let spans = self.extract_spans_filtered(page_index, excluded_layers, excluded_inks)?;
        let options = crate::converters::ConversionOptions {
            extract_tables: true,
            ..Default::default()
        };
        self.assemble_text_from_spans(page_index, spans, &options)
    }

    /// Extract text from a region of a page with layer/ink filtering applied.
    ///
    /// Composes [`Self::extract_text_filtered`] with [`Self::extract_text_in_rect`]: spans
    /// are filtered by layer/ink first, then by region, then assembled via
    /// the full text pipeline (structure-tree ordering, table detection,
    /// column detection, whitespace + line breaks).
    pub fn extract_text_filtered_in_rect(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
        region: crate::geometry::Rect,
        mode: crate::layout::RectFilterMode,
    ) -> Result<String> {
        let spans = if excluded_layers.is_empty() && excluded_inks.is_empty() {
            self.extract_spans(page_index)?
        } else {
            self.extract_spans_filtered(page_index, excluded_layers, excluded_inks)?
        };
        let options = crate::converters::ConversionOptions {
            extract_tables: true,
            include_region: Some((region, mode)),
            ..Default::default()
        };
        self.assemble_text_from_spans(page_index, spans, &options)
    }

    /// Geometric `ColumnAware` (XY-cut) span ordering. Shared by the
    /// `ColumnAware` and `Structure` reading-order branches (the latter uses it
    /// as its baseline and its tiebreak for unstructured spans).
    fn order_spans_column_aware(
        &self,
        spans: Vec<crate::layout::TextSpan>,
        page_index: usize,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        use crate::pipeline::reading_order::{ReadingOrderContext as ROContext, ReadingOrderStrategy, XYCutStrategy};
        let strategy = XYCutStrategy::new();
        // The heading-run pre-pass needs the gutter to tell a heading opening
        // the other column apart from a second `Tj` segment of the same
        // heading line (GH#1757). This is the ordering that reaches the text
        // and markdown lenses, so leaving it `None` leaves the defect live
        // even when `postprocess_spans` has the gutter. ~keep
        let mut context = ROContext::new().with_page(page_index as u32);
        if let Some(gutter_x) = Self::detect_column_gutter(&spans) {
            context = context.with_column_gutter(gutter_x);
        }
        let ordered = strategy.apply(spans, &context)?;
        Ok(ordered.into_iter().map(|o| o.span).collect())
    }

    /// Extract text spans from a page using a specified reading order strategy.
    ///
    /// This method extracts text spans identically to [`extract_spans`](Self::extract_spans),
    /// then applies the chosen reading order strategy to sort them.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `reading_order` - The reading order strategy to apply
    ///
    /// # Returns
    ///
    /// Vector of TextSpan objects sorted according to the chosen reading order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::{PdfDocument, ReadingOrder};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("two_column.pdf")?;
    /// let spans = doc.extract_spans_with_reading_order(0, ReadingOrder::ColumnAware)?;
    /// for span in spans {
    ///     println!("{}", span.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_spans_with_reading_order(
        &self,
        page_index: usize,
        reading_order: ReadingOrder,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        self.extract_spans_filtered_with_reading_order(page_index, reading_order, HashSet::new(), HashSet::new())
    }

    /// Extract positioned spans in a reading order, excluding optional-content
    /// layers and/or Separation/DeviceN inks.
    ///
    /// This is [`extract_spans_with_reading_order`](Self::extract_spans_with_reading_order)
    /// and [`extract_text_filtered`](Self::extract_text_filtered) combined: the
    /// former cannot filter, and the latter returns assembled text rather than
    /// positioned spans. A consumer that lays spans out itself - an HTML/XML
    /// emitter placing each span at its own rectangle - needs both at once.
    ///
    /// The motivating case is render/extract parity. `render_page` honours the
    /// document's default configuration `/OCProperties/D`, but span extraction
    /// treats everything as visible unless the caller names layers (see the
    /// `optional_content` module note). Passing
    /// `optional_content::compute_default_off_ocgs(&doc)` as `excluded_layers`
    /// makes extraction agree with what the page actually displays - without it,
    /// a default-OFF layer holding a copy of the page contributes a SECOND copy
    /// of every word.
    ///
    /// Empty sets are exactly equivalent to the unfiltered call, so this is a
    /// superset of the existing API and costs nothing when no filtering is asked
    /// for.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `reading_order` - The reading order strategy to apply
    /// * `excluded_layers` - OCG layer names to suppress (empty = no filtering)
    /// * `excluded_inks` - Separation/DeviceN ink names to suppress (empty = none)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::{PdfDocument, ReadingOrder};
    /// # use xberg_native_pdf::optional_content;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = PdfDocument::open("layered.pdf")?;
    /// // Agree with what the page displays: drop the default-off layers.
    /// let hidden = optional_content::compute_default_off_ocgs(&doc);
    /// let spans = doc.extract_spans_filtered_with_reading_order(
    ///     0,
    ///     ReadingOrder::ColumnAware,
    ///     hidden,
    ///     Default::default(),
    /// )?;
    /// for span in spans {
    ///     println!("{}", span.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// **Rotation is handled differently here than in [`Self::extract_spans`].**
    /// This function calls only `extract_spans_raw`/`_filtered` plus
    /// `drop_offpage_spans` (see the comment in the body below) and deliberately
    /// skips `postprocess_spans`, so `map_span_into_rotated_frame` never runs on
    /// this path and every returned span's `page_rotation_applied` is always
    /// `0`. Bboxes here stay in raw user space even on a `/Rotate`d page; the
    /// caller owns the upright transform. xberg's own extraction layer — the
    /// only caller of this function — already does this via `upright_origin`,
    /// which rotates by the span's own content-matrix angle. Making this path
    /// call `postprocess_spans` "for consistency" would be a regression, not a
    /// fix: `upright_origin` would then rotate a bbox `postprocess_spans` had
    /// already mapped into the displayed frame, double-transforming it into a
    /// third, wrong coordinate — this was worked out numerically, not assumed.
    pub fn extract_spans_filtered_with_reading_order(
        &self,
        page_index: usize,
        reading_order: ReadingOrder,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        // Extract raw spans using the common extraction logic. The unfiltered
        // path is kept verbatim for empty sets so existing callers are unchanged. ~keep
        let mut spans = if excluded_layers.is_empty() && excluded_inks.is_empty() {
            self.extract_spans_raw(page_index)?
        } else {
            self.extract_spans_raw_filtered(page_index, excluded_layers, excluded_inks)?
        };

        // Drop text lying entirely off the MediaBox - a doc that reuses one big
        // Form XObject across pages relies on the `W n` clip to hide the off-page
        // portion, which the raw extractor does not honour. `extract_spans` applies
        // this via `postprocess_spans`; the reading-order path must too, or it
        // emits every page's worth of spans (measured: a stats report emitted a
        // chart's full hidden data table, ~5x the visible label count).
        //
        // This ports only the off-page filter out of `postprocess_spans`, not the
        // whole pipeline: the page-/Rotate mapping it also does (via
        // `map_span_into_rotated_frame`) is deliberately left out, so
        // `page_rotation_applied` stays 0 on every span this function returns.
        // See this function's doc comment above for why adding that mapping back
        // in would make results wrong, not just more consistent. ~keep
        self.drop_offpage_spans(page_index, &mut spans);

        match reading_order {
            ReadingOrder::TopToBottom => {
                crate::utils::sort_by_visual_rows(
                    &mut spans,
                    |span| span.bbox.y,
                    |span| span.bbox.x,
                    |span| span.font_size.max(span.bbox.height),
                );
            }
            ReadingOrder::ColumnAware => {
                spans = self.order_spans_column_aware(spans, page_index)?;
            }
            ReadingOrder::Structure => {
                // Geometric order is the baseline. The structure tree then fixes ONLY
                // the spans it can fix unambiguously: TABLE cells.
                //
                // A geometric XY-cut reads a wide table column-major and drops cells;
                // the structure tree's pre-order traversal gives the authoritative
                // row-major order (§14.8.2.3). But applying that traversal to the WHOLE
                // page also reorders flowing prose - where the tree's section order can
                // legitimately differ from visual order (it de-prioritises page
                // artifacts, for one) - which is a change, not an improvement. So table
                // content is reordered IN PLACE by structure rank while every non-table
                // span keeps its geometric position. ~keep
                let mut ordered = self.order_spans_column_aware(spans, page_index)?;
                if let Some(tree) = self.struct_tree_trustworthy() {
                    // Populate the structure-content cache, then read it (it carries the
                    // per-MCID `in_table` flag the mcid-order list does not). ~keep
                    let _ = self.cached_mcid_order_for_page(&tree, page_index as u32);
                    let content: Vec<crate::structure::OrderedContent> = self
                        .structure_content_cache
                        .lock_or_recover()
                        .as_ref()
                        .and_then(|c| c.get(&(page_index as u32)))
                        .cloned()
                        .unwrap_or_default();

                    let page_scope = crate::structure::McidScope::Page(page_index as u32);
                    let mut table_rank: HashMap<(crate::structure::McidScope, u32), usize> = HashMap::new();
                    for (i, c) in content.iter().enumerate() {
                        if let (true, Some(m)) = (c.in_table, c.mcid) {
                            let scope = c.mcid_scope.clone().unwrap_or_else(|| page_scope.clone());
                            table_rank.entry((scope, m)).or_insert(i);
                        }
                    }

                    if !table_rank.is_empty() {
                        let key_of = |s: &crate::layout::TextSpan| {
                            s.mcid.and_then(|m| {
                                let scope = s.mcid_scope.clone().unwrap_or_else(|| page_scope.clone());
                                table_rank
                                    .get(&(scope, m))
                                    .or_else(|| table_rank.get(&(page_scope.clone(), m)))
                                    .copied()
                            })
                        };
                        let mut slots: Vec<usize> = Vec::new();
                        let mut cells: Vec<(usize, crate::layout::TextSpan)> = Vec::new();
                        for (idx, s) in ordered.iter().enumerate() {
                            if let Some(r) = key_of(s) {
                                slots.push(idx);
                                cells.push((r, s.clone()));
                            }
                        }
                        // Re-fill those exact slots with the cells in structure
                        // (row-major) order. Non-table spans never move. ~keep
                        cells.sort_by_key(|(r, _)| *r);
                        for (slot, (_, cell)) in slots.into_iter().zip(cells) {
                            ordered[slot] = cell;
                        }
                    }
                }
                spans = ordered;
            }
        }

        let erase = self.erase_regions.lock_or_recover().get(&page_index).cloned();
        if let Some(regions) = erase {
            spans.retain(|span| !regions.iter().any(|r| r.intersects(&span.bbox)));
        }

        self.apply_actualtext_to_spans(page_index, &mut spans);

        Ok(spans)
    }

    /// Extract complete page text data in a single call.
    ///
    /// Returns a [`PageText`](crate::layout::text_block::PageText) containing spans in reading order, per-character
    /// data derived from those spans (using font-metric widths when available),
    /// and the page dimensions. Uses the default `TopToBottom` reading order.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    /// let page_text = doc.extract_page_text(0)?;
    /// println!("Page {}x{} pt", page_text.page_width, page_text.page_height);
    /// println!("{} spans, {} chars", page_text.spans.len(), page_text.chars.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_page_text(&self, page_index: usize) -> Result<crate::layout::PageText> {
        self.extract_page_text_with_options(page_index, ReadingOrder::default())
    }

    /// Extract a page as typed [`StructuredPage`](crate::structured::StructuredPage)
    /// regions.
    ///
    /// Returns the page's text grouped into
    /// [`StructuredRegion`](crate::structured::StructuredRegion)s — body blocks,
    /// headings, header/footer/page-number chrome, and marginal labels — in
    /// reading order, with a best-effort `column_index` for two-column bodies.
    ///
    /// Roles are derived from signals already attached to each span: `/Artifact`
    /// marked content (ISO 32000-1:2008 §14.8.2.2), structure-tree heading levels
    /// (§14.7.2), and span geometry (§14.8.2.3.1). A tagged PDF with a
    /// trustworthy `/StructTreeRoot` (see
    /// [`prefers_structure_reading_order`](Self::prefers_structure_reading_order))
    /// therefore yields tree-driven roles; untagged PDFs use the geometric /
    /// font-size fallbacks. This is an additive aggregation layer — it does not
    /// change any existing extraction output.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("two_column.pdf")?;
    /// let page = doc.extract_structured(0)?;
    /// for region in &page.regions {
    ///     println!("{:?} col={:?}: {}", region.kind, region.column_index, region.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_structured(&self, page_index: usize) -> Result<crate::structured::StructuredPage> {
        self.extract_structured_with_column_mode(page_index, crate::structured::ColumnMode::Auto)
    }

    /// Extract a page as [`StructuredPage`](crate::structured::StructuredPage)
    /// regions with an explicit column-detection
    /// [`ColumnMode`](crate::structured::ColumnMode) (Fix 3).
    ///
    /// `Auto` runs the geometric gutter heuristic (same as
    /// [`extract_structured`](Self::extract_structured)); `Two` forces a
    /// two-column split for layouts the conservative heuristic rejects (short,
    /// ragged reference-edition lines); `Single` suppresses column detection.
    /// The override applies to the geometric path only — ISO 32000-1:2008
    /// §14.8.2.3 leaves untagged reading order undefined — and never overrides a
    /// trustworthy structure tree.
    pub fn extract_structured_with_column_mode(
        &self,
        page_index: usize,
        column_mode: crate::structured::ColumnMode,
    ) -> Result<crate::structured::StructuredPage> {
        let page_text = self.extract_page_text(page_index)?;
        let struct_info = self.structured_mcid_info(page_index);
        Ok(crate::structured::build_structured_page_full(
            page_index,
            page_text.page_width,
            page_text.page_height,
            page_text.spans,
            column_mode,
            &struct_info,
        ))
    }

    /// Per-MCID structure facts for `extract_structured` (ISO 32000-1:2008
    /// §14.8.4): which MCIDs are `Lbl` labels and which logical `Sect`/`Art`
    /// section each belongs to. Empty for untagged or suspect-tagged PDFs, so
    /// the structured output is identical to the geometric path there. Section
    /// ids are document-stable (the same `Sect` element yields the same id on
    /// every page it spans), giving cross-page chapter continuity for free
    /// (§4/§5/§6).
    fn structured_mcid_info(&self, page_index: usize) -> crate::structured::McidStructInfo {
        let mut info = crate::structured::McidStructInfo::default();
        let Some(ref struct_tree) = self.struct_tree_trustworthy() else {
            return info;
        };
        if self.structure_content_cache.lock_or_recover().is_none() {
            let all = crate::structure::traverse_structure_tree_all_pages(struct_tree);
            *self.structure_content_cache.lock_or_recover() = Some(all);
        }
        let cache = self.structure_content_cache.lock_or_recover();
        if let Some(content) = cache.as_ref().and_then(|c| c.get(&(page_index as u32))) {
            for item in content {
                if let Some(mcid) = item.mcid {
                    if matches!(item.list_role, Some(crate::structure::ListRole::Lbl)) {
                        info.lbl.insert(mcid);
                    }
                    if let Some(sid) = item.section_id {
                        info.section.insert(mcid, sid as usize);
                    }
                }
            }
        }
        info
    }

    /// Extract complete page text data with a specific reading order.
    ///
    /// Like [`extract_page_text`](Self::extract_page_text) but allows choosing
    /// between `TopToBottom` and `ColumnAware` reading order.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `reading_order` - Reading order strategy to apply
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::{PdfDocument, ReadingOrder};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("two_column.pdf")?;
    /// let page_text = doc.extract_page_text_with_options(0, ReadingOrder::ColumnAware)?;
    /// for span in &page_text.spans {
    ///     println!("{}", span.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_page_text_with_options(
        &self,
        page_index: usize,
        reading_order: ReadingOrder,
    ) -> Result<crate::layout::PageText> {
        let spans = self.extract_spans_with_reading_order(page_index, reading_order)?;

        let chars: Vec<crate::layout::TextChar> = spans.iter().flat_map(|s| s.to_chars()).collect();

        let media_box = self.get_page_media_box(page_index)?;

        Ok(crate::layout::PageText {
            spans,
            chars,
            // GH#1653: MediaBox is (llx, lly, urx, ury); the page EXTENT is
            // (urx - llx, ury - lly), not the raw upper-right corner. A
            // non-zero-origin MediaBox (e.g. `[10 -100 622 692]`) previously
            // reported 622x692 instead of the true 612x792. ~keep
            page_width: media_box.2 - media_box.0,
            page_height: media_box.3 - media_box.1,
        })
    }

    /// Extract text spans from a page with custom configuration.
    ///
    /// This method allows controlling span merging behavior through configuration,
    /// including adaptive threshold settings for improved extraction quality.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `config` - SpanMergingConfig controlling extraction parameters
    ///
    /// # Returns
    ///
    /// A vector of TextSpan objects extracted from the page with applied configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # use xberg_native_pdf::extractors::SpanMergingConfig;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    ///
    /// // Use adaptive threshold configuration
    /// let config = SpanMergingConfig::adaptive();
    /// let spans = doc.extract_spans_with_config(0, config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_spans_with_config(
        &self,
        page_index: usize,
        config: crate::extractors::SpanMergingConfig,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        use crate::extractors::TextExtractor;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        // Fast pre-check: skip image-only pages before decompression ~keep
        if self.page_cannot_have_text(page_dict) {
            return Ok(Vec::new());
        }

        // Get content stream data — skip page on decode failure (Annex I) ~keep
        let content_data = match self.get_page_content_data(page_index) {
            Ok(data) => data,
            Err(error) => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "decode_page_content",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "returning empty page content after decode failure"
                );
                return Ok(Vec::new());
            }
        };

        // Early-out for pages with no text content (§9.4.3) ~keep
        if !Self::may_contain_text(&content_data) {
            return Ok(Vec::new());
        }

        let mut extractor = TextExtractor::new().with_merging_config(config);

        if let Some(resources) = page_dict.get("Resources") {
            extractor.set_resources(resources.clone());
            extractor.set_document(self);

            if let Err(error) = self.load_fonts(resources, &mut extractor) {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "load_page_fonts",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "using fallback font encoding"
                );
            }
        }

        extractor.extract_text_spans(&content_data)
    }

    /// Extract individual characters from a PDF page.
    ///
    /// This is a **low-level API** for character-level granularity. For most use cases,
    /// prefer `extract_spans()` which provides complete text strings as PDF defines them.
    ///
    /// # Character-level extraction details:
    ///
    /// - Returns individual `TextChar` objects with position, font, and style information
    /// - Characters are sorted in reading order (top-to-bottom, left-to-right)
    /// - Overlapping characters (rendered multiple times for effects) are deduplicated
    /// - Useful for layout analysis, debugging, or custom text processing pipelines
    ///
    /// # Arguments
    ///
    /// * `page_index` - Page number (0-indexed)
    ///
    /// # Returns
    ///
    /// Vector of `TextChar` objects in reading order, or error if extraction fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("document.pdf")?;
    /// let chars = doc.extract_chars(0)?;
    /// for ch in chars {
    ///     println!("'{}' at ({:.1}, {:.1}), font: {}",
    ///         ch.char, ch.bbox.x, ch.bbox.y, ch.font_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// List all Optional Content Group (OCG) layer names in the document.
    ///
    /// Reads `/OCProperties` from the document catalog and returns the `/Name`
    /// of each OCG dictionary listed in `/OCGs`. These names can be passed to
    /// `extract_text_filtered` / `extract_chars_filtered` via `excluded_layers`.
    ///
    /// Returns an empty vec if the document has no optional content.
    pub fn get_layers(&self) -> Result<Vec<String>> {
        let catalog = self.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let oc_props = match catalog_dict.get("OCProperties") {
            Some(obj) => {
                if let Some(r) = obj.as_reference() {
                    self.load_object(r)?
                } else {
                    obj.clone()
                }
            }
            None => return Ok(Vec::new()),
        };

        let oc_dict = match oc_props.as_dict() {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let ocgs_obj = match oc_dict.get("OCGs") {
            Some(obj) => {
                if let Some(r) = obj.as_reference() {
                    self.load_object(r)?
                } else {
                    obj.clone()
                }
            }
            None => return Ok(Vec::new()),
        };

        let ocgs_arr = match ocgs_obj.as_array() {
            Some(a) => a,
            None => return Ok(Vec::new()),
        };

        let mut names = Vec::new();
        for item in ocgs_arr {
            let ocg_obj = if let Some(r) = item.as_reference() {
                match self.load_object(r) {
                    Ok(o) => o,
                    Err(_) => continue,
                }
            } else {
                item.clone()
            };
            if let Some(d) = ocg_obj.as_dict() {
                if let Some(Object::Name(n)) = d.get("Name") {
                    names.push(n.clone());
                } else if let Some(Object::String(s)) = d.get("Name")
                    && let Ok(text) = String::from_utf8(s.clone())
                {
                    names.push(text);
                }
            }
        }
        Ok(names)
    }

    /// List ink / separation names used on a specific page.
    ///
    /// Scans the page's `/Resources /ColorSpace` dictionary for `/Separation`
    /// and `/DeviceN` color space definitions and returns their ink names.
    /// These names can be passed to `extract_text_filtered` /
    /// `extract_chars_filtered` via `excluded_inks`.
    ///
    /// **Note:** Only the page's own `/Resources` is walked. Spot inks
    /// declared inside a Form XObject's local `/Resources /ColorSpace`
    /// dictionary will not be enumerated — even though the renderer and
    /// extractor will still honor them at use time. Callers populating a
    /// UI picker from this list may miss XObject-local inks.
    ///
    /// For the full walk that follows `Do` operators into Form XObject
    /// resources, use [`Self::get_page_inks_deep`] — that is what the
    /// separation renderer uses to allocate plates.
    pub fn get_page_inks(&self, page_index: usize) -> Result<Vec<String>> {
        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        let resources = match page_dict.get("Resources") {
            Some(r) => {
                if let Some(rr) = r.as_reference() {
                    self.load_object(rr)?
                } else {
                    r.clone()
                }
            }
            None => return Ok(Vec::new()),
        };

        let res_dict = match resources.as_dict() {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let cs_obj = match res_dict.get("ColorSpace") {
            Some(obj) => {
                if let Some(r) = obj.as_reference() {
                    self.load_object(r)?
                } else {
                    obj.clone()
                }
            }
            None => return Ok(Vec::new()),
        };

        let cs_dict = match cs_obj.as_dict() {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        // Resolve any indirect references so the extractor sees inline
        // arrays. Mirrors the pre-existing per-entry resolve loop. ~keep
        let mut resolved: std::collections::HashMap<String, Object> =
            std::collections::HashMap::with_capacity(cs_dict.len());
        for (name, cs_def) in cs_dict.iter() {
            let v = if let Some(r) = cs_def.as_reference() {
                match self.load_object(r) {
                    Ok(o) => o,
                    Err(_) => continue,
                }
            } else {
                cs_def.clone()
            };
            resolved.insert(name.clone(), v);
        }

        let mut ink_names = Vec::new();
        extract_inks_from_color_space_dict(&resolved, Some(self), &mut ink_names);

        ink_names.sort();
        ink_names.dedup();
        Ok(ink_names)
    }

    /// List ink / separation names declared on a page **including** those
    /// declared inside Form XObjects reached through the page's content-stream
    /// `Do` operators.
    ///
    /// Walks the page's content stream looking for `Do` operators that invoke
    /// Form XObjects (§8.10), recurses into each form's `/Resources/ColorSpace`
    /// dictionary, and accumulates `/Separation` and `/DeviceN` ink names from
    /// every visited resource tree.
    ///
    /// **Cycle handling:** indirect XObject references are deduplicated by
    /// `ObjectRef`; recursion depth is bounded at `MAX_RECURSION_DEPTH` (100).
    /// A cycle below the depth bound is silently terminated; a tree deeper
    /// than the bound returns [`Error::RecursionLimitExceeded`].
    ///
    /// **Out of scope:** tiling / shading patterns (§8.7) and annotation
    /// appearance streams (§12.5.5) — both can declare their own colour
    /// spaces but the separation renderer does not paint into them, so
    /// surfacing their inks here would create plates that stay empty.
    pub fn get_page_inks_deep(&self, page_index: usize) -> Result<Vec<String>> {
        let resources = self.page_resources_for_inks(page_index)?;
        let content_data = self.get_page_content_data(page_index)?;
        let operators = crate::content::parser::parse_content_stream(&content_data)?;

        let mut ink_names: Vec<String> = Vec::new();
        let mut visited: std::collections::HashSet<crate::object::ObjectRef> = std::collections::HashSet::new();

        self.collect_inks_from_resources(&resources, &mut ink_names)?;
        self.walk_form_xobject_tree_for_inks(&operators, &resources, &mut ink_names, &mut visited, 0)?;

        ink_names.sort();
        ink_names.dedup();
        Ok(ink_names)
    }

    /// Resolve the page's `/Resources` entry, following an indirect
    /// reference if present. Mirrors the same pattern used by
    /// [`Self::get_page_inks`]. Internal helper that does not depend on
    /// [`Self::get_page_resources`].
    fn page_resources_for_inks(&self, page_index: usize) -> Result<Object> {
        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;
        let resources = match page_dict.get("Resources") {
            Some(r) => match r.as_reference() {
                Some(rr) => self.load_object(rr)?,
                None => r.clone(),
            },
            None => Object::Dictionary(std::collections::HashMap::new()),
        };
        Ok(resources)
    }

    /// Dereference `obj` if it is an indirect reference; otherwise clone.
    /// Internal helper that mirrors the rendering-gated
    /// [`Self::resolve_object`] without taking the gate.
    pub(super) fn deref_object_for_inks(&self, obj: &Object) -> Result<Object> {
        match obj.as_reference() {
            Some(r) => self.load_object(r),
            None => Ok(obj.clone()),
        }
    }

    /// Append inks declared in `resources./ColorSpace` (resolving indirect
    /// references) to `out`. Internal helper for both
    /// [`Self::get_page_inks_deep`] and the recursive form walker.
    fn collect_inks_from_resources(&self, resources: &Object, out: &mut Vec<String>) -> Result<()> {
        let res_dict = match resources.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };
        let cs_obj = match res_dict.get("ColorSpace") {
            Some(obj) => self.deref_object_for_inks(obj)?,
            None => return Ok(()),
        };
        let cs_dict_raw = match cs_obj.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };

        let mut resolved: std::collections::HashMap<String, Object> =
            std::collections::HashMap::with_capacity(cs_dict_raw.len());
        for (name, cs_def) in cs_dict_raw.iter() {
            let v = match cs_def.as_reference() {
                Some(r) => match self.load_object(r) {
                    Ok(o) => o,
                    Err(_) => continue,
                },
                None => cs_def.clone(),
            };
            resolved.insert(name.clone(), v);
        }
        extract_inks_from_color_space_dict(&resolved, Some(self), out);
        Ok(())
    }

    /// Recursive walker: for every `Operator::Do { name }` in `operators` that
    /// resolves to a Form XObject, scan that form's `/Resources/ColorSpace`
    /// and recurse into the form's own content stream.
    ///
    /// `visited` is keyed on the XObject's `ObjectRef` (indirect references
    /// only). Inline-stream forms cannot self-reference (no name to invoke);
    /// the depth limit is the backstop for any other malformed shape.
    fn walk_form_xobject_tree_for_inks(
        &self,
        operators: &[crate::content::operators::Operator],
        parent_resources: &Object,
        out: &mut Vec<String>,
        visited: &mut std::collections::HashSet<crate::object::ObjectRef>,
        depth: u32,
    ) -> Result<()> {
        if depth >= MAX_RECURSION_DEPTH {
            return Err(Error::RecursionLimitExceeded(MAX_RECURSION_DEPTH));
        }
        let xobjects = match parent_resources.as_dict() {
            Some(rd) => match rd.get("XObject") {
                Some(o) => self.deref_object_for_inks(o)?,
                None => return Ok(()),
            },
            None => return Ok(()),
        };
        let xobj_dict = match xobjects.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };

        for op in operators {
            let name = match op {
                crate::content::operators::Operator::Do { name } => name,
                _ => continue,
            };
            let xobj_entry = match xobj_dict.get(name) {
                Some(o) => o,
                None => continue,
            };
            let xobj_ref = xobj_entry.as_reference();
            if let Some(r) = xobj_ref {
                // Cycle through indirect refs: silent skip below depth bound. ~keep
                if !visited.insert(r) {
                    continue;
                }
            }
            let xobj = match self.deref_object_for_inks(xobj_entry) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let (form_dict, form_stream) = match xobj {
                Object::Stream { ref dict, .. } => {
                    if dict.get("Subtype").and_then(Object::as_name) != Some("Form") {
                        continue;
                    }
                    let data = match xobj_ref {
                        Some(r) => self.decode_stream_with_encryption(&xobj, r)?,
                        None => xobj.decode_stream_data()?,
                    };
                    (dict.clone(), data)
                }
                _ => continue,
            };

            // §8.10.1: form may override resources or inherit the parent's. ~keep
            let form_resources = match form_dict.get("Resources") {
                Some(res) => self.deref_object_for_inks(res)?,
                None => parent_resources.clone(),
            };
            self.collect_inks_from_resources(&form_resources, out)?;

            // Recurse into the form's own content stream looking for nested
            // `Do`. Malformed streams are tolerated — we want graceful
            // degradation in a discovery API, not a hard error. ~keep
            let form_ops = match crate::content::parser::parse_content_stream(&form_stream) {
                Ok(ops) => ops,
                Err(_) => continue,
            };
            self.walk_form_xobject_tree_for_inks(&form_ops, &form_resources, out, visited, depth + 1)?;
        }
        Ok(())
    }

    /// # Performance Note
    ///
    /// Character extraction is typically 30-50% faster than span extraction
    /// because it skips the text grouping and merging logic.
    pub fn extract_chars(&self, page_index: usize) -> Result<Vec<crate::layout::TextChar>> {
        Ok((*self.cached_page_chars(page_index)?).clone())
    }

    /// Shared, cached character sequence for a page — identical to what
    /// [`Self::extract_chars`] returns, minus the clone. Only the unfiltered
    /// extraction is cached; the layer/ink-filtered variant is keyed on its
    /// filters and stays uncached.
    pub(super) fn cached_page_chars(&self, page_index: usize) -> Result<std::sync::Arc<Vec<crate::layout::TextChar>>> {
        if let Some(cached) = self.page_chars_cache.lock_or_recover().get(&page_index) {
            return Ok(std::sync::Arc::clone(cached));
        }
        let chars = std::sync::Arc::new(self.extract_chars_impl(page_index, HashSet::new(), HashSet::new())?);
        self.page_chars_cache
            .lock_or_recover()
            .insert(page_index, std::sync::Arc::clone(&chars));
        Ok(chars)
    }

    /// Extract characters from a page, excluding content from specified layers and inks.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `excluded_layers` - OCG layer names to suppress (empty = no layer filtering)
    /// * `excluded_inks` - Separation/DeviceN ink names to suppress (empty = no ink filtering)
    pub fn extract_chars_filtered(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextChar>> {
        self.extract_chars_impl(page_index, excluded_layers, excluded_inks)
    }

    fn extract_chars_impl(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextChar>> {
        use crate::extractors::TextExtractor;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        let content_data = match self.get_page_content_data(page_index) {
            Ok(data) => data,
            Err(error) => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "decode_page_content",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "returning empty page content after decode failure"
                );
                return Ok(Vec::new());
            }
        };

        if !Self::may_contain_text(&content_data) {
            return Ok(Vec::new());
        }

        let mut extractor = TextExtractor::new();
        if !excluded_layers.is_empty() {
            extractor.set_excluded_layers(excluded_layers);
        }
        if !excluded_inks.is_empty() {
            extractor.set_excluded_inks(excluded_inks);
        }

        if let Some(resources) = page_dict.get("Resources") {
            extractor.set_resources(resources.clone());
            extractor.set_document(self);
            if let Err(error) = self.load_fonts(resources, &mut extractor) {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "load_page_fonts",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "using fallback font encoding"
                );
            }
        }

        let mut chars = extractor.extract_owned(&content_data)?;

        chars.sort_by(|a, b| {
            let y_cmp = crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y);
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x)
        });

        Ok(chars)
    }

    /// Extract words from a page.
    ///
    /// Groups characters into words based on spatial proximity.
    /// Uses adaptive thresholds based on the document's font size and spacing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let words = doc.extract_words(0)?;
    /// for word in words {
    ///     println!("Word: {} at {:?}", word.text, word.bbox);
    /// }
    /// ```
    pub fn extract_words(&self, page_index: usize) -> Result<Vec<crate::layout::Word>> {
        self.extract_words_with_thresholds(page_index, None, None)
    }

    /// Extract words from a page with optional threshold and profile overrides.
    ///
    /// When `word_gap_threshold` is `None`, the adaptive threshold is computed
    /// automatically from page statistics (median character width × 0.3).
    /// Providing a value (in PDF points) overrides the adaptive computation,
    /// which is useful for tuning word segmentation on specific document types.
    ///
    /// When `profile` is provided, it controls how the underlying text spans are
    /// extracted from the PDF content stream (TJ offset thresholds, word margin
    /// ratios). This affects the raw character data before word clustering.
    pub fn extract_words_with_thresholds(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
    ) -> Result<Vec<crate::layout::Word>> {
        // Default: include /Artifact-tagged spans (matches pre-0.3.42
        // behavior). The spec-correct (§14.8.2.2.1) variant lives in
        // [`Self::extract_words_with_thresholds_no_artifacts`]. ~keep
        Ok(self
            .extract_words_inner(page_index, word_gap_threshold, profile, true)?
            .0)
    }

    /// Same as [`Self::extract_words_with_thresholds`] but drops spans tagged
    /// as `/Artifact` (running headers/footers, page numbers, watermarks;
    /// ISO 32000-1:2008 §14.8.2.2.1). The spec-correct variant.
    pub fn extract_words_with_thresholds_no_artifacts(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
    ) -> Result<Vec<crate::layout::Word>> {
        Ok(self
            .extract_words_inner(page_index, word_gap_threshold, profile, false)?
            .0)
    }

    pub(super) fn extract_words_inner(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
        include_artifacts: bool,
    ) -> Result<(Vec<crate::layout::Word>, Vec<bool>, Vec<bool>)> {
        use crate::layout::{AdaptiveLayoutParams, DocumentProperties, Word, clustering};

        // Span source. The default (no profile) flows through the canonical
        // `page_reading_order` helper: tagged → struct tree,
        // untagged → geometric top-to-bottom. The legacy profile path keeps
        // its previous XY-Cut + row-aware-sort behavior pending the planned
        // removal of `profile`. ~keep
        let spans: Vec<crate::layout::TextSpan> = match profile {
            Some(p) => {
                use crate::pipeline::reading_order::xycut::XYCutStrategy;
                let config = crate::extractors::TextExtractionConfig::new().with_profile(p);
                let mut s = self.extract_spans_raw_with_extraction_config(page_index, config)?;
                s.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
                if !include_artifacts {
                    s.retain(|span| span.artifact_type.is_none());
                }
                let erase = self.erase_regions.lock_or_recover().get(&page_index).cloned();
                if let Some(regions) = erase {
                    s.retain(|span| !regions.iter().any(|r| r.intersects(&span.bbox)));
                }
                let strategy = XYCutStrategy::new();
                let column_gutter = Self::detect_column_gutter(&s);
                strategy
                    .partition_region(&s, column_gutter)
                    .into_iter()
                    .flatten()
                    .collect()
            }
            None => {
                let ordered = if include_artifacts {
                    crate::pipeline::page_reading_order(self, page_index)?
                } else {
                    crate::pipeline::page_reading_order_no_artifacts(self, page_index)?
                };
                ordered.into_iter().map(|os| os.span).collect()
            }
        };
        if spans.is_empty() {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }

        let media_box = self.get_page_media_box(page_index).unwrap_or((0.0, 0.0, 612.0, 792.0));
        let page_bbox = crate::geometry::Rect::new(media_box.0, media_box.1, media_box.2, media_box.3);

        // Materialize each span's chars ONCE (to_chars allocates + decodes); the
        // word-clustering loop below reuses chars_per_span instead of calling
        // to_chars a second time per span. Byte-identical, halves to_chars work. ~keep
        let mut all_chars: Vec<_> = Vec::new();
        let mut span_char_ranges: Vec<std::ops::Range<usize>> = Vec::with_capacity(spans.len());
        for s in spans.iter() {
            let start = all_chars.len();
            all_chars.extend(s.to_chars());
            span_char_ranges.push(start..all_chars.len());
        }
        if all_chars.is_empty() {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }
        let props = DocumentProperties::analyze(&all_chars, page_bbox).map_err(Error::LayoutAnalysis)?;
        let mut params = AdaptiveLayoutParams::from_properties(&props);

        if let Some(wgt) = word_gap_threshold {
            params.word_gap_threshold = wgt;
        }

        // Walk spans in canonical reading order, clustering chars within each span
        // into words. Since spans come pre-ordered, a flat iteration suffices —
        // no block-by-block partition is needed.
        //
        // Track word indices where the source span had split_boundary_before = true.
        // The post-processing merge must not cross these boundaries (table cells, columns). ~keep
        let mut split_boundary_word_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        // Track word indices produced from spans drawn with a rotated text matrix
        // (rotation_degrees != 0 — figure/axis labels, rotated table headers,
        // vertical margin stamps). Such a run's glyphs advance along a rotated
        // axis, but the span bbox flattens them onto the x-axis (width = Σ glyph
        // advances, height = font). Its flattened bbox therefore overlaps
        // unrelated perpendicular columns, and the reading-order-adjacent word
        // merge below would fuse those columns into one giant token (a whole
        // rotated column returned as a 1000+ char "word"). Never merge
        // into or out of a rotated run. ~keep
        let mut rotated_word_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut words = Vec::new();
        // continues_prev[i]: the span-level merger joined word i to word i-1
        // with no word boundary — their glyphs are consecutive in one span
        // with not even a whitespace char between them, and only the
        // geometric clustering below re-split them. Kept parallel to
        // `words` through the merge loop; consumed by the table path, which
        // must not re-decide a join the merger already made from per-glyph
        // advance evidence. ~keep
        let mut continues_prev: Vec<bool> = Vec::new();
        // space_before[i]: word i's first glyph is immediately preceded in the
        // source span by a whitespace character. This is the producer's own word
        // boundary, and it outranks the geometric merge below: a space whose
        // glyphs happen to abut or overlap their neighbours (a tight Times face at
        // 7.83 pt in GH#1948) has a gap of zero or less, which the merge would
        // otherwise read as "one word" and fuse. Kept parallel to `words`,
        // surviving the merge loop, and consumed by the table path, which must
        // re-emit the source space even when the two words' boxes touch. ~keep
        let mut space_before: Vec<bool> = Vec::new();
        for (span_idx, span) in spans.iter().enumerate() {
            let span_chars = &all_chars[span_char_ranges[span_idx].clone()];
            if span_chars.is_empty() {
                continue;
            }

            // Group characters within THIS SPAN. Since PDF spans are often words or line fragments,
            // this is much safer than global character clustering. ~keep
            let clusters = clustering::cluster_chars_into_words(span_chars, params.word_gap_threshold);

            // Record split boundary: the first word created from this span is a hard
            // boundary when split_boundary_before = true (e.g. table cell boundary). ~keep
            let first_word_idx = words.len();
            let is_split_boundary = span.split_boundary_before;
            let is_rotated_run = span.rotation_degrees != 0.0;

            // Source-index interval of each word emitted for this span,
            // aligned to `words[first_word_idx..]`. Two words continue each
            // other exactly when the second's lowest source index directly
            // follows the first's highest — a dropped whitespace char (or a
            // glyph of another word) in between would occupy that index. ~keep
            let mut word_src_ranges: Vec<(usize, usize)> = Vec::new();
            // Whether a word's first glyph is preceded by source whitespace; see
            // `space_before`. Read from the span's own character stream so the
            // marker reflects source order even when the clustering below visited
            // the glyphs in a different (x-sorted) order. ~keep
            let preceded_by_space = |lo: usize| {
                lo > 0 && {
                    let prev = span_chars[lo - 1].char;
                    prev.is_whitespace() || prev == '\n' || prev == '\r'
                }
            };
            for cluster_indices in clusters {
                let mut current_word_chars = Vec::new();
                let mut src_lo = usize::MAX;
                let mut src_hi = 0usize;
                for &ci in &cluster_indices {
                    let c = span_chars[ci].clone();
                    if c.char.is_whitespace() || c.char == '\n' || c.char == '\r' {
                        if !current_word_chars.is_empty() {
                            let mut word = Word::from_chars(std::mem::take(&mut current_word_chars));
                            word.sequence = span.sequence;
                            words.push(word);
                            continues_prev.push(false);
                            space_before.push(preceded_by_space(src_lo));
                            word_src_ranges.push((src_lo, src_hi));
                            src_lo = usize::MAX;
                            src_hi = 0;
                        }
                    } else {
                        current_word_chars.push(c);
                        src_lo = src_lo.min(ci);
                        src_hi = src_hi.max(ci);
                    }
                }
                if !current_word_chars.is_empty() {
                    let mut word = Word::from_chars(current_word_chars);
                    word.sequence = span.sequence;
                    words.push(word);
                    continues_prev.push(false);
                    space_before.push(preceded_by_space(src_lo));
                    word_src_ranges.push((src_lo, src_hi));
                }
            }
            // Rotated and vertical runs advance perpendicular to the line
            // axis the clustering assumes, so every glyph lands in its own
            // cluster and index adjacency would mark a whole column as one
            // continued word. Leave those unmarked; the rotated case is also
            // what the merge loop below skips.
            //
            // The mark is a source-order statement only. It deliberately
            // carries no geometric test: `to_chars` stamps the span's single
            // `bbox.y` onto every glyph it emits, so all words from one span
            // share a y and no same-line test can discriminate here. Geometry
            // is the consumer's job, where the real coordinates are in hand. ~keep
            if !is_rotated_run && span.wmode == 0 {
                for k in 1..word_src_ranges.len() {
                    let (_, prev_hi) = word_src_ranges[k - 1];
                    let (cur_lo, _) = word_src_ranges[k];
                    if cur_lo == prev_hi + 1 {
                        continues_prev[first_word_idx + k] = true;
                    }
                }
            }

            if is_split_boundary && words.len() > first_word_idx {
                split_boundary_word_indices.insert(first_word_idx);
            }
            if is_rotated_run {
                rotated_word_indices.extend(first_word_idx..words.len());
            }
        }

        // Post-processing: merge adjacent words whose spans abut or overlap on
        // the same line. PDFs (especially tagged CJK documents) sometimes encode
        // typographically-adjacent glyphs as separate marked-content runs, e.g.
        // "Q" and "（peu/d）" with a gap of -0.18 points. Without merging these
        // remain separate tokens and never match the ground-truth "Q（peu/d）". ~keep
        //
        // Merge condition: same line (y_diff ≤ 0.5 × max line height) AND
        // horizontal gap ≤ 0.15 × font_size (same threshold as should_insert_space).
        // Skip merge when the current word index is a split boundary.
        //
        // `gap` has no lower bound above, so a word that BACKTRACKS far behind
        // the previous word's origin also satisfies `gap ≤ 0.15 × font_size`
        // (a large negative number is always ≤ a small positive one). Displayed
        // math draws a fraction's denominator AFTER the relation sign that
        // follows the numerator (`dx/dt = …` → the `=` is emitted, then `dt`
        // starts ~2em further left at a small baseline offset) — this is the
        // exact geometry `assemble_text_from_spans`'s backtrack branch breaks
        // the line on, just reached here through word bboxes instead of span
        // bboxes. Left unguarded, this loop fuses the pair into `"=dt"`, and
        // because the merge is incremental (`prev` grows to the union bbox),
        // a chain of such backtracks collapses into one word spanning an
        // entire equation — the far worse case reported against `main`.
        // Mirror the emitter's guard: a word that starts at-or-left of the
        // previous word's ORIGIN (not just its end), with a real baseline
        // offset and an overlap far beyond ordinary kerning, is a backtrack,
        // not a same-line neighbour — never merge across it. Gated off for
        // RTL text, whose leftward flow is ordinary reading order. ~keep
        let mut merged: Vec<Word> = Vec::with_capacity(words.len());
        // RTL-ness of each entry in `merged`, carried alongside it. `looks_rtl`
        // scans a whole string, and `prev` below GROWS by `push_str` on every
        // merge — re-deriving it per iteration made a chain of k merges cost
        // O(k^2) characters, the same blow-up the backtrack guard above exists
        // to prevent. It is an `any()` over the chars, so
        // `looks_rtl(a + b) == looks_rtl(a) || looks_rtl(b)`: maintain it
        // incrementally instead. ~keep
        let mut merged_rtl: Vec<bool> = Vec::with_capacity(words.len());
        let mut merged_continues: Vec<bool> = Vec::with_capacity(words.len());
        let mut merged_space_before: Vec<bool> = Vec::with_capacity(words.len());
        let mut prev_rotated = false;
        for (idx, ((word, word_continues), word_space_before)) in
            words.into_iter().zip(continues_prev).zip(space_before).enumerate()
        {
            let cur_rotated = rotated_word_indices.contains(&idx);
            let word_rtl = crate::text::bidi::looks_rtl(&word.text);
            if !cur_rotated
                && !prev_rotated
                && !word_space_before
                && !split_boundary_word_indices.contains(&idx)
                && let Some(prev) = merged.last_mut()
            {
                let gap = word.bbox.x - (prev.bbox.x + prev.bbox.width);
                let y_diff = (word.bbox.y - prev.bbox.y).abs();
                let delta_x = word.bbox.x - prev.bbox.x;
                let line_h = prev.bbox.height.max(word.bbox.height);
                let font_size = prev.avg_font_size.max(word.avg_font_size).max(1.0);
                let not_rtl = !merged_rtl.last().copied().unwrap_or(false) && !word_rtl;
                let is_math_backtrack = y_diff > 1.0 && delta_x <= 0.5 && gap < -font_size && not_rtl;
                // A LINE WRAP can land at nearly the same y as the line
                // above it (some producers emit sub-1pt baseline drift
                // between consecutive lines, so `y_diff > 1.0` above
                // doesn't always hold), but it always resets x back
                // toward the page's left margin — an order of magnitude
                // further than any real same-line construct (ordinary
                // kerning is near 0; the math backtrack above is ~1-2em).
                // A multi-em backtrack this large can only be two
                // different lines, never a genuine adjacency — reject it
                // regardless of y_diff, or a wrapped line's tail gets
                // fused onto its own next line's head (e.g. "of whom" +
                // "tered with books" → "whomteredwithbooks"). ~keep
                let is_line_wrap_reset = delta_x < -5.0 * font_size && not_rtl;
                if y_diff <= line_h * 0.5 && gap <= font_size * 0.15 && !is_math_backtrack && !is_line_wrap_reset {
                    // Incremental merge — O(k) per merge, O(total_chars) overall.
                    // Avoids the O(n²) clone+from_chars pattern that caused
                    // catastrophic slowdown on TOC dot-leader pages. ~keep
                    prev.absorb(word);
                    if let Some(flag) = merged_rtl.last_mut() {
                        *flag |= word_rtl;
                    }
                    continue;
                }
            }
            merged.push(word);
            merged_rtl.push(word_rtl);
            merged_continues.push(word_continues);
            merged_space_before.push(word_space_before);
            prev_rotated = cur_rotated;
        }

        Ok((merged, merged_continues, merged_space_before))
    }

    /// Extract text lines from a page.
    ///
    /// Groups words into lines based on vertical proximity.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lines = doc.extract_text_lines(0)?;
    /// for line in lines {
    ///     println!("Line: {} at {:?}", line.text, line.bbox);
    /// }
    /// ```
    pub fn extract_text_lines(&self, page_index: usize) -> Result<Vec<crate::layout::TextLine>> {
        self.extract_text_lines_with_thresholds(page_index, None, None, None)
    }

    /// Extract text lines from a page with optional threshold and profile overrides.
    ///
    /// When thresholds are `None`, adaptive values are computed automatically
    /// from page statistics. Providing values (in PDF points) overrides the
    /// adaptive computation for fine-grained control over segmentation.
    ///
    /// When `profile` is provided, it controls how the underlying text spans are
    /// extracted from the PDF content stream (TJ offset thresholds, word margin
    /// ratios). This affects the raw character data before word/line clustering.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `word_gap_threshold` - Optional override for the horizontal gap (in PDF points)
    ///   used to split characters into words. Smaller values produce more words.
    /// * `line_gap_threshold` - Optional override for the vertical gap (in PDF points)
    ///   used to group words into lines. Smaller values produce more lines.
    /// * `profile` - Optional extraction profile for span-level tuning.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Use adaptive thresholds (default behavior)
    /// let lines = doc.extract_text_lines_with_thresholds(0, None, None, None)?;
    ///
    /// // Tune both thresholds for dense forms
    /// let lines = doc.extract_text_lines_with_thresholds(0, Some(1.5), Some(4.0), None)?;
    /// ```
    pub fn extract_text_lines_with_thresholds(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        line_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
    ) -> Result<Vec<crate::layout::TextLine>> {
        self.extract_text_lines_inner(page_index, word_gap_threshold, line_gap_threshold, profile, true)
    }

    /// Same as [`Self::extract_text_lines_with_thresholds`] but drops spans
    /// tagged as `/Artifact` (running headers/footers, page numbers,
    /// watermarks; ISO 32000-1:2008 §14.8.2.2.1). Spec-correct variant.
    pub fn extract_text_lines_with_thresholds_no_artifacts(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        line_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
    ) -> Result<Vec<crate::layout::TextLine>> {
        self.extract_text_lines_inner(page_index, word_gap_threshold, line_gap_threshold, profile, false)
    }

    fn extract_text_lines_inner(
        &self,
        page_index: usize,
        word_gap_threshold: Option<f32>,
        line_gap_threshold: Option<f32>,
        profile: Option<crate::config::ExtractionProfile>,
        include_artifacts: bool,
    ) -> Result<Vec<crate::layout::TextLine>> {
        use crate::layout::{AdaptiveLayoutParams, DocumentProperties, TextLine, Word, clustering};

        // Span source. Default (no profile) → canonical `page_reading_order`
        // helper. Legacy profile path keeps XY-Cut + row-aware
        // sort pending the planned removal of `profile`. ~keep
        let spans: Vec<crate::layout::TextSpan> = match profile {
            Some(p) => {
                use crate::pipeline::reading_order::xycut::XYCutStrategy;
                let config = crate::extractors::TextExtractionConfig::new().with_profile(p);
                let mut s = self.extract_spans_raw_with_extraction_config(page_index, config)?;
                s.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
                if !include_artifacts {
                    s.retain(|span| span.artifact_type.is_none());
                }
                let erase = self.erase_regions.lock_or_recover().get(&page_index).cloned();
                if let Some(regions) = erase {
                    s.retain(|span| !regions.iter().any(|r| r.intersects(&span.bbox)));
                }
                let strategy = XYCutStrategy::new();
                let column_gutter = Self::detect_column_gutter(&s);
                strategy
                    .partition_region(&s, column_gutter)
                    .into_iter()
                    .flatten()
                    .collect()
            }
            None => {
                let ordered = if include_artifacts {
                    crate::pipeline::page_reading_order(self, page_index)?
                } else {
                    crate::pipeline::page_reading_order_no_artifacts(self, page_index)?
                };
                ordered.into_iter().map(|os| os.span).collect()
            }
        };
        if spans.is_empty() {
            return Ok(Vec::new());
        }

        let media_box = self.get_page_media_box(page_index).unwrap_or((0.0, 0.0, 612.0, 792.0));
        let page_bbox = crate::geometry::Rect::new(media_box.0, media_box.1, media_box.2, media_box.3);

        // Materialize each span's chars once (see extract_text_as_words). ~keep
        let mut all_chars: Vec<_> = Vec::new();
        let mut span_char_ranges: Vec<std::ops::Range<usize>> = Vec::with_capacity(spans.len());
        for s in spans.iter() {
            let start = all_chars.len();
            all_chars.extend(s.to_chars());
            span_char_ranges.push(start..all_chars.len());
        }
        let props = DocumentProperties::analyze(&all_chars, page_bbox).map_err(Error::LayoutAnalysis)?;
        let mut params = AdaptiveLayoutParams::from_properties(&props);

        if let Some(wgt) = word_gap_threshold {
            params.word_gap_threshold = wgt;
        }
        if let Some(lgt) = line_gap_threshold {
            params.line_gap_threshold = lgt;
        }

        // Walk spans in canonical reading order, clustering chars → words.
        // No block partition; spans are already pre-ordered.
        //
        // `word_rot_run` maps each word to the index of the rotated span it came
        // from (`None` for horizontal spans). A rotated run's glyphs advance
        // along a rotated axis but the span bbox flattens them onto the x-axis,
        // so the flattened y-band line clustering below would fuse the run with
        // its perpendicular neighbours into one giant line. Rotated runs
        // are therefore lifted out and each emitted as its own line. ~keep
        let mut words: Vec<Word> = Vec::new();
        let mut word_rot_run: Vec<Option<usize>> = Vec::new();
        for (span_idx, span) in spans.iter().enumerate() {
            let span_chars = &all_chars[span_char_ranges[span_idx].clone()];
            if span_chars.is_empty() {
                continue;
            }
            let rot_run = (span.rotation_degrees != 0.0).then_some(span_idx);

            let clusters = clustering::cluster_chars_into_words(span_chars, params.word_gap_threshold);
            for cluster_indices in clusters {
                let cluster_chars: Vec<_> = cluster_indices.iter().map(|&i| span_chars[i].clone()).collect();
                let mut current_word_chars = Vec::new();
                for c in cluster_chars {
                    if c.char.is_whitespace() || c.char == '\n' || c.char == '\r' {
                        if !current_word_chars.is_empty() {
                            let mut word = Word::from_chars(current_word_chars);
                            word.sequence = span.sequence;
                            words.push(word);
                            word_rot_run.push(rot_run);
                            current_word_chars = Vec::new();
                        }
                    } else {
                        current_word_chars.push(c);
                    }
                }
                if !current_word_chars.is_empty() {
                    let mut word = Word::from_chars(current_word_chars);
                    word.sequence = span.sequence;
                    words.push(word);
                    word_rot_run.push(rot_run);
                }
            }
        }

        if words.is_empty() {
            return Ok(Vec::new());
        }

        // Fast path (byte-identical): no rotated runs on the page → cluster every
        // word by global y-tolerance exactly as before. Same-y words merge into
        // the same line regardless of source span (span ordering already handled
        // the multi-column / structure-tree sequencing upstream). ~keep
        if word_rot_run.iter().all(Option::is_none) {
            let line_clusters = clustering::cluster_words_into_lines(&words, params.line_gap_threshold);
            let mut all_lines = Vec::new();
            for cluster_indices in line_clusters {
                let cluster_words: Vec<_> = cluster_indices.iter().map(|&i| words[i].clone()).collect();
                all_lines.push(TextLine::new(cluster_words));
            }
            return Ok(all_lines);
        }

        let horizontal: Vec<Word> = words
            .iter()
            .zip(word_rot_run.iter())
            .filter(|(_, r)| r.is_none())
            .map(|(w, _)| w.clone())
            .collect();
        let mut lines: Vec<Vec<Word>> = Vec::new();
        if !horizontal.is_empty() {
            for cluster_indices in clustering::cluster_words_into_lines(&horizontal, params.line_gap_threshold) {
                lines.push(cluster_indices.iter().map(|&i| horizontal[i].clone()).collect());
            }
        }
        // Rotated runs, grouped into lines. A run's own extents are recorded in
        // its frame, so the offset BETWEEN lines is the coordinate the run does
        // not advance along: x for a +-90 degree run, y for 180. Runs that share
        // that offset are one visual line however many `Tm`s drew them — a
        // rotated table row is typically one run per cell. Grouping by run alone
        // returns one line per cell; grouping without the rotation key
        // fuses perpendicular columns into one line. ~keep
        let mut run_first_word: Vec<(usize, usize)> = Vec::new();
        let mut run_start = 0;
        while run_start < words.len() {
            match word_rot_run[run_start] {
                None => run_start += 1,
                Some(run_id) => {
                    let mut run_end = run_start + 1;
                    while run_end < words.len() && word_rot_run[run_end] == Some(run_id) {
                        run_end += 1;
                    }
                    run_first_word.push((run_start, run_end));
                    run_start = run_end;
                }
            }
        }

        // On a /Rotate page `postprocess_spans` rect-maps each span's bbox into
        // the displayed frame but leaves `rotation_degrees` describing the
        // pre-display one, so the bbox axes and the rotation no longer agree and
        // the offset read below would be taken off the wrong axis. ~keep
        let page_is_unrotated = self.get_page_rotation(page_index).unwrap_or(0) == 0;

        let mut rotated_lines: Vec<(f32, f32, Vec<Word>)> = Vec::new();
        // Offsets of the quarter-turn lines, quantized to 1/100 pt, mapped to
        // the indices of the lines carrying them. The match below used to scan
        // every line built so far, which is O(runs^2) — and a rotated table,
        // the exact page this grouping exists for, is where the run count
        // climbs. The index answers the same question over a bounded range.
        //
        // Semantics are preserved exactly. A line's offset is frozen when it is
        // created, and the winner is the FIRST such line in insertion order, so
        // candidates are filtered by the original predicate and the smallest
        // index wins. Merging neighbours instead would group differently:
        // offsets 0, 3, 6 with tolerance 4 give {0,3},{6} under this rule but
        // {0,3,6} under a chained merge. ~keep
        let mut offset_index: std::collections::BTreeMap<i64, Vec<usize>> = std::collections::BTreeMap::new();
        for (start, end) in run_first_word {
            let word = &words[start];
            let rotation = spans[word_rot_run[start].expect("rotated run")].rotation_degrees;
            // Only a quarter-turn run has its line offset on x; 180 degrees and
            // free angles keep one line per run, as before. Widening past this
            // merges runs that were never one line. ~keep
            let quarter_turn = (rotation.abs() - 90.0).abs() < 0.5;
            if !quarter_turn || !page_is_unrotated {
                rotated_lines.push((rotation, f32::NAN, words[start..end].to_vec()));
                continue;
            }
            let across = word.bbox.x;
            let tolerance = word.bbox.height.max(1.0) * 0.5;
            let quantize = |v: f32| (f64::from(v) * 100.0).round() as i64;
            // Widened by one step each way so a value sitting on a bucket edge
            // cannot be missed by rounding. Saturating: the cast in `quantize`
            // clamps |v| >= ~9.2e16 to i64::MAX/MIN, and a plain +-1 there
            // overflows (panic in debug, an inverted `range` panic in release). ~keep
            let (lo, hi) = (
                quantize(across - tolerance).saturating_sub(1),
                quantize(across + tolerance).saturating_add(1),
            );
            let winner = offset_index
                .range(lo..=hi)
                .flat_map(|(_, indices)| indices.iter().copied())
                .filter(|&i| {
                    let (rot, off, _) = &rotated_lines[i];
                    (*rot - rotation).abs() < 0.5 && (*off - across).abs() <= tolerance
                })
                .min();
            match winner {
                Some(i) => rotated_lines[i].2.extend_from_slice(&words[start..end]),
                None => {
                    offset_index
                        .entry(quantize(across))
                        .or_default()
                        .push(rotated_lines.len());
                    rotated_lines.push((rotation, across, words[start..end].to_vec()));
                }
            }
        }
        lines.extend(rotated_lines.into_iter().map(|(rot, off, mut line)| {
            // A merged quarter-turn line collects its runs in arrival order,
            // which is content-stream order: a subscript drawn after the
            // line's tail lands at the end of the string. Order members
            // along the writing axis instead — ascending y for +90,
            // descending for -90 — the order the text assembler reads them
            // in. Stable, so runs sharing a coordinate keep drawing order. ~keep
            if !off.is_nan() {
                if rot > 0.0 {
                    line.sort_by(|a, b| a.bbox.y.total_cmp(&b.bbox.y));
                } else {
                    line.sort_by(|a, b| b.bbox.y.total_cmp(&a.bbox.y));
                }
            }
            line
        }));
        // Reading order: sort lines by the span sequence of their first word
        // (stable so intra-line order is preserved). ~keep
        lines.sort_by_key(|line| line.first().map(|w| w.sequence).unwrap_or(usize::MAX));

        Ok(lines.into_iter().map(TextLine::new).collect())
    }

    /// Apply intelligent text post-processing to extracted text spans.
    ///
    /// This method applies several text quality improvements:
    /// - Ligature expansion (fi, fl, ffi, ffl → component characters)
    /// - Hyphenation reconstruction (rejoins words split across lines)
    /// - Whitespace normalization (removes excess spaces within words)
    /// - Special character spacing (Greek letters, math symbols)
    /// - OCR text cleanup (when font_name == "OCR" or from known OCR engines)
    ///
    /// # Arguments
    ///
    /// * `spans` - Vector of TextSpan extracted from pages
    ///
    /// # Returns
    ///
    /// Processed spans with improved text quality
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    ///
    /// // Extract spans from page
    /// let spans = doc.extract_spans(0)?;
    ///
    /// // Apply intelligent processing
    /// let processed = doc.apply_intelligent_text_processing(spans);
    ///
    /// for span in &processed {
    ///     println!("{}", span.text); // Ligatures expanded, hyphenation fixed
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply_intelligent_text_processing(&self, mut spans: Vec<TextSpan>) -> Vec<TextSpan> {
        use crate::converters::text_post_processor::TextPostProcessor;

        for span in &mut spans {
            let is_ocr = span.font_name == "OCR"
                || span.font_name.to_lowercase().contains("tesseract")
                || span.font_name.to_lowercase().contains("abbyy");

            // Step 2: Apply text post-processing pipeline
            // (hyphenation, whitespace, special char spacing).
            // Ligature characters from the font's ToUnicode map are preserved as-is. ~keep
            span.text = TextPostProcessor::process(&span.text);

            if is_ocr {
                span.text = span
                    .text
                    .replace("ﬁ", "fi")
                    .replace("ﬂ", "fl")
                    .replace("ﬀ", "ff")
                    .replace("  ", " ");
            }
        }

        spans
    }

    /// Extract hierarchical content structure from a page.
    ///
    /// Returns the page's hierarchical content structure with all children populated.
    /// For tagged PDFs with structure trees, returns the structure with extracted content.
    /// For untagged PDFs, returns a synthetic hierarchy based on geometric analysis.
    ///
    /// # Arguments
    ///
    /// * `page_index` - The page to extract from (0-indexed)
    ///
    /// # Returns
    ///
    /// `Ok(Some(structure))` if structure is found or generated,
    /// `Ok(None)` if no structure is available,
    /// `Err` if an error occurs during extraction
    ///
    /// # PDF Spec Compliance
    ///
    /// - ISO 32000-1:2008, Section 14.7 - Logical Structure
    /// - ISO 32000-1:2008, Section 14.8 - Tagged PDF
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    ///
    /// // Extract hierarchical structure from first page
    /// if let Some(structure) = doc.extract_hierarchical_content(0)? {
    ///     println!("Document structure type: {}", structure.structure_type);
    ///     println!("Number of children: {}", structure.children.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_hierarchical_content(&self, page_index: usize) -> Result<Option<crate::elements::StructureElement>> {
        use crate::extractors::HierarchicalExtractor;
        HierarchicalExtractor::extract_page(self, page_index)
    }

    /// Get the raw content stream data for a page.
    ///
    /// This returns the decoded content stream bytes for the specified page.
    /// The content stream contains PDF operators that define the page's appearance.
    pub fn get_page_content_data(&self, page_index: usize) -> Result<Vec<u8>> {
        Ok((*self.cached_page_content(page_index)?).clone())
    }

    /// Shared, cached content-stream bytes for a page — the same data
    /// [`Self::get_page_content_data`] returns, minus the copy. Extraction
    /// only ever reads the bytes, and a single `extract_words` page touches
    /// this twice (once for spans, once for chars), so handing back the `Arc`
    /// avoids copying the decompressed stream on every call.
    fn cached_page_content(&self, page_index: usize) -> Result<std::sync::Arc<Vec<u8>>> {
        {
            let mut cache = self.page_content_cache.lock_or_recover();
            if let Some(data) = cache.get(&page_index) {
                return Ok(std::sync::Arc::clone(data));
            }
        }

        self.ensure_encryption_initialized()?;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        // Get content stream(s) — Contents is optional per ISO 32000-1:2008 Table 30 ~keep
        let contents_ref = match page_dict.get("Contents") {
            Some(Object::Null) | None => {
                tracing::debug!(target: LOG_TARGET, "Page {} has no /Contents (blank page)", page_index);
                return Ok(std::sync::Arc::new(Vec::new()));
            }
            Some(c) => c,
        };

        let content_data = if let Some(contents_ref_val) = contents_ref.as_reference() {
            let contents = self.load_object(contents_ref_val)?;

            if let Some(contents_array) = contents.as_array() {
                let mut combined = Vec::new();

                for content_item in contents_array.iter() {
                    if matches!(content_item, Object::Null) {
                        continue;
                    }
                    match (|| -> Result<Vec<u8>> {
                        if let Some(ref_val) = content_item.as_reference() {
                            let content_obj = self.load_object(ref_val)?;
                            self.decode_stream_with_encryption(&content_obj, ref_val)
                        } else {
                            content_item.decode_stream_data()
                        }
                    })() {
                        Ok(decoded) => {
                            combined.extend_from_slice(&decoded);
                            combined.push(b'\n');
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: crate::LOG_TARGET_ROOT,
                                operation = "decode_optional_page_content",
                                page_index,
                                error_code = error.telemetry_code(),
                                error_offset = ?error.telemetry_offset(),
                                "skipping corrupt optional page content stream"
                            );
                        }
                    }
                }

                combined
            } else {
                self.decode_stream_with_encryption(&contents, contents_ref_val)?
            }
        } else if let Some(contents_array) = contents_ref.as_array() {
            let mut combined = Vec::new();

            for content_item in contents_array.iter() {
                if matches!(content_item, Object::Null) {
                    continue;
                }
                match (|| -> Result<Vec<u8>> {
                    if let Some(ref_val) = content_item.as_reference() {
                        let content_obj = self.load_object(ref_val)?;
                        self.decode_stream_with_encryption(&content_obj, ref_val)
                    } else {
                        content_item.decode_stream_data()
                    }
                })() {
                    Ok(decoded) => {
                        combined.extend_from_slice(&decoded);
                        combined.push(b'\n');
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: crate::LOG_TARGET_ROOT,
                            operation = "decode_optional_page_content",
                            page_index,
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "skipping corrupt optional page content stream"
                        );
                    }
                }
            }

            combined
        } else {
            // Direct stream object (rare but possible)
            // For direct objects, use regular decoding (no encryption key) ~keep
            contents_ref.decode_stream_data()?
        };

        tracing::trace!(target: LOG_TARGET,
            page = page_index,
            bytes = content_data.len(),
            "retrieved page content data"
        );

        let content_data = std::sync::Arc::new(content_data);
        self.page_content_cache
            .lock_or_recover()
            .insert(page_index, std::sync::Arc::clone(&content_data));

        Ok(content_data)
    }
}
