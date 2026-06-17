pub(crate) mod internal {
    // We want to make the PdfActionPrivate trait private while providing a blanket
    // implementation of PdfActionCommon for any type T where T: PdfActionPrivate.
    // Rust complains, however, that by doing so we are leaking the private trait outside
    // the crate.

    // Instead of making the PdfActionPrivate trait private, we leave it public but place it
    // inside this pub(crate) module in order to prevent it from being visible outside the crate.

    use crate::bindgen::FPDF_ACTION;
    use crate::bindings::PdfiumLibraryBindings;
    use crate::pdf::action::PdfActionCommon;

    /// Internal crate-specific functionality common to all [PdfAction] actions.
    pub(crate) trait PdfActionPrivate<'a>: PdfActionCommon<'a> {
        /// Returns the internal `FPDF_ACTION` handle for this [PdfAction].
        #[allow(dead_code)] // TODO: AJRC - 13/6/24 - remove once handle() function is in use.
        fn handle(&self) -> &FPDF_ACTION;

        /// Returns the [PdfiumLibraryBindings] used by this [PdfAction].
        fn bindings(&self) -> &dyn PdfiumLibraryBindings;
    }
}
