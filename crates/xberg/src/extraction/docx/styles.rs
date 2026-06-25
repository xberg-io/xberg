//! DOCX style resolution from `word/styles.xml`.
//!
//! Parses the styles XML and resolves style inheritance chains to produce
//! fully-flattened run and paragraph properties for any given style ID.

use ahash::AHashMap;

use crate::error::{Result, XbergError};
use crate::extraction::ooxml_constants::WORDPROCESSINGML_NAMESPACE;
// --- Types ---

/// The type of a style definition in DOCX.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum StyleType {
    /// Style applied to an entire paragraph (`<w:style w:type="paragraph">`).
    Paragraph,
    /// Style applied to a character run (`<w:style w:type="character">`).
    Character,
    /// Style applied to a table (`<w:style w:type="table">`).
    Table,
    /// Style applied to a list numbering definition (`<w:style w:type="numbering">`).
    Numbering,
}
/// Run-level formatting properties (bold, italic, font, size, color, etc.).
///
/// All fields are `Option` so that inheritance resolution can distinguish
/// "not set" (`None`) from "explicitly set" (`Some`).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunProperties {
    /// Bold formatting.
    pub bold: Option<bool>,
    /// Italic formatting.
    pub italic: Option<bool>,
    /// Underline formatting.
    pub underline: Option<bool>,
    /// Strikethrough formatting.
    pub strikethrough: Option<bool>,
    /// Hex RGB color, e.g. `"2F5496"`.
    pub color: Option<String>,
    /// Font size in half-points (`w:sz` val). Divide by 2 to get points.
    pub font_size_half_points: Option<i32>,
    /// ASCII font family (`w:rFonts w:ascii`).
    pub font_ascii: Option<String>,
    /// ASCII theme font (`w:rFonts w:asciiTheme`).
    pub font_ascii_theme: Option<String>,
    /// Vertical alignment: "superscript", "subscript", or "baseline".
    pub vert_align: Option<String>,
    /// High ANSI font family (w:rFonts w:hAnsi).
    pub font_h_ansi: Option<String>,
    /// Complex script font family (w:rFonts w:cs).
    pub font_cs: Option<String>,
    /// East Asian font family (w:rFonts w:eastAsia).
    pub font_east_asia: Option<String>,
    /// Highlight color name (e.g., "yellow", "green", "cyan").
    pub highlight: Option<String>,
    /// All caps text transformation.
    pub caps: Option<bool>,
    /// Small caps text transformation.
    pub small_caps: Option<bool>,
    /// Text shadow effect.
    pub shadow: Option<bool>,
    /// Text outline effect.
    pub outline: Option<bool>,
    /// Text emboss effect.
    pub emboss: Option<bool>,
    /// Text imprint (engrave) effect.
    pub imprint: Option<bool>,
    /// Character spacing in twips (from w:spacing w:val).
    pub char_spacing: Option<i32>,
    /// Vertical position offset in half-points (from w:position w:val).
    pub position: Option<i32>,
    /// Kerning threshold in half-points (from w:kern w:val).
    pub kern: Option<i32>,
    /// Theme color reference (e.g., "accent1", "dk1").
    pub theme_color: Option<String>,
    /// Theme color tint modification (hex value).
    pub theme_tint: Option<String>,
    /// Theme color shade modification (hex value).
    pub theme_shade: Option<String>,
}
#[cfg_attr(alef, alef(skip))]
/// Paragraph-level formatting properties (alignment, spacing, indentation, etc.).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParagraphProperties {
    /// `"left"`, `"center"`, `"right"`, `"both"` (justified).
    pub alignment: Option<String>,
    /// Spacing before paragraph in twips.
    pub spacing_before: Option<i32>,
    /// Spacing after paragraph in twips.
    pub spacing_after: Option<i32>,
    /// Line spacing in twips or 240ths of a line.
    pub spacing_line: Option<i32>,
    /// Line spacing rule: "auto", "exact", or "atLeast".
    pub spacing_line_rule: Option<String>,
    /// Left indentation in twips.
    pub indent_left: Option<i32>,
    /// Right indentation in twips.
    pub indent_right: Option<i32>,
    /// First-line indentation in twips.
    pub indent_first_line: Option<i32>,
    /// Hanging indentation in twips.
    pub indent_hanging: Option<i32>,
    /// Outline level 0-8 for heading levels.
    pub outline_level: Option<u8>,
    /// Keep with next paragraph on same page.
    pub keep_next: Option<bool>,
    /// Keep all lines of paragraph on same page.
    pub keep_lines: Option<bool>,
    /// Force page break before paragraph.
    pub page_break_before: Option<bool>,
    /// Prevent widow/orphan lines.
    pub widow_control: Option<bool>,
    /// Suppress automatic hyphenation.
    pub suppress_auto_hyphens: Option<bool>,
    /// Right-to-left paragraph direction.
    pub bidi: Option<bool>,
    /// Background color hex value (from w:shd w:fill).
    pub shading_fill: Option<String>,
    /// Shading pattern value (from w:shd w:val).
    pub shading_val: Option<String>,
    /// Top border style (from w:pBdr/w:top w:val).
    pub border_top: Option<String>,
    /// Bottom border style (from w:pBdr/w:bottom w:val).
    pub border_bottom: Option<String>,
    /// Left border style (from w:pBdr/w:left w:val).
    pub border_left: Option<String>,
    /// Right border style (from w:pBdr/w:right w:val).
    pub border_right: Option<String>,
}

/// A single style definition parsed from `<w:style>` in `word/styles.xml`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct StyleDefinition {
    /// The style ID (`w:styleId` attribute).
    pub id: String,
    /// Human-readable name (`<w:name w:val="..."/>`).
    pub name: Option<String>,
    /// Style type: paragraph, character, table, or numbering.
    pub style_type: StyleType,
    /// ID of the parent style (`<w:basedOn w:val="..."/>`).
    pub based_on: Option<String>,
    /// ID of the style to apply to the next paragraph (`<w:next w:val="..."/>`).
    pub next_style: Option<String>,
    /// Whether this is the default style for its type.
    pub is_default: bool,
    /// Paragraph properties defined directly on this style.
    pub paragraph_properties: ParagraphProperties,
    /// Run properties defined directly on this style.
    pub run_properties: RunProperties,
}

/// Fully resolved (flattened) style after walking the inheritance chain.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedStyle {
    /// Fully-inherited paragraph formatting properties.
    pub paragraph_properties: ParagraphProperties,
    /// Fully-inherited run formatting properties.
    pub run_properties: RunProperties,
}

/// Catalog of all styles parsed from `word/styles.xml`, plus document defaults.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct StyleCatalog {
    /// All named style definitions keyed by their style ID.
    pub styles: AHashMap<String, StyleDefinition>,
    /// Default paragraph formatting properties for the document.
    pub default_paragraph_properties: ParagraphProperties,
    /// Default run formatting properties for the document.
    pub default_run_properties: RunProperties,
}

// --- Parsing ---

/// Parse `word/styles.xml` content into a `StyleCatalog`.
///
/// Uses `roxmltree` for tree-based XML parsing, consistent with the
/// office metadata parsing approach used elsewhere in the codebase.
pub(crate) fn parse_styles_xml(xml: &str) -> Result<StyleCatalog> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| XbergError::parsing(format!("Failed to parse styles.xml: {}", e)))?;

    let root = doc.root_element();

    let mut catalog = StyleCatalog::default();

    for child in root.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "docDefaults" => parse_doc_defaults(&child, &mut catalog),
            "style" => {
                if let Some(style_def) = parse_style_element(&child) {
                    catalog.styles.insert(style_def.id.clone(), style_def);
                }
            }
            // Skip latentStyles and anything else
            _ => {}
        }
    }

    Ok(catalog)
}

/// Parse `<w:docDefaults>` to extract default run and paragraph properties.
fn parse_doc_defaults(node: &roxmltree::Node, catalog: &mut StyleCatalog) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "rPrDefault" => {
                // Look for <w:rPr> inside <w:rPrDefault>
                if let Some(rpr) = find_child_element(&child, "rPr") {
                    catalog.default_run_properties = parse_run_properties(&rpr);
                }
            }
            "pPrDefault" => {
                // Look for <w:pPr> inside <w:pPrDefault>
                if let Some(ppr) = find_child_element(&child, "pPr") {
                    catalog.default_paragraph_properties = parse_paragraph_properties(&ppr);
                }
            }
            _ => {}
        }
    }
}

/// Parse a single `<w:style>` element into a `StyleDefinition`.
///
/// Returns `None` if the element lacks a `w:styleId` or `w:type` attribute.
fn parse_style_element(node: &roxmltree::Node) -> Option<StyleDefinition> {
    let style_id = node.attribute((WORDPROCESSINGML_NAMESPACE, "styleId"))?;

    let type_str = node.attribute((WORDPROCESSINGML_NAMESPACE, "type"))?;
    let style_type = match type_str {
        "paragraph" => StyleType::Paragraph,
        "character" => StyleType::Character,
        "table" => StyleType::Table,
        "numbering" => StyleType::Numbering,
        _ => return None,
    };

    let is_default = node
        .attribute((WORDPROCESSINGML_NAMESPACE, "default"))
        .is_some_and(|v| v == "1" || v == "true");

    let mut name = None;
    let mut based_on = None;
    let mut next_style = None;
    let mut paragraph_properties = ParagraphProperties::default();
    let mut run_properties = RunProperties::default();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "name" => {
                name = get_w_val(&child).map(String::from);
            }
            "basedOn" => {
                based_on = get_w_val(&child).map(String::from);
            }
            "next" => {
                next_style = get_w_val(&child).map(String::from);
            }
            "pPr" => {
                paragraph_properties = parse_paragraph_properties(&child);
            }
            "rPr" => {
                run_properties = parse_run_properties(&child);
            }
            _ => {}
        }
    }

    Some(StyleDefinition {
        id: style_id.to_string(),
        name,
        style_type,
        based_on,
        next_style,
        is_default,
        paragraph_properties,
        run_properties,
    })
}

/// Parse `<w:rPr>` run properties from a node's children.
fn parse_run_properties(node: &roxmltree::Node) -> RunProperties {
    let mut props = RunProperties::default();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "b" => props.bold = Some(parse_toggle_property(&child)),
            "i" => props.italic = Some(parse_toggle_property(&child)),
            "u" => {
                // <w:u/> or <w:u w:val="single"/> means underlined;
                // <w:u w:val="none"/> means not underlined.
                let val = get_w_val(&child);
                props.underline = Some(!matches!(val, Some("none")));
            }
            "strike" | "dstrike" => props.strikethrough = Some(parse_toggle_property(&child)),
            "color" => {
                props.color = get_w_val(&child).map(String::from);
                props.theme_color = get_w_attr_string(&child, "themeColor");
                props.theme_tint = get_w_attr_string(&child, "themeTint");
                props.theme_shade = get_w_attr_string(&child, "themeShade");
            }
            "sz" => {
                props.font_size_half_points = get_w_val(&child).and_then(|v| v.parse::<i32>().ok());
            }
            "rFonts" => {
                props.font_ascii = child.attribute((WORDPROCESSINGML_NAMESPACE, "ascii")).map(String::from);
                props.font_ascii_theme = child
                    .attribute((WORDPROCESSINGML_NAMESPACE, "asciiTheme"))
                    .map(String::from);
                props.font_h_ansi = child.attribute((WORDPROCESSINGML_NAMESPACE, "hAnsi")).map(String::from);
                props.font_cs = child.attribute((WORDPROCESSINGML_NAMESPACE, "cs")).map(String::from);
                props.font_east_asia = child
                    .attribute((WORDPROCESSINGML_NAMESPACE, "eastAsia"))
                    .map(String::from);
            }
            "vertAlign" => {
                props.vert_align = get_w_val(&child).map(String::from);
            }
            "highlight" => {
                props.highlight = get_w_val(&child).map(String::from);
            }
            "caps" => {
                props.caps = Some(parse_toggle_property(&child));
            }
            "smallCaps" => {
                props.small_caps = Some(parse_toggle_property(&child));
            }
            "shadow" => {
                props.shadow = Some(parse_toggle_property(&child));
            }
            "outline" => {
                props.outline = Some(parse_toggle_property(&child));
            }
            "emboss" => {
                props.emboss = Some(parse_toggle_property(&child));
            }
            "imprint" => {
                props.imprint = Some(parse_toggle_property(&child));
            }
            "spacing" => {
                props.char_spacing = get_w_val(&child).and_then(|v| v.parse::<i32>().ok());
            }
            "position" => {
                props.position = get_w_val(&child).and_then(|v| v.parse::<i32>().ok());
            }
            "kern" => {
                props.kern = get_w_val(&child).and_then(|v| v.parse::<i32>().ok());
            }
            _ => {}
        }
    }

    props
}

/// Parse `<w:pPr>` paragraph properties from a node's children.
fn parse_paragraph_properties(node: &roxmltree::Node) -> ParagraphProperties {
    let mut props = ParagraphProperties::default();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "jc" => {
                props.alignment = get_w_val(&child).map(String::from);
            }
            "spacing" => {
                props.spacing_before = get_w_attr_i32(&child, "before");
                props.spacing_after = get_w_attr_i32(&child, "after");
                props.spacing_line = get_w_attr_i32(&child, "line");
                props.spacing_line_rule = get_w_attr_string(&child, "lineRule");
            }
            "ind" => {
                props.indent_left = get_w_attr_i32(&child, "left");
                props.indent_right = get_w_attr_i32(&child, "right");
                props.indent_first_line = get_w_attr_i32(&child, "firstLine");
                props.indent_hanging = get_w_attr_i32(&child, "hanging");
            }
            "outlineLvl" => {
                props.outline_level = get_w_val(&child).and_then(|v| v.parse::<u8>().ok());
            }
            "keepNext" => {
                props.keep_next = Some(parse_toggle_property(&child));
            }
            "keepLines" => {
                props.keep_lines = Some(parse_toggle_property(&child));
            }
            "pageBreakBefore" => {
                props.page_break_before = Some(parse_toggle_property(&child));
            }
            "widowControl" => {
                props.widow_control = Some(parse_toggle_property(&child));
            }
            "suppressAutoHyphens" => {
                props.suppress_auto_hyphens = Some(parse_toggle_property(&child));
            }
            "bidi" => {
                props.bidi = Some(parse_toggle_property(&child));
            }
            "shd" => {
                props.shading_fill = get_w_attr_string(&child, "fill");
                props.shading_val = get_w_attr_string(&child, "val");
            }
            "pBdr" => {
                for border_child in child.children() {
                    if !border_child.is_element() {
                        continue;
                    }
                    match border_child.tag_name().name() {
                        "top" => props.border_top = get_w_val(&border_child).map(String::from),
                        "bottom" => props.border_bottom = get_w_val(&border_child).map(String::from),
                        "left" | "start" => props.border_left = get_w_val(&border_child).map(String::from),
                        "right" | "end" => props.border_right = get_w_val(&border_child).map(String::from),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    props
}

// --- Style Resolution ---

impl StyleCatalog {
    /// Resolve a style by walking its `basedOn` inheritance chain.
    ///
    /// The resolution order is:
    /// 1. Document defaults (`<w:docDefaults>`)
    /// 2. Base style chain (walking `basedOn` from root to leaf)
    /// 3. The style itself
    ///
    /// For `Option` fields, a child value of `Some(x)` overrides the parent.
    /// A value of `None` inherits from the parent. For boolean toggle properties,
    /// `Some(false)` explicitly disables the property.
    ///
    /// The chain depth is limited to 20 to prevent infinite loops from circular references.
    #[cfg(test)]
    pub(crate) fn resolve_style(&self, style_id: &str) -> ResolvedStyle {
        // Start with document defaults
        let mut resolved = ResolvedStyle {
            paragraph_properties: self.default_paragraph_properties.clone(),
            run_properties: self.default_run_properties.clone(),
        };

        // Collect the inheritance chain (from root ancestor to the style itself)
        let chain = self.collect_chain(style_id);

        // Apply each style in order (root ancestor first, target style last)
        for style_def in &chain {
            merge_paragraph_properties(&mut resolved.paragraph_properties, &style_def.paragraph_properties);
            merge_run_properties(&mut resolved.run_properties, &style_def.run_properties);
        }

        resolved
    }

    /// Collect the basedOn chain for a style, ordered from root ancestor to the style itself.
    ///
    /// Limited to 20 levels to prevent cycles.
    #[cfg(test)]
    fn collect_chain(&self, style_id: &str) -> Vec<&StyleDefinition> {
        const MAX_DEPTH: usize = 20;

        let mut chain = Vec::new();
        let mut current_id = Some(style_id.to_string());
        let mut visited = Vec::new();

        while let Some(id) = current_id {
            if visited.len() >= MAX_DEPTH {
                break;
            }
            if visited.contains(&id) {
                // Cycle detected
                break;
            }

            if let Some(style_def) = self.styles.get(&id) {
                visited.push(id);
                chain.push(style_def);
                current_id = style_def.based_on.clone();
            } else {
                break;
            }
        }

        // Reverse so root ancestor is first and the target style is last
        chain.reverse();
        chain
    }
}

// --- Merge helpers ---

/// Merge child run properties onto parent, where `Some` in child overrides parent.
#[cfg(test)]
fn merge_run_properties(base: &mut RunProperties, overlay: &RunProperties) {
    if overlay.bold.is_some() {
        base.bold = overlay.bold;
    }
    if overlay.italic.is_some() {
        base.italic = overlay.italic;
    }
    if overlay.underline.is_some() {
        base.underline = overlay.underline;
    }
    if overlay.strikethrough.is_some() {
        base.strikethrough = overlay.strikethrough;
    }
    if overlay.color.is_some() {
        base.color.clone_from(&overlay.color);
    }
    if overlay.font_size_half_points.is_some() {
        base.font_size_half_points = overlay.font_size_half_points;
    }
    if overlay.font_ascii.is_some() {
        base.font_ascii.clone_from(&overlay.font_ascii);
    }
    if overlay.font_ascii_theme.is_some() {
        base.font_ascii_theme.clone_from(&overlay.font_ascii_theme);
    }
    if overlay.vert_align.is_some() {
        base.vert_align.clone_from(&overlay.vert_align);
    }
    if overlay.font_h_ansi.is_some() {
        base.font_h_ansi.clone_from(&overlay.font_h_ansi);
    }
    if overlay.font_cs.is_some() {
        base.font_cs.clone_from(&overlay.font_cs);
    }
    if overlay.font_east_asia.is_some() {
        base.font_east_asia.clone_from(&overlay.font_east_asia);
    }
    if overlay.highlight.is_some() {
        base.highlight.clone_from(&overlay.highlight);
    }
    if overlay.caps.is_some() {
        base.caps = overlay.caps;
    }
    if overlay.small_caps.is_some() {
        base.small_caps = overlay.small_caps;
    }
    if overlay.shadow.is_some() {
        base.shadow = overlay.shadow;
    }
    if overlay.outline.is_some() {
        base.outline = overlay.outline;
    }
    if overlay.emboss.is_some() {
        base.emboss = overlay.emboss;
    }
    if overlay.imprint.is_some() {
        base.imprint = overlay.imprint;
    }
    if overlay.char_spacing.is_some() {
        base.char_spacing = overlay.char_spacing;
    }
    if overlay.position.is_some() {
        base.position = overlay.position;
    }
    if overlay.kern.is_some() {
        base.kern = overlay.kern;
    }
    // Theme color, tint, and shade form an atomic group: if theme_color changes,
    // tint/shade from the parent must not leak through.
    if overlay.theme_color.is_some() {
        base.theme_color.clone_from(&overlay.theme_color);
        base.theme_tint = overlay.theme_tint.clone();
        base.theme_shade = overlay.theme_shade.clone();
    } else {
        if overlay.theme_tint.is_some() {
            base.theme_tint.clone_from(&overlay.theme_tint);
        }
        if overlay.theme_shade.is_some() {
            base.theme_shade.clone_from(&overlay.theme_shade);
        }
    }
}

/// Merge child paragraph properties onto parent, where `Some` in child overrides parent.
#[cfg(test)]
fn merge_paragraph_properties(base: &mut ParagraphProperties, overlay: &ParagraphProperties) {
    if overlay.alignment.is_some() {
        base.alignment.clone_from(&overlay.alignment);
    }
    if overlay.spacing_before.is_some() {
        base.spacing_before = overlay.spacing_before;
    }
    if overlay.spacing_after.is_some() {
        base.spacing_after = overlay.spacing_after;
    }
    if overlay.spacing_line.is_some() {
        base.spacing_line = overlay.spacing_line;
    }
    if overlay.spacing_line_rule.is_some() {
        base.spacing_line_rule.clone_from(&overlay.spacing_line_rule);
    }
    if overlay.indent_left.is_some() {
        base.indent_left = overlay.indent_left;
    }
    if overlay.indent_right.is_some() {
        base.indent_right = overlay.indent_right;
    }
    if overlay.indent_first_line.is_some() {
        base.indent_first_line = overlay.indent_first_line;
    }
    if overlay.indent_hanging.is_some() {
        base.indent_hanging = overlay.indent_hanging;
    }
    if overlay.outline_level.is_some() {
        base.outline_level = overlay.outline_level;
    }
    if overlay.keep_next.is_some() {
        base.keep_next = overlay.keep_next;
    }
    if overlay.keep_lines.is_some() {
        base.keep_lines = overlay.keep_lines;
    }
    if overlay.page_break_before.is_some() {
        base.page_break_before = overlay.page_break_before;
    }
    if overlay.widow_control.is_some() {
        base.widow_control = overlay.widow_control;
    }
    if overlay.suppress_auto_hyphens.is_some() {
        base.suppress_auto_hyphens = overlay.suppress_auto_hyphens;
    }
    if overlay.bidi.is_some() {
        base.bidi = overlay.bidi;
    }
    if overlay.shading_fill.is_some() {
        base.shading_fill.clone_from(&overlay.shading_fill);
    }
    if overlay.shading_val.is_some() {
        base.shading_val.clone_from(&overlay.shading_val);
    }
    if overlay.border_top.is_some() {
        base.border_top.clone_from(&overlay.border_top);
    }
    if overlay.border_bottom.is_some() {
        base.border_bottom.clone_from(&overlay.border_bottom);
    }
    if overlay.border_left.is_some() {
        base.border_left.clone_from(&overlay.border_left);
    }
    if overlay.border_right.is_some() {
        base.border_right.clone_from(&overlay.border_right);
    }
}

// --- XML helpers ---

/// Get the `w:val` attribute from a node.
///
/// Tries namespaced attribute first, then falls back to unqualified `val`.
fn get_w_val<'a>(node: &'a roxmltree::Node) -> Option<&'a str> {
    node.attribute((WORDPROCESSINGML_NAMESPACE, "val"))
        .or_else(|| node.attribute("val"))
}

/// Get a namespaced integer attribute from a node (e.g., `w:before`, `w:left`).
fn get_w_attr_i32(node: &roxmltree::Node, local_name: &str) -> Option<i32> {
    node.attribute((WORDPROCESSINGML_NAMESPACE, local_name))
        .or_else(|| node.attribute(local_name))
        .and_then(|v| v.parse::<i32>().ok())
}

/// Get a namespaced string attribute from a node.
fn get_w_attr_string(node: &roxmltree::Node, local_name: &str) -> Option<String> {
    node.attribute((WORDPROCESSINGML_NAMESPACE, local_name))
        .or_else(|| node.attribute(local_name))
        .map(String::from)
}

/// Find the first child element with the given local name.
fn find_child_element<'a>(node: &'a roxmltree::Node, local_name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children()
        .find(|c| c.is_element() && c.tag_name().name() == local_name)
}

/// Parse a toggle (boolean) property element.
///
/// - `<w:b/>` (no val attribute) -> `true`
/// - `<w:b w:val="1"/>` or `<w:b w:val="true"/>` -> `true`
/// - `<w:b w:val="0"/>` or `<w:b w:val="false"/>` -> `false`
fn parse_toggle_property(node: &roxmltree::Node) -> bool {
    match get_w_val(node) {
        None => true, // bare element like <w:b/> means enabled
        Some(val) => !matches!(val, "0" | "false"),
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    /// Namespace prefix for building test XML.
    const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn make_styles_xml(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{W_NS}">
{body}
</w:styles>"#
        )
    }

    #[test]
    fn test_parse_empty_styles() {
        let xml = make_styles_xml("");
        let catalog = parse_styles_xml(&xml).expect("should parse empty styles");
        assert!(catalog.styles.is_empty());
        assert_eq!(catalog.default_run_properties, RunProperties::default());
        assert_eq!(catalog.default_paragraph_properties, ParagraphProperties::default());
    }

    #[test]
    fn test_parse_doc_defaults() {
        let xml = make_styles_xml(
            r#"
            <w:docDefaults>
                <w:rPrDefault>
                    <w:rPr>
                        <w:sz w:val="24"/>
                        <w:rFonts w:ascii="Calibri" w:asciiTheme="minorHAnsi"/>
                    </w:rPr>
                </w:rPrDefault>
                <w:pPrDefault>
                    <w:pPr>
                        <w:spacing w:after="160" w:line="259"/>
                    </w:pPr>
                </w:pPrDefault>
            </w:docDefaults>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse doc defaults");

        assert_eq!(catalog.default_run_properties.font_size_half_points, Some(24));
        assert_eq!(catalog.default_run_properties.font_ascii.as_deref(), Some("Calibri"));
        assert_eq!(
            catalog.default_run_properties.font_ascii_theme.as_deref(),
            Some("minorHAnsi")
        );
        assert_eq!(catalog.default_paragraph_properties.spacing_after, Some(160));
        assert_eq!(catalog.default_paragraph_properties.spacing_line, Some(259));
    }

    #[test]
    fn test_parse_style_definitions() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
                <w:name w:val="Normal"/>
                <w:pPr>
                    <w:spacing w:after="200"/>
                    <w:jc w:val="left"/>
                </w:pPr>
                <w:rPr>
                    <w:sz w:val="22"/>
                </w:rPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading1">
                <w:name w:val="heading 1"/>
                <w:basedOn w:val="Normal"/>
                <w:next w:val="Normal"/>
                <w:pPr>
                    <w:keepNext/>
                    <w:keepLines/>
                    <w:spacing w:before="240"/>
                    <w:outlineLvl w:val="0"/>
                </w:pPr>
                <w:rPr>
                    <w:b/>
                    <w:color w:val="2F5496"/>
                    <w:sz w:val="32"/>
                    <w:rFonts w:asciiTheme="majorHAnsi"/>
                </w:rPr>
            </w:style>
            <w:style w:type="character" w:styleId="Strong">
                <w:name w:val="Strong"/>
                <w:rPr>
                    <w:b/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse style definitions");

        assert_eq!(catalog.styles.len(), 3);

        // Normal
        let normal = catalog.styles.get("Normal").expect("Normal style must exist");
        assert_eq!(normal.style_type, StyleType::Paragraph);
        assert!(normal.is_default);
        assert_eq!(normal.name.as_deref(), Some("Normal"));
        assert_eq!(normal.paragraph_properties.spacing_after, Some(200));
        assert_eq!(normal.paragraph_properties.alignment.as_deref(), Some("left"));
        assert_eq!(normal.run_properties.font_size_half_points, Some(22));

        // Heading1
        let heading1 = catalog.styles.get("Heading1").expect("Heading1 style must exist");
        assert_eq!(heading1.style_type, StyleType::Paragraph);
        assert!(!heading1.is_default);
        assert_eq!(heading1.name.as_deref(), Some("heading 1"));
        assert_eq!(heading1.based_on.as_deref(), Some("Normal"));
        assert_eq!(heading1.next_style.as_deref(), Some("Normal"));
        assert_eq!(heading1.paragraph_properties.keep_next, Some(true));
        assert_eq!(heading1.paragraph_properties.keep_lines, Some(true));
        assert_eq!(heading1.paragraph_properties.spacing_before, Some(240));
        assert_eq!(heading1.paragraph_properties.outline_level, Some(0));
        assert_eq!(heading1.run_properties.bold, Some(true));
        assert_eq!(heading1.run_properties.color.as_deref(), Some("2F5496"));
        assert_eq!(heading1.run_properties.font_size_half_points, Some(32));
        assert_eq!(heading1.run_properties.font_ascii_theme.as_deref(), Some("majorHAnsi"));

        // Strong (character style)
        let strong = catalog.styles.get("Strong").expect("Strong style must exist");
        assert_eq!(strong.style_type, StyleType::Character);
        assert_eq!(strong.run_properties.bold, Some(true));
    }

    #[test]
    fn test_resolve_style_inheritance() {
        // 3-level chain: docDefaults -> Normal -> Heading1
        let xml = make_styles_xml(
            r#"
            <w:docDefaults>
                <w:rPrDefault>
                    <w:rPr>
                        <w:sz w:val="24"/>
                        <w:rFonts w:ascii="Calibri"/>
                    </w:rPr>
                </w:rPrDefault>
                <w:pPrDefault>
                    <w:pPr>
                        <w:spacing w:after="160"/>
                    </w:pPr>
                </w:pPrDefault>
            </w:docDefaults>
            <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
                <w:name w:val="Normal"/>
                <w:pPr>
                    <w:spacing w:after="200"/>
                    <w:jc w:val="left"/>
                </w:pPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading1">
                <w:name w:val="heading 1"/>
                <w:basedOn w:val="Normal"/>
                <w:pPr>
                    <w:keepNext/>
                    <w:spacing w:before="240"/>
                    <w:outlineLvl w:val="0"/>
                    <w:jc w:val="center"/>
                </w:pPr>
                <w:rPr>
                    <w:b/>
                    <w:sz w:val="32"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("Heading1");

        // Run properties: docDefaults font_ascii=Calibri, sz=24; Heading1 overrides sz=32, adds bold
        assert_eq!(resolved.run_properties.font_ascii.as_deref(), Some("Calibri"));
        assert_eq!(resolved.run_properties.font_size_half_points, Some(32));
        assert_eq!(resolved.run_properties.bold, Some(true));

        // Paragraph properties: docDefaults spacing_after=160; Normal overrides to 200 and sets jc=left;
        // Heading1 overrides jc=center, adds spacing_before=240, keep_next=true, outline_level=0
        assert_eq!(resolved.paragraph_properties.spacing_after, Some(200));
        assert_eq!(resolved.paragraph_properties.alignment.as_deref(), Some("center"));
        assert_eq!(resolved.paragraph_properties.spacing_before, Some(240));
        assert_eq!(resolved.paragraph_properties.keep_next, Some(true));
        assert_eq!(resolved.paragraph_properties.outline_level, Some(0));
    }

    #[test]
    fn test_resolve_style_toggle() {
        // basedOn has bold=true, derived explicitly disables bold with w:val="0"
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="BoldBase">
                <w:name w:val="Bold Base"/>
                <w:rPr>
                    <w:b/>
                    <w:i/>
                </w:rPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="NoBold">
                <w:name w:val="No Bold"/>
                <w:basedOn w:val="BoldBase"/>
                <w:rPr>
                    <w:b w:val="0"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("NoBold");

        // Bold should be explicitly false (overridden)
        assert_eq!(resolved.run_properties.bold, Some(false));
        // Italic should be inherited from BoldBase
        assert_eq!(resolved.run_properties.italic, Some(true));
    }

    #[test]
    fn test_resolve_style_cycle_protection() {
        // Create a circular basedOn chain: A -> B -> A
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="StyleA">
                <w:name w:val="Style A"/>
                <w:basedOn w:val="StyleB"/>
                <w:rPr>
                    <w:b/>
                </w:rPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="StyleB">
                <w:name w:val="Style B"/>
                <w:basedOn w:val="StyleA"/>
                <w:rPr>
                    <w:i/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        // Must not panic or infinite loop
        let resolved = catalog.resolve_style("StyleA");

        // Both bold and italic should be present (the cycle is broken, and
        // both styles in the chain contribute their properties)
        assert_eq!(resolved.run_properties.bold, Some(true));
        assert_eq!(resolved.run_properties.italic, Some(true));
    }

    #[test]
    fn test_resolve_nonexistent_style() {
        let xml = make_styles_xml("");
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("DoesNotExist");

        // Should return empty defaults without panicking
        assert_eq!(resolved, ResolvedStyle::default());
    }

    #[test]
    fn test_resolve_style_with_missing_base() {
        // Style references a basedOn that doesn't exist in the catalog
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="Orphan">
                <w:name w:val="Orphan"/>
                <w:basedOn w:val="NonexistentBase"/>
                <w:rPr>
                    <w:b/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("Orphan");

        // Should resolve with just the style's own properties
        assert_eq!(resolved.run_properties.bold, Some(true));
    }

    #[test]
    fn test_parse_indentation() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="Indented">
                <w:name w:val="Indented"/>
                <w:pPr>
                    <w:ind w:left="720" w:right="360" w:firstLine="360" w:hanging="180"/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("Indented").expect("style must exist");
        assert_eq!(style.paragraph_properties.indent_left, Some(720));
        assert_eq!(style.paragraph_properties.indent_right, Some(360));
        assert_eq!(style.paragraph_properties.indent_first_line, Some(360));
        assert_eq!(style.paragraph_properties.indent_hanging, Some(180));
    }

    #[test]
    fn test_parse_underline_none() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="NoUnderline">
                <w:name w:val="No Underline"/>
                <w:rPr>
                    <w:u w:val="none"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("NoUnderline").expect("style must exist");
        assert_eq!(style.run_properties.underline, Some(false));
    }

    #[test]
    fn test_parse_underline_single() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="Underlined">
                <w:name w:val="Underlined"/>
                <w:rPr>
                    <w:u w:val="single"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("Underlined").expect("style must exist");
        assert_eq!(style.run_properties.underline, Some(true));
    }

    #[test]
    fn test_parse_underline_bare() {
        // Test for bare <w:u/> (no val attribute) -> underline should be Some(true)
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="BareUnderline">
                <w:name w:val="Bare Underline"/>
                <w:rPr>
                    <w:u/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("BareUnderline").expect("style must exist");
        assert_eq!(style.run_properties.underline, Some(true));
    }

    #[test]
    fn test_parse_bold_explicit_true() {
        // Test for <w:b w:val="true"/> -> bold should be Some(true)
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="ExplicitBold">
                <w:name w:val="Explicit Bold"/>
                <w:rPr>
                    <w:b w:val="true"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("ExplicitBold").expect("style must exist");
        assert_eq!(style.run_properties.bold, Some(true));
    }

    #[test]
    fn test_parse_invalid_xml() {
        let result = parse_styles_xml("<<<not valid xml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_spacing_line_rule() {
        // Test for spacing_line_rule parsing
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="SpacingRule">
                <w:name w:val="Spacing Rule"/>
                <w:pPr>
                    <w:spacing w:line="360" w:lineRule="exact"/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("SpacingRule").expect("style must exist");
        assert_eq!(style.paragraph_properties.spacing_line, Some(360));
        assert_eq!(style.paragraph_properties.spacing_line_rule.as_deref(), Some("exact"));
    }

    #[test]
    fn test_parse_latent_styles_skipped() {
        let xml = make_styles_xml(
            r#"
            <w:latentStyles w:defLockedState="0" w:defUIPriority="99">
                <w:lsdException w:name="Normal" w:uiPriority="0" w:qFormat="1"/>
            </w:latentStyles>
            <w:style w:type="paragraph" w:styleId="Normal">
                <w:name w:val="Normal"/>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        // latentStyles should be skipped, only the actual style parsed
        assert_eq!(catalog.styles.len(), 1);
        assert!(catalog.styles.contains_key("Normal"));
    }

    #[test]
    fn test_parse_vert_align() {
        // Test for vert_align parsing
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="Superscript">
                <w:name w:val="Superscript"/>
                <w:rPr>
                    <w:vertAlign w:val="superscript"/>
                </w:rPr>
            </w:style>
            <w:style w:type="character" w:styleId="Subscript">
                <w:name w:val="Subscript"/>
                <w:rPr>
                    <w:vertAlign w:val="subscript"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let super_style = catalog.styles.get("Superscript").expect("style must exist");
        assert_eq!(super_style.run_properties.vert_align.as_deref(), Some("superscript"));

        let sub_style = catalog.styles.get("Subscript").expect("style must exist");
        assert_eq!(sub_style.run_properties.vert_align.as_deref(), Some("subscript"));
    }

    #[test]
    fn test_parse_multilingual_fonts() {
        // Test for multilingual rFonts parsing
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="MultiLang">
                <w:name w:val="MultiLang"/>
                <w:rPr>
                    <w:rFonts w:ascii="Calibri" w:hAnsi="Arial" w:cs="Courier" w:eastAsia="SimSun"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("MultiLang").expect("style must exist");
        assert_eq!(style.run_properties.font_ascii.as_deref(), Some("Calibri"));
        assert_eq!(style.run_properties.font_h_ansi.as_deref(), Some("Arial"));
        assert_eq!(style.run_properties.font_cs.as_deref(), Some("Courier"));
        assert_eq!(style.run_properties.font_east_asia.as_deref(), Some("SimSun"));
    }

    #[test]
    fn test_resolve_character_style() {
        // Test for character style resolution
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="BaseChar">
                <w:name w:val="Base Char"/>
                <w:rPr>
                    <w:b/>
                    <w:sz w:val="24"/>
                </w:rPr>
            </w:style>
            <w:style w:type="character" w:styleId="DerivedChar">
                <w:name w:val="Derived Char"/>
                <w:basedOn w:val="BaseChar"/>
                <w:rPr>
                    <w:i/>
                    <w:color w:val="FF0000"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("DerivedChar");

        // Should inherit bold and size from BaseChar, add italic and color
        assert_eq!(resolved.run_properties.bold, Some(true));
        assert_eq!(resolved.run_properties.font_size_half_points, Some(24));
        assert_eq!(resolved.run_properties.italic, Some(true));
        assert_eq!(resolved.run_properties.color.as_deref(), Some("FF0000"));
    }

    #[test]
    fn test_deep_inheritance_chain() {
        // Build a 5-level chain: Level0 -> Level1 -> Level2 -> Level3 -> Level4
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="Level0">
                <w:name w:val="Level 0"/>
                <w:rPr>
                    <w:sz w:val="20"/>
                </w:rPr>
                <w:pPr>
                    <w:spacing w:after="100"/>
                </w:pPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Level1">
                <w:name w:val="Level 1"/>
                <w:basedOn w:val="Level0"/>
                <w:rPr>
                    <w:b/>
                </w:rPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Level2">
                <w:name w:val="Level 2"/>
                <w:basedOn w:val="Level1"/>
                <w:rPr>
                    <w:i/>
                </w:rPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Level3">
                <w:name w:val="Level 3"/>
                <w:basedOn w:val="Level2"/>
                <w:pPr>
                    <w:jc w:val="center"/>
                </w:pPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Level4">
                <w:name w:val="Level 4"/>
                <w:basedOn w:val="Level3"/>
                <w:rPr>
                    <w:sz w:val="40"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("Level4");

        // sz: Level0=20, overridden by Level4=40
        assert_eq!(resolved.run_properties.font_size_half_points, Some(40));
        // bold: from Level1
        assert_eq!(resolved.run_properties.bold, Some(true));
        // italic: from Level2
        assert_eq!(resolved.run_properties.italic, Some(true));
        // alignment: from Level3
        assert_eq!(resolved.paragraph_properties.alignment.as_deref(), Some("center"));
        // spacing_after: from Level0 (not overridden)
        assert_eq!(resolved.paragraph_properties.spacing_after, Some(100));
    }

    #[test]
    fn test_parse_page_break_before() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="PageBreakStyle">
                <w:name w:val="Page Break Style"/>
                <w:pPr>
                    <w:pageBreakBefore/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("PageBreakStyle").expect("style must exist");
        assert_eq!(style.paragraph_properties.page_break_before, Some(true));
    }

    #[test]
    fn test_parse_widow_control() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="WidowControl">
                <w:name w:val="Widow Control"/>
                <w:pPr>
                    <w:widowControl w:val="0"/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("WidowControl").expect("style must exist");
        assert_eq!(style.paragraph_properties.widow_control, Some(false));
    }

    #[test]
    fn test_parse_bidi() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="RightToLeft">
                <w:name w:val="Right to Left"/>
                <w:pPr>
                    <w:bidi/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("RightToLeft").expect("style must exist");
        assert_eq!(style.paragraph_properties.bidi, Some(true));
    }

    #[test]
    fn test_parse_shading() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="ShadedPara">
                <w:name w:val="Shaded Paragraph"/>
                <w:pPr>
                    <w:shd w:val="clear" w:fill="FFFF00"/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("ShadedPara").expect("style must exist");
        assert_eq!(style.paragraph_properties.shading_val.as_deref(), Some("clear"));
        assert_eq!(style.paragraph_properties.shading_fill.as_deref(), Some("FFFF00"));
    }

    #[test]
    fn test_parse_paragraph_borders() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="BorderedPara">
                <w:name w:val="Bordered Paragraph"/>
                <w:pPr>
                    <w:pBdr>
                        <w:top w:val="single"/>
                        <w:bottom w:val="double"/>
                        <w:left w:val="triple"/>
                        <w:right w:val="dashed"/>
                    </w:pBdr>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("BorderedPara").expect("style must exist");
        assert_eq!(style.paragraph_properties.border_top.as_deref(), Some("single"));
        assert_eq!(style.paragraph_properties.border_bottom.as_deref(), Some("double"));
        assert_eq!(style.paragraph_properties.border_left.as_deref(), Some("triple"));
        assert_eq!(style.paragraph_properties.border_right.as_deref(), Some("dashed"));
    }

    #[test]
    fn test_paragraph_properties_inheritance() {
        // Test that page_break_before inherits through basedOn chain
        let xml = make_styles_xml(
            r#"
            <w:style w:type="paragraph" w:styleId="BaseStyle">
                <w:name w:val="Base Style"/>
                <w:pPr>
                    <w:pageBreakBefore/>
                    <w:spacing w:after="100"/>
                </w:pPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="DerivedStyle">
                <w:name w:val="Derived Style"/>
                <w:basedOn w:val="BaseStyle"/>
                <w:pPr>
                    <w:spacing w:before="50"/>
                </w:pPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("DerivedStyle");

        // page_break_before should inherit from BaseStyle
        assert_eq!(resolved.paragraph_properties.page_break_before, Some(true));
        // spacing_before should come from DerivedStyle
        assert_eq!(resolved.paragraph_properties.spacing_before, Some(50));
        // spacing_after should inherit from BaseStyle
        assert_eq!(resolved.paragraph_properties.spacing_after, Some(100));
    }

    #[test]
    fn test_parse_highlight() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="HighlightStyle">
                <w:name w:val="Highlight Style"/>
                <w:rPr>
                    <w:highlight w:val="yellow"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("HighlightStyle").expect("style must exist");
        assert_eq!(style.run_properties.highlight.as_deref(), Some("yellow"));
    }

    #[test]
    fn test_parse_caps_and_small_caps() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="CapsStyle">
                <w:name w:val="Caps Style"/>
                <w:rPr>
                    <w:caps/>
                    <w:smallCaps/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("CapsStyle").expect("style must exist");
        assert_eq!(style.run_properties.caps, Some(true));
        assert_eq!(style.run_properties.small_caps, Some(true));
    }

    #[test]
    fn test_parse_text_effects() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="EffectsStyle">
                <w:name w:val="Effects Style"/>
                <w:rPr>
                    <w:shadow/>
                    <w:outline/>
                    <w:emboss/>
                    <w:imprint/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("EffectsStyle").expect("style must exist");
        assert_eq!(style.run_properties.shadow, Some(true));
        assert_eq!(style.run_properties.outline, Some(true));
        assert_eq!(style.run_properties.emboss, Some(true));
        assert_eq!(style.run_properties.imprint, Some(true));
    }

    #[test]
    fn test_parse_char_spacing() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="SpacingStyle">
                <w:name w:val="Spacing Style"/>
                <w:rPr>
                    <w:spacing w:val="20"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("SpacingStyle").expect("style must exist");
        assert_eq!(style.run_properties.char_spacing, Some(20));
    }

    #[test]
    fn test_parse_theme_color() {
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="ThemeColorStyle">
                <w:name w:val="Theme Color Style"/>
                <w:rPr>
                    <w:color w:val="2F5496" w:themeColor="accent1" w:themeTint="BF" w:themeShade="4D"/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let style = catalog.styles.get("ThemeColorStyle").expect("style must exist");
        assert_eq!(style.run_properties.color.as_deref(), Some("2F5496"));
        assert_eq!(style.run_properties.theme_color.as_deref(), Some("accent1"));
        assert_eq!(style.run_properties.theme_tint.as_deref(), Some("BF"));
        assert_eq!(style.run_properties.theme_shade.as_deref(), Some("4D"));
    }

    #[test]
    fn test_run_properties_inheritance_with_effects() {
        // Test that caps/shadow inherit through basedOn chain
        let xml = make_styles_xml(
            r#"
            <w:style w:type="character" w:styleId="BaseCharStyle">
                <w:name w:val="Base Char"/>
                <w:rPr>
                    <w:b/>
                    <w:caps/>
                </w:rPr>
            </w:style>
            <w:style w:type="character" w:styleId="DerivedCharStyle">
                <w:name w:val="Derived Char"/>
                <w:basedOn w:val="BaseCharStyle"/>
                <w:rPr>
                    <w:i/>
                    <w:shadow/>
                </w:rPr>
            </w:style>
            "#,
        );
        let catalog = parse_styles_xml(&xml).expect("should parse");

        let resolved = catalog.resolve_style("DerivedCharStyle");

        // bold and caps should inherit from BaseCharStyle
        assert_eq!(resolved.run_properties.bold, Some(true));
        assert_eq!(resolved.run_properties.caps, Some(true));
        // italic and shadow should come from DerivedCharStyle
        assert_eq!(resolved.run_properties.italic, Some(true));
        assert_eq!(resolved.run_properties.shadow, Some(true));
    }
}
