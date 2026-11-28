use super::*;

// An OOXML producer may bind any prefix to a namespace, a default namespace, or the strict
// (ISO 29500) URIs. The same slide must extract identically under every spelling (#3467).

const PML: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const DML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const MATH: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
const STRICT_PML: &str = "http://purl.oclc.org/ooxml/presentationml/main";
const STRICT_DML: &str = "http://purl.oclc.org/ooxml/drawingml/main";
const STRICT_REL: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";
const STRICT_MATH: &str = "http://purl.oclc.org/ooxml/officeDocument/math";

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82";

struct Uris {
    pml: &'static str,
    dml: &'static str,
    rel: &'static str,
    math: &'static str,
}

const TRANSITIONAL: Uris = Uris {
    pml: PML,
    dml: DML,
    rel: REL,
    math: MATH,
};
const STRICT: Uris = Uris {
    pml: STRICT_PML,
    dml: STRICT_DML,
    rel: STRICT_REL,
    math: STRICT_MATH,
};

/// Slide with one display equation, one inline equation, a hyperlink run and a picture,
/// every element spelled with the given prefixes.
fn prefixed_slide(p: &str, a: &str, r: &str, m: &str, uris: &Uris) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<{p}:sld xmlns:{p}="{pml}" xmlns:{a}="{dml}" xmlns:{r}="{rel}" xmlns:{m}="{math}">
  <{p}:cSld><{p}:spTree>
    <{p}:sp><{p}:txBody>
      <{a}:p>
        <{m}:oMathPara><{m}:oMath><{m}:sSup>
          <{m}:e><{m}:r><{m}:t>x</{m}:t></{m}:r></{m}:e>
          <{m}:sup><{m}:r><{m}:t>2</{m}:t></{m}:r></{m}:sup>
        </{m}:sSup></{m}:oMath></{m}:oMathPara>
      </{a}:p>
      <{a}:p>
        <{a}:r><{a}:rPr><{a}:hlinkClick {r}:id="rId5"/></{a}:rPr><{a}:t>Link text</{a}:t></{a}:r>
        <{m}:oMath><{m}:r><{m}:t>a</{m}:t></{m}:r></{m}:oMath>
      </{a}:p>
    </{p}:txBody></{p}:sp>
    <{p}:pic>
      <{p}:nvPicPr><{p}:cNvPr id="2" name="Pic" descr="Alias picture"/><{p}:cNvPicPr/><{p}:nvPr/></{p}:nvPicPr>
      <{p}:blipFill><{a}:blip {r}:embed="rId2"/></{p}:blipFill>
      <{p}:spPr><{a}:xfrm><{a}:off x="0" y="0"/><{a}:ext cx="952500" cy="952500"/></{a}:xfrm></{p}:spPr>
    </{p}:pic>
  </{p}:spTree></{p}:cSld>
</{p}:sld>"#,
        pml = uris.pml,
        dml = uris.dml,
        rel = uris.rel,
        math = uris.math,
    )
}

/// Same slide with PresentationML as the default namespace and DrawingML/OMML re-declared as
/// the default on the elements that use them; relationship attributes need a prefix.
fn default_namespace_slide() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sld xmlns="{PML}" xmlns:rel="{REL}">
  <cSld><spTree>
    <sp><txBody>
      <p xmlns="{DML}">
        <oMathPara xmlns="{MATH}"><oMath><sSup>
          <e><r><t>x</t></r></e>
          <sup><r><t>2</t></r></sup>
        </sSup></oMath></oMathPara>
      </p>
      <p xmlns="{DML}">
        <r><rPr><hlinkClick rel:id="rId5"/></rPr><t>Link text</t></r>
        <oMath xmlns="{MATH}"><r><t>a</t></r></oMath>
      </p>
    </txBody></sp>
    <pic>
      <nvPicPr><cNvPr id="2" name="Pic" descr="Alias picture"/><cNvPicPr/><nvPr/></nvPicPr>
      <blipFill><blip xmlns="{DML}" rel:embed="rId2"/></blipFill>
      <spPr><xfrm xmlns="{DML}"><off x="0" y="0"/><ext cx="952500" cy="952500"/></xfrm></spPr>
    </pic>
  </spTree></cSld>
</sld>"#
    )
}

fn slide_rels(rel_base: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="{rel_base}/image" Target="../media/image1.png"/>
  <Relationship Id="rId5" Type="{rel_base}/hyperlink" Target="https://example.com/linked" TargetMode="External"/>
</Relationships>"#
    )
}

/// Formulas, picture, hyperlink and text the slide must yield whatever its namespace spelling.
fn extract_summary(slide_xml: &str, rel_base: &str) -> String {
    let pptx = build_single_slide_pptx(slide_xml, Some(&slide_rels(rel_base)), &[("ppt/media/image1.png", PNG)]);
    let extraction =
        super::extract_pptx_from_bytes_with_slide_contents(&pptx, &PptxExtractionOptions::default(), &mut Vec::new())
            .unwrap();
    let result = &extraction.result;
    format!(
        "formulas={:?}\nimages={:?}\nhyperlinks={:?}\ncontains_link_text={}",
        extraction.formulas,
        result
            .images
            .iter()
            .map(|i| (i.description.clone(), i.width))
            .collect::<Vec<_>>(),
        result.hyperlinks,
        result.content.contains("Link text"),
    )
}

fn assert_expected(summary: &str, spelling: &str) {
    assert!(
        summary.contains(r#"formulas=[("x^{2}", true), ("a", false)]"#),
        "{spelling}: equations lost: {summary}"
    );
    assert!(
        summary.contains(r#"images=[(Some("Alias picture"), Some(100))]"#),
        "{spelling}: picture lost: {summary}"
    );
    assert!(
        summary.contains("https://example.com/linked"),
        "{spelling}: hyperlink lost: {summary}"
    );
    assert!(
        summary.contains("contains_link_text=true"),
        "{spelling}: text lost: {summary}"
    );
}

#[test]
fn test_namespace_spelling_does_not_change_extraction() {
    let rel_base = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let strict_rel_base = "http://purl.oclc.org/ooxml/officeDocument/relationships";

    let conventional = extract_summary(&prefixed_slide("p", "a", "r", "m", &TRANSITIONAL), rel_base);
    assert_expected(&conventional, "conventional prefixes");

    let alias = extract_summary(&prefixed_slide("pml", "dml", "rel", "mth", &TRANSITIONAL), rel_base);
    assert_eq!(alias, conventional, "alias prefixes");

    let swapped = extract_summary(&prefixed_slide("m", "r", "p", "a", &TRANSITIONAL), rel_base);
    assert_eq!(
        swapped, conventional,
        "conventional prefixes bound to the wrong namespaces"
    );

    let default_ns = extract_summary(&default_namespace_slide(), rel_base);
    assert_eq!(default_ns, conventional, "default namespaces");

    let strict = extract_summary(&prefixed_slide("p", "a", "r", "m", &STRICT), strict_rel_base);
    assert_eq!(strict, conventional, "strict namespaces");

    let strict_alias = extract_summary(&prefixed_slide("pml", "dml", "rel", "mth", &STRICT), strict_rel_base);
    assert_eq!(strict_alias, conventional, "strict namespaces under alias prefixes");
}
