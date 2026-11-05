//! Reading-order selection, MCID ordering, and RTL line merging.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Check if decoded content stream data may contain text.
    ///
    /// Returns true if the stream contains either:
    /// - A BT (Begin Text) operator (text is directly in the page stream)
    /// - A Do operator (Form XObject invocation that may contain text)
    ///
    /// Per §9.4.3, text-showing operators shall only appear within BT...ET text
    /// objects. However, a page may contain text only inside Form XObjects
    /// referenced via `Do` operators, so we must also check for those.
    pub(crate) fn may_contain_text(data: &[u8]) -> bool {
        // SIMD-accelerated pre-check using memchr to find candidate positions
        // for BT (Begin Text) and Do (XObject invocation) operators.
        // ~50x faster than byte-by-byte scanning for large graphics-heavy pages. ~keep
        fn is_boundary(b: u8) -> bool {
            b.is_ascii_whitespace() || matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
        }

        let len = data.len();
        let mut offset = 0;
        while offset + 1 < len {
            match memchr::memchr2(b'B', b'D', &data[offset..]) {
                None => return false,
                Some(pos) => {
                    let i = offset + pos;
                    if i + 1 >= len {
                        return false;
                    }
                    if data[i] == b'B' && data[i + 1] == b'T' {
                        let before_ok = i == 0 || is_boundary(data[i - 1]);
                        let after_ok = i + 2 >= len || is_boundary(data[i + 2]);
                        if before_ok && after_ok {
                            return true;
                        }
                    }
                    if data[i] == b'D' && data[i + 1] == b'o' {
                        let before_ok = i == 0 || is_boundary(data[i - 1]);
                        let after_ok = i + 2 >= len || is_boundary(data[i + 2]);
                        if before_ok && after_ok {
                            return true;
                        }
                    }
                    offset = i + 1;
                }
            }
        }
        false
    }

    /// Check if a page definitely cannot produce any text based on its resources.
    ///
    /// Returns `true` if the page has no `/Font` resources and no Form XObjects
    /// (which could contain nested text). This allows skipping content stream
    /// decompression and parsing entirely for image-only/scanned pages.
    ///
    /// Returns `false` (conservative) if resources can't be inspected.
    pub(super) fn page_cannot_have_text(&self, page_dict: &HashMap<String, Object>) -> bool {
        let resources = match page_dict.get("Resources") {
            Some(r) => {
                if let Some(ref_obj) = r.as_reference() {
                    match self.load_object(ref_obj) {
                        Ok(obj) => obj,
                        Err(_) => return false, // Can't resolve — be conservative ~keep
                    }
                } else {
                    r.clone()
                }
            }
            None => return true,
        };

        let res_dict = match resources.as_dict() {
            Some(d) => d,
            None => return false,
        };

        if let Some(font_obj) = res_dict.get("Font") {
            let font_dict = if let Some(ref_obj) = font_obj.as_reference() {
                self.load_object(ref_obj).ok()
            } else {
                Some(font_obj.clone())
            };
            if let Some(fd) = font_dict
                && let Some(d) = fd.as_dict()
                && !d.is_empty()
            {
                return false;
            }
        }

        // Check XObjects: if any are Form type, they could contain nested text.
        // Uses lightweight is_form_xobject() peek instead of full load_object()
        // to avoid expensive I/O for image-heavy PDFs (e.g., Deutsche: 375MB images). ~keep
        if let Some(xobj_obj) = res_dict.get("XObject") {
            let xobj_dict_obj = if let Some(ref_obj) = xobj_obj.as_reference() {
                self.load_object(ref_obj).ok()
            } else {
                Some(xobj_obj.clone())
            };
            if let Some(xobj_dict_resolved) = xobj_dict_obj
                && let Some(xobj_dict) = xobj_dict_resolved.as_dict()
            {
                for xobj_ref in xobj_dict.values() {
                    if let Some(ref_obj) = xobj_ref.as_reference() {
                        if self.is_form_xobject(ref_obj) {
                            return false;
                        }
                    } else if let Some(d) = xobj_ref.as_dict()
                        && d.get("Subtype").and_then(|s| s.as_name()) == Some("Form")
                    {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Assemble the page's text spans via the reading-order
    /// pipeline, classifying each region with the per-class
    /// detectors in [`crate::pipeline::reading_order::detectors`].
    /// Returns the assembled spans plus the detector class that
    /// fired on each region.
    ///
    /// The four detectors handle layout shapes that the plain
    /// y-then-x assembly cannot produce correctly:
    ///
    /// - **DramaticScript**: Macbeth-style speaker-tag layouts —
    ///   row-major join required.
    /// - **DenseSingleLine**: SEC DEF 14A 8pt-body interleave —
    ///   single-row regroup required.
    /// - **SubSuperBaselineReattach**: chemical-formula
    ///   subscripts — baseline reattach required.
    /// - **NarrowTrackedJustified**: stretched justified columns —
    ///   per-line median-gap threshold normalisation required.
    ///
    /// Regions that don't match any specific layout fall through to
    /// `Default` (plain y-then-x assembly within the block).
    ///
    /// Callers can use this as a pre-step before applying their own
    /// assembly logic, or rely on the classified `ReadingOrderClass`
    /// to dispatch their assembly strategy. `extract_text` consumes
    /// this implicitly through `extract_spans` + the existing
    /// `XYCutStrategy`.
    pub fn assemble_text_via_reading_order(
        &self,
        page_index: usize,
    ) -> Result<(
        Vec<crate::layout::TextSpan>,
        crate::pipeline::reading_order::ReadingOrderClass,
    )> {
        if self.is_encrypted_unreadable() {
            tracing::warn!(target: LOG_TARGET, "PDF is encrypted and could not be decrypted; returning empty text");
            return Ok((Vec::new(), crate::pipeline::reading_order::ReadingOrderClass::Default));
        }
        let spans = self.extract_spans(page_index)?;
        // Convert spans to detector input. We only need the geometric
        // signal (x/y/width/font_size), not the full TextSpan
        // semantics. ~keep
        let glyphs: Vec<crate::pipeline::reading_order::DetectorGlyph> = spans
            .iter()
            .map(|s| crate::pipeline::reading_order::DetectorGlyph {
                x: s.bbox.x,
                y: s.bbox.y,
                width: s.bbox.width,
                font_size: s.font_size,
                text_len: s.text.chars().count(),
            })
            .collect();
        // Build per-row text strings for DramaticScript detector,
        // together with the leftmost glyph of each row (for the X-
        // consistency check). Group spans by Y (within 0.5 pt),
        // concatenating their texts in the order they appear in
        // `spans` and tracking the smallest X seen per row. ~keep
        let mut rows: Vec<(f32, String, crate::pipeline::reading_order::DetectorGlyph)> = Vec::new();
        for span in &spans {
            let span_glyph = crate::pipeline::reading_order::DetectorGlyph {
                x: span.bbox.x,
                y: span.bbox.y,
                width: span.bbox.width,
                font_size: span.font_size,
                text_len: span.text.chars().count(),
            };
            let mut placed = false;
            for (y, text, first) in rows.iter_mut() {
                if (*y - span.bbox.y).abs() < 0.5 {
                    text.push(' ');
                    text.push_str(&span.text);
                    if span_glyph.x < first.x {
                        *first = span_glyph;
                    }
                    placed = true;
                    break;
                }
            }
            if !placed {
                rows.push((span.bbox.y, span.text.clone(), span_glyph));
            }
        }
        let row_texts: Vec<&str> = rows.iter().map(|(_, t, _)| t.as_str()).collect();
        let row_first_glyphs: Vec<crate::pipeline::reading_order::DetectorGlyph> =
            rows.iter().map(|(_, _, g)| *g).collect();
        let class = crate::pipeline::reading_order::classify_region(&glyphs, &row_first_glyphs, &row_texts);
        Ok((spans, class))
    }

    /// Returns `true` if the page has any text-bearing content (fonts in
    /// resources + at least one `BT`/`Do` operator in the content stream),
    /// `false` if the page is image-only or genuinely empty.
    ///
    /// Callers can route image-only pages to their own OCR pipeline
    /// instead of receiving an empty string with no signal.
    ///
    /// Conservative: returns `true` when the page resources can't be
    /// inspected (load error, encrypted-not-authenticated, etc.) so the
    /// caller still attempts extraction.
    ///
    /// # PDF spec basis
    ///
    /// §8.8 (Image XObjects): image-only pages have `/Resources` whose
    /// only `/XObject` entries are `/Subtype /Image` with no `/Font`
    /// resources.
    pub fn has_text_layer(&self, page_index: usize) -> Result<bool> {
        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;
        if self.page_cannot_have_text(page_dict) {
            return Ok(false);
        }
        // Probe content stream for text-showing operators. If we can't
        // read the content stream, be conservative and say yes (let
        // extraction try). ~keep
        match self.get_page_content_data(page_index) {
            Ok(content_data) => Ok(Self::may_contain_text(&content_data)),
            Err(_) => Ok(true),
        }
    }

    /// Returns the document's `/P` permission flags as a `PdfPermissions`
    /// struct if the document is encrypted; `None` otherwise.
    ///
    /// Per PDF spec §7.6.3.2 the `/P` flag is advisory — xberg-native-pdf
    /// does not enforce restrictions — but callers who want to
    /// enforce them (e.g., refuse copy-protected PDF extraction) can
    /// do so themselves by checking the returned permissions.
    ///
    /// # PDF spec basis
    ///
    /// §7.6.3.2 Table 22 (`/P` Standard Encryption Dictionary entry).
    /// Decoding is implemented in `encryption::permissions::PdfPermissions::from_p_flag`.
    pub fn permissions(&self) -> Option<crate::encryption::PdfPermissions> {
        // ensure_encryption_initialized may fail on malformed Encrypt
        // dicts — that's fine, no permissions surface for those. ~keep
        let _ = self.ensure_encryption_initialized();
        let handler = self.encryption_handler.lock_or_recover();
        let handler = handler.as_ref()?;
        Some(crate::encryption::PdfPermissions::from_p_flag(
            handler.raw_permissions(),
        ))
    }

    /// Order one MCID's spans for emission in the structure-order assemblers
    ///. A single marked-content element can carry spans across several
    /// visual lines; emitting them in raw extraction order can mis-order them,
    /// so sort by the canonical reading-order comparator. Skipped for single-
    /// span MCIDs and for any MCID containing RTL text (whose span order is
    /// handled by the bidi passes) — both stay byte-identical.
    pub(super) fn order_mcid_spans(spans: &[crate::layout::TextSpan]) -> Vec<&crate::layout::TextSpan> {
        use crate::text::rtl_detector::is_rtl_text;
        let mut ordered: Vec<&crate::layout::TextSpan> = spans.iter().collect();
        if spans.len() <= 1 {
            return ordered;
        }
        let has_rtl = spans.iter().any(|s| s.text.chars().any(|c| is_rtl_text(c as u32)));
        let has_latin = spans.iter().any(|s| s.text.chars().any(|c| c.is_ascii_alphabetic()));
        if !has_rtl {
            ordered.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
        } else if !has_latin {
            // Pure-RTL MCID. The tagged struct-tree path never
            // reaches `reverse_rtl_visual_order_runs`, so without an explicit
            // span-order pass the words emerge in visual (reversed) sequence.
            // Emitting each row right-to-left (X descending) reconstructs
            // logical reading order from geometry, independent of whether the
            // producer stored the run visually or logically. Per-span glyph ~keep
            // order is corrected separately by `push_span_text_bidi`.
            ordered = Self::order_pure_rtl_spans(spans);
        }
        // Mixed RTL+Latin MCIDs keep raw order (full UAX #9 bidi deferred). ~keep
        ordered
    }

    /// Order a pure-RTL MCID's spans into logical reading order: group spans
    /// into visual lines using a **font-relative** vertical tolerance, then
    /// emit each line right-to-left (X descending).
    ///
    /// A fixed quantized row band (`row_aware_span_cmp_rtl` with the global
    /// `ROW_BAND_TOLERANCE_PT`) over-segments Arabic lines. Producers routinely
    /// draw zero-advance glyphs — hamza seats, shadda/kasra marks, and even
    /// whole consonants positioned by a separate zero-width show — 1–3 pt off
    /// the baseline. A coarse fixed band rounds those into adjacent rows, which
    /// then emit before or after the body of the line and scatter the run (the
    /// telltale leading run of stray alef/hamza glyphs). Banding by a tolerance
    /// proportional to the glyph size keeps one jittery line intact while still
    /// separating genuinely distinct lines, whose leading is ~1.2× the font
    /// size — comfortably beyond the tolerance. Per-span glyph order is fixed
    /// separately by [`push_span_text_bidi`]; this function only fixes the order
    /// in which spans are emitted.
    pub(super) fn order_pure_rtl_spans(spans: &[crate::layout::TextSpan]) -> Vec<&crate::layout::TextSpan> {
        use crate::utils::safe_float_cmp;
        let mut by_y: Vec<&crate::layout::TextSpan> = spans.iter().collect();
        // Stable sort, Y descending (top of page first). Ties keep extraction
        // (content-stream) order; the X-descending pass below refines each line. ~keep
        by_y.sort_by(|a, b| safe_float_cmp(b.bbox.y, a.bbox.y));

        let mut out: Vec<&crate::layout::TextSpan> = Vec::with_capacity(spans.len());
        let mut line: Vec<&crate::layout::TextSpan> = Vec::new();
        let mut anchor_y = f32::NAN;
        let mut tol = 0.0f32;
        for s in by_y {
            let fs = if s.font_size.is_finite() && s.font_size > 1.0 {
                s.font_size
            } else {
                10.0
            };
            let starts_new_line = anchor_y.is_finite() && (!s.bbox.y.is_finite() || anchor_y - s.bbox.y > tol);
            if anchor_y.is_nan() || starts_new_line {
                if !line.is_empty() {
                    line.sort_by(|a, b| safe_float_cmp(b.bbox.x, a.bbox.x));
                    out.append(&mut line);
                }
                anchor_y = s.bbox.y;
                tol = 0.5 * fs;
            }
            line.push(s);
        }
        if !line.is_empty() {
            line.sort_by(|a, b| safe_float_cmp(b.bbox.x, a.bbox.x));
            out.append(&mut line);
        }
        out
    }

    /// Pre-pass for the cross-span Arabic GLYPH-interleave defect. Some producers
    /// draw one Arabic word as a multi-glyph body span PLUS separate zero-width
    /// mark / consonant spans positioned by their own show, each at its true x.
    /// The atom-level span sort ([`order_pure_rtl_spans`]) then orders those spans
    /// as whole units and reverses each independently, so a zero-width glyph whose
    /// x falls *inside* a sibling span's x-extent lands at a word edge instead of
    /// interleaved — `الثدييات` extracts as `ثالدييات`.
    ///
    /// Returns `Some(owned spans)` with each affected visual LINE collapsed into a
    /// single visual-order span (so the downstream [`push_span_text_bidi`] reverse
    /// produces correct logical order), or `None` when no line exhibits the defect
    /// (then the caller uses the original spans, byte-identical). Gated tightly:
    /// fires only on a pure-RTL line (no ASCII-alpha) that actually contains a
    /// zero-width span interleaved inside another — a page with no such interleave
    /// (BidiSample, ArabicCIDTrueType-logical, hebrew_mirrored) returns `None`.
    pub(super) fn merge_interleaved_rtl_lines(
        spans: &[crate::layout::TextSpan],
    ) -> Option<Vec<crate::layout::TextSpan>> {
        use crate::utils::safe_float_cmp;
        if spans.len() < 3 {
            return None;
        }
        // Group into visual lines by a font-relative Y tolerance (mirrors
        // order_pure_rtl_spans banding). ~keep
        let mut by_y: Vec<&crate::layout::TextSpan> = spans.iter().collect();
        by_y.sort_by(|a, b| safe_float_cmp(b.bbox.y, a.bbox.y));
        let mut lines: Vec<Vec<&crate::layout::TextSpan>> = Vec::new();
        // Roll the line-break reference forward to the PREVIOUS span's y rather
        // than pinning it to the band's first (topmost) span. RTL producers seat
        // a line's glyphs across a few points of vertical jitter — combining
        // marks ride high, a line-final letter can sit a few points low (P2: a
        // width-0 final glyph at dy≈3pt below the baseline). Against a fixed top
        // anchor the line's own span furthest above the baseline sets the band
        // ceiling, so the lowest glyph can exceed `0.5·fs` and split onto its own
        // "line" — which then reverses and lands after the sentence terminator,
        // detaching the line's final letter. Comparing each span to its immediate
        // predecessor keeps a line whose internal step is < tol intact while a
        // real inter-line gap (leading ≈ one full em, well over tol) still opens
        // the next band. ~keep
        let mut prev_y = f32::NAN;
        let mut tol = 0.0f32;
        for s in by_y {
            let fs = if s.font_size.is_finite() && s.font_size > 1.0 {
                s.font_size
            } else {
                10.0
            };
            let new_line = prev_y.is_finite() && (!s.bbox.y.is_finite() || prev_y - s.bbox.y > tol);
            if prev_y.is_nan() || new_line {
                lines.push(Vec::new());
                tol = 0.5 * fs;
            }
            prev_y = s.bbox.y;
            lines.last_mut().unwrap().push(s);
        }

        let mut any_gated = false;
        let mut out: Vec<crate::layout::TextSpan> = Vec::with_capacity(spans.len());
        for line in &lines {
            if Self::rtl_line_needs_glyph_reorder(line) {
                any_gated = true;
                out.push(Self::merge_rtl_line_to_visual_span(line));
            } else {
                out.extend(line.iter().map(|s| (*s).clone()));
            }
        }
        if any_gated { Some(out) } else { None }
    }

    /// True when a visual line exhibits the zero-width-glyph interleave defect:
    /// (1) pure-RTL — no span carries an ASCII-alphabetic char and at least one
    /// carries an RTL letter; AND (2) a zero-width span's x lies STRICTLY inside
    /// another span's `[x, x+width]` on the line. Both are required so ordinary
    /// pure-RTL text (no interleave) and any mixed-Latin line are left untouched.
    pub(super) fn rtl_line_needs_glyph_reorder(line: &[&crate::layout::TextSpan]) -> bool {
        use crate::text::rtl_detector::is_rtl_text;
        if line.len() < 2 {
            return false;
        }
        let mut has_rtl = false;
        for s in line {
            for c in s.text.chars() {
                if c.is_ascii_alphabetic() {
                    return false;
                }
                if is_rtl_text(c as u32) {
                    has_rtl = true;
                }
            }
        }
        if !has_rtl {
            return false;
        }
        line.iter().any(|m| {
            m.bbox.width.abs() < 0.01
                && line.iter().any(|b| {
                    !std::ptr::eq(*m, *b)
                        && b.bbox.width > 0.01
                        && m.bbox.x > b.bbox.x
                        && m.bbox.x < b.bbox.x + b.bbox.width
                })
        })
    }

    pub(super) fn merge_rtl_line_to_visual_span(line: &[&crate::layout::TextSpan]) -> crate::layout::TextSpan {
        use crate::text::rtl_detector::is_rtl_diacritic;
        use crate::utils::safe_float_cmp;
        // Explode to glyphs: split bases from combining marks, DROP shatter spaces
        // that are interior to a multi-glyph span, but record the x of each
        // STANDALONE space span — those are the producer's real word boundaries
        // (geometric gap-thresholding is unreliable for cursive Arabic, so we use
        // the producer's own segmentation instead). ~keep
        let mut bases: Vec<(f32, char)> = Vec::new();
        let mut marks: Vec<(f32, char)> = Vec::new();
        let mut word_space_x: Vec<f32> = Vec::new();
        for s in line {
            if !s.text.is_empty() && s.text.chars().all(|c| c.is_whitespace()) {
                word_space_x.push(s.bbox.x + s.bbox.width * 0.5);
                continue;
            }
            // Pre-collect the span's chars so a whitespace glyph can see its
            // non-mark neighbours (to tell a cursive-join shatter space from a
            // genuine word break). ~keep
            let span_chars: Vec<char> = s.to_chars().into_iter().map(|t| t.char).collect();
            for (idx, tc) in s.to_chars().into_iter().enumerate() {
                let c = tc.char;
                if c.is_whitespace() {
                    // ISO 32000-1 §14.8.2.3.3: a SPACE that borders a
                    // NON-CURSIVE token (clause punctuation / symbol — not an
                    // Arabic/Hebrew letter and not a digit) is a real word
                    // break, so record its x. A space flanked by cursive letters
                    // is the producer's intra-word shatter (dropped), and a
                    // space between digits is a thousands separator (dropped) —
                    // neither is a word boundary. ~keep
                    use crate::text::rtl_detector::{is_arabic_letter, is_arabic_number, is_hebrew_letter};
                    let neighbour = |it: &mut dyn Iterator<Item = &char>| -> Option<char> {
                        it.copied().find(|&p| !p.is_whitespace() && !is_rtl_diacritic(p as u32))
                    };
                    let is_boundary_marker = |o: Option<char>| {
                        o.is_some_and(|p| {
                            let u = p as u32;
                            !is_arabic_letter(u) && !is_hebrew_letter(u) && !is_arabic_number(u) && !p.is_ascii_digit()
                        })
                    };
                    let prev = neighbour(&mut span_chars[..idx].iter().rev());
                    let next = neighbour(&mut span_chars[idx + 1..].iter());
                    if is_boundary_marker(prev) || is_boundary_marker(next) {
                        word_space_x.push(tc.bbox.x + tc.bbox.width * 0.5);
                    }
                    continue;
                }
                if is_rtl_diacritic(c as u32) {
                    marks.push((tc.bbox.x, c));
                } else {
                    bases.push((tc.bbox.x, c));
                }
            }
        }
        if bases.is_empty() {
            return (*line[0]).clone();
        }
        bases.sort_by(|a, b| safe_float_cmp(a.0, b.0));
        let mut trailing: Vec<Vec<char>> = vec![Vec::new(); bases.len()];
        for (mx, mc) in &marks {
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for (i, (bx, _)) in bases.iter().enumerate() {
                let d = (bx - mx).abs();
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            trailing[best].push(*mc);
        }
        // Emit visual (ascending-x) order: each base then its marks, with a single
        // word-boundary marker wherever a producer word-boundary x falls between two
        // bases. The marker is the private-use sentinel [`Self::RTL_WORD_BOUNDARY`],
        // not a plain SPACE, so the downstream `strip_interior_arabic_spaces` (which
        // only removes U+0020) cannot mistake this AUTHORITATIVE producer-segmented
        // word break for a cursive-shatter artefact and delete it; each output site
        // restores it to a SPACE right after the strip. The downstream reverse maps
        // this to logical order with words intact. ~keep
        let mut text = String::new();
        let mut prev_x: Option<f32> = None;
        for (i, (bx, bc)) in bases.iter().enumerate() {
            if let Some(px) = prev_x
                && word_space_x.iter().any(|sx| *sx > px && *sx < *bx)
                && !text.ends_with(Self::RTL_WORD_BOUNDARY)
            {
                text.push(Self::RTL_WORD_BOUNDARY);
            }
            text.push(*bc);
            for m in &trailing[i] {
                text.push(*m);
            }
            prev_x = Some(*bx);
        }
        let mut merged = (*line[0]).clone();
        let x_min = line.iter().map(|s| s.bbox.x).fold(f32::MAX, f32::min);
        let x_max = line.iter().map(|s| s.bbox.x + s.bbox.width).fold(f32::MIN, f32::max);
        merged.text = text;
        merged.bbox.x = x_min;
        merged.bbox.width = (x_max - x_min).max(0.0);
        merged.char_widths = Vec::new();
        merged
    }

    ///
    /// Used by paths that operate on raw spans rather than ordered
    /// spans (`extract_page_text`, `extract_structured`,
    /// `extract_spans_with_reading_order`). Mutates each covered span's
    /// text to the replacement (run-first only) or clears it
    /// (continuation / suppress-only / non-first-page coverage); fully
    /// suppressed spans are removed.
    ///
    /// Untagged documents and pages with no coverage are no-ops.
    pub(crate) fn apply_actualtext_to_spans(&self, page_index: usize, spans: &mut Vec<crate::layout::TextSpan>) {
        let Some(idx) = self.actualtext_index() else {
            return;
        };
        if idx.covered_mcids.is_empty() {
            return;
        }
        let mc_wins: HashSet<u32> = self
            .mc_actualtext_mcids
            .lock_or_recover()
            .get(&page_index)
            .cloned()
            .unwrap_or_default();

        let default_scope = crate::structure::McidScope::Page(page_index as u32);
        // Visibility = "has at least one raw span at this (scope, mcid)".
        // glyph_text accumulates each key's rendered text for the §14.9.4
        // conformance gate (decline destructive replacements). ~keep
        let mut present: HashSet<(crate::structure::McidScope, u32)> = HashSet::new();
        let mut glyph_text: HashMap<(crate::structure::McidScope, u32), String> = HashMap::new();
        for s in spans.iter() {
            if let Some(m) = s.mcid {
                let scope = s.mcid_scope.clone().unwrap_or(default_scope.clone());
                present.insert((scope.clone(), m));
                glyph_text.entry((scope, m)).or_default().push_str(&s.text);
            }
        }
        // Walk the structure-tree's per-page MCID order so the
        // consecutive-run dedup matches the assemblers'. ~keep
        let mcid_order = self
            .struct_tree_marked()
            .map(|t| self.cached_mcid_order_for_page(&t, page_index as u32))
            .unwrap_or_default();
        let actions = Self::actualtext_actions_for_page(
            Some(&idx),
            &mcid_order,
            |scope, m| present.contains(&(scope.clone(), m)),
            &mc_wins,
            &glyph_text,
        );
        if actions.is_empty() {
            return;
        }

        // Apply actions to the raw spans. EmitAndSuppress mutates the
        // first span of the (scope, mcid) key; subsequent spans for
        // the same key are dropped (so a key with multiple spans
        // collapses to one span carrying the replacement). Suppress
        // drops every span with that key. ~keep
        let mut emit_used: HashSet<(crate::structure::McidScope, u32)> = HashSet::new();
        let mut drop_idx: Vec<usize> = Vec::new();
        for (i, s) in spans.iter_mut().enumerate() {
            let Some(m) = s.mcid else { continue };
            let scope = s.mcid_scope.clone().unwrap_or(default_scope.clone());
            let key = (scope, m);
            match actions.get(&key) {
                Some(ActualTextAction::EmitAndSuppress(repl)) => {
                    if emit_used.insert(key) {
                        s.text = repl.to_string();
                    } else {
                        s.text.clear();
                        drop_idx.push(i);
                    }
                }
                Some(ActualTextAction::Suppress) => {
                    s.text.clear();
                    drop_idx.push(i);
                }
                None => {}
            }
        }
        for &i in drop_idx.iter().rev() {
            spans.remove(i);
        }
    }

    /// Compute the per-page `MCID → ActualTextAction` map.
    ///
    /// Walks `mcid_order` (the structure-tree's per-page MCID sequence
    /// in pre-order) and groups consecutive covered MCIDs by the
    /// replacement text they share. Each group emits ONE replacement at
    /// the first visible-and-not-MC-scope-wins MCID; the rest of the
    /// group is marked `Suppress` (raw glyphs dropped). MCIDs whose
    /// `(page, mcid)` lands in `suppress_only` are always `Suppress`
    /// (their replacement already fired on a different page).
    ///
    /// `visible(mcid)` returns `true` when at least one span carries
    /// the MCID and survives all upstream filters (artifact / OCG /
    /// region). A run with zero visible MCIDs is dropped entirely (no
    /// emission, no suppression — nothing to drop).
    ///
    /// MCIDs in `mc_wins` keep the in-stream MC-scope `/ActualText`
    /// replacement applied by the extractor and are exempt from the
    /// ancestor struct-tree scope; they do not break the run dedup —
    /// the run can still find a non-MC-wins MCID to emit at.
    /// §14.9.4 conformance test for a struct-tree `/ActualText` replacement.
    ///
    /// Per ISO 32000-1 §14.9.4 (pdf.md:39253) an `/ActualText` value "shall be
    /// used as a replacement … providing text that is *equivalent to what a
    /// person would see when viewing the content*"; per §14.8.2.4 NOTE 2
    /// (pdf.md:37380) a conforming reader *may choose* whether to use it. We
    /// decline a replacement that is **destructive**: it would suppress glyphs
    /// carrying alphanumeric (letter/digit, any script) content while itself
    /// carrying none — e.g. a producer tagging whole words with `" "` or `"-"`.
    /// Such a value is not "equivalent to what a person would see", so we keep
    /// the rendered glyphs (extracted via ToUnicode, §14.8.2.4) instead.
    /// Legitimate ActualText — the spec's hyphenation EXAMPLE `(c)`→`k-`,
    /// ligature/soft-hyphen substitution (NOTE 3), any real-character
    /// replacement — is alphanumeric and passes.
    fn actual_text_is_destructive(replacement: &str, covered_glyphs: &str) -> bool {
        covered_glyphs.chars().any(char::is_alphanumeric) && !replacement.chars().any(char::is_alphanumeric)
    }

    fn actualtext_actions_for_page<F: Fn(&crate::structure::McidScope, u32) -> bool>(
        idx: Option<&crate::structure::ActualTextIndex>,
        mcid_order: &[(crate::structure::McidScope, u32)],
        visible: F,
        mc_wins: &HashSet<u32>,
        glyph_text: &HashMap<(crate::structure::McidScope, u32), String>,
    ) -> HashMap<(crate::structure::McidScope, u32), ActualTextAction> {
        let mut out: HashMap<(crate::structure::McidScope, u32), ActualTextAction> = HashMap::new();
        let Some(idx) = idx else {
            return out;
        };
        if idx.covered_mcids.is_empty() {
            return out;
        }

        // Two-pass walk to support runs that span the input order
        // perfectly: collect (scope, mcid, replacement?) tuples for
        // covered MCIDs on this page (across all scopes that render on
        // it), then group consecutive equal-replacement entries into
        // runs.
        //
        // Replacement = None for `suppress_only` entries and for
        // covered keys with no text (defensive — shouldn't happen
        // given the builder invariants). ~keep
        let mut entries: Vec<(crate::structure::McidScope, u32, Option<&str>)> = Vec::new();
        for (scope, m) in mcid_order {
            let key = (scope.clone(), *m);
            if !idx.covered_mcids.contains(&key) {
                continue;
            }
            if idx.suppress_only.contains(&key) {
                entries.push((scope.clone(), *m, None));
                continue;
            }
            let text = idx.mcid_to_actual_text.get(&key).map(|s| &**s);
            entries.push((scope.clone(), *m, text));
        }

        let mut i = 0usize;
        while i < entries.len() {
            let repl_opt = entries[i].2;
            // Find the end of the consecutive run sharing this
            // replacement (None matches None — i.e. suppress-only runs
            // also collapse). ~keep
            let mut j = i;
            while j < entries.len() && entries[j].2 == repl_opt {
                j += 1;
            }

            if let Some(repl) = repl_opt {
                // §14.9.4 conformance gate (pdf.md:39253 + NOTE 2 pdf.md:37380):
                // if this replacement would suppress alphanumeric glyphs while
                // carrying none itself, it is not "equivalent to what a person
                // would see" — decline it (emit no action for the run) so the
                // rendered glyphs survive. See `actual_text_is_destructive`. ~keep
                let run_glyphs: String = entries[i..j]
                    .iter()
                    .filter_map(|e| glyph_text.get(&(e.0.clone(), e.1)))
                    .map(String::as_str)
                    .collect();
                if Self::actual_text_is_destructive(repl, &run_glyphs) {
                    i = j;
                    continue;
                }
                // Find first emit-eligible entry (visible, not MC-wins).
                // MC-wins keys are skipped because their replacement
                // came from the extractor's in-stream BDC /ActualText. ~keep
                let mut emit_pick: Option<(crate::structure::McidScope, u32)> = None;
                for entry in &entries[i..j] {
                    if visible(&entry.0, entry.1) && !mc_wins.contains(&entry.1) {
                        emit_pick = Some((entry.0.clone(), entry.1));
                        break;
                    }
                }
                let repl_arc: std::sync::Arc<str> = std::sync::Arc::from(repl);
                for entry in &entries[i..j] {
                    if mc_wins.contains(&entry.1) {
                        // MC-scope wins: do not touch this MCID at all.
                        // The extractor's inline replacement reaches
                        // output unmodified. ~keep
                        continue;
                    }
                    let key = (entry.0.clone(), entry.1);
                    if emit_pick.as_ref() == Some(&key) {
                        out.insert(key, ActualTextAction::EmitAndSuppress(repl_arc.clone()));
                    } else {
                        out.insert(key, ActualTextAction::Suppress);
                    }
                }
            } else {
                // suppress_only run: every key is suppressed (no
                // emission). MC-wins MCIDs stay untouched. ~keep
                for entry in &entries[i..j] {
                    if mc_wins.contains(&entry.1) {
                        continue;
                    }
                    out.insert((entry.0.clone(), entry.1), ActualTextAction::Suppress);
                }
            }

            i = j;
        }
        out
    }

    /// Page's MCID reading order from the all-pages traversal cache
    /// (`structure_content_cache`, populated once). `build_context` previously
    /// re-walked the whole tree per page (≈ O(pages²) on a tagged document);
    /// the cached all-pages walk yields the same per-page order.
    pub(crate) fn cached_mcid_order_for_page(
        &self,
        struct_tree: &crate::structure::StructTreeRoot,
        page_index: u32,
    ) -> Vec<(crate::structure::McidScope, u32)> {
        if self.structure_content_cache.lock_or_recover().is_none() {
            let all_content = crate::structure::traverse_structure_tree_all_pages(struct_tree);
            *self.structure_content_cache.lock_or_recover() = Some(all_content);
        }
        self.structure_content_cache
            .lock_or_recover()
            .as_ref()
            .and_then(|c| c.get(&page_index))
            .map(|content| {
                content
                    .iter()
                    .filter_map(|c| {
                        // Word break markers have mcid=None; skip. ~keep
                        let m = c.mcid?;
                        // Page-scoped MCIDs default to Page(c.page) when
                        // the parser didn't capture a scope. New parses
                        // always populate `mcid_scope`; the unwrap_or
                        // is for legacy traversals only. ~keep
                        let scope = c
                            .mcid_scope
                            .clone()
                            .unwrap_or(crate::structure::McidScope::Page(c.page));
                        Some((scope, m))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract text from a Tagged PDF page using pre-computed structure traversal cache.
    ///
    /// This is the optimized version of `extract_text_structure_order` that uses
    /// the pre-built `structure_content_cache` for O(1) page content lookup instead
    /// of re-traversing the entire structure tree for each page.
    pub(super) fn extract_text_structure_order_cached_with_spans(
        &self,
        page_index: usize,
        all_spans: Vec<TextSpan>,
        include_artifacts: bool,
    ) -> Result<String> {
        tracing::debug!(target: LOG_TARGET, page = page_index, "extracting text using cached structure order");

        if all_spans.is_empty() {
            let mut text = String::new();
            self.append_non_widget_annotation_text(page_index, &mut text);
            return Ok(text);
        }

        // Drop content marked /Artifact (PDF Spec ISO 32000-1:2008
        // §14.8.2.2 — headers, footers, page numbers, decorations) —
        // unless the caller opted in via `include_artifacts` (default
        // true). The geometric branch in `assemble_text_from_spans`
        // applies the same filter; tagged PDFs taking the structure-order
        // path must honour it too, otherwise artifact spans (including
        // any MC-scope `/ActualText` replacements inside an `/Artifact`
        // BDC) leak into output. Untagged-PDF running-header
        // detection runs at document level and feeds the same flag. ~keep
        let all_spans: Vec<TextSpan> = if include_artifacts {
            all_spans
        } else {
            all_spans.into_iter().filter(|s| s.artifact_type.is_none()).collect()
        };

        let mut mcid_map: HashMap<u32, Vec<TextSpan>> = HashMap::new();
        let mut spans_without_mcid: Vec<TextSpan> = Vec::new();

        for span in all_spans {
            if let Some(mcid) = span.mcid {
                mcid_map.entry(mcid).or_default().push(span);
            } else {
                spans_without_mcid.push(span);
            }
        }

        let ordered_content_owned: Vec<crate::structure::OrderedContent>;
        let ordered_content = {
            let cache = self.structure_content_cache.lock_or_recover();
            ordered_content_owned = cache
                .as_ref()
                .and_then(|c| c.get(&(page_index as u32)))
                .cloned()
                .unwrap_or_default();
            &ordered_content_owned as &[crate::structure::OrderedContent]
        };

        let at_index = self.actualtext_index();
        // MC-scope-wins precedence set: MCIDs whose BDC carried inline
        // `/ActualText` keep the in-stream replacement (most specific
        // declaration) and are exempt from ancestor struct-tree
        // emissions. ~keep
        let mc_wins: HashSet<u32> = self
            .mc_actualtext_mcids
            .lock_or_recover()
            .get(&page_index)
            .cloned()
            .unwrap_or_default();
        let default_scope = crate::structure::McidScope::Page(page_index as u32);
        let mcid_order: Vec<(crate::structure::McidScope, u32)> = ordered_content
            .iter()
            .filter_map(|c| {
                c.mcid
                    .map(|m| (c.mcid_scope.clone().unwrap_or(default_scope.clone()), m))
            })
            .collect();
        // Per-key rendered glyph text for the §14.9.4 conformance gate. ~keep
        let mut glyph_text: HashMap<(crate::structure::McidScope, u32), String> = HashMap::new();
        for (scope, m) in &mcid_order {
            if let Some(sp) = mcid_map.get(m) {
                let joined: String = sp.iter().map(|s| s.text.as_str()).collect();
                glyph_text.entry((scope.clone(), *m)).or_default().push_str(&joined);
            }
        }
        let actions = Self::actualtext_actions_for_page(
            at_index.as_deref(),
            &mcid_order,
            |_scope, m| mcid_map.contains_key(&m),
            &mc_wins,
            &glyph_text,
        );

        tracing::debug!(target: LOG_TARGET,
            "Cached structure content: {} items for page {}, {} MCIDs with spans, {} ActualText actions on this page",
            ordered_content.len(),
            page_index,
            mcid_map.len(),
            actions.len()
        );

        let mut text = String::with_capacity(mcid_map.len() * 50);
        let mut prev_span: Option<TextSpan> = None;
        let mut prev_in_table = false;
        let mut consumed_mcids: HashSet<u32> = HashSet::new();

        for content in ordered_content {
            if content.is_word_break {
                if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
                    text.push(' ');
                }
                continue;
            }

            let Some(mcid) = content.mcid else {
                continue;
            };
            // ISO 32000-1 §14.7: a marked-content sequence's MCID is unique
            // within its content stream and is referenced from the structure
            // hierarchy at most once. A malformed struct tree that re-references
            // the same MCID multiple times would otherwise emit that MCID's
            // glyphs once per reference. Emit each MCID once. (A destructive
            // /ActualText replacement — now declined by the §14.9.4 conformance
            // gate — can mask this by collapsing each consecutive run to a
            // single emit.) ~keep
            if !consumed_mcids.insert(mcid) {
                continue;
            }
            let mcid_scope_key = content.mcid_scope.clone().unwrap_or(default_scope.clone());

            match actions.get(&(mcid_scope_key, mcid)) {
                Some(ActualTextAction::EmitAndSuppress(repl)) => {
                    consumed_mcids.insert(mcid);
                    if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(repl);
                    continue;
                }
                Some(ActualTextAction::Suppress) => {
                    consumed_mcids.insert(mcid);
                    continue;
                }
                None => {}
            }

            if let Some(spans) = mcid_map.get(&mcid) {
                consumed_mcids.insert(mcid);
                let rtl_run = Self::mcid_run_is_pure_rtl(spans);
                // Repair the cross-span Arabic glyph-interleave defect (zero-width
                // mark/consonant spans landing at word edges) before ordering. ~keep
                let merged_rtl = Self::merge_interleaved_rtl_lines(spans);
                let use_spans: &[crate::layout::TextSpan] = merged_rtl.as_deref().unwrap_or(spans);
                for span in Self::order_mcid_spans(use_spans) {
                    if let Some(prev) = &prev_span {
                        let y_diff = (prev.bbox.y - span.bbox.y).abs();
                        if y_diff > Self::same_line_threshold(prev, span) {
                            // Suppress the break when a Hangul eojeol wrapped
                            // mid-syllable (no inter-eojeol space at the wrap), so
                            // the word stays whole for word-segmentation scoring. ~keep
                            if !Self::hangul_midword_line_wrap(&text, prev, span) {
                                Self::push_line_breaks(
                                    &mut text,
                                    prev,
                                    span,
                                    y_diff,
                                    content.in_table && prev_in_table,
                                );
                            }
                        } else if Self::should_insert_space(prev, span) || Self::stacked_cell_needs_space(prev, span) {
                            text.push(' ');
                        }
                    }

                    Self::push_span_text_bidi(&mut text, span, rtl_run);
                    prev_span = Some(span.clone());
                    prev_in_table = content.in_table;
                }
            }
        }

        let mut unconsumed: Vec<(&u32, &Vec<TextSpan>)> = mcid_map
            .iter()
            .filter(|(mcid, _)| !consumed_mcids.contains(mcid))
            .collect();
        unconsumed.sort_by_key(|(mcid, _)| **mcid);
        if !unconsumed.is_empty() {
            tracing::warn!(target: LOG_TARGET,
                "Appending {} unreferenced MCIDs (e.g., from Form XObjects without StructParents)",
                unconsumed.len()
            );
            for (_mcid, spans) in &unconsumed {
                let rtl_run = Self::mcid_run_is_pure_rtl(spans);
                for span in *spans {
                    if let Some(prev) = &prev_span {
                        let y_diff = (prev.bbox.y - span.bbox.y).abs();
                        if y_diff > Self::same_line_threshold(prev, span) {
                            text.push('\n');
                        } else if Self::should_insert_space(prev, span) {
                            text.push(' ');
                        }
                    }
                    Self::push_span_text_bidi(&mut text, span, rtl_run);
                    prev_span = Some(span.clone());
                }
            }
        }

        if !spans_without_mcid.is_empty() {
            tracing::warn!(target: LOG_TARGET,
                "Found {} text spans without MCID (including form field widgets) - appending sorted by position",
                spans_without_mcid.len()
            );
            crate::utils::sort_by_row_band(&mut spans_without_mcid, |s| s.bbox.y, |s| s.bbox.x);
            for span in &spans_without_mcid {
                if let Some(prev) = &prev_span {
                    let y_diff = (prev.bbox.y - span.bbox.y).abs();
                    if y_diff > Self::same_line_threshold(prev, span) {
                        text.push('\n');
                    } else if Self::should_insert_space(prev, span) {
                        text.push(' ');
                    }
                }
                Self::push_span_text_bidi(&mut text, span, false);
                prev_span = Some(span.clone());
            }
        }

        // Annotation text is already included via annotation_content_spans() in
        // extract_spans() — do NOT call append_non_widget_annotation_text() here
        // (would cause double-emission of all annotation text). ~keep

        Ok(text)
    }

    /// Extract text spans from a page (PDF spec compliant - RECOMMENDED).
    ///
    /// This is the recommended method for text extraction. It extracts complete
    /// text strings as the PDF provides them via Tj/TJ operators, following the
    /// PDF specification ISO 32000-1:2008.
    ///
    /// Whole merged spans are kept when their bounding boxes overlap the normalized
    /// page MediaBox expanded by 2 points; partial overlaps are kept without clipping.
    /// This membership test runs in unrotated PDF coordinates before page rotation.
    /// [`Self::extract_chars`] retains off-page characters for low-level inspection.
    ///
    /// # Benefits over extract_chars
    /// - Avoids overlapping character issues
    /// - Preserves PDF's text positioning intent
    /// - More robust for complex layouts
    /// - Matches industry best practices (PyMuPDF, etc.)
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// Vector of TextSpan objects in reading order
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("document.pdf")?;
    /// let spans = doc.extract_spans(0)?;
    /// for span in spans {
    ///     println!("Text: {} at ({}, {})", span.text, span.bbox.x, span.bbox.y);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_spans(&self, page_index: usize) -> Result<Vec<crate::layout::TextSpan>> {
        // Serve repeat per-page extractions from cache (the converters reach
        // here twice per page; see `page_spans_cache`). ~keep
        if let Some(cached) = self.page_spans_cache.lock_or_recover().get(&page_index) {
            return Ok((**cached).clone());
        }
        let spans = self.extract_spans_raw(page_index)?;
        let spans = self.postprocess_spans(page_index, spans)?;
        self.page_spans_cache
            .lock_or_recover()
            .insert(page_index, std::sync::Arc::new(spans.clone()));
        Ok(spans)
    }

    /// Get (building and caching if needed) the lightweight search index for
    /// one page. Backs `search()`/`search_page()` — see `search_index` field
    /// docs for why this is a separate, unbounded cache from
    /// `page_spans_cache`.
    pub(crate) fn search_page_index(
        &self,
        page_index: usize,
    ) -> Result<std::sync::Arc<crate::search::SearchPageIndex>> {
        if let Some(cached) = self.search_index.lock_or_recover().get(&page_index) {
            return Ok(std::sync::Arc::clone(cached));
        }
        let spans = self.extract_spans(page_index)?;
        let index = std::sync::Arc::new(crate::search::SearchPageIndex::from_spans(&spans));
        self.search_index
            .lock_or_recover()
            .insert(page_index, std::sync::Arc::clone(&index));
        Ok(index)
    }

    /// Build the search index for every page up front.
    ///
    /// `search()` builds this lazily one page at a time as needed, so calling
    /// this is optional — it exists for callers who want the first `search()`
    /// call to be as fast as the rest, at the cost of paying full-document
    /// extraction immediately instead of spread across the first sweep.
    pub fn prepare_search(&self) -> Result<()> {
        for page_index in 0..self.page_count()? {
            self.search_page_index(page_index)?;
        }
        Ok(())
    }

    /// Drop the cached search index, if any, freeing its memory.
    /// `search()`/`search_page()` will rebuild it lazily on next use.
    pub fn clear_search_index(&self) {
        self.search_index.lock_or_recover().clear();
    }

    pub(super) fn extract_spans_filtered(
        &self,
        page_index: usize,
        excluded_layers: HashSet<String>,
        excluded_inks: HashSet<String>,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        let spans = self.extract_spans_raw_filtered(page_index, excluded_layers, excluded_inks)?;
        self.postprocess_spans(page_index, spans)
    }
}
