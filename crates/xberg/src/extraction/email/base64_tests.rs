use super::eml::*;
use super::*;

/// A body and an attachment whose base64 ends in a stray `>` are decoded. `mail-parser` stops at
/// that character and keeps the encoded text.
#[test]
fn test_base64_with_a_stray_character_is_decoded() {
    // "Hello Bob, the figures are attached." with a `>` after the last symbol
    let encoded = "SGVsbG8gQm9iLCB0aGUgZmln\r\ndXJlcyBhcmUgYXR0YWNoZWQu>\r\n";

    let body = format!(
        "From: alice@example.com\r\nSubject: Figures\r\nMIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{encoded}"
    );
    let result = parse_eml_content(body.as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "Hello Bob, the figures are attached.");

    let attachment = format!(
        "From: alice@example.com\r\nSubject: Figures\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\r\n\
         --BOUNDARY\r\nContent-Type: text/plain\r\n\r\nSee the note.\r\n\
         --BOUNDARY\r\nContent-Type: text/plain; name=\"note.txt\"\r\n\
         Content-Disposition: attachment; filename=\"note.txt\"\r\n\
         Content-Transfer-Encoding: base64\r\n\r\n{encoded}--BOUNDARY--\r\n"
    );
    let result = parse_eml_content(attachment.as_bytes()).unwrap();
    let note = &result.attachments[0];
    assert_eq!(note.data.as_deref(), Some(&b"Hello Bob, the figures are attached."[..]));
    assert_eq!(note.size, Some(36));
}

/// The contents of a text part are already decoded with its charset, so a UTF-16 body has to be
/// recovered from the message's raw bytes.
#[test]
fn test_utf16_base64_with_a_stray_character_is_decoded() {
    // "Hi" in UTF-16LE is `SABpAA==`; the `>` stops mail-parser's decoder ~keep
    let body = "From: alice@example.com\r\nSubject: Hi\r\nMIME-Version: 1.0\r\n\
                Content-Type: text/plain; charset=utf-16le\r\nContent-Transfer-Encoding: base64\r\n\r\n\
                SA>BpAA==\r\n";
    let result = parse_eml_content(body.as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "Hi");

    let attachment = "From: alice@example.com\r\nSubject: Hi\r\nMIME-Version: 1.0\r\n\
                      Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\r\n\
                      --BOUNDARY\r\nContent-Type: text/plain\r\n\r\nSee the note.\r\n\
                      --BOUNDARY\r\nContent-Type: text/plain; charset=utf-16le; name=\"hi.txt\"\r\n\
                      Content-Disposition: attachment; filename=\"hi.txt\"\r\n\
                      Content-Transfer-Encoding: base64\r\n\r\nSA>BpAA==\r\n--BOUNDARY--\r\n";
    let result = parse_eml_content(attachment.as_bytes()).unwrap();
    assert_eq!(result.attachments[0].data.as_deref(), Some(&b"H\0i\0"[..]));
}

/// The base64 of this PDF has a `>` in its last line, and after it one symbol is left on its own,
/// since the message was cut off in the middle of the file.
#[test]
fn test_truncated_base64_attachment_is_decoded_as_far_as_it_goes() {
    let eml_data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test_documents/email/mailgun_pdf_attachment.eml"
    ))
    .expect("mailgun_pdf_attachment.eml should exist");
    let result = parse_eml_content(&eml_data).unwrap();
    let pdf = result.attachments[0].data.as_deref().expect("attachment data");
    assert!(pdf.starts_with(b"%PDF-1.3"), "got {:?}", &pdf[..pdf.len().min(16)]);
    assert!(
        pdf.windows(33)
            .any(|window| window == b"/MediaBox [0 0 612.0000 792.0000]"),
        "got {:?}",
        String::from_utf8_lossy(pdf)
    );
    assert_eq!(pdf.len(), 411);
}

fn single_part_eml(transfer_encoding: &str, body: &str) -> String {
    format!(
        "From: alice@example.com\r\nSubject: Figures\r\nMIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: {transfer_encoding}\r\n\r\n{body}"
    )
}

/// A plain sentence in a part labelled base64 is not base64, so the body and the attachment keep
/// the sentence.
#[test]
fn test_plain_text_labelled_base64_keeps_its_words() {
    let sentence = "Hello Bob, the figures are attached.";
    let result = parse_eml_content(single_part_eml("base64", &format!("{sentence}\r\n")).as_bytes()).unwrap();
    assert_eq!(result.content.trim(), sentence);

    let tabbed = "Hello\tBob,\tthe\tfigures\tare\tattached.";
    let result = parse_eml_content(single_part_eml("base64", &format!("{tabbed}\r\n")).as_bytes()).unwrap();
    assert_eq!(result.content.trim(), tabbed);

    let attachment = format!(
        "From: alice@example.com\r\nSubject: Figures\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\r\n\
         --BOUNDARY\r\nContent-Type: text/plain\r\n\r\nSee the note.\r\n\
         --BOUNDARY\r\nContent-Type: text/plain; name=\"note.txt\"\r\n\
         Content-Disposition: attachment; filename=\"note.txt\"\r\n\
         Content-Transfer-Encoding: base64\r\n\r\n{sentence}\r\n--BOUNDARY--\r\n"
    );
    let result = parse_eml_content(attachment.as_bytes()).unwrap();
    let note = result.attachments[0].data.as_deref().expect("attachment data");
    assert_eq!(String::from_utf8_lossy(note).trim(), sentence);
}

/// Only a space between two symbols reads as words. A stray character next to a space does not.
#[test]
fn test_stray_characters_next_to_spaces_still_decode() {
    let result = parse_eml_content(single_part_eml("base64", "SGVs >b> G8=\r\n").as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "Hello");
}

/// A quoted-printable part that `mail-parser` cannot decode keeps its text: only a part labelled
/// base64 is decoded again.
#[test]
fn test_quoted_printable_with_a_decoding_problem_is_not_read_as_base64() {
    let result = parse_eml_content(single_part_eml("quoted-printable", "if (a==b) return;\r\n").as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "if (a==b) return;");
}

/// RFC 2045 takes the first `=` as the end of the data, so the symbols after it are not decoded.
#[test]
fn test_recovered_base64_ends_at_the_first_padding_character() {
    let result = parse_eml_content(single_part_eml("base64", "SGVs>bG8=\r\nV29ybGQ=\r\n").as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "Hello");
}

/// Base64 that is cut off after two symbols of a group still holds one whole byte there.
#[test]
fn test_base64_cut_off_inside_a_group_keeps_its_last_whole_byte() {
    let result = parse_eml_content(single_part_eml("base64", "SGVsbG8gV2>\r\n").as_bytes()).unwrap();
    assert_eq!(result.content.trim(), "Hello W");
}

/// Each text body is recovered from its own part.
#[test]
fn test_second_text_body_with_a_stray_character_is_decoded() {
    let eml = "From: alice@example.com\r\nSubject: Figures\r\nMIME-Version: 1.0\r\n\
               Content-Type: multipart/mixed; boundary=\"OUTER\"\r\n\r\n\
               --OUTER\r\nContent-Type: text/plain\r\n\r\nIntro.\r\n\
               --OUTER\r\nContent-Type: multipart/alternative; boundary=\"INNER\"\r\n\r\n\
               --INNER\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n\
               V29y>bGQ=\r\n\
               --INNER\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>World</p>\r\n\
               --INNER--\r\n--OUTER--\r\n";
    let result = parse_eml_content(eml.as_bytes()).unwrap();
    assert_eq!(result.plain_text.as_deref(), Some("Intro.\n\nWorld"));
}

/// An attached message nested deeper than `mail-parser` parses base64 messages is decoded but
/// not parsed, and flagged `NESTING_LIMIT`. Its contents are decoded already.
#[test]
fn test_attached_message_decoded_by_mail_parser_is_not_decoded_again() {
    use base64::Engine as _;

    let mut message = "From: inner@example.com\r\nSubject: Inner\r\n\r\nInner body\r\n".to_string();
    for level in 0..4 {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&message);
        message = format!(
            "From: level{level}@example.com\r\nSubject: Level {level}\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"B{level}\"\r\n\r\n\
             --B{level}\r\nContent-Type: text/plain\r\n\r\nLevel {level}\r\n\
             --B{level}\r\nContent-Type: message/rfc822\r\nContent-Transfer-Encoding: base64\r\n\r\n\
             {encoded}\r\n--B{level}--\r\n"
        );
    }

    fn flagged_decoded_parts<'a>(message: mail_parser::MessageRef<'a>, out: &mut Vec<mail_parser::MessagePart<'a>>) {
        for part in message.parts() {
            match part.kind() {
                mail_parser::PartKind::Message(nested) => flagged_decoded_parts(nested, out),
                _ if part.flags().contains(mail_parser::PartFlags::NESTING_LIMIT)
                    && part.encoding() == mail_parser::Encoding::Base64 =>
                {
                    out.push(part)
                }
                _ => {}
            }
        }
    }
    let parsed = mail_parser::MessageParser::default().parse(message.as_bytes()).unwrap();
    let mut flagged = Vec::new();
    flagged_decoded_parts(parsed.root(), &mut flagged);

    assert_eq!(
        flagged.len(),
        1,
        "expected the innermost message to be decoded but not parsed"
    );
    let part = flagged[0];
    assert!(
        part.decoded().starts_with(b"From: inner@example.com"),
        "got {:?}",
        String::from_utf8_lossy(part.decoded().as_ref())
    );
    assert_eq!(recover_base64_contents(&part), None);
}
