//! DOCX theme parsing from `word/theme/theme1.xml`.
//!
//! Parses DrawingML theme elements to extract color schemes and font schemes.
//! This module provides access to document theme colors and fonts that can be
//! referenced from styles and other content.

use crate::error::{Result, XbergError};
use crate::extraction::ooxml_constants::DRAWINGML_NAMESPACE;

// --- Types ---

/// A theme color definition, either direct RGB or a system color with fallback.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeColor {
    /// Direct hex RGB color (e.g., "156082").
    Rgb(String),
    /// System color with fallback RGB (e.g., "windowText" with lastClr "000000").
    System {
        /// System color name (e.g., `"windowText"`, `"window"`).
        name: String,
        /// Fallback hex RGB color used when the system color cannot be resolved.
        last_color: String,
    },
}

/// Color scheme containing all 12 standard Office theme colors.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct ColorScheme {
    /// Color scheme name.
    pub name: String,
    /// Dark 1 (dark background) color.
    pub dk1: Option<ThemeColor>,
    /// Light 1 (light background) color.
    pub lt1: Option<ThemeColor>,
    /// Dark 2 color.
    pub dk2: Option<ThemeColor>,
    /// Light 2 color.
    pub lt2: Option<ThemeColor>,
    /// Accent color 1.
    pub accent1: Option<ThemeColor>,
    /// Accent color 2.
    pub accent2: Option<ThemeColor>,
    /// Accent color 3.
    pub accent3: Option<ThemeColor>,
    /// Accent color 4.
    pub accent4: Option<ThemeColor>,
    /// Accent color 5.
    pub accent5: Option<ThemeColor>,
    /// Accent color 6.
    pub accent6: Option<ThemeColor>,
    /// Hyperlink color.
    pub hlink: Option<ThemeColor>,
    /// Followed hyperlink color.
    pub fol_hlink: Option<ThemeColor>,
}

/// Font scheme containing major (heading) and minor (body) fonts.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct FontScheme {
    /// Font scheme name.
    pub name: String,
    /// Major (heading) font - Latin script.
    pub major_latin: Option<String>,
    /// Major (heading) font - East Asian script.
    pub major_east_asian: Option<String>,
    /// Major (heading) font - Complex script.
    pub major_complex_script: Option<String>,
    /// Minor (body) font - Latin script.
    pub minor_latin: Option<String>,
    /// Minor (body) font - East Asian script.
    pub minor_east_asian: Option<String>,
    /// Minor (body) font - Complex script.
    pub minor_complex_script: Option<String>,
}

/// Complete theme with color scheme and font scheme.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct Theme {
    /// Theme name (e.g., "Office Theme").
    pub name: String,
    /// Color scheme (12 standard colors).
    pub color_scheme: Option<ColorScheme>,
    /// Font scheme (major and minor fonts).
    pub font_scheme: Option<FontScheme>,
}

// --- Parsing ---

/// Parse `word/theme/theme1.xml` content into a `Theme`.
///
/// Uses `roxmltree` for tree-based XML parsing of DrawingML theme elements.
///
/// # Arguments
/// * `xml` - The theme XML content as a string
///
/// # Returns
/// * `Ok(Theme)` - The parsed theme
/// * `Err(XbergError)` - If parsing fails
pub(crate) fn parse_theme_xml(xml: &str) -> Result<Theme> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| XbergError::parsing(format!("Failed to parse theme.xml: {}", e)))?;

    let root = doc.root_element();

    let mut theme = Theme {
        name: root.attribute("name").map(|s| s.to_string()).unwrap_or_default(),
        color_scheme: None,
        font_scheme: None,
    };

    // Find themeElements
    if let Some(theme_elements) = root.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "themeElements"
            && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        // Parse color scheme
        if let Some(color_scheme_elem) = theme_elements.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "clrScheme"
                && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        }) {
            theme.color_scheme = Some(parse_color_scheme(color_scheme_elem));
        }

        // Parse font scheme
        if let Some(font_scheme_elem) = theme_elements.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "fontScheme"
                && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        }) {
            theme.font_scheme = Some(parse_font_scheme(font_scheme_elem));
        }
    }

    Ok(theme)
}

/// Parse a color scheme element (`a:clrScheme`).
fn parse_color_scheme(elem: roxmltree::Node) -> ColorScheme {
    let name = elem.attribute("name").map(|s| s.to_string()).unwrap_or_default();

    let mut scheme = ColorScheme {
        name,
        ..Default::default()
    };

    for child in elem.children() {
        if !child.is_element() {
            continue;
        }

        let tag_name = child.tag_name().name();
        let namespace = child.tag_name().namespace();

        if namespace != Some(DRAWINGML_NAMESPACE) {
            continue;
        }

        let color = parse_color_element(child);

        match tag_name {
            "dk1" => scheme.dk1 = color,
            "lt1" => scheme.lt1 = color,
            "dk2" => scheme.dk2 = color,
            "lt2" => scheme.lt2 = color,
            "accent1" => scheme.accent1 = color,
            "accent2" => scheme.accent2 = color,
            "accent3" => scheme.accent3 = color,
            "accent4" => scheme.accent4 = color,
            "accent5" => scheme.accent5 = color,
            "accent6" => scheme.accent6 = color,
            "hlink" => scheme.hlink = color,
            "folHlink" => scheme.fol_hlink = color,
            _ => {}
        }
    }

    scheme
}

/// Parse a single color element (containing either `a:srgbClr` or `a:sysClr`).
fn parse_color_element(elem: roxmltree::Node) -> Option<ThemeColor> {
    for child in elem.children() {
        if !child.is_element() {
            continue;
        }

        let tag_name = child.tag_name().name();
        let namespace = child.tag_name().namespace();

        if namespace != Some(DRAWINGML_NAMESPACE) {
            continue;
        }

        match tag_name {
            "srgbClr" => {
                if let Some(val) = child.attribute("val") {
                    return Some(ThemeColor::Rgb(val.to_string()));
                }
            }
            "sysClr" => {
                if let Some(name) = child.attribute("val") {
                    let last_color = child.attribute("lastClr").unwrap_or_default().to_string();
                    return Some(ThemeColor::System {
                        name: name.to_string(),
                        last_color,
                    });
                }
            }
            _ => {}
        }
    }

    None
}

/// Parse a font scheme element (`a:fontScheme`).
fn parse_font_scheme(elem: roxmltree::Node) -> FontScheme {
    let name = elem.attribute("name").map(|s| s.to_string()).unwrap_or_default();

    let mut scheme = FontScheme {
        name,
        ..Default::default()
    };

    for child in elem.children() {
        if !child.is_element() {
            continue;
        }

        let tag_name = child.tag_name().name();
        let namespace = child.tag_name().namespace();

        if namespace != Some(DRAWINGML_NAMESPACE) {
            continue;
        }

        match tag_name {
            "majorFont" => parse_font_family(child, &mut scheme, true),
            "minorFont" => parse_font_family(child, &mut scheme, false),
            _ => {}
        }
    }

    scheme
}

/// Parse major or minor font family elements.
fn parse_font_family(elem: roxmltree::Node, scheme: &mut FontScheme, is_major: bool) {
    for child in elem.children() {
        if !child.is_element() {
            continue;
        }

        let tag_name = child.tag_name().name();
        let namespace = child.tag_name().namespace();

        if namespace != Some(DRAWINGML_NAMESPACE) {
            continue;
        }

        let typeface = child
            .attribute("typeface")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        match tag_name {
            "latin" => {
                if is_major {
                    scheme.major_latin = typeface;
                } else {
                    scheme.minor_latin = typeface;
                }
            }
            "ea" => {
                if is_major {
                    scheme.major_east_asian = typeface;
                } else {
                    scheme.minor_east_asian = typeface;
                }
            }
            "cs" => {
                if is_major {
                    scheme.major_complex_script = typeface;
                } else {
                    scheme.minor_complex_script = typeface;
                }
            }
            _ => {}
        }
    }
}

// --- Utilities ---

/// Resolve a theme color reference to an RGB hex string.
///
/// Given a color reference like "accent1" or "dk1", returns the RGB hex value
/// from the theme if found. For system colors, returns the fallback `lastClr`.
/// For RGB colors, returns the hex value directly.
///
/// # Arguments
/// * `theme` - The theme containing the color definitions
/// * `color_ref` - The color reference name (e.g., "accent1", "dk1")
///
/// # Returns
/// * `Some(&str)` - The RGB hex color (without '#')
/// * `None` - If the color reference is not found
#[cfg(test)]
pub(crate) fn resolve_theme_color<'a>(theme: &'a Theme, color_ref: &str) -> Option<&'a str> {
    if let Some(color_scheme) = &theme.color_scheme {
        let color = match color_ref {
            "dk1" => &color_scheme.dk1,
            "lt1" => &color_scheme.lt1,
            "dk2" => &color_scheme.dk2,
            "lt2" => &color_scheme.lt2,
            "accent1" => &color_scheme.accent1,
            "accent2" => &color_scheme.accent2,
            "accent3" => &color_scheme.accent3,
            "accent4" => &color_scheme.accent4,
            "accent5" => &color_scheme.accent5,
            "accent6" => &color_scheme.accent6,
            "hlink" => &color_scheme.hlink,
            "folHlink" => &color_scheme.fol_hlink,
            _ => return None,
        };

        return color.as_ref().map(|c| match c {
            ThemeColor::Rgb(hex) => hex.as_str(),
            ThemeColor::System { last_color, .. } => last_color.as_str(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const STANDARD_THEME_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="0E2841"/></a:dk2>
      <a:lt2><a:srgbClr val="E8E8E8"/></a:lt2>
      <a:accent1><a:srgbClr val="156082"/></a:accent1>
      <a:accent2><a:srgbClr val="E97132"/></a:accent2>
      <a:accent3><a:srgbClr val="196B24"/></a:accent3>
      <a:accent4><a:srgbClr val="0F9ED5"/></a:accent4>
      <a:accent5><a:srgbClr val="A02B93"/></a:accent5>
      <a:accent6><a:srgbClr val="4EA72E"/></a:accent6>
      <a:hlink><a:srgbClr val="467886"/></a:hlink>
      <a:folHlink><a:srgbClr val="96607D"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont>
        <a:latin typeface="Aptos Display"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Aptos"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;

    #[test]
    fn test_parse_standard_office_theme() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");

        assert_eq!(theme.name, "Office Theme");
        assert!(theme.color_scheme.is_some());
        assert!(theme.font_scheme.is_some());

        let color_scheme = theme.color_scheme.as_ref().unwrap();
        assert_eq!(color_scheme.name, "Office");
    }

    #[test]
    fn test_parse_color_scheme_all_12_colors() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");
        let color_scheme = theme.color_scheme.as_ref().expect("No color scheme");

        assert!(color_scheme.dk1.is_some());
        assert!(color_scheme.lt1.is_some());
        assert!(color_scheme.dk2.is_some());
        assert!(color_scheme.lt2.is_some());
        assert!(color_scheme.accent1.is_some());
        assert!(color_scheme.accent2.is_some());
        assert!(color_scheme.accent3.is_some());
        assert!(color_scheme.accent4.is_some());
        assert!(color_scheme.accent5.is_some());
        assert!(color_scheme.accent6.is_some());
        assert!(color_scheme.hlink.is_some());
        assert!(color_scheme.fol_hlink.is_some());
    }

    #[test]
    fn test_parse_font_scheme() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");
        let font_scheme = theme.font_scheme.as_ref().expect("No font scheme");

        assert_eq!(font_scheme.name, "Office");
        assert_eq!(font_scheme.major_latin, Some("Aptos Display".to_string()));
        assert_eq!(font_scheme.minor_latin, Some("Aptos".to_string()));
    }

    #[test]
    fn test_resolve_theme_color_by_reference() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");

        // Test RGB color
        let accent1 = resolve_theme_color(&theme, "accent1");
        assert_eq!(accent1, Some("156082"));

        // Test system color (returns lastClr)
        let dk1 = resolve_theme_color(&theme, "dk1");
        assert_eq!(dk1, Some("000000"));

        let lt1 = resolve_theme_color(&theme, "lt1");
        assert_eq!(lt1, Some("FFFFFF"));
    }

    #[test]
    fn test_resolve_theme_color_all_colors() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");

        let colors = vec![
            "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink",
            "folHlink",
        ];

        for color_ref in colors {
            let resolved = resolve_theme_color(&theme, color_ref);
            assert!(resolved.is_some(), "Color {} should be resolvable", color_ref);
            let color_hex = resolved.unwrap();
            assert!(!color_hex.is_empty(), "Color {} hex should not be empty", color_ref);
        }
    }

    #[test]
    fn test_resolve_theme_color_invalid_reference() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");

        let invalid = resolve_theme_color(&theme, "invalid_color");
        assert_eq!(invalid, None);
    }

    #[test]
    fn test_parse_theme_name() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");
        assert_eq!(theme.name, "Office Theme");
    }

    #[test]
    fn test_parse_theme_empty_xml() {
        let empty_theme = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
</a:theme>"#;

        let theme = parse_theme_xml(empty_theme).expect("Failed to parse theme");
        assert_eq!(theme.name, "");
        assert!(theme.color_scheme.is_none());
        assert!(theme.font_scheme.is_none());
    }

    #[test]
    fn test_parse_system_vs_rgb_colors() {
        let theme = parse_theme_xml(STANDARD_THEME_XML).expect("Failed to parse theme");
        let color_scheme = theme.color_scheme.as_ref().unwrap();

        // dk1 should be system color
        match &color_scheme.dk1 {
            Some(ThemeColor::System { name, last_color }) => {
                assert_eq!(name, "windowText");
                assert_eq!(last_color, "000000");
            }
            _ => panic!("dk1 should be a system color"),
        }

        // accent1 should be RGB color
        match &color_scheme.accent1 {
            Some(ThemeColor::Rgb(hex)) => {
                assert_eq!(hex, "156082");
            }
            _ => panic!("accent1 should be an RGB color"),
        }
    }
}
