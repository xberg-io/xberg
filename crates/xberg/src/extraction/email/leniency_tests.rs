use super::*;

fn assert_empty_headerless_result(result: &EmailExtractionResult) {
    assert!(result.subject.is_none());
    assert!(result.from_email.is_none());
    assert!(result.to_emails.is_empty());
    assert!(result.cc_emails.is_empty());
    assert!(result.bcc_emails.is_empty());
    assert!(result.date.is_none());
    assert!(result.message_id.is_none());
    assert!(result.plain_text.is_none());
    assert!(result.html_content.is_none());
    assert!(result.content.is_empty());
    assert!(result.attachments.is_empty());
    assert!(result.metadata.is_empty());
}

#[test]
fn should_preserve_empty_results_for_legacy_unterminated_headerless_inputs() {
    for raw in [
        b"not an email".as_slice(),
        b"first line\nlast line",
        b"<p>Hello</p>",
        b"\xff\0\x80",
        b": trailing",
    ] {
        let result = parse_eml_content(raw).expect("legacy accepted input without an actual body");
        assert_empty_headerless_result(&result);
    }
}

#[test]
fn should_keep_legacy_rejection_and_public_validation_boundaries() {
    for raw in [
        b"not an email\n".as_slice(),
        b"not an email\r\n",
        b"first line\nlast line\n",
        b"\xff\0\x80\n",
        b" \t",
        b"::",
        b"",
    ] {
        assert!(
            matches!(parse_eml_content(raw), Err(XbergError::Parsing { .. })),
            "{raw:?}"
        );
    }
    assert!(matches!(
        extract_email_content(b"", "message/rfc822", None),
        Err(XbergError::Validation { .. })
    ));
    assert!(matches!(
        extract_email_content(b"plain", "application/pdf", None),
        Err(XbergError::Validation { .. })
    ));
}

#[test]
fn should_keep_header_only_messages_without_an_invented_empty_body() {
    let result = parse_eml_content(b"X-Custom: value").expect("actual header without a body separator");
    assert_empty_headerless_result(&result);
}

#[test]
fn should_extract_headerless_body_after_an_actual_blank_line() {
    for raw in [b"\r\nBody text".as_slice(), b" \r\nBody text"] {
        let result = parse_eml_content(raw).expect("real empty header section");
        assert_eq!(result.plain_text.as_deref(), Some("Body text"));
        assert_eq!(result.content, "Body text");
        assert!(result.html_content.is_none());
        assert!(result.subject.is_none());
        assert!(result.from_email.is_none());
        assert!(result.attachments.is_empty());
    }
}

#[test]
fn should_preserve_declared_html_body_and_raw_content_type() {
    let result =
        parse_eml_content(b"Content-Type: text/html\r\n\r\n<p>Hello</p>").expect("minimal declared HTML message");
    assert_eq!(result.html_content.as_deref(), Some("<p>Hello</p>"));
    assert_eq!(result.content, "Hello");
    assert_eq!(
        result.metadata.get("content_type").map(String::as_str),
        Some("text/html")
    );
    assert!(result.subject.is_none());
    assert!(result.from_email.is_none());
    assert!(result.attachments.is_empty());
}
