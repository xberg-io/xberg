use super::*;

fn extracted_slide_content(shapes: &str) -> String {
    let slide_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>{shapes}</p:spTree></p:cSld>
</p:sld>"#
    );
    let pptx = build_single_slide_pptx(&slide_xml, None, &[]);
    extract_pptx_from_bytes(
        &pptx,
        &PptxExtractionOptions {
            extract_images: false,
            ..Default::default()
        },
        &mut Vec::new(),
    )
    .unwrap()
    .content
}

fn text_box(id: u32, x: i64, y: i64, cx: i64, cy: i64, body: &str) -> String {
    format!(
        r#"<p:sp>
            <p:nvSpPr><p:cNvPr id="{id}" name="Box {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm></p:spPr>
            <p:txBody><a:bodyPr/>{body}</p:txBody>
        </p:sp>"#
    )
}

/// PowerPoint splits a word into several runs when part of it changes language or
/// formatting; the runs are contiguous text, so no space may appear between them.
#[test]
fn test_runs_splitting_a_word_join_without_a_space() {
    let content = extracted_slide_content(&text_box(
        2,
        914400,
        1371600,
        4572000,
        914400,
        r#"<a:p><a:r><a:rPr lang="en-GB"/><a:t>Sum</a:t></a:r><a:r><a:rPr lang="en-CH"/><a:t>mary info:</a:t></a:r></a:p>
           <a:p><a:r><a:t>I</a:t></a:r><a:r><a:t>1 and </a:t></a:r><a:r><a:t>more</a:t></a:r></a:p>"#,
    ));

    assert!(content.contains("Summary info:"), "{content:?}");
    assert!(content.contains("I1 and more"), "{content:?}");
}

fn text_run(text: &str, formatting: elements::Formatting) -> Run {
    Run {
        text: text.to_string(),
        formatting,
        hyperlink_id: None,
        math_latex: None,
    }
}

fn math_run(latex: &str) -> Run {
    Run {
        math_latex: Some((latex.to_string(), false)),
        ..text_run("", elements::Formatting::default())
    }
}

/// An empty run between a math run and text leaves the math run the one the text follows.
#[test]
fn test_an_empty_run_after_math_keeps_the_space_before_text() {
    let runs = [
        math_run("x"),
        text_run("", elements::Formatting::default()),
        text_run("b", elements::Formatting::default()),
    ];
    assert_eq!(join_runs_with_spacing(&runs, Run::extract), "x b");
}

/// A table cell renders on one line, so its paragraphs still need a separator once
/// runs stop being spaced apart.
#[test]
fn test_table_cell_paragraphs_stay_separated() {
    let content = extracted_slide_content(
        r#"<p:graphicFrame>
            <p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
            <p:xfrm><a:off x="914400" y="914400"/><a:ext cx="4572000" cy="740664"/></p:xfrm>
            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
                <a:tbl><a:tblGrid><a:gridCol w="4572000"/></a:tblGrid>
                    <a:tr h="370332"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>North</a:t></a:r></a:p><a:p><a:r><a:t>East</a:t></a:r></a:p></a:txBody></a:tc></a:tr>
                </a:tbl>
            </a:graphicData></a:graphic>
        </p:graphicFrame>"#,
    );

    assert!(content.contains("North East"), "{content:?}");
}

/// Shapes side by side read left to right even when the right one starts a little
/// higher; a shape below them still comes after both.
#[test]
fn test_side_by_side_shapes_read_left_to_right() {
    let shapes = [
        text_box(
            2,
            6180463,
            1344058,
            4076241,
            3877937,
            "<a:p><a:r><a:t>Right column</a:t></a:r></a:p>",
        ),
        text_box(
            3,
            838200,
            1825625,
            3623631,
            2136775,
            "<a:p><a:r><a:t>Left column</a:t></a:r></a:p>",
        ),
        text_box(
            4,
            838200,
            5600000,
            3623631,
            457200,
            "<a:p><a:r><a:t>Footer below</a:t></a:r></a:p>",
        ),
    ]
    .concat();
    let content = extracted_slide_content(&shapes);

    let at = |needle: &str| {
        content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {content:?}"))
    };
    assert!(at("Left column") < at("Right column"), "{content:?}");
    assert!(at("Right column") < at("Footer below"), "{content:?}");
}

/// A full-slide shape overlaps everything but covers too little of each shape's row to
/// join it, so the shapes on top of it keep their top-to-bottom order.
#[test]
fn test_full_slide_shape_does_not_merge_stacked_rows() {
    let shapes = [
        text_box(2, 0, 0, 12192000, 6858000, "<a:p><a:r><a:t>Backdrop</a:t></a:r></a:p>"),
        text_box(
            3,
            3000000,
            1000000,
            4000000,
            600000,
            "<a:p><a:r><a:t>Upper text</a:t></a:r></a:p>",
        ),
        text_box(
            4,
            900000,
            3000000,
            4000000,
            600000,
            "<a:p><a:r><a:t>Lower text</a:t></a:r></a:p>",
        ),
    ]
    .concat();
    let content = extracted_slide_content(&shapes);

    let at = |needle: &str| {
        content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {content:?}"))
    };
    assert!(at("Upper text") < at("Lower text"), "{content:?}");
}
