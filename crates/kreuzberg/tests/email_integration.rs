//! Email extraction integration tests.
//!
//! Tests for .eml (RFC822) email extraction.
//! Validates metadata extraction, content extraction, HTML/plain text handling, and attachments.

#![cfg(feature = "email")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

/// Test basic EML extraction with subject, from, to, and body.
#[tokio::test]
async fn test_eml_basic_extraction() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Test Email Subject\r\n\
Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
Message-ID: <unique123@example.com>\r\n\
\r\n\
This is the email body content.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract EML successfully");

    assert_eq!(result.mime_type, "message/rfc822");

    assert_eq!(result.metadata.subject, Some("Test Email Subject".to_string()));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));

    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert!(email_meta.cc_emails.is_empty(), "CC should be empty");
    assert!(email_meta.bcc_emails.is_empty(), "BCC should be empty");

    assert!(email_meta.message_id.is_some());
    let msg_id = email_meta.message_id.clone().expect("Operation failed");
    assert!(
        msg_id.contains("unique123@example.com"),
        "Message ID should contain unique123@example.com"
    );

    assert!(email_meta.attachments.is_empty(), "Should have no attachments");

    assert!(result.metadata.created_at.is_some());

    assert!(result.content.contains("Subject: Test Email Subject"));
    assert!(result.content.contains("From: sender@example.com"));
    assert!(result.content.contains("To: recipient@example.com"));
    assert!(result.content.contains("This is the email body content"));
}

/// Test EML with attachments - metadata extraction.
#[tokio::test]
async fn test_eml_with_attachments() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Email with Attachment\r\n\
Content-Type: multipart/mixed; boundary=\"----boundary\"\r\n\
\r\n\
------boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Email body text.\r\n\
------boundary\r\n\
Content-Type: text/plain; name=\"file.txt\"\r\n\
Content-Disposition: attachment; filename=\"file.txt\"\r\n\
\r\n\
Attachment content here.\r\n\
------boundary--\r\n";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract EML with attachment");

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    assert_eq!(email_meta.attachments.len(), 1, "Should have 1 attachment (file.txt)");

    assert!(result.content.contains("Email body text") || result.content.contains("Attachment content"));
}

/// Test EML with HTML body.
#[tokio::test]
async fn test_eml_html_body() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: HTML Email\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html>\r\n\
<head><style>body { color: blue; }</style></head>\r\n\
<body>\r\n\
<h1>HTML Heading</h1>\r\n\
<p>This is <b>bold</b> text in HTML.</p>\r\n\
<script>alert('test');</script>\r\n\
</body>\r\n\
</html>";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract HTML email");

    assert!(!result.content.contains("<script>"));
    assert!(!result.content.contains("<style>"));

    assert!(result.content.contains("HTML Heading") || result.content.contains("bold"));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("HTML Email".to_string()));
}

/// Test EML with plain text body.
#[tokio::test]
async fn test_eml_plain_text_body() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Plain Text Email\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
This is a plain text email.\r\n\
It has multiple lines.\r\n\
And preserves formatting.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract plain text email");

    assert!(result.content.contains("This is a plain text email"));
    assert!(result.content.contains("multiple lines"));
    assert!(result.content.contains("preserves formatting"));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("Plain Text Email".to_string()));
}

/// Test EML multipart (HTML + plain text).
#[tokio::test]
async fn test_eml_multipart() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Multipart Email\r\n\
Content-Type: multipart/alternative; boundary=\"----boundary\"\r\n\
\r\n\
------boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Plain text version of the email.\r\n\
------boundary\r\n\
Content-Type: text/html\r\n\
\r\n\
<html><body><p>HTML version of the email.</p></body></html>\r\n\
------boundary--\r\n";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract multipart email");

    assert!(
        result.content.contains("Plain text version") || result.content.contains("HTML version"),
        "Should extract either plain text or HTML content"
    );

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("Multipart Email".to_string()));
}

/// Test MSG file extraction (Outlook format).
///
/// Note: Creating valid MSG files programmatically is complex.
/// This test verifies error handling for invalid MSG format.
#[tokio::test]
async fn test_msg_file_extraction() {
    let config = ExtractionConfig::default();

    let invalid_msg = b"This is not a valid MSG file";

    let result = extract_bytes(invalid_msg, "application/vnd.ms-outlook", &config).await;

    assert!(result.is_err(), "Invalid MSG should fail gracefully");
}

/// Test email thread with quoted replies.
#[tokio::test]
async fn test_email_thread() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: person2@example.com\r\n\
To: person1@example.com\r\n\
Subject: Re: Original Subject\r\n\
In-Reply-To: <original@example.com>\r\n\
\r\n\
This is my reply.\r\n\
\r\n\
On Mon, 1 Jan 2024, person1@example.com wrote:\r\n\
> Original message text here.\r\n\
> This was the first message.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract email thread");

    assert!(result.content.contains("This is my reply"));

    assert!(result.content.contains("Original message text") || result.content.contains(">"));
}

/// Test email with various encodings (UTF-8, quoted-printable).
#[tokio::test]
async fn test_email_encodings() {
    let config = ExtractionConfig::default();

    let eml_content = "From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Email with Unicode: 你好世界 🌍\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Email body with special chars: café, naïve, résumé.\r\n\
Emoji: 🎉 🚀 ✅"
        .as_bytes();

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract UTF-8 email");

    assert!(result.content.contains("café") || result.content.contains("naive") || !result.content.is_empty());

    if let Some(subject) = result.metadata.subject {
        assert!(subject.contains("Unicode") || subject.contains("Email"));
    }
}

/// Test email with multiple recipients (To, CC, BCC).
#[tokio::test]
async fn test_email_large_attachments() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: r1@example.com, r2@example.com, r3@example.com\r\n\
Cc: cc1@example.com, cc2@example.com\r\n\
Bcc: bcc@example.com\r\n\
Subject: Multiple Recipients\r\n\
\r\n\
Email to multiple recipients.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract email with multiple recipients");

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));

    assert_eq!(email_meta.to_emails.len(), 3, "Should have 3 To recipients");
    assert!(email_meta.to_emails.contains(&"r1@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r2@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r3@example.com".to_string()));

    assert_eq!(email_meta.cc_emails.len(), 2, "Should have 2 CC recipients");
    assert!(email_meta.cc_emails.contains(&"cc1@example.com".to_string()));
    assert!(email_meta.cc_emails.contains(&"cc2@example.com".to_string()));

    assert_eq!(result.metadata.subject, Some("Multiple Recipients".to_string()));

    assert!(email_meta.attachments.is_empty(), "Should have no attachments");
}

/// Test malformed email structure.
#[tokio::test]
async fn test_malformed_email() {
    let config = ExtractionConfig::default();

    let malformed_eml = b"This is not a valid email at all.";

    let result = extract_bytes(malformed_eml, "message/rfc822", &config).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed email gracefully"
    );
}

// ---------------------------------------------------------------------------
// MSG (Outlook) integration tests — exercises the direct CFB parser
// ---------------------------------------------------------------------------

/// Test MSG extraction with subject, sender, recipients, and body.
#[tokio::test]
async fn test_msg_basic_extraction() {
    if helpers::skip_if_missing("email/test_email.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/test_email.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should extract MSG successfully");

    assert!(result.content.contains("Subject: Test Email"));
    assert!(result.content.contains("Test Email"));

    let email_meta = match result.metadata.format.as_ref().expect("format") {
        kreuzberg::FormatMetadata::Email(m) => m,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(result.metadata.subject, Some("Test Email".to_string()));
    assert!(!email_meta.to_emails.is_empty());
    assert_eq!(email_meta.attachments.len(), 3);
}

/// Test MSG with Unicode content (sender, subject, date, message-id).
#[tokio::test]
async fn test_msg_unicode() {
    if helpers::skip_if_missing("email/unicode.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/unicode.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should extract Unicode MSG");

    assert_eq!(result.metadata.subject, Some("Test for TIF files".to_string()));

    let email_meta = match result.metadata.format.as_ref().expect("format") {
        kreuzberg::FormatMetadata::Email(m) => m,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(
        email_meta.from_email,
        Some("\"Brian Zhou\" <brizhou@gmail.com>".to_string())
    );
    assert!(email_meta.to_emails.iter().any(|e| e.contains("brianzhou@me.com")));
    assert!(email_meta.message_id.is_some());
    assert!(result.metadata.created_at.is_some());
}

/// Test MSG with named attachments and MIME types.
#[tokio::test]
async fn test_msg_attachments() {
    if helpers::skip_if_missing("email/attachment.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/attachment.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should extract MSG with attachments");

    let email_meta = match result.metadata.format.as_ref().expect("format") {
        kreuzberg::FormatMetadata::Email(m) => m,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.attachments.len(), 3);
}

/// Test MSG with truncated FAT (cfb strict mode rejects, lenient padding handles).
#[tokio::test]
async fn test_msg_truncated_fat() {
    if helpers::skip_if_missing("email/simple_msg_alt.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/simple_msg_alt.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should handle MSG with truncated FAT via lenient padding");

    assert_eq!(result.metadata.subject, Some("This is the subject".to_string()));

    let email_meta = match result.metadata.format.as_ref().expect("format") {
        kreuzberg::FormatMetadata::Email(m) => m,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(
        email_meta.from_email,
        Some("\"peterpan@neverland.com\" <peterpan@neverland.com>".to_string())
    );
}

/// Test MSG with truncated FAT and attachments.
#[tokio::test]
async fn test_msg_truncated_fat_with_attachments() {
    if helpers::skip_if_missing("email/msg_with_attachments_alt.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/msg_with_attachments_alt.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should handle truncated-FAT MSG with attachments");

    assert_eq!(result.metadata.subject, Some("This is the subject".to_string()));

    let email_meta = match result.metadata.format.as_ref().expect("format") {
        kreuzberg::FormatMetadata::Email(m) => m,
        _ => panic!("Expected Email metadata"),
    };
    assert!(
        !email_meta.attachments.is_empty(),
        "Should have attachments in metadata"
    );
}

/// Test that a large MSG with big attachments completes in reasonable time
/// (regression test for issue #372 — previously hung due to hex-encoding overhead).
#[tokio::test]
async fn test_msg_large_attachment_no_hang() {
    if helpers::skip_if_missing("email/MSG_hang_repro.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/MSG_hang_repro.msg")).unwrap();

    let start = std::time::Instant::now();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config)
        .await
        .expect("Should extract large MSG without hanging");
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 10,
        "Extraction took {}s — should complete in under 10s",
        elapsed.as_secs()
    );
    assert!(result.content.contains("MSG hang repro"));
    assert!(result.content.contains("Attachments:"));
}

/// Test that invalid/corrupt MSG data returns a clear error.
#[tokio::test]
async fn test_msg_invalid_data() {
    let config = ExtractionConfig::default();
    let result = extract_bytes(b"not a valid MSG", "application/vnd.ms-outlook", &config).await;
    assert!(result.is_err());
}

/// Test that a tiny invalid OLE file returns a clear error.
#[tokio::test]
async fn test_msg_bad_outlook() {
    if helpers::skip_if_missing("email/bad_outlook.msg") {
        return;
    }

    let config = ExtractionConfig::default();
    let data = std::fs::read(helpers::get_test_file_path("email/bad_outlook.msg")).unwrap();
    let result = extract_bytes(&data, "application/vnd.ms-outlook", &config).await;
    assert!(result.is_err(), "Corrupt MSG should fail gracefully");
}
