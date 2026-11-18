use super::*;

fn limits(source: &str) -> Limits {
    Limits {
        input: source.len(),
        owned: reservation(source.len()).unwrap(),
        operations: usize::MAX,
    }
}

fn decode(source: &str, ordinal: usize) -> Leaf {
    let mut work = Work::default();
    let leaf = decode_leaf(source, ordinal, limits(source), &mut work).unwrap();
    assert!(work.parses > 0);
    assert!(work.operations >= source.len());
    assert_eq!(leaf.text.chars().count(), leaf.origins.len());
    for origin in &leaf.origins {
        assert!(leaf.text.get(origin.decoded.clone()).is_some());
        assert!(source.get(origin.source.clone()).is_some());
    }
    leaf
}

#[test]
fn escaped_punctuation_uses_actual_text_leaf_source_bytes() {
    let source = r"AT\&T a\<b Jane\*\*Doe\*\*";
    let leaf = decode(source, 0);
    assert_eq!(leaf.text, "AT&T a<b Jane**Doe**");
    assert_eq!(leaf.source, 0..source.len());
    assert_eq!(
        leaf.origins[2],
        Origin {
            decoded: 2..3,
            source: 2..4
        }
    );
    assert_eq!(
        leaf.origins[6],
        Origin {
            decoded: 6..7,
            source: 7..9
        }
    );
}

#[test]
fn repeated_unicode_leaves_keep_distinct_parser_coordinates() {
    let source = "é **é** é";
    let left = decode(source, 0);
    let middle = decode(source, 1);
    let right = decode(source, 2);
    assert_eq!(
        (left.text.as_str(), middle.text.as_str(), right.text.as_str()),
        ("é ", "é", " é")
    );
    assert_eq!((left.source, middle.source, right.source), (0..3, 5..7, 9..12));
    assert_eq!(
        middle.origins,
        vec![Origin {
            decoded: 0..2,
            source: 5..7
        }]
    );
}

#[test]
fn text_coordinates_cross_crlf_and_preserve_tabs() {
    let source = "one\r\né\ttwo";
    let leaf = decode(source, 1);
    assert_eq!(leaf.text, "é\ttwo");
    assert_eq!(leaf.source, 5..11);
    assert_eq!(
        leaf.origins[0],
        Origin {
            decoded: 0..2,
            source: 5..7
        }
    );
    assert_eq!(
        leaf.origins[1],
        Origin {
            decoded: 2..3,
            source: 7..8
        }
    );
}

#[test]
fn public_parser_decodes_named_numeric_and_multiscalar_entity_atoms() {
    let source = "&amp;&#x1F600;&NotEqualTilde;";
    let leaf = decode(source, 0);
    assert_eq!(leaf.text, "&😀≂\u{338}");
    assert_eq!(
        leaf.origins,
        vec![
            Origin {
                decoded: 0..1,
                source: 0..5
            },
            Origin {
                decoded: 1..5,
                source: 5..14
            },
            Origin {
                decoded: 5..8,
                source: 14..29
            },
            Origin {
                decoded: 8..10,
                source: 14..29
            },
        ]
    );
    let unknown = decode("&notARealEntity;", 0);
    assert_eq!(unknown.text, "&notARealEntity;");
}

#[test]
fn inline_code_keeps_stars_entities_and_tabs_literal() {
    let source = "`Jane**Doe** &amp;\t`";
    let leaf = decode(source, 0);
    assert_eq!(leaf.text, "Jane**Doe** &amp;\t");
    assert_eq!(leaf.source, 0..source.len());
    for origin in &leaf.origins {
        assert_eq!(&leaf.text[origin.decoded.clone()], &source[origin.source.clone()]);
    }
}

#[test]
fn inline_code_normalization_preserves_exact_crlf_and_trim_origins() {
    let leaf = decode("`α\r\nβ`", 0);
    assert_eq!(leaf.text, "α β");
    assert_eq!(
        leaf.origins,
        vec![
            Origin {
                decoded: 0..2,
                source: 1..3
            },
            Origin {
                decoded: 2..3,
                source: 3..5
            },
            Origin {
                decoded: 3..5,
                source: 5..7
            },
        ]
    );
    let trimmed = decode("` foo `", 0);
    assert_eq!(trimmed.text, "foo");
    assert_eq!(trimmed.origins[0].source, 2..3);
    assert_eq!(trimmed.origins[2].source, 4..5);
    assert_eq!(decode("`   `", 0).text, "   ");
}

#[test]
fn every_preflight_budget_rejects_before_parser_or_output() {
    let source = "&amp;";
    let exact = limits(source);
    for cap in [
        Limits {
            input: source.len() - 1,
            ..exact
        },
        Limits {
            owned: exact.owned - 1,
            ..exact
        },
        Limits {
            operations: source.len() - 1,
            ..exact
        },
    ] {
        let mut work = Work::default();
        assert_eq!(decode_leaf(source, 0, cap, &mut work).unwrap_err(), Error::Budget);
        assert_eq!(work.parses, 0);
    }
    assert_eq!(reservation(usize::MAX), Err(Error::Budget));
}

#[test]
fn entity_operation_budget_is_charged_before_atom_parse() {
    let source = "&amp;";
    let mut work = Work::default();
    let mut decoder = Decoder::prepare(source, limits(source), &mut work).unwrap();
    decoder.limits.operations = work.operations;
    assert_eq!(decoder.entity(0..source.len(), &mut work), Err(Error::Budget));
    assert_eq!(work.parses, 0);
    assert_eq!(decoder.leaf.text, "");
    assert!(decoder.leaf.origins.is_empty());
}

#[test]
fn invalid_code_and_missing_leaf_return_errors_without_a_leaf() {
    let source = "`abc`";
    let mut work = Work::default();
    let mut decoder = Decoder::prepare(source, limits(source), &mut work).unwrap();
    assert_eq!(
        decoder.code(0..source.len(), 0, "abc", &mut work),
        Err(Error::InvalidSource)
    );
    assert!(decoder.leaf.text.is_empty());
    assert_eq!(
        decode_leaf(source, 1, limits(source), &mut Work::default()).unwrap_err(),
        Error::MissingLeaf
    );
}

#[test]
fn owned_reservations_are_pass_local_and_cumulative() {
    let source = "abc";
    let mut work = Work::default();
    let leaf = decode_leaf(source, 0, limits(source), &mut work).unwrap();
    assert_eq!(leaf.text, "abc");
    assert_eq!(work.reserved, reservation(source.len()).unwrap());
    let parses = work.parses;
    assert_eq!(
        decode_leaf(source, 0, limits(source), &mut work).unwrap_err(),
        Error::Budget
    );
    assert_eq!(work.parses, parses);
}

#[test]
fn unknown_entity_partial_match_keeps_identity_origins_and_safe_bytes() {
    let source = "prefix &notARealEntity; suffix";
    let leaf = decode(source, 0);
    assert_eq!(leaf.text, source);
    assert_eq!(&leaf.text[12..16], "Real");
    let selected: Vec<_> = leaf
        .origins
        .iter()
        .filter(|origin| origin.decoded.start >= 12 && origin.decoded.end <= 16)
        .collect();
    assert_eq!(selected.len(), 4);
    let envelope = selected.first().unwrap().source.start..selected.last().unwrap().source.end;
    assert!(source.get(envelope.clone()).is_some());
    let mut replacement = source.to_string();
    replacement.replace_range(envelope, "[REDACTED]");
    let origins: Vec<_> = leaf
        .origins
        .iter()
        .map(|origin| (origin.decoded.clone(), origin.source.clone()))
        .collect();
    let identity: Vec<_> = (0..source.len()).map(|i| (i..i + 1, i..i + 1)).collect();
    assert_eq!(
        (origins, replacement),
        (identity, "prefix &notA[REDACTED]Entity; suffix".to_string())
    );
}
