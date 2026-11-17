use super::*;

// --- Regression tests for #47, #79, #80, #90, #91, #238 ---
//
// Shared helper used by 5 of the 6 tests below (`build_single_slide_pptx`);
// #238 builds its own bytes since it needs corrupt, not merely custom,
// `docProps/core.xml`/comment parts. ~keep

/// Build a minimal single-slide PPTX with caller-supplied slide XML,
/// optional slide relationships XML, and optional extra ZIP parts (used
/// for chart/SmartArt data parts and referenced media).
pub(crate) fn build_single_slide_pptx(
    slide_xml: &str,
    slide_rels_xml: Option<&str>,
    extra_parts: &[(&str, &[u8])],
) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="png" ContentType="image/png"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

    zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#).unwrap();

    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();

    if let Some(rels) = slide_rels_xml {
        zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();
    }

    zip.start_file("docProps/core.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Test</dc:title></cp:coreProperties>"#,
    )
    .unwrap();

    zip.start_file("docProps/app.xml", opts).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Slides>1</Slides></Properties>").unwrap();

    for (path, data) in extra_parts {
        zip.start_file(*path, opts).unwrap();
        zip.write_all(data).unwrap();
    }

    let _ = zip.finish().unwrap();
    buffer
}

/// Math runs must leave the parser as LaTeX beside the text, with the display
/// flag intact: the flattened text cannot tell a converted equation apart from
/// author text that holds the same characters.
#[test]
fn test_math_runs_are_collected_beside_the_text() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree><p:sp><p:txBody>
        <a:p>
            <m:oMathPara>
                <m:oMath>
                    <m:sSup>
                        <m:e><m:r><m:t>x</m:t></m:r></m:e>
                        <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                    </m:sSup>
                </m:oMath>
            </m:oMathPara>
        </a:p>
        <a:p>
            <a:r><a:t>Rate </a:t></a:r>
            <m:oMath><m:r><m:t>a</m:t></m:r></m:oMath>
        </a:p>
    </p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let extraction = super::extract_pptx_from_bytes_with_slide_contents(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert_eq!(
        extraction.formulas,
        vec![("x^{2}".to_string(), true), ("a".to_string(), false)],
        "display and inline math both reach the caller"
    );
}

/// The slide structure carries math as formula nodes, not as LaTeX buried in
/// the text of a paragraph or a list item.
#[test]
fn test_slide_structure_emits_formula_nodes() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a:r><a:t>Energy of a body at rest</a:t></a:r></a:p>
            <a:p>
                <a:r><a:t>Rate </a:t></a:r>
                <m:oMath><m:r><m:t>Q</m:t></m:r></m:oMath>
            </a:p>
        </p:txBody></p:sp>
        <p:sp><p:txBody>
            <a:p><a:pPr lvl="0"><a:buChar char="-"/></a:pPr>
                <a:r><a:t>Growth </a:t></a:r>
                <m:oMath><m:r><m:t>Z</m:t></m:r></m:oMath>
            </a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let extraction = super::extract_pptx_from_bytes_with_slide_contents(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            include_structure: true,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let structure = extraction.result.document.expect("structure requested");
    let math: Vec<&str> = structure
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            crate::types::document_structure::NodeContent::Formula { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        math,
        vec!["Q", "Z"],
        "text shape and list item both emit a formula node"
    );

    let texts: Vec<String> = structure
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            crate::types::document_structure::NodeContent::Paragraph { text }
            | crate::types::document_structure::NodeContent::Heading { text, .. }
            | crate::types::document_structure::NodeContent::ListItem { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    let joined = texts.join(" | ");
    assert!(
        !joined.contains('Q') && !joined.contains('Z'),
        "no text node keeps the bare LaTeX: {joined}"
    );
    assert!(
        joined.contains("Rate") && joined.contains("Growth"),
        "words survive: {joined}"
    );
}

/// A crafted `<a:pPr lvl="4294967294">` must not turn `add_list_item`'s
/// per-level `"  "` indent loop into ~4.29 billion pushes. Against the unfixed
/// parser (`lvl_attr.parse::<u32>().unwrap_or(0) + 1` with no upper bound) this
/// scenario does not fail an assertion -- the process hangs or is OOM-killed
/// building several GB of indentation before any `assert_eq!` runs. The fixed
/// parser must instead come back immediately with the level clamped to
/// `MAX_LIST_NESTING_LEVEL` (8), i.e. 8 two-space indents.
#[test]
fn test_pptx_list_level_bomb_is_clamped_not_allocated() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a:r><a:t>Intro</a:t></a:r></a:p>
            <a:p><a:pPr lvl="4294967294"><a:buChar char="•"/></a:pPr><a:r><a:t>Bomb</a:t></a:r></a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .expect("extraction must succeed for an out-of-range (clamped) list level");

    // Match the WHOLE line, not a substring: `contains` is satisfied by any
    // indent at least this deep, so it passes even with the clamp widened to
    // 50_000 and cannot detect over-indentation -- the exact thing the clamp
    // is here to bound.
    let expected_line = format!("{}- Bomb", "  ".repeat(8));
    assert!(
        result.content.lines().any(|line| line == expected_line),
        "lvl=4294967294 must clamp to exactly 8 levels of indentation, got: {:?}",
        result.content
    );
}

/// Positive control: a normal `lvl="2"` list item (well within the clamp) must
/// render with exactly the same two-space-per-level indentation as before this
/// fix -- the guard against a cap set too tight.
#[test]
fn test_pptx_list_level_two_positive_control_unchanged() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a:r><a:t>Intro</a:t></a:r></a:p>
            <a:p><a:pPr lvl="2"><a:buChar char="•"/></a:pPr><a:r><a:t>Nested</a:t></a:r></a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .expect("extraction must succeed for a normal lvl=2 list item");

    assert!(
        result.content.lines().any(|line| line == "    - Nested"),
        "lvl=2 must render with exactly 2 two-space indents, got: {:?}",
        result.content
    );
}

/// #47: OMML math wrapped in `mc:AlternateContent`/`mc:Choice`/`a14:m` must be
/// converted to LaTeX via the shared `docx::math` OMML converter, not dropped,
/// and the `mc:Fallback` text must not also appear (Choice content wins).
#[test]
fn test_issue_47_omml_math_wired_into_pptx() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
       xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree><p:sp><p:txBody>
        <a:p>
            <mc:AlternateContent>
                <mc:Choice Requires="a14">
                    <a14:m>
                        <m:oMathPara>
                            <m:oMath>
                                <m:sSup>
                                    <m:e><m:r><m:t>x</m:t></m:r></m:e>
                                    <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                                </m:sSup>
                            </m:oMath>
                        </m:oMathPara>
                    </a14:m>
                </mc:Choice>
                <mc:Fallback>
                    <a:r><a:t>[equation]</a:t></a:r>
                </mc:Fallback>
            </mc:AlternateContent>
        </a:p>
    </p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert!(
        result.content.contains("$$x^{2}$$"),
        "expected OMML math rendered as display LaTeX, got: {:?}",
        result.content
    );
    assert!(
        !result.content.contains("[equation]"),
        "mc:Choice content succeeded, so mc:Fallback text must not also appear, got: {:?}",
        result.content
    );
}

/// #79: `mc:AlternateContent` wrapping a whole shape must fall back to
/// `mc:Fallback` when `mc:Choice` carries no PresentationML-recognized
/// content, and `p:cxnSp` connectors must contribute their text.
#[test]
fn test_issue_79_alternate_content_fallback_and_connector_text() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
       xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main">
    <p:cSld><p:spTree>
        <mc:AlternateContent>
            <mc:Choice Requires="p159">
                <p14:futureShape/>
            </mc:Choice>
            <mc:Fallback>
                <p:sp>
                    <p:txBody><a:p><a:r><a:t>Fallback shape text</a:t></a:r></a:p></p:txBody>
                </p:sp>
            </mc:Fallback>
        </mc:AlternateContent>
        <p:cxnSp>
            <p:txBody><a:p><a:r><a:t>Connector label</a:t></a:r></a:p></p:txBody>
        </p:cxnSp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert!(
        result.content.contains("Fallback shape text"),
        "mc:Choice has no PresentationML content, so mc:Fallback must be used, got: {:?}",
        result.content
    );
    assert!(
        result.content.contains("Connector label"),
        "p:cxnSp connector text must be extracted, got: {:?}",
        result.content
    );
}

/// #80: chart (`c:chart`) and SmartArt/diagram (`dgm:relIds`) graphic frames
/// reference text-bearing parts in separate ZIP entries; that text must be
/// resolved and included in the extracted content.
#[test]
fn test_issue_80_chart_and_smartart_text_extracted() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:graphicFrame>
            <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart 1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
            <a:graphic>
                <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                    <c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId2"/>
                </a:graphicData>
            </a:graphic>
        </p:graphicFrame>
        <p:graphicFrame>
            <p:nvGraphicFramePr><p:cNvPr id="3" name="SmartArt 1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
            <a:graphic>
                <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
                    <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId3"/>
                </a:graphicData>
            </a:graphic>
        </p:graphicFrame>
    </p:spTree></p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData" Target="../diagrams/data1.xml"/>
</Relationships>"#;

    let chart_xml: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:title><c:tx><c:rich><a:p><a:r><a:t>Revenue by Quarter</a:t></a:r></a:p></c:rich></c:tx></c:title>
    <c:plotArea><c:barChart><c:ser>
      <c:cat><c:strRef><c:strCache><c:pt idx="0"><c:v>Q1</c:v></c:pt></c:strCache></c:strRef></c:cat>
      <c:val><c:numRef><c:numCache><c:pt idx="0"><c:v>42</c:v></c:pt></c:numCache></c:numRef></c:val>
    </c:ser></c:barChart></c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    let diagram_xml: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="1" type="node"><dgm:t><a:p><a:r><a:t>Step One</a:t></a:r></a:p></dgm:t></dgm:pt>
    <dgm:pt modelId="2" type="node"><dgm:t><a:p><a:r><a:t>Step Two</a:t></a:r></a:p></dgm:t></dgm:pt>
  </dgm:ptLst>
</dgm:dataModel>"#;

    let pptx = build_single_slide_pptx(
        slide_xml,
        Some(slide_rels),
        &[
            ("ppt/charts/chart1.xml", chart_xml),
            ("ppt/diagrams/data1.xml", diagram_xml),
        ],
    );

    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert!(
        result.content.contains("Revenue by Quarter"),
        "chart title must be extracted, got: {:?}",
        result.content
    );
    assert!(
        result.content.contains("Q1") && result.content.contains("42"),
        "chart category/value text must be extracted, got: {:?}",
        result.content
    );
    assert!(
        result.content.contains("Step One") && result.content.contains("Step Two"),
        "SmartArt node text must be extracted, got: {:?}",
        result.content
    );
}

/// #90: `a:fld` (cached field text, e.g. slide number) and `a:br` (explicit
/// line break) must contribute to paragraph text instead of being skipped.
///
/// A separate title placeholder shape is included so the body paragraph
/// below is rendered through the normal paragraph path (`add_text`, which
/// preserves internal newlines) rather than being picked by the "shortest
/// text becomes the title" heuristic, whose title path collapses `\n` to
/// a space and would otherwise mask the very break this test verifies.
#[test]
fn test_issue_90_field_and_break_runs_extracted() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>
        <p:sp>
            <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
            <p:txBody><a:p><a:r><a:t>Slide Title</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp>
            <p:txBody>
                <a:p>
                    <a:r><a:t>Page </a:t></a:r>
                    <a:fld id="{00000000-0000-0000-0000-000000000000}" type="slidenum"><a:t>3</a:t></a:fld>
                    <a:br/>
                    <a:r><a:t>Second line</a:t></a:r>
                </a:p>
            </p:txBody>
        </p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    assert!(
        result.content.contains("Page 3\nSecond line"),
        "a:fld cached text and a:br line break must both be extracted in order, got: {:?}",
        result.content
    );
}

/// #91: images must be paired with their own shape's dimensions/alt-text by
/// relationship ID, not by hash-map iteration order, and an image whose
/// target cannot be read must be warned about and skipped rather than
/// silently mis-paired.
#[test]
fn test_issue_91_image_pairing_by_rel_id_and_unreadable_image_warns() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="2" name="Pic1" descr="First image"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="952500" cy="952500"/></a:xfrm></p:spPr>
        </p:pic>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="3" name="Pic2" descr="Second image"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId3"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1905000" cy="1905000"/></a:xfrm></p:spPr>
        </p:pic>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="4" name="Pic3" descr="Missing image"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId4"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
        </p:pic>
    </p:spTree></p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image2.png"/>
    <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
</Relationships>"#;

    let png_bytes: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82";

    let pptx = build_single_slide_pptx(
        slide_xml,
        Some(slide_rels),
        &[("ppt/media/image1.png", png_bytes), ("ppt/media/image2.png", png_bytes)],
    );

    let mut warnings = Vec::new();
    let result = extract_pptx_from_bytes(&pptx, &PptxExtractionOptions::default(), &mut warnings).unwrap();

    assert_eq!(
        result.images.len(),
        2,
        "the image with a missing target must not be counted as extracted"
    );
    assert_eq!(result.images[0].description.as_deref(), Some("First image"));
    assert_eq!(result.images[0].width, Some(100));
    assert_eq!(result.images[1].description.as_deref(), Some("Second image"));
    assert_eq!(result.images[1].width, Some(200));

    assert!(
        warnings.iter().any(|w| w.source == "pptx"
            && w.message == "Image 'rId4' referenced on slide 1 could not be read; it was not extracted"),
        "expected a warning naming the unreadable image, got: {:?}",
        warnings
    );
}

/// #238: a corrupt `docProps/core.xml` or comment part must be surfaced as a
/// `ProcessingWarning` naming the part, not silently dropped while the rest
/// of the document is returned as if it were complete.
#[test]
fn test_issue_238_warns_on_corrupt_metadata_and_comments() {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

    zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#).unwrap();

    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Slide text</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#,
    )
    .unwrap();

    zip.start_file("ppt/comments/comment1.xml", opts).unwrap();
    zip.write_all(b"not valid xml content").unwrap();

    zip.start_file("docProps/core.xml", opts).unwrap();
    zip.write_all(b"not valid xml content").unwrap();

    zip.start_file("docProps/app.xml", opts).unwrap();
    zip.write_all(b"<?xml version=\"1.0\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Slides>1</Slides></Properties>").unwrap();

    let _ = zip.finish().unwrap();

    let mut warnings = Vec::new();
    let result = extract_pptx_from_bytes(
        &buffer,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut warnings,
    )
    .unwrap();

    assert!(
        result.content.contains("Slide text"),
        "extraction of the rest of the document must still succeed"
    );

    assert!(
        warnings
            .iter()
            .any(|w| w.source == "pptx" && w.message.contains("docProps/core.xml")),
        "expected a warning naming the corrupt docProps/core.xml, got: {:?}",
        warnings
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.source == "pptx" && w.message.contains("comment1.xml")),
        "expected a warning naming the corrupt comment file, got: {:?}",
        warnings
    );
}

fn create_pptx_with_note_relationships(relationships: &[(usize, &str, &str)], notes: &[(&str, &str)]) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::write::{SimpleFileOptions, ZipWriter};

    let bytes = create_pptx_with_sections_and_notes(&[("Slide 1", None), ("Slide 2", None), ("Slide 3", None)], &[]);
    let mut zip = ZipWriter::new_append(Cursor::new(bytes)).unwrap();
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (slide, target, mode) in relationships {
        let target_mode = if mode.is_empty() {
            String::new()
        } else {
            format!(r#" TargetMode="{mode}""#)
        };
        let xml = format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="notes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide"
 Target="{target}"{target_mode}/></Relationships>"#
        );
        zip.start_file(format!("ppt/slides/_rels/slide{slide}.xml.rels"), options)
            .unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
    }
    for (path, text) in notes {
        let xml = format!(
            r#"<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:notes>"#
        );
        zip.start_file(*path, options).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn extracted_speaker_notes(bytes: &[u8]) -> Vec<Option<String>> {
    let result = extract_pptx_from_bytes(
        bytes,
        &PptxExtractionOptions {
            extract_images: false,
            page_config: Some(crate::core::config::PageConfig::default()),
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();
    let pages = result.page_contents.unwrap();
    assert_eq!(pages.len(), 3);
    assert_eq!(
        pages.iter().map(|page| page.page_number).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    pages.into_iter().map(|page| page.speaker_notes).collect()
}

#[test]
fn test_speaker_notes_follow_sparse_relationship() {
    let bytes = create_pptx_with_note_relationships(
        &[(2, "../notesSlides/notesSlide1.xml", "Internal")],
        &[("ppt/notesSlides/notesSlide1.xml", "Note for slide 2")],
    );
    assert_eq!(
        extracted_speaker_notes(&bytes),
        vec![None, Some("Note for slide 2".to_string()), None]
    );
}

#[test]
fn test_speaker_notes_follow_shuffled_relationships() {
    let bytes = create_pptx_with_note_relationships(
        &[
            (1, "../notesSlides/notesSlide9.xml", "Internal"),
            (2, "../notesSlides/notesSlide4.xml", "Internal"),
            (3, "../notesSlides/notesSlide1.xml", "Internal"),
        ],
        &[
            ("ppt/notesSlides/notesSlide1.xml", "Third note"),
            ("ppt/notesSlides/notesSlide9.xml", "First note"),
            ("ppt/notesSlides/notesSlide4.xml", "Second note"),
        ],
    );
    assert_eq!(
        extracted_speaker_notes(&bytes),
        vec![
            Some("First note".into()),
            Some("Second note".into()),
            Some("Third note".into())
        ]
    );
}

#[test]
fn test_speaker_notes_normalize_relationship_targets() {
    let bytes = create_pptx_with_note_relationships(
        &[
            (1, "../notesSlides/./notesSlide8.xml", "Internal"),
            (2, "/ppt/notesSlides/notesSlide7.xml", "Internal"),
            (3, "../notesSlides/../notesSlides/notesSlide6.xml", "Internal"),
        ],
        &[
            ("ppt/notesSlides/notesSlide8.xml", "First note"),
            ("ppt/notesSlides/notesSlide7.xml", "Second note"),
            ("ppt/notesSlides/notesSlide6.xml", "Third note"),
        ],
    );
    assert_eq!(
        extracted_speaker_notes(&bytes),
        vec![
            Some("First note".into()),
            Some("Second note".into()),
            Some("Third note".into())
        ]
    );
}

#[test]
fn test_speaker_notes_ignore_unlinked_parts() {
    let bytes = create_pptx_with_note_relationships(&[], &[("ppt/notesSlides/notesSlide1.xml", "Orphan note")]);
    assert_eq!(extracted_speaker_notes(&bytes), vec![None, None, None]);
}

#[test]
fn test_speaker_notes_ignore_missing_external_and_unsafe_targets() {
    let bytes = create_pptx_with_note_relationships(
        &[
            (1, "../notesSlides/missing.xml", "Internal"),
            (2, "../notesSlides/notesSlide2.xml", "External"),
            (3, "../../../notesSlide3.xml", "Internal"),
        ],
        &[
            ("ppt/notesSlides/notesSlide2.xml", "External decoy"),
            ("notesSlide3.xml", "Traversal decoy"),
        ],
    );
    assert_eq!(extracted_speaker_notes(&bytes), vec![None, None, None]);
}

#[test]
fn test_speaker_notes_without_note_parts() {
    let bytes = create_pptx_with_note_relationships(&[], &[]);
    assert_eq!(extracted_speaker_notes(&bytes), vec![None, None, None]);
}

#[test]
fn test_speaker_notes_default_relationship_mode_is_internal() {
    let bytes = create_pptx_with_note_relationships(
        &[(2, "../notesSlides/notesSlide9.xml", "")],
        &[("ppt/notesSlides/notesSlide9.xml", "Default internal note")],
    );
    assert_eq!(
        extracted_speaker_notes(&bytes),
        vec![None, Some("Default internal note".into()), None]
    );
}

#[test]
fn test_speaker_notes_ignore_similar_relationship_types() {
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="decoy" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlideExtra"
 Target="../notesSlides/notesSlide1.xml"/></Relationships>"#;
    let notes = br#"<notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<a:t>Wrong relationship type</a:t></notes>"#;
    let bytes = build_single_slide_pptx("<slide/>", Some(rels), &[("ppt/notesSlides/notesSlide1.xml", notes)]);
    let mut container =
        PptxContainer::from_bytes(&bytes, &crate::extractors::security::SecurityLimits::default()).unwrap();
    assert_eq!(
        extract_all_notes(&mut container, &mut Vec::new()).unwrap(),
        std::collections::HashMap::new()
    );
}

#[test]
fn test_speaker_notes_warn_on_relationship_read_failure_and_keep_other_notes() {
    let mut bytes = create_pptx_with_note_relationships(
        &[
            (1, "../notesSlides/notesSlide1.xml", "Internal"),
            (2, "../notesSlides/notesSlide2.xml", "Internal"),
        ],
        &[
            ("ppt/notesSlides/notesSlide1.xml", "Retained note"),
            ("ppt/notesSlides/notesSlide2.xml", "Unreadable relationship"),
        ],
    );
    let rels_path = "ppt/slides/_rels/slide2.xml.rels";
    let offset = {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        usize::try_from(archive.by_name(rels_path).unwrap().data_start().unwrap()).unwrap()
    };
    bytes[offset] ^= 1;
    let mut container =
        PptxContainer::from_bytes(&bytes, &crate::extractors::security::SecurityLimits::default()).unwrap();
    let read_error = container.read_file(rels_path).unwrap_err();
    assert!(
        read_error.to_string().to_ascii_lowercase().contains("checksum"),
        "{read_error}"
    );
    let mut warnings = Vec::new();
    let notes = extract_all_notes(&mut container, &mut warnings).unwrap();
    assert_eq!(
        notes,
        std::collections::HashMap::from([(1, "Retained note".to_string())])
    );
    assert!(
        warnings.iter().any(|warning| warning.source == "pptx"
            && warning.message.contains("speaker notes for slide 2")
            && warning.message.to_ascii_lowercase().contains("checksum")),
        "{warnings:?}"
    );
}

#[test]
fn test_speaker_notes_follow_strict_ooxml_relationship() {
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="notes" Type="http://purl.oclc.org/ooxml/officeDocument/relationships/notesSlide"
 Target="../notesSlides/notesSlide1.xml"/></Relationships>"#;
    let notes = br#"<notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<a:t>Strict notes relationship</a:t></notes>"#;
    let bytes = build_single_slide_pptx("<slide/>", Some(rels), &[("ppt/notesSlides/notesSlide1.xml", notes)]);
    let mut container =
        PptxContainer::from_bytes(&bytes, &crate::extractors::security::SecurityLimits::default()).unwrap();
    assert_eq!(
        extract_all_notes(&mut container, &mut Vec::new()).unwrap(),
        std::collections::HashMap::from([(1, "Strict notes relationship".to_string())])
    );
}

/// A graphic frame (table, chart, SmartArt) keeps its offset in its own `p:xfrm`, not in an
/// `a:xfrm`, so its position used to read as (0, 0): a table in the middle of a slide was
/// emitted before everything above it.
#[test]
fn test_graphic_frame_keeps_its_place_in_reading_order() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>
        <p:sp>
            <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="914400"/></a:xfrm></p:spPr>
            <p:txBody><a:bodyPr/><a:p><a:r><a:t>Quarterly report</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp>
            <p:nvSpPr><p:cNvPr id="3" name="Above"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="914400" y="1371600"/><a:ext cx="4572000" cy="457200"/></a:xfrm></p:spPr>
            <p:txBody><a:bodyPr/><a:p><a:r><a:t>Text above the table</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:graphicFrame>
            <p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
            <p:xfrm><a:off x="914400" y="2743200"/><a:ext cx="4572000" cy="740664"/></p:xfrm>
            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
                <a:tbl><a:tblGrid><a:gridCol w="2286000"/><a:gridCol w="2286000"/></a:tblGrid>
                    <a:tr h="370332"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Region</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Sales</a:t></a:r></a:p></a:txBody></a:tc></a:tr>
                    <a:tr h="370332"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>North</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>42</a:t></a:r></a:p></a:txBody></a:tc></a:tr>
                </a:tbl>
            </a:graphicData></a:graphic>
        </p:graphicFrame>
        <p:sp>
            <p:nvSpPr><p:cNvPr id="5" name="Below"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="914400" y="4572000"/><a:ext cx="4572000" cy="457200"/></a:xfrm></p:spPr>
            <p:txBody><a:bodyPr/><a:p><a:r><a:t>Text below the table</a:t></a:r></a:p></p:txBody>
        </p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

    let pptx = build_single_slide_pptx(slide_xml, None, &[]);
    let result = extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap();

    let at = |needle: &str| {
        result
            .content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {:?}", result.content))
    };
    assert!(
        at("Text above the table") < at("Region") && at("Region") < at("Text below the table"),
        "the table must stay between the text above and below it: {:?}",
        result.content
    );
}
