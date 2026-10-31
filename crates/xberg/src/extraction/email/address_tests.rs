use super::parse_eml_content;

#[test]
fn should_flatten_recipient_groups_in_header_order_without_empty_group_names() {
    let eml = concat!(
        "From: sender@example.com\r\n",
        "To: solo@example.com, Team: Alice <alice@example.com>, Bob <bob@example.com>;, tail@example.com\r\n",
        "Cc: Copy: Carol <carol@example.com>;, cc@example.com\r\n",
        "Bcc: undisclosed-recipients:;, Secret: Dave <dave@example.com>;, bcc@example.com\r\n",
        "Reply-To: Support: Help One <help1@example.com>, Help Two <help2@example.com>;, direct@example.com\r\n",
        "Subject: Recipient groups\r\n\r\nBody"
    );
    let result = parse_eml_content(eml.as_bytes()).expect("real RFC822 mailbox and group headers");
    assert_eq!(
        result.to_emails,
        [
            "solo@example.com",
            "alice@example.com",
            "bob@example.com",
            "tail@example.com"
        ]
    );
    assert_eq!(result.cc_emails, ["carol@example.com", "cc@example.com"]);
    assert_eq!(result.bcc_emails, ["dave@example.com", "bcc@example.com"]);
    assert_eq!(
        result.metadata.get("reply_to").map(String::as_str),
        Some("help1@example.com, help2@example.com, direct@example.com")
    );
    assert_eq!(result.content, "Body");
}
