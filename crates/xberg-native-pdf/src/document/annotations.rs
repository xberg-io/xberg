//! Annotation, widget, and form-field text.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Parse font size from a /DA (Default Appearance) string.
    ///
    /// DA strings follow the format: `"/FontName size Tf ..."` (e.g., `"/Helv 12 Tf 0 g"`).
    /// Returns the font size preceding the `Tf` operator, or a default of 10.0 if not found.
    fn parse_font_size_from_da(da: &str) -> f32 {
        let tokens: Vec<&str> = da.split_whitespace().collect();
        for i in 0..tokens.len() {
            if tokens[i] == "Tf"
                && i > 0
                && let Ok(size) = tokens[i - 1].parse::<f32>()
                && size > 0.0
            {
                return size;
            }
        }
        10.0
    }

    /// Extract widget annotation values as TextSpans positioned at their /Rect locations.
    ///
    /// Converts each widget annotation's field value into a `TextSpan` with the annotation's
    /// bounding box. These spans merge naturally with content stream spans and get positioned
    /// correctly by existing layout algorithms.
    pub(super) fn extract_widget_spans(&self, page_index: usize) -> Vec<TextSpan> {
        use crate::extractors::forms::field_flags;
        use crate::geometry::Rect;

        let page_obj = match self.get_page(page_index) {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };
        let page_dict = match page_obj.as_dict() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let annots_arr = match page_dict.get("Annots") {
            Some(Object::Array(arr)) => arr.clone(),
            Some(Object::Reference(r)) => match self.load_object(*r) {
                Ok(Object::Array(arr)) => arr,
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        };

        let mut spans = Vec::new();
        let base_sequence = 1_000_000;
        // ~keep

        for (idx, annot_obj) in annots_arr.iter().enumerate() {
            let annot_ref = match annot_obj {
                Object::Reference(r) => *r,
                _ => continue,
            };
            let dict = match self.load_object(annot_ref) {
                Ok(obj) => match obj.as_dict() {
                    Some(d) => d.clone(),
                    None => continue,
                },
                Err(_) => continue,
            };

            let subtype = match dict.get("Subtype").and_then(|s| s.as_name()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            if !subtype.eq_ignore_ascii_case("widget") {
                continue;
            }

            // Check /F flags — skip invisible/hidden/noview annotations
            // Bit 1 (0x1) = Invisible, Bit 2 (0x2) = Hidden, Bit 6 (0x20) = NoView ~keep
            if let Some(Object::Integer(f)) = dict.get("F")
                && *f & (0x1 | 0x2 | 0x20) != 0
            {
                continue;
            }

            let rect = match dict.get("Rect") {
                Some(Object::Array(arr)) if arr.len() == 4 => {
                    let mut coords = [0.0f32; 4];
                    let mut ok = true;
                    for (i, item) in arr.iter().enumerate() {
                        match item {
                            Object::Integer(n) => coords[i] = *n as f32,
                            Object::Real(f) => coords[i] = *f as f32,
                            _ => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        continue;
                    }
                    let x = coords[0].min(coords[2]);
                    let y = coords[1].min(coords[3]);
                    let w = (coords[2] - coords[0]).abs();
                    let h = (coords[3] - coords[1]).abs();
                    if w < 0.1 || h < 0.1 {
                        continue;
                    }
                    Rect::new(x, y, w, h)
                }
                Some(Object::Reference(r)) => match self.load_object(*r) {
                    Ok(Object::Array(arr)) if arr.len() == 4 => {
                        let mut coords = [0.0f32; 4];
                        let mut ok = true;
                        for (i, item) in arr.iter().enumerate() {
                            match item {
                                Object::Integer(n) => coords[i] = *n as f32,
                                Object::Real(f) => coords[i] = *f as f32,
                                _ => {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        if !ok {
                            continue;
                        }
                        let x = coords[0].min(coords[2]);
                        let y = coords[1].min(coords[3]);
                        let w = (coords[2] - coords[0]).abs();
                        let h = (coords[3] - coords[1]).abs();
                        if w < 0.1 || h < 0.1 {
                            continue;
                        }
                        Rect::new(x, y, w, h)
                    }
                    _ => continue,
                },
                _ => continue,
            };

            let ft = dict
                .get("FT")
                .and_then(|o| o.as_name())
                .map(|s| s.to_string())
                .or_else(|| self.resolve_inherited_ft(&dict));

            let ff = dict
                .get("Ff")
                .and_then(|o| match o {
                    Object::Integer(i) => Some(*i as u32),
                    _ => None,
                })
                .or_else(|| self.resolve_inherited_ff(&dict));
            let ff = ff.unwrap_or(0);

            let display_text = match ft.as_deref() {
                Some("Tx") => {
                    if ff & field_flags::PASSWORD != 0 {
                        Some("********".to_string())
                    } else {
                        let value = Self::parse_string_value_static(dict.get("V"))
                            .or_else(|| self.resolve_inherited_field_value(&dict));
                        match value {
                            Some(v) if !v.trim().is_empty() => {
                                // Bound the value to the widget's visual
                                // capacity. Multi-line text-area fields
                                // can hold scrollable content far larger
                                // than the bbox visually renders; per
                                // spec §12.7.4.3 `/V` is the field's
                                // data, but `extract_text` semantics
                                // target what would be visible on the
                                // page. Truncate keeps the rendered
                                // portion and drops the overflow. ~keep
                                Some(Self::truncate_to_widget_capacity(v.trim().to_string(), &rect))
                            }
                            _ => {
                                // Fallback: try AP stream text. Truncate
                                // to bbox capacity — some PDFs reuse a
                                // single Form XObject for many widgets'
                                // `/AP /N`, pointing every widget's
                                // appearance at the page-background
                                // content; without the cap each widget
                                // would extract that content once. ~keep
                                self.extract_text_from_ap_stream(&dict).and_then(|t| {
                                    let t = t.trim().to_string();
                                    if t.is_empty() {
                                        return None;
                                    }
                                    Some(Self::truncate_to_widget_capacity(t, &rect))
                                })
                            }
                        }
                    }
                }
                Some("Btn") => {
                    if ff & field_flags::PUSH_BUTTON != 0 {
                        // Push button: caption is in /MK /CA per PDF Spec
                        // ISO 32000-1:2008 §12.5.6.19 (Appearance Characteristics
                        // Dictionary). Extracting it lets screen readers
                        // text-extraction consumers see the button label. ~keep
                        dict.get("MK")
                            .and_then(|mk| mk.as_dict())
                            .and_then(|mk| Self::parse_string_value_static(mk.get("CA")))
                            .and_then(|s| {
                                let t = s.trim().to_string();
                                if t.is_empty() { None } else { Some(t) }
                            })
                    } else {
                        let value = Self::parse_string_value_static(dict.get("V"))
                            .or_else(|| self.resolve_inherited_field_value(&dict));
                        let is_checked = match &value {
                            Some(v) => {
                                let v_lower = v.to_ascii_lowercase();
                                v_lower != "off" && !v_lower.is_empty()
                            }
                            None => false,
                        };
                        if is_checked {
                            Some("[x]".to_string())
                        } else {
                            // An UNCHECKED box carries no text. Emitting "[ ]"
                            // here injected noise that pdftotext/PyMuPDF never
                            // produce — the dominant cause of xberg-native-pdf being
                            // the sole outlier on AcroForm-heavy PDFs in the
                            // cross-corpus sweep (CORPUS-1). Emit nothing. ~keep
                            None
                        }
                    }
                }
                Some("Ch") => {
                    let value = dict.get("V");
                    match value {
                        Some(Object::Array(arr)) => {
                            let items: Vec<String> = arr
                                .iter()
                                .filter_map(|item| Self::parse_string_value_static(Some(item)))
                                .collect();
                            if items.is_empty() { None } else { Some(items.join(", ")) }
                        }
                        other => Self::parse_string_value_static(other)
                            .or_else(|| self.resolve_inherited_field_value(&dict))
                            .and_then(|v| {
                                let t = v.trim().to_string();
                                if t.is_empty() { None } else { Some(t) }
                            }),
                    }
                }
                Some("Sig") => None,
                _ => Self::parse_string_value_static(dict.get("V"))
                    .or_else(|| self.resolve_inherited_field_value(&dict))
                    .and_then(|v| {
                        let t = v.trim().to_string();
                        if t.is_empty() { None } else { Some(t) }
                    }),
            };

            let text = match display_text {
                Some(t) if !t.is_empty() => t,
                _ => {
                    // CORPUS-5: a widget with no extractable /V value (notably a
                    // signature field, /FT /Sig) often carries its VISIBLE text
                    // in the /AP/N appearance stream (e.g. "Firmato
                    // elettronicamente da ..."). pdftotext / PyMuPDF surface it;
                    // fall back to the appearance stream so it isn't dropped.
                    // Fields that DO yield a /V value take the arm above, so this
                    // never double-extracts. ~keep
                    match self.extract_text_from_ap_stream(&dict) {
                        Some(ap) if !ap.trim().is_empty() => ap.trim().to_string(),
                        _ => continue,
                    }
                }
            };

            let font_size = {
                let da = dict
                    .get("DA")
                    .and_then(|o| match o {
                        Object::String(s) => Some(Self::decode_pdf_text_string(s)),
                        _ => None,
                    })
                    .or_else(|| self.resolve_inherited_da(&dict));

                match da {
                    Some(da_str) => {
                        let size = Self::parse_font_size_from_da(&da_str);
                        if size <= 0.0 {
                            (rect.height * 0.7).clamp(6.0, 24.0)
                        } else {
                            size
                        }
                    }
                    None => (rect.height * 0.7).clamp(6.0, 24.0),
                }
            };

            spans.push(TextSpan {
                provenance: None,
                artifact_type: None,
                text,
                bbox: rect,
                font_name: String::new(),
                font_size,
                font_weight: crate::layout::text_block::FontWeight::Normal,
                is_italic: false,
                is_monospace: false,
                color: crate::layout::text_block::Color { r: 0.0, g: 0.0, b: 0.0 },
                mcid: None,
                mcid_scope: None,
                sequence: base_sequence + idx,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
                char_widths: vec![],
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: 0.0,
                wmode: 0,
                text_rise: 0.0,
                rtl_draw_logical: false,
                ligature_glyph: false,
                mirrored: false,
                page_rotation_applied: 0,
            });
        }

        spans
    }

    /// Build TextSpan objects from the /Contents field of content-bearing annotations.
    ///
    /// Sticky note (/Subtype/Text), FreeText, Stamp, and markup annotations carry
    /// human-readable text in their /Contents field. Widget annotations are already
    /// handled by `extract_widget_spans`; Popup annotations hold no independent
    /// content (their text belongs to the parent annotation).
    pub(super) fn annotation_content_spans(&self, page_index: usize) -> Vec<TextSpan> {
        use crate::geometry::Rect;

        let page_obj = match self.get_page(page_index) {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };
        let page_dict = match page_obj.as_dict() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let annots_arr = match page_dict.get("Annots") {
            Some(Object::Array(arr)) => arr.clone(),
            Some(Object::Reference(r)) => match self.load_object(*r) {
                Ok(Object::Array(arr)) => arr,
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        };

        let mut spans: Vec<TextSpan> = Vec::new();
        let base_sequence = 2_000_000usize; // sort after widget spans ~keep

        for (idx, annot_obj) in annots_arr.iter().enumerate() {
            let annot_ref = match annot_obj {
                Object::Reference(r) => *r,
                _ => continue,
            };
            let dict = match self.load_object(annot_ref) {
                Ok(obj) => match obj.as_dict() {
                    Some(d) => d.clone(),
                    None => continue,
                },
                Err(_) => continue,
            };

            let subtype = match dict.get("Subtype").and_then(|s| s.as_name()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let subtype_lc = subtype.to_ascii_lowercase();

            // Skip Widget (handled by extract_widget_spans) and Popup (no independent content).
            // ~keep
            if subtype_lc == "widget" || subtype_lc == "popup" {
                continue;
            }

            if let Some(Object::Integer(f)) = dict.get("F")
                && *f & (0x1 | 0x2 | 0x20) != 0
            {
                continue;
            }

            // Only FreeText and Stamp have /Contents representing visible page text.
            // Text (sticky-note) /Contents is reviewer comment text shown in a pop-up
            // window, not rendered on the page — exclude it to avoid injecting popup
            // notes into the body text stream.
            // For FreeText/Stamp: try /Contents first; fall back to AP stream so that
            // Stamp annotations with empty /Contents but a rendered AP stream are included. ~keep
            let is_visible = matches!(subtype_lc.as_str(), "freetext" | "stamp");
            if !is_visible {
                continue;
            }

            let text = {
                let from_contents = if let Some(Object::String(s)) = dict.get("Contents") {
                    let decoded = Self::decode_pdf_text_string(s).trim().to_string();
                    if decoded.is_empty() { None } else { Some(decoded) }
                } else {
                    None
                };
                if let Some(t) = from_contents {
                    t
                } else {
                    match self.extract_text_from_ap_stream(&dict) {
                        Some(ap_text) if !ap_text.trim().is_empty() => ap_text.trim().to_string(),
                        _ => continue,
                    }
                }
            };

            let rect_obj = match dict.get("Rect") {
                Some(Object::Reference(r)) => match self.load_object(*r) {
                    Ok(o) => o,
                    Err(_) => continue,
                },
                Some(o) => o.clone(),
                None => continue,
            };
            let rect = match rect_obj.as_array() {
                Some(arr) if arr.len() == 4 => {
                    let mut coords = [0.0f32; 4];
                    let mut ok = true;
                    for (i, item) in arr.iter().enumerate() {
                        match item {
                            Object::Integer(n) => coords[i] = *n as f32,
                            Object::Real(f) => coords[i] = *f as f32,
                            _ => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        continue;
                    }
                    let x = coords[0].min(coords[2]);
                    let y = coords[1].min(coords[3]);
                    let w = (coords[2] - coords[0]).abs();
                    let h = (coords[3] - coords[1]).abs();
                    Rect {
                        x,
                        y,
                        width: w.max(1.0),
                        height: h.max(1.0),
                    }
                }
                _ => continue,
            };

            spans.push(TextSpan {
                provenance: None,
                artifact_type: None,
                text,
                bbox: rect,
                font_name: String::new(),
                font_size: 12.0,
                font_weight: crate::layout::text_block::FontWeight::Normal,
                is_italic: false,
                is_monospace: false,
                color: crate::layout::text_block::Color { r: 0.0, g: 0.0, b: 0.0 },
                mcid: None,
                mcid_scope: None,
                sequence: base_sequence + idx,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
                char_widths: vec![],
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: 0.0,
                wmode: 0,
                text_rise: 0.0,
                rtl_draw_logical: false,
                ligature_glyph: false,
                mirrored: false,
                page_rotation_applied: 0,
            });
        }

        spans
    }

    /// Walk /Parent chain to find inherited /Ff (field flags) value.
    fn resolve_inherited_ff(&self, dict: &std::collections::HashMap<String, Object>) -> Option<u32> {
        let mut parent_ref = match dict.get("Parent") {
            Some(Object::Reference(r)) => Some(*r),
            _ => return None,
        };
        let mut depth = 0;
        while let Some(pref) = parent_ref {
            if depth >= 10 {
                break;
            }
            depth += 1;
            match self.load_object(pref) {
                Ok(parent_obj) => {
                    if let Some(parent_dict) = parent_obj.as_dict() {
                        if let Some(Object::Integer(ff)) = parent_dict.get("Ff") {
                            return Some(*ff as u32);
                        }
                        parent_ref = match parent_dict.get("Parent") {
                            Some(Object::Reference(r)) => Some(*r),
                            _ => None,
                        };
                    } else {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        None
    }

    /// Walk /Parent chain (and AcroForm) to find inherited /DA (Default Appearance) string.
    fn resolve_inherited_da(&self, dict: &std::collections::HashMap<String, Object>) -> Option<String> {
        let mut parent_ref = match dict.get("Parent") {
            Some(Object::Reference(r)) => Some(*r),
            _ => None,
        };
        let mut depth = 0;
        while let Some(pref) = parent_ref {
            if depth >= 10 {
                break;
            }
            depth += 1;
            match self.load_object(pref) {
                Ok(parent_obj) => {
                    if let Some(parent_dict) = parent_obj.as_dict() {
                        if let Some(Object::String(da)) = parent_dict.get("DA") {
                            return Some(Self::decode_pdf_text_string(da));
                        }
                        parent_ref = match parent_dict.get("Parent") {
                            Some(Object::Reference(r)) => Some(*r),
                            _ => None,
                        };
                    } else {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }

        if let Some(trailer_dict) = self.trailer.as_dict()
            && let Some(root_ref) = trailer_dict.get("Root").and_then(|o| o.as_reference())
            && let Ok(root_obj) = self.load_object(root_ref)
            && let Some(root_dict) = root_obj.as_dict()
        {
            let acroform = match root_dict.get("AcroForm") {
                Some(Object::Reference(r)) => self.load_object(*r).ok(),
                Some(obj) => Some(obj.clone()),
                None => None,
            };
            if let Some(acroform_obj) = acroform
                && let Some(af_dict) = acroform_obj.as_dict()
                && let Some(Object::String(da)) = af_dict.get("DA")
            {
                return Some(Self::decode_pdf_text_string(da));
            }
        }

        None
    }

    /// Append text from non-widget annotations on a page.
    ///
    /// Extracts text from FreeText annotations (text box contents), Stamp annotations
    /// (appearance stream text), and other non-widget annotation types.
    /// Widget annotations are handled separately via `extract_widget_spans()`.
    /// Skips hidden and invisible annotations per PDF spec flags.
    pub(super) fn append_non_widget_annotation_text(&self, page_index: usize, text: &mut String) {
        // Lightweight annotation text extraction — avoids full get_annotations() overhead.
        // Only reads /Subtype, /V, /Contents, /F, and /Parent (for field value inheritance).
        // Uses get_page() which is cached after first access. ~keep
        let page_obj = match self.get_page(page_index) {
            Ok(o) => o,
            Err(_) => return,
        };
        let page_dict = match page_obj.as_dict() {
            Some(d) => d,
            None => return,
        };

        let annots_arr = match page_dict.get("Annots") {
            Some(Object::Array(arr)) => arr.clone(),
            Some(Object::Reference(r)) => match self.load_object(*r) {
                Ok(Object::Array(arr)) => arr,
                _ => return,
            },
            _ => return,
        };

        let mut annot_texts: Vec<String> = Vec::new();

        for annot_obj in &annots_arr {
            let len_before_annot = annot_texts.len();
            let annot_ref = match annot_obj {
                Object::Reference(r) => *r,
                _ => continue,
            };
            let dict = match self.load_object(annot_ref) {
                Ok(obj) => match obj.as_dict() {
                    Some(d) => d.clone(),
                    None => continue,
                },
                Err(_) => continue,
            };

            // Check /F flags — skip invisible/hidden annotations
            // Bit 1 (0x1) = Invisible, Bit 2 (0x2) = Hidden, Bit 6 (0x20) = NoView ~keep
            if let Some(Object::Integer(f)) = dict.get("F")
                && *f & (0x1 | 0x2 | 0x20) != 0
            {
                continue;
            }

            let subtype = match dict.get("Subtype").and_then(|s| s.as_name()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let subtype_lower = subtype.to_ascii_lowercase();

            match subtype_lower.as_str() {
                "widget" => {
                    // Widgets are now handled by extract_widget_spans() as inline TextSpans.
                    // Skip them here to avoid duplicate text at the end of output. ~keep
                    continue;
                }
                "freetext" | "stamp" => {
                    if let Some(Object::String(s)) = dict.get("Contents") {
                        let decoded = Self::decode_pdf_text_string(s);
                        let trimmed = decoded.trim().to_string();
                        if !trimmed.is_empty() {
                            annot_texts.push(trimmed);
                        }
                    }
                }
                // Text (sticky-note) /Contents is reviewer popup comment text, not
                // visible page content — skip to avoid injecting popup notes. ~keep
                "text" => {}
                // Geometric shape annotations — per §12.5.6.2, their /Contents is
                // also popup/comment text, same as the markup group below. ~keep
                "line" | "circle" | "square" | "polygon" | "polyline" => {}
                // Markup/comment annotations — per ISO 32000-1 §12.5.6.2 (Table 166),
                // the /Contents of all these subtypes is popup/comment text written
                // by a reviewer, NOT text displayed on the page. Exclude to avoid
                // injecting user annotation notes into the body text stream.
                // Per §12.5.6.2, all of these annotations' /Contents is popup/comment
                // text (displayed in a pop-up window), not rendered page content.
                // FileAttachment is explicitly in this category per §12.5.6.2 even
                // though §12.5.6.15 calls it "descriptive text" — the pop-up semantics
                // take precedence. ~keep
                "highlight" | "underline" | "strikeout" | "squiggly" | "caret" | "fileattachment" | "redact"
                | "ink" => {}
                // Link /Contents is an accessibility alternate description (§12.5.6.5).
                // Treated as supplementary text on pages with no body content. ~keep
                "link" => {
                    if let Some(Object::String(s)) = dict.get("Contents") {
                        let decoded = Self::decode_pdf_text_string(s);
                        let trimmed = decoded.trim().to_string();
                        if !trimmed.is_empty() {
                            annot_texts.push(trimmed);
                        }
                    }
                }
                // Popup annotations — per §12.5.6.14 Table 183, the parent
                // annotation's /Contents overrides the popup's own /Contents. ~keep
                "popup" => {
                    let mut got_text = false;
                    if let Some(parent_ref) = dict.get("Parent").and_then(|o| o.as_reference())
                        && let Ok(parent_obj) = self.load_object(parent_ref)
                        && let Some(parent_dict) = parent_obj.as_dict()
                        && let Some(Object::String(s)) = parent_dict.get("Contents")
                    {
                        let decoded = Self::decode_pdf_text_string(s);
                        let trimmed = decoded.trim().to_string();
                        if !trimmed.is_empty() {
                            annot_texts.push(trimmed);
                            got_text = true;
                        }
                    }
                    if !got_text && let Some(Object::String(s)) = dict.get("Contents") {
                        let decoded = Self::decode_pdf_text_string(s);
                        let trimmed = decoded.trim().to_string();
                        if !trimmed.is_empty() {
                            annot_texts.push(trimmed);
                        }
                    }
                }
                _ => {
                    if let Some(Object::String(s)) = dict.get("Contents") {
                        let decoded = Self::decode_pdf_text_string(s);
                        let trimmed = decoded.trim().to_string();
                        if !trimmed.is_empty() {
                            annot_texts.push(trimmed);
                        }
                    }
                }
            }

            let text_before = annot_texts.len();
            if text_before == len_before_annot
                && let Some(ap_text) = self.extract_text_from_ap_stream(&dict)
            {
                let trimmed = ap_text.trim().to_string();
                if !trimmed.is_empty() {
                    annot_texts.push(trimmed);
                }
            }
        }

        if !annot_texts.is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&annot_texts.join("\n"));
        }
    }

    /// Extract text from an annotation's Normal Appearance stream (/AP/N).
    ///
    /// AP streams are content streams with their own /Resources. This creates
    /// a temporary TextExtractor, loads fonts from the AP stream resources,
    /// and extracts text spans from the decoded stream data.
    pub fn extract_annotation_appearance_text(
        &self,
        annotation: &crate::annotations::Annotation,
        excluded_layers: &std::collections::HashSet<String>,
    ) -> Option<String> {
        self.extract_text_from_ap_stream_filtered(annotation.raw_dict.as_ref()?, excluded_layers)
    }

    fn extract_text_from_ap_stream(&self, annot_dict: &std::collections::HashMap<String, Object>) -> Option<String> {
        self.extract_text_from_ap_stream_filtered(annot_dict, &std::collections::HashSet::new())
    }

    fn extract_text_from_ap_stream_filtered(
        &self,
        annot_dict: &std::collections::HashMap<String, Object>,
        excluded_layers: &std::collections::HashSet<String>,
    ) -> Option<String> {
        use crate::extractors::TextExtractor;

        let ap_obj = annot_dict.get("AP")?;
        let ap = if let Some(r) = ap_obj.as_reference() {
            self.load_object(r).ok()?
        } else {
            ap_obj.clone()
        };
        let ap_dict = ap.as_dict()?;

        let n_obj = ap_dict.get("N")?;
        let (n_stream, n_ref) = match n_obj {
            Object::Reference(r) => (self.load_object(*r).ok()?, *r),
            _ => return None,
        };

        let n_dict = n_stream.as_dict()?;

        let stream_data = match self.decode_stream_with_encryption(&n_stream, n_ref) {
            Ok(data) => data,
            Err(_) => return None,
        };

        if !Self::may_contain_text(&stream_data) {
            return None;
        }

        let mut extractor = TextExtractor::new();
        if !excluded_layers.is_empty() {
            extractor.set_excluded_layers(excluded_layers.clone());
        }

        // Load fonts from the AP/N stream's own /Resources. No resources on
        // the AP stream — try the annotation's /DR or parent page resources
        // — means we can't decode fonts, so bail. ~keep
        {
            let resources = n_dict.get("Resources")?;
            let res_obj = if let Some(r) = resources.as_reference() {
                self.load_object(r).ok().unwrap_or_else(|| resources.clone())
            } else {
                resources.clone()
            };
            extractor.set_resources(res_obj.clone());
            extractor.set_document(self);
            let _ = self.load_fonts(&res_obj, &mut extractor);
        }

        let spans = extractor.extract_text_spans(&stream_data).ok()?;
        if spans.is_empty() {
            return None;
        }

        let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        if text.trim().is_empty() {
            return None;
        }
        Some(text)
    }

    /// Char-count capacity for what physically fits inside a widget
    /// bbox at body font sizes. Per PDF spec §12.7.4.3 the field's
    /// value is `/V`; the appearance stream is visual rendering
    /// only. When we fall back to AP extraction the result must be
    /// bounded by what the widget could visually show — PDFs that
    /// reuse a single Form XObject for many widgets' `/AP /N` would
    /// otherwise dump the shared content once per widget, and
    /// scrollable multi-line text fields hold far more characters
    /// in `/V` than ever render at once.
    ///
    /// Heuristic: ~14 chars per cm² at body font sizes. At PDF
    /// 72 dpi (1 pt = 0.0353 cm), the formula
    /// `capacity = 0.0175 * w_pt * h_pt + 64` applies; the constant
    /// term absorbs short labels where the area estimate alone is
    /// too tight to even hold the field's name.
    fn widget_text_capacity(bbox: &crate::geometry::Rect) -> usize {
        let area = bbox.width.max(0.0) * bbox.height.max(0.0);
        (0.0175 * area) as usize + 64
    }

    /// Truncate `text` to the widget's visual capacity. If `text`
    /// already fits, returns it unchanged. Used to bound AP-fallback
    /// extraction (and other content paths) so a single widget can't
    /// dump page-background prose or scrollable field internals into
    /// the page text.
    fn truncate_to_widget_capacity(text: String, bbox: &crate::geometry::Rect) -> String {
        let cap = Self::widget_text_capacity(bbox);
        let n = text.chars().count();
        if n <= cap {
            return text;
        }
        text.chars().take(cap).collect()
    }

    /// Walk /Parent chain to find inherited /FT (field type) value.
    fn resolve_inherited_ft(&self, dict: &std::collections::HashMap<String, Object>) -> Option<String> {
        let mut parent_ref = match dict.get("Parent") {
            Some(Object::Reference(r)) => Some(*r),
            _ => return None,
        };
        let mut depth = 0;
        while let Some(pref) = parent_ref {
            if depth >= 10 {
                break;
            }
            depth += 1;
            match self.load_object(pref) {
                Ok(parent_obj) => {
                    if let Some(parent_dict) = parent_obj.as_dict() {
                        if let Some(ft) = parent_dict.get("FT").and_then(|o| o.as_name()) {
                            return Some(ft.to_string());
                        }
                        parent_ref = match parent_dict.get("Parent") {
                            Some(Object::Reference(r)) => Some(*r),
                            _ => None,
                        };
                    } else {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        None
    }

    /// Walk /Parent chain to find inherited /V value (PDF spec 12.7.3.1).
    fn resolve_inherited_field_value(&self, dict: &std::collections::HashMap<String, Object>) -> Option<String> {
        let mut parent_ref = match dict.get("Parent") {
            Some(Object::Reference(r)) => Some(*r),
            _ => return None,
        };
        let mut depth = 0;
        while let Some(pref) = parent_ref {
            if depth >= 10 {
                break;
            }
            depth += 1;
            match self.load_object(pref) {
                Ok(parent_obj) => {
                    if let Some(parent_dict) = parent_obj.as_dict() {
                        if let Some(v) = Self::parse_string_value_static(parent_dict.get("V")) {
                            return Some(v);
                        }
                        parent_ref = match parent_dict.get("Parent") {
                            Some(Object::Reference(r)) => Some(*r),
                            _ => None,
                        };
                    } else {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        None
    }

    /// Parse a string value from a PDF object with proper PDF string decoding.
    /// Handles UTF-16BE (BOM \xFE\xFF) and PDFDocEncoding per ISO 32000-1 §7.9.2.2.
    pub(super) fn parse_string_value_static(obj: Option<&Object>) -> Option<String> {
        match obj {
            Some(Object::String(s)) => Some(Self::decode_pdf_text_string(s)),
            Some(Object::Name(n)) => Some(n.clone()),
            Some(Object::Integer(i)) => Some(i.to_string()),
            Some(Object::Real(f)) => Some(f.to_string()),
            _ => None,
        }
    }

    /// Decode a PDF text string that may be UTF-16BE/LE (with BOM) or PDFDocEncoding.
    pub(super) fn decode_pdf_text_string(bytes: &[u8]) -> String {
        if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
            let utf16_bytes = &bytes[2..];
            let utf16_pairs: Vec<u16> = utf16_bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16(&utf16_pairs).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
        } else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
            let utf16_bytes = &bytes[2..];
            let utf16_pairs: Vec<u16> = utf16_bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16(&utf16_pairs).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
        } else {
            bytes
                .iter()
                .filter_map(|&b| crate::fonts::font_dict::pdfdoc_encoding_lookup(b))
                .collect()
        }
    }

    /// Strip XHTML tags from rich content (/RC) to extract plain text.
    ///
    /// Per PDF Spec ISO 32000-1:2008 Section 12.7.3.4, /RC entries contain
    /// XHTML-formatted rich text. This method strips tags to produce plain text.
    #[cfg(test)]
    pub(super) fn strip_xhtml_tags(xhtml: &str) -> String {
        let mut result = String::with_capacity(xhtml.len());
        let mut inside_tag = false;
        for ch in xhtml.chars() {
            match ch {
                '<' => inside_tag = true,
                '>' => inside_tag = false,
                _ if !inside_tag => result.push(ch),
                _ => {}
            }
        }
        result
    }
}
