//! Defines the [PdfDestination] struct, exposing functionality related to the target destination
//! of a link contained within a single [PdfPage].

use crate::bindgen::{
    FPDF_DEST, FPDF_DOCUMENT, FS_FLOAT, PDFDEST_VIEW_FIT, PDFDEST_VIEW_FITB, PDFDEST_VIEW_FITBH, PDFDEST_VIEW_FITBV,
    PDFDEST_VIEW_FITH, PDFDEST_VIEW_FITR, PDFDEST_VIEW_FITV, PDFDEST_VIEW_UNKNOWN_MODE, PDFDEST_VIEW_XYZ,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::document::pages::PdfPageIndex;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use crate::utils::mem::create_sized_buffer;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// The view settings that a PDF viewer should apply when displaying the target
/// [PdfPage] nominated by a [PdfDestination] in its display window.
#[derive(Debug, Copy, Clone)]
pub enum PdfDestinationViewSettings {
    /// The view settings are unknown.
    Unknown,

    /// Display the target `PdfPage` with the given (x, y) coordinates positioned at the
    /// upper-left corner of the window and the contents of the page magnified by the given
    /// zoom factor. A missing value for any of the parameters indicates that the current value
    /// of that parameter is to be retained unchanged.
    SpecificCoordinatesAndZoom(Option<PdfPoints>, Option<PdfPoints>, Option<f32>),

    /// Display the target `PdfPage` with its contents magnified just enough
    /// to fit the entire page within the window both horizontally and vertically. If
    /// the required horizontal and vertical magnification factors are different, use
    /// the smaller of the two, centering the page within the window in the other
    /// dimension.
    FitPageToWindow,

    /// Display the target `PdfPage` with the given y coordinate positioned at the top edge of
    /// the window and the contents of the page magnified just enough to fit the entire width
    /// of the page within the window. A missing value for the y coordinate indicates
    /// that the current value of that parameter is to be retained unchanged.
    FitPageHorizontallyToWindow(Option<PdfPoints>),

    /// Display the target `PdfPage` with the given x coordinate positioned at the left edge of
    /// the window and the contents of the page magnified just enough to fit the entire height
    /// of the page within the window. A missing value for the x coordinate indicates
    /// that the current value of that parameter is to be retained unchanged.
    FitPageVerticallyToWindow(Option<PdfPoints>),

    /// Display the target `PdfPage` with its contents magnified just enough to fit the
    /// given rectangle entirely within the window both horizontally and vertically.
    /// If the required horizontal and vertical magnification factors are different, then use
    /// the smaller of the two, centering the rectangle within the window in the other dimension.
    FitPageToRectangle(PdfRect),

    /// Display the target `PdfPage` with its contents magnified just enough to fit its bounding box
    /// entirely within the window both horizontally and vertically. If the required horizontal and
    /// vertical magnification factors are different, then use the smaller of the two,
    /// centering the bounding box within the window in the other dimension.
    ///
    /// This variant was added in PDF version 1.1.
    FitBoundsToWindow,

    /// Display the target `PdfPage` with the given y coordinate positioned at the top edge of
    /// the window and the contents of the page magnified just enough to fit the entire width
    /// of its bounding box within the window. A missing value for the y coordinate indicates
    /// that the current value of that parameter is to be retained unchanged.
    ///
    /// This variant was added in PDF version 1.1.
    FitBoundsHorizontallyToWindow(Option<PdfPoints>),

    /// Display the target `PdfPage` with the given x coordinate positioned at the left edge of
    /// the window and the contents of the page magnified just enough to fit the entire height
    /// of its bounding box within the window. A missing value for the x coordinate indicates
    /// that the current value of that parameter is to be retained unchanged.
    ///
    /// This variant was added in PDF version 1.1.
    FitBoundsVerticallyToWindow(Option<PdfPoints>),
}

impl PdfDestinationViewSettings {
    pub(crate) fn from_pdfium(destination: &PdfDestination) -> Result<PdfDestinationViewSettings, PdfiumError> {
        // We use a combination of calls to FPDFDest_GetLocationInPage() and
        // FPDFDest_GetView() to account for all supported view settings
        // in a null-safe manner.

        let mut has_x_value = destination.bindings.FALSE();

        let mut has_y_value = destination.bindings.FALSE();

        let mut has_zoom_value = destination.bindings.FALSE();

        let mut x_value: FS_FLOAT = 0.0;

        let mut y_value: FS_FLOAT = 0.0;

        let mut zoom_value: FS_FLOAT = 0.0;

        let (x, y, zoom) = if destination
            .bindings
            .is_true(destination.bindings.FPDFDest_GetLocationInPage(
                destination.destination_handle,
                &mut has_x_value,
                &mut has_y_value,
                &mut has_zoom_value,
                &mut x_value,
                &mut y_value,
                &mut zoom_value,
            )) {
            let x = if destination.bindings.is_true(has_x_value) {
                Some(PdfPoints::new(x_value))
            } else {
                None
            };

            let y = if destination.bindings.is_true(has_y_value) {
                Some(PdfPoints::new(y_value))
            } else {
                None
            };

            let zoom = if destination.bindings.is_true(has_zoom_value) {
                // The PDF specification states that a zoom value of 0 has the same meaning
                // as a null value.

                if zoom_value != 0.0 { Some(zoom_value) } else { None }
            } else {
                None
            };

            (x, y, zoom)
        } else {
            (None, None, None)
        };

        let mut p_num_params = 0;

        let mut p_params: Vec<FS_FLOAT> = create_sized_buffer(4);

        let view = destination.bindings.FPDFDest_GetView(
            destination.destination_handle,
            &mut p_num_params,
            p_params.as_mut_ptr(),
        );

        let view_mode = {
            #[cfg(target_arch = "wasm32")]
            let v: u32 = view;
            #[cfg(not(target_arch = "wasm32"))]
            let v: u32 = view as u32;
            v
        };
        match view_mode {
            PDFDEST_VIEW_UNKNOWN_MODE => Ok(PdfDestinationViewSettings::Unknown),
            PDFDEST_VIEW_XYZ => {
                if p_num_params == 3 {
                    Ok(PdfDestinationViewSettings::SpecificCoordinatesAndZoom(x, y, zoom))
                } else {
                    Err(PdfiumError::PdfDestinationViewInvalidParameters)
                }
            }
            PDFDEST_VIEW_FIT => {
                if p_num_params == 0 {
                    Ok(PdfDestinationViewSettings::FitPageToWindow)
                } else {
                    Err(PdfiumError::PdfDestinationViewInvalidParameters)
                }
            }
            PDFDEST_VIEW_FITH => match p_num_params {
                0 => Ok(PdfDestinationViewSettings::FitPageHorizontallyToWindow(None)),
                1 => Ok(PdfDestinationViewSettings::FitPageHorizontallyToWindow(Some(
                    PdfPoints::new(p_params[0]),
                ))),
                _ => Err(PdfiumError::PdfDestinationViewInvalidParameters),
            },
            PDFDEST_VIEW_FITV => match p_num_params {
                0 => Ok(PdfDestinationViewSettings::FitPageVerticallyToWindow(None)),
                1 => Ok(PdfDestinationViewSettings::FitPageVerticallyToWindow(Some(
                    PdfPoints::new(p_params[0]),
                ))),
                _ => Err(PdfiumError::PdfDestinationViewInvalidParameters),
            },
            PDFDEST_VIEW_FITR => {
                if p_num_params == 4 {
                    // Rectangle values are defined in p_params[] in the order
                    // left, bottom, right, top.

                    let left = p_params[0];
                    let bottom = p_params[1];
                    let right = p_params[2];
                    let top = p_params[3];

                    Ok(PdfDestinationViewSettings::FitPageToRectangle(
                        PdfRect::new_from_values(bottom, left, top, right),
                    ))
                } else {
                    Err(PdfiumError::PdfDestinationViewInvalidParameters)
                }
            }
            PDFDEST_VIEW_FITB => {
                if p_num_params == 0 {
                    Ok(PdfDestinationViewSettings::FitBoundsToWindow)
                } else {
                    Err(PdfiumError::PdfDestinationViewInvalidParameters)
                }
            }
            PDFDEST_VIEW_FITBH => match p_num_params {
                0 => Ok(PdfDestinationViewSettings::FitBoundsHorizontallyToWindow(None)),
                1 => Ok(PdfDestinationViewSettings::FitBoundsHorizontallyToWindow(Some(
                    PdfPoints::new(p_params[0]),
                ))),
                _ => Err(PdfiumError::PdfDestinationViewInvalidParameters),
            },
            PDFDEST_VIEW_FITBV => match p_num_params {
                0 => Ok(PdfDestinationViewSettings::FitBoundsVerticallyToWindow(None)),
                1 => Ok(PdfDestinationViewSettings::FitBoundsVerticallyToWindow(Some(
                    PdfPoints::new(p_params[0]),
                ))),
                _ => Err(PdfiumError::PdfDestinationViewInvalidParameters),
            },
            _ => Err(PdfiumError::UnknownPdfDestinationViewType),
        }
    }
}

/// The page and region, if any, that will be the target of any behaviour that will occur
/// when the user interacts with a link in a PDF viewer.
pub struct PdfDestination<'a> {
    document_handle: FPDF_DOCUMENT,
    destination_handle: FPDF_DEST,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfDestination<'a> {
    // TODO: AJRC - 18/2/23 - as the PdfDestination struct is fleshed out, the example at
    // examples/links.rs should be expanded to demonstrate the new functionality.

    pub(crate) fn from_pdfium(
        document_handle: FPDF_DOCUMENT,
        destination_handle: FPDF_DEST,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        PdfDestination {
            document_handle,
            destination_handle,
            bindings,
        }
    }

    /// Returns the internal `FPDF_DEST` handle for this [PdfDestination].
    #[inline]
    #[allow(unused)]
    pub(crate) fn destination_handle(&self) -> FPDF_DEST {
        self.destination_handle
    }

    /// Returns the internal `FPDF_DOCUMENT` handle for this [PdfDestination].
    #[inline]
    #[allow(unused)]
    pub(crate) fn document_handle(&self) -> FPDF_DOCUMENT {
        self.document_handle
    }

    /// Returns the zero-based index of the `PdfPage` containing this [PdfDestination].
    #[inline]
    pub fn page_index(&self) -> Result<PdfPageIndex, PdfiumError> {
        match self
            .bindings
            .FPDFDest_GetDestPageIndex(self.document_handle, self.destination_handle)
        {
            -1 => Err(PdfiumError::DestinationPageIndexNotAvailable),
            index => Ok(index as PdfPageIndex),
        }
    }

    /// Returns the view settings that a PDF viewer should apply when displaying the target
    ///`PdfPage` containing this [PdfDestination].
    #[inline]
    pub fn view_settings(&self) -> Result<PdfDestinationViewSettings, PdfiumError> {
        PdfDestinationViewSettings::from_pdfium(self)
    }

    /// Returns the [PdfiumLibraryBindings] used by this [PdfDestination].
    #[inline]
    pub fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.bindings
    }
}
