//! XML parsing for PPTX slide content.
//!
//! This module handles parsing slide XML, extracting text, tables, lists, images,
//! and relationships from PowerPoint presentations.

use roxmltree::{Document, Node};

use crate::error::{KreuzbergError, Result};
use crate::text::utf8_validation;

use super::elements::{
    ElementPosition, Formatting, ImageReference, ListElement, ListItem, ParsedContent, Run, SlideElement, TableCell,
    TableElement, TableRow, TextElement,
};

use crate::extraction::ooxml_constants::{DRAWINGML_NAMESPACE, PRESENTATIONML_NAMESPACE, RELATIONSHIPS_NAMESPACE};

pub(super) fn parse_slide_xml(xml_data: &[u8]) -> Result<Vec<SlideElement>> {
    let xml_str = utf8_validation::from_utf8(xml_data)
        .map_err(|_| KreuzbergError::parsing("Invalid UTF-8 in slide XML".to_string()))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse slide XML: {}", e)))?;

    let root = doc.root_element();
    let ns = root.tag_name().namespace();

    let c_sld = root
        .descendants()
        .find(|n| n.tag_name().name() == "cSld" && n.tag_name().namespace() == ns)
        .ok_or_else(|| KreuzbergError::parsing("No <p:cSld> tag found".to_string()))?;

    let sp_tree = c_sld
        .children()
        .find(|n| n.tag_name().name() == "spTree" && n.tag_name().namespace() == ns)
        .ok_or_else(|| KreuzbergError::parsing("No <p:spTree> tag found".to_string()))?;

    let mut elements = Vec::new();
    for child_node in sp_tree.children().filter(|n| n.is_element()) {
        elements.extend(parse_group(&child_node)?);
    }

    Ok(elements)
}

fn parse_group(node: &Node) -> Result<Vec<SlideElement>> {
    let mut elements = Vec::new();

    let tag_name = node.tag_name().name();
    let namespace = node.tag_name().namespace().unwrap_or("");

    if namespace != PRESENTATIONML_NAMESPACE {
        return Ok(elements);
    }

    let position = extract_position(node);

    match tag_name {
        "sp" => {
            let position = extract_position(node);
            for content in parse_sp(node)? {
                match content {
                    ParsedContent::Text(text) => elements.push(SlideElement::Text(text, position)),
                    ParsedContent::List(list) => elements.push(SlideElement::List(list, position)),
                }
            }
        }
        "graphicFrame" => {
            match parse_graphic_frame(node)? {
                GraphicFrameResult::Table(table) => {
                    elements.push(SlideElement::Table(table, position));
                }
                GraphicFrameResult::Graphic(uri) => {
                    // Emit a placeholder text element for non-table graphics (charts, diagrams, etc.)
                    let placeholder = format!("\\[Graphic: other: {}\\]", uri);
                    let text_el = TextElement {
                        runs: vec![Run {
                            text: placeholder,
                            formatting: Formatting::default(),
                            hyperlink_id: None,
                        }],
                        is_title: false,
                    };
                    elements.push(SlideElement::Text(text_el, position));
                }
                GraphicFrameResult::None => {}
            }
        }
        "pic" => {
            let image_reference = parse_pic(node)?;
            elements.push(SlideElement::Image(image_reference, position));
        }
        "grpSp" => {
            for child in node.children().filter(|n| n.is_element()) {
                elements.extend(parse_group(&child)?);
            }
        }
        _ => elements.push(SlideElement::Unknown),
    }

    Ok(elements)
}

/// Check whether a shape node contains a title placeholder.
///
/// OOXML placeholder types for titles:
/// - `type="title"` (general title, idx 0 in most slide layouts)
/// - `type="ctrTitle"` (centered title, used on title slides)
fn is_title_placeholder(sp_node: &Node) -> bool {
    // Path: p:sp / p:nvSpPr / p:nvPr / p:ph[@type]
    let nv_sp_pr = sp_node
        .children()
        .find(|n| n.tag_name().name() == "nvSpPr" && n.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE));
    if let Some(nv_sp_pr) = nv_sp_pr {
        let nv_pr = nv_sp_pr
            .children()
            .find(|n| n.tag_name().name() == "nvPr" && n.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE));
        if let Some(nv_pr) = nv_pr {
            let ph = nv_pr
                .children()
                .find(|n| n.tag_name().name() == "ph" && n.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE));
            if let Some(ph) = ph {
                return matches!(ph.attribute("type"), Some("title") | Some("ctrTitle"));
            }
        }
    }
    false
}

/// Parse a shape element, returning zero or more content items.
///
/// Mixed shapes (containing both plain text paragraphs and bulleted list
/// paragraphs) are split into separate `Text` and `List` items so that the
/// output matches tools like Pandoc which render them as distinct blocks.
fn parse_sp(sp_node: &Node) -> Result<Vec<ParsedContent>> {
    // Some shapes like image placeholders (<p:ph type="pic"/>) don't have txBody.
    // These should be skipped gracefully - they contain no text to extract.
    // GitHub Issue #321 Bug 1
    let tx_body_node = match sp_node
        .children()
        .find(|n| n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE))
    {
        Some(node) => node,
        None => return Ok(Vec::new()), // Skip shapes without txBody
    };

    let is_title = is_title_placeholder(sp_node);

    // Title placeholders are always treated as a single text element.
    if is_title {
        let mut text_el = parse_text(&tx_body_node)?;
        text_el.is_title = true;
        return Ok(vec![ParsedContent::Text(text_el)]);
    }

    // Inspect each paragraph to classify it as list or text. Adjacent
    // paragraphs of the same kind are grouped together so that a shape
    // with mixed content produces multiple content items (e.g. a plain
    // paragraph followed by a bullet list).
    let paragraphs: Vec<_> = tx_body_node
        .children()
        .filter(|n| {
            n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        })
        .collect();

    // Fast path: check if all paragraphs are uniformly text or list.
    let has_any_bullet = paragraphs.iter().any(|p| paragraph_has_bullet(p));
    let all_have_bullet = has_any_bullet
        && paragraphs.iter().all(|p| {
            // An empty paragraph is neutral
            let has_runs = p.children().any(|n| {
                n.is_element() && n.tag_name().name() == "r" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
            });
            !has_runs || paragraph_has_bullet(p)
        });

    if !has_any_bullet {
        // All text
        let text_el = parse_text(&tx_body_node)?;
        return Ok(vec![ParsedContent::Text(text_el)]);
    }
    if all_have_bullet {
        // All list
        let list_el = parse_list(&tx_body_node)?;
        return Ok(vec![ParsedContent::List(list_el)]);
    }

    // Mixed content: split into groups of consecutive text vs list paragraphs.
    let mut results = Vec::new();
    let mut current_text_runs: Vec<Run> = Vec::new();
    let mut current_list_items: Vec<ListItem> = Vec::new();

    for p_node in &paragraphs {
        let has_runs = p_node.children().any(|n| {
            n.is_element() && n.tag_name().name() == "r" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        });
        if !has_runs {
            continue;
        }

        if paragraph_has_bullet(p_node) {
            // Flush any accumulated text runs
            if !current_text_runs.is_empty() {
                results.push(ParsedContent::Text(TextElement {
                    runs: std::mem::take(&mut current_text_runs),
                    is_title: false,
                }));
            }
            let (level, is_ordered) = parse_list_properties(p_node)?;
            let runs = parse_paragraph(p_node, true)?;
            current_list_items.push(ListItem {
                level,
                is_ordered,
                runs,
            });
        } else {
            // Flush any accumulated list items
            if !current_list_items.is_empty() {
                results.push(ParsedContent::List(ListElement {
                    items: std::mem::take(&mut current_list_items),
                }));
            }
            let mut runs = parse_paragraph(p_node, true)?;
            current_text_runs.append(&mut runs);
        }
    }

    // Flush remaining
    if !current_text_runs.is_empty() {
        results.push(ParsedContent::Text(TextElement {
            runs: current_text_runs,
            is_title: false,
        }));
    }
    if !current_list_items.is_empty() {
        results.push(ParsedContent::List(ListElement {
            items: current_list_items,
        }));
    }

    Ok(results)
}

/// Check whether a paragraph element has bullet markers (buAutoNum or buChar).
fn paragraph_has_bullet(p_node: &Node) -> bool {
    p_node.children().any(|n| {
        n.is_element()
            && n.tag_name().name() == "pPr"
            && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
            && n.children().any(|child| {
                child.is_element() && (child.tag_name().name() == "buAutoNum" || child.tag_name().name() == "buChar")
            })
    })
}

pub(super) fn parse_text(tx_body_node: &Node) -> Result<TextElement> {
    let mut runs = Vec::new();

    for p_node in tx_body_node.children().filter(|n| {
        n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        let mut paragraph_runs = parse_paragraph(&p_node, true)?;
        runs.append(&mut paragraph_runs);
    }

    Ok(TextElement { runs, is_title: false })
}

/// Result of parsing a graphicFrame element.
enum GraphicFrameResult {
    /// A table was found.
    Table(TableElement),
    /// A non-table graphic with the given URI (chart, diagram, SmartArt, etc.).
    Graphic(String),
    /// No recognizable graphic content.
    None,
}

fn parse_graphic_frame(node: &Node) -> Result<GraphicFrameResult> {
    // Find any graphicData element to determine the graphic type
    let graphic_data_node = node.descendants().find(|n| {
        n.is_element() && n.tag_name().name() == "graphicData" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    });

    if let Some(graphic_data) = graphic_data_node {
        let uri = graphic_data.attribute("uri").unwrap_or("");

        // Check if this is a table graphic
        if uri == "http://schemas.openxmlformats.org/drawingml/2006/table"
            && let Some(tbl_node) = graphic_data.children().find(|n| {
                n.is_element() && n.tag_name().name() == "tbl" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
            })
        {
            let table = parse_table(&tbl_node)?;
            return Ok(GraphicFrameResult::Table(table));
        }

        // Non-table graphic (chart, diagram, SmartArt, etc.)
        if !uri.is_empty() {
            return Ok(GraphicFrameResult::Graphic(uri.to_string()));
        }
    }

    Ok(GraphicFrameResult::None)
}

fn parse_table(tbl_node: &Node) -> Result<TableElement> {
    let mut rows = Vec::new();

    for tr_node in tbl_node.children().filter(|n| {
        n.is_element() && n.tag_name().name() == "tr" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        let row = parse_table_row(&tr_node)?;
        rows.push(row);
    }

    Ok(TableElement { rows })
}

fn parse_table_row(tr_node: &Node) -> Result<TableRow> {
    let mut cells = Vec::new();

    for tc_node in tr_node.children().filter(|n| {
        n.is_element() && n.tag_name().name() == "tc" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        let cell = parse_table_cell(&tc_node)?;
        cells.push(cell);
    }

    Ok(TableRow { cells })
}

fn parse_table_cell(tc_node: &Node) -> Result<TableCell> {
    let mut runs = Vec::new();

    if let Some(tx_body_node) = tc_node.children().find(|n| {
        n.is_element() && n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        for p_node in tx_body_node.children().filter(|n| {
            n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        }) {
            let mut paragraph_runs = parse_paragraph(&p_node, false)?;
            runs.append(&mut paragraph_runs);
        }
    }

    Ok(TableCell { runs })
}

fn parse_pic(pic_node: &Node) -> Result<ImageReference> {
    let blip_node = pic_node
        .descendants()
        .find(|n| {
            n.is_element() && n.tag_name().name() == "blip" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        })
        .ok_or_else(|| KreuzbergError::parsing("Image blip not found".to_string()))?;

    let embed_attr = blip_node
        .attribute((RELATIONSHIPS_NAMESPACE, "embed"))
        .or_else(|| blip_node.attribute("r:embed"))
        .ok_or_else(|| KreuzbergError::parsing("Image embed attribute not found".to_string()))?;

    // Extract alt text from nvPicPr > cNvPr descr attribute
    let description = pic_node
        .descendants()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "cNvPr"
                && n.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE)
        })
        .or_else(|| {
            // Also check for non-namespaced cNvPr (common in pic elements)
            pic_node.descendants().find(|n| {
                n.is_element()
                    && n.tag_name().name() == "cNvPr"
                    && n.parent().is_some_and(|p| p.tag_name().name() == "nvPicPr")
            })
        })
        .and_then(|cnv| cnv.attribute("descr"))
        .filter(|d| !d.is_empty())
        .map(|d| d.to_string());

    let image_ref = ImageReference {
        id: embed_attr.to_string(),
        target: String::new(),
        description,
    };

    Ok(image_ref)
}

fn parse_list(tx_body_node: &Node) -> Result<ListElement> {
    let mut items = Vec::new();

    for p_node in tx_body_node.children().filter(|n| {
        n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        let (level, is_ordered) = parse_list_properties(&p_node)?;

        let runs = parse_paragraph(&p_node, true)?;

        items.push(ListItem {
            level,
            is_ordered,
            runs,
        });
    }

    Ok(ListElement { items })
}

fn parse_list_properties(p_node: &Node) -> Result<(u32, bool)> {
    let mut level = 1;
    let mut is_ordered = false;

    if let Some(p_pr_node) = p_node.children().find(|n| {
        n.is_element() && n.tag_name().name() == "pPr" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        if let Some(lvl_attr) = p_pr_node.attribute("lvl") {
            level = lvl_attr.parse::<u32>().unwrap_or(0) + 1;
        }

        is_ordered = p_pr_node.children().any(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
                && n.tag_name().name() == "buAutoNum"
        });
    }

    Ok((level, is_ordered))
}

fn parse_paragraph(p_node: &Node, add_new_line: bool) -> Result<Vec<Run>> {
    let run_nodes: Vec<_> = p_node
        .children()
        .filter(|n| {
            n.is_element() && n.tag_name().name() == "r" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        })
        .collect();

    let count = run_nodes.len();
    let mut runs: Vec<Run> = Vec::new();

    for (idx, r_node) in run_nodes.iter().enumerate() {
        let mut run = parse_run(r_node)?;

        if add_new_line && idx == count - 1 {
            run.text.push('\n');
        }

        runs.push(run);
    }
    Ok(runs)
}

fn parse_run(r_node: &Node) -> Result<Run> {
    let mut text = String::new();
    let mut formatting = Formatting::default();
    let mut hyperlink_id = None;

    if let Some(r_pr_node) = r_node.children().find(|n| {
        n.is_element() && n.tag_name().name() == "rPr" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
    }) {
        if let Some(b_attr) = r_pr_node.attribute("b") {
            formatting.bold = b_attr == "1" || b_attr.eq_ignore_ascii_case("true");
        }
        if let Some(i_attr) = r_pr_node.attribute("i") {
            formatting.italic = i_attr == "1" || i_attr.eq_ignore_ascii_case("true");
        }
        if let Some(u_attr) = r_pr_node.attribute("u") {
            formatting.underlined = u_attr != "none";
        }
        if let Some(strike_attr) = r_pr_node.attribute("strike") {
            formatting.strikethrough = matches!(strike_attr, "sngStrike" | "dblStrike");
        }
        if let Some(sz_attr) = r_pr_node.attribute("sz") {
            formatting.font_size = sz_attr.parse::<u32>().ok();
        }
        if let Some(lang_attr) = r_pr_node.attribute("lang") {
            formatting.lang = lang_attr.to_string();
        }

        // Capture hyperlink reference: <a:hlinkClick r:id="rIdN"/>
        if let Some(hlink_node) = r_pr_node.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "hlinkClick"
                && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE)
        }) {
            hyperlink_id = hlink_node
                .attribute((RELATIONSHIPS_NAMESPACE, "id"))
                .or_else(|| hlink_node.attribute("r:id"))
                .map(|s| s.to_string());
        }
    }

    if let Some(t_node) = r_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "t" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE))
        && let Some(t) = t_node.text()
    {
        text.push_str(t);
    }
    Ok(Run {
        text,
        formatting,
        hyperlink_id,
    })
}

pub(super) fn extract_position(node: &Node) -> ElementPosition {
    let default = ElementPosition::default();

    node.descendants()
        .find(|n| n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE) && n.tag_name().name() == "xfrm")
        .and_then(|xfrm| {
            let x = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE))
                .and_then(|off| off.attribute("x")?.parse::<i64>().ok())?;

            let y = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE))
                .and_then(|off| off.attribute("y")?.parse::<i64>().ok())?;

            // Extract extent (cx, cy) from a:ext element
            let (cx, cy) = xfrm
                .children()
                .find(|n| n.tag_name().name() == "ext" && n.tag_name().namespace() == Some(DRAWINGML_NAMESPACE))
                .map(|ext| {
                    let cx = ext.attribute("cx").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
                    let cy = ext.attribute("cy").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
                    (cx, cy)
                })
                .unwrap_or((0, 0));

            Some(ElementPosition { x, y, cx, cy })
        })
        .unwrap_or(default)
}

/// Parsed relationships from a slide rels file.
pub(super) struct SlideRels {
    pub(super) images: Vec<ImageReference>,
    pub(super) hyperlinks: Vec<super::elements::HyperlinkReference>,
}

pub(super) fn parse_slide_rels(rels_data: &[u8]) -> Result<SlideRels> {
    let xml_str = utf8_validation::from_utf8(rels_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in rels XML: {}", e)))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse rels XML: {}", e)))?;

    let mut images = Vec::new();
    let mut hyperlinks = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("Relationship")
            && let Some(rel_type) = node.attribute("Type")
            && let (Some(id), Some(target)) = (node.attribute("Id"), node.attribute("Target"))
        {
            if rel_type.contains("image") {
                images.push(ImageReference {
                    id: id.to_string(),
                    target: target.to_string(),
                    description: None,
                });
            } else if rel_type.contains("hyperlink") {
                hyperlinks.push(super::elements::HyperlinkReference {
                    id: id.to_string(),
                    url: target.to_string(),
                });
            }
        }
    }

    Ok(SlideRels { images, hyperlinks })
}

pub(super) fn parse_presentation_rels(rels_data: &[u8]) -> Result<Vec<String>> {
    let xml_str = utf8_validation::from_utf8(rels_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in presentation rels: {}", e)))?;

    let doc = Document::parse(xml_str)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse presentation rels: {}", e)))?;

    let mut slide_paths = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("Relationship")
            && let Some(rel_type) = node.attribute("Type")
            && rel_type.contains("slide")
            && !rel_type.contains("slideMaster")
            && let Some(target) = node.attribute("Target")
        {
            let normalized_target = target.strip_prefix('/').unwrap_or(target);
            let final_path = if normalized_target.starts_with("ppt/") {
                normalized_target.to_string()
            } else {
                format!("ppt/{}", normalized_target)
            };
            slide_paths.push(final_path);
        }
    }

    // Sort slide paths to ensure correct ordering regardless of XML order.
    // PowerPoint doesn't guarantee relationship order in the rels file.
    // GitHub Issue #329: Without sorting, slides can be processed in wrong order,
    // causing images to have incorrect page numbers.
    slide_paths.sort();

    Ok(slide_paths)
}
