//! Unit tests for [`super`].
//!
//! Split out of `cmap.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_parse_bfchar_single() {
    let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
}

#[test]
fn test_parse_bfchar_multiple() {
    let data = b"beginbfchar\n<0041> <0041>\n<0042> <0042>\n<0043> <0043>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
}

#[test]
fn test_large_bfrange_compresses_and_resolves() {
    // A 513-code contiguous range collapses into `ranges`, leaving `chars`
    // empty, and still resolves via computed range lookup. ~keep
    let data = b"beginbfrange\n<0100> <0300> <0500>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert!(!cmap.ranges.is_empty(), "large contiguous range should compress");
    assert!(cmap.chars.is_empty(), "compressed codes should leave `chars`");
    assert_eq!(cmap.get(&0x100).as_deref(), Some("\u{0500}"));
    assert_eq!(cmap.get(&0x300).as_deref(), Some("\u{0700}"));
    assert_eq!(cmap.get(&0x0FF), None);
    assert_eq!(cmap.get(&0x301), None);
}

#[test]
fn test_bfchar_override_survives_range_compression() {
    // A bfchar after a bfrange wins for that code (§9.10.3); compression must
    // not swallow it (it breaks contiguity and stays in `chars`). ~keep
    let data = b"beginbfrange\n<0100> <0300> <0500>\nendbfrange\n\
                 beginbfchar\n<0200> <0041>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x200).as_deref(), Some("A"), "later bfchar must win");
    assert_eq!(cmap.get(&0x1FF).as_deref(), Some("\u{05FF}"));
    assert_eq!(cmap.get(&0x201).as_deref(), Some("\u{0601}"));
}

#[test]
fn test_parse_bfchar_non_ascii() {
    let data = b"beginbfchar\n<00E9> <00E9>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0xE9).as_deref(), Some("é"));
}

#[test]
fn test_parse_bfrange_simple() {
    let data = b"beginbfrange\n<0041> <0043> <0041>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
}

#[test]
fn test_parse_bfrange_ascii_printable() {
    let data = b"beginbfrange\n<0020> <007E> <0020>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x20).as_deref(), Some(" "));
    assert_eq!(cmap.get(&0x30).as_deref(), Some("0"));
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x7A).as_deref(), Some("z"));
    assert_eq!(cmap.get(&0x7E).as_deref(), Some("~"));
}

#[test]
fn test_parse_mixed_bfchar_bfrange() {
    let data = b"beginbfchar\n<0041> <0058>\nendbfchar\nbeginbfrange\n<0042> <0044> <0042>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x41).as_deref(), Some("X"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
    assert_eq!(cmap.get(&0x44).as_deref(), Some("D"));
}

#[test]
fn test_parse_empty_cmap() {
    let data = b"";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert!(cmap.is_empty());
}

#[test]
fn test_parse_cmap_with_whitespace() {
    let data = b"beginbfchar\n  <0041>    <0041>  \n  <0042>  <0042>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
}

#[test]
fn test_parse_bfchar_line() {
    assert_eq!(parse_bfchar_line("<0041> <0041>"), vec![(0x41, "A".to_string())]);
    assert_eq!(parse_bfchar_line("<00E9> <00E9>"), vec![(0xE9, "é".to_string())]);
    assert!(parse_bfchar_line("invalid line").is_empty());
}

#[test]
fn test_parse_bfchar_multiple_pairs_per_line() {
    let result = parse_bfchar_line("<01> <0041> <02> <0042> <03> <0043>");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], (0x01, "A".to_string()));
    assert_eq!(result[1], (0x02, "B".to_string()));
    assert_eq!(result[2], (0x03, "C".to_string()));
}

#[test]
fn test_parse_bfrange_section() {
    let result = parse_bfrange_section("<0041> <0043> <0041>");
    assert_eq!(result.len(), 1);
    let mappings = result[0].as_ref().unwrap();
    assert_eq!(mappings.len(), 3);
    assert_eq!(mappings[0], (0x41, "A".to_string()));
    assert_eq!(mappings[1], (0x42, "B".to_string()));
    assert_eq!(mappings[2], (0x43, "C".to_string()));
}

#[test]
fn test_parse_bfrange_section_single_char() {
    let result = parse_bfrange_section("<0041> <0041> <0041>");
    assert_eq!(result, vec![Some(vec![(0x41, "A".to_string())])]);
}

#[test]
fn test_parse_bfrange_section_invalid() {
    assert_eq!(parse_bfrange_section("invalid"), vec![None]);
    assert_eq!(parse_bfrange_section("<0041> <0042>"), vec![None]);
    assert!(parse_bfrange_section(" % only a comment\n").is_empty());
}

#[test]
fn test_extract_sections() {
    let content = "before\nbeginbfchar\ndata1\nendbfchar\nmiddle\nbeginbfchar\ndata2\nendbfchar\nafter";
    let sections = extract_sections(content, "beginbfchar", "endbfchar");
    assert_eq!(sections.len(), 2);
    assert!(sections[0].contains("data1"));
    assert!(sections[1].contains("data2"));
}

#[test]
fn test_extract_sections_none() {
    let content = "no sections here";
    let sections = extract_sections(content, "beginbfchar", "endbfchar");
    assert_eq!(sections.len(), 0);
}

#[test]
fn test_parse_cid_to_unicode() {
    let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
    let cmap = parse_cid_to_unicode(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
}

#[test]
fn test_parse_hex_case_insensitive() {
    let data = b"beginbfchar\n<00aB> <00Ab>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0xAB).as_deref(), Some("«"));
}

#[test]
fn test_parse_multiple_sections() {
    let data = b"beginbfchar\n<0041> <0041>\nendbfchar\nbeginbfchar\n<0042> <0042>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.len(), 2);
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
}

#[test]
fn test_parse_bfchar_ligature() {
    let data = b"beginbfchar\n<000C> <00660069>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x0C).as_deref(), Some("fi"));
}

#[test]
fn test_parse_bfchar_multiple_ligatures() {
    let data = b"beginbfchar\n<000B> <00660066>\n<000C> <00660069>\n<000D> <0066006C>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x0B).as_deref(), Some("ff"));
    assert_eq!(cmap.get(&0x0C).as_deref(), Some("fi"));
    assert_eq!(cmap.get(&0x0D).as_deref(), Some("fl"));
}

#[test]
fn test_parse_bfrange_array_ligatures() {
    let data = b"beginbfrange\n<005F> <0061> [<00660066> <00660069> <00660066006C>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x5F).as_deref(), Some("ff"));
    assert_eq!(cmap.get(&0x60).as_deref(), Some("fi"));
    assert_eq!(cmap.get(&0x61).as_deref(), Some("ffl"));
}

#[test]
fn test_parse_bfrange_array_mixed() {
    let data = b"beginbfrange\n<0010> <0012> [<0041> <00660069> <0043>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("fi"));
    assert_eq!(cmap.get(&0x12).as_deref(), Some("C"));
}

/// A writer can break a long destination array over several lines (GH#2476).
/// Each continuation line holds three hex strings, the same shape as a
/// sequential `<lo> <hi> <dst>` entry, so a line-based reader maps the codes
/// two letters ahead.
#[test]
fn test_parse_bfrange_array_over_several_lines() {
    let data = b"beginbfrange\n<61> <67> [\n<0061> <0062> <0063>\n% a comment inside the array\n\
                 <0064> <0065> <0066>\n<0067>]\n<68> <69> <0068>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    let decoded: String = (0x61..=0x69).map(|code| cmap.get(&code).unwrap_or_default()).collect();
    assert_eq!(decoded, "abcdefghi");
}

#[test]
fn test_parse_bfrange_two_entries_on_one_line() {
    let data = b"beginbfrange\n<41> <42> <0061> <43> <43> [<0063>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("a"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("b"));
    assert_eq!(cmap.get(&0x43).as_deref(), Some("c"));
}

/// A token that does not fit the entry grammar drops that one entry; the
/// next entry still parses.
#[test]
fn test_parse_bfrange_malformed_entry_keeps_the_next_entry() {
    let data = b"beginbfrange\n<41> <42> 7\n<43> <43> <0063>\n<44> <45> [<0064> <46> <46> <0066>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41), None);
    assert_eq!(cmap.get(&0x43).as_deref(), Some("c"));
    assert_eq!(cmap.get(&0x44), None, "an array with no closing bracket is dropped");
}

#[test]
fn test_parse_zekat_cmap() {
    let cmap_data = r#"
/CIDInit /ProcSet findresource begin
19 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (UCS)
/Supplement 0
>> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfrange
<0003> <0004> <0020>
endbfrange
3 beginbfchar
<000F> <002C>
<0011> <002E>
<0024> <0041>
endbfchar
1 beginbfrange
<0027> <0029> <0044>
endbfrange
2 beginbfchar
<002C> <0049>
<002E> <004B>
endbfchar
2 beginbfrange
<0030> <0032> <004D>
<0035> <0037> <0052>
endbfrange
2 beginbfchar
<0039> <0056>
<003D> <005A>
endbfchar
5 beginbfrange
<0044> <0048> <0061>
<004A> <004C> <0067>
<004E> <0053> <006B>
<0055> <0059> <0072>
<005C> <005D> <0079>
endbfrange
5 beginbfchar
<006B> <00E2>
<006F> <00E7>
<007C> <00F6>
<0081> <00FC>
<00AB> <2026>
endbfchar
1 beginbfrange
<00B3> <00B4> <201C>
endbfrange
4 beginbfchar
<00C6> <00C2>
<00D5> <0131>
<00F7> <011F>
<00FA> <015F>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes();

    let cmap = parse_tounicode_cmap(cmap_data).expect("Failed to parse CMap");

    assert_eq!(cmap.get(&0x3D).as_deref(), Some("Z"));
    assert_eq!(cmap.get(&0x24).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0xC6).as_deref(), Some("\u{00C2}"));
}

/// `/WMode 1 def` on a CMap stream marks the font as vertical writing,
/// even when the CMap name does not advertise a `-V` suffix. This is the
/// authoritative signal per ISO 32000-1 §9.7.5.4 and is required for
/// embedded CMap streams used by tategaki layouts where the writer keeps
/// a horizontal-shaped CMap name but flips the writing mode internally.
#[test]
fn test_parse_wmode_vertical() {
    let data = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
/WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.wmode, 1, "explicit /WMode 1 def must set vertical writing");
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    assert_eq!(cmap.code_width, 2);
}

/// Default WMode is `0` (horizontal) when the directive is absent. Most
/// ToUnicode CMaps for horizontal text omit `/WMode` entirely; this
/// guards the dominant code path.
#[test]
fn test_parse_wmode_default_horizontal() {
    let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.wmode, 0, "missing /WMode must default to horizontal");
}

/// `/WMode 0 def` is a no-op but must be parsed without warning.
#[test]
fn test_parse_wmode_explicit_horizontal() {
    let data = b"\
begincmap
/WMode 0 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.wmode, 0);
}

/// M5: a `/WMode N def` directive that lives inside a PostScript
/// comment (`%` to end-of-line, §3.3.1) must NOT flip the writing
/// mode. Without comment-stripping, this commented-out producer
/// debug line would silently switch a horizontal CMap to vertical.
#[test]
fn test_parse_wmode_ignored_inside_postscript_comment() {
    let data = b"\
begincmap
% /WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(
        cmap.wmode, 0,
        "/WMode 1 def inside a PostScript comment must be ignored"
    );
}

/// M5 corollary: a legitimate `/WMode 1 def` on a later line is
/// still picked up even when an earlier line carries an unrelated
/// comment.
#[test]
fn test_parse_wmode_after_comment_still_seen() {
    let data = b"\
begincmap
% some prologue comment unrelated to wmode
/WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.wmode, 1);
}

/// M6: a non-standard `/WMode 2 def` must NOT silently flip writing
/// mode; the spec only defines 0 and 1 (§9.7.5.4). Parser returns
/// None (callers fall back to horizontal default) and emits a warn
/// log so producer bugs are diagnosable.
#[test]
fn test_parse_wmode_non_standard_value_falls_back() {
    let data = b"\
begincmap
/WMode 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(
        cmap.wmode, 0,
        "/WMode 2 def is non-standard; parser must fall back to horizontal"
    );
}

// ------------------------------------------------------------------
// Malformed-input regression tests (fail-loudly / degraded / empty
// classification for `parse_tounicode_cmap`).
//
// `capture_warnings` installs a minimal in-process `tracing::Subscriber`
// that records event messages, scoped only to the closure passed to it.
// It is test-only code living entirely in this module — no production
// type is touched to make these assertions possible. ~keep
// ------------------------------------------------------------------

#[derive(Clone, Default)]
struct RecordingSubscriber {
    messages: Arc<Mutex<Vec<String>>>,
}

#[derive(Default)]
struct MessageVisitor(Vec<String>);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl tracing::Subscriber for RecordingSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        self.messages.lock_or_recover().push(visitor.0.join(" "));
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

/// Run `f` under a subscriber that records every tracing event message,
/// returning them in emission order.
fn capture_warnings<F: FnOnce()>(f: F) -> Vec<String> {
    let subscriber = RecordingSubscriber::default();
    let messages = Arc::clone(&subscriber.messages);
    tracing::subscriber::with_default(subscriber, f);
    messages.lock_or_recover().clone()
}

/// FATAL: a non-empty stream with none of the CMap structural keywords is
/// not a CMap at all — any mapping derived from it would be arbitrary, so
/// the parser must fail loudly instead of returning a silently empty
/// CMap indistinguishable from a font that legitimately maps nothing.
#[test]
fn garbage_stream_without_cmap_syntax_is_rejected() {
    let data = b"RANDOM BINARY GARBAGE, NOT A CMAP STREAM AT ALL 0xDEADBEEF 1234567890";
    let err = parse_tounicode_cmap(data)
        .expect_err("a stream with no CMap keywords at all must be rejected, not silently empty");
    let message = err.to_string();
    assert!(
        message.to_lowercase().contains("cmap"),
        "error message should name the defect: {message}"
    );
}

/// LEGITIMATELY EMPTY: a zero-length stream is not evidence of
/// corruption — some producers emit an empty ToUnicode stream for a font
/// that genuinely maps nothing. No warning should fire.
#[test]
fn zero_length_stream_is_legitimately_empty_without_warning() {
    let logs = capture_warnings(|| {
        let cmap = parse_tounicode_cmap(b"").unwrap();
        assert!(cmap.is_empty());
    });
    assert!(
        logs.is_empty(),
        "a zero-length stream must not warn, it is a legitimate empty CMap: {logs:?}"
    );
}

/// DEGRADED (truncation): a `beginbfchar` block with no matching
/// `endbfchar` before EOF is dropped, not guessed at, and the defect is
/// surfaced via `tracing::warn!` rather than silently disappearing.
#[test]
fn truncated_bfchar_block_is_dropped_with_warning() {
    let data = b"beginbfchar\n<0041> <0041>\n";
    let logs = capture_warnings(|| {
        let cmap = parse_tounicode_cmap(data).expect("truncation is degraded, not fatal");
        assert!(
            cmap.is_empty(),
            "a block with no closing endbfchar must be dropped entirely, not partially guessed"
        );
    });
    assert!(
        logs.iter()
            .any(|m| m.contains("beginbfchar") && m.contains("endbfchar")),
        "expected a WARN naming the truncated beginbfchar block, got: {logs:?}"
    );
}

/// DEGRADED (truncation): the same defect class for `begincodespacerange`,
/// verifying that dropping one section type does not lose unrelated
/// sections (`beginbfchar` is scanned independently) and that the
/// dropped section's effect (2-byte `code_width`) is correctly absent.
#[test]
fn truncated_codespacerange_block_is_dropped_with_warning() {
    let data = b"begincodespacerange\n<0000> <FFFF>\nbeginbfchar\n<0041> <0041>\nendbfchar";
    let logs = capture_warnings(|| {
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(
            cmap.code_width, 1,
            "an unterminated codespacerange section must not set code_width"
        );
        assert_eq!(
            cmap.get(&0x41).as_deref(),
            Some("A"),
            "the unrelated, well-formed bfchar block must still parse"
        );
    });
    assert!(
        logs.iter()
            .any(|m| m.contains("begincodespacerange") && m.contains("endcodespacerange")),
        "expected a WARN naming the truncated codespacerange block, got: {logs:?}"
    );
}

/// DEGRADED (malformed hex): a line with an unparseable src code is
/// dropped, its well-formed neighbor still parses, and the defect is
/// surfaced via `tracing::warn!`.
#[test]
fn malformed_bfchar_hex_operand_is_skipped_with_warning() {
    let data = b"beginbfchar\n<0041> <0041>\n<ZZZZ> <0042>\nendbfchar";
    let logs = capture_warnings(|| {
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"), "well-formed entry still parses");
        assert_eq!(cmap.len(), 1, "the malformed entry must not appear in the map");
    });
    assert!(
        logs.iter()
            .any(|m| m.contains("bfchar") && m.contains("failed to parse")),
        "expected a WARN about the malformed bfchar line, got: {logs:?}"
    );
}

/// A parse failure now reaching `LazyCMap::get()` must be memoized: the
/// `Err` arm's `tracing::warn!` fires at most once per `LazyCMap`, not
/// once per character, since `get()` sits on the per-character decode
/// path (`FontInfo::char_to_unicode`).
#[test]
fn lazy_cmap_memoizes_a_parse_failure_and_warns_once() {
    const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_CMAP_NAME_9c12";
    let data = format!("RANDOM BINARY GARBAGE {CONFIDENTIAL_MARKER}").into_bytes();
    let lazy = LazyCMap::new(data);
    let logs = capture_warnings(|| {
        assert!(lazy.get().is_none(), "a garbage stream must fail to parse");
        assert!(lazy.get().is_none(), "second call must reuse the memoized failure");
        assert!(lazy.get().is_none(), "third call must reuse the memoized failure");
    });
    let failure_warnings = logs
        .iter()
        .filter(|message| {
            message.contains("operation=\"parse_tounicode_cmap\"")
                && message.contains("error_code=\"font_error\"")
                && message.contains("message=PDF operation degraded")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        failure_warnings.len(),
        1,
        "LazyCMap::get() must warn on a parse failure exactly once, not per call: {logs:?}"
    );
    assert!(!format!("{logs:?}").contains(CONFIDENTIAL_MARKER));
}
