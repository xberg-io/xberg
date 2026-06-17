//! Defines the [PdfiumApiVersion] enum for the supported Pdfium API version.

/// The Pdfium `FPDF_*` API release version used by this build.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfiumApiVersion {
    V7678,
}

impl PdfiumApiVersion {
    /// Returns the Pdfium API version this crate was compiled against.
    pub(crate) fn current() -> Self {
        PdfiumApiVersion::V7678
    }
}
