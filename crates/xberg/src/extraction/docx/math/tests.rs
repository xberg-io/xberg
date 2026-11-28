use super::*;

/// Helper: parse an OMML XML fragment and return rendered LaTeX.
fn omml_to_latex(xml: &str) -> String {
    let wrapped = format!("<m:oMath>{}</m:oMath>", xml);
    let mut reader = Reader::from_str(&wrapped);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "m:oMath" => break,
            Ok(Event::Eof) => return String::new(),
            _ => {}
        }
        buf.clear();
    }
    let mut budget = SecurityBudget::with_defaults();
    collect_and_convert_omath(&mut reader, &mut budget).unwrap_or_default()
}

/// Helper (GH#1395): parse an OMML fragment through `collect_and_convert_omath`
/// using a caller-supplied budget, so the caller can inspect the budget's
/// residual depth state after conversion completes.
fn omml_with_budget(xml: &str, budget: &mut SecurityBudget) {
    let wrapped = format!("<m:oMath>{}</m:oMath>", xml);
    let mut reader = Reader::from_str(&wrapped);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "m:oMath" => break,
            Ok(Event::Eof) => panic!("unexpected EOF before <m:oMath>"),
            _ => {}
        }
        buf.clear();
    }
    // The DOCX parser's outer loop enters for the `<m:oMath>` start tag. ~keep
    budget.enter().expect("depth for <m:oMath>");
    collect_and_convert_omath(&mut reader, budget).expect("conversion ok");
}

/// Test-only probe (GH#1395): count how many more `budget.enter()` calls succeed
/// before the depth cap trips. A freshly-built budget accepts exactly `max_depth`
/// more entries; if the OMML conversion leaked N depth levels, only
/// `max_depth - N` succeed. This makes the depth counter observable through
/// `SecurityBudget`'s existing public API without touching `extractors/security.rs`.
fn probe_remaining_depth(budget: &mut SecurityBudget, max_depth: usize) -> usize {
    let mut successes = 0usize;
    for _ in 0..=max_depth {
        if budget.enter().is_ok() {
            successes += 1;
        } else {
            break;
        }
    }
    successes
}

fn budget_with_max_depth(max_depth: usize) -> SecurityBudget {
    let limits = crate::extractors::security::SecurityLimits {
        max_nesting_depth: max_depth,
        max_xml_depth: max_depth,
        ..Default::default()
    };
    SecurityBudget::from_limits(&limits)
}

/// GH#1395: `m:rPr` is skipped via `skip_to_end`, which reads through its own
/// `</m:rPr>` directly — the outer loop's `Event::End` arm never sees it, so
/// the `budget.enter()` made for `m:rPr`'s start tag must be refunded at the
/// call site or every run with formatting properties leaks one depth level.
#[test]
fn should_reset_depth_counter_to_zero_after_parsing_run_with_rpr() {
    let mut budget = budget_with_max_depth(64);
    omml_with_budget(
        r#"<m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>x</m:t></m:r>"#,
        &mut budget,
    );
    assert_eq!(
        probe_remaining_depth(&mut budget, 64),
        64,
        "m:rPr must not leak a depth level"
    );
}

/// GH#1395: same `skip_to_end` shape as `m:rPr`, but on a `*Pr` sibling that
/// hangs off a different element (`m:sSup`).
#[test]
fn should_reset_depth_counter_to_zero_after_parsing_ssup_with_ssuppr() {
    let mut budget = budget_with_max_depth(64);
    omml_with_budget(
        r#"<m:sSup>
                <m:sSupPr><m:ctrlPr/></m:sSupPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
                <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
            </m:sSup>"#,
        &mut budget,
    );
    assert_eq!(
        probe_remaining_depth(&mut budget, 64),
        64,
        "m:sSupPr must not leak a depth level"
    );
}

/// GH#1395: `m:fPr` is handled by `collect_frac_pr`, a dedicated recursive
/// parser (not `skip_to_end`) that also reads through its own `</m:fPr>`
/// without calling `budget.leave()` — same leak shape, different mechanism.
#[test]
fn should_reset_depth_counter_to_zero_after_parsing_frac_with_fpr() {
    let mut budget = budget_with_max_depth(64);
    omml_with_budget(
        r#"<m:f>
                <m:fPr><m:type m:val="noBar"/></m:fPr>
                <m:num><m:r><m:t>n</m:t></m:r></m:num>
                <m:den><m:r><m:t>k</m:t></m:r></m:den>
            </m:f>"#,
        &mut budget,
    );
    assert_eq!(
        probe_remaining_depth(&mut budget, 64),
        64,
        "m:fPr must not leak a depth level"
    );
}

/// GH#1395: an unrecognized OMML tag falls into `dispatch_omml_tag`'s `_` arm,
/// which also delegates to `skip_to_end` and must refund the same way.
#[test]
fn should_reset_depth_counter_to_zero_after_parsing_unrecognized_tag() {
    let mut budget = budget_with_max_depth(64);
    omml_with_budget(
        r#"<m:unknownObj><m:e><m:r><m:t>x</m:t></m:r></m:e></m:unknownObj>"#,
        &mut budget,
    );
    assert_eq!(
        probe_remaining_depth(&mut budget, 64),
        64,
        "an unrecognized OMML tag (skip_to_end fallback) must not leak a depth level"
    );
}

#[test]
fn should_reset_depth_counter_to_zero_after_parsing_group_chr_with_group_chr_pr() {
    let mut budget = budget_with_max_depth(64);
    omml_with_budget(
        r#"<m:groupChr>
                <m:groupChrPr><m:ctrlPr><w:rPr/></m:ctrlPr></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
        &mut budget,
    );
    assert_eq!(
        probe_remaining_depth(&mut budget, 64),
        64,
        "m:groupChrPr must not leak a depth level"
    );
}

#[test]
fn test_run_plain_text() {
    let latex = omml_to_latex(r#"<m:r><m:t>hello</m:t></m:r>"#);
    assert_eq!(latex, "hello");
}

#[test]
fn should_escape_literal_caret_and_tilde_in_math_run() {
    let latex = omml_to_latex(r#"<m:r><m:t>x^2~y</m:t></m:r>"#);
    assert_eq!(latex, r"x\hat{}2\sim y");
}

#[test]
fn should_escape_entity_decoded_caret_and_tilde_once() {
    let latex = omml_to_latex(r#"<m:r><m:t>&#94;&#126;_&amp;</m:t></m:r>"#);
    assert_eq!(latex, r"\hat{}\sim \_\&");
}

#[test]
fn should_keep_structural_superscripts_and_accents_when_escaping_run_text() {
    let latex = omml_to_latex(
        r#"<m:sSup>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
                <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
            </m:sSup>
            <m:acc>
                <m:accPr><m:chr m:val="^"/></m:accPr>
                <m:e><m:r><m:t>y</m:t></m:r></m:e>
            </m:acc>
            <m:acc>
                <m:accPr><m:chr m:val="~"/></m:accPr>
                <m:e><m:r><m:t>z</m:t></m:r></m:e>
            </m:acc>"#,
    );
    assert_eq!(latex, r"x^{2}\hat{y}\tilde{z}");
}

#[test]
fn test_run_unicode_pi() {
    let latex = omml_to_latex("<m:r><m:t>\u{03C0}</m:t></m:r>");
    assert_eq!(latex, "\\pi");
}

#[test]
fn should_trim_inline_and_display_latex() {
    assert_eq!(omml_to_latex("<m:r><m:t>&#x3B1;</m:t></m:r>"), "\\alpha");
    assert_eq!(omml_to_latex(r#"<m:r><m:t xml:space="preserve">  </m:t></m:r>"#), "");
    let xml = "<m:oMathPara><m:oMath><m:r><m:t>&#x3B1;</m:t></m:r></m:oMath><m:oMath><m:r><m:t>&#x3B2;</m:t></m:r></m:oMath></m:oMathPara>";
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "m:oMathPara" => break,
            Ok(Event::Eof) => panic!("unexpected EOF"),
            _ => {}
        }
        buf.clear();
    }
    let mut budget = SecurityBudget::with_defaults();
    let latex = collect_and_convert_omath_para(&mut reader, &mut budget).expect("conversion ok");
    assert_eq!(latex, "\\alpha \\\\ \\beta");
}

#[test]
fn test_ssup() {
    let latex = omml_to_latex(
        r#"<m:sSup>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
                <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
            </m:sSup>"#,
    );
    assert_eq!(latex, "x^{2}");
}

#[test]
fn test_ssub() {
    let latex = omml_to_latex(
        r#"<m:sSub>
                <m:e><m:r><m:t>a</m:t></m:r></m:e>
                <m:sub><m:r><m:t>n</m:t></m:r></m:sub>
            </m:sSub>"#,
    );
    assert_eq!(latex, "a_{n}");
}

#[test]
fn test_ssubsup() {
    let latex = omml_to_latex(
        r#"<m:sSubSup>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
                <m:sub><m:r><m:t>i</m:t></m:r></m:sub>
                <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
            </m:sSubSup>"#,
    );
    assert_eq!(latex, "x_{i}^{2}");
}

#[test]
fn test_frac_bar() {
    let latex = omml_to_latex(
        r#"<m:f>
                <m:num><m:r><m:t>a</m:t></m:r></m:num>
                <m:den><m:r><m:t>b</m:t></m:r></m:den>
            </m:f>"#,
    );
    assert_eq!(latex, "\\frac{a}{b}");
}

#[test]
fn test_frac_nobar() {
    let latex = omml_to_latex(
        r#"<m:f>
                <m:fPr><m:type m:val="noBar"/></m:fPr>
                <m:num><m:r><m:t>n</m:t></m:r></m:num>
                <m:den><m:r><m:t>k</m:t></m:r></m:den>
            </m:f>"#,
    );
    assert_eq!(latex, "\\binom{n}{k}");
}

#[test]
fn test_frac_lin() {
    let latex = omml_to_latex(
        r#"<m:f>
                <m:fPr><m:type m:val="lin"/></m:fPr>
                <m:num><m:r><m:t>a</m:t></m:r></m:num>
                <m:den><m:r><m:t>b</m:t></m:r></m:den>
            </m:f>"#,
    );
    assert_eq!(latex, "a/b");
}

#[test]
fn test_rad_simple() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:radPr><m:degHide m:val="1"/></m:radPr>
                <m:deg/>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt{x}");
}

#[test]
fn test_rad_with_degree() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:radPr><m:degHide m:val="0"/></m:radPr>
                <m:deg><m:r><m:t>3</m:t></m:r></m:deg>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt[3]{x}");
}

#[test]
fn test_rad_degree_without_deg_hide() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:radPr><m:ctrlPr><w:rPr><w:rFonts w:ascii="Cambria Math"/></w:rPr></m:ctrlPr></m:radPr>
                <m:deg><m:r><m:t>3</m:t></m:r></m:deg>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt[3]{x}");
}

#[test]
fn test_rad_degree_without_rad_pr() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:deg><m:r><m:t>n</m:t></m:r></m:deg>
                <m:e><m:r><m:t>y</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt[n]{y}");
}

#[test]
fn test_rad_deg_hide_without_val() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:radPr><m:degHide/></m:radPr>
                <m:deg><m:r><m:t>2</m:t></m:r></m:deg>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt{x}");
}

#[test]
fn test_rad_deg_hide_off() {
    let latex = omml_to_latex(
        r#"<m:rad>
                <m:radPr><m:degHide m:val="off"/></m:radPr>
                <m:deg><m:r><m:t>3</m:t></m:r></m:deg>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:rad>"#,
    );
    assert_eq!(latex, "\\sqrt[3]{x}");
}

#[test]
fn test_group_chr_underbrace() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\underbrace{x}");
}

#[test]
fn test_group_chr_character_reference() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:groupChrPr><m:chr m:val="&#x23DF;"/></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\underbrace{x}");
}

#[test]
fn test_group_chr_overbrace() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:groupChrPr><m:chr m:val="⏞"/><m:pos m:val="top"/></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\overbrace{x}");
}

#[test]
fn test_group_chr_default_brace_on_top() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:groupChrPr><m:pos m:val="top"/></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\overbrace{x}");
}

#[test]
fn test_group_chr_arrow_over() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:groupChrPr><m:chr m:val="→"/><m:pos m:val="top"/></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\overset{\\rightarrow }{x}");
}

#[test]
fn test_group_chr_arrow_under() {
    let latex = omml_to_latex(
        r#"<m:groupChr>
                <m:groupChrPr><m:chr m:val="→"/></m:groupChrPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:groupChr>"#,
    );
    assert_eq!(latex, "\\underset{\\rightarrow }{x}");
}

#[test]
fn test_group_chr_in_lim_low() {
    let latex = omml_to_latex(
        r#"<m:limLow>
                <m:e><m:groupChr><m:groupChrPr><m:ctrlPr/></m:groupChrPr>
                    <m:e><m:r><m:t>a+b</m:t></m:r></m:e>
                </m:groupChr></m:e>
                <m:lim><m:r><m:t>n</m:t></m:r></m:lim>
            </m:limLow>"#,
    );
    assert_eq!(latex, "\\underset{n}{\\underbrace{a+b}}");
}

#[test]
fn test_nary_sum() {
    let latex = omml_to_latex(
        r#"<m:nary>
                <m:naryPr><m:chr m:val="∑"/></m:naryPr>
                <m:sub><m:r><m:t>i=1</m:t></m:r></m:sub>
                <m:sup><m:r><m:t>n</m:t></m:r></m:sup>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:nary>"#,
    );
    assert_eq!(latex, "\\sum_{i=1}^{n}{x}");
}

#[test]
fn test_nary_character_reference() {
    let latex = omml_to_latex(
        r#"<m:nary>
                <m:naryPr><m:chr m:val="&#x2211;"/></m:naryPr>
                <m:sub><m:r><m:t>i=1</m:t></m:r></m:sub>
                <m:sup><m:r><m:t>n</m:t></m:r></m:sup>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:nary>"#,
    );
    assert_eq!(latex, "\\sum_{i=1}^{n}{x}");
}

#[test]
fn test_delim_parens() {
    let latex = omml_to_latex(
        r#"<m:d>
                <m:e><m:r><m:t>x+y</m:t></m:r></m:e>
            </m:d>"#,
    );
    assert_eq!(latex, "\\left(x+y\\right)");
}

#[test]
fn test_delim_brackets() {
    let latex = omml_to_latex(
        r#"<m:d>
                <m:dPr><m:begChr m:val="["/><m:endChr m:val="]"/></m:dPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:d>"#,
    );
    assert_eq!(latex, "\\left[x\\right]");
}

#[test]
fn test_acc_hat() {
    let latex = omml_to_latex(
        r#"<m:acc>
                <m:accPr><m:chr m:val="̂"/></m:accPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:acc>"#,
    );
    assert_eq!(latex, "\\hat{x}");
}

#[test]
fn test_bar_overline() {
    let latex = omml_to_latex(
        r#"<m:bar>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:bar>"#,
    );
    assert_eq!(latex, "\\overline{x}");
}

#[test]
fn test_bar_underline() {
    let latex = omml_to_latex(
        r#"<m:bar>
                <m:barPr><m:pos m:val="bot"/></m:barPr>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:bar>"#,
    );
    assert_eq!(latex, "\\underline{x}");
}

#[test]
fn test_borderbox() {
    let latex = omml_to_latex(
        r#"<m:borderBox>
                <m:e><m:r><m:t>E=mc</m:t></m:r></m:e>
            </m:borderBox>"#,
    );
    assert_eq!(latex, "\\boxed{E=mc}");
}

#[test]
fn test_matrix() {
    let latex = omml_to_latex(
        r#"<m:m>
                <m:mr>
                    <m:e><m:r><m:t>a</m:t></m:r></m:e>
                    <m:e><m:r><m:t>b</m:t></m:r></m:e>
                </m:mr>
                <m:mr>
                    <m:e><m:r><m:t>c</m:t></m:r></m:e>
                    <m:e><m:r><m:t>d</m:t></m:r></m:e>
                </m:mr>
            </m:m>"#,
    );
    assert_eq!(latex, "\\begin{matrix}a & b \\\\ c & d\\end{matrix}");
}

#[test]
fn test_eqarr() {
    let latex = omml_to_latex(
        r#"<m:eqArr>
                <m:e><m:r><m:t>x=1</m:t></m:r></m:e>
                <m:e><m:r><m:t>y=2</m:t></m:r></m:e>
            </m:eqArr>"#,
    );
    assert_eq!(latex, "\\begin{aligned}x=1 \\\\ y=2\\end{aligned}");
}

#[test]
fn test_func() {
    let latex = omml_to_latex(
        r#"<m:func>
                <m:fName><m:r><m:t>sin</m:t></m:r></m:fName>
                <m:e><m:r><m:t>x</m:t></m:r></m:e>
            </m:func>"#,
    );
    assert_eq!(latex, "\\sin{x}");
}

#[test]
fn test_limlow() {
    let latex = omml_to_latex(
        r#"<m:limLow>
                <m:e><m:r><m:t>lim</m:t></m:r></m:e>
                <m:lim><m:r><m:t>n→∞</m:t></m:r></m:lim>
            </m:limLow>"#,
    );
    assert_eq!(latex, "\\lim_{n\\rightarrow \\infty }");
}

#[test]
fn test_nested_quadratic_formula() {
    let latex = omml_to_latex(
        r#"<m:r><m:t>x=</m:t></m:r>
            <m:f>
                <m:num>
                    <m:r><m:t>-b</m:t></m:r>
                    <m:r><m:t>±</m:t></m:r>
                    <m:rad>
                        <m:radPr><m:degHide m:val="1"/></m:radPr>
                        <m:deg/>
                        <m:e>
                            <m:sSup>
                                <m:e><m:r><m:t>b</m:t></m:r></m:e>
                                <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                            </m:sSup>
                            <m:r><m:t>-4ac</m:t></m:r>
                        </m:e>
                    </m:rad>
                </m:num>
                <m:den>
                    <m:r><m:t>2a</m:t></m:r>
                </m:den>
            </m:f>"#,
    );
    assert_eq!(latex, "x=\\frac{-b\\pm \\sqrt{b^{2}-4ac}}{2a}");
}

#[test]
fn test_omath_para_display() {
    let xml = r#"<m:oMathPara><m:oMath><m:r><m:t>E=mc</m:t></m:r><m:sSup><m:e><m:r><m:t/></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath></m:oMathPara>"#;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "m:oMathPara" => break,
            Ok(Event::Eof) => panic!("unexpected EOF"),
            _ => {}
        }
        buf.clear();
    }
    let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
    let latex = collect_and_convert_omath_para(&mut reader, &mut budget).expect("conversion ok");
    assert!(latex.contains("E=mc"));
    assert!(latex.contains("^{2}"));
}

#[test]
fn test_run_with_rpr() {
    let latex = omml_to_latex(r#"<m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>x</m:t></m:r>"#);
    assert_eq!(latex, "x");
}

#[test]
fn test_nary_integral_default() {
    let latex = omml_to_latex(
        r#"<m:nary>
                <m:naryPr/>
                <m:sub><m:r><m:t>0</m:t></m:r></m:sub>
                <m:sup><m:r><m:t>1</m:t></m:r></m:sup>
                <m:e><m:r><m:t>f(x)dx</m:t></m:r></m:e>
            </m:nary>"#,
    );
    assert_eq!(latex, "\\int_{0}^{1}{f(x)dx}");
}

#[test]
fn test_spre() {
    let latex = omml_to_latex(
        r#"<m:sPre>
                <m:sub><m:r><m:t>2</m:t></m:r></m:sub>
                <m:sup><m:r><m:t>3</m:t></m:r></m:sup>
                <m:e><m:r><m:t>X</m:t></m:r></m:e>
            </m:sPre>"#,
    );
    assert_eq!(latex, "{}_{2}^{3}X");
}

#[test]
fn test_delim_multiple_elements() {
    let latex = omml_to_latex(
        r#"<m:d>
                <m:e><m:r><m:t>a</m:t></m:r></m:e>
                <m:e><m:r><m:t>b</m:t></m:r></m:e>
            </m:d>"#,
    );
    assert_eq!(latex, "\\left(a \\mid b\\right)");
}

#[test]
fn should_reject_group_chr_properties_exceeding_xml_depth() {
    let mut budget = budget_with_max_depth(4);
    budget.enter().unwrap();
    let mut reader = Reader::from_str(
        "<m:groupChr><m:groupChrPr><m:ctrlPr><w:rPr><w:b></w:b></w:rPr></m:ctrlPr></m:groupChrPr></m:groupChr>",
    );
    assert!(matches!(
        collect_and_convert_omath(&mut reader, &mut budget),
        Err(SecurityError::NestingTooDeep { max: 4, .. })
    ));
}

#[test]
fn should_reject_group_character_exceeding_entity_limit() {
    let limits = crate::extractors::security::SecurityLimits {
        max_entity_length: 3,
        ..Default::default()
    };
    let mut budget = SecurityBudget::from_limits(&limits);
    budget.enter().unwrap();
    let mut reader = Reader::from_str(r#"<m:groupChr><m:groupChrPr><m:chr m:val="abcd"/></m:groupChrPr></m:groupChr>"#);
    assert!(matches!(
        collect_and_convert_omath(&mut reader, &mut budget),
        Err(SecurityError::EntityTooLong { max: 3, .. })
    ));
}

#[test]
fn should_account_group_character_in_content_limit() {
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: 3,
        ..Default::default()
    };
    let mut budget = SecurityBudget::from_limits(&limits);
    budget.enter().unwrap();
    let mut reader = Reader::from_str(r#"<m:groupChr><m:groupChrPr><m:chr m:val="abcd"/></m:groupChrPr></m:groupChr>"#);
    assert!(matches!(
        collect_and_convert_omath(&mut reader, &mut budget),
        Err(SecurityError::ContentTooLarge { max: 3, .. })
    ));
}

#[test]
fn should_parse_all_radical_toggle_spellings() {
    for (value, expected) in [
        ("0", "\\sqrt[3]{x}"),
        ("false", "\\sqrt[3]{x}"),
        ("off", "\\sqrt[3]{x}"),
        ("1", "\\sqrt{x}"),
        ("true", "\\sqrt{x}"),
        ("on", "\\sqrt{x}"),
    ] {
        let xml = format!(
            r#"<m:rad><m:radPr><m:degHide m:val="{value}"/></m:radPr><m:deg><m:r><m:t>3</m:t></m:r></m:deg><m:e><m:r><m:t>x</m:t></m:r></m:e></m:rad>"#
        );
        assert_eq!(omml_to_latex(&xml), expected, "toggle {value}");
    }
}

#[test]
fn should_keep_group_character_position_independent_of_vertical_justification() {
    for (position, expected) in [("top", "\\overbrace{x}"), ("bot", "\\underbrace{x}")] {
        for justification in ["top", "bot"] {
            let xml = format!(
                r#"<m:groupChr><m:groupChrPr><m:pos m:val="{position}"/><m:vertJc m:val="{justification}"/></m:groupChrPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:groupChr>"#
            );
            assert_eq!(omml_to_latex(&xml), expected);
        }
    }
}

fn mathml_to_latex(body: &str) -> String {
    let xml = format!(r#"<math xmlns="http://www.w3.org/1998/Math/MathML">{body}</math>"#);
    let mut budget = SecurityBudget::with_defaults();
    let latex = crate::extraction::mathml::convert_mathml_str_to_latex(&xml, &mut budget).expect("conversion ok");
    latex.trim().to_string()
}

fn omml_func(name: &str, body: &str) -> String {
    format!("<m:func><m:fName>{name}</m:fName><m:e><m:r><m:t>{body}</m:t></m:r></m:e></m:func>")
}

fn omml_lim_low(base: &str, lim: &str) -> String {
    format!("<m:limLow><m:e><m:r><m:t>{base}</m:t></m:r></m:e><m:lim><m:r><m:t>{lim}</m:t></m:r></m:lim></m:limLow>")
}

fn omml_nary(chr: &str, pr: &str) -> String {
    format!(
        r#"<m:nary><m:naryPr><m:chr m:val="{chr}"/>{pr}</m:naryPr>
            <m:sub><m:r><m:t>a</m:t></m:r></m:sub><m:sup><m:r><m:t>b</m:t></m:r></m:sup>
            <m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>"#
    )
}

#[test]
fn should_render_every_shared_function_name_as_its_command() {
    let names = [
        "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "coth", "exp",
        "ln", "lg", "log", "deg", "dim", "hom", "ker", "arg", "lim", "liminf", "limsup", "max", "min", "sup", "inf",
        "det", "gcd", "Pr",
    ];
    for name in names {
        let latex = omml_to_latex(&omml_func(&format!("<m:r><m:t>{name}</m:t></m:r>"), "x"));
        assert_eq!(latex, format!("\\{name}{{x}}"));
    }
    let latex = omml_to_latex(&omml_func("<m:r><m:t>lim inf</m:t></m:r>", "x"));
    assert_eq!(latex, "\\liminf{x}");
    let latex = omml_to_latex(&omml_func("<m:r><m:t>foo</m:t></m:r>", "x"));
    assert_eq!(latex, "\\mathrm{foo}{x}");
}

#[test]
fn should_render_a_limit_under_a_function_name_as_limits() {
    let latex = omml_to_latex(&omml_func(&omml_lim_low("lim", "n&#x2192;&#x221E;"), "f(n)"));
    assert_eq!(latex, "\\lim_{n\\rightarrow \\infty }{f(n)}");
    let nested = omml_func("<m:r><m:t>sin</m:t></m:r>", "x");
    let latex = omml_to_latex(&format!(
        "<m:func><m:fName>{}</m:fName><m:e>{nested}</m:e></m:func>",
        omml_lim_low("max", "x")
    ));
    assert_eq!(latex, "\\max_{x}{\\sin{x}}");
    let latex = omml_to_latex(&omml_func(&omml_lim_low("f", "x"), "y"));
    assert_eq!(latex, "\\mathrm{\\underset{x}{f}}{y}");
}

#[test]
fn should_render_limits_on_names_and_big_operators_like_mathml() {
    for (base, op) in [
        ("lim", "\\lim"),
        ("liminf", "\\liminf"),
        ("limsup", "\\limsup"),
        ("max", "\\max"),
        ("min", "\\min"),
        ("sup", "\\sup"),
        ("inf", "\\inf"),
        ("det", "\\det"),
        ("gcd", "\\gcd"),
        ("Pr", "\\Pr"),
        ("&#x2211;", "\\sum"),
        ("&#x220F;", "\\prod"),
        ("&#x22C3;", "\\bigcup"),
        ("&#x222B;", "\\int\\limits"),
        ("&#x222E;", "\\oint\\limits"),
    ] {
        let lower = omml_to_latex(&omml_lim_low(base, "n"));
        assert_eq!(lower, format!("{op}_{{n}}"));
        assert_eq!(
            lower,
            mathml_to_latex(&format!("<munder><mo>{base}</mo><mi>n</mi></munder>"))
        );
        let upper = omml_to_latex(&format!(
            "<m:limUpp><m:e><m:r><m:t>{base}</m:t></m:r></m:e><m:lim><m:r><m:t>n</m:t></m:r></m:lim></m:limUpp>"
        ));
        assert_eq!(upper, format!("{op}^{{n}}"));
        assert_eq!(
            upper,
            mathml_to_latex(&format!("<mover><mo>{base}</mo><mi>n</mi></mover>"))
        );
    }
    assert_eq!(omml_to_latex(&omml_lim_low("sin", "n")), "\\underset{n}{sin}");
    assert_eq!(omml_to_latex(&omml_lim_low("x", "n")), "\\underset{n}{x}");
}

#[test]
fn should_stack_integral_limits_only_when_lim_loc_is_und_ovr() {
    let cases = [
        ("&#x2211;", "\\sum", "\\sum"),
        ("&#x220F;", "\\prod", "\\prod"),
        ("&#x222B;", "\\int\\limits", "\\int"),
        ("&#x222E;", "\\oint\\limits", "\\oint"),
    ];
    for (chr, stacked, side) in cases {
        let und_ovr = omml_to_latex(&omml_nary(chr, r#"<m:limLoc m:val="undOvr"/>"#));
        assert_eq!(und_ovr, format!("{stacked}_{{a}}^{{b}}{{x}}"));
        let mathml = mathml_to_latex(&format!("<munderover><mo>{chr}</mo><mi>a</mi><mi>b</mi></munderover>"));
        assert_eq!(und_ovr, format!("{mathml}{{x}}"));
        let sub_sup = omml_to_latex(&omml_nary(chr, r#"<m:limLoc m:val="subSup"/>"#));
        assert_eq!(sub_sup, format!("{side}_{{a}}^{{b}}{{x}}"));
        assert_eq!(omml_to_latex(&omml_nary(chr, "")), sub_sup);
    }
    let und_ovr = r#"<m:limLoc m:val="undOvr"/>"#;
    let sub_hidden = omml_to_latex(&omml_nary("&#x222B;", &format!(r#"{und_ovr}<m:subHide m:val="1"/>"#)));
    assert_eq!(sub_hidden, "\\int\\limits^{b}{x}");
    let sup_hidden = omml_to_latex(&omml_nary("&#x222B;", &format!(r#"{und_ovr}<m:supHide m:val="1"/>"#)));
    assert_eq!(sup_hidden, "\\int\\limits_{a}{x}");
    let both_hidden = omml_to_latex(&omml_nary(
        "&#x222B;",
        &format!(r#"{und_ovr}<m:subHide m:val="1"/><m:supHide m:val="1"/>"#),
    ));
    assert_eq!(both_hidden, "\\int{x}");
}

/// `levels` nested `<m:sSup><m:e>` pairs around one run, wrapped in `<m:oMath>`.
fn nested_ssup_xml(levels: usize) -> String {
    let mut xml = String::from("<m:oMath>");
    xml.push_str(&"<m:sSup><m:e>".repeat(levels));
    xml.push_str("<m:r><m:t>x</m:t></m:r>");
    xml.push_str(&"</m:e></m:sSup>".repeat(levels));
    xml.push_str("</m:oMath>");
    xml
}

/// Run the converter with default limits on a thread with a small stack, like a
/// tokio worker or a debug test thread.
fn convert_on_small_stack(xml: String) -> Result<String, SecurityError> {
    const SMALL_STACK: usize = 2 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(move || {
            let mut reader = Reader::from_str(&xml);
            reader.config_mut().trim_text(false);
            let mut buf = Vec::new();
            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) if e.name().as_ref() == "m:oMath" => break,
                    Ok(Event::Eof) => panic!("no <m:oMath>"),
                    _ => {}
                }
                buf.clear();
            }
            let mut budget = SecurityBudget::with_defaults();
            budget.enter().expect("depth for <m:oMath>");
            collect_and_convert_omath(&mut reader, &mut budget)
        })
        .expect("spawn")
        .join()
        .expect("converter must not overflow the stack")
}

#[test]
fn should_drop_formula_nested_200000_deep_without_overflowing_the_stack() {
    assert_eq!(convert_on_small_stack(nested_ssup_xml(200_000)).unwrap(), "");
}

#[test]
fn should_drop_formula_just_past_the_nesting_cap() {
    assert_eq!(convert_on_small_stack(nested_ssup_xml(MAX_MATH_NESTING)).unwrap(), "");
}

#[test]
fn should_convert_formula_nested_50_deep() {
    let latex = convert_on_small_stack(nested_ssup_xml(50)).unwrap();
    assert_eq!(latex, format!("{{x^{{}}}}{}", "^{}".repeat(49)));
}

#[test]
fn should_convert_formula_at_the_nesting_cap() {
    let latex = convert_on_small_stack(nested_ssup_xml(MAX_MATH_NESTING - 1)).unwrap();
    assert_eq!(latex, format!("{{x^{{}}}}{}", "^{}".repeat(MAX_MATH_NESTING - 2)));
}

#[test]
fn should_keep_converting_after_a_dropped_deep_formula() {
    assert_eq!(convert_on_small_stack(nested_ssup_xml(MAX_MATH_NESTING)).unwrap(), "");
    assert_eq!(omml_to_latex("<m:r><m:t>y</m:t></m:r>"), "y");
}
