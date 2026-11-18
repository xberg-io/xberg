use super::*;

const NESTED_MESSAGE_EML: &[u8] = b"From: digest@example.com\r\n\
Subject: Digest\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/digest; boundary=digest\r\n\
\r\n\
--digest\r\n\
Content-Type: message/rfc822\r\n\
\r\n\
From: child@example.com\r\n\
Subject: Nested\r\n\
\r\n\
Nested body\r\n\
--digest--\r\n";

#[cfg(feature = "tokio-runtime")]
const IMPLICIT_NESTED_MESSAGE_EML: &[u8] = b"From: digest@example.com\r\n\
Subject: Digest\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/digest; boundary=digest\r\n\
\r\n\
--digest\r\n\
\r\n\
From: child@example.com\r\n\
Subject: Implicit Nested\r\n\
\r\n\
Implicit nested body\r\n\
--digest--\r\n";

#[cfg(feature = "tokio-runtime")]
const TEXT_ATTACHMENT_EML: &[u8] = b"From: Alice <alice@example.com>\r\n\
Subject: Attachment\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=part\r\n\
\r\n\
--part\r\n\
Content-Type: text/plain\r\n\
\r\n\
Parent body\r\n\
--part\r\n\
Content-Type: text/plain\r\n\
Content-Disposition: attachment; filename=note.txt\r\n\
\r\n\
Attachment body\r\n\
--part--\r\n";

#[test]
fn test_email_extractor_plugin_interface() {
    let extractor = EmailExtractor::new();
    assert_eq!(extractor.name(), "email-extractor");
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[test]
fn test_email_extractor_supported_mime_types() {
    let extractor = EmailExtractor::new();
    let mime_types = extractor.supported_mime_types();
    assert_eq!(mime_types.len(), 2);
    assert!(mime_types.contains(&"message/rfc822"));
    assert!(mime_types.contains(&"application/vnd.ms-outlook"));
}

#[test]
fn test_email_extractor_uses_config() {
    use crate::core::config::EmailConfig;

    let config = ExtractionConfig {
        email: Some(EmailConfig {
            msg_fallback_codepage: Some(1251),
        }),
        ..Default::default()
    };
    let extractor = EmailExtractor::new();
    let result = extractor.extract_sync(b"", "application/vnd.ms-outlook", &config);
    assert!(result.is_err());
}

#[test]
fn test_sender_name_and_address_are_rendered_once() {
    let eml = b"From: Alice Example <alice@example.com>\r\nSubject: Sender\r\n\r\nBody";
    let doc = EmailExtractor::new()
        .extract_sync(eml, "message/rfc822", &ExtractionConfig::default())
        .unwrap();
    let from_values: Vec<&str> = doc
        .elements
        .iter()
        .filter(|element| matches!(element.kind, crate::types::internal::ElementKind::MetadataBlock))
        .flat_map(|element| element.attributes.iter())
        .flat_map(|attributes| attributes.values())
        .map(String::as_str)
        .filter(|value| value.contains("alice@example.com"))
        .collect();

    assert_eq!(from_values, vec!["Alice Example <alice@example.com>"]);
    let crate::types::FormatMetadata::Email(email) = doc.metadata.format.unwrap() else {
        panic!("expected email metadata");
    };
    assert_eq!(email.from_name.as_deref(), Some("Alice Example"));
    assert_eq!(email.from_email.as_deref(), Some("alice@example.com"));
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_msg_preserves_sync_output_prefix_when_extracting_attachments() {
    let Some(data) = crate::utils::read_test_fixture("email/attachment.msg") else {
        return;
    };
    let config = ExtractionConfig {
        max_archive_depth: 1,
        ..Default::default()
    };
    let extractor = EmailExtractor::new();

    let sync_doc = extractor
        .extract_sync(&data, "application/vnd.ms-outlook", &config)
        .unwrap();
    let async_doc = extractor
        .extract_content(&data, "application/vnd.ms-outlook", &config)
        .await
        .unwrap();

    assert_eq!(&async_doc.elements[..sync_doc.elements.len()], sync_doc.elements);
    assert_eq!(
        serde_json::to_value(&async_doc.metadata).unwrap(),
        serde_json::to_value(&sync_doc.metadata).unwrap()
    );
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_preserves_extractable_attachment_children() {
    let config = ExtractionConfig {
        max_archive_depth: 1,
        ..Default::default()
    };

    let doc = EmailExtractor::new()
        .extract_content(TEXT_ATTACHMENT_EML, "message/rfc822", &config)
        .await
        .unwrap();
    let plain = crate::rendering::render_plain(&doc);
    let markdown = crate::rendering::render_markdown(&doc);
    let children = doc.children.as_ref().expect("text attachment should be extracted");

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].path, "note.txt");
    assert!(children[0].result.content.contains("Attachment body"));
    assert!(markdown.contains("## note.txt"));
    assert!(plain.contains(children[0].result.content.trim()));
}

/// The PDF in this fixture is base64 with a stray `>`. Its still-encoded text (`JVBERi0x…`)
/// must not come out as the attachment's content.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_does_not_extract_undecoded_base64_as_attachment_text() {
    let data = crate::utils::read_test_fixture("email/mailgun_pdf_attachment.eml")
        .expect("mailgun attachment fixture must be present");
    let doc = EmailExtractor::new()
        .extract_content(&data, "message/rfc822", &ExtractionConfig::default())
        .await
        .unwrap();
    let plain = crate::rendering::render_plain(&doc);

    assert!(plain.contains("Test - body content"), "got: {plain:?}");
    assert!(
        !plain.contains("JVBERi0x"),
        "the attachment's base64 leaked into the text: {plain:?}"
    );
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_skips_attachment_inline_when_combined_content_exceeds_limit() {
    let config = ExtractionConfig {
        max_archive_depth: 1,
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_content_size: 100,
            ..Default::default()
        }),
        ..Default::default()
    };

    let doc = EmailExtractor::new()
        .extract_content(TEXT_ATTACHMENT_EML, "message/rfc822", &config)
        .await
        .expect("parent email should survive an over-budget inline attachment");
    let plain = crate::rendering::render_plain(&doc);
    let children = doc.children.as_ref().expect("attachment child must be retained");

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].path, "note.txt");
    assert_eq!(children[0].result.content.trim(), "Attachment body");
    assert!(plain.contains("Parent body"));
    assert!(!plain.contains("Attachment body"));
    assert_eq!(doc.processing_warnings.len(), 1);
    assert_eq!(doc.processing_warnings[0].source, "email_attachment_inlining");
    assert!(doc.processing_warnings[0].message.contains("note.txt"));
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_preserves_nested_message_child() {
    let config = ExtractionConfig {
        max_archive_depth: 1,
        ..Default::default()
    };

    let doc = EmailExtractor::new()
        .extract_content(NESTED_MESSAGE_EML, "message/rfc822", &config)
        .await
        .unwrap();
    let children = doc.children.expect("nested message should be extracted");

    assert_eq!(children.len(), 1, "nested message must be emitted exactly once");
    let nested = &children[0];
    assert_eq!(nested.path, "nested_message_0.eml");
    assert_eq!(nested.mime_type, "message/rfc822");
    assert!(nested.result.content.contains("Nested body"));
    assert!(!doc.elements.iter().any(|element| {
        matches!(element.kind, crate::types::internal::ElementKind::Heading { level: 2 }) && element.text == nested.path
    }));
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_digest_emits_implicit_nested_message_once() {
    let config = ExtractionConfig {
        max_archive_depth: 1,
        ..Default::default()
    };
    let parsed = crate::extraction::email::parse_eml_content(IMPLICIT_NESTED_MESSAGE_EML).unwrap();

    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].mime_type.as_deref(), Some("message/rfc822"));

    let doc = EmailExtractor::new()
        .extract_content(IMPLICIT_NESTED_MESSAGE_EML, "message/rfc822", &config)
        .await
        .unwrap();
    let children = doc.children.expect("implicit nested message should be extracted");

    assert_eq!(
        children.len(),
        1,
        "implicit nested message must be emitted exactly once"
    );
    assert_eq!(children[0].path, "nested_message_0.eml");
    assert_eq!(children[0].mime_type, "message/rfc822");
    assert!(children[0].result.content.contains("Implicit nested body"));
}

#[test]
fn test_sync_email_preserves_nested_attachment_payload() {
    let parsed = crate::extraction::email::parse_eml_content(NESTED_MESSAGE_EML).unwrap();

    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].mime_type.as_deref(), Some("message/rfc822"));
    let data = parsed.attachments[0]
        .data
        .as_deref()
        .expect("sync extraction must retain nested attachment bytes");
    assert!(String::from_utf8_lossy(data).contains("Nested body"));
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_skips_nested_payloads_at_depth_zero() {
    let config = ExtractionConfig {
        max_archive_depth: 0,
        ..Default::default()
    };

    let doc = EmailExtractor::new()
        .extract_content(NESTED_MESSAGE_EML, "message/rfc822", &config)
        .await
        .unwrap();

    assert!(doc.children.is_none(), "depth zero must not emit nested children");
    assert!(
        doc.processing_warnings.is_empty(),
        "depth zero is an intentional recursion limit"
    );
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn test_async_email_rejects_nested_payload_before_copying_over_cap() {
    let config = ExtractionConfig {
        max_archive_depth: 1,
        max_embedded_file_bytes: Some(8),
        ..Default::default()
    };
    let parsed = crate::extraction::email::extract_email_content_with_nested(
        NESTED_MESSAGE_EML,
        "message/rfc822",
        None,
        config.max_embedded_file_bytes,
        &crate::extractors::security::SecurityLimits::default(),
    )
    .unwrap();

    assert!(parsed.result.attachments[0].data.is_none());
    assert!(parsed.nested_messages[0].data.is_none());

    let doc = EmailExtractor::new()
        .extract_content(NESTED_MESSAGE_EML, "message/rfc822", &config)
        .await
        .unwrap();

    assert!(doc.children.is_none(), "over-cap nested messages must not be extracted");
    assert_eq!(doc.processing_warnings.len(), 1);
    assert_eq!(doc.processing_warnings[0].source, "nested_message_extraction");
    assert!(doc.processing_warnings[0].message.contains("exceeds cap 8 bytes"));
}

#[test]
fn test_email_with_table_and_list_preservation() {
    let eml = r#"From: Alice <alice@example.com>
To: Bob <bob@example.com>
Subject: Table and List Repro
Content-Type: multipart/alternative; boundary="boundary"

--boundary
Content-Type: text/plain; charset=utf-8

Plain text fallback.

--boundary
Content-Type: text/html; charset=utf-8

<html>
<body>
    <p>Introduction.</p>
    <table>
        <tr><th>Name</th><th>Value</th></tr>
        <tr><td>Key</td><td>123</td></tr>
    </table>
    <ol>
        <li>First item</li>
        <li>Second item</li>
    </ol>
    <p>Conclusion.</p>
</body>
</html>
--boundary--
"#;
    let extractor = EmailExtractor::new();
    let config = ExtractionConfig::default();
    let doc = extractor
        .extract_sync(eml.as_bytes(), "message/rfc822", &config)
        .unwrap();

    let has_table = doc
        .elements
        .iter()
        .any(|e| matches!(e.kind, crate::types::internal::ElementKind::Table { .. }));
    let list_item_count = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::ListItem { .. }))
        .count();
    let paragraph_count = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::Paragraph))
        .count();

    assert!(has_table, "Table element should be present");
    assert_eq!(list_item_count, 2, "Should have 2 list items");
    assert_eq!(
        paragraph_count, 2,
        "Should have exactly 2 body paragraphs (no duplicates)"
    );
}

/// Regression test for task #727, from the consumer side: `process_node` already
/// forwards `node.annotations` for list items, so the only reason bold text in an HTML
/// `<li>` renders flat is that the HTML structure builder never attached it.
///
/// Against the unfixed code the rendered line is `- alpha bold`.
#[test]
fn test_html_list_item_formatting_survives_into_markdown() {
    let eml = r#"From: Alice <alice@example.com>
To: Bob <bob@example.com>
Subject: Formatted list
Content-Type: multipart/alternative; boundary="boundary"

--boundary
Content-Type: text/plain; charset=utf-8

Plain text fallback.

--boundary
Content-Type: text/html; charset=utf-8

<html><body><ul><li>alpha <strong>bold</strong></li></ul></body></html>
--boundary--
"#;
    let doc = EmailExtractor::new()
        .extract_sync(eml.as_bytes(), "message/rfc822", &ExtractionConfig::default())
        .unwrap();
    let markdown = crate::rendering::render_markdown(&doc);

    let line = markdown
        .lines()
        .find(|line| line.contains("alpha"))
        .expect("expected the list item in the rendered markdown");
    assert!(
        line.contains("**bold**"),
        "list-item formatting must reach the renderer, got line {line:?}; full markdown:\n{markdown}"
    );
}

/// Regression test for task #728, from the consumer side: `process_node` walks the
/// `DocumentStructure` in *tree* order, so a sublist parented at the root instead of
/// under its `<li>` is emitted after the whole outer list — putting the parent item's
/// trailing text ahead of the sublist it follows in the source.
///
/// Against the unfixed code the item texts come out as `["parent", "tail", "child"]`,
/// all at depth 1 (`process_node`'s recursion into `ListItem` children never runs,
/// because HTML-derived list items have no children).
#[test]
fn test_html_sublist_is_rendered_inside_its_parent_item() {
    let eml = r#"From: Alice <alice@example.com>
To: Bob <bob@example.com>
Subject: Nested list
Content-Type: multipart/alternative; boundary="boundary"

--boundary
Content-Type: text/plain; charset=utf-8

Plain text fallback.

--boundary
Content-Type: text/html; charset=utf-8

<html><body><ul><li>parent<ul><li>child</li></ul>tail</li></ul></body></html>
--boundary--
"#;
    let doc = EmailExtractor::new()
        .extract_sync(eml.as_bytes(), "message/rfc822", &ExtractionConfig::default())
        .unwrap();

    let items: Vec<(&str, u16)> = doc
        .elements
        .iter()
        .filter(|element| matches!(element.kind, crate::types::internal::ElementKind::ListItem { .. }))
        .map(|element| (element.text.as_str(), element.depth))
        .collect();

    assert_eq!(
        items.iter().map(|(text, _)| *text).collect::<Vec<_>>(),
        vec!["parent", "child", "tail"],
        "the sublist must be emitted between its parent item and that item's trailing text"
    );
    assert_eq!(
        items.iter().map(|(_, depth)| *depth).collect::<Vec<_>>(),
        vec![1, 2, 1],
        "the sublist's item must sit one level deeper than the outer list's items"
    );
}

/// An HTML part as Outlook writes it: a `Generator` meta tag, a style sheet in an HTML
/// comment and a `<title>` in the head. Against the unfixed code the style sheet, the
/// script, `Generator: Microsoft Word 15 (filtered medium)` and the title all came out as
/// paragraphs of the message.
#[test]
fn test_html_head_and_script_are_not_message_text() {
    let eml = r#"From: Alice <alice@example.com>
To: Bob <bob@example.com>
Subject: Quarterly figures
Content-Type: multipart/alternative; boundary="boundary"

--boundary
Content-Type: text/plain; charset=utf-8

Hello Bob, the figures are attached.

--boundary
Content-Type: text/html; charset=utf-8

<html><head><meta name="Generator" content="Microsoft Word 15 (filtered medium)"><style><!--
/* Font Definitions */
@font-face {font-family:"Cambria Math";}
p.MsoNormal {margin:0cm; font-size:11.0pt;}
--></style><title>Quarterly figures</title></head>
<body><p class=MsoNormal>Hello Bob, the figures are attached.</p><script>track();</script><style>.footer {color: grey;}</style></body></html>
--boundary--
"#;
    let doc = EmailExtractor::new()
        .extract_sync(eml.as_bytes(), "message/rfc822", &ExtractionConfig::default())
        .unwrap();

    for text in [
        crate::rendering::render_plain(&doc),
        crate::rendering::render_markdown(&doc),
    ] {
        assert!(text.contains("Hello Bob, the figures are attached."), "got: {text:?}");
        for leaked in [
            "font-family",
            "Cambria Math",
            "Generator",
            "Microsoft Word",
            "track();",
            ".footer",
        ] {
            assert!(
                !text.contains(leaked),
                "{leaked:?} leaked into the message text: {text:?}"
            );
        }
        assert_eq!(
            text.matches("Quarterly figures").count(),
            1,
            "the subject header only, not the HTML title: {text:?}"
        );
    }
}

#[test]
fn test_html_head_boundaries_in_raw_content_do_not_duplicate_subject() {
    for hidden in [
        r#"<script>const boundary = "<body>";</script>"#,
        r#"<style>p::before { content: "</head>"; }</style>"#,
        "<!-- <body> </head> -->",
    ] {
        let eml = format!(
            "From: Alice <alice@example.com>\r\nSubject: Quarterly figures\r\n\
             Content-Type: text/html; charset=utf-8\r\n\r\n\
             <html><head>{hidden}<title>Quarterly figures</title></head>\
             <body><p>Hello Bob, the figures are attached.</p></body></html>"
        );
        let doc = EmailExtractor::new()
            .extract_sync(eml.as_bytes(), "message/rfc822", &ExtractionConfig::default())
            .unwrap();
        for text in [
            crate::rendering::render_plain(&doc),
            crate::rendering::render_markdown(&doc),
        ] {
            assert_eq!(
                text.matches("Hello Bob, the figures are attached.").count(),
                1,
                "{text:?}"
            );
            assert_eq!(text.matches("Quarterly figures").count(), 1, "{hidden:?}: {text:?}");
            assert!(!text.contains("boundary"), "{text:?}");
            assert!(!text.contains("p::before"), "{text:?}");
        }
    }
}

#[test]
fn test_content_size_guard_fires_when_limit_exceeded() {
    use crate::extractors::security::SecurityLimits;

    let long_body = "x".repeat(1000);
    let eml = format!("From: sender@example.com\r\nSubject: Big\r\n\r\n{}", long_body);

    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_content_size: 10,
            ..SecurityLimits::default()
        }),
        ..Default::default()
    };

    let extractor = EmailExtractor::new();
    let result = extractor.extract_sync(eml.as_bytes(), "message/rfc822", &config);

    assert!(
        result.is_err(),
        "extraction must fail when content exceeds max_content_size"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("security") || err_msg.to_lowercase().contains("content"),
        "error must mention security or content: {}",
        err_msg
    );
}

#[test]
fn test_content_size_guard_passes_for_normal_email() {
    let eml = "From: sender@example.com\r\nSubject: Hi\r\n\r\nHello world.";
    let config = ExtractionConfig::default();
    let extractor = EmailExtractor::new();
    let result = extractor.extract_sync(eml.as_bytes(), "message/rfc822", &config);
    assert!(result.is_ok(), "normal email must not be rejected: {:?}", result.err());
}
