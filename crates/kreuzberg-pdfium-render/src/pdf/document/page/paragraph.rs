//! Defines the [PdfParagraph] struct, exposing functionality related to a group of
//! styled text strings that should be laid out together on a `PdfPage` as single paragraph.

use crate::bindgen::FPDF_PAGEOBJECT;
use crate::error::PdfiumError;
use crate::pdf::document::PdfDocument;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::text::PdfPageTextObject;
use crate::pdf::document::page::object::{PdfPageObject, PdfPageObjectCommon};
use crate::pdf::font::{PdfFont, PdfFontWeight};
use crate::pdf::points::PdfPoints;
use itertools::Itertools;
use maybe_owned::MaybeOwned;
use std::cmp::Ordering;

/// Update an `Option<PdfPoints>` to track the minimum value seen.
fn update_min(slot: &mut Option<PdfPoints>, value: PdfPoints) {
    match *slot {
        Some(current) if current <= value => {}
        _ => *slot = Some(value),
    }
}

/// Update an `Option<PdfPoints>` to track the maximum value seen.
fn update_max(slot: &mut Option<PdfPoints>, value: PdfPoints) {
    match *slot {
        Some(current) if current >= value => {}
        _ => *slot = Some(value),
    }
}

/// A single styled string in a [PdfParagraph].
pub struct PdfStyledString<'a> {
    text: String,
    font: MaybeOwned<'a, PdfFont<'a>>,
    font_size: PdfPoints,
}

impl<'a> PdfStyledString<'a> {
    /// Creates a new [PdfStyledString] from the given arguments.
    #[inline]
    pub fn new(text: String, font: &'a PdfFont<'a>, font_size: PdfPoints) -> Self {
        PdfStyledString {
            text,
            font: MaybeOwned::Borrowed(font),
            font_size,
        }
    }

    /// Creates a new [PdfStyledString] from the given [PdfPageTextObject].
    #[inline]
    pub fn from_text_object(text_object: &'a PdfPageTextObject<'a>) -> Self {
        PdfStyledString {
            text: text_object.text(),
            font: MaybeOwned::Owned(text_object.font()),
            font_size: text_object.unscaled_font_size(),
        }
    }

    /// Adds the given string to the text in this [PdfStyledString]. The given separator will be used
    /// to separate the existing text in this [PdfStyledString] from the given string.
    #[inline]
    pub(crate) fn push(&mut self, text: impl ToString, separator: &str) {
        if !self.text.ends_with(separator) {
            self.text.push_str(separator);
        }

        self.text.push_str(text.to_string().as_str());
    }

    /// Returns the text in this [PdfStyledString].
    #[inline]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Returns the [PdfFont] used to style this [PdfStyledString].
    #[inline]
    pub fn font(&self) -> &PdfFont<'_> {
        self.font.as_ref()
    }

    /// Returns the font size used to style this [PdfStyledString].
    #[inline]
    pub fn font_size(&self) -> PdfPoints {
        self.font_size
    }

    /// Returns `true` if the font and font size of this [PdfStyledString] is the same as
    /// that of the given string.
    #[inline]
    pub fn does_match_string_styling(&self, other: &PdfStyledString) -> bool {
        self.does_match_raw_styling(other.font_size(), other.font())
    }

    /// Returns `true` if the font and font size of this [PdfStyledString] is the same as
    /// that of the given [PdfPageTextObject].
    #[inline]
    pub fn does_match_object_styling(&self, other: &PdfPageTextObject) -> bool {
        self.does_match_raw_styling(other.unscaled_font_size(), &other.font())
    }

    /// Returns `true` if this styled string's font is bold.
    ///
    /// Checks the font descriptor's force-bold flag, the font weight (>= 700),
    /// and the font family name for "bold" substring.
    pub fn is_bold(&self) -> bool {
        let font = self.font();

        if font.is_bold_reenforced() {
            return true;
        }

        if let Ok(weight) = font.weight()
            && matches!(
                weight,
                PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | PdfFontWeight::Weight900
            )
        {
            return true;
        }

        font.family().to_lowercase().contains("bold")
    }

    /// Returns `true` if this styled string's font is italic.
    ///
    /// Checks the font descriptor's italic flag and the font family name
    /// for "italic" or "oblique" substrings.
    pub fn is_italic(&self) -> bool {
        let font = self.font();

        if font.is_italic() {
            return true;
        }

        let name = font.family().to_lowercase();
        name.contains("italic") || name.contains("oblique")
    }

    /// Returns `true` if this styled string's font is monospace.
    ///
    /// Checks the font descriptor's fixed-pitch flag and the font family name
    /// against common monospace font patterns.
    pub fn is_monospace(&self) -> bool {
        let font = self.font();

        if font.is_fixed_pitch() {
            return true;
        }

        let name = font.family().to_lowercase();
        const MONOSPACE_PATTERNS: &[&str] = &[
            "mono",
            "courier",
            "consolas",
            "menlo",
            "source code",
            "inconsolata",
            "fira code",
            "liberation mono",
            "lucida console",
            "andale mono",
            "dejavu sans mono",
            "roboto mono",
            "noto mono",
            "ibm plex mono",
            "jetbrains mono",
            "cascadia",
            "hack",
        ];
        MONOSPACE_PATTERNS.iter().any(|p| name.contains(p))
    }

    fn does_match_raw_styling(&self, other_font_size: PdfPoints, other_font: &PdfFont) -> bool {
        // It's more expensive to try to match the fonts based on name, so we try to match
        // based on FPDF_FONT handles first.

        if self.font_size() != other_font_size {
            return false;
        }

        let this_font = self.font();

        if this_font.handle() != other_font.handle() {
            return false;
        }

        let this_font_name = this_font.family();

        let other_font_name = other_font.family();

        if this_font_name.is_empty() && other_font_name.is_empty() {
            // We can't distinguish based on font names, and the sizes and font handles are identical,
            // so best guess is the styling matches.

            return true;
        }

        (!this_font_name.is_empty() || !other_font_name.is_empty()) && this_font_name == other_font_name
    }

    /// Creates a new [PdfPageTextObject] from this styled string, using the Pdfium bindings in
    /// the given document.
    #[inline]
    pub fn as_text_object(&self, document: &PdfDocument<'a>) -> Result<PdfPageTextObject<'a>, PdfiumError> {
        PdfPageTextObject::new(document, self.text(), self.font(), self.font_size())
    }
}

/// A single fragment in a [PdfParagraph]. The fragment may later be split into sub-fragments when
/// assembling the [PdfParagraph] into lines.
pub enum PdfParagraphFragment<'a> {
    /// A run of styled text.
    StyledString(PdfStyledString<'a>),
    /// A line break with alignment and position information from the preceding line.
    LineBreak {
        alignment: PdfLineAlignment,
        bottom: PdfPoints,
        left: PdfPoints,
    },
    /// A non-text page object (image, path, shading, etc.).
    NonTextObject(FPDF_PAGEOBJECT),
}

/// Controls the line alignment behaviour of a [PdfParagraph].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfParagraphAlignment {
    /// All lines will be non-justified, aligned to the left.
    LeftAlign,

    /// All lines will be non-justified, aligned to the right.
    RightAlign,

    /// All lines will be non-justified and centered.
    Center,

    /// All lines except the last will be justified.
    Justify,

    /// All lines, including the last, will be justified.
    ForceJustify,
}

/// The paragraph-relative alignment of a single [PdfLine].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfLineAlignment {
    /// No alignment detected.
    None,
    /// Left-aligned.
    LeftAlign,
    /// Right-aligned.
    RightAlign,
    /// Centered.
    Center,
    /// Justified.
    Justify,
}

/// A span of paragraph fragments that make up one line in a [PdfParagraph].
pub struct PdfLine<'a> {
    /// The alignment of this line within the paragraph.
    pub alignment: PdfLineAlignment,
    /// The bottom Y position of this line in PDF points.
    pub bottom: PdfPoints,
    /// The left X position of this line in PDF points.
    pub left: PdfPoints,
    /// The width of this line in PDF points.
    pub width: PdfPoints,
    /// The fragments composing this line.
    pub fragments: Vec<PdfParagraphFragment<'a>>,
}

impl<'a> PdfLine<'a> {
    #[inline]
    fn new(
        alignment: PdfLineAlignment,
        bottom: PdfPoints,
        left: PdfPoints,
        width: PdfPoints,
        fragments: Vec<PdfParagraphFragment<'a>>,
    ) -> Self {
        PdfLine {
            alignment,
            bottom,
            left,
            width,
            fragments,
        }
    }
}

/// A group of [PdfPageTextObject] objects contained in the same `PdfPageObjects` collection
/// that should be laid out together as a single paragraph.
///
/// Text layout in PDF files is handled entirely by text objects. Each text object contains
/// a single span of text that is styled consistently and can be at most a single line long.
/// Multiple text objects stitched together visually at the time the page is generated are
/// interpreted by the reader as paragraphs, but there is no concept in the PDF file format
/// of a multi-line text block, and there is no native functionality for retrieving a single
/// paragraph from its constituent text objects. This makes it difficult to work with long spans
/// of text.
///
/// The [PdfParagraph] is an attempt to improve multi-line text handling. Paragraphs can
/// be created from existing groups of page objects, or created by scratch; once created, text in
/// a paragraph can be edited and re-formatted, and then used to generate a group of text objects
/// that can be placed on a page.
pub struct PdfParagraph<'a> {
    fragments: Vec<PdfParagraphFragment<'a>>,
    bottom: Option<PdfPoints>,
    left: Option<PdfPoints>,
    max_width: Option<PdfPoints>,
    alignment: PdfParagraphAlignment,
}

impl<'a> PdfParagraph<'a> {
    // TODO: lifetime issues, using iterator is a possibility but PdfPage::objects().iter()
    // and PdfPageGroupObject::iter() return iterators over PdfPageObject<'a> whereas
    // &[PdfPageObject<'a>] returns an iterator over &PdfPageObject<'a>

    // /// Creates a set of one or more [PdfParagraph] objects from the objects on the given [PdfPage].
    // #[inline]
    // pub fn from_page(page: &'a PdfPage<'a>) -> Vec<PdfParagraph<'a>> {
    //     let objects = page.objects().iter().collect::<Vec<_>>();
    //
    //     Self::from_iter(objects.as_slice())
    // }
    //
    // #[inline]
    // pub fn from_group(group: &'a PdfPageGroupObject<'a>) -> Vec<PdfParagraph<'a>> {
    //     let objects = group.iter().collect::<Vec<_>>();
    //
    //     Self::from_iter(objects.as_slice())
    // }

    /// Creates a set of one or more [PdfParagraph] objects from the given slice of page objects.
    pub fn from_objects(objects: &'a [PdfPageObject<'a>]) -> Vec<PdfParagraph<'a>> {
        let mut lines = Vec::new();

        let mut current_line_fragments = Vec::new();

        let mut objects_bottom = None;

        let mut objects_top = None;

        let mut objects_left = None;

        let mut objects_right = None;

        // Extract positions from all given objects, so we can attempt to arrange them
        // in reading order irrespective of their original positions.

        let positioned_objects = objects
            .iter()
            .map(|object| {
                let bounds = object.bounds().ok();

                let object_bottom = bounds.map(|b| b.bottom()).unwrap_or(PdfPoints::ZERO);
                let object_top = bounds.map(|b| b.top()).unwrap_or(PdfPoints::ZERO);
                let object_left = bounds.map(|b| b.left()).unwrap_or(PdfPoints::ZERO);
                let object_right = bounds.map(|b| b.right()).unwrap_or(PdfPoints::ZERO);

                update_min(&mut objects_bottom, object_bottom);
                update_max(&mut objects_top, object_top);
                update_min(&mut objects_left, object_left);
                update_max(&mut objects_right, object_right);

                (object_bottom, object_top, object_left, object_right, object)
            })
            .sorted_by(|a, b| {
                let (a_top, a_left) = (a.1, a.2);
                let (b_top, b_left) = (b.1, b.2);

                // Sort by position: top-to-bottom first (higher y = earlier in reading order),
                // then left-to-right within a line.
                // Use the top coordinate for vertical ordering (PDF y increases upward).
                match b_top.value.total_cmp(&a_top.value) {
                    Ordering::Equal => {
                        // Same vertical position: sort by horizontal (left-to-right)
                        a_left.value.total_cmp(&b_left.value)
                    }
                    // If b_top > a_top, b is higher on page → b comes first → a is Greater
                    // If b_top < a_top, a is higher on page → a comes first → a is Less
                    other => other,
                }
            })
            .collect::<Vec<_>>();

        // Filter out significantly rotated text objects (e.g. vertical sidebar text).
        // Rotated objects produce individual characters that interleave with body text.
        let positioned_objects: Vec<_> = positioned_objects
            .into_iter()
            .filter(|(_, _, _, _, object)| object.as_text_object().is_none() || !is_significantly_rotated(object))
            .collect();

        let paragraph_left = objects_left.unwrap_or(PdfPoints::ZERO);
        let paragraph_right = objects_right.unwrap_or(paragraph_left);

        let mut current_line_bottom = PdfPoints::ZERO;
        let mut current_line_left = PdfPoints::ZERO;
        let mut current_line_right = PdfPoints::ZERO;
        let mut current_line_alignment = PdfLineAlignment::None;

        let mut last_object_bottom = None;
        let mut last_object_height = None;
        let mut last_object_left = None;
        let mut last_object_right = None;

        for (bottom, top, left, right, object) in positioned_objects.iter() {
            let top = *top;

            let bottom = *bottom;

            let left = *left;

            let right = *right;

            if last_object_left.is_none() || left < last_object_left.unwrap() {
                // We're at the start of a new line. Does this line break indicate a new paragraph?

                let next_line_alignment = Self::guess_line_alignment(
                    last_object_left,
                    last_object_right,
                    left,
                    right,
                    paragraph_left,
                    paragraph_right,
                );

                if next_line_alignment != current_line_alignment
                    || last_object_bottom.unwrap_or(PdfPoints::ZERO) - last_object_height.unwrap_or(PdfPoints::ZERO)
                        > top
                {
                    // Yes, this line break probably indicates a new paragraph.

                    lines.push(PdfLine::new(
                        current_line_alignment,
                        current_line_bottom,
                        current_line_left,
                        right - current_line_left,
                        current_line_fragments,
                    ));

                    current_line_fragments = vec![PdfParagraphFragment::LineBreak {
                        alignment: current_line_alignment,
                        bottom,
                        left,
                    }];
                    current_line_left = left;
                    current_line_right = PdfPoints::ZERO;
                    current_line_bottom = bottom;
                    current_line_alignment = next_line_alignment;
                } else {
                    // The line break probably just represents a carriage-return rather than the
                    // deliberate end of a paragraph.
                }
            }

            last_object_left = Some(left);
            last_object_right = Some(right);
            last_object_bottom = Some(bottom);
            last_object_height = Some(top - bottom);

            if let Some(object) = object.as_text_object() {
                // If the styling of this object is the same as the last styled string fragment,
                // then append the text of this object to the last fragment; otherwise, start a
                // new text fragment.

                current_line_right = right;

                if let Some(PdfParagraphFragment::StyledString(last_string)) = current_line_fragments.last_mut() {
                    if last_string.does_match_object_styling(object) {
                        // The styles of the two text objects are the same. They should be
                        // merged them into the same styled string - but should they
                        // be part of the same word, or separate words?

                        let separator = if let Ok(bounds) = object.bounds() {
                            if let Some(last_object_right) = last_object_right {
                                if last_object_right > bounds.left() {
                                    // The last and current objects are touching.
                                    // Assume they're part of the same word, despite being
                                    // in separate objects.

                                    ""
                                } else {
                                    // The last and current objects are separated.

                                    " "
                                }
                            } else {
                                // We're at the start of a line.

                                ""
                            }
                        } else {
                            // Cannot measure the bounds of the current object; by default,
                            // assume it's separated from the last object.

                            " "
                        };

                        last_string.push(object.text(), separator);
                    } else {
                        // The styles of the two text objects are different, so they can't be merged.

                        current_line_fragments.push(PdfParagraphFragment::StyledString(
                            PdfStyledString::from_text_object(object),
                        ));
                    }
                } else {
                    // The last fragment wasn't a string fragment, so we have to start a new fragment.

                    current_line_fragments.push(PdfParagraphFragment::StyledString(PdfStyledString::from_text_object(
                        object,
                    )));
                }
            } else {
                current_line_fragments.push(PdfParagraphFragment::NonTextObject(object.object_handle()));
            }
        }

        lines.push(PdfLine::new(
            current_line_alignment,
            current_line_bottom,
            current_line_left,
            current_line_right - current_line_left,
            current_line_fragments,
        ));

        // Assemble lines into paragraphs.

        let mut paragraphs = Vec::new();

        let mut current_paragraph_fragments = Vec::new();

        let mut current_paragraph_bottom = None;

        let mut current_paragraph_left = None;

        let mut current_paragraph_right = None;

        let mut last_line_alignment = lines
            .first()
            .map(|line| line.alignment)
            .unwrap_or(PdfLineAlignment::None);

        let mut first_line_alignment = last_line_alignment;

        for mut line in lines.drain(..) {
            if line.alignment != last_line_alignment {
                // TODO: this won't work as expected for non-force-justified paragraphs
                // where the last line in the paragraph is left-aligned, not justified

                // Finalize the current paragraph...

                if !current_paragraph_fragments.is_empty() {
                    paragraphs.push(Self::paragraph_from_lines(
                        current_paragraph_fragments,
                        current_paragraph_bottom,
                        current_paragraph_left,
                        current_paragraph_right,
                        first_line_alignment,
                        last_line_alignment,
                    ));

                    // ... and start a new paragraph.

                    current_paragraph_fragments = Vec::new();
                    current_paragraph_bottom = None;
                    current_paragraph_left = None;
                    current_paragraph_right = None;
                    first_line_alignment = last_line_alignment
                }
            }

            current_paragraph_fragments.append(&mut line.fragments);

            last_line_alignment = line.alignment;

            update_min(&mut current_paragraph_left, line.left);
            update_max(&mut current_paragraph_right, line.left + line.width);
            update_min(&mut current_paragraph_bottom, line.bottom);
        }

        // Finalize the last paragraph.

        paragraphs.push(Self::paragraph_from_lines(
            current_paragraph_fragments,
            current_paragraph_bottom,
            current_paragraph_left,
            current_paragraph_right,
            first_line_alignment,
            last_line_alignment,
        ));

        paragraphs
    }

    fn paragraph_from_lines(
        fragments: Vec<PdfParagraphFragment<'a>>,
        bottom: Option<PdfPoints>,
        left: Option<PdfPoints>,
        right: Option<PdfPoints>,
        first_line_alignment: PdfLineAlignment,
        last_line_alignment: PdfLineAlignment,
    ) -> PdfParagraph<'a> {
        PdfParagraph {
            fragments,
            bottom,
            left,
            max_width: match (left, right) {
                (Some(left), Some(right)) => Some(right - left),
                _ => None,
            },
            alignment: if first_line_alignment == last_line_alignment
                && first_line_alignment == PdfLineAlignment::Justify
            {
                // Every line in the paragraph, including the last line, is justified.

                PdfParagraphAlignment::ForceJustify
            } else {
                match first_line_alignment {
                    PdfLineAlignment::None | PdfLineAlignment::LeftAlign => PdfParagraphAlignment::LeftAlign,
                    PdfLineAlignment::RightAlign => PdfParagraphAlignment::RightAlign,
                    PdfLineAlignment::Center => PdfParagraphAlignment::Center,
                    PdfLineAlignment::Justify => PdfParagraphAlignment::Justify,
                }
            },
        }
    }

    fn guess_line_alignment(
        previous_line_left: Option<PdfPoints>,
        previous_line_right: Option<PdfPoints>,
        line_left: PdfPoints,
        line_right: PdfPoints,
        paragraph_left: PdfPoints,
        paragraph_right: PdfPoints,
    ) -> PdfLineAlignment {
        const ALIGNMENT_THRESHOLD: f32 = 2.0;

        // Is this line in alignment with the previous line?

        if let (Some(previous_line_left), Some(previous_line_right)) = (previous_line_left, previous_line_right) {
            let is_aligned_left = (previous_line_left.value - line_left.value).abs() < ALIGNMENT_THRESHOLD;

            let is_aligned_right = (previous_line_right.value - line_right.value).abs() < ALIGNMENT_THRESHOLD;

            match (is_aligned_left, is_aligned_right) {
                (true, true) => PdfLineAlignment::Justify,
                (true, false) => PdfLineAlignment::LeftAlign,
                (false, true) => PdfLineAlignment::RightAlign,
                (false, false) => PdfLineAlignment::Center,
            }
        } else {
            let is_aligned_left = (paragraph_left.value - line_left.value).abs() < ALIGNMENT_THRESHOLD;

            let is_aligned_right = (paragraph_right.value - line_right.value).abs() < ALIGNMENT_THRESHOLD;

            match (is_aligned_left, is_aligned_right) {
                (true, true) => PdfLineAlignment::Justify,
                (true, false) => PdfLineAlignment::LeftAlign,
                (false, true) => PdfLineAlignment::RightAlign,
                (false, false) => PdfLineAlignment::Center,
            }
        }
    }

    /// Creates a new, empty [PdfParagraph] with the given maximum line width
    /// and alignment settings.
    #[inline]
    pub fn empty(maximum_width: PdfPoints, alignment: PdfParagraphAlignment) -> Self {
        PdfParagraph {
            fragments: vec![],
            bottom: None,
            left: None,
            max_width: Some(maximum_width),
            alignment,
        }
    }

    /// Returns `true` if this [PdfParagraph] contains no fragments.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Returns a reference to the fragments in this paragraph.
    #[inline]
    pub fn fragments(&self) -> &[PdfParagraphFragment<'a>] {
        &self.fragments
    }

    /// Returns the bottom Y position of this paragraph, if known.
    #[inline]
    pub fn bottom(&self) -> Option<PdfPoints> {
        self.bottom
    }

    /// Returns the left X position of this paragraph, if known.
    #[inline]
    pub fn left(&self) -> Option<PdfPoints> {
        self.left
    }

    /// Returns the alignment of this paragraph.
    #[inline]
    pub fn alignment(&self) -> PdfParagraphAlignment {
        self.alignment
    }

    /// Adds a new fragment containing the given styled string to this paragraph.
    #[inline]
    pub fn push(&mut self, string: PdfStyledString<'a>) {
        // If the styling of this object is the same as the last styled string fragment,
        // then append the text of this object to the last fragment; otherwise, start a
        // new text fragment.

        if let Some(PdfParagraphFragment::StyledString(last_string)) = self.fragments.last_mut() {
            if last_string.does_match_string_styling(&string) {
                // The styles of the two styled strings are the same. Merge them into the same
                // styled string.

                last_string.push(string.text(), " ");
            } else {
                // The styles of the two styled strings are different, so they can't be merged.

                self.fragments.push(PdfParagraphFragment::StyledString(string));
            }
        } else {
            // The last fragment wasn't a string fragment.

            self.fragments.push(PdfParagraphFragment::StyledString(string));
        }
    }

    /// Returns the maximum line width of this paragraph.
    #[inline]
    pub fn maximum_width(&self) -> PdfPoints {
        self.max_width.unwrap_or(PdfPoints::ZERO)
    }

    /// Sets the maximum line width of this paragraph to the given value.
    #[inline]
    pub fn set_maximum_width(&mut self, width: PdfPoints) {
        self.max_width = Some(width);
    }

    /// Returns the text contained within all text fragments in this paragraph.
    #[inline]
    pub fn text(&self) -> String {
        self.fragments
            .iter()
            .filter_map(|fragment| match fragment {
                PdfParagraphFragment::StyledString(string) => Some(string.text.as_str()),
                PdfParagraphFragment::LineBreak { .. } => Some("\n"),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Returns the text contained within all text fragments in this paragraph,
    /// separating each text fragment with the given separator.
    pub fn text_separated(&self, separator: &str) -> String {
        self.fragments
            .iter()
            .filter_map(|fragment| match fragment {
                PdfParagraphFragment::StyledString(string) => Some(string.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Assembles the fragments in this paragraph into lines, taking into account the paragraph's
    /// current sizing, overflow, indent, and alignment settings. Consumes the paragraph and
    /// returns the assembled lines.
    pub fn into_lines(self) -> Vec<PdfLine<'a>> {
        let mut lines: Vec<PdfLine<'a>> = Vec::new();
        let mut current_fragments: Vec<PdfParagraphFragment<'a>> = Vec::new();
        let mut current_width = PdfPoints::ZERO;
        let mut current_bottom = self.bottom.unwrap_or(PdfPoints::ZERO);
        let mut current_left = self.left.unwrap_or(PdfPoints::ZERO);

        let effective_max_width = self.max_width.unwrap_or(PdfPoints::new(f32::MAX));

        for fragment in self.fragments {
            match fragment {
                PdfParagraphFragment::LineBreak {
                    alignment,
                    bottom: line_bottom,
                    left: line_left,
                } => {
                    if !current_fragments.is_empty() {
                        lines.push(PdfLine::new(
                            alignment,
                            current_bottom,
                            current_left,
                            current_width,
                            std::mem::take(&mut current_fragments),
                        ));
                        current_width = PdfPoints::ZERO;
                        current_bottom = line_bottom;
                        current_left = line_left;
                    }
                }
                PdfParagraphFragment::StyledString(ref styled) => {
                    // Estimate width from text length and font size
                    let estimated_width = PdfPoints::new(styled.text().len() as f32 * styled.font_size().value * 0.5);

                    if current_width.value + estimated_width.value > effective_max_width.value
                        && !current_fragments.is_empty()
                    {
                        // Line break needed
                        lines.push(PdfLine::new(
                            PdfLineAlignment::None,
                            current_bottom,
                            current_left,
                            current_width,
                            std::mem::take(&mut current_fragments),
                        ));
                        current_width = PdfPoints::ZERO;
                    }

                    current_width = PdfPoints::new(current_width.value + estimated_width.value);
                    current_fragments.push(fragment);
                }
                PdfParagraphFragment::NonTextObject(_) => {
                    current_fragments.push(fragment);
                }
            }
        }

        // Flush remaining fragments
        if !current_fragments.is_empty() {
            lines.push(PdfLine::new(
                PdfLineAlignment::None,
                current_bottom,
                current_left,
                current_width,
                current_fragments,
            ));
        }

        lines
    }
}

/// Returns true if a page object is rotated more than 10 degrees from horizontal.
///
/// Used to filter out vertical sidebar text (e.g. arXiv identifiers) that would
/// otherwise produce individual characters interleaved with body text.
fn is_significantly_rotated(object: &PdfPageObject) -> bool {
    const ROTATION_THRESHOLD_DEGREES: f32 = 10.0;
    let rotation = object.get_rotation_counter_clockwise_degrees().abs();
    let normalized = if rotation > 180.0 { 360.0 - rotation } else { rotation };
    normalized > ROTATION_THRESHOLD_DEGREES
}

#[cfg(test)]
mod tests {
    use crate::pdf::document::page::paragraph::PdfParagraph;
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};

    #[test]
    fn test_paragraph_construction() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file(&test_fixture_path("text-test.pdf"), None)?;

        let page = document.pages().get(0)?;

        let objects = page.objects().iter().collect::<Vec<_>>();

        let paragraphs = PdfParagraph::from_objects(objects.as_slice());

        // Verify that paragraphs were constructed from the page objects
        assert!(
            !paragraphs.is_empty(),
            "Expected at least one paragraph from page objects"
        );

        // Verify text extraction works for each paragraph
        for paragraph in paragraphs.iter() {
            let text = paragraph.text();
            // Each paragraph should produce some text (the test fixture has text content)
            assert!(
                !text.trim().is_empty() || paragraph.is_empty(),
                "Non-empty paragraph should produce non-empty text"
            );
        }

        // Verify text_separated also works
        for paragraph in paragraphs.iter() {
            let separated = paragraph.text_separated(" ");
            let plain = paragraph.text();
            // Both text methods should return content for non-empty paragraphs
            if !paragraph.is_empty() {
                assert!(
                    !separated.is_empty() || !plain.is_empty(),
                    "Text extraction should return content for non-empty paragraphs"
                );
            }
        }

        Ok(())
    }
}
