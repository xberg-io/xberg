use super::*;

const LEADING: &str = "[r]: /x \"sec<u>ret</u>\"\n\nhello\n\nworld";
const BETWEEN: &str = "hello\n\n[r]: /x \"sec<u>ret</u>\"\n\nworld";
const TRAILING: &str = "hello\n\nworld\n\n[r]: /x \"sec<u>ret</u>\"";

async fn rejects_unprojected_definition(source: &'static str, strategy: RedactionStrategy) {
    let config = config(strategy, &["secret"], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    let after_callback = serde_json::to_value(&*result).unwrap();
    let outcome = late(&mut result, &cache, &config).await;
    assert!(
        matches!(outcome, Err(crate::XbergError::Plugin { .. })),
        "a reference definition outside projected paragraphs must fail closed: {source:?}"
    );
    assert_eq!(serde_json::to_value(&*result).unwrap(), after_callback);
    assert!(result.redaction_report.is_none());
}

macro_rules! definition_case {
    ($name:ident, $source:ident, $strategy:ident) => {
        #[tokio::test]
        async fn $name() {
            rejects_unprojected_definition($source, RedactionStrategy::$strategy).await;
        }
    };
}

definition_case!(leading_definition_is_rejected_for_mask, LEADING, Mask);
definition_case!(leading_definition_is_rejected_for_hash, LEADING, Hash);
definition_case!(leading_definition_is_rejected_for_token_replace, LEADING, TokenReplace);
definition_case!(leading_definition_is_rejected_for_drop, LEADING, Drop);
definition_case!(between_definition_is_rejected_for_mask, BETWEEN, Mask);
definition_case!(between_definition_is_rejected_for_hash, BETWEEN, Hash);
definition_case!(between_definition_is_rejected_for_token_replace, BETWEEN, TokenReplace);
definition_case!(between_definition_is_rejected_for_drop, BETWEEN, Drop);
definition_case!(trailing_definition_is_rejected_for_mask, TRAILING, Mask);
definition_case!(trailing_definition_is_rejected_for_hash, TRAILING, Hash);
definition_case!(
    trailing_definition_is_rejected_for_token_replace,
    TRAILING,
    TokenReplace
);
definition_case!(trailing_definition_is_rejected_for_drop, TRAILING, Drop);

async fn dropped(source: &'static str, expected: &str, expected_offsets: &[(u32, u32)]) {
    let config = config(RedactionStrategy::Drop, &["secret"], &[]);
    let (mut result, cache) = edited(source, None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, expected);
    let report = output.redaction_report.unwrap();
    assert_eq!(report.total_redacted as usize, expected_offsets.len());
    assert_eq!(
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end))
            .collect::<Vec<_>>(),
        expected_offsets
    );
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.replacement_token.is_empty())
    );
}

macro_rules! drop_case {
    ($name:ident, $source:literal, $expected:literal) => {
        #[tokio::test]
        async fn $name() {
            let source = $source;
            let offsets = source
                .match_indices("secret")
                .map(|(start, _)| (start as u32, (start + 6) as u32))
                .collect::<Vec<_>>();
            assert!(!offsets.is_empty());
            dropped(source, $expected, &offsets).await;
        }
    };
}

drop_case!(
    drop_removes_a_wholly_redacted_strong_wrapper,
    "unproved **secret** suffix",
    "unproved  suffix"
);
drop_case!(
    drop_removes_a_wholly_redacted_emphasis_wrapper,
    "unproved *secret* suffix",
    "unproved  suffix"
);
drop_case!(
    drop_removes_a_wholly_redacted_underline_wrapper,
    "unproved <u>secret</u> suffix",
    "unproved  suffix"
);
drop_case!(
    drop_removes_nested_wholly_redacted_wrappers,
    "unproved **<u>secret</u>** suffix",
    "unproved  suffix"
);
drop_case!(
    drop_preserves_both_audits_when_adjacent_matches_empty_a_wrapper,
    "unproved **secretsecret** suffix",
    "unproved  suffix"
);
drop_case!(
    drop_preserves_inner_whitespace_when_removing_empty_wrappers,
    "unproved **secret secret** suffix",
    "unproved   suffix"
);
drop_case!(
    drop_removes_a_code_wrapper_emptied_by_multiple_matches,
    "unproved `secretsecret` suffix",
    "unproved  suffix"
);
drop_case!(
    drop_preserves_unrelated_and_already_empty_wrappers,
    "unproved **safe** <u></u> **secret** suffix",
    "unproved **safe** <u></u>  suffix"
);
drop_case!(
    drop_preserves_utf8_neighbors_and_original_byte_audits,
    "é **secret** β",
    "é  β"
);

#[tokio::test]
async fn drop_of_a_split_term_retains_the_original_proved_source_envelope_audit() {
    dropped("unproved **sec<u>ret</u>** suffix", "unproved  suffix", &[(9, 26)]).await;
}

#[tokio::test]
async fn drop_of_literal_tag_names_cannot_inherit_generated_tag_exemptions() {
    let config = config(RedactionStrategy::Drop, &["sup"], &[]);
    let (mut result, cache) = edited("Jane<sup>Doe</sup>", None, &config).await;
    late(&mut result, &cache, &config).await.unwrap();
    let output = finish(result);
    assert_eq!(output.content, "Jane<>Doe</>");
    let report = output.redaction_report.unwrap();
    assert_eq!(report.total_redacted, 2);
    assert_eq!(
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end))
            .collect::<Vec<_>>(),
        [(5, 8), (14, 17)]
    );
}

#[tokio::test]
async fn whitespace_only_edges_keep_literal_bytes_and_exact_strategy_audits() {
    let source = "\n\nunproved **secret** suffix\n\n";
    for (strategy, replacement) in [
        (RedactionStrategy::Mask, "[REDACTED]"),
        (RedactionStrategy::Hash, "[HASH:2bb80d537b1da3e3]"),
        (RedactionStrategy::TokenReplace, "[PRIVATE_1]"),
        (RedactionStrategy::Drop, ""),
    ] {
        let config = config(strategy, &["secret"], &[]);
        let (mut result, cache) = edited(source, None, &config).await;
        late(&mut result, &cache, &config).await.unwrap();
        let output = finish(result);
        let expected = if matches!(strategy, RedactionStrategy::Drop) {
            "\n\nunproved  suffix\n\n".to_string()
        } else {
            format!("\n\nunproved **{replacement}** suffix\n\n")
        };
        assert_eq!(output.content, expected);
        let report = output.redaction_report.unwrap();
        assert_eq!(report.total_redacted, 1);
        assert_eq!((report.findings[0].start, report.findings[0].end), (13, 19));
        assert_eq!(report.findings[0].replacement_token, replacement);
    }
}

const ONLY: &str = "[r]: /x \"sec<u>ret</u>\"";
definition_case!(reference_only_source_is_rejected_for_mask, ONLY, Mask);
definition_case!(reference_only_source_is_rejected_for_hash, ONLY, Hash);
definition_case!(reference_only_source_is_rejected_for_token_replace, ONLY, TokenReplace);
definition_case!(reference_only_source_is_rejected_for_drop, ONLY, Drop);

drop_case!(
    drop_keeps_wrappers_with_unredacted_payload,
    "unproved **secret safe** suffix",
    "unproved ** safe** suffix"
);
drop_case!(
    drop_removes_only_the_emptied_nested_wrapper,
    "unproved **<u>secret</u> safe** suffix",
    "unproved ** safe** suffix"
);
drop_case!(
    drop_removes_owned_tags_but_preserves_visible_padding,
    "unproved <u> secret </u> suffix",
    "unproved    suffix"
);

drop_case!(
    drop_preserves_empty_tag_before_the_first_origin_of_an_emptied_wrapper,
    "unproved **<u></u>secret** suffix",
    "unproved <u></u> suffix"
);
drop_case!(
    drop_preserves_empty_tag_after_the_last_origin_of_an_emptied_wrapper,
    "unproved **secret<u></u>** suffix",
    "unproved <u></u> suffix"
);
