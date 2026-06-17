use crate::bindgen::{FPDF_ANNOTATION, FPDF_DOCUMENT, FPDF_PAGE};

/// The parent ownership hierarchy for a page object bound to a specific [PdfDocument].
#[derive(Copy, Clone)]
pub(crate) struct PdfPageObjectOwnedByDocument {
    document_handle: FPDF_DOCUMENT,
}

impl PdfPageObjectOwnedByDocument {
    pub fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }
}

/// The parent ownership hierarchy for a page object contained by a [PdfPage].
#[derive(Copy, Clone)]
pub(crate) struct PdfPageObjectOwnedByPage {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
}

impl PdfPageObjectOwnedByPage {
    pub fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    pub fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }
}

/// The parent ownership hierarchy for a page object contained by a [PdfPageAnnotation]
/// that is itself attached to a [PdfPage].
#[derive(Copy, Clone)]
pub(crate) struct PdfPageObjectOwnedByAttachedAnnotation {
    document_handle: FPDF_DOCUMENT,
    page_handle: FPDF_PAGE,
    annotation_handle: FPDF_ANNOTATION,
}

impl PdfPageObjectOwnedByAttachedAnnotation {
    pub fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    pub fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }

    pub fn annotation_handle(&self) -> FPDF_ANNOTATION {
        self.annotation_handle
    }
}

/// The parent ownership hierarchy for a page object contained by a [PdfPageAnnotation]
/// where the [PdfPageAnnotation] is not currently attached to any [PdfPage].
#[derive(Copy, Clone)]
pub(crate) struct PdfPageObjectOwnedByUnattachedAnnotation {
    document_handle: FPDF_DOCUMENT,
    annotation_handle: FPDF_ANNOTATION,
}

impl PdfPageObjectOwnedByUnattachedAnnotation {
    pub fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    pub fn annotation_handle(&self) -> FPDF_ANNOTATION {
        self.annotation_handle
    }
}

#[derive(Copy, Clone)]
pub(crate) enum PdfPageObjectOwnership {
    /// The object is not currently owned by an object container.
    Unowned,

    /// The object is not currently owned by an object container, but is bound to a specific [PdfDocument].
    /// It can only be attached to a [PdfPage] or [PdfPageAnnotation] within that document.
    Document(PdfPageObjectOwnedByDocument),

    /// The object is currently owned by an object container attached to a [PdfPage].
    Page(PdfPageObjectOwnedByPage),

    /// The object is currently owned by an object container attached to a [PdfPageAnnotation]
    /// that is itself attached to a [PdfPage].
    AttachedAnnotation(PdfPageObjectOwnedByAttachedAnnotation),

    /// The object is currently owned by an object container attached to a [PdfPageAnnotation]
    /// that is not currently attached to any [PdfPage].
    UnattachedAnnotation(PdfPageObjectOwnedByUnattachedAnnotation),
}

impl PdfPageObjectOwnership {
    pub fn unowned() -> Self {
        Self::Unowned
    }

    pub fn owned_by_document(document_handle: FPDF_DOCUMENT) -> Self {
        Self::Document(PdfPageObjectOwnedByDocument { document_handle })
    }

    pub fn owned_by_page(document_handle: FPDF_DOCUMENT, page_handle: FPDF_PAGE) -> Self {
        Self::Page(PdfPageObjectOwnedByPage {
            document_handle,
            page_handle,
        })
    }

    pub fn owned_by_attached_annotation(
        document_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        annotation_handle: FPDF_ANNOTATION,
    ) -> Self {
        Self::AttachedAnnotation(PdfPageObjectOwnedByAttachedAnnotation {
            document_handle,
            page_handle,
            annotation_handle,
        })
    }

    pub fn owned_by_unattached_annotation(document_handle: FPDF_DOCUMENT, annotation_handle: FPDF_ANNOTATION) -> Self {
        Self::UnattachedAnnotation(PdfPageObjectOwnedByUnattachedAnnotation {
            document_handle,
            annotation_handle,
        })
    }

    /// Returns `true` if the memory allocated to the [PdfPageObject] holding
    /// this [PdfObjectOwnership] instance is owned by an object container attached to
    /// either a [PdfPage] or a [PdfAnnotation].
    pub fn is_owned(&self) -> bool {
        !matches!(self, PdfPageObjectOwnership::Unowned)
    }
}
