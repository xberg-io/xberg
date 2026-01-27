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

const P_NAMESPACE: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const RELS_NAMESPACE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

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

    if namespace != P_NAMESPACE {
        return Ok(elements);
    }

    let position = extract_position(node);

    match tag_name {
        "sp" => {
            let position = extract_position(node);
            // parse_sp returns None for shapes without txBody (e.g., image placeholders)
            if let Some(content) = parse_sp(node)? {
                match content {
                    ParsedContent::Text(text) => elements.push(SlideElement::Text(text, position)),
                    ParsedContent::List(list) => elements.push(SlideElement::List(list, position)),
                }
            }
        }
        "graphicFrame" => {
            if let Some(graphic_element) = parse_graphic_frame(node)? {
                elements.push(SlideElement::Table(graphic_element, position));
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

fn parse_sp(sp_node: &Node) -> Result<Option<ParsedContent>> {
    // Some shapes like image placeholders (<p:ph type="pic"/>) don't have txBody.
    // These should be skipped gracefully - they contain no text to extract.
    // GitHub Issue #321 Bug 1
    let tx_body_node = match sp_node
        .children()
        .find(|n| n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(P_NAMESPACE))
    {
        Some(node) => node,
        None => return Ok(None), // Skip shapes without txBody
    };

    let is_list = tx_body_node.descendants().any(|n| {
        n.is_element()
            && n.tag_name().name() == "pPr"
            && n.tag_name().namespace() == Some(A_NAMESPACE)
            && (n.attribute("lvl").is_some()
                || n.children().any(|child| {
                    child.is_element()
                        && (child.tag_name().name() == "buAutoNum" || child.tag_name().name() == "buChar")
                }))
    });

    if is_list {
        Ok(Some(ParsedContent::List(parse_list(&tx_body_node)?)))
    } else {
        Ok(Some(ParsedContent::Text(parse_text(&tx_body_node)?)))
    }
}

pub(super) fn parse_text(tx_body_node: &Node) -> Result<TextElement> {
    let mut runs = Vec::new();

    for p_node in tx_body_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let mut paragraph_runs = parse_paragraph(&p_node, true)?;
        runs.append(&mut paragraph_runs);
    }

    Ok(TextElement { runs })
}

fn parse_graphic_frame(node: &Node) -> Result<Option<TableElement>> {
    let graphic_data_node = node.descendants().find(|n| {
        n.is_element()
            && n.tag_name().name() == "graphicData"
            && n.tag_name().namespace() == Some(A_NAMESPACE)
            && n.attribute("uri") == Some("http://schemas.openxmlformats.org/drawingml/2006/table")
    });

    if let Some(graphic_data) = graphic_data_node
        && let Some(tbl_node) = graphic_data
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "tbl" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let table = parse_table(&tbl_node)?;
        return Ok(Some(table));
    }

    Ok(None)
}

fn parse_table(tbl_node: &Node) -> Result<TableElement> {
    let mut rows = Vec::new();

    for tr_node in tbl_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "tr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let row = parse_table_row(&tr_node)?;
        rows.push(row);
    }

    Ok(TableElement { rows })
}

fn parse_table_row(tr_node: &Node) -> Result<TableRow> {
    let mut cells = Vec::new();

    for tc_node in tr_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "tc" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let cell = parse_table_cell(&tc_node)?;
        cells.push(cell);
    }

    Ok(TableRow { cells })
}

fn parse_table_cell(tc_node: &Node) -> Result<TableCell> {
    let mut runs = Vec::new();

    if let Some(tx_body_node) = tc_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        for p_node in tx_body_node
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
        {
            let mut paragraph_runs = parse_paragraph(&p_node, false)?;
            runs.append(&mut paragraph_runs);
        }
    }

    Ok(TableCell { runs })
}

fn parse_pic(pic_node: &Node) -> Result<ImageReference> {
    let blip_node = pic_node
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "blip" && n.tag_name().namespace() == Some(A_NAMESPACE))
        .ok_or_else(|| KreuzbergError::parsing("Image blip not found".to_string()))?;

    let embed_attr = blip_node
        .attribute((RELS_NAMESPACE, "embed"))
        .or_else(|| blip_node.attribute("r:embed"))
        .ok_or_else(|| KreuzbergError::parsing("Image embed attribute not found".to_string()))?;

    let image_ref = ImageReference {
        id: embed_attr.to_string(),
        target: String::new(),
    };

    Ok(image_ref)
}

fn parse_list(tx_body_node: &Node) -> Result<ListElement> {
    let mut items = Vec::new();

    for p_node in tx_body_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
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

    if let Some(p_pr_node) = p_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "pPr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        if let Some(lvl_attr) = p_pr_node.attribute("lvl") {
            level = lvl_attr.parse::<u32>().unwrap_or(0) + 1;
        }

        is_ordered = p_pr_node.children().any(|n| {
            n.is_element() && n.tag_name().namespace() == Some(A_NAMESPACE) && n.tag_name().name() == "buAutoNum"
        });
    }

    Ok((level, is_ordered))
}

fn parse_paragraph(p_node: &Node, add_new_line: bool) -> Result<Vec<Run>> {
    let run_nodes: Vec<_> = p_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "r" && n.tag_name().namespace() == Some(A_NAMESPACE))
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

    if let Some(r_pr_node) = r_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "rPr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        if let Some(b_attr) = r_pr_node.attribute("b") {
            formatting.bold = b_attr == "1" || b_attr.eq_ignore_ascii_case("true");
        }
        if let Some(i_attr) = r_pr_node.attribute("i") {
            formatting.italic = i_attr == "1" || i_attr.eq_ignore_ascii_case("true");
        }
        if let Some(u_attr) = r_pr_node.attribute("u") {
            formatting.underlined = u_attr != "none";
        }
        if let Some(lang_attr) = r_pr_node.attribute("lang") {
            formatting.lang = lang_attr.to_string();
        }
    }

    if let Some(t_node) = r_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "t" && n.tag_name().namespace() == Some(A_NAMESPACE))
        && let Some(t) = t_node.text()
    {
        text.push_str(t);
    }
    Ok(Run { text, formatting })
}

pub(super) fn extract_position(node: &Node) -> ElementPosition {
    let default = ElementPosition::default();

    node.descendants()
        .find(|n| n.tag_name().namespace() == Some(A_NAMESPACE) && n.tag_name().name() == "xfrm")
        .and_then(|xfrm| {
            let x = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(A_NAMESPACE))
                .and_then(|off| off.attribute("x")?.parse::<i64>().ok())?;

            let y = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(A_NAMESPACE))
                .and_then(|off| off.attribute("y")?.parse::<i64>().ok())?;

            Some(ElementPosition { x, y })
        })
        .unwrap_or(default)
}

pub(super) fn parse_slide_rels(rels_data: &[u8]) -> Result<Vec<ImageReference>> {
    let xml_str = utf8_validation::from_utf8(rels_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in rels XML: {}", e)))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse rels XML: {}", e)))?;

    let mut images = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("Relationship")
            && let Some(rel_type) = node.attribute("Type")
            && rel_type.contains("image")
            && let (Some(id), Some(target)) = (node.attribute("Id"), node.attribute("Target"))
        {
            images.push(ImageReference {
                id: id.to_string(),
                target: target.to_string(),
            });
        }
    }

    Ok(images)
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
