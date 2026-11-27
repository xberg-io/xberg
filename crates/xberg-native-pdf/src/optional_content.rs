//! Shared Optional Content (OCG / OCMD) helpers.
//!
//! Both the text-extraction path (`extractors::text`) and the rendering
//! pipeline (`rendering::page_renderer`) need to decide whether a BDC scope
//! tagged `/OC` belongs to an excluded layer. This module owns the logic so
//! the two callers cannot drift apart (a previous duplication caused a real
//! correctness bug where the renderer failed to decode UTF-16LE / PDFDocEncoding
//! layer names that the extractor handled correctly).
//!
//! NOTE — the two callers apply different *default* policies, by design.
//! `render_page` honours the PDF's own default configuration `/OCProperties/D`
//! (so an OCG marked off in `/D/OFF`, or off under `/D/BaseState /OFF`, is hidden
//! even when the caller passes no layer filter). `extract_text` instead treats
//! all content as visible unless the caller explicitly names layers to exclude,
//! and skips OCG evaluation entirely when that set is empty. Both are spec-
//! defensible — §8.11.3 NOTE 4 leaves it to the consumer whether optional content
//! is processed for a non-interactive (e.g. text-extraction) purpose — but it
//! means the two surfaces can legitimately disagree on visibility for the same
//! PDF. Callers wanting render/extract parity should pass matching `excluded_layers`.
//!
//! References:
//!  - ISO 32000-1:2008 §8.11.2 — Optional Content
//!  - ISO 32000-1:2008 §8.11.2.2 — Optional Content Membership Dictionaries
//!  - ISO 32000-1:2008 §7.9.2 — Text string encoding (UTF-16BE/LE/PDFDocEncoding)

use std::collections::{HashMap, HashSet};

use crate::document::PdfDocument;
use crate::object::Object;

/// Compute the document's default-off OCG names per ISO 32000-1 §8.11.4.
///
/// Reads `/OCProperties/D` (the default configuration) and returns the set of
/// OCG `/Name` strings that are off in the default state. Logic:
///
///  - `/D/BaseState` = `/ON` (or absent): OCGs default to on. `/D/OFF` lists
///     names that are explicitly turned off.
///  - `/D/BaseState` = `/OFF`: OCGs default to off. All OCGs in `/OCProperties
///    /OCGs` that are NOT in `/D/ON` are off.
///  - `/D/BaseState` = `/Unchanged`: treat as `/ON` (libraries have no prior
///    viewer state to preserve).
///  - `/OCProperties` or `/D` absent: returns empty set (all OCGs on).
///
/// Callers typically combine this with caller-supplied `excluded_layers` via
/// `default_off | excluded_layers` to get the effective "off" set for filtering.
pub fn compute_default_off_ocgs(doc: &PdfDocument) -> HashSet<String> {
    let mut off = HashSet::new();
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return off,
    };
    let catalog_dict = match catalog.as_dict() {
        Some(d) => d,
        None => return off,
    };
    let oc_props = match resolve_indirect(catalog_dict.get("OCProperties"), doc) {
        Some(o) => o,
        None => return off,
    };
    let oc_dict = match oc_props.as_dict() {
        Some(d) => d,
        None => return off,
    };
    let d_obj = match resolve_indirect(oc_dict.get("D"), doc) {
        Some(o) => o,
        None => return off,
    };
    let d_dict = match d_obj.as_dict() {
        Some(d) => d,
        None => return off,
    };

    let base_state = d_dict.get("BaseState").and_then(|o| o.as_name()).unwrap_or("ON");
    let on_set = collect_ocg_name_set(d_dict.get("ON"), doc);
    let off_set = collect_ocg_name_set(d_dict.get("OFF"), doc);

    if base_state == "OFF" {
        let all_ocgs = collect_ocg_name_set(oc_dict.get("OCGs"), doc);
        for name in all_ocgs {
            if !on_set.contains(&name) {
                off.insert(name);
            }
        }
    } else {
        off.extend(off_set);
    }

    off
}

fn resolve_indirect(obj: Option<&Object>, doc: &PdfDocument) -> Option<Object> {
    let obj = obj?;
    match obj.as_reference() {
        Some(r) => doc.load_object(r).ok(),
        None => Some(obj.clone()),
    }
}

fn collect_ocg_name_set(arr_obj: Option<&Object>, doc: &PdfDocument) -> HashSet<String> {
    let mut names = HashSet::new();
    let resolved = match resolve_indirect(arr_obj, doc) {
        Some(o) => o,
        None => return names,
    };
    let arr = match resolved.as_array() {
        Some(a) => a,
        None => return names,
    };
    for item in arr {
        let resolved = match item.as_reference() {
            Some(r) => match doc.load_object(r) {
                Ok(o) => o,
                Err(_) => continue,
            },
            None => item.clone(),
        };
        if let Some(d) = resolved.as_dict() {
            if let Some(Object::Name(n)) = d.get("Name") {
                names.insert(n.clone());
            } else if let Some(name_obj) = d.get("Name")
                && let Some(b) = name_obj.as_string()
            {
                names.insert(decode_pdf_text_string(b));
            }
        }
    }
    names
}

/// OCMD visibility policy (`/P` entry). Per ISO 32000-1 §8.11.2.2 Table 102.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcmdPolicy {
    /// Visible if **any** referenced OCG is on. (Default per spec.)
    AnyOn,
    /// Visible only if **all** referenced OCGs are on.
    AllOn,
    /// Visible if **any** referenced OCG is off.
    AnyOff,
    /// Visible only if **all** referenced OCGs are off.
    AllOff,
}

impl OcmdPolicy {
    fn from_name(s: &str) -> Self {
        match s {
            "AllOn" => OcmdPolicy::AllOn,
            "AnyOff" => OcmdPolicy::AnyOff,
            "AllOff" => OcmdPolicy::AllOff,
            _ => OcmdPolicy::AnyOn,
        }
    }
}

/// Decode a PDF text string per ISO 32000-1 §7.9.2.
///
/// Kept here as a re-export for callers of this module; the canonical
/// implementation lives in [`crate::object::decode_pdf_text_string`].
pub use crate::object::decode_pdf_text_string;

/// Check if a `Name` value (which can be either a `/Name` token or a PDF string)
/// matches any entry in `excluded`.
pub fn ocg_name_is_excluded(name_obj: &Object, excluded: &HashSet<String>) -> bool {
    if let Some(name_str) = name_obj.as_name() {
        return excluded.contains(name_str);
    }
    if let Some(name_bytes) = name_obj.as_string() {
        let name_str = decode_pdf_text_string(name_bytes);
        return excluded.contains(&name_str);
    }
    false
}

/// Resolve a BDC `properties` operand into a property dictionary.
///
/// `properties` is either an inline dictionary (e.g. `BDC /OC << /Name /Foo >>`)
/// or a name (e.g. `BDC /OC /MC0`) which references an entry in the page's
/// `/Resources /Properties` dictionary.
///
/// Handles indirect references at every level (the resources dict, the
/// Properties sub-dict, and the property entry itself can all be `Reference`s).
/// `doc` may be `None` — in that case only the inline-dict fast path resolves;
/// the name-reference path requires indirect-object resolution.
pub fn resolve_bdc_properties(
    properties: &Object,
    resources: Option<&Object>,
    doc: Option<&PdfDocument>,
) -> Option<HashMap<String, Object>> {
    if let Some(dict) = properties.as_dict() {
        return Some(dict.clone());
    }

    let prop_name = properties.as_name()?;
    let resources = resources?;
    let doc = doc?;
    let shared_resources;
    let res_dict = if let Some(res_ref) = resources.as_reference() {
        shared_resources = doc.load_object_shared(res_ref).ok()?;
        shared_resources.as_dict()?
    } else {
        resources.as_dict()?
    };
    let properties_dict_obj = res_dict.get("Properties")?;
    let shared_properties;
    let properties_dict = if let Some(r) = properties_dict_obj.as_reference() {
        shared_properties = doc.load_object_shared(r).ok()?;
        shared_properties.as_dict()?
    } else {
        properties_dict_obj.as_dict()?
    };
    let prop_obj = properties_dict.get(prop_name)?;
    let shared_property;
    let resolved = if let Some(r) = prop_obj.as_reference() {
        shared_property = doc.load_object_shared(r).ok()?;
        shared_property.as_dict()?
    } else {
        prop_obj.as_dict()?
    };
    Some(resolved.clone())
}

/// Collect the `/Name` strings of every OCG referenced by an OCMD `/OCGs` entry.
///
/// `/OCGs` may be either a single OCG dictionary (or reference) or an array of
/// them. Each entry that resolves to a dictionary with a `/Name` contributes
/// one name. References that fail to resolve are silently skipped.
fn collect_ocmd_ocg_names(ocgs_obj: &Object, doc: &PdfDocument) -> Vec<Object> {
    let refs: Vec<&Object> = if let Some(arr) = ocgs_obj.as_array() {
        arr.iter().collect()
    } else {
        vec![ocgs_obj]
    };

    let mut names = Vec::with_capacity(refs.len());
    for obj in refs {
        let resolved = if let Some(r) = obj.as_reference() {
            match doc.load_object(r) {
                Ok(o) => o,
                Err(_) => continue,
            }
        } else {
            obj.clone()
        };
        if let Some(d) = resolved.as_dict()
            && let Some(name_obj) = d.get("Name")
        {
            names.push(name_obj.clone());
        }
    }
    names
}

/// Decide whether a resolved BDC properties dict represents an excluded
/// optional-content scope (OCG or OCMD).
///
/// Semantics:
///  - **OCG** (dict has `/Name`): excluded iff the name is in `excluded`.
///  - **OCMD** (dict has `/Type /OCMD`): if `/VE` is present, evaluate the
///    visibility expression tree (`/And`, `/Or`, `/Not`); otherwise apply
///    the `/P` policy (`AnyOn` default) over the `/OCGs` membership where
///    `on = !excluded`. Content is excluded iff the result is "hidden".
pub fn check_ocg_excluded(props_dict: &HashMap<String, Object>, doc: &PdfDocument, excluded: &HashSet<String>) -> bool {
    if let Some(ocg_name) = props_dict.get("Name") {
        return ocg_name_is_excluded(ocg_name, excluded);
    }

    if let Some(Object::Name(t)) = props_dict.get("Type")
        && t == "OCMD"
    {
        // /VE takes precedence over /P per ISO 32000-1 §8.11.2.4. ~keep
        if let Some(ve) = props_dict.get("VE") {
            return !evaluate_visibility_expression(ve, doc, excluded);
        }

        let policy = props_dict
            .get("P")
            .and_then(|o| o.as_name())
            .map(OcmdPolicy::from_name)
            .unwrap_or(OcmdPolicy::AnyOn);

        let names = match props_dict.get("OCGs") {
            Some(o) => collect_ocmd_ocg_names(o, doc),
            None => return false,
        };

        return ocmd_is_hidden(&names, policy, excluded);
    }

    false
}

/// Evaluate an OCMD `/VE` visibility expression to a boolean visibility state.
///
/// Per ISO 32000-1 §8.11.2.4 a visibility expression is an array whose first
/// element is the operator (`/And`, `/Or`, or `/Not`) and remaining elements
/// are operands. Each operand is either:
///  - An indirect reference to an OCG dict (evaluates to that OCG's on-state)
///  - A nested visibility expression array (evaluates recursively)
///
/// Returns `true` if the expression resolves to "visible", `false` to "hidden".
/// For our exclusion model an OCG is on iff its `/Name` is NOT in `excluded`.
/// Bounded recursion depth prevents stack overflow on hostile/malformed input.
fn evaluate_visibility_expression(expr: &Object, doc: &PdfDocument, excluded: &HashSet<String>) -> bool {
    fn eval(expr: &Object, doc: &PdfDocument, excluded: &HashSet<String>, depth: u8) -> bool {
        if depth > 16 {
            return true; // permissive: don't suppress on malformed/circular input ~keep
        }

        let resolved = match expr.as_reference() {
            Some(r) => doc.load_object(r).unwrap_or_else(|_| expr.clone()),
            None => expr.clone(),
        };

        if let Some(d) = resolved.as_dict() {
            if let Some(name) = d.get("Name") {
                return !ocg_name_is_excluded(name, excluded);
            }
            return true;
        }

        let arr = match resolved.as_array() {
            Some(a) => a,
            None => return true,
        };
        let op = match arr.first().and_then(|o| o.as_name()) {
            Some(n) => n,
            None => return true,
        };

        match op {
            "Not" => !arr.get(1).map(|o| eval(o, doc, excluded, depth + 1)).unwrap_or(true),
            "And" => arr[1..].iter().all(|o| eval(o, doc, excluded, depth + 1)),
            "Or" => arr[1..].iter().any(|o| eval(o, doc, excluded, depth + 1)),
            _ => true,
        }
    }

    eval(expr, doc, excluded, 0)
}

/// Resolve BDC properties and decide if the resulting scope is excluded.
///
/// Convenience wrapper combining [`resolve_bdc_properties`] and
/// [`check_ocg_excluded`] — the typical call site (BDC operator handler) just
/// needs the boolean.
pub fn resolve_and_check_ocg_excluded(
    properties: &Object,
    resources: Option<&Object>,
    doc: Option<&PdfDocument>,
    excluded: &HashSet<String>,
) -> bool {
    let props_dict = match resolve_bdc_properties(properties, resources, doc) {
        Some(d) => d,
        None => return false,
    };
    // OCMD evaluation needs the document to resolve referenced OCG /Name fields,
    // but the inline-OCG case (Name in the props dict) does not. If we have no
    // doc, only the OCG-Name short-circuit inside check_ocg_excluded can fire. ~keep
    match doc {
        Some(d) => check_ocg_excluded(&props_dict, d, excluded),
        None => {
            if let Some(ocg_name) = props_dict.get("Name") {
                return ocg_name_is_excluded(ocg_name, excluded);
            }
            false
        }
    }
}

/// Resolve an annotation `/OC` entry (an OCG or OCMD dict, possibly indirect)
/// and decide whether the annotation belongs to an excluded layer.
///
/// Per ISO 32000-1 §12.5.2, annotation dictionaries can carry an `/OC` entry
/// that references the OCG / OCMD the annotation belongs to. If that scope is
/// excluded, the annotation should not be rendered.
pub fn annotation_is_excluded(oc_obj: &Object, doc: &PdfDocument, excluded: &HashSet<String>) -> bool {
    if excluded.is_empty() {
        return false;
    }
    let resolved = if let Some(r) = oc_obj.as_reference() {
        match doc.load_object(r) {
            Ok(o) => o,
            Err(_) => return false,
        }
    } else {
        oc_obj.clone()
    };
    let dict = match resolved.as_dict() {
        Some(d) => d,
        None => return false,
    };
    check_ocg_excluded(dict, doc, excluded)
}

/// Apply an OCMD policy to a list of referenced OCG names.
///
/// Returns `true` if the content should be **hidden** (i.e. the scope evaluates
/// to "not visible"). With no referenced OCGs the result is `false` (spec says
/// such an OCMD is always visible, which mirrors the AnyOn-with-empty case).
///
/// Semantics (membership state: `on = !excluded`):
///  - `AnyOn`  — visible iff any referenced OCG is on → hide iff all are off.
///  - `AllOn`  — visible iff all referenced OCGs are on → hide iff any is off.
///  - `AnyOff` — visible iff any referenced OCG is off → hide iff all are on.
///  - `AllOff` — visible iff all referenced OCGs are off → hide iff any is on.
fn ocmd_is_hidden(ocg_names: &[Object], policy: OcmdPolicy, excluded: &HashSet<String>) -> bool {
    if ocg_names.is_empty() {
        return false;
    }

    let mut any_on = false;
    let mut any_off = false;
    let mut all_on = true;
    let mut all_off = true;

    for name in ocg_names {
        let is_off = ocg_name_is_excluded(name, excluded);
        if is_off {
            any_off = true;
            all_on = false;
        } else {
            any_on = true;
            all_off = false;
        }
    }

    match policy {
        OcmdPolicy::AnyOn => !any_on,
        OcmdPolicy::AllOn => !all_on,
        OcmdPolicy::AnyOff => !any_off,
        OcmdPolicy::AllOff => !all_off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_utf16be_bom() {
        let bytes = b"\xFE\xFF\x00L\x00a\x00y\x00e\x00r";
        assert_eq!(decode_pdf_text_string(bytes), "Layer");
    }

    #[test]
    fn decode_utf16le_bom() {
        let bytes = b"\xFF\xFEL\x00a\x00y\x00e\x00r\x00";
        assert_eq!(decode_pdf_text_string(bytes), "Layer");
    }

    #[test]
    fn decode_utf8_ascii() {
        assert_eq!(decode_pdf_text_string(b"Hello"), "Hello");
    }

    #[test]
    fn decode_pdfdoc_fallback() {
        // 0x85 in PDFDocEncoding = U+2013 (endash) ~keep
        assert_eq!(decode_pdf_text_string(&[0x85]), "\u{2013}");
    }

    #[test]
    fn ocg_name_is_excluded_matches_name_token() {
        let mut excluded = HashSet::new();
        excluded.insert("Watermark".to_string());
        assert!(ocg_name_is_excluded(&Object::Name("Watermark".into()), &excluded));
        assert!(!ocg_name_is_excluded(&Object::Name("Other".into()), &excluded));
    }

    #[test]
    fn ocg_name_is_excluded_matches_utf16le_string() {
        let mut excluded = HashSet::new();
        excluded.insert("Layer".to_string());
        let bytes: Vec<u8> = b"\xFF\xFEL\x00a\x00y\x00e\x00r\x00".to_vec();
        assert!(ocg_name_is_excluded(&Object::String(bytes), &excluded));
    }

    /// Catalog and empty page tree as objects 1 and 2, then `bodies` as objects 3, 4, ...
    fn document_with_objects(bodies: &[&str]) -> PdfDocument {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![pdf.len()];
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        for (index, body) in bodies.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 3).as_bytes());
        }
        let xref_offset = pdf.len();
        let size = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
        );
        PdfDocument::from_bytes(pdf).expect("open synthetic document")
    }

    #[test]
    fn named_bdc_properties_resolve_through_indirect_levels_without_copying_them() {
        let document = document_with_objects(&[
            "<< /Properties 4 0 R /Font << /F1 << /Type /Font >> >> >>",
            "<< /MC0 5 0 R /MC1 << /Type /OCG /Name /Inline >> >>",
            "<< /Type /OCG /Name /Watermark >>",
        ]);
        let resources = Object::Reference(crate::object::ObjectRef::new(3, 0));
        let resolve =
            |name: &str| resolve_bdc_properties(&Object::Name(name.into()), Some(&resources), Some(&document));

        let indirect = resolve("MC0").expect("indirect property dictionary");
        assert_eq!(indirect.get("Name").and_then(Object::as_name), Some("Watermark"));
        let direct = resolve("MC1").expect("direct property dictionary");
        assert_eq!(direct.get("Name").and_then(Object::as_name), Some("Inline"));
        assert_eq!(resolve("Missing"), None);

        let copies_before = document.cached_object_clone_count();
        assert_eq!(resolve("MC0"), Some(indirect));
        assert_eq!(resolve("MC1"), Some(direct));
        assert_eq!(
            document.cached_object_clone_count(),
            copies_before,
            "warm lookups must borrow the cached Resources and Properties dictionaries"
        );
    }

    #[test]
    fn inline_and_direct_bdc_properties_resolve_without_a_document_lookup() {
        let inline = Object::Dictionary(HashMap::from([("MCID".to_string(), Object::Integer(7))]));
        assert_eq!(
            resolve_bdc_properties(&inline, None, None).and_then(|dict| dict.get("MCID").and_then(Object::as_integer)),
            Some(7)
        );

        let document = document_with_objects(&[]);
        let resources = Object::Dictionary(HashMap::from([(
            "Properties".to_string(),
            Object::Dictionary(HashMap::from([
                ("MC0".to_string(), inline.clone()),
                ("NotADict".to_string(), Object::Integer(1)),
            ])),
        )]));
        let resolve =
            |name: &str| resolve_bdc_properties(&Object::Name(name.into()), Some(&resources), Some(&document));
        assert_eq!(resolve("MC0").as_ref(), inline.as_dict());
        assert_eq!(resolve("NotADict"), None);
        assert_eq!(
            resolve_bdc_properties(&Object::Name("MC0".into()), Some(&resources), None),
            None
        );
    }

    #[test]
    fn policy_default_is_any_on() {
        assert_eq!(OcmdPolicy::from_name("nonsense"), OcmdPolicy::AnyOn);
        assert_eq!(OcmdPolicy::from_name("AnyOn"), OcmdPolicy::AnyOn);
        assert_eq!(OcmdPolicy::from_name("AllOn"), OcmdPolicy::AllOn);
        assert_eq!(OcmdPolicy::from_name("AnyOff"), OcmdPolicy::AnyOff);
        assert_eq!(OcmdPolicy::from_name("AllOff"), OcmdPolicy::AllOff);
    }

    fn names(slice: &[&str]) -> Vec<Object> {
        slice.iter().map(|s| Object::Name((*s).into())).collect()
    }

    #[test]
    fn policy_any_on_hides_when_all_excluded() {
        let mut excluded = HashSet::new();
        excluded.insert("A".to_string());
        excluded.insert("B".to_string());

        assert!(ocmd_is_hidden(&names(&["A", "B"]), OcmdPolicy::AnyOn, &excluded));
        assert!(!ocmd_is_hidden(&names(&["A", "C"]), OcmdPolicy::AnyOn, &excluded));
    }

    #[test]
    fn policy_all_on_hides_when_any_excluded() {
        let mut excluded = HashSet::new();
        excluded.insert("A".to_string());

        assert!(ocmd_is_hidden(&names(&["A", "B"]), OcmdPolicy::AllOn, &excluded));
        assert!(!ocmd_is_hidden(&names(&["C", "B"]), OcmdPolicy::AllOn, &excluded));
    }

    #[test]
    fn policy_any_off_hides_when_all_on() {
        let mut excluded = HashSet::new();
        excluded.insert("A".to_string());

        assert!(!ocmd_is_hidden(&names(&["A", "B"]), OcmdPolicy::AnyOff, &excluded));
        assert!(ocmd_is_hidden(&names(&["B", "C"]), OcmdPolicy::AnyOff, &excluded));
    }

    #[test]
    fn policy_all_off_hides_when_any_on() {
        let mut excluded = HashSet::new();
        excluded.insert("A".to_string());
        excluded.insert("B".to_string());

        assert!(ocmd_is_hidden(&names(&["A", "C"]), OcmdPolicy::AllOff, &excluded));
        assert!(!ocmd_is_hidden(&names(&["A", "B"]), OcmdPolicy::AllOff, &excluded));
    }

    #[test]
    fn empty_ocgs_is_not_hidden() {
        let excluded = HashSet::new();
        assert!(!ocmd_is_hidden(&[], OcmdPolicy::AnyOn, &excluded));
        assert!(!ocmd_is_hidden(&[], OcmdPolicy::AllOn, &excluded));
        assert!(!ocmd_is_hidden(&[], OcmdPolicy::AnyOff, &excluded));
        assert!(!ocmd_is_hidden(&[], OcmdPolicy::AllOff, &excluded));
    }
}
