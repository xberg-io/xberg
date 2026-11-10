//! Shared parser for PDF `ExtGState` dictionary entries.
//!
//! The page renderer and the separation-plate renderer apply transparency and
//! blend-mode overrides from `gs` operators; path extraction reads the dash
//! pattern and stroke opacity. Keeping the parser in a single module avoids
//! drift between the three readers.

use crate::content::graphics_state::{GraphicsState, SoftMaskForm, SoftMaskSubtype};
use crate::document::PdfDocument;
use crate::error::Result;
use crate::object::Object;

/// How the parser follows a reference. The renderers resolve through the document; path
/// extraction resolves through its failure ledger, so a failed load is recorded and not
/// repeated until recovery publishes the object.
pub(crate) trait ObjectResolver {
    fn resolve_object(&self, obj: &Object) -> Result<Object>;
}

impl ObjectResolver for PdfDocument {
    fn resolve_object(&self, obj: &Object) -> Result<Object> {
        PdfDocument::resolve_object(self, obj)
    }
}

/// A parsed `/SMask` value from an ExtGState dict (§11.4.7 / Table 144).
/// `None` corresponds to the spec `/None` value (clear the current mask);
/// `Form` carries the Form XObject reference plus optional backdrop and
/// transfer function (see [`SoftMaskForm`]).
#[derive(Clone, Debug)]
pub(crate) enum SoftMaskValue {
    /// `/SMask /None` — clear the current mask.
    None,
    /// `/SMask <<` … Form-XObject soft mask.
    Form(SoftMaskForm),
}

/// Parsed effects of a PDF `ExtGState` dictionary: fill/stroke alpha, blend
/// mode, the overprint parameters from ISO 32000-1 §11.7.4, §11.4.7
/// Form-XObject soft masks and the dash pattern. [`ParsedExtGState::apply`]
/// leaves the dash to path extraction, which reads it from this struct.
#[derive(Clone, Debug, Default)]
pub(crate) struct ParsedExtGState {
    pub(crate) fill_alpha: Option<f32>,
    pub(crate) stroke_alpha: Option<f32>,
    pub(crate) blend_mode: Option<String>,
    /// Overprint for stroking operations (ExtGState `/OP`, §11.7.4).
    pub(crate) stroke_overprint: Option<bool>,
    /// Overprint for non-stroking operations (ExtGState `/op`, §11.7.4).
    pub(crate) fill_overprint: Option<bool>,
    /// Overprint mode (ExtGState `/OPM`, §11.7.4). 0 = standard, 1 = nonzero.
    pub(crate) overprint_mode: Option<u8>,
    /// Soft mask dispatch (§11.4.7). `None` means the entry was absent —
    /// gs.smask is left untouched. `Some(SoftMaskValue::None)` is the
    /// spec `/None` value (clear). `Some(SoftMaskValue::Form(..))` is a
    /// Form-XObject mask.
    pub(crate) smask: Option<SoftMaskValue>,
    /// Dash pattern (ExtGState `/D`, Table 57): on/off lengths and phase.
    /// `None` when the entry is absent or malformed.
    pub(crate) dash: Option<(Vec<f32>, f32)>,
}

impl ParsedExtGState {
    /// Apply this dictionary's fields to `gs`. Fields that were not present
    /// in the source dictionary are left untouched on `gs`.
    pub(crate) fn apply(&self, gs: &mut GraphicsState) {
        if let Some(a) = self.fill_alpha {
            gs.fill_alpha = a;
        }
        if let Some(a) = self.stroke_alpha {
            gs.stroke_alpha = a;
        }
        if let Some(ref m) = self.blend_mode {
            gs.blend_mode = m.clone();
        }
        if let Some(v) = self.fill_overprint {
            gs.fill_overprint = v;
        }
        if let Some(v) = self.stroke_overprint {
            gs.stroke_overprint = v;
        }
        if let Some(v) = self.overprint_mode {
            gs.overprint_mode = v;
        }
        if let Some(ref sm) = self.smask {
            match sm {
                SoftMaskValue::None => {
                    gs.smask = None;
                }
                SoftMaskValue::Form(f) => {
                    gs.smask = Some(f.clone());
                }
            }
        }
    }
}

/// Parse the fields we need from an `ExtGState` *entry* (the inner dict, not
/// the resource dict that holds it). Resolves `state_obj` once if it is a
/// reference.
pub(crate) fn parse_ext_g_state_inner(state_obj: &Object, doc: &impl ObjectResolver) -> Result<ParsedExtGState> {
    let mut out = ParsedExtGState::default();
    let state_resolved = doc.resolve_object(state_obj)?;
    let state_dict = match state_resolved.as_dict() {
        Some(d) => d,
        None => return Ok(out),
    };

    // ISO 32000-1 §7.3.10: ANY direct object can be replaced by an indirect
    // reference. Reading `/ca 3 0 R` as-is yields a `Reference` whose typed
    // accessors all return None; the field would silently drop to its
    // default. Resolve every value once before classifying. `/SMask` has
    // its own resolve below because its dict entries need the same
    // treatment. ~keep
    let read = |key: &str| -> Option<Object> {
        let raw = state_dict.get(key)?;
        doc.resolve_object(raw).ok()
    };

    if let Some(ca) = read("ca") {
        out.fill_alpha = ca
            .as_real()
            .map(|v| v as f32)
            .or_else(|| ca.as_integer().map(|v| v as f32));
    }
    if let Some(ca_upper) = read("CA") {
        out.stroke_alpha = ca_upper
            .as_real()
            .map(|v| v as f32)
            .or_else(|| ca_upper.as_integer().map(|v| v as f32));
    }
    out.dash = read("D").and_then(|d| parse_dash(&d, doc));
    if let Some(bm) = read("BM") {
        out.blend_mode = Some(parse_blend_mode(&bm, doc));
    }

    // ISO 32000-1 §11.7.4 / Table 128. `/OP` is the stroking overprint;
    // `/op` (lowercase) is the non-stroking overprint. When `/OP` is
    // present without `/op`, the spec says it sets both. ~keep
    let op_stroke = read("OP").and_then(|v| v.as_bool());
    let op_fill = read("op").and_then(|v| v.as_bool());
    out.stroke_overprint = op_stroke;
    out.fill_overprint = op_fill.or(op_stroke);

    if let Some(opm) = read("OPM").and_then(|v| v.as_integer()) {
        // Spec defines only 0 (standard) and 1 (nonzero). Any other
        // value is undefined; clamp to 0 so a malformed PDF doesn't
        // accidentally enable nonzero-overprint mode. ~keep
        out.overprint_mode = Some(if opm == 1 { 1 } else { 0 });
    }

    // ISO 32000-1:2008 §11.4.7 / Table 144. `/SMask` is either the
    // name `/None` (clear the current soft mask) or a soft-mask
    // dictionary referencing a Form XObject. Image-attached soft
    // masks (via an image XObject's own /SMask entry) are handled
    // at the image-blit site; this parser covers the ExtGState
    // path. ~keep
    if let Some(smask_obj) = state_dict.get("SMask") {
        out.smask = parse_smask(smask_obj, doc);
    }

    Ok(out)
}

/// Read a number through references. Non-finite values are rejected.
fn finite_number(obj: &Object, doc: &impl ObjectResolver) -> Option<f32> {
    let obj = doc.resolve_object(obj).ok()?;
    let n = obj.as_real().or_else(|| obj.as_integer().map(|i| i as f64))? as f32;
    n.is_finite().then_some(n)
}

/// Classify an ExtGState `/D` value: `[dashArray phase]` with non-negative
/// lengths. Any malformed part rejects the whole pattern, so the caller keeps
/// the dash it already has. ~keep
fn parse_dash(d: &Object, doc: &impl ObjectResolver) -> Option<(Vec<f32>, f32)> {
    let [array, phase] = d.as_array()?.as_slice() else {
        return None;
    };
    let array = doc.resolve_object(array).ok()?;
    let lengths = array
        .as_array()?
        .iter()
        .map(|length| finite_number(length, doc).filter(|length| *length >= 0.0))
        .collect::<Option<Vec<_>>>()?;
    Some((lengths, finite_number(phase, doc)?))
}

/// Resolve a `/BM` value to a blend-mode name.
///
/// ISO 32000-1 §11.3.5 + §11.6.3: `/BM` may be a name OR an array of
/// names. For an array, "the first name that names a blend mode
/// supported by the conforming reader shall be used". Unrecognised
/// names fall back to `/Normal` per §11.6.3. The classifier in
/// `crate::rendering::sidecar::is_recognised_mode` enumerates every
/// standard mode from §11.3.5.2 + §11.3.5.3; we share that list so
/// detection and dispatch stay in lockstep.
///
/// Array elements may themselves be indirect refs (§7.3.10), so
/// each is resolved before pattern-matching its name. ~keep
fn parse_blend_mode(bm: &Object, doc: &impl ObjectResolver) -> String {
    match bm {
        Object::Name(n) if crate::rendering::sidecar::is_recognised_mode(n) => n.clone(),
        Object::Name(_) => "Normal".to_string(),
        Object::Array(arr) => arr
            .iter()
            .filter_map(|elem| doc.resolve_object(elem).ok())
            .filter_map(|elem| elem.as_name().map(str::to_string))
            .find(|name| crate::rendering::sidecar::is_recognised_mode(name))
            .unwrap_or_else(|| "Normal".to_string()),
        _ => "Normal".to_string(),
    }
}

/// Classify an ExtGState `/SMask` value (§11.4.7 / Table 144). Returns `None`
/// when the value is neither `/None` nor a soft-mask dict carrying a `/G`
/// Form-XObject reference, which leaves the current mask untouched.
fn parse_smask(smask_obj: &Object, doc: &impl ObjectResolver) -> Option<SoftMaskValue> {
    // Resolve through references before classifying. ~keep
    let resolved = doc.resolve_object(smask_obj).unwrap_or(smask_obj.clone());
    let mask_dict = match &resolved {
        Object::Name(n) if n == "None" => return Some(SoftMaskValue::None),
        Object::Dictionary(mask_dict) => mask_dict,
        _ => return None,
    };

    // §7.3.10: sub-entries of a SMask dict may themselves be
    // indirect refs. The /G entry is always a Reference by
    // design (it's the Form XObject id, used as a key into
    // the xref) so it stays an explicit Reference-match. The
    // /S, /BC, and /TR entries are values that the spec
    // allows to be direct OR indirect — resolve before
    // reading. ~keep
    let resolve_in_smask = |key: &str| -> Option<Object> {
        let raw = mask_dict.get(key)?;
        doc.resolve_object(raw).ok()
    };

    // Subtype: /S /Alpha or /S /Luminosity (default Alpha
    // per spec). Anything else falls through to None — a
    // malformed mask must not silently mis-render. ~keep
    let subtype = match resolve_in_smask("S").as_ref().and_then(Object::as_name) {
        Some("Alpha") => SoftMaskSubtype::Alpha,
        Some("Luminosity") => SoftMaskSubtype::Luminosity,
        _ => SoftMaskSubtype::Alpha,
    };

    // /G — required Form XObject reference. Stays as a raw
    // Reference; the renderer loads the form via xref. ~keep
    let form_ref = mask_dict.get("G").and_then(|o| match o {
        Object::Reference(r) => Some(*r),
        _ => None,
    })?;

    // /BC backdrop colour — array of N reals. Only
    // honoured for /S /Luminosity per §11.4.7; for
    // /S /Alpha the spec ignores /BC. Each array
    // element may itself be an indirect ref (§7.3.10). ~keep
    let backdrop = if subtype == SoftMaskSubtype::Luminosity {
        resolve_in_smask("BC").and_then(|o| {
            o.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| doc.resolve_object(v).ok())
                    .filter_map(|v| {
                        v.as_real()
                            .map(|r| r as f32)
                            .or_else(|| v.as_integer().map(|i| i as f32))
                    })
                    .collect::<Vec<f32>>()
            })
        })
    } else {
        None
    };

    // /TR transfer function — stored as the resolved
    // value; the renderer evaluates per-pixel via the
    // Function evaluator already used for tint
    // transforms. Indirect-ref TR (very common — `/TR
    // 12 0 R` pointing at a Function dict) is now
    // resolved at parse time rather than at every
    // per-pixel call. ~keep
    let transfer = resolve_in_smask("TR");

    Some(SoftMaskValue::Form(SoftMaskForm {
        form_ref,
        subtype,
        backdrop,
        transfer,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Minimal PDF document used purely as a `&PdfDocument` argument for
    /// `parse_ext_g_state_inner`. The parser only calls `resolve_object`
    /// on the input; when the input is already an inline dict (not a
    /// `Reference`), that call short-circuits to a clone and never touches
    /// the document's xref. So any successfully-parsed PDF is sufficient.
    fn fixture_doc() -> PdfDocument {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");
        let cat_off = buf.len();
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let pages_off = buf.len();
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref_off = buf.len();
        buf.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        buf.extend_from_slice(format!("{:010} 00000 n \n", cat_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", pages_off).as_bytes());
        buf.extend_from_slice(
            format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes(),
        );
        PdfDocument::from_bytes(buf).expect("fixture PDF parses")
    }

    fn dict(entries: &[(&str, Object)]) -> Object {
        let mut m = HashMap::new();
        for (k, v) in entries {
            m.insert((*k).to_string(), v.clone());
        }
        Object::Dictionary(m)
    }

    #[test]
    fn parses_op_op_opm_from_extgstate_dict() {
        let obj = dict(&[
            ("OP", Object::Boolean(true)),
            ("op", Object::Boolean(false)),
            ("OPM", Object::Integer(1)),
        ]);
        let doc = fixture_doc();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_overprint, Some(true));
        assert_eq!(parsed.fill_overprint, Some(false));
        assert_eq!(parsed.overprint_mode, Some(1));
    }

    #[test]
    fn op_without_op_sets_both_overprints() {
        // §11.7.4 / Table 128: "Specifying an OP entry sets both
        // parameters unless there is also an op entry in the same
        // graphics state parameter dictionary". ~keep
        let obj = dict(&[("OP", Object::Boolean(true))]);
        let doc = fixture_doc();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_overprint, Some(true));
        assert_eq!(parsed.fill_overprint, Some(true));
    }

    #[test]
    fn op_without_op_uppercase_only_does_not_affect_stroke() {
        // /op is the non-stroking parameter only; /OP is absent so the
        // stroking overprint stays unset (caller falls back to gs default). ~keep
        let obj = dict(&[("op", Object::Boolean(true))]);
        let doc = fixture_doc();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_overprint, None);
        assert_eq!(parsed.fill_overprint, Some(true));
    }

    #[test]
    fn opm_clamps_unknown_values_to_zero() {
        // §11.7.4: OPM is 0 or 1; any other value is undefined. We clamp
        // to 0 (standard mode) to preserve the spec-default behavior on
        // malformed PDFs. ~keep
        let obj = dict(&[("OPM", Object::Integer(42))]);
        let doc = fixture_doc();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.overprint_mode, Some(0));
    }

    #[test]
    fn missing_overprint_keys_leave_options_none() {
        // Empty dict → no fields touched. Apply() is a no-op on the gs. ~keep
        let obj = dict(&[]);
        let doc = fixture_doc();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_overprint, None);
        assert_eq!(parsed.fill_overprint, None);
        assert_eq!(parsed.overprint_mode, None);
    }

    /// PDF whose xref carries primitive indirect objects we can reference
    /// from a synthetic ExtGState dict.
    ///
    ///   3 0 obj  0.5            (real)
    ///   4 0 obj  true           (bool)
    ///   5 0 obj  1              (integer)
    ///   6 0 obj  /Multiply      (name)
    ///   7 0 obj  [/Multiply]    (array of names)
    fn fixture_doc_with_indirect_values() -> PdfDocument {
        use crate::object::ObjectRef;
        let _ = ObjectRef::new(0, 0);
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets: Vec<usize> = Vec::new();
        let mut emit = |buf: &mut Vec<u8>, body: &str| {
            let off = buf.len();
            buf.extend_from_slice(body.as_bytes());
            offsets.push(off);
        };
        emit(&mut buf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        emit(&mut buf, "2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        emit(&mut buf, "3 0 obj\n0.5\nendobj\n");
        emit(&mut buf, "4 0 obj\ntrue\nendobj\n");
        emit(&mut buf, "5 0 obj\n1\nendobj\n");
        emit(&mut buf, "6 0 obj\n/Multiply\nendobj\n");
        emit(&mut buf, "7 0 obj\n[/Multiply]\nendobj\n");
        emit(&mut buf, "8 0 obj\n[1 0.5]\nendobj\n");
        let xref_off = buf.len();
        buf.extend_from_slice(b"xref\n0 9\n0000000000 65535 f \n");
        for off in &offsets {
            buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        buf.extend_from_slice(
            format!("trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes(),
        );
        PdfDocument::from_bytes(buf).expect("fixture PDF parses")
    }

    fn obj_ref(num: u32) -> Object {
        use crate::object::ObjectRef;
        Object::Reference(ObjectRef::new(num, 0))
    }

    // ISO 32000-1 §7.3.10 — any direct object value may be replaced by an
    // indirect reference. The ExtGState parser MUST resolve indirect
    // references for every value before reading the typed accessor, or
    // PDFs that emit e.g. `/ca 3 0 R` silently fall back to defaults.
    //
    // These probes pin that resolution for every value the parser reads.
    // Sensitivity-verify by reverting the resolved_value() call inside the
    // parser to a bare `state_dict.get(...)`: every probe below fails
    // because the typed accessor (.as_real / .as_bool / .as_integer /
    // .as_name / .as_array) returns None on an unresolved Reference. ~keep

    #[test]
    fn resolves_indirect_fill_alpha() {
        let obj = dict(&[("ca", obj_ref(3))]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.fill_alpha, Some(0.5_f32));
    }

    #[test]
    fn resolves_indirect_stroke_alpha() {
        let obj = dict(&[("CA", obj_ref(3))]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_alpha, Some(0.5_f32));
    }

    #[test]
    fn resolves_indirect_blend_mode_name() {
        let obj = dict(&[("BM", obj_ref(6))]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.blend_mode.as_deref(), Some("Multiply"));
    }

    #[test]
    fn resolves_indirect_blend_mode_array() {
        let obj = dict(&[("BM", obj_ref(7))]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.blend_mode.as_deref(), Some("Multiply"));
    }

    #[test]
    fn resolves_indirect_op_op_opm() {
        let obj = dict(&[("OP", obj_ref(4)), ("op", obj_ref(4)), ("OPM", obj_ref(5))]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.stroke_overprint, Some(true));
        assert_eq!(parsed.fill_overprint, Some(true));
        assert_eq!(parsed.overprint_mode, Some(1));
    }

    #[test]
    fn resolves_indirect_blend_mode_array_with_indirect_elements() {
        // PDFs in the wild emit `/BM [5 0 R 6 0 R]` where each element is
        // itself an indirect reference to a name object. §7.3.10 lets any
        // direct value be an indirect ref, including inside an array.
        // The parser must resolve each element before classifying. ~keep
        use crate::object::ObjectRef;
        let array_with_indirect_name = Object::Array(vec![Object::Reference(ObjectRef::new(6, 0))]);
        let obj = dict(&[("BM", array_with_indirect_name)]);
        let doc = fixture_doc_with_indirect_values();
        let parsed = parse_ext_g_state_inner(&obj, &doc).expect("parses");
        assert_eq!(parsed.blend_mode.as_deref(), Some("Multiply"));
    }

    fn dash_entry(lengths: Vec<Object>, phase: Object) -> Object {
        dict(&[("D", Object::Array(vec![Object::Array(lengths), phase]))])
    }

    #[test]
    fn parses_the_dash_array_and_phase() {
        let obj = dash_entry(vec![Object::Integer(3), Object::Real(2.5)], Object::Integer(1));
        let parsed = parse_ext_g_state_inner(&obj, &fixture_doc()).expect("parses");
        assert_eq!(parsed.dash, Some((vec![3.0, 2.5], 1.0)));
    }

    #[test]
    fn dash_values_resolve_through_references() {
        use crate::object::ObjectRef;
        let reference = |id| Object::Reference(ObjectRef::new(id, 0));
        // Object 8 is the array [1 0.5], object 3 is 0.5 and object 5 is 1.
        let in_array = dict(&[("D", Object::Array(vec![reference(8), Object::Integer(2)]))]);
        let in_elements = dash_entry(vec![reference(5), reference(3)], reference(5));
        let whole = dict(&[("D", reference(7))]);
        let doc = fixture_doc_with_indirect_values();
        assert_eq!(
            parse_ext_g_state_inner(&in_array, &doc).expect("parses").dash,
            Some((vec![1.0, 0.5], 2.0))
        );
        assert_eq!(
            parse_ext_g_state_inner(&in_elements, &doc).expect("parses").dash,
            Some((vec![1.0, 0.5], 1.0))
        );
        // Object 7 is [/Multiply]: not a two-operand dash, so it is rejected.
        assert_eq!(parse_ext_g_state_inner(&whole, &doc).expect("parses").dash, None);
    }

    #[test]
    fn a_malformed_dash_is_rejected_whole() {
        let doc = fixture_doc();
        let negative = dash_entry(vec![Object::Integer(3), Object::Integer(-1)], Object::Integer(0));
        let not_number = dash_entry(vec![Object::Integer(3), Object::Name("x".into())], Object::Integer(0));
        let bad_phase = dash_entry(vec![Object::Integer(3)], Object::Name("x".into()));
        let too_large = dash_entry(vec![Object::Real(1e39)], Object::Integer(0));
        let one_operand = dict(&[("D", Object::Array(vec![Object::Array(vec![Object::Integer(3)])]))]);
        let three_operands = dict(&[(
            "D",
            Object::Array(vec![
                Object::Array(vec![Object::Integer(3), Object::Integer(2)]),
                Object::Integer(1),
                Object::Integer(5),
            ]),
        )]);
        for obj in [negative, not_number, bad_phase, too_large, one_operand, three_operands] {
            assert_eq!(parse_ext_g_state_inner(&obj, &doc).expect("parses").dash, None);
        }
    }
}
