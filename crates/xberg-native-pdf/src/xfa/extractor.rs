//! XFA data extraction from PDF documents.
//!
//! This module provides utilities to detect and extract XFA form data
//! from PDF documents.

use crate::decoders::decode_stream;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;

const MAX_XFA_ARRAY_ENTRIES: usize = 100_000;
const MAX_XFA_DECODED_BYTES: usize = 256 * 1024 * 1024;

/// XFA data extractor.
///
/// Provides static methods to detect and extract XFA form data from PDF documents.
pub struct XfaExtractor;

impl XfaExtractor {
    /// Check if a PDF document contains an XFA form.
    ///
    /// XFA forms are indicated by the presence of an /XFA entry in the /AcroForm
    /// dictionary of the document catalog.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use xberg_native_pdf::PdfDocument;
    /// use xberg_native_pdf::xfa::XfaExtractor;
    ///
    /// let mut doc = PdfDocument::open("form.pdf")?;
    /// if XfaExtractor::has_xfa(&mut doc)? {
    ///     println!("Document contains XFA form");
    /// }
    /// ```
    pub fn has_xfa(doc: &mut PdfDocument) -> Result<bool> {
        let catalog = doc.catalog()?;
        let catalog_dict = match catalog.as_dict() {
            Some(d) => d,
            None => return Ok(false),
        };

        let acroform_obj = match catalog_dict.get("AcroForm") {
            Some(obj) => Self::resolve_object(doc, obj)?,
            None => return Ok(false),
        };

        let acroform_dict = match acroform_obj.as_dict() {
            Some(d) => d,
            None => return Ok(false),
        };

        Ok(acroform_dict.contains_key("XFA"))
    }

    /// Extract XFA data from a PDF document.
    ///
    /// XFA data in PDFs can be stored as:
    /// 1. A single stream containing the complete XDP document
    /// 2. An array of alternating name/stream pairs for different XFA packets
    ///
    /// This method returns the raw XFA data bytes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use xberg_native_pdf::PdfDocument;
    /// use xberg_native_pdf::xfa::{XfaExtractor, XfaParser};
    ///
    /// let mut doc = PdfDocument::open("form.pdf")?;
    /// if let Ok(xfa_data) = XfaExtractor::extract_xfa(&mut doc) {
    ///     let mut parser = XfaParser::new();
    ///     let form = parser.parse(&xfa_data)?;
    ///     println!("Found {} fields", form.field_count());
    /// }
    /// ```
    pub fn extract_xfa(doc: &mut PdfDocument) -> Result<Vec<u8>> {
        let catalog = doc.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let acroform_obj = catalog_dict
            .get("AcroForm")
            .ok_or_else(|| Error::InvalidPdf("No AcroForm in document".to_string()))?;
        let acroform_obj = Self::resolve_object(doc, acroform_obj)?;

        let acroform_dict = acroform_obj
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("AcroForm is not a dictionary".to_string()))?;

        let xfa_raw = acroform_dict
            .get("XFA")
            .ok_or_else(|| Error::InvalidPdf("No XFA entry in AcroForm".to_string()))?;
        let xfa_obj = Self::resolve_object(doc, xfa_raw)?;

        match &xfa_obj {
            Object::Stream { .. } => Self::decode_xfa_stream(doc, xfa_raw, &xfa_obj),
            Object::Array(arr) => Self::extract_xfa_array(doc, arr),
            _ => Err(Error::InvalidPdf("XFA entry is neither stream nor array".to_string())),
        }
    }

    /// Extract XFA packets from an array format.
    ///
    /// Returns a vector of (name, data) pairs for each XFA packet.
    pub fn extract_xfa_packets(doc: &mut PdfDocument) -> Result<Vec<(String, Vec<u8>)>> {
        let catalog = doc.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let acroform_obj = catalog_dict
            .get("AcroForm")
            .ok_or_else(|| Error::InvalidPdf("No AcroForm in document".to_string()))?;
        let acroform_obj = Self::resolve_object(doc, acroform_obj)?;

        let acroform_dict = acroform_obj
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("AcroForm is not a dictionary".to_string()))?;

        let xfa_raw = acroform_dict
            .get("XFA")
            .ok_or_else(|| Error::InvalidPdf("No XFA entry in AcroForm".to_string()))?;
        let xfa_obj = Self::resolve_object(doc, xfa_raw)?;

        match &xfa_obj {
            Object::Stream { .. } => {
                let decoded = Self::decode_xfa_stream(doc, xfa_raw, &xfa_obj)?;
                Ok(vec![("xdp".to_string(), decoded)])
            }
            Object::Array(arr) => Self::extract_xfa_packets_from_array(doc, arr),
            _ => Err(Error::InvalidPdf("XFA entry is neither stream nor array".to_string())),
        }
    }

    /// Resolve an indirect reference to its object.
    fn resolve_object(doc: &mut PdfDocument, obj: &Object) -> Result<Object> {
        if let Some(ref_val) = obj.as_reference() {
            doc.load_object(ref_val)
        } else {
            Ok(obj.clone())
        }
    }

    /// Decode a stream's data using its filter chain.
    fn decode_stream_data(dict: &std::collections::HashMap<String, Object>, data: &[u8]) -> Result<Vec<u8>> {
        let filters = Self::get_filters(dict);
        if filters.is_empty() {
            Ok(data.to_vec())
        } else {
            decode_stream(data, &filters)
        }
    }

    /// Extract filter names from stream dictionary.
    fn get_filters(dict: &std::collections::HashMap<String, Object>) -> Vec<String> {
        let filter_obj = dict.get("Filter");
        match filter_obj {
            Some(Object::Name(n)) => vec![n.clone()],
            Some(Object::Array(arr)) => arr
                .iter()
                .filter_map(|o| if let Object::Name(n) = o { Some(n.clone()) } else { None })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Decode an XFA stream through the document's decryption when it is an indirect object.
    fn decode_xfa_stream(doc: &PdfDocument, raw: &Object, resolved: &Object) -> Result<Vec<u8>> {
        match (raw.as_reference(), resolved) {
            (Some(obj_ref), _) => doc.decode_stream_with_encryption(resolved, obj_ref),
            (None, Object::Stream { dict, data }) => Self::decode_stream_data(dict, data),
            _ => Err(Error::InvalidPdf("XFA packet is not a stream".to_string())),
        }
    }

    /// Decode every stream in an alternating name/stream array, decoding each distinct reference once and
    /// bounding both the array length and the total decoded size.
    fn decode_xfa_array(doc: &mut PdfDocument, arr: &[Object]) -> Result<Vec<(Option<String>, Vec<u8>)>> {
        if arr.len() > MAX_XFA_ARRAY_ENTRIES {
            return Err(Error::InvalidPdf(format!(
                "XFA array has {} entries (limit {MAX_XFA_ARRAY_ENTRIES})",
                arr.len()
            )));
        }
        let mut decoded_by_ref: std::collections::HashMap<crate::object::ObjectRef, std::sync::Arc<Vec<u8>>> =
            std::collections::HashMap::new();
        let mut total = 0usize;
        let mut packets = Vec::new();

        let mut i = 0;
        while i + 1 < arr.len() {
            let name = match &arr[i] {
                Object::Name(n) => Some(n.clone()),
                Object::String(s) => Some(String::from_utf8_lossy(s).to_string()),
                _ => None,
            };
            let raw = &arr[i + 1];
            let cached = raw.as_reference().and_then(|r| decoded_by_ref.get(&r).cloned());
            let decoded = match cached {
                Some(decoded) => decoded,
                None => {
                    let stream_obj = Self::resolve_object(doc, raw)?;
                    if !matches!(stream_obj, Object::Stream { .. }) {
                        i += 2;
                        continue;
                    }
                    let decoded = std::sync::Arc::new(Self::decode_xfa_stream(doc, raw, &stream_obj)?);
                    if let Some(r) = raw.as_reference() {
                        decoded_by_ref.insert(r, std::sync::Arc::clone(&decoded));
                    }
                    decoded
                }
            };
            total = total.saturating_add(decoded.len());
            if total > MAX_XFA_DECODED_BYTES {
                return Err(Error::InvalidPdf(format!(
                    "XFA packets exceed the {MAX_XFA_DECODED_BYTES} byte decoded limit"
                )));
            }
            packets.push((name, decoded.as_ref().clone()));
            i += 2;
        }

        Ok(packets)
    }

    /// Extract and combine XFA data from an array of packets.
    fn extract_xfa_array(doc: &mut PdfDocument, arr: &[Object]) -> Result<Vec<u8>> {
        let mut combined = Vec::new();
        for (_, decoded) in Self::decode_xfa_array(doc, arr)? {
            combined.extend_from_slice(&decoded);
        }
        Ok(combined)
    }

    /// Extract packets from XFA array with names preserved.
    fn extract_xfa_packets_from_array(doc: &mut PdfDocument, arr: &[Object]) -> Result<Vec<(String, Vec<u8>)>> {
        Ok(Self::decode_xfa_array(doc, arr)?
            .into_iter()
            .enumerate()
            .map(|(index, (name, decoded))| (name.unwrap_or_else(|| format!("packet_{index}")), decoded))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_get_filters_single_name() {
        let mut dict = HashMap::new();
        dict.insert("Filter".to_string(), Object::Name("FlateDecode".to_string()));
        let filters = XfaExtractor::get_filters(&dict);
        assert_eq!(filters, vec!["FlateDecode"]);
    }

    #[test]
    fn test_get_filters_array_of_names() {
        let mut dict = HashMap::new();
        dict.insert(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("ASCII85Decode".to_string()),
                Object::Name("FlateDecode".to_string()),
            ]),
        );
        let filters = XfaExtractor::get_filters(&dict);
        assert_eq!(filters, vec!["ASCII85Decode", "FlateDecode"]);
    }

    #[test]
    fn test_get_filters_array_with_non_name_elements() {
        let mut dict = HashMap::new();
        dict.insert(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("FlateDecode".to_string()),
                Object::Integer(42),
                Object::Name("LZWDecode".to_string()),
            ]),
        );
        let filters = XfaExtractor::get_filters(&dict);
        assert_eq!(filters, vec!["FlateDecode", "LZWDecode"]);
    }

    #[test]
    fn test_get_filters_no_filter_key() {
        let dict = HashMap::new();
        let filters = XfaExtractor::get_filters(&dict);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_get_filters_invalid_filter_type() {
        let mut dict = HashMap::new();
        dict.insert("Filter".to_string(), Object::Integer(99));
        let filters = XfaExtractor::get_filters(&dict);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_get_filters_null_filter() {
        let mut dict = HashMap::new();
        dict.insert("Filter".to_string(), Object::Null);
        let filters = XfaExtractor::get_filters(&dict);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_get_filters_empty_array() {
        let mut dict = HashMap::new();
        dict.insert("Filter".to_string(), Object::Array(vec![]));
        let filters = XfaExtractor::get_filters(&dict);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_decode_stream_data_no_filters() {
        let dict = HashMap::new();
        let data = b"raw stream content";
        let result = XfaExtractor::decode_stream_data(&dict, data).unwrap();
        assert_eq!(result, b"raw stream content");
    }

    #[test]
    fn test_decode_stream_data_with_asciihex_filter() {
        let mut dict = HashMap::new();
        dict.insert("Filter".to_string(), Object::Name("ASCIIHexDecode".to_string()));
        let data = b"48656C6C6F";
        let result = XfaExtractor::decode_stream_data(&dict, data).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_stream_data_empty_data_no_filters() {
        let dict = HashMap::new();
        let data = b"";
        let result = XfaExtractor::decode_stream_data(&dict, data).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_object_non_reference() {
        let obj = Object::Name("TestName".to_string());
        assert!(obj.as_reference().is_none());
    }

    #[test]
    fn test_packet_name_from_name_object() {
        let name_obj = Object::Name("template".to_string());
        if let Object::Name(n) = &name_obj {
            assert_eq!(n, "template");
        }
    }

    #[test]
    fn test_packet_name_from_string_object() {
        let string_obj = Object::String(b"config".to_vec());
        if let Object::String(s) = &string_obj {
            let name = String::from_utf8_lossy(s).to_string();
            assert_eq!(name, "config");
        }
    }

    #[test]
    fn test_packet_name_fallback() {
        let obj = Object::Integer(42);
        let i = 4_usize;
        let name = match &obj {
            Object::Name(n) => n.clone(),
            Object::String(s) => String::from_utf8_lossy(s).to_string(),
            _ => format!("packet_{}", i / 2),
        };
        assert_eq!(name, "packet_2");
    }

    #[test]
    fn test_packet_name_from_string_non_utf8() {
        let string_obj = Object::String(vec![0xFF, 0xFE, 0xFD]);
        if let Object::String(s) = &string_obj {
            let name = String::from_utf8_lossy(s).to_string();
            assert!(name.contains('\u{FFFD}'));
        }
    }

    fn build_pdf_with_xfa_array(stream: &[u8], repeats: usize) -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for body in [
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_string(),
            "<< /Type /Pages /Kids [] /Count 0 >>".to_string(),
            "<< >>".to_string(),
            format!("<< /XFA [{}] >>", "(template) 5 0 R ".repeat(repeats)),
        ] {
            offsets.push(pdf.len());
            let number = offsets.len();
            pdf.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", stream.len()).as_bytes());
        pdf.extend_from_slice(stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
        for offset in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
                offsets.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    #[test]
    fn repeated_xfa_packet_refs_cannot_exceed_the_decoded_budget() {
        let stream = vec![b' '; 5 * 1024 * 1024];
        let mut doc = PdfDocument::from_bytes(build_pdf_with_xfa_array(&stream, 64)).unwrap();

        assert!(XfaExtractor::extract_xfa(&mut doc).is_err());
        assert!(XfaExtractor::extract_xfa_packets(&mut doc).is_err());
    }

    #[test]
    fn repeated_xfa_packet_refs_within_budget_are_all_returned() {
        let mut doc = PdfDocument::from_bytes(build_pdf_with_xfa_array(b"<xdp/>", 3)).unwrap();

        assert_eq!(XfaExtractor::extract_xfa(&mut doc).unwrap(), b"<xdp/><xdp/><xdp/>");
        let packets = XfaExtractor::extract_xfa_packets(&mut doc).unwrap();
        assert_eq!(packets.len(), 3);
        assert_eq!(packets[0].0, "template");
    }
}
