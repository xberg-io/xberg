//! Vector path extraction and optional-content layer resolution.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Inspect only a bounded page-local stream before materializing its vector paths.
    #[doc(hidden)]
    pub fn extract_bounded_page_local_paths(
        &self,
        page_index: usize,
        byte_limit: usize,
        operator_limit: usize,
    ) -> Result<Vec<crate::elements::PathContent>> {
        use crate::content::Operator;
        use crate::content::parser::visit_complete_content_stream;

        let content = self.cached_page_content(page_index)?;
        if content.len() > byte_limit {
            return Ok(Vec::new());
        }
        let mut has_curve = false;
        let supported = visit_complete_content_stream(&content, operator_limit, |op| {
            has_curve |= matches!(
                op,
                Operator::CurveTo { .. } | Operator::CurveToV { .. } | Operator::CurveToY { .. }
            );
            !matches!(
                op,
                Operator::Do { .. } | Operator::InlineImage { .. } | Operator::ClipNonZero | Operator::ClipEvenOdd
            )
        });
        if !matches!(supported, Ok(true)) || !has_curve {
            return Ok(Vec::new());
        }
        self.extract_paths(page_index)
    }

    /// Extract path (vector graphics) content from a page.
    ///
    /// This extracts all vector graphics operations from the page's content stream,
    /// including lines, curves, rectangles, and shapes.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// A vector of `PathContent` objects representing all paths on the page.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    ///
    /// // Extract paths from first page
    /// let paths = doc.extract_paths(0)?;
    ///
    /// for path in paths {
    ///     println!("Path with {} operations, bbox: {:?}",
    ///         path.operations.len(), path.bbox);
    ///     if path.has_stroke() {
    ///         println!(" Stroked with width: {}", path.stroke_width);
    ///     }
    ///     if path.has_fill() {
    ///         println!(" Filled");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_paths(&self, page_index: usize) -> Result<Vec<crate::elements::PathContent>> {
        use crate::content::{Operator, parse_content_stream_paths_only};
        use crate::elements::{LineCap, LineJoin};
        use crate::extractors::paths::{FillRule, PathExtractor, PathGraphicsStateStack};
        use crate::layout::Color;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        // Get content stream data — skip page on decode failure (Annex I) ~keep
        let content_data = match self.get_page_content_data(page_index) {
            Ok(data) => data,
            Err(error) => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "decode_page_path_content",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "returning empty paths after decode failure"
                );
                return Ok(Vec::new());
            }
        };

        let operators = match parse_content_stream_paths_only(&content_data) {
            Ok(ops) => ops,
            Err(error) => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "parse_page_paths",
                    page_index,
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "returning empty paths after parse failure"
                );
                return Ok(Vec::new());
            }
        };

        let mut extractor = PathExtractor::new();
        let mut state_stack = PathGraphicsStateStack::new();

        if let Some(resources) = page_dict.get("Resources") {
            let resolved_resources = if let Some(ref_obj) = resources.as_reference() {
                self.load_object(ref_obj)?
            } else {
                resources.clone()
            };
            extractor.set_resources(resolved_resources);
        }

        for op in operators {
            match op {
                Operator::SaveState => {
                    state_stack.save();
                }
                Operator::RestoreState => {
                    state_stack.restore();
                    extractor.update_from_path_state(state_stack.current());
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    let state = state_stack.current_mut();
                    let new_matrix = crate::content::Matrix { a, b, c, d, e, f };
                    // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                    state.ctm = new_matrix.multiply(&state.ctm);
                    extractor.set_ctm(state.ctm);
                }

                Operator::SetStrokeRgb { r, g, b } => {
                    state_stack.current_mut().stroke_color_rgb = (r, g, b);
                    extractor.set_stroke_color(Color::new(r, g, b));
                }
                Operator::SetStrokeGray { gray } => {
                    state_stack.current_mut().stroke_color_rgb = (gray, gray, gray);
                    extractor.set_stroke_color(Color::new(gray, gray, gray));
                }
                Operator::SetStrokeCmyk { c, m, y, k } => {
                    let (r, g, b) = crate::color::cmyk_to_rgb(c, m, y, k);
                    state_stack.current_mut().stroke_color_rgb = (r, g, b);
                    extractor.set_stroke_color(Color::new(r, g, b));
                }

                Operator::SetFillRgb { r, g, b } => {
                    state_stack.current_mut().fill_color_rgb = (r, g, b);
                    extractor.set_fill_color(Color::new(r, g, b));
                }
                Operator::SetFillGray { gray } => {
                    state_stack.current_mut().fill_color_rgb = (gray, gray, gray);
                    extractor.set_fill_color(Color::new(gray, gray, gray));
                }
                Operator::SetFillCmyk { c, m, y, k } => {
                    let (r, g, b) = crate::color::cmyk_to_rgb(c, m, y, k);
                    state_stack.current_mut().fill_color_rgb = (r, g, b);
                    extractor.set_fill_color(Color::new(r, g, b));
                }

                Operator::SetLineWidth { width } => {
                    state_stack.current_mut().line_width = width;
                    extractor.set_line_width(width);
                }
                Operator::SetLineCap { cap_style } => {
                    state_stack.current_mut().line_cap = cap_style;
                    let cap = match cap_style {
                        1 => LineCap::Round,
                        2 => LineCap::Square,
                        _ => LineCap::Butt,
                    };
                    extractor.set_line_cap(cap);
                }
                Operator::SetExtGState { dict_name } => {
                    self.apply_path_extgstate(&dict_name, &mut extractor, &mut state_stack);
                }
                Operator::SetDash { array, phase } => {
                    state_stack.current_mut().dash = (array.clone(), phase);
                    extractor.set_dash(array, phase);
                }
                Operator::SetLineJoin { join_style } => {
                    state_stack.current_mut().line_join = join_style;
                    let join = match join_style {
                        1 => LineJoin::Round,
                        2 => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    };
                    extractor.set_line_join(join);
                }

                Operator::MoveTo { x, y } => {
                    extractor.move_to(x, y);
                }
                Operator::LineTo { x, y } => {
                    extractor.line_to(x, y);
                }
                Operator::CurveTo { x1, y1, x2, y2, x3, y3 } => {
                    extractor.curve_to(x1, y1, x2, y2, x3, y3);
                }
                Operator::CurveToV { x2, y2, x3, y3 } => {
                    extractor.curve_to_v(x2, y2, x3, y3);
                }
                Operator::CurveToY { x1, y1, x3, y3 } => {
                    extractor.curve_to_y(x1, y1, x3, y3);
                }
                Operator::Rectangle { x, y, width, height } => {
                    extractor.rectangle(x, y, width, height);
                }
                Operator::ClosePath => {
                    extractor.close_path();
                }

                Operator::Stroke => {
                    extractor.stroke();
                }
                Operator::Fill => {
                    extractor.fill(FillRule::NonZero);
                }
                Operator::FillEvenOdd => {
                    extractor.fill(FillRule::EvenOdd);
                }
                Operator::CloseStroke => {
                    extractor.close_and_stroke();
                }
                Operator::CloseFillStroke => {
                    extractor.close_fill_and_stroke(FillRule::NonZero);
                }
                Operator::FillStroke => {
                    extractor.fill_and_stroke(FillRule::NonZero);
                }
                Operator::FillStrokeEvenOdd => {
                    extractor.fill_and_stroke(FillRule::EvenOdd);
                }
                Operator::CloseFillStrokeEvenOdd => {
                    extractor.close_fill_and_stroke(FillRule::EvenOdd);
                }
                Operator::EndPath => {
                    extractor.end_path();
                }

                Operator::ClipNonZero => {
                    extractor.clip_non_zero();
                }
                Operator::ClipEvenOdd => {
                    extractor.clip_even_odd();
                }

                Operator::Do { name } => {
                    if let Err(error) = self.process_form_xobject_paths(&name, &mut extractor, &mut state_stack) {
                        tracing::warn!(target: LOG_TARGET,
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "failed to process XObject in path extraction"
                        );
                    }
                }

                // Marked content operators — maintain the active Optional
                // Content Group (PDF "layer") so each finalized path gets
                // tagged with the OCG it was emitted under. Per ISO 32000-1
                // §14.6, every `BDC`/`BMC` must be balanced by an `EMC`,
                // so we always push (with `None` for non-`/OC` tags) and
                // always pop — keeps the stack depth in sync with the
                // marked-content nesting. ~keep
                Operator::BeginMarkedContent { .. } => {
                    extractor.push_oc_layer(None);
                }
                Operator::BeginMarkedContentDict { tag, properties } => {
                    let layer = if tag == "OC" {
                        self.resolve_oc_layer_name(extractor.current_resources(), &properties)
                    } else {
                        None
                    };
                    extractor.push_oc_layer(layer);
                }
                Operator::EndMarkedContent => {
                    extractor.pop_oc_layer();
                }

                _ => {}
            }
        }

        Ok(extractor.finish())
    }

    /// Resolve a `BDC /OC <properties>` property operand to the human-readable
    /// layer name of the Optional Content it refers to (PDF spec
    /// ISO 32000-1:2008 §8.11, §14.6).
    ///
    /// `properties` is the operand parsed by `Operator::BeginMarkedContentDict`
    /// — per spec it is either:
    ///
    /// 1. An inline dictionary: an OCG (or OCMD) — read its name directly.
    /// 2. A name (e.g. `/MC0`) that references `<resources> /Properties
    ///    <name>` → an OCG or OCMD dictionary → read its name.
    ///
    /// `resources` is the resource dictionary currently in scope: the page
    /// `/Resources` at page level, or the active Form XObject's own
    /// `/Resources` when extracting inside an XObject (§14.6.2, §8.10.1).
    ///
    /// Returns `None` for malformed PDFs, missing `/Resources /Properties`
    /// entries, or optional-content objects without a resolvable name.
    /// Callers treat `None` as "path belongs to no named layer" — extraction
    /// continues normally.
    pub(super) fn resolve_oc_layer_name(
        &self,
        resources: Option<&crate::object::Object>,
        properties: &crate::object::Object,
    ) -> Option<String> {
        const OC_NAME_MAX_DEPTH: u8 = 8;

        if let Some(dict) = properties.as_dict() {
            return self.read_oc_name(dict, OC_NAME_MAX_DEPTH);
        }

        let prop_name = properties.as_name()?;
        let resources_obj = self.deref_object(resources?)?;
        let properties_dict = resources_obj.as_dict()?.get("Properties")?;
        let properties_obj = self.deref_object(properties_dict)?;
        let target = properties_obj.as_dict()?.get(prop_name)?;
        let target_obj = self.deref_object(target)?;
        self.read_oc_name(target_obj.as_dict()?, OC_NAME_MAX_DEPTH)
    }

    /// Read the human-readable layer name from an Optional Content dictionary.
    ///
    /// - An **OCG** (§8.11.2.1) carries its label in `/Name` — a PDF *text
    ///   string*, decoded via [`Self::decode_pdf_text_string`] so
    ///   PDFDocEncoding (Annex D) and UTF-16 (BE/LE, with BOM) layer names
    ///   round-trip identically to the rest of the library.
    /// - An **OCMD** (§8.11.3.2, Table 99) has no `/Name` of its own; its
    ///   member OCGs live in `/OCGs`, which is *either* a single OCG *or* an
    ///   array of them (array entries may be `null`). We follow the first
    ///   entry that resolves to a dictionary and read its name.
    ///
    /// `depth` bounds the `/OCGs` chain so a malformed PDF whose membership
    /// dictionary points back to another OCMD cannot recurse forever.
    /// Returns `None` for missing / non-dictionary / nameless inputs — the
    /// path is simply left unlabelled.
    pub(super) fn read_oc_name(
        &self,
        dict: &std::collections::HashMap<String, crate::object::Object>,
        depth: u8,
    ) -> Option<String> {
        use crate::object::Object;

        if depth == 0 {
            return None;
        }

        if matches!(dict.get("Type").and_then(|t| t.as_name()), Some("OCMD")) {
            let ocgs = self.deref_object(dict.get("OCGs")?)?;
            let first_ocg = match ocgs.as_array() {
                Some(entries) => entries
                    .iter()
                    .find_map(|e| self.deref_object(e).filter(|o| o.as_dict().is_some())),
                None => Some(ocgs.clone()),
            };
            return self.read_oc_name(first_ocg?.as_dict()?, depth - 1);
        }

        match dict.get("Name")? {
            Object::String(bytes) => Some(Self::decode_pdf_text_string(bytes)),
            // Tolerate a /Name written as a PDF name object (non-conformant,
            // but seen in real exports). ~keep
            Object::Name(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn apply_path_extgstate(
        &self,
        name: &str,
        extractor: &mut crate::extractors::paths::PathExtractor,
        stack: &mut crate::extractors::paths::PathGraphicsStateStack,
    ) {
        extractor.prepare_path_extgstates(name, self.object_recovery_epoch.load(Ordering::Acquire), |reference| {
            self.path_reference_available(reference)
        });
        if extractor.apply_cached_path_extgstate(name, stack.current_mut()) {
            return;
        }
        extractor.extgstate_container(
            |reference| self.path_reference_available(reference),
            |reference| self.load_object(reference),
        );
        let patch = self.resolve_path_extgstate(name, extractor).unwrap_or_default();
        extractor.cache_path_extgstate(name, patch);
        extractor.apply_cached_path_extgstate(name, stack.current_mut());
    }

    fn resolve_path_extgstate(
        &self,
        name: &str,
        extractor: &crate::extractors::paths::PathExtractor,
    ) -> Option<crate::extractors::paths::PathGraphicsPatch> {
        let entry = extractor.current_resources()?.as_dict()?.get("ExtGState")?;
        let states = if entry.as_reference().is_some() {
            extractor.loaded_extgstate_container()?
        } else {
            entry
        };
        let dictionary = states
            .as_dict()
            .and_then(|states| states.get(name))
            .and_then(|state| self.path_state_object(state, extractor))?;
        let dictionary = dictionary.as_dict()?;
        Some(crate::extractors::paths::PathGraphicsPatch {
            dash: dictionary
                .get("D")
                .and_then(|dash| self.path_dash_pattern(dash, extractor)),
            stroke_alpha: dictionary
                .get("CA")
                .and_then(|alpha| self.path_stroke_alpha(alpha, extractor)),
        })
    }

    fn path_stroke_alpha(
        &self,
        object: &crate::object::Object,
        extractor: &crate::extractors::paths::PathExtractor,
    ) -> Option<f32> {
        let object = self.path_state_object(object, extractor)?;
        let number = object
            .as_real()
            .or_else(|| object.as_integer().map(|value| value as f64))?;
        // Clamp at PDF number precision so finite out-of-f32-range constants are not discarded. ~keep
        number.is_finite().then(|| number.clamp(0.0, 1.0) as f32)
    }

    fn path_state_number(
        &self,
        object: &crate::object::Object,
        extractor: &crate::extractors::paths::PathExtractor,
    ) -> Option<f32> {
        let object = self.path_state_object(object, extractor)?;
        let number = object
            .as_real()
            .or_else(|| object.as_integer().map(|value| value as f64))? as f32;
        number.is_finite().then_some(number)
    }

    fn path_dash_pattern(
        &self,
        object: &crate::object::Object,
        extractor: &crate::extractors::paths::PathExtractor,
    ) -> Option<(Vec<f32>, f32)> {
        let object = self.path_state_object(object, extractor)?;
        let [array, phase] = object.as_array()?.as_slice() else {
            return None;
        };
        let array = self.path_state_object(array, extractor)?;
        let lengths = array
            .as_array()?
            .iter()
            .map(|length| {
                self.path_state_number(length, extractor)
                    .filter(|length| *length >= 0.0)
            })
            .collect::<Option<Vec<_>>>()?;
        Some((lengths, self.path_state_number(phase, extractor)?))
    }

    fn path_state_object<'a>(
        &self,
        object: &'a crate::object::Object,
        extractor: &crate::extractors::paths::PathExtractor,
    ) -> Option<std::borrow::Cow<'a, crate::object::Object>> {
        match object.as_reference() {
            Some(reference) => extractor
                .load_path_object(
                    reference,
                    |reference| self.path_reference_available(reference),
                    |reference| self.load_object(reference),
                )
                .map(std::borrow::Cow::Owned),
            None => Some(std::borrow::Cow::Borrowed(object)),
        }
    }

    fn path_reference_available(&self, reference: crate::object::ObjectRef) -> bool {
        if self
            .object_cache
            .lock_or_recover()
            .get(&reference)
            .is_some_and(|object| !matches!(object, Object::Null))
        {
            return true;
        }
        let Some(entry) = self.xref.get(reference.id) else {
            return false;
        };
        if entry.entry_type != crate::xref::XRefEntryType::Compressed || reference.generation != 0 {
            return false;
        }
        let Ok(stream) = u32::try_from(entry.offset) else {
            return false;
        };
        self.object_stream_cache
            .lock_or_recover()
            .get(&crate::object::ObjectRef::new(stream, 0))
            .and_then(|objects| objects.get(&reference.id))
            .is_some_and(|object| !matches!(object, Object::Null))
    }

    /// Dereference one level of indirection, loading the target object;
    /// pass direct objects through unchanged. `None` if a reference fails to
    /// load — callers treat that as "unresolvable, leave unlabelled".
    fn deref_object(&self, obj: &crate::object::Object) -> Option<crate::object::Object> {
        match obj.as_reference() {
            Some(r) => self.load_object(r).ok(),
            None => Some(obj.clone()),
        }
    }

    /// Extract rectangles from a page.
    ///
    /// Identifies paths that form axis-aligned rectangles.
    pub fn extract_rects(&self, page_index: usize) -> Result<Vec<crate::elements::PathContent>> {
        let paths = self.extract_paths(page_index)?;
        Ok(paths.into_iter().filter(|p| p.is_rectangle()).collect())
    }

    /// Extract straight lines from a page.
    ///
    /// Identifies paths that form a single straight line segment.
    pub fn extract_lines(&self, page_index: usize) -> Result<Vec<crate::elements::PathContent>> {
        let paths = self.extract_paths(page_index)?;
        Ok(paths.into_iter().filter(|p| p.is_straight_line()).collect())
    }

    /// Process paths from a Form XObject.
    ///
    /// This method recursively extracts paths from Form XObjects encountered via the `Do` operator.
    /// It handles:
    /// - XObject resolution from resources
    /// - Type checking (Form vs Image)
    /// - Stream decoding and operator parsing
    /// - Coordinate transformations via /Matrix
    /// - Graphics state isolation
    ///
    /// # Arguments
    ///
    /// * `name` - The XObject name from the `Do` operator
    /// * `extractor` - The path extractor to accumulate paths
    /// * `state_stack` - The graphics state stack for transformations
    fn process_form_xobject_paths(
        &self,
        name: &str,
        extractor: &mut crate::extractors::paths::PathExtractor,
        state_stack: &mut crate::extractors::paths::PathGraphicsStateStack,
    ) -> Result<()> {
        use crate::content::{Matrix, Operator, parse_content_stream_paths_only};
        use crate::elements::{LineCap, LineJoin};
        use crate::extractors::paths::FillRule;
        use crate::layout::Color;

        let xobject_ref = match extractor.resolve_xobject_ref(name, |ref_obj| self.load_object(ref_obj)) {
            Some(r) => r,
            None => return Ok(()),
        };

        if !extractor.can_process_xobject(xobject_ref) {
            return Ok(());
        }
        extractor.push_xobject(xobject_ref);

        let xobject = match self.load_object(xobject_ref) {
            Ok(obj) => obj,
            Err(e) => {
                extractor.pop_xobject_failed();
                return Err(e);
            }
        };
        let xobject_dict = match xobject.as_dict() {
            Some(dict) => dict,
            None => {
                extractor.pop_xobject_failed();
                return Err(Error::ParseError {
                    offset: 0,
                    reason: "XObject is not a dictionary".to_string(),
                });
            }
        };

        match xobject_dict.get("Subtype") {
            Some(subtype_obj) => {
                if let Some(subtype_name) = subtype_obj.as_name() {
                    if subtype_name != "Form" {
                        extractor.pop_xobject();
                        return Ok(());
                    }
                } else {
                    extractor.pop_xobject();
                    return Ok(());
                }
            }
            None => {
                extractor.pop_xobject();
                return Ok(());
            }
        }

        // Decode stream — reuse document-level cache shared with text extraction. ~keep
        let cached_stream = { self.xobject_stream_cache.lock_or_recover().get(&xobject_ref).cloned() };
        let stream_data = if let Some(cached) = cached_stream {
            cached.as_ref().clone()
        } else {
            match self.decode_stream_with_encryption(&xobject, xobject_ref) {
                Ok(data) => {
                    const MAX_STREAM_CACHE_BYTES: usize = 50 * 1024 * 1024;
                    let current = self.xobject_stream_cache_bytes.load(Ordering::Relaxed);
                    if current + data.len() <= MAX_STREAM_CACHE_BYTES {
                        self.xobject_stream_cache_bytes
                            .store(current + data.len(), Ordering::Relaxed);
                        self.xobject_stream_cache
                            .lock_or_recover()
                            .insert(xobject_ref, std::sync::Arc::new(data.clone()));
                    }
                    data
                }
                Err(e) => {
                    extractor.pop_xobject_failed();
                    return Err(e);
                }
            }
        };

        let operators = match parse_content_stream_paths_only(&stream_data) {
            Ok(ops) => ops,
            Err(e) => {
                extractor.pop_xobject_failed();
                return Err(e);
            }
        };

        let matrix = if let Some(matrix_obj) = xobject_dict.get("Matrix") {
            if let Some(array) = matrix_obj.as_array() {
                if array.len() >= 6 {
                    let mut matrix = Matrix::identity();
                    let mut values = [0.0f32; 6];
                    let mut valid = true;

                    for (i, val) in array.iter().take(6).enumerate() {
                        let num = if let Some(f) = val.as_real() {
                            f as f32
                        } else if let Some(i_val) = val.as_integer() {
                            i_val as f32
                        } else {
                            valid = false;
                            break;
                        };
                        values[i] = num;
                    }

                    if valid {
                        matrix.a = values[0];
                        matrix.b = values[1];
                        matrix.c = values[2];
                        matrix.d = values[3];
                        matrix.e = values[4];
                        matrix.f = values[5];
                        matrix
                    } else {
                        Matrix::identity()
                    }
                } else {
                    Matrix::identity()
                }
            } else {
                Matrix::identity()
            }
        } else {
            Matrix::identity()
        };

        state_stack.save();

        if extractor.has_current_path() {
            extractor.end_path();
        }

        // Apply XObject transformation to CTM
        // PDF spec ISO 32000-1:2008 §8.10.1: Form XObject Matrix concatenates as M × CTM ~keep
        let state = state_stack.current_mut();
        state.ctm = matrix.multiply(&state.ctm);
        extractor.set_ctm(state.ctm);

        // Switch resource scope to this Form XObject's own /Resources, if any.
        // Form XObjects with their own Resources define a fresh XObject name
        // scope (ISO 32000-1 §8.10.1). Looking up nested `Do` names against the
        // parent scope can pick up unrelated sibling forms with colliding
        // names, which turns sibling Form XObjects into a cross-recursive tree
        // (O(N!) traversals and unbounded path accumulation). ~keep
        let saved_scope = if let Some(xobj_resources) = xobject_dict.get("Resources") {
            let resolved = if let Some(res_ref) = xobj_resources.as_reference() {
                self.load_object(res_ref).unwrap_or_else(|_| xobj_resources.clone())
            } else {
                xobj_resources.clone()
            };
            Some(extractor.swap_resources(Some(resolved)))
        } else {
            None
        };

        // Remember the marked-content nesting depth on entry so we can drop
        // anything this XObject leaves unbalanced (see truncate below). ~keep
        let oc_base_depth = extractor.oc_layer_depth();

        for op in operators {
            match op {
                Operator::SaveState => {
                    state_stack.save();
                }
                Operator::RestoreState => {
                    state_stack.restore();
                    extractor.update_from_path_state(state_stack.current());
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    let state = state_stack.current_mut();
                    let new_matrix = Matrix { a, b, c, d, e, f };
                    // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                    state.ctm = new_matrix.multiply(&state.ctm);
                    extractor.set_ctm(state.ctm);
                }

                // Color and line style operators — must update both state_stack
                // and extractor so q/Q save/restore works correctly. ~keep
                Operator::SetStrokeRgb { r, g, b } => {
                    state_stack.current_mut().stroke_color_rgb = (r, g, b);
                    extractor.set_stroke_color(Color::new(r, g, b));
                }
                Operator::SetStrokeGray { gray } => {
                    state_stack.current_mut().stroke_color_rgb = (gray, gray, gray);
                    extractor.set_stroke_color(Color::new(gray, gray, gray));
                }
                Operator::SetStrokeCmyk { c, m, y, k } => {
                    let (r, g, b) = crate::color::cmyk_to_rgb(c, m, y, k);
                    state_stack.current_mut().stroke_color_rgb = (r, g, b);
                    extractor.set_stroke_color(Color::new(r, g, b));
                }
                Operator::SetFillRgb { r, g, b } => {
                    state_stack.current_mut().fill_color_rgb = (r, g, b);
                    extractor.set_fill_color(Color::new(r, g, b));
                }
                Operator::SetFillGray { gray } => {
                    state_stack.current_mut().fill_color_rgb = (gray, gray, gray);
                    extractor.set_fill_color(Color::new(gray, gray, gray));
                }
                Operator::SetFillCmyk { c, m, y, k } => {
                    let (r, g, b) = crate::color::cmyk_to_rgb(c, m, y, k);
                    state_stack.current_mut().fill_color_rgb = (r, g, b);
                    extractor.set_fill_color(Color::new(r, g, b));
                }
                Operator::SetLineWidth { width } => {
                    state_stack.current_mut().line_width = width;
                    extractor.set_line_width(width);
                }
                Operator::SetLineCap { cap_style } => {
                    state_stack.current_mut().line_cap = cap_style;
                    let cap = match cap_style {
                        1 => LineCap::Round,
                        2 => LineCap::Square,
                        _ => LineCap::Butt,
                    };
                    extractor.set_line_cap(cap);
                }
                Operator::SetExtGState { dict_name } => {
                    self.apply_path_extgstate(&dict_name, extractor, state_stack);
                }
                Operator::SetDash { array, phase } => {
                    state_stack.current_mut().dash = (array.clone(), phase);
                    extractor.set_dash(array, phase);
                }
                Operator::SetLineJoin { join_style } => {
                    state_stack.current_mut().line_join = join_style;
                    let join = match join_style {
                        1 => LineJoin::Round,
                        2 => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    };
                    extractor.set_line_join(join);
                }

                Operator::MoveTo { x, y } => extractor.move_to(x, y),
                Operator::LineTo { x, y } => extractor.line_to(x, y),
                Operator::CurveTo { x1, y1, x2, y2, x3, y3 } => {
                    extractor.curve_to(x1, y1, x2, y2, x3, y3);
                }
                Operator::CurveToV { x2, y2, x3, y3 } => {
                    extractor.curve_to_v(x2, y2, x3, y3);
                }
                Operator::CurveToY { x1, y1, x3, y3 } => {
                    extractor.curve_to_y(x1, y1, x3, y3);
                }
                Operator::Rectangle { x, y, width, height } => {
                    extractor.rectangle(x, y, width, height);
                }
                Operator::ClosePath => extractor.close_path(),

                Operator::Stroke => extractor.stroke(),
                Operator::CloseStroke => extractor.close_and_stroke(),
                Operator::Fill => extractor.fill(FillRule::NonZero),
                Operator::FillEvenOdd => extractor.fill(FillRule::EvenOdd),
                Operator::CloseFillStroke => extractor.close_fill_and_stroke(FillRule::NonZero),
                Operator::FillStroke => extractor.fill_and_stroke(FillRule::NonZero),
                Operator::FillStrokeEvenOdd => extractor.fill_and_stroke(FillRule::EvenOdd),
                Operator::CloseFillStrokeEvenOdd => {
                    extractor.close_fill_and_stroke(FillRule::EvenOdd);
                }
                Operator::EndPath => extractor.end_path(),

                Operator::ClipNonZero => extractor.clip_non_zero(),
                Operator::ClipEvenOdd => extractor.clip_even_odd(),

                Operator::Do { name: nested_name } => {
                    if let Err(error) = self.process_form_xobject_paths(&nested_name, extractor, state_stack) {
                        tracing::warn!(target: LOG_TARGET,
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "failed to process nested XObject"
                        );
                    }
                }

                // Marked content — same Optional Content Group ("layer")
                // tracking as the page-level loop, but `/OC` property
                // references resolve against *this* XObject's resource scope
                // (swapped in above), per §14.6.2 + §8.10.1. CAD exports that
                // reuse Form XObjects for repeated symbols (gridline labels,
                // callouts) carry their `/OC` markers and local `/Properties`
                // here rather than on the page. ~keep
                Operator::BeginMarkedContent { .. } => {
                    extractor.push_oc_layer(None);
                }
                Operator::BeginMarkedContentDict { tag, properties } => {
                    let layer = if tag == "OC" {
                        self.resolve_oc_layer_name(extractor.current_resources(), &properties)
                    } else {
                        None
                    };
                    extractor.push_oc_layer(layer);
                }
                Operator::EndMarkedContent => {
                    extractor.pop_oc_layer();
                }

                _ => {}
            }
        }

        if extractor.has_current_path() {
            extractor.end_path();
        }

        extractor.truncate_oc_layers(oc_base_depth);

        if let Some(saved) = saved_scope {
            extractor.restore_resources(saved);
        }

        state_stack.restore();
        extractor.update_from_path_state(state_stack.current());

        extractor.pop_xobject();

        Ok(())
    }

    /// Extract paths from a specific rectangular region of a page.
    ///
    /// Only paths whose bounding box intersects the specified region are returned.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    /// * `region` - The rectangular region to extract from
    ///
    /// # Returns
    ///
    /// A vector of `PathContent` objects within the specified region.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # use xberg_native_pdf::geometry::Rect;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = PdfDocument::open("example.pdf")?;
    ///
    /// // Extract paths from a specific region (e.g., header area)
    /// let header_region = Rect::new(0.0, 700.0, 612.0, 92.0);
    /// let paths = doc.extract_paths_in_rect(0, header_region)?;
    ///
    /// println!("Found {} paths in header region", paths.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_paths_in_rect(
        &self,
        page_index: usize,
        region: crate::geometry::Rect,
    ) -> Result<Vec<crate::elements::PathContent>> {
        let paths = self.extract_paths(page_index)?;

        // Filter paths by region intersection against RENDERED extents: a
        // region query answers "what does the reader see here", so a rule
        // whose drawn bar crosses the region must match even when its
        // geometric bbox is a distant speck. Identical to the
        // geometric test for unstroked paths. ~keep
        Ok(paths
            .into_iter()
            .filter(|path| path.rendered_bbox().intersects(&region))
            .collect())
    }
}
