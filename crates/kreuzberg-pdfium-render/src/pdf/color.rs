//! Defines the [PdfColor] struct, a 32-bit RGB color value with an optional alpha channel.

use crate::bindgen::FPDF_DWORD;
use crate::error::PdfiumError;

/// A 32-bit RGB color value with an optional alpha channel.
///
/// A variety of non-transparent colors are available as const values on this struct.
///
/// Note that when used as a form field highlight color, a solid color with no opacity
/// will overprint any user data in the field. Use the [PdfColor::with_alpha()] function
/// to apply an alpha channel value to an existing [PdfColor].
#[derive(Debug, Copy, Clone)]
pub struct PdfColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl PdfColor {
    pub const WHITE: PdfColor = PdfColor::new(255, 255, 255, 255);
    pub const BLACK: PdfColor = PdfColor::new(0, 0, 0, 255);

    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::WHITE instead."
    )]
    pub const SOLID_WHITE: PdfColor = PdfColor::new(255, 255, 255, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::RED instead."
    )]
    pub const SOLID_RED: PdfColor = PdfColor::new(255, 0, 0, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::GREEN instead."
    )]
    pub const SOLID_GREEN: PdfColor = PdfColor::new(0, 255, 0, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::BLUE instead."
    )]
    pub const SOLID_BLUE: PdfColor = PdfColor::new(0, 0, 255, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::MAGENTA instead."
    )]
    pub const SOLID_MAGENTA: PdfColor = PdfColor::new(255, 0, 255, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::CYAN instead."
    )]
    pub const SOLID_CYAN: PdfColor = PdfColor::new(0, 255, 255, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::YELLOW instead."
    )]
    pub const SOLID_YELLOW: PdfColor = PdfColor::new(255, 255, 0, 255);
    #[deprecated(
        since = "0.8.6",
        note = "The SOLID_ prefix is superfluous. Use PdfColor::BLACK instead."
    )]
    pub const SOLID_BLACK: PdfColor = PdfColor::new(0, 0, 0, 255);

    pub const GREY_90: PdfColor = PdfColor::new(230, 230, 230, 255);
    pub const GREY_80: PdfColor = PdfColor::new(204, 204, 204, 255);
    pub const GREY_70: PdfColor = PdfColor::new(179, 179, 179, 255);
    pub const GREY_60: PdfColor = PdfColor::new(153, 153, 153, 255);
    pub const GREY_50: PdfColor = PdfColor::new(128, 128, 128, 255);
    pub const GREY_40: PdfColor = PdfColor::new(102, 102, 102, 255);
    pub const GREY_30: PdfColor = PdfColor::new(77, 77, 77, 255);
    pub const GREY_20: PdfColor = PdfColor::new(51, 51, 51, 255);
    pub const GREY_10: PdfColor = PdfColor::new(26, 26, 26, 255);

    // Additional colors taken from https://www.rapidtables.com/web/color/RGB_Color.html

    pub const LIME: PdfColor = PdfColor::new(0, 255, 0, 255);
    pub const BLUE: PdfColor = PdfColor::new(0, 0, 255, 255);
    pub const YELLOW: PdfColor = PdfColor::new(255, 255, 0, 255);
    pub const CYAN: PdfColor = PdfColor::new(0, 255, 255, 255);
    pub const MAGENTA: PdfColor = PdfColor::new(255, 0, 255, 255);
    pub const SILVER: PdfColor = PdfColor::new(192, 192, 192, 255);
    pub const OLIVE: PdfColor = PdfColor::new(128, 128, 0, 255);
    pub const PURPLE: PdfColor = PdfColor::new(128, 0, 128, 255);
    pub const TEAL: PdfColor = PdfColor::new(0, 128, 128, 255);
    pub const NAVY: PdfColor = PdfColor::new(0, 0, 128, 255);
    pub const MAROON: PdfColor = PdfColor::new(128, 0, 0, 255);
    pub const DARK_RED: PdfColor = PdfColor::new(139, 0, 0, 255);
    pub const BROWN: PdfColor = PdfColor::new(165, 42, 42, 255);
    pub const FIREBRICK: PdfColor = PdfColor::new(178, 34, 34, 255);
    pub const CRIMSON: PdfColor = PdfColor::new(220, 20, 60, 255);
    pub const RED: PdfColor = PdfColor::new(255, 0, 0, 255);
    pub const TOMATO: PdfColor = PdfColor::new(255, 99, 71, 255);
    pub const CORAL: PdfColor = PdfColor::new(255, 127, 80, 255);
    pub const INDIAN_RED: PdfColor = PdfColor::new(205, 92, 92, 255);
    pub const LIGHT_CORAL: PdfColor = PdfColor::new(240, 128, 128, 255);
    pub const DARK_SALMON: PdfColor = PdfColor::new(233, 150, 122, 255);
    pub const SALMON: PdfColor = PdfColor::new(250, 128, 114, 255);
    pub const LIGHT_SALMON: PdfColor = PdfColor::new(255, 160, 122, 255);
    pub const ORANGE_RED: PdfColor = PdfColor::new(255, 69, 0, 255);
    pub const DARK_ORANGE: PdfColor = PdfColor::new(255, 140, 0, 255);
    pub const ORANGE: PdfColor = PdfColor::new(255, 165, 0, 255);
    pub const GOLD: PdfColor = PdfColor::new(255, 215, 0, 255);
    pub const DARK_GOLDEN_ROD: PdfColor = PdfColor::new(184, 134, 11, 255);
    pub const GOLDEN_ROD: PdfColor = PdfColor::new(218, 165, 32, 255);
    pub const PALE_GOLDEN_ROD: PdfColor = PdfColor::new(238, 232, 170, 255);
    pub const DARK_KHAKI: PdfColor = PdfColor::new(189, 183, 107, 255);
    pub const KHAKI: PdfColor = PdfColor::new(240, 230, 140, 255);
    pub const YELLOW_GREEN: PdfColor = PdfColor::new(154, 205, 50, 255);
    pub const DARK_OLIVE_GREEN: PdfColor = PdfColor::new(85, 107, 47, 255);
    pub const OLIVE_DRAB: PdfColor = PdfColor::new(107, 142, 35, 255);
    pub const LAWN_GREEN: PdfColor = PdfColor::new(124, 252, 0, 255);
    pub const CHARTREUSE: PdfColor = PdfColor::new(127, 255, 0, 255);
    pub const GREEN_YELLOW: PdfColor = PdfColor::new(173, 255, 47, 255);
    pub const DARK_GREEN: PdfColor = PdfColor::new(0, 100, 0, 255);
    pub const GREEN: PdfColor = PdfColor::new(0, 128, 0, 255);
    pub const FOREST_GREEN: PdfColor = PdfColor::new(34, 139, 34, 255);
    pub const LIME_GREEN: PdfColor = PdfColor::new(50, 205, 50, 255);
    pub const LIGHT_GREEN: PdfColor = PdfColor::new(144, 238, 144, 255);
    pub const PALE_GREEN: PdfColor = PdfColor::new(152, 251, 152, 255);
    pub const DARK_SEA_GREEN: PdfColor = PdfColor::new(143, 188, 143, 255);
    pub const MEDIUM_SPRING_GREEN: PdfColor = PdfColor::new(0, 250, 154, 255);
    pub const SPRING_GREEN: PdfColor = PdfColor::new(0, 255, 127, 255);
    pub const SEA_GREEN: PdfColor = PdfColor::new(46, 139, 87, 255);
    pub const MEDIUM_AQUA_MARINE: PdfColor = PdfColor::new(102, 205, 170, 255);
    pub const MEDIUM_SEA_GREEN: PdfColor = PdfColor::new(60, 179, 113, 255);
    pub const LIGHT_SEA_GREEN: PdfColor = PdfColor::new(32, 178, 170, 255);
    pub const DARK_SLATE_GRAY: PdfColor = PdfColor::new(47, 79, 79, 255);
    pub const DARK_CYAN: PdfColor = PdfColor::new(0, 139, 139, 255);
    pub const AQUA: PdfColor = PdfColor::new(0, 255, 255, 255);
    pub const LIGHT_CYAN: PdfColor = PdfColor::new(224, 255, 255, 255);
    pub const DARK_TURQUOISE: PdfColor = PdfColor::new(0, 206, 209, 255);
    pub const TURQUOISE: PdfColor = PdfColor::new(64, 224, 208, 255);
    pub const MEDIUM_TURQUOISE: PdfColor = PdfColor::new(72, 209, 204, 255);
    pub const PALE_TURQUOISE: PdfColor = PdfColor::new(175, 238, 238, 255);
    pub const AQUA_MARINE: PdfColor = PdfColor::new(127, 255, 212, 255);
    pub const POWDER_BLUE: PdfColor = PdfColor::new(176, 224, 230, 255);
    pub const CADET_BLUE: PdfColor = PdfColor::new(95, 158, 160, 255);
    pub const STEEL_BLUE: PdfColor = PdfColor::new(70, 130, 180, 255);
    pub const CORNFLOWER_BLUE: PdfColor = PdfColor::new(100, 149, 237, 255);
    pub const DEEP_SKY_BLUE: PdfColor = PdfColor::new(0, 191, 255, 255);
    pub const DODGER_BLUE: PdfColor = PdfColor::new(30, 144, 255, 255);
    pub const LIGHT_BLUE: PdfColor = PdfColor::new(173, 216, 230, 255);
    pub const SKY_BLUE: PdfColor = PdfColor::new(135, 206, 235, 255);
    pub const LIGHT_SKY_BLUE: PdfColor = PdfColor::new(135, 206, 250, 255);
    pub const MIDNIGHT_BLUE: PdfColor = PdfColor::new(25, 25, 112, 255);
    pub const DARK_BLUE: PdfColor = PdfColor::new(0, 0, 139, 255);
    pub const MEDIUM_BLUE: PdfColor = PdfColor::new(0, 0, 205, 255);
    pub const ROYAL_BLUE: PdfColor = PdfColor::new(65, 105, 225, 255);
    pub const BLUE_VIOLET: PdfColor = PdfColor::new(138, 43, 226, 255);
    pub const INDIGO: PdfColor = PdfColor::new(75, 0, 130, 255);
    pub const DARK_SLATE_BLUE: PdfColor = PdfColor::new(72, 61, 139, 255);
    pub const SLATE_BLUE: PdfColor = PdfColor::new(106, 90, 205, 255);
    pub const MEDIUM_SLATE_BLUE: PdfColor = PdfColor::new(123, 104, 238, 255);
    pub const MEDIUM_PURPLE: PdfColor = PdfColor::new(147, 112, 219, 255);
    pub const DARK_MAGENTA: PdfColor = PdfColor::new(139, 0, 139, 255);
    pub const DARK_VIOLET: PdfColor = PdfColor::new(148, 0, 211, 255);
    pub const DARK_ORCHID: PdfColor = PdfColor::new(153, 50, 204, 255);
    pub const MEDIUM_ORCHID: PdfColor = PdfColor::new(186, 85, 211, 255);
    pub const THISTLE: PdfColor = PdfColor::new(216, 191, 216, 255);
    pub const PLUM: PdfColor = PdfColor::new(221, 160, 221, 255);
    pub const VIOLET: PdfColor = PdfColor::new(238, 130, 238, 255);
    pub const ORCHID: PdfColor = PdfColor::new(218, 112, 214, 255);
    pub const MEDIUM_VIOLET_RED: PdfColor = PdfColor::new(199, 21, 133, 255);
    pub const PALE_VIOLET_RED: PdfColor = PdfColor::new(219, 112, 147, 255);
    pub const DEEP_PINK: PdfColor = PdfColor::new(255, 20, 147, 255);
    pub const HOT_PINK: PdfColor = PdfColor::new(255, 105, 180, 255);
    pub const LIGHT_PINK: PdfColor = PdfColor::new(255, 182, 193, 255);
    pub const PINK: PdfColor = PdfColor::new(255, 192, 203, 255);
    pub const ANTIQUE_WHITE: PdfColor = PdfColor::new(250, 235, 215, 255);
    pub const BEIGE: PdfColor = PdfColor::new(245, 245, 220, 255);
    pub const BISQUE: PdfColor = PdfColor::new(255, 228, 196, 255);
    pub const BLANCHED_ALMOND: PdfColor = PdfColor::new(255, 235, 205, 255);
    pub const WHEAT: PdfColor = PdfColor::new(245, 222, 179, 255);
    pub const CORN_SILK: PdfColor = PdfColor::new(255, 248, 220, 255);
    pub const LEMON_CHIFFON: PdfColor = PdfColor::new(255, 250, 205, 255);
    pub const LIGHT_GOLDEN_ROD_YELLOW: PdfColor = PdfColor::new(250, 250, 210, 255);
    pub const LIGHT_YELLOW: PdfColor = PdfColor::new(255, 255, 224, 255);
    pub const SADDLE_BROWN: PdfColor = PdfColor::new(139, 69, 19, 255);
    pub const SIENNA: PdfColor = PdfColor::new(160, 82, 45, 255);
    pub const CHOCOLATE: PdfColor = PdfColor::new(210, 105, 30, 255);
    pub const PERU: PdfColor = PdfColor::new(205, 133, 63, 255);
    pub const SANDY_BROWN: PdfColor = PdfColor::new(244, 164, 96, 255);
    pub const BURLY_WOOD: PdfColor = PdfColor::new(222, 184, 135, 255);
    pub const TAN: PdfColor = PdfColor::new(210, 180, 140, 255);
    pub const ROSY_BROWN: PdfColor = PdfColor::new(188, 143, 143, 255);
    pub const MOCCASIN: PdfColor = PdfColor::new(255, 228, 181, 255);
    pub const NAVAJO_WHITE: PdfColor = PdfColor::new(255, 222, 173, 255);
    pub const PEACH_PUFF: PdfColor = PdfColor::new(255, 218, 185, 255);
    pub const MISTY_ROSE: PdfColor = PdfColor::new(255, 228, 225, 255);
    pub const LAVENDER_BLUSH: PdfColor = PdfColor::new(255, 240, 245, 255);
    pub const LINEN: PdfColor = PdfColor::new(250, 240, 230, 255);
    pub const OLD_LACE: PdfColor = PdfColor::new(253, 245, 230, 255);
    pub const PAPAYA_WHIP: PdfColor = PdfColor::new(255, 239, 213, 255);
    pub const SEA_SHELL: PdfColor = PdfColor::new(255, 245, 238, 255);
    pub const MINT_CREAM: PdfColor = PdfColor::new(245, 255, 250, 255);
    pub const SLATE_GRAY: PdfColor = PdfColor::new(112, 128, 144, 255);
    pub const LIGHT_SLATE_GRAY: PdfColor = PdfColor::new(119, 136, 153, 255);
    pub const LIGHT_STEEL_BLUE: PdfColor = PdfColor::new(176, 196, 222, 255);
    pub const LAVENDER: PdfColor = PdfColor::new(230, 230, 250, 255);
    pub const FLORAL_WHITE: PdfColor = PdfColor::new(255, 250, 240, 255);
    pub const ALICE_BLUE: PdfColor = PdfColor::new(240, 248, 255, 255);
    pub const GHOST_WHITE: PdfColor = PdfColor::new(248, 248, 255, 255);
    pub const HONEYDEW: PdfColor = PdfColor::new(240, 255, 240, 255);
    pub const IVORY: PdfColor = PdfColor::new(255, 255, 240, 255);
    pub const AZURE: PdfColor = PdfColor::new(240, 255, 255, 255);
    pub const SNOW: PdfColor = PdfColor::new(255, 250, 250, 255);
    pub const DIM_GREY: PdfColor = PdfColor::new(105, 105, 105, 255);
    pub const GREY: PdfColor = PdfColor::new(128, 128, 128, 255);
    pub const DARK_GREY: PdfColor = PdfColor::new(169, 169, 169, 255);
    pub const LIGHT_GREY: PdfColor = PdfColor::new(211, 211, 211, 255);
    pub const GAINSBORO: PdfColor = PdfColor::new(220, 220, 220, 255);
    pub const WHITE_SMOKE: PdfColor = PdfColor::new(245, 245, 245, 255);

    #[inline]
    // The from_pdfium() function is not currently used, but we expect it to be in future
    #[allow(dead_code)]
    pub(crate) const fn from_pdfium(argb: FPDF_DWORD) -> Self {
        Self::new(
            ((argb & 0xFF0000) >> 16) as u8,
            ((argb & 0xFF00) >> 8) as u8,
            (argb & 0xFF) as u8,
            ((argb & 0xFF000000) >> 24) as u8,
        )
    }

    /// Constructs a new [PdfColor] object from the given arguments.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        }
    }

    /// Returns the result of importing the given hexadecimal color specification,
    /// as in HTML. For example, `#800080` represents a shade of purple with 100% opacity,
    /// and `#40800080` is the same shade of purple with 25% opacity. The leading hash
    /// symbol is required.
    pub fn from_hex(hex: &str) -> Result<Self, PdfiumError> {
        if hex.starts_with('#') {
            match hex.len() {
                7 => {
                    // Potential HTML-style RGB triplet in hexadecimal format
                    // with leading #.

                    FPDF_DWORD::from_str_radix(&hex[1..hex.len()], 16)
                        .map(PdfColor::from_pdfium)
                        .map(|color| color.with_alpha(255))
                        .map_err(PdfiumError::ParseHexadecimalColorError)
                }
                9 => {
                    // Potential ARGB quadruplet in hexadecimal format with leading #.

                    FPDF_DWORD::from_str_radix(&hex[1..hex.len()], 16)
                        .map(PdfColor::from_pdfium)
                        .map_err(PdfiumError::ParseHexadecimalColorError)
                }
                _ => Err(PdfiumError::ParseHexadecimalColorUnexpectedLength),
            }
        } else {
            Err(PdfiumError::ParseHexadecimalColorMissingLeadingHash)
        }
    }

    /// Returns the result of averaging the RGB and alpha values of the two given [PdfColor] objects.
    #[inline]
    pub const fn mix(a: &PdfColor, b: &PdfColor) -> Self {
        a.mix_with(b)
    }

    /// Returns the result of averaging the RGB and alpha values of this [PdfColor] with the given [PdfColor].
    #[inline]
    pub const fn mix_with(&self, other: &PdfColor) -> Self {
        Self {
            r: (self.r + other.r) / 2,
            g: (self.g + other.g) / 2,
            b: (self.b + other.b) / 2,
            a: (self.a + other.a) / 2,
        }
    }

    /// Constructs a new [PdfColor] by copying the red, green, and blue color components
    /// of this color and applying the given alpha value.
    #[inline]
    pub const fn with_alpha(&self, alpha: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    /// Returns the alpha (opacity) component of this color, with 0 = completely transparent
    /// and 255 = completely opaque (solid).
    #[inline]
    pub fn alpha(&self) -> u8 {
        self.a
    }

    /// Returns the red component of this color.
    #[inline]
    pub fn red(&self) -> u8 {
        self.r
    }

    /// Returns the green component of this color.
    #[inline]
    pub fn green(&self) -> u8 {
        self.g
    }

    /// Returns the blue component of this color.
    #[inline]
    pub fn blue(&self) -> u8 {
        self.b
    }

    /// Returns the hexadecimal representation of this color, as in HTML, without
    /// a leading hash symbol. Excludes the alpha channel value. For example,
    /// `PdfColor::PURPLE.to_hex()` will return "800080".
    #[inline]
    pub fn to_hex(&self) -> String {
        format!("{:02X?}{:02X?}{:02X?}", self.r, self.g, self.b)
    }

    /// Returns the hexadecimal representation of this color, as in HTML, without
    /// a leading hash symbol. Includes the alpha channel value. For example,
    /// `PdfColor::PURPLE.to_hex_with_alpha()` will return "FF800080".
    #[inline]
    pub fn to_hex_with_alpha(&self) -> String {
        format!("{:02X?}{:02X?}{:02X?}{:02X?}", self.a, self.r, self.g, self.b)
    }

    /// Returns this color encoded as a 32-bit hexadecimal 0xAARRGGBB value,
    /// suitable for passing to Pdfium.
    #[inline]
    pub(crate) fn as_pdfium_color(&self) -> FPDF_DWORD {
        let (alpha, r, g, b) = self.color_components();

        ((alpha << 24) | (b << 16) | (g << 8) | r) as FPDF_DWORD
    }

    /// Returns a tuple comprising this color encoded as a 32-bit hexadecimal 0xFFRRGGBB value
    /// and this alpha encoded as an 8-bit value, suitable for passing to Pdfium.
    #[inline]
    pub(crate) fn as_pdfium_color_with_alpha(&self) -> (FPDF_DWORD, u8) {
        let (alpha, r, g, b) = self.color_components();

        (((0xFF << 24) | (b << 16) | (g << 8) | r) as FPDF_DWORD, alpha as u8)
    }

    /// Returns the raw color components of this [PdfColor] in the order (alpha, R, G, B).
    #[inline]
    fn color_components(&self) -> (FPDF_DWORD, FPDF_DWORD, FPDF_DWORD, FPDF_DWORD) {
        (
            self.a as FPDF_DWORD,
            self.r as FPDF_DWORD,
            self.g as FPDF_DWORD,
            self.b as FPDF_DWORD,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_from_hex() {
        assert_eq!(
            PdfColor::from_hex("#800080").unwrap().color_components(),
            PdfColor::PURPLE.color_components()
        );
        assert_eq!(
            PdfColor::from_hex("#FF800080").unwrap().color_components(),
            PdfColor::PURPLE.color_components()
        );
        assert_eq!(
            PdfColor::from_hex("#40800080").unwrap().color_components(),
            PdfColor::PURPLE.with_alpha(64).color_components()
        );
    }

    #[test]
    fn test_to_hex() {
        assert_eq!(PdfColor::PURPLE.to_hex(), "800080");
        assert_eq!(PdfColor::PURPLE.with_alpha(64).to_hex(), "800080");
        assert_eq!(PdfColor::PURPLE.to_hex_with_alpha(), "FF800080");
        assert_eq!(PdfColor::PURPLE.with_alpha(64).to_hex_with_alpha(), "40800080");
    }
}
