//! Construction, configuration, and per-page state setters.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// Create a new text extractor with default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::TextExtractor;
    ///
    /// let extractor = TextExtractor::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(TextExtractionConfig::default())
    }

    /// Create a new text extractor with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for text extraction heuristics
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::{TextExtractor, TextExtractionConfig};
    ///
    /// // Use custom space threshold
    /// let config = TextExtractionConfig::with_space_threshold(-80.0);
    /// let extractor = TextExtractor::with_config(config);
    /// ```
    pub fn with_config(config: TextExtractionConfig) -> Self {
        let word_boundary_mode = config.word_boundary_mode;
        Self {
            state_stack: GraphicsStateStack::new(),
            fonts: HashMap::new(),
            font_provenance_cache: Default::default(),
            spans: Vec::new(),
            preserve_script_baselines: false,
            chars: Vec::new(),
            resources: None,
            document: None,
            processed_xobjects: HashSet::new(),
            cached_xobject_refs: HashMap::new(),
            xobject_depth: 0,
            xobject_decode_count: 0,
            config,
            merging_config: SpanMergingConfig::default(),
            current_mcid: None,
            mc_actualtext_mcids: HashSet::new(),
            extract_spans: true,
            tj_span_buffer: None,
            span_sequence_counter: 0,
            marked_content_stack: Vec::new(),
            saw_reversed_chars: false,
            inside_artifact: false,
            excluded_layers: HashSet::new(),
            inside_excluded_layer: false,
            inside_placed_pdf: false,
            placed_pdf_keep: false,
            placed_hold: 0,
            placed_hold_groups: 0,
            span_holds: Vec::new(),
            char_holds: Vec::new(),
            form_placed_tags: 0,
            excluded_inks: HashSet::new(),
            inside_excluded_ink: false,
            tj_offset_history: Vec::with_capacity(1000),
            tj_sum: 0.0,
            tj_sum_sq: 0.0,
            tj_stats_len: 0,
            tj_character_array: Vec::new(),
            current_x_position: 0.0,
            word_boundary_mode,
            cached_current_font: None,
            extraction_width_tables: HashMap::new(),
            cached_extraction_widths: None,
            // Default to Page(0); `set_page_index` overrides before
            // extraction. Form XObject `Do` invocations push their
            // own scope on top. ~keep
            mcid_scope_stack: vec![crate::structure::McidScope::Page(0)],
        }
    }

    /// Stamp this extractor with the page index it is processing.
    ///
    /// Used so spans (and the lookup keys for `/ActualText`) carry the
    /// correct `McidScope::Page(page_index)` when the extractor is not
    /// currently inside a Form XObject.
    pub fn set_page_index(&mut self, page_index: u32) {
        // The first entry is always the page scope (Form scopes are
        // pushed on top by `Do` and popped before the extractor
        // finishes); update it in place. ~keep
        if let Some(first) = self.mcid_scope_stack.first_mut() {
            *first = crate::structure::McidScope::Page(page_index);
        } else {
            self.mcid_scope_stack
                .push(crate::structure::McidScope::Page(page_index));
        }
    }

    pub(super) fn set_cached_current_font(&mut self, font: Option<Arc<FontInfo>>) {
        self.cached_extraction_widths = font.as_ref().filter(|font| font.subtype != "Type0").map(|font| {
            let key = Arc::as_ptr(font) as usize;
            self.extraction_width_tables
                .entry(key)
                .or_insert_with(|| (font.clone(), Arc::new(font.build_extraction_byte_widths())))
                .1
                .clone()
        });
        self.cached_current_font = font;
    }

    pub(super) fn simple_widths<'a>(
        cached: Option<&'a [f32; 256]>,
        font: &'a FontInfo,
        repair_zero_widths: bool,
    ) -> &'a [f32; 256] {
        if repair_zero_widths {
            cached.unwrap_or_else(|| font.get_byte_to_width_table())
        } else {
            font.get_byte_to_width_table()
        }
    }

    /// Current MCID scope (top of the stack) — what should be stamped
    /// on every new `TextSpan`.
    pub(super) fn current_mcid_scope(&self) -> crate::structure::McidScope {
        self.mcid_scope_stack
            .last()
            .cloned()
            .unwrap_or(crate::structure::McidScope::Page(0))
    }

    /// Create a new text extractor with custom merging configuration.
    ///
    /// This allows fine-tuning how adjacent spans are merged and when spaces
    /// are inserted, useful for documents with unusual spacing patterns.
    ///
    /// # Arguments
    ///
    /// * `merging_config` - Configuration for span merging thresholds
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::extractors::{TextExtractor, SpanMergingConfig};
    ///
    /// // Use aggressive space insertion for dense layouts
    /// let config = SpanMergingConfig::aggressive();
    /// let extractor = TextExtractor::new().with_merging_config(config);
    /// ```
    pub fn with_merging_config(mut self, merging_config: SpanMergingConfig) -> Self {
        self.merging_config = merging_config;
        self
    }

    /// Set the resources dictionary for this extractor.
    ///
    /// This allows the extractor to access XObjects and fonts during extraction.
    pub fn set_resources(&mut self, resources: Object) {
        self.resources = Some(resources);
    }

    /// Set the document reference for loading XObjects.
    pub fn set_document(&mut self, document: &'doc crate::document::PdfDocument) {
        self.document = Some(document);
    }

    /// Take ownership of the set of MCIDs whose marked-content
    /// sequence carried an inline `/ActualText` property on this
    /// extraction.
    ///
    /// The set is observed by the BDC handler; this method drains it
    /// out so the document layer can stash it on a per-page side
    /// channel for the struct-tree-scope ActualText applier.
    pub fn take_mc_actualtext_mcids(&mut self) -> HashSet<u32> {
        std::mem::take(&mut self.mc_actualtext_mcids)
    }

    /// Set layer names (Optional Content Groups) to exclude from extraction.
    ///
    /// Content within BDC/EMC scopes tagged "OC" whose OCG /Name matches one of
    /// the provided names will be suppressed during text extraction.
    pub fn set_excluded_layers(&mut self, layers: HashSet<String>) {
        self.excluded_layers = layers;
    }

    /// Set ink / separation names to exclude from extraction.
    ///
    /// When the fill color space is a Separation or DeviceN whose ink name(s)
    /// intersect with any of the provided names, subsequent text is suppressed
    /// until the color space changes to a non-excluded one.
    ///
    /// **DeviceN behavior:** For DeviceN color spaces (e.g.
    /// `[/DeviceN [/Cyan /SpotGold] ...]`), text is suppressed if ANY ink in
    /// the array matches — even process colors sharing the DeviceN definition.
    /// This is because tint values are not evaluated during extraction.
    pub fn set_excluded_inks(&mut self, inks: HashSet<String>) {
        self.excluded_inks = inks;
    }

    /// Convenience wrapper: identical to `set_document`.
    pub fn set_document_ptr(&mut self, doc: &'doc crate::document::PdfDocument) {
        self.set_document(doc);
    }

    /// Prepare for span extraction mode (same setup as extract_text_spans preamble).
    pub fn prepare_for_span_extraction(&mut self) {
        self.extract_spans = true;
        self.spans.clear();
        self.span_holds.clear();
        self.span_sequence_counter = 0;
    }

    /// Public wrapper for execute_operator (normally private).
    pub fn execute_operator_public(&mut self, op: crate::content::Operator) -> Result<()> {
        self.execute_operator(op)
    }

    /// Public wrapper for flush_tj_span_buffer (normally private).
    pub fn flush_public(&mut self) -> Result<()> {
        self.flush_tj_span_buffer()
    }

    /// Get the number of extracted characters.
    pub fn char_count(&self) -> usize {
        self.chars.len()
    }

    /// Clear all extracted characters.
    pub fn clear(&mut self) {
        self.chars.clear();
        self.char_holds.clear();
    }
}
