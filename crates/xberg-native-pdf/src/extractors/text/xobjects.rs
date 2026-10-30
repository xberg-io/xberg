//! Form-XObject resolution and recursive processing.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// Resolve XObject name to ObjectRef using cached mapping.
    fn resolve_xobject_ref(&mut self, name: &str) -> Result<Option<ObjectRef>> {
        if let Some(cached) = self.cached_xobject_refs.get(name) {
            return Ok(*cached);
        }

        let resources = match &self.resources {
            Some(res) => res.clone(),
            None => return Ok(None),
        };

        let doc = match self.document {
            Some(d) => d,
            None => return Ok(None),
        };

        let Some(xobject_obj) = Self::xobject_dict_of(doc, &resources)? else {
            return Ok(None);
        };

        let xobject_dict = match xobject_obj.as_dict() {
            Some(d) => d,
            None => return Ok(None),
        };

        for (key, val) in xobject_dict.iter() {
            let obj_ref = val.as_reference();
            self.cached_xobject_refs.insert(key.clone(), obj_ref);
        }

        Ok(self.cached_xobject_refs.get(name).copied().flatten())
    }

    /// The `/XObject` dictionary of a resources object, with references resolved.
    pub(super) fn xobject_dict_of(doc: &crate::document::PdfDocument, resources: &Object) -> Result<Option<Object>> {
        let resources_obj = if let Some(res_ref) = resources.as_reference() {
            doc.load_object(res_ref)?
        } else {
            resources.clone()
        };
        let Some(xobject_entry) = resources_obj.as_dict().and_then(|d| d.get("XObject")).cloned() else {
            return Ok(None);
        };
        let xobject_obj = if let Some(xobj_ref) = xobject_entry.as_reference() {
            doc.load_object(xobj_ref)?
        } else {
            xobject_entry
        };
        Ok(xobject_obj.as_dict().is_some().then_some(xobject_obj))
    }

    /// The decoded stream of a Form XObject, through the document's stream cache.
    pub(super) fn decode_form_stream(
        doc: &crate::document::PdfDocument,
        xobject: &Object,
        xobject_ref: ObjectRef,
    ) -> Result<Vec<u8>> {
        let cached_stream = { doc.xobject_stream_cache.lock().unwrap().get(&xobject_ref).cloned() };
        if let Some(cached) = cached_stream {
            return Ok(cached.as_ref().clone());
        }
        let data = doc.decode_stream_with_encryption(xobject, xobject_ref)?;
        const MAX_STREAM_CACHE_BYTES: usize = 50 * 1024 * 1024;
        let current = doc
            .xobject_stream_cache_bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        if current + data.len() <= MAX_STREAM_CACHE_BYTES {
            doc.xobject_stream_cache_bytes
                .store(current + data.len(), std::sync::atomic::Ordering::Relaxed);
            doc.xobject_stream_cache
                .lock()
                .unwrap()
                .insert(xobject_ref, std::sync::Arc::new(data.clone()));
        }
        Ok(data)
    }

    /// Quantize a CTM to 6 millipoint-rounded i64 values so it can be used as
    /// a HashSet/HashMap key without floating-point equality hazards.
    /// Rounds to nearest millipoint instead of truncating with `as i64`, so
    /// floating-point noise in the same logical CTM produces a stable key
    /// (truncation alone could send 0.99999... / 1.00001... to different
    /// buckets). Split out of `process_xobject` unchanged, for file size. ~keep
    fn quantize_ctm(ctm: Matrix) -> [i64; 6] {
        [
            (ctm.a * 1000.0).round() as i64,
            (ctm.b * 1000.0).round() as i64,
            (ctm.c * 1000.0).round() as i64,
            (ctm.d * 1000.0).round() as i64,
            (ctm.e * 1000.0).round() as i64,
            (ctm.f * 1000.0).round() as i64,
        ]
    }

    /// Parse /Matrix from a Form XObject dict. Defaults to identity per ISO
    /// 32000-1 §8.10.1 when absent or malformed. Split out of
    /// `process_xobject` unchanged, for file size. ~keep
    fn parse_form_matrix(xobject_dict: &std::collections::HashMap<String, Object>) -> Matrix {
        let Some(Object::Array(arr)) = xobject_dict.get("Matrix") else {
            return Matrix::identity();
        };
        let get_f32 = |i: usize| -> f32 {
            match arr.get(i) {
                Some(Object::Real(v)) => *v as f32,
                Some(Object::Integer(v)) => *v as f32,
                _ => {
                    if i == 0 || i == 3 {
                        1.0
                    } else {
                        0.0
                    }
                }
            }
        };
        Matrix {
            a: get_f32(0),
            b: get_f32(1),
            c: get_f32(2),
            d: get_f32(3),
            e: get_f32(4),
            f: get_f32(5),
        }
    }

    /// Parse /BBox (form coordinate space) from a Form XObject dict, for the
    /// §8.10.1 form clip. A form XObject's painting is clipped to its /BBox;
    /// text the form draws outside the BBox is invisible in a conformant
    /// renderer and must not be extracted. Returns `[x0,y0,x1,y1]` with
    /// `[x0,y0]` normalized to the min corner; `None` disables the clip
    /// (defensive — /BBox is required, but malformed dicts exist). Split out
    /// of `process_xobject` unchanged, for file size. ~keep
    fn parse_form_bbox(xobject_dict: &std::collections::HashMap<String, Object>) -> Option<[f32; 4]> {
        let Some(Object::Array(arr)) = xobject_dict.get("BBox") else {
            return None;
        };
        if arr.len() < 4 {
            return None;
        }
        let f = |i: usize| -> Option<f32> {
            match arr.get(i) {
                Some(Object::Real(v)) => Some(*v as f32),
                Some(Object::Integer(v)) => Some(*v as f32),
                _ => None,
            }
        };
        match (f(0), f(1), f(2), f(3)) {
            (Some(a), Some(b), Some(c), Some(d))
                if a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite() =>
            {
                Some([a.min(c), b.min(d), a.max(c), b.max(d)])
            }
            _ => None,
        }
    }

    pub(super) fn process_xobject(&mut self, name: &str) -> Result<()> {
        if self.xobject_depth >= Self::MAX_XOBJECT_DEPTH {
            return Ok(());
        }
        if self.xobject_decode_count >= Self::MAX_XOBJECT_DECODES {
            return Ok(());
        }

        // Resolve name → ObjectRef using cached mapping (avoids expensive
        // repeated resolution of resources/XObject dict chain) ~keep
        let xobject_ref = match self.resolve_xobject_ref(name)? {
            Some(r) => r,
            None => return Ok(()),
        };

        // Build a CTM-aware deduplication key.
        //
        // Using just `xobject_ref` as the key incorrectly blocked re-processing
        // the same Form XObject when it was invoked a second time on the same page
        // with a different CTM (e.g., same header/footer XObject stamped at two
        // different Y positions, or the pattern where each page's
        // content stream sets a different `cm` translation before calling `Do`).
        //
        // The CTM is encoded as 6 millipoint-rounded i64 values so it can be
        // stored in a HashSet without floating-point equality hazards.
        // Infinite-recursion cycles are still prevented because a truly recursive
        // call re-enters with the *same* XObject ref AND the same CTM at that
        // nesting depth; the depth limiter (MAX_XOBJECT_DEPTH) provides a
        // second backstop. ~keep
        let current_ctm = self.state_stack.current().ctm;
        let ctm_key = Self::quantize_ctm(current_ctm);
        let xobj_key = (xobject_ref, ctm_key);

        // Skip already-processed (XObject, CTM) pairs — each unique combination
        // is processed at most once per page for text extraction. ~keep
        if self.processed_xobjects.contains(&xobj_key) {
            return Ok(());
        }

        self.processed_xobjects.insert(xobj_key);

        let doc = match self.document {
            Some(d) => d,
            None => return Ok(()),
        };

        if doc.xobject_text_free_cache.lock().unwrap().contains(&xobject_ref) {
            return Ok(());
        }

        // Quick Subtype check: skip Image XObjects without loading the full object.
        // Image XObjects can be megabytes of compressed pixel data — loading them
        // just to discover Subtype=Image is a major bottleneck (10-15ms per image). ~keep
        if !doc.is_form_xobject(xobject_ref) {
            return Ok(());
        }

        // Span result cache: reuse extracted spans from self-contained Form XObjects.
        //
        // The cache key is (ObjectRef, ctm_key) where ctm_key encodes the caller's
        // CTM as 6 millipoint-rounded i64 values. This allows the same Form XObject
        // to have independent cached results for each unique CTM it is painted with,
        // fixing the issue where cross-page reuse of a single Form XObject with
        // different per-page CTM translations returned stale page-0 coordinates on
        // all subsequent pages.
        //
        // `ctm_key` was already computed above for the `processed_xobjects` guard. ~keep
        let spans_cache_key = (xobject_ref, ctm_key);
        let has_filters = !self.excluded_layers.is_empty() || !self.excluded_inks.is_empty();
        if self.extract_spans && !has_filters {
            let cached_spans = { doc.xobject_spans_cache.lock().unwrap().get(&spans_cache_key).cloned() };
            if let Some(cached_spans) = cached_spans {
                if let Some(spans) = cached_spans {
                    self.spans.extend(spans.iter().cloned());
                }
                return Ok(());
            }
        }

        // Load the XObject (now known to be Form or unknown — worth the full load) ~keep
        let xobject = doc.load_object(xobject_ref)?;

        let xobject_dict = match xobject.as_dict() {
            Some(d) => d,
            None => {
                tracing::debug!(target: LOG_TARGET, "XObject '{}' is not a dictionary", name);
                return Ok(());
            }
        };

        let subtype = xobject_dict.get("Subtype").and_then(|s| s.as_name());

        match subtype {
            Some("Form") => {
                tracing::debug!(target: LOG_TARGET, "Processing Form XObject: {}", name);

                // Pre-decode resource check: if the XObject's own /Resources has
                // neither /Font nor /XObject entries, it cannot render text directly
                // and cannot invoke nested XObjects. Skip it without decoding the
                // stream, which avoids expensive FlateDecode decompression. ~keep
                if let Some(xobj_resources) = xobject_dict.get("Resources") {
                    let xobj_res = if let Some(res_ref) = xobj_resources.as_reference() {
                        doc.load_object(res_ref).ok()
                    } else {
                        Some(xobj_resources.clone())
                    };

                    if let Some(ref res_obj) = xobj_res
                        && let Some(res_dict) = res_obj.as_dict()
                    {
                        let has_font = res_dict.contains_key("Font");
                        let has_xobject = res_dict.contains_key("XObject");
                        if !has_font && !has_xobject {
                            tracing::debug!(target: LOG_TARGET, "Skipping Form XObject '{}': no Font/XObject in Resources", name);
                            doc.xobject_text_free_cache.lock().unwrap().insert(xobject_ref);
                            return Ok(());
                        }
                    }
                } else {
                    // No Resources at all — XObject inherits page-level fonts but
                    // still must be decoded to check for text operators. However,
                    // Form XObjects that are pure graphics often omit Resources
                    // entirely when they have no font/xobject needs. Check if the
                    // page has any active fonts; if not, skip. ~keep
                }

                self.xobject_decode_count += 1;
                let stream_data = match Self::decode_form_stream(doc, &xobject, xobject_ref) {
                    Ok(data) => data,
                    Err(error) => {
                        tracing::warn!(target: LOG_TARGET,
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "failed to decode Form XObject stream; skipping"
                        );
                        return Ok(());
                    }
                };

                if !crate::document::PdfDocument::may_contain_text(&stream_data) {
                    tracing::debug!(target: LOG_TARGET,
                        "Skipping text-free Form XObject '{}' ({} bytes)",
                        name,
                        stream_data.len()
                    );
                    doc.xobject_text_free_cache.lock().unwrap().insert(xobject_ref);
                    return Ok(());
                }

                // Parse /Matrix (default: identity per ISO 32000-1 §8.10.1) and
                // /BBox (the §8.10.1 form clip) from the Form XObject dict. See
                // `parse_form_matrix`/`parse_form_bbox` for the per-field
                // parsing rules (unchanged, just split out for file size). ~keep
                let form_matrix = Self::parse_form_matrix(xobject_dict);
                let form_bbox = Self::parse_form_bbox(xobject_dict);

                // Only save/restore fonts+resources when XObject has its own Resources.
                // Avoids expensive HashMap clone for XObjects that inherit page fonts. ~keep
                let has_own_resources = xobject_dict.contains_key("Resources");

                let saved_fonts;
                let saved_resources;
                let saved_xobj_cache;

                if has_own_resources {
                    saved_fonts = Some(self.fonts.clone());
                    saved_resources = self.resources.clone();
                    saved_xobj_cache = Some(std::mem::take(&mut self.cached_xobject_refs));

                    // Safety: has_own_resources was set by contains_key("Resources")
                    // so get("Resources") will always return Some here ~keep
                    let xobj_resources = xobject_dict
                        .get("Resources")
                        .expect("contains_key confirmed Resources exists");
                    let xobj_res = if let Some(res_ref) = xobj_resources.as_reference() {
                        match doc.load_object(res_ref) {
                            Ok(obj) => obj,
                            Err(_) => xobj_resources.clone(),
                        }
                    } else {
                        xobj_resources.clone()
                    };

                    if let Err(error) = doc.load_fonts(&xobj_res, self) {
                        tracing::warn!(
                            target: crate::LOG_TARGET_ROOT,
                            operation = "load_form_fonts",
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "using page fonts for form"
                        );
                    }

                    self.resources = Some(xobj_res);
                } else {
                    saved_fonts = None;
                    saved_resources = None;
                    saved_xobj_cache = None;
                }

                let spans_before = self.spans.len();
                // The /BBox clip below must prune the character layer on the
                // same decision it prunes spans, or `extract_chars` reports
                // marks `extract_text` correctly dropped. ~keep
                let chars_before = self.chars.len();

                // Save graphics state (implicit q per ISO 32000-1 §8.10.1) ~keep
                self.state_stack.save();

                let state = self.state_stack.current_mut();
                state.ctm = form_matrix.multiply(&state.ctm);

                // Effective form→page transform used for every span drawn inside
                // the form; the /BBox clip below maps the BBox through this same
                // CTM so the comparison happens in one coordinate space. ~keep
                let form_ctm = self.state_stack.current().ctm;

                // Push the Form XObject scope (ISO 32000-1:2008
                // §14.7.4.3). Every MCID emitted inside this form's
                // content stream lives in the form's MCID namespace,
                // *not* the page's. Two distinct forms on the same
                // page that both emit MCID 0 stay distinct because
                // they push different `Form(form_ref)` scopes. ~keep
                self.mcid_scope_stack
                    .push(crate::structure::McidScope::Form(xobject_ref));

                // The page verdict reads the page stream only, which does not hold
                // the text of the forms it paints. A form that carries its own
                // /PlacedPDF scope gets its own verdict, counted against all the text
                // the page paints outside any placed scope, so a body placed inside a
                // form is not dropped and a duplicate overlay in a form is still
                // suppressed. A page that already keeps /PlacedPDF text keeps it
                // inside the form. ~keep
                let saved_placed_pdf_keep = self.placed_pdf_keep;
                let form_scan = if saved_placed_pdf_keep {
                    None
                } else {
                    Self::scan_placed_text(&stream_data)
                };
                // A form with the tag is kept or dropped by the verdict of the page that
                // paints it, whether it judges itself or follows the page. ~keep
                let verdicts_before = self.form_placed_verdicts;
                if Self::has_placed_tag(&stream_data) {
                    self.form_placed_verdicts += 1;
                }
                let form_scan_made = form_scan.is_some();
                if let Some(scan) = form_scan {
                    self.form_verdict_depth += 1;
                    let keep = Self::placed_scan_dominates(&scan.placed, self.page_outside_text());
                    self.placed_pdf_keep = keep;
                }

                self.xobject_depth += 1;
                let parse_result = self.run_content_stream(&stream_data);
                self.xobject_depth -= 1;
                self.placed_pdf_keep = saved_placed_pdf_keep;
                if form_scan_made {
                    self.form_verdict_depth -= 1;
                }
                // Pop the Form XObject scope pushed before the
                // content-stream walk. Cleared regardless of parse
                // success so the parent stream's scope is correctly
                // restored even on errors. ~keep
                self.mcid_scope_stack.pop();
                if let Err(error) = parse_result {
                    tracing::warn!(
                        target: crate::LOG_TARGET_ROOT,
                        operation = "parse_form_content",
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "returning partial form text after parse failure"
                    );
                }

                // Apply the Form XObject /BBox clip (ISO 32000-1:2008 §8.10.1): a
                // form's marks are clipped to its BBox, so text the form paints
                // OUTSIDE the BBox is invisible in a conformant renderer (pdfium,
                // Acrobat, MuPDF) and must not be extracted. Some producers (e.g.
                // pdfTeX \includegraphics of a figure PDF that retained a full
                // draft-galley page) paint a redundant copy of the article body
                // outside the figure's BBox; without this clip it surfaces as
                // duplicate text overlapping the real page body. Done BEFORE the
                // span cache so cached results are already clipped. Byte-identical
                // on every form whose painted text lies inside its BBox (the
                // conformant majority) — only out-of-BBox marks are dropped. ~keep
                if let Some([bx0, by0, bx1, by1]) = form_bbox {
                    let grew = self.spans.len() > spans_before || self.chars.len() > chars_before;
                    if grew && bx1 > bx0 && by1 > by0 {
                        // Map the BBox corners through the form CTM into page space
                        // and take the axis-aligned bound (a superset for rotated
                        // forms — conservative, never over-clips). ~keep
                        let c = [
                            form_ctm.transform_point(bx0, by0),
                            form_ctm.transform_point(bx1, by0),
                            form_ctm.transform_point(bx1, by1),
                            form_ctm.transform_point(bx0, by1),
                        ];
                        let min_x = c.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                        let max_x = c.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                        let min_y = c.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                        let max_y = c.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                        if min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite() {
                            // Tolerance so glyphs sitting exactly on the clip edge
                            // are kept (conformant clipping is exact; this only
                            // guards float rounding, far below any real margin). ~keep
                            const TOL: f32 = 1.0;
                            let inside_bbox = |b: &crate::geometry::Rect| {
                                let cx = b.x + b.width * 0.5;
                                let cy = b.y + b.height * 0.5;
                                cx >= min_x - TOL && cx <= max_x + TOL && cy >= min_y - TOL && cy <= max_y + TOL
                            };
                            let inside = |s: &TextSpan| inside_bbox(&s.bbox);
                            // Fast path: when every span this form painted is
                            // already inside its /BBox (the conformant majority —
                            // and where this clip is a no-op anyway), skip the
                            // split_off/extend allocation churn entirely. Only the
                            // rare out-of-BBox case (the draft-galley underlay)
                            // pays for the rebuild. Cheap O(form-spans) scan, no
                            // allocation; keeps large form-heavy docs fast. ~keep
                            let spans_stray = self.spans[spans_before..].iter().any(|s| !inside(s));
                            let chars_stray = self.chars[chars_before..].iter().any(|c| !inside_bbox(&c.bbox));
                            if spans_stray || chars_stray {
                                // Out-of-BBox spans exist. Distinguish a real
                                // figure form (whose stray out-of-BBox text is a
                                // draft-galley underlay safe to drop) from a
                                // full-page content-frame wrapper whose declared
                                // BBox happens to exclude real body text. A
                                // conformant *renderer* clips both, but every text
                                // *extractor* (poppler/pdftotext, the common
                                // reference) keeps a wrapper's body — and that body
                                // may be its only copy. The discriminator is
                                // coverage: a figure occupies a sub-region of the
                                // page; a wrapper covers most of it. Only clip when
                                // the form is figure-sized, so the galley-dedup win
                                // stays while page-wrapper bodies are preserved. ~keep
                                let clip_area = (max_x - min_x) * (max_y - min_y);
                                let page_idx = match self.mcid_scope_stack.first() {
                                    Some(crate::structure::McidScope::Page(p)) => *p as usize,
                                    _ => 0,
                                };
                                let page_area = self
                                    .document
                                    .and_then(|d| d.get_page_media_box(page_idx).ok())
                                    .map(|(llx, lly, urx, ury)| ((urx - llx) * (ury - lly)).abs())
                                    .filter(|a| *a > 0.0);
                                // ≥60% of page area ⇒ content-frame wrapper, not a
                                // figure (figures measured ≤27%; wrappers ≥82%). ~keep
                                let is_page_wrapper = page_area.is_some_and(|pa| clip_area >= 0.6 * pa);
                                if !is_page_wrapper {
                                    if spans_stray {
                                        let added = self.spans.split_off(spans_before);
                                        let kept: Vec<TextSpan> = added.into_iter().filter(|s| inside(s)).collect();
                                        self.spans.extend(kept);
                                    }
                                    if chars_stray {
                                        let added = self.chars.split_off(chars_before);
                                        let kept: Vec<TextChar> =
                                            added.into_iter().filter(|c| inside_bbox(&c.bbox)).collect();
                                        self.chars.extend(kept);
                                    }
                                }
                            }
                        }
                    }
                }

                // Cache span results for self-contained Form XObjects.
                //
                // The cache key `spans_cache_key` already encodes (ObjectRef, ctm_key),
                // so each unique (XObject, CTM) pair gets its own entry. There is no
                // longer any need to restrict caching to identity-CTM invocations —
                // different CTMs produce different cache entries and therefore cannot
                // pollute each other.
                //
                // We still require `has_own_resources` so that font lookups are
                // self-contained; XObjects that inherit page-level fonts would
                // produce spans whose glyph mappings depend on caller context. ~keep
                // A form whose run held a /PlacedPDF tag, or ran inside a form that made a verdict, is not
                // cached: another page would reuse spans kept or dropped by this page's verdict. ~keep
                if has_own_resources
                    && self.extract_spans
                    && !has_filters
                    && self.form_placed_verdicts == verdicts_before
                    && self.form_verdict_depth == 0
                {
                    let new_spans = if self.spans.len() > spans_before {
                        Some(self.spans[spans_before..].to_vec())
                    } else {
                        None
                    };
                    doc.xobject_spans_cache
                        .lock()
                        .unwrap()
                        .insert(spans_cache_key, new_spans);
                }

                // Restore graphics state (implicit Q per ISO 32000-1 §8.10.1) ~keep
                self.state_stack.restore();
                let current_font = self
                    .state_stack
                    .current()
                    .font_name
                    .as_ref()
                    .and_then(|name| self.fonts.get(name))
                    .cloned();
                self.set_cached_current_font(current_font);

                if let Some(fonts) = saved_fonts {
                    self.fonts = fonts;
                }
                if let Some(res) = saved_resources {
                    self.resources = Some(res);
                }
                if let Some(cache) = saved_xobj_cache {
                    self.cached_xobject_refs = cache;
                }
                // Re-evaluate ink exclusion against the restored color space
                // and resources. The XObject may have set an excluded ink that
                // must not persist into the caller's scope. ~keep
                if !self.excluded_inks.is_empty() {
                    let cs = self.state_stack.current().fill_color_space.clone();
                    self.inside_excluded_ink = self.is_excluded_ink_color_space(&cs);
                }

                // Keep xobject_ref in processed_xobjects permanently.
                // For text extraction, re-processing the same Form XObject produces
                // identical text. Keeping it prevents O(n!) fan-out in pages with
                // deep XObject trees (e.g., 4000+ nested chart elements). ~keep

                Ok(())
            }
            Some("Image") => {
                tracing::debug!(target: LOG_TARGET, "Skipping Image XObject: {}", name);
                Ok(())
            }
            _ => {
                tracing::debug!(target: LOG_TARGET, "Unknown XObject subtype for '{}': {:?}", name, subtype);
                Ok(())
            }
        }
    }
}
