//! Marked-content scopes, artifacts, optional content, and font registration.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

/// Text of one hold group, or of the page text that is not held, as the
/// duplicate measure of `placed_text_kept` reads it.
#[derive(Default, Clone)]
struct HeldText {
    chars: usize,
    txt: Vec<u8>,
    /// Letters and digits of each line in paint order, lowercase. Spaces and
    /// punctuation go.
    lines: Vec<String>,
    /// Vertical position of the last span.
    last_y: Option<f32>,
}

impl HeldText {
    /// Add one span. Every span ends a word, as a text run does.
    fn add_span(&mut self, span: &TextSpan) {
        let text = &span.text;
        self.chars += text.chars().count();
        self.txt.extend_from_slice(text.as_bytes());
        self.txt.push(b' ');

        let letters: String = text
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        // Half a font size: raised text stays on its line and the next line does not. ~keep
        let y = span.bbox.y;
        let same_line = self.last_y.is_some_and(|last| (last - y).abs() <= span.font_size / 2.0);
        self.last_y = Some(y);
        match self.lines.last_mut() {
            Some(line) if same_line => line.push_str(&letters),
            _ => self.lines.push(letters),
        }
    }
}

/// Push `item` with its hold group. `holds` stays empty until an item is held.
pub(super) fn push_held<T>(items: &mut Vec<T>, holds: &mut Vec<u32>, item: T, hold: u32) {
    if hold != 0 || !holds.is_empty() {
        holds.resize(items.len(), 0);
        holds.push(hold);
    }
    items.push(item);
}

/// Keep the items from `start` on that pass `keep`, with their hold groups.
pub(super) fn retain_held_from<T>(items: &mut Vec<T>, holds: &mut Vec<u32>, start: usize, keep: impl Fn(&T) -> bool) {
    let mask: Vec<bool> = items[start..].iter().map(keep).collect();
    let len = items.len();
    let retain = |index: usize| index < start || mask[index - start];
    let mut index = 0;
    items.retain(|_| {
        index += 1;
        retain(index - 1)
    });
    if !holds.is_empty() {
        holds.resize(len, 0);
        let mut index = 0;
        holds.retain(|_| {
            index += 1;
            retain(index - 1)
        });
    }
}

/// Drop each item whose hold group the page rejected. Kept groups stay in paint
/// position.
fn apply_verdicts<T>(items: &mut Vec<T>, holds: &mut Vec<u32>, verdicts: &[bool]) {
    if holds.is_empty() {
        return;
    }
    holds.resize(items.len(), 0);
    let mut index = 0;
    items.retain(|_| {
        let hold = holds[index];
        index += 1;
        hold.checked_sub(1)
            .and_then(|group| verdicts.get(group as usize))
            .is_none_or(|&keep| keep)
    });
    holds.clear();
}

impl<'doc> TextExtractor<'doc> {
    /// Start the `/PlacedPDF` state of a char run: the page verdict from its own
    /// stream, and group numbers from the start, as in its verdict run.
    pub(super) fn begin_placed_page(&mut self, content_stream: &[u8]) {
        self.placed_pdf_keep = Self::placed_pdf_text_dominates(content_stream);
        self.placed_hold_groups = 0;
    }

    /// Hold group of text emitted now (0 when it is not held), or `None` when
    /// the current state suppresses it.
    pub(super) fn emit_hold(&self) -> Option<u32> {
        if self.is_content_suppressed() {
            None
        } else if self.inside_placed_pdf && !self.placed_pdf_keep {
            Some(self.placed_hold)
        } else {
            Some(0)
        }
    }

    /// Emit a span, unless the current state suppresses it.
    pub(super) fn push_span(&mut self, span: TextSpan) {
        if let Some(hold) = self.emit_hold() {
            push_held(&mut self.spans, &mut self.span_holds, span, hold);
        }
    }

    /// Emit a character, unless the current state suppresses it.
    pub(super) fn push_char(&mut self, ch: TextChar) {
        if let Some(hold) = self.emit_hold() {
            push_held(&mut self.chars, &mut self.char_holds, ch, hold);
        }
    }

    /// One verdict for each hold group of the page, read from the span text after
    /// clipping. A verdict run opens the groups of its char run: both walk the
    /// same forms in the same order. ~keep
    fn span_verdicts(&self) -> Vec<bool> {
        let mut other = HeldText::default();
        let mut held = vec![HeldText::default(); self.placed_hold_groups as usize];
        for (index, span) in self.spans.iter().enumerate() {
            let hold = self.span_holds.get(index).copied().unwrap_or(0);
            match hold.checked_sub(1).and_then(|group| held.get_mut(group as usize)) {
                Some(group) => group.add_span(span),
                None => other.add_span(span),
            }
        }
        held.iter()
            .map(|group| {
                Self::placed_text_kept(group.chars, other.chars, || {
                    let words = Self::text_duplication_fraction(&group.txt, &other.txt);
                    words.max(Self::copied_line_fraction(&group.lines, &other.lines))
                })
            })
            .collect()
    }

    /// Judge each hold group of a span run once all its text is known. Paint
    /// order does not matter. ~keep
    pub(super) fn settle_span_holds(&mut self) {
        if self.span_holds.is_empty() {
            return;
        }
        let verdicts = self.span_verdicts();
        apply_verdicts(&mut self.spans, &mut self.span_holds, &verdicts);
    }

    /// Apply the span verdicts to a char run. Only a page that holds a group pays
    /// for the span run, which sees the page, the fonts and the filters of the char
    /// run, so `extract_chars` and `extract_chars_filtered` each get the verdict of
    /// their own filters. ~keep
    pub(super) fn settle_char_holds(&mut self, content_stream: &[u8]) -> Result<()> {
        if self.char_holds.is_empty() {
            return Ok(());
        }
        let mut pass = TextExtractor::with_config(self.config.clone());
        pass.merging_config = self.merging_config.clone();
        pass.document = self.document;
        pass.resources = self.resources.clone();
        pass.fonts = self.fonts.clone();
        pass.excluded_layers = self.excluded_layers.clone();
        pass.excluded_inks = self.excluded_inks.clone();
        pass.mcid_scope_stack = self.mcid_scope_stack.clone();
        pass.extract_spans = true;
        pass.verdict_run = true;
        pass.run_content_stream(content_stream)?;
        pass.flush_tj_span_buffer()?;
        let verdicts = pass.span_verdicts();
        apply_verdicts(&mut self.chars, &mut self.char_holds, &verdicts);
        Ok(())
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

    /// Whether a content stream contains a marked-content tag decoded as `/PlacedPDF`.
    pub(super) fn has_placed_tag(content_stream: &[u8]) -> bool {
        if !content_stream.windows(b"PlacedPDF".len()).any(|w| w == b"PlacedPDF") && !content_stream.contains(&b'#') {
            return false;
        }
        // Full parsing can return a capped prefix while text execution still reaches
        // a later tag after skipping graphics. Use the same scanner as execution. ~keep
        let mut found = false;
        let result = parse_and_execute_text_only(content_stream, |operator| {
            found |= matches!(
                operator,
                Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. }
                    if tag == "PlacedPDF"
            );
            Ok(())
        });
        // A failed scan cannot prove the form's spans independent of the page. ~keep
        result.is_err() || found
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
        self.inside_excluded_layer
            || self.inside_excluded_ink
            || (self.inside_placed_pdf && !self.placed_pdf_keep && self.placed_hold == 0)
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
    /// The scan reads the page stream only. A form with its own `/PlacedPDF` tag
    /// holds its placed text instead, judged by `span_verdicts`. ~keep
    pub(super) fn placed_pdf_text_dominates(content_stream: &[u8]) -> bool {
        // Escaped names must reach the parser, which decides whether the tag is PlacedPDF. ~keep
        if !content_stream.windows(b"PlacedPDF".len()).any(|w| w == b"PlacedPDF") && !content_stream.contains(&b'#') {
            return false;
        }
        let Ok(operators) = parse_content_stream(content_stream) else {
            return false;
        };

        let mut placed_stack: Vec<bool> = Vec::new();
        let mut placed_chars: usize = 0;
        let mut other_chars: usize = 0;
        let mut placed_txt: Vec<u8> = Vec::new();
        let mut other_txt: Vec<u8> = Vec::new();
        let inside = |stack: &[bool]| stack.iter().any(|&p| p);
        for op in &operators {
            match op {
                Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. } => {
                    placed_stack.push(tag == "PlacedPDF");
                }
                Operator::EndMarkedContent => {
                    placed_stack.pop();
                }
                Operator::Tj { text } | Operator::Quote { text } => {
                    let (chars, txt) = if inside(&placed_stack) {
                        (&mut placed_chars, &mut placed_txt)
                    } else {
                        (&mut other_chars, &mut other_txt)
                    };
                    *chars += text.len();
                    txt.extend_from_slice(text);
                    txt.push(b' ');
                }
                Operator::DoubleQuote { text, .. } => {
                    let (chars, txt) = if inside(&placed_stack) {
                        (&mut placed_chars, &mut placed_txt)
                    } else {
                        (&mut other_chars, &mut other_txt)
                    };
                    *chars += text.len();
                    txt.extend_from_slice(text);
                    txt.push(b' ');
                }
                Operator::TJ { array } => {
                    let (chars, txt) = if inside(&placed_stack) {
                        (&mut placed_chars, &mut placed_txt)
                    } else {
                        (&mut other_chars, &mut other_txt)
                    };
                    for e in array {
                        if let TextElement::String(s) = e {
                            *chars += s.len();
                            txt.extend_from_slice(s);
                        }
                    }
                    txt.push(b' ');
                }
                _ => {}
            }
        }
        Self::placed_text_kept(placed_chars, other_chars, || {
            Self::text_duplication_fraction(&placed_txt, &other_txt)
        })
    }

    /// The keep/suppress gates of `placed_pdf_text_dominates`, for placed text
    /// judged against the non-placed text beside it.
    fn placed_text_kept(placed_chars: usize, other_chars: usize, duplication: impl FnOnce() -> f64) -> bool {
        // A substantial placed body; below this a placed region is treated as a
        // decorative figure, not the page's logical content. ~keep
        const MIN_PLACED_CHARS: usize = 800;
        // Keep placed text that is mostly unique; suppress it once half or more
        // of it is also present in the non-placed text (a duplicate overlay). ~keep
        const MAX_DUP_FRACTION: f64 = 0.5;

        if placed_chars < MIN_PLACED_CHARS {
            return false;
        }
        if other_chars.saturating_mul(3) < placed_chars {
            return true;
        }
        // Gate 3: placed text is substantial but the outside text is comparable
        // or larger. Keep it unless half or more of the placed text also appears
        // outside (a duplicate overlay). Tokenising here (behind gates 1 and 2)
        // keeps the common single-column path allocation-free. ~keep
        duplication() < MAX_DUP_FRACTION
    }

    /// Fraction of the letters of `a` in lines that `b` shows as well. Each line of
    /// `b` answers one line of `a`. A line compares by its letters and digits, so a
    /// copy reads the same wherever a `TJ` offset or a text run cuts its words. A row
    /// on one baseline with its own cells does not match. A table painted by column
    /// has a line for each cell, so its labels can match, and rows that differ only
    /// in punctuation or spacing match. ~keep
    fn copied_line_fraction(a: &[String], b: &[String]) -> f64 {
        // A shorter line does not count, so a short grid row cannot match. A longer
        // row of a skewed grid can still match by chance. ~keep
        const MIN_LINE_LETTERS: usize = 8;
        let mut unmatched: HashMap<&str, usize> = HashMap::new();
        for line in b {
            *unmatched.entry(line).or_default() += 1;
        }
        let (mut copied, mut total) = (0, 0);
        for line in a {
            let letters = line.chars().count();
            total += letters;
            if letters >= MIN_LINE_LETTERS
                && let Some(count) = unmatched.get_mut(line.as_str())
                && *count > 0
            {
                *count -= 1;
                copied += letters;
            }
        }
        if total == 0 {
            return 0.0;
        }
        copied as f64 / total as f64
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

    pub(crate) fn has_fonts(&self) -> bool {
        !self.fonts.is_empty()
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
