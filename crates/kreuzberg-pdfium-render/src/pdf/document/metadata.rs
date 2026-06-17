//! Defines the [PdfMetadata] struct, a collection of all the metadata tags in a [PdfDocument].

use crate::bindgen::FPDF_DOCUMENT;
use crate::bindings::PdfiumLibraryBindings;
use crate::utils::mem::create_byte_buffer;
use crate::utils::utf16le::get_string_from_pdfium_utf16le_bytes;
use std::os::raw::c_void;
use std::slice::Iter;

#[cfg(doc)]
use crate::pdf::document::PdfDocument;

/// Valid metadata tag types in a [PdfDocument].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfDocumentMetadataTagType {
    Title,
    Author,
    Subject,
    Keywords,
    Creator,
    Producer,
    CreationDate,
    ModificationDate,
}

/// A single metadata tag in a [PdfDocument].
#[derive(Debug, Clone, PartialEq)]
pub struct PdfDocumentMetadataTag {
    tag: PdfDocumentMetadataTagType,
    value: String,
}

impl PdfDocumentMetadataTag {
    #[inline]
    pub(crate) fn new(tag: PdfDocumentMetadataTagType, value: String) -> Self {
        PdfDocumentMetadataTag { tag, value }
    }

    /// Returns the type of this metadata tag.
    #[inline]
    pub fn tag_type(&self) -> PdfDocumentMetadataTagType {
        self.tag
    }

    /// Returns the value of this metadata tag.
    #[inline]
    pub fn value(&self) -> &str {
        self.value.as_str()
    }
}

/// A collection of all the metadata tags in a [PdfDocument].
pub struct PdfMetadata<'a> {
    document_handle: FPDF_DOCUMENT,
    bindings: &'a dyn PdfiumLibraryBindings,
    tags: Vec<PdfDocumentMetadataTag>,
}

impl<'a> PdfMetadata<'a> {
    pub(crate) fn from_pdfium(document_handle: FPDF_DOCUMENT, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        let mut result = PdfMetadata {
            document_handle,
            bindings,
            tags: vec![],
        };

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Title) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Author) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Subject) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Keywords) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Creator) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::Producer) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::CreationDate) {
            result.tags.push(tag);
        }

        if let Some(tag) = result.get(PdfDocumentMetadataTagType::ModificationDate) {
            result.tags.push(tag);
        }

        result
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfMetadata] collection.
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the number of metadata tags in this [PdfMetadata] collection.
    #[inline]
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Returns true if this [PdfMetadata] collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns one metadata tag from this [PdfMetadata] collection, if it is defined.
    pub fn get(&self, tag: PdfDocumentMetadataTagType) -> Option<PdfDocumentMetadataTag> {
        let result = match tag {
            PdfDocumentMetadataTagType::Title => self.get_raw_metadata_tag("Title"),
            PdfDocumentMetadataTagType::Author => self.get_raw_metadata_tag("Author"),
            PdfDocumentMetadataTagType::Subject => self.get_raw_metadata_tag("Subject"),
            PdfDocumentMetadataTagType::Keywords => self.get_raw_metadata_tag("Keywords"),
            PdfDocumentMetadataTagType::Creator => self.get_raw_metadata_tag("Creator"),
            PdfDocumentMetadataTagType::Producer => self.get_raw_metadata_tag("Producer"),
            PdfDocumentMetadataTagType::CreationDate => self.get_raw_metadata_tag("CreationDate"),
            PdfDocumentMetadataTagType::ModificationDate => self.get_raw_metadata_tag("ModificationDate"),
        };

        result.map(|value| PdfDocumentMetadataTag::new(tag, value))
    }

    #[inline]
    fn get_raw_metadata_tag(&self, tag: &str) -> Option<String> {
        // Retrieving the tag text from Pdfium is a two-step operation. First, we call
        // FPDF_GetMetaText() with a null buffer; this will retrieve the length of
        // the metadata text in bytes. If the length is zero, then there is no such tag.

        // If the length is non-zero, then we reserve a byte buffer of the given
        // length and call FPDF_GetMetaText() again with a pointer to the buffer;
        // this will write the metadata text to the buffer in UTF16-LE format.

        let buffer_length = self
            .bindings
            .FPDF_GetMetaText(self.document_handle, tag, std::ptr::null_mut(), 0);

        if buffer_length == 0 {
            // The tag is not present.

            return None;
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings.FPDF_GetMetaText(
            self.document_handle,
            tag,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        get_string_from_pdfium_utf16le_bytes(buffer)
    }

    /// Returns an iterator over all the tags in this [PdfMetadata] collection.
    #[inline]
    pub fn iter(&self) -> Iter<'_, PdfDocumentMetadataTag> {
        self.tags.iter()
    }
}
