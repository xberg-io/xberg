//! Content-stream entry points.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// Extract text from a content stream.
    ///
    /// Parses the content stream and executes operators to extract positioned
    /// characters with Unicode mappings and font information.
    ///
    /// # Arguments
    ///
    /// * `content_stream` - The raw content stream data (should be decoded first)
    ///
    /// # Returns
    ///
    /// A vector of TextChar structures containing positioned characters.
    ///
    /// # Errors
    ///
    /// Returns an error if the content stream cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::extractors::TextExtractor;
    /// # fn example(content_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut extractor = TextExtractor::new();
    /// let chars = extractor.extract(content_data)?;
    /// println!("Extracted {} characters", chars.len());
    /// # Ok(())
    /// # }
    /// ```
    /// Extract text as complete spans (PDF spec compliant).
    ///
    /// This is the recommended method for text extraction. It extracts complete
    /// text strings as the PDF provides them via Tj/TJ operators, following the
    /// PDF specification ISO 32000-1:2008.
    ///
    /// # Benefits
    /// - Avoids overlapping character issues
    /// - Preserves PDF's text positioning intent
    /// - More robust for complex layouts
    /// - Matches industry best practices
    ///
    /// # Arguments
    ///
    /// * `content_stream` - The PDF content stream data
    ///
    /// # Returns
    ///
    /// Vector of TextSpan objects in reading order
    #[tracing::instrument(name = "pdf.extract_spans", skip_all, fields(bytes = content_stream.len()))]
    pub fn extract_text_spans(&mut self, content_stream: &[u8]) -> Result<Vec<TextSpan>> {
        // Charge every recoverable decode in this page to the document, so it
        // reports one summary instead of one WARN per byte run (GH#1547). ~keep
        let _recovery_scope = self
            .document
            .map(|doc| crate::extractors::recovery_tally::RecoveryScope::enter(doc.recovery_counts()));
        let result = self.extract_text_spans_impl(content_stream);
        if let Err(error) = &result {
            crate::error::trace_failure("extract_text_spans", error);
        }
        result
    }

    fn extract_text_spans_impl(&mut self, content_stream: &[u8]) -> Result<Vec<TextSpan>> {
        self.extract_spans = true;
        self.spans.clear();
        self.span_sequence_counter = 0;
        self.placed_pdf_keep = Self::placed_pdf_text_dominates(content_stream);

        extract_log_debug!("Parsing content stream for text extraction");
        self.run_content_stream(content_stream)?;

        self.flush_tj_span_buffer()?;

        // Detect RTL glyph DRAW DIRECTION on the raw stream order, BEFORE the
        // reading-order sort destroys it (ISO 32000-1 §14.8.2.3.3 method 1). ~keep
        self.detect_rtl_draw_direction();

        if tracing::enabled!(tracing::Level::DEBUG) {
            let space_spans = self
                .spans
                .iter()
                .filter(|s| s.text.chars().all(|c| c.is_whitespace()))
                .count();
            let offset_semantic = self.spans.iter().filter(|s| s.offset_semantic).count();
            tracing::debug!(target: LOG_TARGET,
                "Before sort_spans_by_reading_order(): {} spans total, {} space-only, {} offset_semantic=true",
                self.spans.len(),
                space_spans,
                offset_semantic
            );
        }

        // Snap super/subscript glyph spans onto the baseline of an
        // adjacent base span BEFORE row-aware sorting. PDFs raise
        // or lower the text matrix via the `Ts` (text-rise) operator
        // for super/subscripts (§9.3.7); the rendered glyphs end up
        // at a Y offset of typically 0.3–0.5 × font_size from the
        // baseline. Without the snap, sorting groups all raised
        // glyphs into a separate Y-band above the body, producing
        // output like `"1,2 ★ 3,4 5 / Chibueze, …"` instead of
        // `"Chibueze,1,2★ Caleb,3,4† …"`. ~keep
        self.snap_superscript_baselines();

        self.sort_spans_by_reading_order();

        self.deduplicate_overlapping_spans();

        self.merge_adjacent_spans();

        // Resolve each span's font resource alias (e.g. "F1") to the resolved
        // /BaseFont name (e.g. "Helvetica", "CIDFont+F1") so extract_spans /
        // extract_words / extract_text_lines report the same font name that
        // extract_chars already does (which reads `font.base_font`). Run AFTER
        // merging so span reconstruction still keys off the raw resource alias
        // exactly as before, and it has no effect on the assembled text/md/html
        // output (font names are not emitted there) — only the API surface.
        //
        // The §9.10.2 mapping provenance (the tier the span's font offered, or
        // `Fallback` when it carries no mapping resource — the text is then a
        // fabricated glyph-index echo, not read from the file) is looked up in
        // THIS SAME PASS, by the raw resource alias: `self.fonts` is keyed by
        // that alias (`add_font`/`add_font_shared`), never by the resolved
        // /BaseFont name. Looking it up after the rename below finds nothing
        // for any font whose /BaseFont differs from its resource alias — which
        // is effectively every real font — silently dropping the provenance
        // fact issue #1254 exists to carry (issue #1667). `None` when the font
        // is unresolvable. ~keep
        let resolved: Vec<(Option<String>, Option<crate::fonts::MappingProvenance>)> = self
            .spans
            .iter()
            .map(|s| {
                self.fonts.get(&s.font_name).map_or((None, None), |f| {
                    (
                        Some(f.base_font.clone()).filter(|b| !b.is_empty()),
                        Some(f.best_mapping_provenance()),
                    )
                })
            })
            .collect();
        for (span, (resolved_base_font, provenance)) in self.spans.iter_mut().zip(resolved) {
            if let Some(base_font) = resolved_base_font {
                span.font_name = base_font;
            }
            span.provenance = provenance;
        }

        Ok(std::mem::take(&mut self.spans))
    }

    /// Feed a content stream's operators to `execute_operator`.
    ///
    /// The single parser dispatch for every extraction mode (chars, spans,
    /// Form XObject recursion). The modes must differ only in how
    /// `execute_operator` handles the show-text operators, never in which
    /// operators reach it: char and span mode once used different parsers
    /// (`parse_content_stream_text_only` vs `parse_and_execute_text_only`),
    /// which reconstruct the graphics state around a text region by different
    /// routes, so they could agree on every text operator and still hand the
    /// extractor a different CTM. On one real-world page that cost
    /// a 90°-rotated chart axis: char mode returned 2322 glyphs, all at
    /// rotation 0, where span mode saw 2590 glyphs, 122 at 90°.
    pub(super) fn run_content_stream(&mut self, content_stream: &[u8]) -> Result<()> {
        if self.excluded_inks.is_empty() {
            parse_and_execute_text_only(content_stream, |op| self.execute_operator(op))
        } else {
            // Ink filtering needs the color operators (cs, rg, g, k). The
            // text-only parser does not guarantee their delivery — its >256KB
            // prescan route parses only text regions — so use the full parser. ~keep
            let operators = parse_content_stream(content_stream)?;
            for op in operators {
                self.execute_operator(op)?;
            }
            Ok(())
        }
    }

    /// Extract individual characters from a PDF content stream.
    ///
    /// This is a low-level method that extracts characters one by one.
    /// For most use cases, prefer using `extract_text_spans()` which groups
    /// characters into text spans according to PDF semantics.
    pub fn extract(&mut self, content_stream: &[u8]) -> Result<Vec<TextChar>> {
        // Charge every recoverable decode in this page to the document, so it
        // reports one summary instead of one WARN per byte run (GH#1547). ~keep
        let _recovery_scope = self
            .document
            .map(|doc| crate::extractors::recovery_tally::RecoveryScope::enter(doc.recovery_counts()));
        self.extract_into_self(content_stream)?;
        Ok(self.chars.clone())
    }

    /// Run the character extraction and leave the result in `self.chars`.
    fn extract_into_self(&mut self, content_stream: &[u8]) -> Result<()> {
        self.extract_spans = false;
        self.chars.clear();
        self.spans.clear(); // Ensure spans are clear so they don't poison xobject_spans_cache ~keep
        self.placed_pdf_keep = Self::placed_pdf_text_dominates(content_stream);

        self.run_content_stream(content_stream)?;

        // Sort characters by reading order (top-to-bottom, left-to-right)
        // PDF content streams are in rendering order, not reading order.
        // PDF Y coordinates increase upward, so higher Y = top of page.
        // We need to sort by Y descending (top first), then X ascending (left to right). ~keep
        self.sort_by_reading_order();

        // Deduplicate overlapping characters
        // Some PDFs render text multiple times (for effects like boldness, shadowing).
        // This causes characters to appear at very close X positions (< 2pt).
        // We deduplicate by keeping only the first character when multiple chars
        // at the same Y position have X positions within 2pt of each other. ~keep
        self.deduplicate_overlapping_chars();

        Ok(())
    }

    /// Same extraction as [`Self::extract`], but hands the buffer over instead
    /// of copying it. Every `TextChar` owns a `font_name` String, so `extract`'s
    /// clone re-allocates once per glyph — measurable on long documents. Leaves
    /// `self.chars` empty, so callers that read `char_count`/`chars` afterwards
    /// must keep using [`Self::extract`].
    pub fn extract_owned(&mut self, content_stream: &[u8]) -> Result<Vec<TextChar>> {
        self.extract_into_self(content_stream)?;
        Ok(std::mem::take(&mut self.chars))
    }
}
