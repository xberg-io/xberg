//! Defines the [PdfStructElement] struct, exposing functionality related to a single
//! structure element in a PDF structure tree.

use crate::bindgen::FPDF_STRUCTELEMENT;
use crate::bindings::PdfiumLibraryBindings;
use crate::utils::mem::create_byte_buffer;
use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
use std::os::raw::{c_int, c_ulong, c_void};

/// The type of a PDF structure element, corresponding to the standard structure types
/// defined in the PDF specification (ISO 32000).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PdfStructElementType {
    Document,
    Part,
    Div,
    Span,
    P,
    H,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Table,
    TR,
    TH,
    TD,
    THead,
    TBody,
    TFoot,
    L,
    LI,
    Lbl,
    LBody,
    Figure,
    Formula,
    Form,
    Code,
    BlockQuote,
    Caption,
    Link,
    Note,
    TOC,
    TOCI,
    Sect,
    Art,
    Reference,
    BibEntry,
    Quote,
    Index,
    NonStruct,
    Other(String),
}

impl PdfStructElementType {
    /// Parses a PDF structure type string (the /S entry) into a [PdfStructElementType].
    pub fn from_pdf_type_string(s: &str) -> Self {
        match s {
            "Document" => Self::Document,
            "Part" => Self::Part,
            "Div" => Self::Div,
            "Span" => Self::Span,
            "P" => Self::P,
            "H" => Self::H,
            "H1" => Self::H1,
            "H2" => Self::H2,
            "H3" => Self::H3,
            "H4" => Self::H4,
            "H5" => Self::H5,
            "H6" => Self::H6,
            "Table" => Self::Table,
            "TR" => Self::TR,
            "TH" => Self::TH,
            "TD" => Self::TD,
            "THead" => Self::THead,
            "TBody" => Self::TBody,
            "TFoot" => Self::TFoot,
            "L" => Self::L,
            "LI" => Self::LI,
            "Lbl" => Self::Lbl,
            "LBody" => Self::LBody,
            "Figure" => Self::Figure,
            "Formula" => Self::Formula,
            "Form" => Self::Form,
            "Code" => Self::Code,
            "BlockQuote" => Self::BlockQuote,
            "Caption" => Self::Caption,
            "Link" => Self::Link,
            "Note" => Self::Note,
            "TOC" => Self::TOC,
            "TOCI" => Self::TOCI,
            "Sect" => Self::Sect,
            "Art" => Self::Art,
            "Reference" => Self::Reference,
            "BibEntry" => Self::BibEntry,
            "Quote" => Self::Quote,
            "Index" => Self::Index,
            "NonStruct" => Self::NonStruct,
            other => Self::Other(other.to_string()),
        }
    }

    /// Returns `true` if this element type is a heading (H, H1-H6).
    pub fn is_heading(&self) -> bool {
        matches!(
            self,
            Self::H | Self::H1 | Self::H2 | Self::H3 | Self::H4 | Self::H5 | Self::H6
        )
    }

    /// Returns the heading level (1-6) for H1-H6 elements, or `None` for non-heading elements
    /// and the generic H element.
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            Self::H1 => Some(1),
            Self::H2 => Some(2),
            Self::H3 => Some(3),
            Self::H4 => Some(4),
            Self::H5 => Some(5),
            Self::H6 => Some(6),
            _ => None,
        }
    }

    /// Returns `true` if this element type is a table-related element
    /// (Table, TR, TH, TD, THead, TBody, TFoot).
    pub fn is_table_element(&self) -> bool {
        matches!(
            self,
            Self::Table | Self::TR | Self::TH | Self::TD | Self::THead | Self::TBody | Self::TFoot
        )
    }

    /// Returns `true` if this element type is a list-related element (L, LI, Lbl, LBody).
    pub fn is_list_element(&self) -> bool {
        matches!(self, Self::L | Self::LI | Self::Lbl | Self::LBody)
    }

    /// Returns `true` if this element type is a block-level element.
    pub fn is_block_level(&self) -> bool {
        matches!(
            self,
            Self::Document
                | Self::Part
                | Self::Div
                | Self::Sect
                | Self::Art
                | Self::P
                | Self::H
                | Self::H1
                | Self::H2
                | Self::H3
                | Self::H4
                | Self::H5
                | Self::H6
                | Self::Table
                | Self::L
                | Self::Figure
                | Self::Formula
                | Self::Form
                | Self::BlockQuote
                | Self::Code
                | Self::TOC
                | Self::TOCI
                | Self::Index
                | Self::Caption
                | Self::Note
        )
    }
}

/// A single element in the structure tree of a tagged PDF page.
///
/// Structure elements carry semantic information such as element type (paragraph,
/// heading, table cell, etc.), alternative text, actual text, language, and
/// marked content identifiers that associate the element with content on the page.
///
/// The element handle is not owned by this struct; it is owned by the parent
/// `FPDF_STRUCTTREE` and remains valid for as long as that tree is open.
#[derive(Clone)]
pub struct PdfStructElement<'a> {
    element_handle: FPDF_STRUCTELEMENT,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfStructElement<'a> {
    pub(crate) fn from_pdfium(element_handle: FPDF_STRUCTELEMENT, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self {
            element_handle,
            bindings,
        }
    }

    /// Extracts a UTF-16LE string from pdfium using the standard two-call buffer pattern.
    /// The `get_fn` is called first with a null buffer to determine the required size,
    /// then again with a properly sized buffer.
    fn extract_utf16_string<F>(&self, get_fn: F) -> Option<String>
    where
        F: Fn(FPDF_STRUCTELEMENT, *mut c_void, c_ulong) -> c_ulong,
    {
        let buffer_length = get_fn(self.element_handle, std::ptr::null_mut(), 0);

        if buffer_length == 0 {
            return None;
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = get_fn(self.element_handle, buffer.as_mut_ptr() as *mut c_void, buffer_length);

        assert_eq!(result, buffer_length);

        get_string_from_pdfium_utf16le_bytes(buffer)
    }

    /// Returns the parsed [PdfStructElementType] of this element.
    pub fn element_type(&self) -> PdfStructElementType {
        match self.element_type_raw() {
            Some(raw) => PdfStructElementType::from_pdf_type_string(&raw),
            None => PdfStructElementType::Other(String::new()),
        }
    }

    /// Returns the raw type string (/S entry) of this element, if any.
    pub fn element_type_raw(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetType(handle, buf, len))
    }

    /// Returns the title (/T entry) of this element, if any.
    pub fn title(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetTitle(handle, buf, len))
    }

    /// Returns the alternative text (/Alt entry) of this element, if any.
    /// This is typically used for accessibility purposes.
    pub fn alt_text(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetAltText(handle, buf, len))
    }

    /// Returns the actual text (/ActualText entry) of this element, if any.
    pub fn actual_text(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetActualText(handle, buf, len))
    }

    /// Returns the ID of this element, if any.
    pub fn id(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetID(handle, buf, len))
    }

    /// Returns the language (IETF BCP 47 code) of this element, if any.
    pub fn lang(&self) -> Option<String> {
        self.extract_utf16_string(|handle, buf, len| self.bindings.FPDF_StructElement_GetLang(handle, buf, len))
    }

    /// Returns the primary marked content ID of this element, or `None` if no ID exists.
    ///
    /// Consider using [PdfStructElement::all_marked_content_ids] to retrieve all MCIDs,
    /// as an element may have more than one.
    pub fn marked_content_id(&self) -> Option<i32> {
        let id = self.bindings.FPDF_StructElement_GetMarkedContentID(self.element_handle);
        if id == -1 { None } else { Some(id) }
    }

    /// Returns the count of marked content IDs associated with this element.
    pub fn marked_content_id_count(&self) -> usize {
        let count = self
            .bindings
            .FPDF_StructElement_GetMarkedContentIdCount(self.element_handle);
        if count < 0 { 0 } else { count as usize }
    }

    /// Returns the marked content ID at the given index, or `None` if the index is
    /// out of bounds or no ID exists at that index.
    pub fn marked_content_id_at_index(&self, index: usize) -> Option<i32> {
        let id = self
            .bindings
            .FPDF_StructElement_GetMarkedContentIdAtIndex(self.element_handle, index as c_int);
        if id == -1 { None } else { Some(id) }
    }

    /// Returns all marked content IDs associated with this element.
    pub fn all_marked_content_ids(&self) -> Vec<i32> {
        let count = self.marked_content_id_count();
        let mut ids = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(id) = self.marked_content_id_at_index(i) {
                ids.push(id);
            }
        }
        ids
    }

    /// Returns the parent structure element, or `None` if this is a root element.
    pub fn parent(&self) -> Option<PdfStructElement<'a>> {
        let handle = self.bindings.FPDF_StructElement_GetParent(self.element_handle);
        if handle.is_null() {
            None
        } else {
            Some(PdfStructElement::from_pdfium(handle, self.bindings))
        }
    }

    /// Returns the number of direct children of this element.
    pub fn children_count(&self) -> usize {
        let count = self.bindings.FPDF_StructElement_CountChildren(self.element_handle);
        if count < 0 { 0 } else { count as usize }
    }

    /// Returns the child element at the given index, or `None` if the index is out of
    /// bounds or the child at that index is not a structure element (e.g. it is a
    /// marked-content reference).
    pub fn child_at_index(&self, index: usize) -> Option<PdfStructElement<'a>> {
        let handle = self
            .bindings
            .FPDF_StructElement_GetChildAtIndex(self.element_handle, index as c_int);
        if handle.is_null() {
            None
        } else {
            Some(PdfStructElement::from_pdfium(handle, self.bindings))
        }
    }

    /// Returns an iterator over the direct children of this element.
    pub fn children(&self) -> PdfStructElementChildrenIterator<'a> {
        PdfStructElementChildrenIterator {
            element: self.clone(),
            count: self.children_count(),
            index: 0,
        }
    }

    /// Returns the number of attributes on this element.
    pub fn attribute_count(&self) -> usize {
        let count = self.bindings.FPDF_StructElement_GetAttributeCount(self.element_handle);
        if count < 0 { 0 } else { count as usize }
    }

    /// Returns the value of a string attribute with the given name, if any.
    pub fn string_attribute(&self, name: &str) -> Option<String> {
        let buffer_length =
            self.bindings
                .FPDF_StructElement_GetStringAttribute(self.element_handle, name, std::ptr::null_mut(), 0);

        if buffer_length == 0 {
            return None;
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings.FPDF_StructElement_GetStringAttribute(
            self.element_handle,
            name,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        get_string_from_pdfium_utf16le_bytes(buffer)
    }
}

/// An iterator over the direct children of a [PdfStructElement].
pub struct PdfStructElementChildrenIterator<'a> {
    element: PdfStructElement<'a>,
    count: usize,
    index: usize,
}

impl<'a> Iterator for PdfStructElementChildrenIterator<'a> {
    type Item = PdfStructElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let current = self.index;
            self.index += 1;
            // Some children may not be elements (e.g. marked-content references),
            // so child_at_index returns None for those. Skip them.
            if let Some(child) = self.element.child_at_index(current) {
                return Some(child);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_pdf_type_string_all_standard_types() {
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Document"),
            PdfStructElementType::Document
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Part"),
            PdfStructElementType::Part
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Div"),
            PdfStructElementType::Div
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Span"),
            PdfStructElementType::Span
        );
        assert_eq!(PdfStructElementType::from_pdf_type_string("P"), PdfStructElementType::P);
        assert_eq!(PdfStructElementType::from_pdf_type_string("H"), PdfStructElementType::H);
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H1"),
            PdfStructElementType::H1
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H2"),
            PdfStructElementType::H2
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H3"),
            PdfStructElementType::H3
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H4"),
            PdfStructElementType::H4
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H5"),
            PdfStructElementType::H5
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("H6"),
            PdfStructElementType::H6
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Table"),
            PdfStructElementType::Table
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TR"),
            PdfStructElementType::TR
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TH"),
            PdfStructElementType::TH
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TD"),
            PdfStructElementType::TD
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("THead"),
            PdfStructElementType::THead
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TBody"),
            PdfStructElementType::TBody
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TFoot"),
            PdfStructElementType::TFoot
        );
        assert_eq!(PdfStructElementType::from_pdf_type_string("L"), PdfStructElementType::L);
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("LI"),
            PdfStructElementType::LI
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Lbl"),
            PdfStructElementType::Lbl
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("LBody"),
            PdfStructElementType::LBody
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Figure"),
            PdfStructElementType::Figure
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Formula"),
            PdfStructElementType::Formula
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Form"),
            PdfStructElementType::Form
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Code"),
            PdfStructElementType::Code
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("BlockQuote"),
            PdfStructElementType::BlockQuote
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Caption"),
            PdfStructElementType::Caption
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Link"),
            PdfStructElementType::Link
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Note"),
            PdfStructElementType::Note
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TOC"),
            PdfStructElementType::TOC
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("TOCI"),
            PdfStructElementType::TOCI
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Sect"),
            PdfStructElementType::Sect
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Art"),
            PdfStructElementType::Art
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Reference"),
            PdfStructElementType::Reference
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("BibEntry"),
            PdfStructElementType::BibEntry
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Quote"),
            PdfStructElementType::Quote
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("Index"),
            PdfStructElementType::Index
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("NonStruct"),
            PdfStructElementType::NonStruct
        );
    }

    #[test]
    fn test_from_pdf_type_string_unknown() {
        assert_eq!(
            PdfStructElementType::from_pdf_type_string("CustomType"),
            PdfStructElementType::Other("CustomType".to_string())
        );
        assert_eq!(
            PdfStructElementType::from_pdf_type_string(""),
            PdfStructElementType::Other(String::new())
        );
    }

    #[test]
    fn test_is_heading() {
        assert!(PdfStructElementType::H.is_heading());
        assert!(PdfStructElementType::H1.is_heading());
        assert!(PdfStructElementType::H2.is_heading());
        assert!(PdfStructElementType::H3.is_heading());
        assert!(PdfStructElementType::H4.is_heading());
        assert!(PdfStructElementType::H5.is_heading());
        assert!(PdfStructElementType::H6.is_heading());

        assert!(!PdfStructElementType::P.is_heading());
        assert!(!PdfStructElementType::Table.is_heading());
        assert!(!PdfStructElementType::Document.is_heading());
        assert!(!PdfStructElementType::Other("H7".to_string()).is_heading());
    }

    #[test]
    fn test_heading_level() {
        assert_eq!(PdfStructElementType::H1.heading_level(), Some(1));
        assert_eq!(PdfStructElementType::H2.heading_level(), Some(2));
        assert_eq!(PdfStructElementType::H3.heading_level(), Some(3));
        assert_eq!(PdfStructElementType::H4.heading_level(), Some(4));
        assert_eq!(PdfStructElementType::H5.heading_level(), Some(5));
        assert_eq!(PdfStructElementType::H6.heading_level(), Some(6));

        // H (generic) has no specific level
        assert_eq!(PdfStructElementType::H.heading_level(), None);
        assert_eq!(PdfStructElementType::P.heading_level(), None);
    }

    #[test]
    fn test_is_table_element() {
        assert!(PdfStructElementType::Table.is_table_element());
        assert!(PdfStructElementType::TR.is_table_element());
        assert!(PdfStructElementType::TH.is_table_element());
        assert!(PdfStructElementType::TD.is_table_element());
        assert!(PdfStructElementType::THead.is_table_element());
        assert!(PdfStructElementType::TBody.is_table_element());
        assert!(PdfStructElementType::TFoot.is_table_element());

        assert!(!PdfStructElementType::P.is_table_element());
        assert!(!PdfStructElementType::L.is_table_element());
    }

    #[test]
    fn test_is_list_element() {
        assert!(PdfStructElementType::L.is_list_element());
        assert!(PdfStructElementType::LI.is_list_element());
        assert!(PdfStructElementType::Lbl.is_list_element());
        assert!(PdfStructElementType::LBody.is_list_element());

        assert!(!PdfStructElementType::P.is_list_element());
        assert!(!PdfStructElementType::Table.is_list_element());
    }

    #[test]
    fn test_is_block_level() {
        // Block-level elements
        assert!(PdfStructElementType::Document.is_block_level());
        assert!(PdfStructElementType::Part.is_block_level());
        assert!(PdfStructElementType::P.is_block_level());
        assert!(PdfStructElementType::H1.is_block_level());
        assert!(PdfStructElementType::Table.is_block_level());
        assert!(PdfStructElementType::L.is_block_level());
        assert!(PdfStructElementType::Figure.is_block_level());
        assert!(PdfStructElementType::BlockQuote.is_block_level());
        assert!(PdfStructElementType::Code.is_block_level());

        // Inline elements
        assert!(!PdfStructElementType::Span.is_block_level());
        assert!(!PdfStructElementType::Link.is_block_level());
        assert!(!PdfStructElementType::TD.is_block_level());
        assert!(!PdfStructElementType::TH.is_block_level());
        assert!(!PdfStructElementType::TR.is_block_level());
        assert!(!PdfStructElementType::LI.is_block_level());
        assert!(!PdfStructElementType::Lbl.is_block_level());
        assert!(!PdfStructElementType::LBody.is_block_level());
    }

    #[test]
    fn test_element_type_equality() {
        assert_eq!(PdfStructElementType::P, PdfStructElementType::P);
        assert_ne!(PdfStructElementType::P, PdfStructElementType::H1);
        assert_eq!(
            PdfStructElementType::Other("X".to_string()),
            PdfStructElementType::Other("X".to_string())
        );
        assert_ne!(
            PdfStructElementType::Other("X".to_string()),
            PdfStructElementType::Other("Y".to_string())
        );
    }

    #[test]
    fn test_element_type_clone() {
        let t = PdfStructElementType::H1;
        let cloned = t.clone();
        assert_eq!(t, cloned);

        let other = PdfStructElementType::Other("Custom".to_string());
        let cloned = other.clone();
        assert_eq!(other, cloned);
    }

    #[test]
    fn test_element_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PdfStructElementType::P);
        set.insert(PdfStructElementType::H1);
        set.insert(PdfStructElementType::P); // duplicate
        assert_eq!(set.len(), 2);
    }
}
