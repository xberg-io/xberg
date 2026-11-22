//! Defines the [PdfPageText] struct, exposing functionality related to the
//! collection of Unicode characters visible on a single [PdfPage].

pub mod char;
pub mod chars;
pub mod search;
pub mod segment;
pub mod segments;

use crate::bindgen::{FPDF_TEXTPAGE, FPDF_WCHAR, FPDF_WIDESTRING};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::page::PdfPage;
use crate::pdf::document::page::annotation::PdfPageAnnotation;
use crate::pdf::document::page::annotation::PdfPageAnnotationCommon;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::text::PdfPageTextObject;
use crate::pdf::document::page::text::chars::{PdfPageTextCharIndex, PdfPageTextChars};
use crate::pdf::document::page::text::search::{PdfPageTextSearch, PdfSearchOptions};
use crate::pdf::document::page::text::segments::PdfPageTextSegments;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use crate::utils::mem::{create_byte_buffer, create_sized_buffer};
use crate::utils::utf16le::{get_pdfium_utf16le_bytes_from_str, get_string_from_pdfium_utf16le_bytes};
use bytemuck::cast_slice;
use std::fmt::{Display, Formatter};
use std::os::raw::{c_double, c_int};
use std::ptr::null_mut;

/// Parameters for [`should_emit_generated_space`], bundled so that helper stays under the
/// workspace parameter-count limit. ~keep
struct GeneratedSpaceSearch {
    /// First character index to search from (inclusive).
    next_search_start: i32,
    /// End of the character range (exclusive).
    end: i32,
    /// Right edge, in device units, of the character preceding the generated space.
    prev_right_x: f32,
    /// Font size to fall back to when the next real character reports none.
    prev_font_size: f32,
    space_ratio: f32,
}

/// Whether a generated space should be emitted, based on the horizontal gap between
/// `prev_right_x` and the next real (non-generated) character at or after `next_search_start`,
/// up to `end`. Matches the original inline decision exactly: no real character before `end`
/// (or a `GetCharBox` failure on the one found) also emits a space. Split out of
/// [`filter_generated_spaces_direct`] to keep that function's nesting under the workspace
/// limit. ~keep
fn should_emit_generated_space(
    text_page_handle: FPDF_TEXTPAGE,
    bindings: &dyn PdfiumLibraryBindings,
    search: GeneratedSpaceSearch,
) -> bool {
    let GeneratedSpaceSearch {
        next_search_start,
        end,
        prev_right_x,
        prev_font_size,
        space_ratio,
    } = search;
    let mut j = next_search_start;
    while j < end {
        let jdx = j as c_int;
        if bindings.FPDFText_IsGenerated(text_page_handle, jdx) == 0 {
            let mut left = 0.0_f64;
            let mut bottom = 0.0_f64;
            let mut right = 0.0_f64;
            let mut top = 0.0_f64;
            return if bindings.FPDFText_GetCharBox(text_page_handle, jdx, &mut left, &mut right, &mut bottom, &mut top)
                != 0
            {
                let gap = left as f32 - prev_right_x;
                let next_fs = bindings.FPDFText_GetFontSize(text_page_handle, jdx) as f32;
                let ref_fs = if next_fs > 0.0 { next_fs } else { prev_font_size };
                gap > ref_fs * space_ratio
            } else {
                true
            };
        }
        j += 1;
    }
    true
}

/// Shared gap-based space filtering for respaced text methods.
///
/// Iterates `chars` and builds a `String`, emitting a space for each generated
/// space character only when the horizontal gap to the next real character
/// exceeds `font_size * space_ratio`. This is the single implementation shared
/// by `all_respaced`, `inside_rect_respaced`, and `PdfPageTextSegment::text_respaced`.
/// Filter generated spaces using direct FFI calls for minimal overhead.
///
/// Single pass over character indices. For each character:
/// - Non-generated: 2 FFI calls (GetUnicode + GetCharBox)
/// - Generated space: 1 FFI call (IsGenerated), plus GetCharBox only for the next
///   non-generated char to measure the gap
///
/// This avoids the PdfPageTextChar wrapper overhead and minimizes FFI roundtrips.
pub(super) fn filter_generated_spaces_direct(
    text_page_handle: crate::bindgen::FPDF_TEXTPAGE,
    start: i32,
    count: i32,
    space_ratio: f32,
    bindings: &dyn PdfiumLibraryBindings,
) -> String {
    if count <= 0 {
        return String::new();
    }

    let end = start + count;
    let mut result = String::with_capacity(count as usize);
    let mut prev_right_x: Option<f32> = None;
    let mut prev_font_size: f32 = 12.0;

    let mut i = start;
    while i < end {
        let idx = i as std::os::raw::c_int;

        if bindings.FPDFText_IsGenerated(text_page_handle, idx) != 0 {
            if let Some(prev_r) = prev_right_x
                && should_emit_generated_space(
                    text_page_handle,
                    bindings,
                    GeneratedSpaceSearch {
                        next_search_start: i + 1,
                        end,
                        prev_right_x: prev_r,
                        prev_font_size,
                        space_ratio,
                    },
                )
            {
                result.push(' ');
            }
            i += 1;
            continue;
        }

        let unicode_val = bindings.FPDFText_GetUnicode(text_page_handle, idx);
        if let Some(uc) = char::from_u32(unicode_val) {
            if uc == '\r' {
                result.push('\n');
                prev_right_x = None;
                i += 1;
                continue;
            }
            if !uc.is_control() || uc == '\n' || uc == '\t' {
                result.push(uc);
            }
        }

        let mut left = 0.0_f64;
        let mut bottom = 0.0_f64;
        let mut right = 0.0_f64;
        let mut top = 0.0_f64;
        if bindings.FPDFText_GetCharBox(text_page_handle, idx, &mut left, &mut right, &mut bottom, &mut top) != 0 {
            prev_right_x = Some(right as f32);
        }

        let fs = bindings.FPDFText_GetFontSize(text_page_handle, idx) as f32;
        if fs > 0.0 {
            prev_font_size = fs;
        }

        i += 1;
    }

    result
}

/// The collection of Unicode characters visible on a single [PdfPage].
///
/// Use the [PdfPageText::all()] function to easily return all characters in the containing
/// [PdfPage] in the order in which they are defined in the PDF file.
///
/// Use the [PdfPageText::search()] function to initialise a new [PdfPageTextSearch] object,
/// yielding the results of searching for a target string within the character collection.
///
/// In complex custom layouts, the order in which characters are defined in the document
/// and the order in which they appear visually during rendering (and thus the order in
/// which they are read by a user) may not necessarily match.
///
/// [PdfPageText] implements both the [ToString] and the [Display] traits.
pub struct PdfPageText<'a> {
    text_page_handle: FPDF_TEXTPAGE,
    page: &'a PdfPage<'a>,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfPageText<'a> {
    pub(crate) fn from_pdfium(
        text_page_handle: FPDF_TEXTPAGE,
        page: &'a PdfPage<'a>,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfPageText {
            text_page_handle,
            page,
            bindings,
        }
    }

    /// Returns the internal `FPDF_TEXTPAGE` handle for this [PdfPageText].
    #[inline]
    pub(crate) fn text_page_handle(&self) -> FPDF_TEXTPAGE {
        self.text_page_handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfPageText].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the total number of characters in all text segments in the containing [PdfPage].
    ///
    /// The character count includes whitespace and newlines, and so may differ slightly
    /// from the result of calling `PdfPageText::all().len()`.
    #[inline]
    pub fn len(&self) -> i32 {
        self.bindings.FPDFText_CountChars(self.text_page_handle())
    }

    /// Returns `true` if there are no characters in any text box collection in the containing [PdfPage].
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a collection of all the `PdfPageTextSegment` text segments in the containing [PdfPage].
    #[inline]
    pub fn segments(&self) -> PdfPageTextSegments<'_> {
        PdfPageTextSegments::new(self, 0, self.len(), self.bindings())
    }

    /// Returns a subset of the `PdfPageTextSegment` text segments in the containing [PdfPage].
    /// Only text segments containing characters in the given index range will be included.
    #[inline]
    pub fn segments_subset(&self, start: PdfPageTextCharIndex, count: PdfPageTextCharIndex) -> PdfPageTextSegments<'_> {
        PdfPageTextSegments::new(self, start as i32, count as i32, self.bindings())
    }

    /// Returns a collection of all the `PdfPageTextChar` characters in the containing [PdfPage].
    #[inline]
    pub fn chars(&self) -> PdfPageTextChars<'_> {
        PdfPageTextChars::new(
            self.page.document_handle(),
            self.page.page_handle(),
            self.text_page_handle(),
            (0..self.len()).collect(),
            self.bindings(),
        )
    }

    /// Returns a collection of all the `PdfPageTextChar` characters in the given [PdfPageTextObject].
    ///
    /// The return result will be empty if the given [PdfPageTextObject] is not attached to the
    /// containing [PdfPage].
    #[inline]
    pub fn chars_for_object(&self, object: &PdfPageTextObject) -> Result<PdfPageTextChars<'_>, PdfiumError> {
        Ok(PdfPageTextChars::new(
            self.page.document_handle(),
            self.page.page_handle(),
            self.text_page_handle(),
            self.chars()
                .iter()
                .filter(|char| {
                    self.bindings
                        .FPDFText_GetTextObject(self.text_page_handle(), char.index() as i32)
                        == object.object_handle()
                })
                .map(|char| char.index() as i32)
                .collect(),
            self.bindings(),
        ))
    }

    /// Returns a collection of all the `PdfPageTextChar` characters in the given [PdfPageAnnotation].
    ///
    /// The return result will be empty if the given [PdfPageAnnotation] is not attached to the
    /// containing [PdfPage].
    #[inline]
    pub fn chars_for_annotation(&self, annotation: &PdfPageAnnotation) -> Result<PdfPageTextChars<'_>, PdfiumError> {
        self.chars_inside_rect(annotation.bounds()?)
            .map_err(|_| PdfiumError::NoCharsInAnnotation)
    }

    /// Returns a collection of all the `PdfPageTextChar` characters that lie within the bounds of
    /// the given [PdfRect] in the containing [PdfPage].
    #[inline]
    pub fn chars_inside_rect(&self, rect: PdfRect) -> Result<PdfPageTextChars<'_>, PdfiumError> {
        let tolerance_x = rect.width() / 2.0;
        let tolerance_y = rect.height() / 2.0;
        let center_height = rect.bottom() + tolerance_y;

        match (
            Self::get_char_index_near_point(
                self.text_page_handle(),
                rect.left(),
                tolerance_x,
                center_height,
                tolerance_y,
                self.bindings(),
            ),
            Self::get_char_index_near_point(
                self.text_page_handle(),
                rect.right(),
                tolerance_x,
                center_height,
                tolerance_y,
                self.bindings(),
            ),
        ) {
            (Some(start), Some(end)) => Ok(PdfPageTextChars::new(
                self.page.document_handle(),
                self.page.page_handle(),
                self.text_page_handle(),
                (start as i32..=end as i32 + 1).collect(),
                self.bindings,
            )),
            (Some(start), None) => Ok(PdfPageTextChars::new(
                self.page.document_handle(),
                self.page.page_handle(),
                self.text_page_handle(),
                (start as i32..=start as i32 + 1).collect(),
                self.bindings,
            )),
            (None, Some(end)) => Ok(PdfPageTextChars::new(
                self.page.document_handle(),
                self.page.page_handle(),
                self.text_page_handle(),
                (end as i32..=end as i32 + 1).collect(),
                self.bindings,
            )),
            _ => Err(PdfiumError::NoCharsInRect),
        }
    }

    /// Returns the character near to the given x and y positions on the containing [PdfPage],
    /// if any. The returned character will be no further from the given positions than the given
    /// tolerance values.
    pub(crate) fn get_char_index_near_point(
        text_page_handle: FPDF_TEXTPAGE,
        x: PdfPoints,
        tolerance_x: PdfPoints,
        y: PdfPoints,
        tolerance_y: PdfPoints,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Option<PdfPageTextCharIndex> {
        match bindings.FPDFText_GetCharIndexAtPos(
            text_page_handle,
            x.value as c_double,
            y.value as c_double,
            tolerance_x.value as c_double,
            tolerance_y.value as c_double,
        ) {
            -1 => None,
            -3 => None,
            index => Some(index as PdfPageTextCharIndex),
        }
    }

    /// Returns all characters that lie within the containing [PdfPage], in the order in which
    /// they are defined in the document, concatenated into a single string.
    ///
    /// In complex custom layouts, the order in which characters are defined in the document
    /// and the order in which they appear visually during rendering (and thus the order in
    /// which they are read by a user) may not necessarily match.
    pub fn all(&self) -> String {
        self.inside_rect(self.page.page_size())
    }

    /// Returns every character on the page via `FPDFText_GetText`, without a bounding rectangle.
    ///
    /// Unlike [`all()`], which clips to `(0, 0, width, height)` through `FPDFText_GetBoundedText`,
    /// this does not depend on the page's box origin or `/Rotate` matching that rectangle, so it
    /// cannot drop characters whose boxes fall outside it.
    pub fn whole_page(&self) -> String {
        let char_count = self.len();

        if char_count <= 0 {
            return String::new();
        }

        // FPDFText_GetText writes `count` characters plus a terminating NUL.
        let mut buffer = create_sized_buffer(char_count as usize + 1);

        let written = self
            .bindings()
            .FPDFText_GetText(self.text_page_handle(), 0, char_count, buffer.as_mut_ptr());

        if written <= 1 {
            return String::new();
        }

        // `written` counts the terminator.
        let units = (written as usize - 1).min(buffer.len());
        get_string_from_pdfium_utf16le_bytes(cast_slice(&buffer[..units]).to_vec()).unwrap_or_default()
    }

    /// Returns all page text with corrected word spacing, filtering out spurious
    /// spaces that pdfium inserts mid-word due to aggressive inter-glyph heuristics.
    ///
    /// For each generated space character (`is_generated() == true`), checks whether
    /// the horizontal gap between the preceding character's right edge and the following
    /// character's left edge exceeds `font_size * space_ratio`. Only inserts a space
    /// when the gap is large enough to represent a true word boundary.
    ///
    /// `space_ratio` controls sensitivity: 0.25 matches MinerU's threshold.
    pub fn all_respaced(&self, space_ratio: f32) -> String {
        let count = self.len();
        filter_generated_spaces_direct(self.text_page_handle(), 0, count, space_ratio, self.bindings)
    }

    /// Returns all characters that lie within the bounds of the given [PdfRect] in the
    /// containing [PdfPage], in the order in which they are defined in the document,
    /// concatenated into a single string.
    ///
    /// In complex custom layouts, the order in which characters are defined in the document
    /// and the order in which they appear visually during rendering (and thus the order in
    /// which they are read by a user) may not necessarily match.
    pub fn inside_rect(&self, rect: PdfRect) -> String {
        let left = rect.left().value as f64;

        let top = rect.top().value as f64;

        let right = rect.right().value as f64;

        let bottom = rect.bottom().value as f64;

        let chars_count =
            self.bindings()
                .FPDFText_GetBoundedText(self.text_page_handle(), left, top, right, bottom, null_mut(), 0);

        if chars_count == 0 {
            return String::new();
        }

        let mut buffer = create_sized_buffer(chars_count as usize);

        let result = self.bindings().FPDFText_GetBoundedText(
            self.text_page_handle(),
            left,
            top,
            right,
            bottom,
            buffer.as_mut_ptr(),
            chars_count,
        );

        assert_eq!(result, chars_count);

        get_string_from_pdfium_utf16le_bytes(cast_slice(buffer.as_slice()).to_vec()).unwrap_or_default()
    }

    /// Returns text within the given rectangle with corrected word spacing.
    ///
    /// Like [`inside_rect()`] but reconstructs text from individual characters,
    /// filtering out spurious spaces that pdfium inserts mid-word.
    /// See [`all_respaced()`] for details on the `space_ratio` parameter.
    pub fn inside_rect_respaced(&self, rect: PdfRect, space_ratio: f32) -> String {
        let chars = match self.chars_inside_rect(rect) {
            Ok(c) => c,
            Err(_) => {
                log::warn!("chars_inside_rect failed, falling back to unrespaced text");
                return self.inside_rect(rect);
            }
        };
        let count = chars.len();
        if count == 0 {
            return String::new();
        }
        let start = chars.first_char_index().unwrap_or(0) as i32;
        filter_generated_spaces_direct(self.text_page_handle(), start, count as i32, space_ratio, self.bindings)
    }

    /// Returns all characters assigned to the given [PdfPageTextObject] in this [PdfPageText] object,
    /// concatenated into a single string.
    pub fn for_object(&self, object: &PdfPageTextObject) -> String {
        let buffer_length =
            self.bindings()
                .FPDFTextObj_GetText(object.object_handle(), self.text_page_handle(), null_mut(), 0);

        if buffer_length == 0 {
            return String::new();
        }

        let mut buffer = create_byte_buffer(buffer_length as usize);

        let result = self.bindings().FPDFTextObj_GetText(
            object.object_handle(),
            self.text_page_handle(),
            buffer.as_mut_ptr() as *mut FPDF_WCHAR,
            buffer_length,
        );

        assert_eq!(result, buffer_length);

        get_string_from_pdfium_utf16le_bytes(buffer).unwrap_or_default()
    }

    /// Returns the raw `FPDF_PAGEOBJECT` handle for the text object that contains
    /// the character at the given index, or `None` if the index is out of range or
    /// the character is not associated with a text object (e.g. generated chars).
    ///
    /// The returned handle is an opaque pointer suitable for identity comparison
    /// (same pointer = same text object). Cast to `usize` for storage.
    pub fn text_object_for_char_index(&self, index: usize) -> Option<usize> {
        let handle = self
            .bindings()
            .FPDFText_GetTextObject(self.text_page_handle(), index as std::ffi::c_int);
        if handle.is_null() { None } else { Some(handle as usize) }
    }

    /// Returns all characters that lie within the bounds of the given [PdfPageAnnotation] in the
    /// containing [PdfPage], in the order in which they are defined in the document,
    /// concatenated into a single string.
    ///
    /// In complex custom layouts, the order in which characters are defined in the document
    /// and the order in which they appear visually during rendering (and thus the order in
    /// which they are read by a user) may not necessarily match.
    #[inline]
    pub fn for_annotation(&self, annotation: &PdfPageAnnotation) -> Result<String, PdfiumError> {
        let bounds = annotation.bounds()?;

        Ok(self.inside_rect(bounds))
    }

    /// Starts a search for the given text string, returning a new [PdfPageTextSearch]
    /// object that can be used to step through the search results.
    #[inline]
    pub fn search(&self, text: &str, options: &PdfSearchOptions) -> Result<PdfPageTextSearch<'_>, PdfiumError> {
        self.search_from(text, options, 0)
    }

    /// Starts a search for the given test string from the given character position,
    /// returning a new [PdfPageTextSearch] object that can be used to step through
    /// the search results.
    pub fn search_from(
        &self,
        text: &str,
        options: &PdfSearchOptions,
        index: PdfPageTextCharIndex,
    ) -> Result<PdfPageTextSearch<'_>, PdfiumError> {
        if text.is_empty() {
            Err(PdfiumError::TextSearchTargetIsEmpty)
        } else {
            Ok(PdfPageTextSearch::from_pdfium(
                self.bindings().FPDFText_FindStart(
                    self.text_page_handle(),
                    get_pdfium_utf16le_bytes_from_str(text).as_ptr() as FPDF_WIDESTRING,
                    options.as_pdfium(),
                    index as c_int,
                ),
                self,
                self.bindings(),
            ))
        }
    }
}

impl<'a> Display for PdfPageText<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.all().as_str())
    }
}

impl<'a> Drop for PdfPageText<'a> {
    /// Closes the [PdfPageText] collection, releasing held memory.
    #[inline]
    fn drop(&mut self) {
        self.bindings().FPDFText_ClosePage(self.text_page_handle());
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use std::ffi::OsStr;
    use std::fs;

    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};

    #[test]
    fn test_overlapping_chars_results() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let font = document.fonts_mut().courier();

        let txt1 = page.objects_mut().create_text_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            "AAAAAA",
            font,
            PdfPoints::new(10.0),
        )?;

        let txt2 = page.objects_mut().create_text_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            "BBBBBB",
            font,
            PdfPoints::new(10.0),
        )?;

        let txt3 = page.objects_mut().create_text_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            "CDCDCDE",
            font,
            PdfPoints::new(10.0),
        )?;

        let page_text = page.text()?;

        assert!(test_one_overlapping_text_object_results(&txt1, &page_text, "AAAAAA")?);
        assert!(test_one_overlapping_text_object_results(&txt2, &page_text, "BBBBBB")?);
        assert!(test_one_overlapping_text_object_results(&txt3, &page_text, "CDCDCDE")?);

        Ok(())
    }

    // Glyph A maps through ToUnicode to U+20000, a surrogate pair in UTF-16. The cross-reference table is
    // omitted; pdfium rebuilds it by scanning the objects.
    const SUPPLEMENTARY_PLANE_PDF: &[u8] = b"%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R
  /Resources << /Font << /F1 5 0 R >> >> >> endobj
4 0 obj << /Length 33 >> stream
BT /F1 24 Tf 50 700 Td (AB) Tj ET
endstream endobj
5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >> endobj
6 0 obj << /Length 160 >> stream
/CIDInit /ProcSet findresource begin 12 dict begin begincmap
1 begincodespacerange <00> <FF> endcodespacerange
2 beginbfchar
<41> <D840DC00>
<42> <0042>
endbfchar
endcmap end end
endstream endobj
trailer << /Root 1 0 R >>
%%EOF
";

    #[test]
    fn whole_page_keeps_supplementary_plane_characters() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();
        let document = pdfium.load_pdf_from_byte_slice(SUPPLEMENTARY_PLANE_PDF, None)?;
        let page = document.pages().get(0)?;

        assert_eq!(page.text()?.whole_page().trim(), "\u{20000}B");

        Ok(())
    }

    fn test_one_overlapping_text_object_results(
        object: &PdfPageObject,
        page_text: &PdfPageText,
        expected: &str,
    ) -> Result<bool, PdfiumError> {
        if let Some(txt) = object.as_text_object() {
            assert_eq!(txt.text().trim(), expected);
            assert_eq!(page_text.for_object(txt).trim(), expected);

            for (index, char) in txt.chars(page_text)?.iter().enumerate() {
                assert_eq!(txt.text().chars().nth(index), char.unicode_char());
                assert_eq!(expected.chars().nth(index), char.unicode_char());
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[test]
    fn test_text_chars_results_equality() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let fixture_dir = test_fixture_path("");
        let samples = fs::read_dir(&fixture_dir)
            .unwrap()
            .filter_map(|entry| match entry {
                Ok(e) => Some(e.path()),
                Err(_) => None,
            })
            .filter(|path| path.extension() == Some(OsStr::new("pdf")))
            .collect::<Vec<_>>();

        assert!(!samples.is_empty());

        for sample in samples {
            println!("Testing all text objects in file {}", sample.display());

            let document = pdfium.load_pdf_from_file(&sample, None)?;

            for page in document.pages().iter() {
                let text = page.text()?;

                for object in page.objects().iter() {
                    if let Some(obj) = object.as_text_object() {
                        let chars = obj
                            .chars(&text)?
                            .iter()
                            .filter_map(|char| char.unicode_string())
                            .join("");

                        assert_eq!(obj.text().trim(), chars.replace("\0", "").trim());
                    }
                }
            }
        }

        Ok(())
    }
}
