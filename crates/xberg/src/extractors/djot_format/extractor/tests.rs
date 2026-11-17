use super::*;
use crate::types::internal::ElementKind;

#[tokio::test]
async fn test_footnote_ref_follows_its_own_block_when_a_nested_block_closes_first() {
    // A list item and a definition push their text when they close, after the heading,
    // table and blockquote nested in them. Their references still follow them.
    let content = b"- parent[^a]\n\n  # Nested heading\n\n  | x | y |\n  |---|---|\n  | 1 | 2 |\n\n  > quoted\n\n- second[^b]\n\n: Term\n\n  definition[^c]\n\n  # Definition heading\n\n[^a]: Note a.\n\n[^b]: Note b.\n\n[^c]: Note c.\n";
    let doc = DjotExtractor::new()
        .extract_content(content, "text/djot", &ExtractionConfig::default())
        .await
        .expect("extraction should succeed");

    let mut cited = Vec::new();
    let mut last = None;
    for element in &doc.elements {
        if element.kind == ElementKind::FootnoteRef {
            let (kind, text): (ElementKind, String) = last.clone().expect("a reference follows a block");
            cited.push((element.text.clone(), kind, text));
        } else {
            last = Some((element.kind, element.text.clone()));
        }
    }
    assert_eq!(cited.len(), 3, "{cited:?}");
    let (ref name, kind, ref text) = cited[0];
    assert!(
        name == "a" && matches!(kind, ElementKind::ListItem { .. }) && text.starts_with("parent"),
        "{cited:?}"
    );
    let (ref name, kind, ref text) = cited[1];
    assert!(
        name == "b" && matches!(kind, ElementKind::ListItem { .. }) && text == "second",
        "{cited:?}"
    );
    let (ref name, kind, ref text) = cited[2];
    assert!(
        name == "c" && kind == ElementKind::DefinitionDescription && text == "definition",
        "{cited:?}"
    );
}

#[tokio::test]
async fn test_footnote_ref_of_a_caption_is_dropped_with_the_caption_of_an_empty_table() {
    // A table without rows is not pushed, so its caption is dropped. The reference in that
    // caption goes with it and does not land before the next table.
    let content = b"|---|\n^ dropped caption[^a]\n\n| x | y |\n|---|---|\n| 1 | 2 |\n\n[^a]: Note a.\n";
    let doc = DjotExtractor::new()
        .extract_content(content, "text/djot", &ExtractionConfig::default())
        .await
        .expect("extraction should succeed");

    let refs: Vec<_> = doc
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::FootnoteRef)
        .map(|element| element.text.clone())
        .collect();
    assert!(refs.is_empty(), "{refs:?} in {:?}", doc.elements);
}

#[tokio::test]
async fn test_dropping_an_empty_table_caption_keeps_the_references_of_other_blocks() {
    // The list item holds reference `a` until it closes, after the empty table inside it.
    // Only the caption reference `b` goes.
    let content = b"- item[^a]\n\n  |---|\n  ^ cap[^b]\n\n- second\n\n[^a]: A.\n\n[^b]: B.\n";
    let doc = DjotExtractor::new()
        .extract_content(content, "text/djot", &ExtractionConfig::default())
        .await
        .expect("extraction should succeed");

    let refs: Vec<_> = doc
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::FootnoteRef)
        .map(|element| element.text.as_str())
        .collect();
    assert_eq!(refs, ["a"], "{:?}", doc.elements);
}

#[test]
fn test_djot_extractor_creation() {
    let extractor = DjotExtractor::new();
    assert_eq!(extractor.name(), "djot-extractor");
}

#[test]
fn test_can_extract_djot_mime_types() {
    let extractor = DjotExtractor::new();
    let mime_types = extractor.supported_mime_types();

    assert!(mime_types.contains(&"text/djot"));
    assert!(mime_types.contains(&"text/x-djot"));
}

#[test]
fn test_plugin_interface() {
    let extractor = DjotExtractor::new();
    assert_eq!(extractor.author(), "Xberg Team");
    assert!(!extractor.version().is_empty());
    assert!(!extractor.description().is_empty());
}

#[tokio::test]
async fn test_extract_simple_djot() {
    let content =
        b"# Header\n\nThis is a paragraph with *bold* and _italic_ text.\n\n## Subheading\n\nMore content here.";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let result = extractor.extract_content(content, "text/djot", &config).await;
    assert!(result.is_ok());

    let result = result.unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
    assert!(result.content.contains("Header"));
    assert!(result.content.contains("This is a paragraph"));
    assert!(result.content.contains("bold"));
    assert!(result.content.contains("italic"));
}

/// Regression test for the bug where extracted tables never reached rendered output:
/// `extract_content` used to extract tables via a separate `extract_tables_from_events`
/// pass and re-push them with the raw, element-less `InternalDocument::push_table`, which
/// only records the table data without creating a matching `ElementKind::Table` element.
/// Since every renderer walks `doc.elements`, the table content was silently dropped. Tables
/// are now parsed in place inside `build_internal_document`, preserving both their document
/// position and producing a proper element.
#[tokio::test]
async fn test_djot_tables_render_in_output() {
    let content = b"Intro paragraph.\n\n| Name | Age |\n|------|-----|\n| Alice | 30 |\n\nOutro paragraph.";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let doc = extractor
        .extract_content(content, "text/djot", &config)
        .await
        .expect("extraction should succeed");

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells[0], vec!["Name", "Age"]);
    assert_eq!(doc.tables[0].cells[1], vec!["Alice", "30"]);

    let table_element_count = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::Table { .. }))
        .count();
    assert_eq!(
        table_element_count, 1,
        "expected exactly one Table element in {:?}",
        doc.elements
    );

    let result =
        crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Markdown);
    assert!(
        result.content.contains("Alice") && result.content.contains("30"),
        "table content missing from rendered output: {}",
        result.content
    );
    let intro_pos = result.content.find("Intro paragraph").expect("intro present");
    let table_pos = result.content.find("Alice").expect("table content present");
    let outro_pos = result.content.find("Outro paragraph").expect("outro present");
    assert!(
        intro_pos < table_pos && table_pos < outro_pos,
        "table should be positioned in document flow: {}",
        result.content
    );
}

#[tokio::test]
async fn test_trimmed_paragraph_with_emoji_djot() {
    let djot = "  *bold* \u{1F389} text  ".as_bytes();
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("Should handle emoji in trimmed djot paragraph");
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert!(result.content.contains("bold"), "Bold text preserved");
    assert!(result.content.contains("\u{1F389}"), "Emoji preserved after trim");
}

#[tokio::test]
async fn test_cjk_paragraph_with_formatting_djot() {
    let djot = "# CJK\n\nこれは*太字*テスト".as_bytes();
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("Should handle CJK with bold formatting");
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert!(result.content.contains("太字"), "Bold CJK content present");
    assert!(result.content.contains("これは"), "Leading CJK preserved");
}

#[tokio::test]
async fn test_image_uri_extraction_djot() {
    let djot = b"![A diagram](https://example.com/diagram.png)\n\nSome text.";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let doc = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("image djot should extract");

    let has_image_uri = doc.uris.iter().any(|u| u.url.contains("diagram.png"));
    assert!(has_image_uri, "image URI should be captured from Djot image node");
}

/// Collect `(text, ordered, depth)` for every `ListItem` element, in document order.
fn list_items(doc: &crate::types::internal::InternalDocument) -> Vec<(String, bool, u16)> {
    use crate::types::internal::ElementKind;
    doc.elements
        .iter()
        .filter_map(|e| match e.kind {
            ElementKind::ListItem { ordered } => Some((e.text.clone(), ordered, e.depth)),
            _ => None,
        })
        .collect()
}

/// A list item with two paragraphs: the second one's first word must not run into the
/// first one's last word.
#[tokio::test]
async fn should_separate_the_paragraphs_of_a_list_item_djot() {
    let djot = b"- one\n- two\n\n  continued paragraph in item\n- three\n";
    let doc = DjotExtractor::new()
        .extract_content(djot, "text/djot", &ExtractionConfig::default())
        .await
        .expect("djot list should extract");

    let items: Vec<String> = list_items(&doc).into_iter().map(|(text, _, _)| text).collect();
    assert_eq!(items, ["one", "two continued paragraph in item", "three"]);
}

/// Regression test for GH#1459 (Djot's own copy of the same bug): a nested list used
/// to silently lose every ancestor item's text. Only the deepest item ("L3") survived
/// because `Start(Container::ListItem)` unconditionally clears `list_item_text` with
/// no flush of the enclosing item's buffer at `Start(Container::List)`.
///
/// Against the unfixed code this assertion fails: `list_items(&doc).len()` is `1`
/// (only `[("L3", false, 3)]`) instead of the expected 3 levels.
#[tokio::test]
async fn test_nested_list_preserves_all_ancestor_text_djot() {
    // Djot requires a blank line between a list item's own content and a nested
    // sublist marker (unlike CommonMark, where it's optional) — see jotdown's
    // `parse_list_nest` test fixture, which this mirrors.
    let djot = b"- L1\n\n  - L2\n\n    - L3\n";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let doc = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("nested djot list should extract");

    let items = list_items(&doc);
    assert_eq!(
        items,
        vec![
            ("L1".to_string(), false, 1),
            ("L2".to_string(), false, 2),
            ("L3".to_string(), false, 3),
        ],
        "all three nesting levels must survive with increasing depth, got {items:?}"
    );
}

/// Regression test for GH#1459 (Djot): the parent item's text can arrive across
/// multiple jotdown events (a `Strong` span plus a soft-break-joined continuation
/// line) before the sublist starts. The flush at `Start(Container::List)` must carry
/// the whole accumulated buffer and its annotations, not just the most recent `Str`
/// event.
///
/// Against the unfixed code this assertion fails: `list_items(&doc)` contains only
/// `[("Child", false, 2)]` — the parent's text and its bold annotation are discarded
/// entirely when `Start(Container::ListItem)` for "Child" clears `list_item_text`.
#[tokio::test]
async fn test_nested_list_flushes_full_multi_event_parent_buffer_djot() {
    use crate::types::document_structure::AnnotationKind;

    // No lazy soft-break continuation line here (unlike the Markdown counterpart of
    // this test): jotdown's list-item paragraph continuation indentation rules are
    // not the same as CommonMark's, so this sticks to the single-line item content
    // that jotdown's own test fixtures use. The `Strong` span alone is enough to
    // prove the flush carries multiple events, not just the last `Str`.
    let djot = b"- Parent *bold* line\n\n  - Child\n";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let doc = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("nested djot list with formatting should extract");

    let items = list_items(&doc);
    assert_eq!(
        items,
        vec![
            ("Parent bold line".to_string(), false, 1),
            ("Child".to_string(), false, 2),
        ],
        "the parent's multi-event text (leading Str + Strong span + trailing Str) must \
         be flushed whole before descending into the sublist, got {items:?}"
    );

    let parent = doc
        .elements
        .iter()
        .find(|e| e.text == "Parent bold line")
        .expect("flushed parent item must be present");
    assert_eq!(
        parent.annotations.len(),
        1,
        "the bold annotation on \"bold\" must survive the flush, got {:?}",
        parent.annotations
    );
    let annotation = &parent.annotations[0];
    assert_eq!(annotation.kind, AnnotationKind::Bold);
    assert_eq!(
        &parent.text[annotation.start as usize..annotation.end as usize],
        "bold",
        "annotation byte range must still point at \"bold\" after the flush"
    );
}

/// Regression test for GH#1459 (Djot): trailing text after a sublist used to be
/// emitted as a bare `Paragraph` element instead of staying list-item content,
/// because `End(Container::ListItem)` for the inner item set the old boolean
/// `in_list_item` back to `false` while the outer item was still open, so the outer
/// item's trailing text hit the `Paragraph` start guard.
///
/// Against the unfixed code this assertion fails: `list_items(&doc)` contains only
/// `[("Child", false, 2)]` (the parent's own text is *also* lost, per the flush
/// defect above), and a `Paragraph` element with text `"Trailing"` exists in
/// `doc.elements` where none should.
#[tokio::test]
async fn test_trailing_text_after_sublist_stays_list_item_djot() {
    use crate::types::internal::ElementKind;

    // Blank line before the nested marker (djot requirement, see the earlier test's
    // comment); "Trailing" is indented to the OUTER item's content column (2), which
    // is less than the nested item's (4), so it closes the sublist and continues the
    // outer item rather than the inner one.
    let djot = b"- Parent\n\n  - Child\n\n  Trailing\n";
    let extractor = DjotExtractor::new();
    let config = ExtractionConfig::default();

    let doc = extractor
        .extract_content(djot, "text/djot", &config)
        .await
        .expect("djot list with trailing text should extract");

    let items = list_items(&doc);
    assert_eq!(
        items,
        vec![
            ("Parent".to_string(), false, 1),
            ("Child".to_string(), false, 2),
            ("Trailing".to_string(), false, 1),
        ],
        "trailing text after the sublist must become a sibling list item at the \
         parent's depth, got {items:?}"
    );

    let stray_paragraph = doc
        .elements
        .iter()
        .any(|e| matches!(e.kind, ElementKind::Paragraph) && e.text == "Trailing");
    assert!(
        !stray_paragraph,
        "trailing list-item text must not be emitted as a bare Paragraph element"
    );
}

/// Smart quotes, dashes, the ellipsis, a non-breaking space and a `:symbol:` keep their
/// characters in headings, paragraphs, list items and table cells.
#[tokio::test]
async fn should_keep_smart_punctuation_in_every_block_djot() {
    let djot = b"# Don't panic\n\nIt's \"fine\" -- mostly... or---not.\n\n'quoted' a\\ b :smile:\n\n- Here's an item\n\n| Cell |\n|------|\n| Here's a cell |\n";
    let doc = DjotExtractor::new()
        .extract_content(djot, "text/djot", &ExtractionConfig::default())
        .await
        .expect("djot with smart punctuation should extract");

    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    for expected in [
        "Don\u{2019}t panic",
        "It\u{2019}s \u{201C}fine\u{201D} \u{2013} mostly\u{2026} or\u{2014}not.",
        "\u{2018}quoted\u{2019} a\u{A0}b :smile:",
        "Here\u{2019}s an item",
    ] {
        assert!(texts.contains(&expected), "expected {expected:?} in {texts:?}");
    }
    assert_eq!(doc.tables[0].cells[1], vec!["Here\u{2019}s a cell"]);
}

/// A `^ caption` line under a table becomes a paragraph right after the table, linked to it as
/// its caption.
#[tokio::test]
async fn should_attach_a_table_caption_to_its_table_djot() {
    let djot =
        b"Before.\n\n| Name | Age |\n|------|-----|\n| Alice | 30 |\n\n^ Ages of the _team_\nmembers.\n\nAfter.\n";
    let doc = DjotExtractor::new()
        .extract_content(djot, "text/djot", &ExtractionConfig::default())
        .await
        .expect("djot table with caption should extract");

    let table_idx = doc
        .elements
        .iter()
        .position(|e| matches!(e.kind, ElementKind::Table { .. }))
        .expect("table element");
    let caption_idx = doc
        .elements
        .iter()
        .position(|e| e.text == "Ages of the team members.")
        .unwrap_or_else(|| panic!("caption paragraph missing from {:?}", doc.elements));
    assert!(
        doc.relationships.contains(&crate::types::internal::Relationship {
            source: caption_idx as u32,
            target: RelationshipTarget::Index(table_idx as u32),
            kind: RelationshipKind::Caption,
        }),
        "caption must point at its table, got {:?}",
        doc.relationships
    );
    let after_idx = doc
        .elements
        .iter()
        .position(|e| e.text == "After.")
        .expect("paragraph after");
    assert!(table_idx < caption_idx && caption_idx < after_idx);
}

/// A hard line break keeps its newline in table captions, list items, headings, definition terms
/// and definitions.
#[tokio::test]
async fn should_keep_hard_breaks_in_every_block_djot() {
    for djot in [
        &b"| Name |\n|------|\n| Alice |\n\n^ first\\\nsecond\n"[..],
        &b"- first\\\n  second\n"[..],
        &b"# first\\\nsecond\n"[..],
        &b": term\n\n  first\\\n  second\n"[..],
        &b": first\\\n  second\n\n  definition\n"[..],
    ] {
        let doc = DjotExtractor::new()
            .extract_content(djot, "text/djot", &ExtractionConfig::default())
            .await
            .expect("djot with a hard break should extract");

        let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
        assert!(
            texts.contains(&"first\nsecond"),
            "{:?} must keep its hard break, got {texts:?}",
            String::from_utf8_lossy(djot)
        );
    }
}

/// An image in a table caption or cell keeps its alt text as the image's text, as its URI label
/// and in the caption or cell text.
#[tokio::test]
async fn should_keep_image_alt_text_in_table_captions_and_cells_djot() {
    let djot = b"| Chart |\n|-------|\n| ![sales chart](chart.png) |\n\n^ Regions on the ![region map](map.png)\n";
    let doc = DjotExtractor::new()
        .extract_content(djot, "text/djot", &ExtractionConfig::default())
        .await
        .expect("djot table with images should extract");

    for (url, alt) in [("chart.png", "sales chart"), ("map.png", "region map")] {
        let uri = doc
            .uris
            .iter()
            .find(|u| u.url == url)
            .unwrap_or_else(|| panic!("{url} missing from {:?}", doc.uris));
        assert_eq!(uri.label.as_deref(), Some(alt), "label of {url}");
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Image { .. }) && e.text == alt),
            "image element with text {alt:?} missing from {:?}",
            doc.elements
        );
    }
    assert_eq!(doc.tables[0].cells[1], vec!["sales chart"]);
    assert!(
        doc.elements.iter().any(|e| e.text == "Regions on the region map"),
        "caption text must keep the alt text, got {:?}",
        doc.elements
    );
}

/// A footnote reference in a caption follows the caption paragraph, after the table.
#[tokio::test]
async fn should_place_a_caption_footnote_reference_after_the_caption_djot() {
    let djot = b"| a |\n\n^ Caption with note[^n]\n\n[^n]: the note\n";
    let doc = DjotExtractor::new()
        .extract_content(djot, "text/djot", &ExtractionConfig::default())
        .await
        .expect("djot table with a caption footnote should extract");

    let position = |found: &dyn Fn(&crate::types::internal::InternalElement) -> bool| {
        doc.elements
            .iter()
            .position(found)
            .unwrap_or_else(|| panic!("element missing from {:?}", doc.elements))
    };
    let table_idx = position(&|e| matches!(e.kind, ElementKind::Table { .. }));
    let caption_idx = position(&|e| e.text == "Caption with note");
    let reference_idx = position(&|e| matches!(e.kind, ElementKind::FootnoteRef));
    assert!(
        table_idx < caption_idx && caption_idx + 1 == reference_idx,
        "got {:?}",
        doc.elements
    );
}

/// A plain table gets no caption, and the caption of a table without rows does not go to the
/// next table. The reference in that caption is dropped with it.
#[tokio::test]
async fn should_give_a_caption_only_to_its_own_table_djot() {
    for djot in [
        &b"| a |\n"[..],
        &b"|---|\n\n^ stale caption[^n]\n\n| a |\n\n[^n]: the note\n"[..],
    ] {
        let doc = DjotExtractor::new()
            .extract_content(djot, "text/djot", &ExtractionConfig::default())
            .await
            .expect("djot table should extract");

        assert!(
            !doc.relationships.iter().any(|r| r.kind == RelationshipKind::Caption),
            "{:?} got {:?}",
            String::from_utf8_lossy(djot),
            doc.relationships
        );
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::Table { .. })),
            "table element missing from {:?}",
            doc.elements
        );
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::FootnoteRef)),
            "got {:?}",
            doc.elements
        );
    }
}

/// A soft break in a footnote definition is a space and a hard break is a newline.
#[tokio::test]
async fn should_keep_line_breaks_in_a_footnote_definition_djot() {
    let cases: [(&[u8], &str); 2] = [
        (b"Text[^a].\n\n[^a]: first\n    second\n", "first second"),
        (b"Text[^a].\n\n[^a]: first\\\n    second\n", "first\nsecond"),
    ];
    for (content, expected) in cases {
        let doc = DjotExtractor::new()
            .extract_content(content, "text/djot", &ExtractionConfig::default())
            .await
            .expect("djot with a two-line footnote should extract");
        let definition = doc
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::FootnoteDefinition)
            .expect("the footnote definition is extracted");
        assert_eq!(definition.text, expected, "got {:?}", doc.elements);
    }
}

#[tokio::test]
async fn should_keep_an_image_alt_break_out_of_the_footnote_text_djot() {
    let cases: [(&[u8], &str); 2] = [
        (
            b"Text[^a].\n\n[^a]: see ![first\n    second](x.png) end\n",
            "first second",
        ),
        (
            b"Text[^a].\n\n[^a]: see ![first\\\n    second](x.png) end\n",
            "first\nsecond",
        ),
    ];
    for (content, expected_alt) in cases {
        let doc = DjotExtractor::new()
            .extract_content(content, "text/djot", &ExtractionConfig::default())
            .await
            .expect("djot with an image in a footnote should extract");
        let definition = doc
            .elements
            .iter()
            .find(|e| e.kind == ElementKind::FootnoteDefinition)
            .expect("the footnote definition is extracted");
        assert_eq!(definition.text, "see  end", "got {:?}", doc.elements);
        let image = doc
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Image { .. }))
            .expect("the image is extracted");
        assert_eq!(image.text, expected_alt, "got {:?}", doc.elements);
    }
}
