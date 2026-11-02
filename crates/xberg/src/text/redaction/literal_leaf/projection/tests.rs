use super::*;

fn limits(source: &str, depth: usize) -> Limits {
    Limits {
        input: source.len(),
        owned: reservation(source.len()).unwrap() + capacities(source, depth).unwrap().2,
        operations: usize::MAX,
    }
}

fn parsed(source: &str) -> Projection<'_> {
    let depth = source.len() + 2;
    let mut work = Work::default();
    let projection = project(source, limits(source, depth), depth, &mut work).unwrap();
    assert_eq!(work.document_parses, 1);
    projection
}

#[test]
fn three_real_formatted_leak_shapes_map_to_exact_source_envelopes() {
    for (source, text, semantic, raw) in [
        (r"Visit AT\&T now safe-branch", "AT&T", 6..10, 6..11),
        (r"Visit a\<b now safe-branch", "a<b", 6..9, 6..10),
        ("Visit Jane<u>Doe</u> now safe-branch", "JaneDoe", 6..13, 6..20),
    ] {
        let projection = parsed(source);
        let mapped = projection
            .map(
                semantic.clone(),
                text,
                limits(source, source.len() + 2),
                &mut Work::default(),
            )
            .unwrap();
        assert_eq!(
            mapped,
            Mapped {
                raw: raw.clone(),
                semantic
            }
        );
        let mut output = source.to_string();
        output.replace_range(mapped.raw, "[REDACTED]");
        assert_eq!(output, "Visit [REDACTED] now safe-branch");
    }
}

#[test]
fn nested_recognized_wrappers_close_only_when_their_whole_semantics_are_covered() {
    let source = "x **Jane<u>Doe</u>** y";
    let projection = parsed(source);
    assert_eq!(projection.leaf.text, "x JaneDoe y");
    let all = projection
        .map(2..9, "JaneDoe", limits(source, source.len() + 2), &mut Work::default())
        .unwrap();
    assert_eq!(&source[all.raw], "**Jane<u>Doe</u>**");
    let part = projection
        .map(7..9, "oe", limits(source, source.len() + 2), &mut Work::default())
        .unwrap();
    assert_eq!(&source[part.raw], "oe");
    let emphasis = parsed("a *Jane* b<sup>2</sup>c");
    assert_eq!(emphasis.leaf.text, "a Jane b2c");
}

#[test]
fn code_trim_never_removes_preceding_unicode_or_prior_origins() {
    let source = "é **β** ` foo ` γ";
    let projection = parsed(source);
    assert_eq!(projection.leaf.text, "é β foo γ");
    assert_eq!(
        projection.leaf.origins[0],
        Origin {
            decoded: 0..2,
            source: 0..2
        }
    );
    assert_eq!(
        &source[projection
            .map(6..9, "foo", limits(source, source.len() + 2), &mut Work::default())
            .unwrap()
            .raw],
        "` foo `"
    );
    let crlf_source = "é `α\r\nβ` z";
    let crlf = parsed(crlf_source);
    assert_eq!(crlf.leaf.text, "é α\r β z");
    let crlf_match = crlf
        .map(
            3..9,
            "α\r β",
            limits(crlf_source, crlf_source.len() + 2),
            &mut Work::default(),
        )
        .unwrap();
    assert_eq!(&crlf_source[crlf_match.raw], "`α\r\nβ`");
    let repeat = parsed("é **é** é");
    assert_eq!(repeat.leaf.text, "é é é");
    assert_eq!(
        repeat.leaf.origins.iter().map(|o| o.source.clone()).collect::<Vec<_>>(),
        vec![0..2, 2..3, 5..7, 9..10, 10..12]
    );
}

#[test]
fn one_whole_source_parse_is_distinct_from_charged_entity_atom_parses() {
    let source = "a &amp; **b** &NotEqualTilde; ` c `";
    let depth = source.len() + 2;
    let mut work = Work::default();
    let projection = project(source, limits(source, depth), depth, &mut work).unwrap();
    assert_eq!(projection.leaf.text, "a & b ≂\u{338} c");
    assert_eq!(work.document_parses, 1);
    assert_eq!(work.parses, 3);
    assert_eq!(work.reserved, limits(source, depth).owned);
}

#[test]
fn unsupported_fields_blocks_and_unbalanced_tags_are_not_silently_omitted() {
    for source in [
        "# header",
        "a\n\nb",
        "a\nb",
        "[Jane](https://example.test \"secret\")",
        "![Jane](x)",
        "a <u title=\"secret\">Jane</u>",
        "a <u>Jane",
        "a </u>Jane",
        "a <u>Jane</sup>",
        "a <u>**Jane</u>**",
        "|a|b|\n|-|-|\n|c|d|",
        "- item",
        "~~Jane~~",
    ] {
        assert_eq!(
            project(
                source,
                limits(source, source.len() + 2),
                source.len() + 2,
                &mut Work::default()
            )
            .unwrap_err(),
            Error::Unsupported,
            "{source}"
        );
    }
}

#[test]
fn code_and_escaped_stars_never_invent_semantic_markup_or_tag_authority() {
    for source in ["a `Jane**Doe**` b", r"a Jane\*\*Doe\*\* b"] {
        let projection = parsed(source);
        assert_eq!(projection.leaf.text, "a Jane**Doe** b");
        assert_eq!(projection.source, source);
    }
    let source = "Jane<u>Doe</u>";
    let projection = parsed(source);
    assert_eq!(projection.source, source);
    assert_eq!(&projection.source[4..7], "<u>");
    assert_eq!(&projection.source[10..14], "</u>");
    let escaped = parsed(r"a \<u>Jane\</u> b");
    assert_eq!(escaped.leaf.text, "a <u>Jane</u> b");
    assert!(escaped.constructs.is_empty());
}

#[test]
fn utf8_invalid_stale_and_incomplete_origin_controls_reject() {
    let source = "α **β**";
    let projection = parsed(source);
    let cap = limits(source, source.len() + 2);
    assert_eq!(
        projection.map(1..2, "x", cap, &mut Work::default()),
        Err(Error::InvalidSource)
    );
    assert_eq!(
        projection.map(0..2, "β", cap, &mut Work::default()),
        Err(Error::InvalidSource)
    );
    let mut projection = parsed(source);
    projection.leaf.origins[0].source = 1..2;
    assert_eq!(
        projection.validate(cap, &mut Work::default()),
        Err(Error::InvalidSource)
    );
    let mut backwards = parsed(source);
    backwards.leaf.origins[2].source = 0..2;
    assert_eq!(backwards.validate(cap, &mut Work::default()), Err(Error::InvalidSource));
    let mut overlap = parsed("ab **cd**");
    overlap.leaf.origins[1].source = 0..2;
    assert_eq!(
        overlap.validate(limits(overlap.source, overlap.source.len() + 2), &mut Work::default()),
        Err(Error::InvalidSource)
    );
    let mut projection = parsed(source);
    projection.leaf.origins.remove(0);
    assert_eq!(
        projection.validate(cap, &mut Work::default()),
        Err(Error::InvalidSource)
    );
}

#[test]
fn all_wrapper_owned_capacities_are_preflighted_and_depth_work_is_bounded() {
    let source = "a **b**";
    let depth = source.len() + 2;
    let cap = limits(source, depth);
    let mut work = Work::default();
    assert_eq!(
        project(
            source,
            Limits {
                owned: cap.owned - 1,
                ..cap
            },
            depth,
            &mut work
        )
        .unwrap_err(),
        Error::Budget
    );
    assert_eq!(work.parses, 0);
    assert_eq!(work.document_parses, 0);
    assert_eq!(work.reserved, 0);
    let mut work = Work::default();
    assert_eq!(project(source, cap, 1, &mut work).unwrap_err(), Error::Budget);
    assert_eq!(work.document_parses, 1);
    let projection = parsed(source);
    assert_eq!(
        projection.map(0..1, "a", Limits { operations: 0, ..cap }, &mut Work::default()),
        Err(Error::Budget)
    );
}
