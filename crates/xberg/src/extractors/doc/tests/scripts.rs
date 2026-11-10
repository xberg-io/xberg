use super::*;
use crate::types::builder;

#[test]
fn should_project_script_annotations_across_inline_picture_splits() {
    let picture = DocPicture {
        paragraph_index: 0,
        text_offset: 2,
        standalone: false,
        image: crate::types::ExtractedImage {
            format: "png".into(),
            ..Default::default()
        },
    };
    let mut doc = InternalDocument::new("doc");
    push_paragraph_runs(
        &mut doc,
        ElementKind::Paragraph,
        "é9",
        &[builder::superscript(0, 3)],
        &[picture],
        true,
    );
    assert_eq!(doc.elements.len(), 3);
    assert_eq!(doc.elements[0].text, "é");
    assert_eq!(doc.elements[0].annotations, [builder::superscript(0, 2)]);
    assert_eq!(doc.elements[2].text, "9");
    assert_eq!(doc.elements[2].annotations, [builder::superscript(0, 1)]);
}

#[test]
fn should_preserve_script_annotations_on_list_items() {
    let mut doc = InternalDocument::new("doc");
    let mut open = Vec::new();
    let list = crate::extraction::doc::DocListMembership {
        level: 0,
        ordered: true,
        list_id: 0,
        ordinal: None,
    };
    push_list_paragraph(&mut doc, &mut open, list, "109", &[builder::superscript(2, 3)]);
    assert_eq!(doc.elements.len(), 2);
    assert_eq!(doc.elements[1].kind, ElementKind::ListItem { ordered: true });
    assert_eq!(doc.elements[1].text, "109");
    assert_eq!(doc.elements[1].annotations, [builder::superscript(2, 3)]);
}
