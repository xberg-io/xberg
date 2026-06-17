//! Defines the [PdfLink] struct, exposing functionality related to a single link contained
//! within a [PdfPage] or a [PdfPageAnnotation].

use crate::bindgen::{FPDF_DOCUMENT, FPDF_LINK, FS_RECTF};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::action::PdfAction;
use crate::pdf::destination::PdfDestination;
use crate::pdf::rect::PdfRect;

#[cfg(doc)]
use {
    crate::pdf::action::PdfActionType, crate::pdf::document::page::PdfPage,
    crate::pdf::document::page::annotation::PdfPageAnnotation,
};

/// A single link contained within a [PdfPage] or a [PdfPageAnnotation].
///
/// Each link may have a corresponding [PdfAction] that will be triggered when the user
/// interacts with the link, and a [PdfDestination] that indicates the target of any behaviour
/// triggered by the [PdfAction].
pub struct PdfLink<'a> {
    handle: FPDF_LINK,
    document: FPDF_DOCUMENT,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfLink<'a> {
    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_LINK,
        document: FPDF_DOCUMENT,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfLink {
            handle,
            document,
            bindings,
        }
    }

    /// Returns the internal `FPDF_LINK` handle for this [PdfLink].
    #[inline]
    pub(crate) fn handle(&self) -> FPDF_LINK {
        self.handle
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfLink].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.bindings
    }

    /// Returns the [PdfAction] associated with this [PdfLink], if any.
    ///
    /// The action indicates the behaviour that will occur when the user interacts with the
    /// link in a PDF viewer. For most links, this will be a local navigation action
    /// of type [PdfActionType::GoToDestinationInSameDocument], but the PDF file format supports
    /// a variety of other actions.
    pub fn action(&self) -> Option<PdfAction<'a>> {
        let handle = self.bindings().FPDFLink_GetAction(self.handle());

        if handle.is_null() {
            None
        } else {
            Some(PdfAction::from_pdfium(handle, self.document, self.bindings()))
        }
    }

    /// Returns the [PdfDestination] associated with this [PdfLink], if any.
    ///
    /// The destination specifies the page and region, if any, that will be the target
    /// of any behaviour that will occur when the user interacts with the link in a PDF viewer.
    pub fn destination(&self) -> Option<PdfDestination<'a>> {
        let handle = self.bindings().FPDFLink_GetDest(self.document, self.handle());

        if handle.is_null() {
            None
        } else {
            Some(PdfDestination::from_pdfium(self.document, handle, self.bindings()))
        }
    }

    /// Returns the area on the page that the user can use to interact with this [PdfLink]
    /// in a PDF viewer, if any.
    pub fn rect(&self) -> Result<PdfRect, PdfiumError> {
        let mut rect = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };

        PdfRect::from_pdfium_as_result(
            self.bindings().FPDFLink_GetAnnotRect(self.handle(), &mut rect),
            rect,
            self.bindings(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};

    #[test]
    fn test_link_rect() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        // The document under test contains a single page with a single link.

        let document = pdfium.load_pdf_from_file(&test_fixture_path("links-test.pdf"), None)?;

        const EXPECTED: PdfRect = PdfRect::new_from_values(733.3627, 207.85417, 757.6127, 333.1458);

        // Allow a little bit of error, because it's unreasonable to expect floating point
        // calculations to be identical across builds and platforms.

        const ABS_ERR: PdfPoints = PdfPoints::new(f32::EPSILON * 1000.);

        let actual = document
            .pages()
            .iter()
            .next()
            .unwrap()
            .links()
            .iter()
            .next()
            .unwrap()
            .rect()?;

        assert!((actual.top() - EXPECTED.top()).abs() < ABS_ERR);
        assert!((actual.bottom() - EXPECTED.bottom()).abs() < ABS_ERR);
        assert!((actual.left() - EXPECTED.left()).abs() < ABS_ERR);
        assert!((actual.right() - EXPECTED.right()).abs() < ABS_ERR);

        Ok(())
    }
}
