use super::*;

const STRATEGIES: [RedactionStrategy; 3] = [
    RedactionStrategy::Mask,
    RedactionStrategy::Hash,
    RedactionStrategy::TokenReplace,
];

fn replacement(strategy: RedactionStrategy, term: &str) -> &'static str {
    match strategy {
        RedactionStrategy::Mask => "[REDACTED]",
        RedactionStrategy::TokenReplace => "[PRIVATE_1]",
        RedactionStrategy::Hash => match term {
            "secret" => "[HASH:2bb80d537b1da3e3]",
            "JaneDoe" => "[HASH:573aa9a1c4169d65]",
            "AT&T" => "[HASH:d1aff162effb746e]",
            _ => panic!("unexpected fixture identity"),
        },
        RedactionStrategy::Drop => panic!("drop is outside this identity regression"),
    }
}

async fn assert_literal_identity(
    source: &'static str,
    term: &'static str,
    strategy: RedactionStrategy,
    expected: String,
    raw: std::ops::Range<u32>,
) {
    let config = config(strategy, &[term], &[]);
    let (mut result, cache) = edited(source, Some(term), &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, expected, "{source}: {strategy:?}");
    let token = replacement(strategy, term);
    assert_eq!(output.metadata.title.as_deref(), Some(token));
    let report = output.redaction_report.unwrap();
    assert_eq!(report.total_redacted, 2);
    assert_eq!(report.findings.len(), 2);
    assert_eq!((report.findings[0].start, report.findings[0].end), (raw.start, raw.end));
    assert!(report.findings.iter().all(|finding| finding.replacement_token == token));
}

#[tokio::test]
async fn contiguous_literal_findings_preserve_surrounding_markup_for_mask_hash_and_token() {
    for strategy in STRATEGIES {
        let token = replacement(strategy, "secret");
        for (source, expected, raw) in [
            ("unproved <u>secret</u>", format!("unproved <u>{token}</u>"), 12..18),
            ("unproved **secret**", format!("unproved **{token}**"), 11..17),
            (
                "unproved **<u>secret</u>**",
                format!("unproved **<u>{token}</u>**"),
                14..20,
            ),
        ] {
            assert_eq!(&source[raw.start as usize..raw.end as usize], "secret");
            assert_literal_identity(source, "secret", strategy, expected, raw).await;
        }
    }
}

#[tokio::test]
async fn split_encoded_and_normalized_code_keep_complete_envelopes_and_semantic_identity() {
    for strategy in STRATEGIES {
        for (source, term, raw) in [
            ("Visit Jane<u>Doe</u> now", "JaneDoe", 6..20),
            (r"Visit AT\&T now", "AT&T", 6..11),
            ("Visit ` secret ` now", "secret", 6..16),
        ] {
            let expected = format!("Visit {} now", replacement(strategy, term));
            assert_literal_identity(source, term, strategy, expected, raw).await;
        }
    }
}

#[tokio::test]
async fn multiple_literal_paragraphs_preserve_crlf_spacing_and_balanced_markup() {
    let source = "first <u>secret</u>\r\n \t\r\nsecond **secret**";
    for strategy in STRATEGIES {
        let token = replacement(strategy, "secret");
        let config = config(strategy, &["secret"], &[]);
        let (mut result, cache) = edited(source, Some("secret"), &config).await;
        late(&mut result, &cache, &config).await.unwrap();
        let output = finish(result);
        assert_eq!(
            output.content,
            format!("first <u>{token}</u>\r\n \t\r\nsecond **{token}**")
        );
        assert_eq!(output.metadata.title.as_deref(), Some(token));
        let report = output.redaction_report.unwrap();
        assert_eq!(report.total_redacted, 3);
        assert!(report.findings.iter().all(|finding| finding.replacement_token == token));
    }
}

#[tokio::test]
async fn paragraph_whitespace_never_joins_distinct_words_into_a_finding() {
    let source = "Jane\r\n \t\r\nDoe";
    let config = config(RedactionStrategy::Mask, &["JaneDoe"], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, source);
    assert_eq!(output.redaction_report.unwrap().total_redacted, 0);
}

#[tokio::test]
async fn later_unsupported_paragraphs_abort_all_strategies_before_any_document_mutation() {
    for strategy in STRATEGIES {
        for source in [
            "secret\n\n# secret",
            "secret\n\n- secret",
            "secret\n\n[secret](https://example.test)",
            "secret\n\n<u>secret</sup>",
            "<u>secret\n\nsecret</u>",
        ] {
            let config = config(strategy, &["secret"], &[]);
            let (mut result, cache) = edited(source, Some("secret"), &config).await;
            let before = serde_json::to_value(&*result).unwrap();
            let error = late(&mut result, &cache, &config)
                .await
                .expect_err("unsupported syntax must abort");
            assert!(matches!(&error, XbergError::Plugin { plugin_name, .. } if plugin_name == "redaction"));
            assert!(error.to_string().contains("unsupported syntax"), "{error}");
            assert_eq!(serde_json::to_value(&*result).unwrap(), before);
            assert!(result.redaction_report.is_none());
        }
    }
}
