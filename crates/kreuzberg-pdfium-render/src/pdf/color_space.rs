//! Defines the [PdfColorSpace] enum, defining all the color spaces supported by the PDF file format.

use crate::bindgen::{
    FPDF_COLORSPACE_CALGRAY, FPDF_COLORSPACE_CALRGB, FPDF_COLORSPACE_DEVICECMYK, FPDF_COLORSPACE_DEVICEGRAY,
    FPDF_COLORSPACE_DEVICEN, FPDF_COLORSPACE_DEVICERGB, FPDF_COLORSPACE_ICCBASED, FPDF_COLORSPACE_INDEXED,
    FPDF_COLORSPACE_LAB, FPDF_COLORSPACE_PATTERN, FPDF_COLORSPACE_SEPARATION, FPDF_COLORSPACE_UNKNOWN,
};
use crate::error::PdfiumError;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// The color space of any displayable object on a [PdfPage].
///
/// Colors can be described in any of a variety of color systems called _color spaces_.
/// Some color spaces are related to device color representation (e.g. grayscale, RGB, CMYK);
/// others are related to human visual perception.
///
/// Color spaces can be classified into _color space families_. Spaces within a family
/// share the same general characteristics. Families fall into three broad categories:
///
/// * **Device color spaces** directly specify colors or shades of gray that the output
///   device is to produce. The precise displayed color is device-specific and is not calibrated.
///   Color space families in this category include [PdfColorSpace::DeviceGray],
///   [PdfColorSpace::DeviceRGB], and [PdfColorSpace::DeviceCMYK].
/// * **Calibrated color spaces** are based on international standards for specifying human-visible
///   colors created by the Commission Internationale de l'Éclairage (International Commission on
///   Illumination) and the International Color Consortium. The precise displayed color is
///   device-independent; it does not rely on the characteristics of any particular output device.
///   Color space families in this category include [PdfColorSpace::CalibratedCIEGray],
///   [PdfColorSpace::CalibratedCIERGB], [PdfColorSpace::CalibratedCIELab], and
///   [PdfColorSpace::CalibratedICCProfile].
/// * **Special color spaces** add features or properties to another color space, such as
///   patterns, color mapping, separations, and high-fidelity and/or multi-tone color.
///   Color space families in this category include [PdfColorSpace::Pattern],
///   [PdfColorSpace::Indexed], [PdfColorSpace::Separation], and [PdfColorSpace::DeviceN].
///
/// Non-RGB color spaces typically define a transform that enables color values in the color
/// space to be converted to an RGB approximation for on-screen display.
///
/// For more information on color spaces and their utilization in PDF files, see Section 4.5
/// of the PDF Reference Manual version 1.7, starting on page 235.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfColorSpace {
    /// An unknown or unset color space. Color spaces were added to the PDF file format
    /// gradually from versions 1.1 to versions 1.3.
    Unknown = FPDF_COLORSPACE_UNKNOWN as isize,

    /// Black, white, and intermediate shades of gray are special cases of full color.
    /// A grayscale value is represented by a single number in the range `0.0..=1.0`, where
    /// 0.0 corresponds to black, 1.0 to white, and intermediate values to different gray levels.
    ///
    /// This is a non-calibrated color space; the exact color produced for a particular set of
    /// component values may vary slightly from device to device.
    DeviceGray = FPDF_COLORSPACE_DEVICEGRAY as isize,

    /// Colors in this color space are specified according to the additive Red-Green-Blue color
    /// model used by light-emitting displays and projectors. Color values are defined by three
    /// components representing the intensities of the additive primary colorants red, green,
    /// and blue. Each component is specified by a number in the range `0.0..=1.0`, where
    /// 0.0 corresponds to a complete absence of the component and 1.0 corresponds to maximum
    /// intensity of the component. If all three components have equal intensity, the perceived
    /// result theoretically is a pure gray on the scale from black to white. If the intensities
    /// are not all equal, the result is some color other than a pure gray.
    ///
    /// This is a non-calibrated color space; the exact color produced for a particular set of
    /// component values may vary slightly from device to device.
    DeviceRGB = FPDF_COLORSPACE_DEVICERGB as isize,

    /// Colors in this color space are specified according to the subtractive Cyan-Magenta-Yellow-Black
    /// model typical of printers and other paper-based output devices. In theory, each of the three
    /// standard process colorants used in printing (cyan, magenta, and yellow) absorbs one of the
    /// additive primary colors (red, green, and blue, respectively). Black, a fourth standard process
    /// colorant, absorbs all of the additive primaries in equal amounts. Color values are defined
    /// by four components representing the concentrations of these process colorants. Each component
    /// is specified by a number in the range `0.0..=1.0`, where 0.0 denotes the complete absence of
    /// a process colorant (that is, absorbs none of the corresponding additive primary) and 1.0
    /// denotes maximum concentration (absorbs as much as possible of the additive primary). Note
    /// that the sense of these numbers is opposite to that of RGB color components.
    ///
    /// This is a non-calibrated color space; the exact color produced for a particular set of
    /// component values may vary slightly from device to device.
    DeviceCMYK = FPDF_COLORSPACE_DEVICECMYK as isize,

    /// Colors in this color space are specified by a single component, arbitrarily named A,
    /// that represents the gray component of a calibrated gray color space.
    ///
    /// This is a calibrated color space, based on the tristimulus components of the CIE 1931 XYZ
    /// color space. The three components of the color space are defined in terms of human
    /// color vision and are independent of any particular output device.
    CalibratedCIEGray = FPDF_COLORSPACE_CALGRAY as isize,

    /// Colors in this color space are specified by three components, arbitrarily named A, B, and C,
    /// representing calibrated red, green, and blue color values. These three color components must
    /// be in the range `0.0..=1.0`.
    ///
    /// This is a calibrated color space, based on the tristimulus components of the CIE 1931 XYZ
    /// color space. The three components of the color space are defined in terms of human
    /// color vision and are independent of any particular output device.
    CalibratedCIERGB = FPDF_COLORSPACE_CALRGB as isize,

    /// Colors in this color space are specified by three components, named L*, a*, and b*,
    /// of a CIE 1976 L\*a\*b color space. The range of the first (L*) component is always 0 to 100;
    /// the ranges of the second (a*) and third (b*) components are defined by the color space.
    ///
    /// This is a calibrated color space; the three components of the color space are defined in
    /// terms of human color vision and are independent of any particular output device.
    CalibratedCIELab = FPDF_COLORSPACE_LAB as isize,

    /// Colors in this color space are based on a cross-platform color profile defined by
    /// the International Color Consortium (ICC).
    ///
    /// This is a calibrated color space; colors are defined by international standards
    /// and are independent of any particular output device.
    CalibratedICCProfile = FPDF_COLORSPACE_ICCBASED as isize,

    /// Some output devices, such as image-setters, produce a separate, monochromatic rendition of
    /// a page - a _separation_ - for each colorant. When the separations are later combined - on a
    /// printing press, for example - with proper inks or other colorants added to them, the result
    /// is a full-color page.
    ///
    /// This special color space provides a means for specifying the use of additional colorants,
    /// called a _tint_, or for isolating the control of individual color components of a device
    /// color space for a subtractive device.
    Separation = FPDF_COLORSPACE_SEPARATION as isize,

    /// Colors in this special color space can contain an arbitrary number of color components.
    /// This provides greater flexibility than is possible with standard device color spaces
    /// or with individual separation color spaces. DeviceN color spaces are used in applications
    /// such as high-fidelity color (such as the Pantone Hexachrome system), multi-tone
    /// color systems (such as duotone), and spot color systems (using subtractive colorants outside
    /// the standard Cyan-Magenta-Yellow-Black model).
    DeviceN = FPDF_COLORSPACE_DEVICEN as isize,

    /// This special color space indicates an object or area should be painted according to
    /// color values stored in a lookup table rather than a color space.
    Indexed = FPDF_COLORSPACE_INDEXED as isize,

    /// This special color space indicates an object or area should be painted using a pattern,
    /// rather than a single color.
    Pattern = FPDF_COLORSPACE_PATTERN as isize,
}

impl PdfColorSpace {
    pub(crate) fn from_pdfium(value: u32) -> Result<PdfColorSpace, PdfiumError> {
        match value {
            FPDF_COLORSPACE_CALGRAY => Ok(PdfColorSpace::CalibratedCIEGray),
            FPDF_COLORSPACE_CALRGB => Ok(PdfColorSpace::CalibratedCIERGB),
            FPDF_COLORSPACE_DEVICECMYK => Ok(PdfColorSpace::DeviceCMYK),
            FPDF_COLORSPACE_DEVICEGRAY => Ok(PdfColorSpace::DeviceGray),
            FPDF_COLORSPACE_DEVICEN => Ok(PdfColorSpace::DeviceN),
            FPDF_COLORSPACE_DEVICERGB => Ok(PdfColorSpace::DeviceRGB),
            FPDF_COLORSPACE_ICCBASED => Ok(PdfColorSpace::CalibratedICCProfile),
            FPDF_COLORSPACE_INDEXED => Ok(PdfColorSpace::Indexed),
            FPDF_COLORSPACE_LAB => Ok(PdfColorSpace::CalibratedCIELab),
            FPDF_COLORSPACE_PATTERN => Ok(PdfColorSpace::Pattern),
            FPDF_COLORSPACE_SEPARATION => Ok(PdfColorSpace::Separation),
            FPDF_COLORSPACE_UNKNOWN => Ok(PdfColorSpace::Unknown),
            _ => Err(PdfiumError::UnknownPdfColorSpace),
        }
    }
}
