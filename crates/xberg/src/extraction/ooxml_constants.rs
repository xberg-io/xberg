//! Shared Office Open XML (OOXML) namespace constants.
//!
//! These constants define the XML namespace URIs used across DOCX, PPTX, and XLSX
//! document formats. Centralizing them avoids duplication and ensures consistency.

/// DrawingML namespace - shared by DOCX/PPTX for images, shapes, charts, and themes.
///
/// Used in: `<a:...>` elements (text runs, fonts, colors, shapes)
pub const DRAWINGML_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// WordprocessingML namespace - DOCX-specific content elements.
///
/// Used in: `<w:...>` elements (paragraphs, runs, tables, sections)
pub const WORDPROCESSINGML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// PresentationML namespace - PPTX-specific content elements.
///
/// Used in: `<p:...>` elements (slides, shapes, transitions)
pub const PRESENTATIONML_NAMESPACE: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

/// Office Document Relationships namespace.
///
/// Used in: `.rels` files for linking document parts (images, styles, etc.)
pub const RELATIONSHIPS_NAMESPACE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Package Relationships namespace.
///
/// Used in: `_rels/.rels` for top-level package relationships
pub const PACKAGE_RELATIONSHIPS_NAMESPACE: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Content Types namespace.
///
/// Used in: `[Content_Types].xml`
pub const CONTENT_TYPES_NAMESPACE: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// WordprocessingDrawing namespace - DOCX drawing positioning and wrapping.
///
/// Used in: `<wp:...>` elements (inline, anchor, extent, positionH, positionV)
pub const WORDPROCESSINGDRAWING_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";

/// Picture namespace - DrawingML picture elements.
///
/// Used in: `<pic:...>` elements (pic, blipFill)
pub const PICTURE_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";

/// PowerPoint 2010 extension namespace - sections and other extended features.
///
/// Used in: `<p14:sectionLst>`, `<p14:section>` elements inside `<p:extLst>`
pub const PRESENTATIONML_2010_NAMESPACE: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";

/// Strict (ISO 29500) namespace URIs and the transitional URI each one stands for. Element and
/// attribute names are identical in the two families, so rewriting the URI lets the parsers
/// match the transitional constants alone.
const STRICT_TO_TRANSITIONAL: [(&str, &str); 7] = [
    (
        "http://purl.oclc.org/ooxml/presentationml/main",
        PRESENTATIONML_NAMESPACE,
    ),
    ("http://purl.oclc.org/ooxml/drawingml/main", DRAWINGML_NAMESPACE),
    (
        "http://purl.oclc.org/ooxml/drawingml/chart",
        "http://schemas.openxmlformats.org/drawingml/2006/chart",
    ),
    (
        "http://purl.oclc.org/ooxml/drawingml/diagram",
        "http://schemas.openxmlformats.org/drawingml/2006/diagram",
    ),
    (
        "http://purl.oclc.org/ooxml/drawingml/table",
        "http://schemas.openxmlformats.org/drawingml/2006/table",
    ),
    (
        "http://purl.oclc.org/ooxml/officeDocument/relationships",
        RELATIONSHIPS_NAMESPACE,
    ),
    (
        "http://purl.oclc.org/ooxml/officeDocument/math",
        "http://schemas.openxmlformats.org/officeDocument/2006/math",
    ),
];

fn transitional_uri(strict: &str) -> Option<&'static str> {
    STRICT_TO_TRANSITIONAL
        .iter()
        .find_map(|(known, transitional)| (*known == strict).then_some(*transitional))
}

/// Rewrite the strict (ISO 29500) namespace URIs in `xml` to their transitional equivalents.
///
/// Only an attribute value inside a tag that is exactly a strict URI is rewritten (namespace
/// declarations and `graphicData/@uri`); element text, comments, CDATA and processing instructions are left alone.
/// Returns the input borrowed when it holds no strict URI, which is the common case.
pub(crate) fn normalize_strict_namespaces(xml: &str) -> std::borrow::Cow<'_, str> {
    if !xml.contains("purl.oclc.org/ooxml/") {
        return std::borrow::Cow::Borrowed(xml);
    }
    let bytes = xml.as_bytes();
    let mut out = String::with_capacity(xml.len());
    let mut copied = 0;
    let mut i = 0;
    while let Some(offset) = xml.get(i..).and_then(|rest| rest.find('<')) {
        let start = i + offset;
        let rest = &xml[start..];
        let skip_to = if rest.starts_with("<!--") {
            Some("-->")
        } else if rest.starts_with("<![CDATA[") {
            Some("]]>")
        } else if rest.starts_with("<?") {
            Some("?>")
        } else {
            None
        };
        if let Some(terminator) = skip_to {
            i = rest
                .find(terminator)
                .map_or(xml.len(), |end| start + end + terminator.len());
            continue;
        }
        let mut j = start + 1;
        while j < bytes.len() && bytes[j] != b'>' {
            if matches!(bytes[j], b'"' | b'\'') {
                let Some(len) = xml[j + 1..].find(bytes[j] as char) else {
                    j = bytes.len();
                    break;
                };
                let (value_start, value_end) = (j + 1, j + 1 + len);
                if let Some(transitional) = transitional_uri(&xml[value_start..value_end]) {
                    out.push_str(&xml[copied..value_start]);
                    out.push_str(transitional);
                    copied = value_end;
                }
                j = value_end;
            }
            j += 1;
        }
        i = j + 1;
    }
    if copied == 0 {
        return std::borrow::Cow::Borrowed(xml);
    }
    out.push_str(&xml[copied..]);
    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_uris_in_attribute_values_become_transitional() {
        let xml = r#"<p:sld xmlns:p="http://purl.oclc.org/ooxml/presentationml/main" xmlns:r='http://purl.oclc.org/ooxml/officeDocument/relationships'/>"#;
        assert_eq!(
            normalize_strict_namespaces(xml),
            r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r='http://schemas.openxmlformats.org/officeDocument/2006/relationships'/>"#
        );
    }

    #[test]
    fn strict_uri_in_element_text_is_left_alone() {
        let xml = r#"<a:t xmlns:a="http://purl.oclc.org/ooxml/drawingml/main">see http://purl.oclc.org/ooxml/drawingml/main</a:t>"#;
        assert_eq!(
            normalize_strict_namespaces(xml),
            r#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">see http://purl.oclc.org/ooxml/drawingml/main</a:t>"#
        );
    }

    #[test]
    fn quoted_strict_uri_in_element_text_and_comments_is_left_alone() {
        let xml = r#"<a:t xmlns:a="http://purl.oclc.org/ooxml/drawingml/main">"http://purl.oclc.org/ooxml/drawingml/main" 'http://purl.oclc.org/ooxml/drawingml/main'<!-- "http://purl.oclc.org/ooxml/drawingml/main" --></a:t>"#;
        assert_eq!(
            normalize_strict_namespaces(xml),
            r#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"http://purl.oclc.org/ooxml/drawingml/main" 'http://purl.oclc.org/ooxml/drawingml/main'<!-- "http://purl.oclc.org/ooxml/drawingml/main" --></a:t>"#
        );
    }

    #[test]
    fn truncated_tag_does_not_panic() {
        let xml = r#"<a:t xmlns:a="http://purl.oclc.org/ooxml/drawingml/main" b="unterminated"#;
        assert_eq!(
            normalize_strict_namespaces(xml),
            r#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" b="unterminated"#
        );
    }

    #[test]
    fn cdata_comments_and_processing_instructions_are_not_scanned_as_tags() {
        let strict = "http://purl.oclc.org/ooxml/drawingml/main";
        let xml = format!(r#"<?pi "{strict}"?><a:t xmlns:a="{strict}"><![CDATA[<b c="{strict}">]]></a:t>"#);
        let expected = format!(
            r#"<?pi "{strict}"?><a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><![CDATA[<b c="{strict}">]]></a:t>"#
        );
        assert_eq!(normalize_strict_namespaces(&xml), expected);
    }

    #[test]
    fn greater_than_inside_an_attribute_value_does_not_end_the_tag() {
        let strict = "http://purl.oclc.org/ooxml/drawingml/main";
        let xml = format!(r#"<a:t note="a > b" xmlns:a="{strict}"/>"#);
        assert_eq!(
            normalize_strict_namespaces(&xml),
            r#"<a:t note="a > b" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#
        );
    }

    #[test]
    fn strict_uri_that_is_not_rewritten_stays_borrowed() {
        let xml = r#"<a:t>http://purl.oclc.org/ooxml/drawingml/main</a:t>"#;
        assert!(matches!(
            normalize_strict_namespaces(xml),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn transitional_input_is_borrowed() {
        let xml = r#"<a:t xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;
        assert!(matches!(
            normalize_strict_namespaces(xml),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}
