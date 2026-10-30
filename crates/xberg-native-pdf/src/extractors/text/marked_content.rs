//! Marked-content scopes, artifacts, optional content, and font registration.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

/// Length and bytes of the text-show operands in one `/PlacedPDF` bucket.
#[derive(Debug, Default, Clone)]
pub(super) struct PlacedBucket {
    pub(super) chars: usize,
    txt: Vec<u8>,
}

impl PlacedBucket {
    /// Add the text of another bucket to this one.
    fn append(&mut self, other: &PlacedBucket) {
        self.chars = self.chars.saturating_add(other.chars);
        self.txt.extend_from_slice(&other.txt);
    }
}

/// Text of one content stream, split into placed and non-placed buckets.
#[derive(Default)]
pub(super) struct PlacedScan {
    pub(super) placed: PlacedBucket,
    other: PlacedBucket,
}

/// State of one walk over the forms that a page paints.
#[derive(Default)]
struct OutsideWalk {
    decodes: u32,
    visited: HashSet<(ObjectRef, bool)>,
    other: PlacedBucket,
}

impl<'doc> TextExtractor<'doc> {
    /// Decide the placed verdict of a page stream. The page stream and its
    /// resources are kept for the walk behind `page_outside_text`.
    pub(super) fn begin_page_placed_scan(&mut self, content_stream: &[u8]) {
        self.placed_pdf_keep = Self::placed_pdf_text_dominates(content_stream);
        self.page_content.clear();
        self.page_content.extend_from_slice(content_stream);
        self.page_resources = self.resources.clone();
        self.page_outside_text = None;
    }

    /// Text the page paints outside any `/PlacedPDF` scope, in its own stream and
    /// in every form it paints at any depth. The walk reads the streams, not the
    /// paint order, so a form is judged against a body painted before or after it.
    /// Built on the first call for a page, because only a form with its own placed
    /// scope needs it. ~keep
    pub(super) fn page_outside_text(&mut self) -> &PlacedBucket {
        if self.page_outside_text.is_none() {
            let mut walk = OutsideWalk::default();
            Self::collect_outside_text(
                self.document,
                &self.page_content,
                self.page_resources.as_ref(),
                false,
                0,
                &mut walk,
            );
            self.page_outside_text = Some(walk.other);
        }
        self.page_outside_text.get_or_insert_with(PlacedBucket::default)
    }

    /// Add the non-placed text of `stream` and of the forms it paints to `walk`.
    /// `placed` says whether the stream is painted inside a placed scope. The walk
    /// keeps the depth and decode limits of `process_xobject` and reads each form
    /// once per placed state, which also stops a cycle.
    fn collect_outside_text(
        doc: Option<&crate::document::PdfDocument>,
        stream: &[u8],
        resources: Option<&Object>,
        placed: bool,
        depth: u32,
        walk: &mut OutsideWalk,
    ) {
        let mut forms: Vec<(String, bool)> = Vec::new();
        let Some(scan) = Self::scan_text_buckets(stream, placed, |name, inside| forms.push((name.to_owned(), inside)))
        else {
            return;
        };
        walk.other.append(&scan.other);
        let (Some(doc), Some(resources)) = (doc, resources) else {
            return;
        };
        if forms.is_empty() || depth >= Self::MAX_XOBJECT_DEPTH {
            return;
        }
        let Ok(Some(xobjects)) = Self::xobject_dict_of(doc, resources) else {
            return;
        };
        let Some(xobjects) = xobjects.as_dict() else {
            return;
        };
        for (name, inside) in forms {
            if walk.decodes >= Self::MAX_XOBJECT_DECODES {
                return;
            }
            let Some(form_ref) = xobjects.get(&name).and_then(Object::as_reference) else {
                continue;
            };
            if !walk.visited.insert((form_ref, inside)) || !doc.is_form_xobject(form_ref) {
                continue;
            }
            let Ok(form) = doc.load_object(form_ref) else {
                continue;
            };
            let Some(dict) = form.as_dict() else {
                continue;
            };
            if dict.get("Subtype").and_then(|s| s.as_name()) != Some("Form") {
                continue;
            }
            walk.decodes += 1;
            let Ok(data) = Self::decode_form_stream(doc, &form, form_ref) else {
                continue;
            };
            let own_resources = dict.get("Resources").cloned();
            let form_resources = own_resources.as_ref().unwrap_or(resources);
            Self::collect_outside_text(Some(doc), &data, Some(form_resources), inside, depth + 1, walk);
        }
    }

    /// Update the artifact state based on the marked content stack.
    ///
    /// This method computes whether we're currently inside an artifact region
    /// by checking if any ancestor in the marked_content_stack has is_artifact=true.
    /// Per PDF Spec Section 14.6, artifact content should be excluded from text extraction.
    ///
    /// # Performance
    ///
    /// This is O(n) where n is the depth of the marked content stack (typically 1-5).
    /// Called each time a marked content boundary is crossed (BMC/BDC/EMC).
    pub(super) fn update_artifact_state(&mut self) {
        self.inside_artifact = self.marked_content_stack.iter().any(|ctx| ctx.is_artifact);
    }

    /// Update the excluded-layer state based on the marked content stack.
    ///
    /// True if any ancestor in the stack is an excluded OCG layer.
    /// Called each time a marked content boundary is crossed (BMC/BDC/EMC).
    pub(super) fn update_layer_state(&mut self) {
        self.inside_excluded_layer = self.marked_content_stack.iter().any(|ctx| ctx.is_excluded_layer);
        self.inside_placed_pdf = self.marked_content_stack.iter().any(|ctx| ctx.is_placed_pdf);
    }

    /// Whether content emission should be suppressed.
    ///
    /// Returns true when the current graphics/marked-content state means
    /// extracted text should be discarded. Currently checks:
    /// - Inside an excluded OCG layer (`inside_excluded_layer`)
    /// - Inside an excluded ink / separation color space (`inside_excluded_ink`)
    /// - Inside an InDesign `/PlacedPDF` figure region (`inside_placed_pdf`)
    ///
    /// Note: artifact filtering is handled separately via span metadata and
    /// downstream filtering, so `inside_artifact` is intentionally not checked here.
    pub(super) fn is_content_suppressed(&self) -> bool {
        self.inside_excluded_layer || self.inside_excluded_ink || (self.inside_placed_pdf && !self.placed_pdf_keep)
    }

    /// Decide whether `/PlacedPDF` text should be kept (not suppressed) for a page.
    ///
    /// Cheap read-only pre-scan of the page content stream. Text-show operands
    /// (`Tj`/`TJ`/`'`/`"`) are bucketed by whether they are emitted INSIDE a
    /// `/PlacedPDF` marked-content scope or OUTSIDE it, and the placed bucket is
    /// KEPT (not suppressed) unless it looks like a decorative/duplicate overlay:
    ///
    /// 1. Too little placed text (`< MIN_PLACED_CHARS`) -> suppress. A stray
    ///    placed logo or figure caption is not the page body.
    /// 2. Placed text clearly dominates the page (non-placed text is a small
    ///    minority, ~3:1) -> keep. The publisher placed the whole article body as
    ///    one `/PlacedPDF` and left only a running header outside (MATEC Web of
    ///    Conferences).
    /// 3. Placed text is substantial but the non-placed text is comparable or
    ///    larger -> keep ONLY when the placed words are mostly NOT already present
    ///    outside. A placed region that mostly repeats the surrounding text is a
    ///    draft galley / overlay copy and stays suppressed (PMC8100493, the de-dup
    ///    win); one carrying mostly-unique words is the page's real placed content
    ///    (e.g. an InDesign figure that holds the page's labels and body text, as
    ///    on placed floor-plan / marketing spreads) and must be kept.
    ///
    /// The scan reads the page stream only. `process_xobject` scans each form's
    /// own stream too, and judges the form's placed text against all the text the
    /// page paints outside any placed scope (`page_outside_text`). Without that
    /// text, a duplicate overlay in a form would have nothing to repeat and would
    /// be kept. ~keep
    pub(super) fn placed_pdf_text_dominates(content_stream: &[u8]) -> bool {
        Self::scan_placed_text(content_stream).is_some_and(|scan| Self::placed_scan_dominates(&scan.placed, &scan.other))
    }

    /// Whether a content stream mentions the `/PlacedPDF` tag at all.
    pub(super) fn has_placed_tag(content_stream: &[u8]) -> bool {
        content_stream.windows(b"PlacedPDF".len()).any(|w| w == b"PlacedPDF")
    }

    /// Bucket the text-show operands of one content stream by `/PlacedPDF` scope.
    /// Returns `None` when the stream carries no placed tag or does not parse.
    pub(super) fn scan_placed_text(content_stream: &[u8]) -> Option<PlacedScan> {
        // Gate: only streams that actually carry the InDesign tag pay for a parse. ~keep
        if !Self::has_placed_tag(content_stream) {
            return None;
        }
        Self::scan_text_buckets(content_stream, false, |_, _| {})
    }

    /// The bucketing of `scan_placed_text`, without the tag gate. `placed` says
    /// whether the stream starts inside a placed scope. `on_form` gets the name of
    /// each form the stream paints, and whether it is painted inside a placed scope.
    fn scan_text_buckets(
        content_stream: &[u8],
        placed: bool,
        mut on_form: impl FnMut(&str, bool),
    ) -> Option<PlacedScan> {
        let operators = parse_content_stream(content_stream).ok()?;

        let mut placed_stack: Vec<bool> = Vec::new();
        let mut scan = PlacedScan::default();
        let inside = |stack: &[bool]| placed || stack.iter().any(|&p| p);
        for op in &operators {
            match op {
                Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. } => {
                    placed_stack.push(tag == "PlacedPDF");
                }
                Operator::EndMarkedContent => {
                    placed_stack.pop();
                }
                Operator::Tj { text } | Operator::Quote { text } | Operator::DoubleQuote { text, .. } => {
                    let bucket = if inside(&placed_stack) {
                        &mut scan.placed
                    } else {
                        &mut scan.other
                    };
                    bucket.chars += text.len();
                    bucket.txt.extend_from_slice(text);
                    bucket.txt.push(b' ');
                }
                Operator::TJ { array } => {
                    let bucket = if inside(&placed_stack) {
                        &mut scan.placed
                    } else {
                        &mut scan.other
                    };
                    for e in array {
                        if let TextElement::String(s) = e {
                            bucket.chars += s.len();
                            bucket.txt.extend_from_slice(s);
                        }
                    }
                    bucket.txt.push(b' ');
                }
                Operator::Do { name } => on_form(name, inside(&placed_stack)),
                _ => {}
            }
        }
        Some(scan)
    }

    /// Apply the keep/suppress gates to placed text against the non-placed text
    /// it is judged with.
    pub(super) fn placed_scan_dominates(placed: &PlacedBucket, other: &PlacedBucket) -> bool {
        // A substantial placed body; below this a placed region is treated as a
        // decorative figure, not the page's logical content. ~keep
        const MIN_PLACED_CHARS: usize = 800;
        // Keep placed text whose words are mostly unique; suppress it once a
        // majority is also present in the non-placed text (a duplicate overlay). ~keep
        const MAX_DUP_FRACTION: f64 = 0.5;

        let placed_chars = placed.chars;
        let other_chars = other.chars;
        if placed_chars < MIN_PLACED_CHARS {
            return false;
        }
        if other_chars.saturating_mul(3) < placed_chars {
            return true;
        }
        // Gate 3: placed text is substantial but the outside text is comparable
        // or larger. Keep it unless a majority of the placed words also appear
        // outside (a duplicate overlay). Tokenising here (behind gates 1 and 2)
        // keeps the common single-column path allocation-free. ~keep
        Self::text_duplication_fraction(&placed.txt, &other.txt) < MAX_DUP_FRACTION
    }

    /// Fraction of alphanumeric word tokens in `a` (counting repeats) that also
    /// occur anywhere in `b`. Words are lowercased runs of >= 2 alphanumeric
    /// bytes; punctuation and single characters are ignored. Returns 0.0 when `a`
    /// has no such tokens (nothing to be a duplicate of).
    fn text_duplication_fraction(a: &[u8], b: &[u8]) -> f64 {
        fn tokens(bytes: &[u8]) -> Vec<Vec<u8>> {
            let mut out = Vec::new();
            let mut cur = Vec::new();
            for &c in bytes {
                if c.is_ascii_alphanumeric() {
                    cur.push(c.to_ascii_lowercase());
                } else if !cur.is_empty() {
                    if cur.len() >= 2 {
                        out.push(std::mem::take(&mut cur));
                    } else {
                        cur.clear();
                    }
                }
            }
            if cur.len() >= 2 {
                out.push(cur);
            }
            out
        }
        let a_tokens = tokens(a);
        if a_tokens.is_empty() {
            return 0.0;
        }
        let b_set: std::collections::HashSet<Vec<u8>> = tokens(b).into_iter().collect();
        let shared = a_tokens.iter().filter(|t| b_set.contains(*t)).count();
        shared as f64 / a_tokens.len() as f64
    }

    /// Parse artifact type and subtype from artifact properties dictionary.
    ///
    /// Per PDF Spec Section 14.8.2.2, artifacts have optional /Type and /Subtype entries:
    /// - /Type: Pagination, Layout, Page, or Background
    /// - /Subtype: For Pagination artifacts: Header, Footer, Watermark, etc.
    ///
    /// # Arguments
    ///
    /// * `props_dict` - The properties dictionary from BDC operator
    ///
    /// # Returns
    ///
    /// The classified artifact type, or None if no type is specified
    pub(super) fn parse_artifact_type(props_dict: &HashMap<String, Object>) -> Option<ArtifactType> {
        let artifact_type_name = props_dict
            .get("Type")
            .and_then(|obj| obj.as_name())
            .map(|s| s.to_lowercase());

        let subtype_name = props_dict
            .get("Subtype")
            .and_then(|obj| obj.as_name())
            .map(|s| s.to_lowercase());

        match artifact_type_name.as_deref() {
            Some("pagination") => {
                let subtype = match subtype_name.as_deref() {
                    Some("header") => PaginationSubtype::Header,
                    Some("footer") => PaginationSubtype::Footer,
                    Some("watermark") => PaginationSubtype::Watermark,
                    Some("pagenumber") | Some("page") => PaginationSubtype::PageNumber,
                    _ => PaginationSubtype::Other,
                };
                Some(ArtifactType::Pagination(subtype))
            }
            Some("layout") => Some(ArtifactType::Layout),
            Some("page") => Some(ArtifactType::Page),
            Some("background") => Some(ArtifactType::Background),
            None => {
                // No /Type specified - check if /Subtype alone indicates pagination
                // Some PDFs use /Subtype without /Type ~keep
                match subtype_name.as_deref() {
                    Some("header") => Some(ArtifactType::Pagination(PaginationSubtype::Header)),
                    Some("footer") => Some(ArtifactType::Pagination(PaginationSubtype::Footer)),
                    Some("watermark") => Some(ArtifactType::Pagination(PaginationSubtype::Watermark)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Decode a PDF text string (handles UTF-16BE/LE with BOM and PDFDocEncoding).
    ///
    /// Thin delegate to [`crate::optional_content::decode_pdf_text_string`] — that
    /// module owns the canonical implementation shared with the rendering path
    /// (UTF-16BE/LE with BOM, PDFDocEncoding fallback per ISO 32000-1:2008 §7.9.2).
    pub(super) fn decode_pdf_text_string(bytes: &[u8]) -> String {
        crate::optional_content::decode_pdf_text_string(bytes)
    }

    /// Resolve BDC properties: can be an inline dictionary or a name referencing /Properties resource.
    ///
    /// Thin delegate to [`crate::optional_content::resolve_bdc_properties`].
    /// Passing `self.document` as `Option` lets the inline-dict fast path work
    /// even on a freshly-constructed extractor with no document attached (used
    /// by unit tests).
    pub(super) fn resolve_bdc_properties(
        &self,
        properties: &Object,
    ) -> Option<std::collections::HashMap<String, Object>> {
        crate::optional_content::resolve_bdc_properties(properties, self.resources.as_ref(), self.document)
    }

    /// Resolve a named color space from the /Resources /ColorSpace dictionary.
    ///
    /// PDF content streams reference color spaces by name (e.g. `cs /CS1`).
    /// Device color spaces like "DeviceRGB" are built-in, but Separation and
    /// DeviceN color spaces live in the page resources:
    ///
    /// ```text
    /// /Resources << /ColorSpace << /CS1 [/Separation /PANTONE_Red /DeviceCMYK ...] >> >>
    /// ```
    ///
    /// Returns the resolved color space array if the name refers to a resource entry.
    fn resolve_color_space(&self, name: &str) -> Option<Vec<Object>> {
        let resources = self.resources.as_ref()?;
        let res_dict = if let Some(res_ref) = resources.as_reference() {
            self.document?.load_object(res_ref).ok()?
        } else {
            resources.clone()
        };
        let res_dict = res_dict.as_dict()?;
        let cs_dict_obj = res_dict.get("ColorSpace")?;
        let cs_dict = if let Some(r) = cs_dict_obj.as_reference() {
            self.document?.load_object(r).ok()?
        } else {
            cs_dict_obj.clone()
        };
        let cs_dict = cs_dict.as_dict()?;
        let cs_obj = cs_dict.get(name)?;
        let resolved = if let Some(r) = cs_obj.as_reference() {
            self.document?.load_object(r).ok()?
        } else {
            cs_obj.clone()
        };
        resolved.as_array().cloned()
    }

    /// Check if a color space name refers to an excluded ink.
    ///
    /// Resolves the color space from resources and checks:
    /// - `[/Separation /InkName /AlternateCS /TintTransform]` — single ink name
    /// - `[/DeviceN [/Ink1 /Ink2 ...] /AlternateCS /TintTransform]` — multiple ink names
    ///
    /// Returns true if any ink name in the color space matches `excluded_inks`.
    ///
    /// **Note:** For DeviceN, this is all-or-nothing — if any ink matches, the
    /// entire color space is treated as excluded. Tint values are not evaluated.
    pub(super) fn is_excluded_ink_color_space(&self, name: &str) -> bool {
        if self.excluded_inks.is_empty() {
            return false;
        }
        if let Some(cs_array) = self.resolve_color_space(name)
            && cs_array.len() >= 2
            && let Some(cs_type) = cs_array[0].as_name()
        {
            // §8.6.6.2 / §8.6.6.3: the colorant slot (Separation's
            // ink-name, DeviceN's names array) can be an indirect
            // reference. Some subsetters share the names list across
            // multiple DeviceN spaces, emitting
            // `[/DeviceN 4 0 R /DeviceCMYK <attrs>]` where `4 0 R`
            // points to the actual names list. Resolve before
            // pattern-matching. ~keep
            let deref = |obj: &Object| -> Object {
                match (obj.as_reference(), self.document) {
                    (Some(r), Some(d)) => d.load_object(r).unwrap_or_else(|_| obj.clone()),
                    _ => obj.clone(),
                }
            };
            match cs_type {
                "Separation" => {
                    let name_obj = deref(&cs_array[1]);
                    if let Some(ink_name) = name_obj.as_name() {
                        return self.excluded_inks.contains(ink_name);
                    }
                }
                "DeviceN" => {
                    let names_obj = deref(&cs_array[1]);
                    if let Some(ink_names) = names_obj.as_array() {
                        return ink_names
                            .iter()
                            .any(|obj| obj.as_name().map(|n| self.excluded_inks.contains(n)).unwrap_or(false));
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check whether a BDC properties dict represents an excluded OCG or OCMD.
    ///
    /// Thin delegate to [`crate::optional_content::check_ocg_excluded`] — that
    /// module is the single source of truth for OCG/OCMD evaluation, including
    /// OCMD `/P` visibility-policy handling.
    pub(super) fn check_ocg_excluded(&self, props_dict: &std::collections::HashMap<String, Object>) -> bool {
        let doc = match self.document {
            Some(d) => d,
            None => return false,
        };
        crate::optional_content::check_ocg_excluded(props_dict, doc, &self.excluded_layers)
    }

    /// Get current ActualText from marked content stack (PDF Spec Section 14.9.4).
    ///
    /// Searches from the innermost marked content context outward, returning
    /// the first ActualText found. If no ActualText is defined, returns None.
    ///
    /// ActualText provides the exact text representation for content that's
    /// represented non-standardly, such as ligatures (fi, fl, ffi, ffl) or
    /// decorated glyphs.
    #[cfg(test)]
    pub(super) fn get_current_actual_text(&self) -> Option<String> {
        self.marked_content_stack
            .iter()
            .rev()
            .find_map(|ctx| ctx.actual_text.clone())
    }

    /// Return the innermost active `/ActualText`, alongside a flag
    /// indicating whether it has ALREADY been emitted in the current
    /// MC scope.
    ///
    /// Per ISO 32000-1:2008 §14.9.4 the `/ActualText` of a marked-
    /// content sequence replaces the ENTIRE sequence — even if the
    /// sequence contains multiple `Tj` / `TJ` operators the replacement
    /// is emitted ONCE.
    ///
    /// Returns `(text, already_emitted)`:
    /// - `(Some(text), false)` on the FIRST show-text inside a scope
    ///   that carries `/ActualText` — the caller should emit `text`
    ///   AND mark the scope's `actual_text_emitted` via
    ///   [`Self::mark_actual_text_emitted`].
    /// - `(Some(text), true)` on subsequent show-text operators inside
    ///   the same scope — the caller must suppress emission entirely
    ///   (no raw glyphs, no replacement). The text matrix advance
    ///   still runs so positioning stays consistent.
    /// - `(None, _)` when no `/ActualText` is active.
    pub(super) fn peek_current_actual_text(&self) -> (Option<String>, bool) {
        for ctx in self.marked_content_stack.iter().rev() {
            if let Some(ref text) = ctx.actual_text {
                return (Some(text.clone()), ctx.actual_text_emitted);
            }
        }
        (None, false)
    }

    /// Mark the innermost scope's `/ActualText` as emitted. See
    /// [`Self::peek_current_actual_text`].
    pub(super) fn mark_actual_text_emitted(&mut self) {
        for ctx in self.marked_content_stack.iter_mut().rev() {
            if ctx.actual_text.is_some() {
                ctx.actual_text_emitted = true;
                return;
            }
        }
    }

    /// Add a font to the extractor.
    ///
    /// Fonts must be added before processing content streams that reference them.
    ///
    /// # Arguments
    ///
    /// * `name` - The font resource name (e.g., "F1", "TT1")
    /// * `font` - The font information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::extractors::TextExtractor;
    /// # use xberg_native_pdf::fonts::FontInfo;
    /// # fn example(font: FontInfo) {
    /// let mut extractor = TextExtractor::new();
    /// extractor.add_font("F1".to_string(), font);
    /// # }
    /// ```
    pub fn add_font(&mut self, name: String, font: FontInfo) {
        self.fonts.insert(name, Arc::new(font));
    }

    /// Add a pre-shared font (Arc-wrapped) to the extractor. Avoids deep cloning.
    pub(crate) fn add_font_shared(&mut self, name: String, font: Arc<FontInfo>) {
        self.fonts.insert(name, font);
    }

    /// Return the current font set for caching purposes.
    pub fn get_font_set(&self) -> Vec<(String, Arc<FontInfo>)> {
        self.fonts.iter().map(|(k, v)| (k.clone(), Arc::clone(v))).collect()
    }

    /// Share TrueType cmap tables between fonts with matching base font names.
    /// When a CIDFontType2 Identity-H font has no truetype_cmap, borrow from
    /// another font on the same page with the same base font name (ignoring subset prefix).
    ///
    /// Returns the number of fonts actually mutated (donations applied). A font
    /// that already carries a cmap — e.g. one served from `PdfDocument`'s
    /// donated-set cache (#1746) — is skipped and does not count, so callers
    /// can use the return value to observe when a cache hit made this call a
    /// no-op instead of redoing the `Arc::make_mut` clone. ~keep
    pub fn share_truetype_cmaps(&mut self) -> usize {
        let best_cmaps = crate::fonts::unicode_decode::best_truetype_cmaps(self.fonts.values().map(|f| &**f));

        if best_cmaps.is_empty() {
            return 0;
        }

        let mut applied = 0;
        for font_arc in self.fonts.values_mut() {
            if font_arc.truetype_cmap().is_some() {
                continue;
            }
            if font_arc.subtype != "Type0" {
                continue;
            }
            let is_identity = matches!(&font_arc.encoding, crate::fonts::Encoding::Identity)
                || matches!(&font_arc.encoding, crate::fonts::Encoding::Standard(n) if n.contains("Identity"));
            if !is_identity {
                continue;
            }

            let stripped = strip_subset_prefix(&font_arc.base_font);
            if let Some((donor_cmap, _)) = best_cmaps.get(stripped) {
                tracing::info!(target: LOG_TARGET,
                    "Sharing TrueType cmap ({} entries) to '{}' (Identity-H, no embedded font)",
                    donor_cmap.len(),
                    font_arc.base_font
                );
                // Use Arc::make_mut + set_truetype_cmap for copy-on-write sharing ~keep
                Arc::make_mut(font_arc).set_truetype_cmap(Some(donor_cmap.clone()));
                applied += 1;
            }
        }
        applied
    }
}
