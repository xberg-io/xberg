use super::*;

#[test]
fn markdown_ranges_use_original_utf8_bytes_across_lines() {
    let text = "éx<sup>2</sup>\n\nα<u>n</u>";
    let ranges = standard_inline_html_ranges(text).expect("canonical generated tags");
    assert_eq!(ranges, [3..8, 9..15, 19..22, 23..27]);
    assert_eq!(
        ranges.iter().map(|range| &text[range.clone()]).collect::<Vec<_>>(),
        ["<sup>", "</sup>", "<u>", "</u>"]
    );
}

#[test]
fn literal_code_unknown_attributed_and_unbalanced_tags_are_not_exempt() {
    for text in [
        r"\<sup>2\</sup>",
        "`<sup>`2`</sup>`",
        "<sup class=\"source\">2</sup>",
        "<unknown>2</unknown>",
        "x<sup>a<u>b</u> tail",
    ] {
        assert_eq!(standard_inline_html_ranges(text), Some(Vec::new()), "{text}");
    }
}

#[test]
fn unverifiable_tag_source_positions_are_rejected() {
    let arena = Arena::new();
    let node = arena.alloc(AstNode::from(NodeValue::HtmlInline("<sup>".into())));
    node.data.borrow_mut().sourcepos = (1, 1, 1, 5).into();
    assert_eq!(original_tag_range(node, "x<sup>", &[0]), None);
    node.data.borrow_mut().sourcepos = (1, 2, 1, 6).into();
    assert_eq!(original_tag_range(node, "x<sup>", &[0]), Some(1..6));
}
