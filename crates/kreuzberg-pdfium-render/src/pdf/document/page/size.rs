//! Defines the [PdfPagePaperSize] enum, a set of common ANSI and ISO paper sizes.

use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;

#[cfg(doc)]
use crate::pdf::document::page::PdfPage;

/// A standardized paper size.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPagePaperStandardSize {
    /// ANSI Standard Paper A size (US Letter), 216 x 279 mm / 8.5 x 11.0 in
    USLetterAnsiA,

    /// US Half Letter size, 140 x 216 mm / 5.5 x 8.5 in
    USHalfLetter,

    /// US Government Letter size, 203 x 254 mm / 8.0 x 10.0 in
    USGovernmentLetter,

    /// US Legal size, 216 x 356 mm / 8.5 x 14.0 in
    USLegal,

    /// US Junior Legal size, 127 x 203 mm / 5.0 x 8.0 in
    USJuniorLegal,

    /// US Government Legal size, 216 x 330 mm / 8.5 x 13.0 in
    USGovernmentLegal,

    /// ANSI Standard Paper B size (US Ledger / Tabloid), 279 x 432 mm / 11.0 x 17.0 in
    USLedgerTabloidAnsiB,

    /// ISO 216 4A0, quadruple the size of ISO 216 standard A0, 1682 x 2378 mm
    A0x4,

    /// ISO 216 2A0, double the size of ISO 216 standard A0, 1189 x 1682 mm
    A0x2,

    /// ISO 216 A0, 841 x 1189 mm
    A0,

    /// ISO 216 A1, 594 x 841 mm
    A1,

    /// ISO 216 A2, 420 x 594 mm
    A2,

    /// ISO 216 A3, 297 x 420 mm
    A3,

    /// ISO 216 A4, 210 x 297 mm
    A4,

    /// ISO 216 A4R, equivalent to A4 rotated 90 degrees, 297 x 210 mm
    A4R,

    /// ISO 216 A5, 148 x 210 mm
    A5,

    /// ISO 216 A6, 105 x 148 mm
    A6,

    /// ISO 216 A7, 74 x 105 mm
    A7,

    /// ISO 216 A8, 52 x 74 mm
    A8,

    /// ISO 216 A9, 37 x 52 mm
    A9,

    /// ISO 216 A10, 26 x 37 mm
    A10,

    /// ISO 216 B0, 1000 x 1414 mm
    B0,

    /// ISO 216 B1, 707 x 1000 mm
    B1,

    /// ISO 216 B2, 500 x 707 mm
    B2,

    /// ISO 216 B3, 353 x 500 mm
    B3,

    /// ISO 216 B4, 250 x 353 mm
    B4,

    /// ISO 216 B5, 176 x 250 mm
    B5,

    /// ISO 216 B6, 125 x 176 mm
    B6,

    /// ISO 216 B7, 88 x 125 mm
    B7,

    /// ISO 216 B8, 62 x 88 mm
    B8,

    /// ISO 216 B9, 44 x 62 mm
    B9,

    /// ISO 216 B10, 31 x 44 mm
    B10,

    /// ISO 216 C0, 917 x 1297 mm
    C0,

    /// ISO 216 C1, 648 x 917 mm
    C1,

    /// ISO 216 C2, 458 x 648 mm
    C2,

    /// ISO 216 C3, 324 x 458 mm
    C3,

    /// ISO 216 C4, 229 x 324 mm
    C4,

    /// ISO 216 C5, 162 x 229 mm
    C5,

    /// ISO 216 C6, 114 x 162 mm
    C6,

    /// ISO 216 C7, 81 x 114 mm
    C7,

    /// ISO 216 C8, 57 x 81 mm
    C8,

    /// ISO 216 C9, 40 x 57 mm
    C9,

    /// ISO 216 C10, 28 x 40 mm
    C10,

    /// ANSI Standard Paper B+ (Super B) size, equivalent to ANSI B with a 1 inch margin,
    /// 330 x 483 mm / 13.0 x 19.0 in
    AnsiBPlus,

    /// ANSI Standard Paper C size, 432 x 559 mm / 17.0 x 22.0 in
    AnsiC,

    /// ANSI Standard Paper D size, 559 x 864 mm / 22.0 x 34.0 in
    AnsiD,

    /// ANSI Standard Paper E size, 864 x 1118 mm / 34.0 x 44.0 in
    AnsiE,

    /// North American architectural A size, 229 x 305 mm / 9.0 x 12.0 in
    ArchA,

    /// North American architectural B size, 305 x 457 mm / 12.0 x 18.0 in
    ArchB,

    /// North American architectural C size, 457 x 610 mm / 18.0 x 24.0 in
    ArchC,

    /// North American architectural D size, 610 x 914 mm / 24.0 x 36.0 in
    ArchD,

    /// North American architectural E size, 762 x 1067 mm / 30.0 x 42.0 in
    ArchE,
}

impl PdfPagePaperStandardSize {
    /// Returns the [PdfPagePaperStandardSize] variant, if any, that exactly matches the
    /// given dimensions in millimeters.
    pub fn from_mm_dimensions(width: u32, height: u32) -> Option<PdfPagePaperStandardSize> {
        match (width, height) {
            (216, 279) => Some(PdfPagePaperStandardSize::USLetterAnsiA),
            (140, 216) => Some(PdfPagePaperStandardSize::USHalfLetter),
            (203, 254) => Some(PdfPagePaperStandardSize::USGovernmentLetter),
            (216, 356) => Some(PdfPagePaperStandardSize::USLegal),
            (127, 203) => Some(PdfPagePaperStandardSize::USJuniorLegal),
            (216, 330) => Some(PdfPagePaperStandardSize::USGovernmentLegal),
            (279, 432) => Some(PdfPagePaperStandardSize::USLedgerTabloidAnsiB),
            (1682, 2378) => Some(PdfPagePaperStandardSize::A0x4),
            (1189, 1682) => Some(PdfPagePaperStandardSize::A0x2),
            (841, 1189) => Some(PdfPagePaperStandardSize::A0),
            (594, 841) => Some(PdfPagePaperStandardSize::A1),
            (420, 594) => Some(PdfPagePaperStandardSize::A2),
            (297, 420) => Some(PdfPagePaperStandardSize::A3),
            (210, 297) => Some(PdfPagePaperStandardSize::A4),
            (297, 210) => Some(PdfPagePaperStandardSize::A4R),
            (148, 210) => Some(PdfPagePaperStandardSize::A5),
            (105, 148) => Some(PdfPagePaperStandardSize::A6),
            (74, 105) => Some(PdfPagePaperStandardSize::A7),
            (52, 74) => Some(PdfPagePaperStandardSize::A8),
            (37, 52) => Some(PdfPagePaperStandardSize::A9),
            (26, 37) => Some(PdfPagePaperStandardSize::A10),
            (1000, 1414) => Some(PdfPagePaperStandardSize::B0),
            (707, 1000) => Some(PdfPagePaperStandardSize::B1),
            (500, 707) => Some(PdfPagePaperStandardSize::B2),
            (353, 500) => Some(PdfPagePaperStandardSize::B3),
            (250, 353) => Some(PdfPagePaperStandardSize::B4),
            (176, 250) => Some(PdfPagePaperStandardSize::B5),
            (125, 176) => Some(PdfPagePaperStandardSize::B6),
            (88, 125) => Some(PdfPagePaperStandardSize::B7),
            (62, 88) => Some(PdfPagePaperStandardSize::B8),
            (44, 62) => Some(PdfPagePaperStandardSize::B9),
            (31, 44) => Some(PdfPagePaperStandardSize::B10),
            (917, 1297) => Some(PdfPagePaperStandardSize::C0),
            (648, 917) => Some(PdfPagePaperStandardSize::C1),
            (458, 648) => Some(PdfPagePaperStandardSize::C2),
            (324, 458) => Some(PdfPagePaperStandardSize::C3),
            (229, 324) => Some(PdfPagePaperStandardSize::C4),
            (162, 229) => Some(PdfPagePaperStandardSize::C5),
            (114, 162) => Some(PdfPagePaperStandardSize::C6),
            (81, 114) => Some(PdfPagePaperStandardSize::C7),
            (57, 81) => Some(PdfPagePaperStandardSize::C8),
            (40, 57) => Some(PdfPagePaperStandardSize::C9),
            (28, 40) => Some(PdfPagePaperStandardSize::C10),
            (330, 483) => Some(PdfPagePaperStandardSize::AnsiBPlus),
            (432, 559) => Some(PdfPagePaperStandardSize::AnsiC),
            (559, 864) => Some(PdfPagePaperStandardSize::AnsiD),
            (864, 1118) => Some(PdfPagePaperStandardSize::AnsiE),
            (229, 305) => Some(PdfPagePaperStandardSize::ArchA),
            (305, 457) => Some(PdfPagePaperStandardSize::ArchB),
            (457, 610) => Some(PdfPagePaperStandardSize::ArchC),
            (610, 914) => Some(PdfPagePaperStandardSize::ArchD),
            (762, 1067) => Some(PdfPagePaperStandardSize::ArchE),
            _ => None,
        }
    }

    /// Returns the width of this [PdfPagePaperStandardSize] in portrait orientation.
    pub fn width(&self) -> PdfPoints {
        PdfPoints::from_mm(match self {
            PdfPagePaperStandardSize::USLetterAnsiA => 216.0,
            PdfPagePaperStandardSize::USHalfLetter => 140.0,
            PdfPagePaperStandardSize::USGovernmentLetter => 203.0,
            PdfPagePaperStandardSize::USLegal => 216.0,
            PdfPagePaperStandardSize::USJuniorLegal => 127.0,
            PdfPagePaperStandardSize::USGovernmentLegal => 216.0,
            PdfPagePaperStandardSize::USLedgerTabloidAnsiB => 279.0,
            PdfPagePaperStandardSize::A0x4 => 1682.0,
            PdfPagePaperStandardSize::A0x2 => 1189.0,
            PdfPagePaperStandardSize::A0 => 841.0,
            PdfPagePaperStandardSize::A1 => 594.0,
            PdfPagePaperStandardSize::A2 => 420.0,
            PdfPagePaperStandardSize::A3 => 297.0,
            PdfPagePaperStandardSize::A4 => 210.0,
            PdfPagePaperStandardSize::A4R => 297.0,
            PdfPagePaperStandardSize::A5 => 148.0,
            PdfPagePaperStandardSize::A6 => 105.0,
            PdfPagePaperStandardSize::A7 => 74.0,
            PdfPagePaperStandardSize::A8 => 52.0,
            PdfPagePaperStandardSize::A9 => 37.0,
            PdfPagePaperStandardSize::A10 => 26.0,
            PdfPagePaperStandardSize::B0 => 1000.0,
            PdfPagePaperStandardSize::B1 => 707.0,
            PdfPagePaperStandardSize::B2 => 500.0,
            PdfPagePaperStandardSize::B3 => 353.0,
            PdfPagePaperStandardSize::B4 => 250.0,
            PdfPagePaperStandardSize::B5 => 176.0,
            PdfPagePaperStandardSize::B6 => 125.0,
            PdfPagePaperStandardSize::B7 => 88.0,
            PdfPagePaperStandardSize::B8 => 62.0,
            PdfPagePaperStandardSize::B9 => 44.0,
            PdfPagePaperStandardSize::B10 => 31.0,
            PdfPagePaperStandardSize::C0 => 917.0,
            PdfPagePaperStandardSize::C1 => 648.0,
            PdfPagePaperStandardSize::C2 => 458.0,
            PdfPagePaperStandardSize::C3 => 324.0,
            PdfPagePaperStandardSize::C4 => 229.0,
            PdfPagePaperStandardSize::C5 => 162.0,
            PdfPagePaperStandardSize::C6 => 114.0,
            PdfPagePaperStandardSize::C7 => 81.0,
            PdfPagePaperStandardSize::C8 => 57.0,
            PdfPagePaperStandardSize::C9 => 40.0,
            PdfPagePaperStandardSize::C10 => 28.0,
            PdfPagePaperStandardSize::AnsiBPlus => 330.0,
            PdfPagePaperStandardSize::AnsiC => 432.0,
            PdfPagePaperStandardSize::AnsiD => 559.0,
            PdfPagePaperStandardSize::AnsiE => 864.0,
            PdfPagePaperStandardSize::ArchA => 229.0,
            PdfPagePaperStandardSize::ArchB => 305.0,
            PdfPagePaperStandardSize::ArchC => 457.0,
            PdfPagePaperStandardSize::ArchD => 610.0,
            PdfPagePaperStandardSize::ArchE => 762.0,
        })
    }

    /// Returns the height of this [PdfPagePaperStandardSize] in portrait orientation.
    pub fn height(&self) -> PdfPoints {
        PdfPoints::from_mm(match self {
            PdfPagePaperStandardSize::USLetterAnsiA => 279.0,
            PdfPagePaperStandardSize::USHalfLetter => 216.0,
            PdfPagePaperStandardSize::USGovernmentLetter => 254.0,
            PdfPagePaperStandardSize::USLegal => 356.0,
            PdfPagePaperStandardSize::USJuniorLegal => 203.0,
            PdfPagePaperStandardSize::USGovernmentLegal => 330.0,
            PdfPagePaperStandardSize::USLedgerTabloidAnsiB => 432.0,
            PdfPagePaperStandardSize::A0x4 => 2378.0,
            PdfPagePaperStandardSize::A0x2 => 1682.0,
            PdfPagePaperStandardSize::A0 => 1189.0,
            PdfPagePaperStandardSize::A1 => 841.0,
            PdfPagePaperStandardSize::A2 => 594.0,
            PdfPagePaperStandardSize::A3 => 420.0,
            PdfPagePaperStandardSize::A4 => 297.0,
            PdfPagePaperStandardSize::A4R => 210.0,
            PdfPagePaperStandardSize::A5 => 210.0,
            PdfPagePaperStandardSize::A6 => 148.0,
            PdfPagePaperStandardSize::A7 => 105.0,
            PdfPagePaperStandardSize::A8 => 74.0,
            PdfPagePaperStandardSize::A9 => 52.0,
            PdfPagePaperStandardSize::A10 => 37.0,
            PdfPagePaperStandardSize::B0 => 1414.0,
            PdfPagePaperStandardSize::B1 => 1000.0,
            PdfPagePaperStandardSize::B2 => 707.0,
            PdfPagePaperStandardSize::B3 => 500.0,
            PdfPagePaperStandardSize::B4 => 353.0,
            PdfPagePaperStandardSize::B5 => 250.0,
            PdfPagePaperStandardSize::B6 => 176.0,
            PdfPagePaperStandardSize::B7 => 125.0,
            PdfPagePaperStandardSize::B8 => 88.0,
            PdfPagePaperStandardSize::B9 => 62.0,
            PdfPagePaperStandardSize::B10 => 44.0,
            PdfPagePaperStandardSize::C0 => 1297.0,
            PdfPagePaperStandardSize::C1 => 917.0,
            PdfPagePaperStandardSize::C2 => 648.0,
            PdfPagePaperStandardSize::C3 => 458.0,
            PdfPagePaperStandardSize::C4 => 324.0,
            PdfPagePaperStandardSize::C5 => 229.0,
            PdfPagePaperStandardSize::C6 => 162.0,
            PdfPagePaperStandardSize::C7 => 114.0,
            PdfPagePaperStandardSize::C8 => 81.0,
            PdfPagePaperStandardSize::C9 => 57.0,
            PdfPagePaperStandardSize::C10 => 40.0,
            PdfPagePaperStandardSize::AnsiBPlus => 483.0,
            PdfPagePaperStandardSize::AnsiC => 559.0,
            PdfPagePaperStandardSize::AnsiD => 864.0,
            PdfPagePaperStandardSize::AnsiE => 1118.0,
            PdfPagePaperStandardSize::ArchA => 305.0,
            PdfPagePaperStandardSize::ArchB => 457.0,
            PdfPagePaperStandardSize::ArchC => 610.0,
            PdfPagePaperStandardSize::ArchD => 914.0,
            PdfPagePaperStandardSize::ArchE => 1067.0,
        })
    }
}

/// The paper size of a [PdfPage].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfPagePaperSize {
    /// A known paper size in portrait orientation.
    Portrait(PdfPagePaperStandardSize),

    /// A known paper size in landscape orientation.
    Landscape(PdfPagePaperStandardSize),

    /// A custom paper size, expressed as a (width, height) tuple in [PdfPoints].
    Custom(PdfPoints, PdfPoints),
}

impl PdfPagePaperSize {
    /// Returns the [PdfPagePaperSize] matching the given dimensions,
    /// or [PdfPagePaperSize::Custom] if no match can be made.
    #[inline]
    pub fn from_points(width: PdfPoints, height: PdfPoints) -> Self {
        let width_mm = width.to_mm().trunc() as u32;

        let height_mm = height.to_mm().trunc() as u32;

        match PdfPagePaperStandardSize::from_mm_dimensions(width_mm, height_mm) {
            Some(size) => PdfPagePaperSize::Portrait(size),
            None => {
                // Try swapping the width and height. This will detect a rotated paper size.

                match PdfPagePaperStandardSize::from_mm_dimensions(height_mm, width_mm) {
                    Some(size) => PdfPagePaperSize::Landscape(size),
                    None => {
                        // Still no match. Return the original result.

                        PdfPagePaperSize::Custom(width, height)
                    }
                }
            }
        }
    }

    /// Returns the [PdfPagePaperSize] matching the given dimensions,
    /// or [PdfPagePaperSize::Custom] if no match can be made.
    #[inline]
    pub fn from_inches(width: f32, height: f32) -> Self {
        Self::from_points(PdfPoints::from_inches(width), PdfPoints::from_inches(height))
    }

    /// Returns the [PdfPagePaperSize] matching the given dimensions,
    /// or [PdfPagePaperSize::Custom] if no match can be made.
    #[inline]
    pub fn from_cm(width: f32, height: f32) -> Self {
        Self::from_points(PdfPoints::from_cm(width), PdfPoints::from_cm(height))
    }

    /// Returns the [PdfPagePaperSize] matching the given dimensions,
    /// or [PdfPagePaperSize::Custom] if no match can be made.
    #[inline]
    pub fn from_mm(width: f32, height: f32) -> Self {
        Self::from_points(PdfPoints::from_mm(width), PdfPoints::from_mm(height))
    }

    /// Creates a new portrait [PdfPagePaperSize] from a standard [PdfPagePaperStandardSize].
    #[inline]
    pub fn new_portrait(size: PdfPagePaperStandardSize) -> Self {
        PdfPagePaperSize::Portrait(size)
    }

    /// Creates a new landscape [PdfPagePaperSize] from a standard [PdfPagePaperStandardSize].
    #[inline]
    pub fn new_landscape(size: PdfPagePaperStandardSize) -> Self {
        PdfPagePaperSize::Landscape(size)
    }

    /// Creates a new custom [PdfPagePaperSize] from the given dimensions.
    #[inline]
    pub fn new_custom(width: PdfPoints, height: PdfPoints) -> Self {
        PdfPagePaperSize::Custom(width, height)
    }

    /// Creates a new portrait A4 [PdfPagePaperSize].
    #[inline]
    pub fn a4() -> Self {
        Self::new_portrait(PdfPagePaperStandardSize::A4)
    }

    /// Creates a new portrait A4R [PdfPagePaperSize], equivalent to landscape A4. In terms of
    /// paper size, this is equivalent to calling [PdfPagePaperSize::a4().to_landscape()]
    #[inline]
    pub fn a4r() -> Self {
        Self::new_portrait(PdfPagePaperStandardSize::A4R)
    }

    /// Creates a new portrait A3 [PdfPagePaperSize].
    #[inline]
    pub fn a3() -> Self {
        Self::new_portrait(PdfPagePaperStandardSize::A3)
    }

    /// Rotates a landscape [PdfPagePaperSize] into a portrait [PdfPagePaperSize] and vice versa,
    /// consuming this [PdfPagePaperSize].
    ///
    /// Custom sizes have their height and width dimensions swapped.
    pub fn rotate(self) -> Self {
        match self {
            PdfPagePaperSize::Portrait(size) => PdfPagePaperSize::Landscape(size),
            PdfPagePaperSize::Landscape(size) => PdfPagePaperSize::Portrait(size),
            PdfPagePaperSize::Custom(width, height) => PdfPagePaperSize::Custom(height, width),
        }
    }

    /// Rotates a portrait [PdfPagePaperSize] into a landscape [PdfPagePaperSize] if necessary.
    /// A new [PdfPagePaperSize] value is returned; this [PdfPagePaperSize] is not affected.
    /// Sizes already in landscape are not changed. Custom sizes are changed only if the
    /// current height is greater than the width, in which case the dimensions are swapped.
    pub fn landscape(&self) -> Self {
        match self {
            PdfPagePaperSize::Portrait(size) => PdfPagePaperSize::Landscape(*size),
            PdfPagePaperSize::Landscape(_) => *self,
            PdfPagePaperSize::Custom(width, height) => {
                if height > width {
                    PdfPagePaperSize::Custom(*height, *width)
                } else {
                    *self
                }
            }
        }
    }

    /// Rotates a landscape [PdfPagePaperSize] into a portrait [PdfPagePaperSize] if necessary.
    /// A new [PdfPagePaperSize] value is returned; this [PdfPagePaperSize] is not affected.
    /// Sizes already in portrait are not changed. Custom sizes are changed only if the
    /// current width is greater than the height, in which case the dimensions are swapped.
    pub fn portrait(&self) -> Self {
        match self {
            PdfPagePaperSize::Portrait(_) => *self,
            PdfPagePaperSize::Landscape(size) => PdfPagePaperSize::Portrait(*size),
            PdfPagePaperSize::Custom(width, height) => {
                if width > height {
                    PdfPagePaperSize::Custom(*height, *width)
                } else {
                    *self
                }
            }
        }
    }

    /// Returns the width of this [PdfPagePaperSize].
    #[inline]
    pub fn width(&self) -> PdfPoints {
        match self {
            PdfPagePaperSize::Portrait(size) => size.width(),
            PdfPagePaperSize::Landscape(size) => size.height(),
            PdfPagePaperSize::Custom(width, _) => *width,
        }
    }

    /// Returns the height of this [PdfPagePaperSize].
    #[inline]
    pub fn height(&self) -> PdfPoints {
        match self {
            PdfPagePaperSize::Portrait(size) => size.height(),
            PdfPagePaperSize::Landscape(size) => size.width(),
            PdfPagePaperSize::Custom(_, height) => *height,
        }
    }

    /// Returns the dimensions of this [PdfPagePaperSize] as a [PdfRect].
    #[inline]
    pub fn as_rect(&self) -> PdfRect {
        PdfRect::new(PdfPoints::ZERO, PdfPoints::ZERO, self.height(), self.width())
    }
}
