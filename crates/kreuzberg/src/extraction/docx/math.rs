//! OMML (Office Math Markup Language) to LaTeX converter.
//!
//! Converts OMML math elements found in DOCX files to LaTeX notation.
//! When the streaming parser encounters `m:oMathPara` or `m:oMath`, it
//! delegates here. We collect the subtree into a `MathNode` tree, then
//! recursively render it to LaTeX.

use quick_xml::Reader;
use quick_xml::events::Event;

// --- MathNode tree ---

#[derive(Debug, Clone)]
pub enum FracType {
    Bar,
    NoBar,
    Linear,
    Skewed,
}

#[derive(Debug, Clone)]
pub enum MathNode {
    /// Plain text from m:r/m:t
    Run(String),
    /// Superscript: base^{sup}
    SSup { base: Vec<MathNode>, sup: Vec<MathNode> },
    /// Subscript: base_{sub}
    SSub { base: Vec<MathNode>, sub: Vec<MathNode> },
    /// Sub-superscript: base_{sub}^{sup}
    SSubSup {
        base: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
    /// Fraction: \frac{num}{den}
    Frac {
        num: Vec<MathNode>,
        den: Vec<MathNode>,
        frac_type: FracType,
    },
    /// Radical: \sqrt{body} or \sqrt[deg]{body}
    Rad {
        deg: Vec<MathNode>,
        body: Vec<MathNode>,
        deg_hide: bool,
    },
    /// N-ary operator: \sum_{sub}^{sup}{body}
    Nary {
        chr: String,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
        body: Vec<MathNode>,
        sub_hide: bool,
        sup_hide: bool,
    },
    /// Delimiter: \left( ... \right)
    Delim {
        begin_chr: String,
        end_chr: String,
        sep_chr: String,
        elements: Vec<Vec<MathNode>>,
    },
    /// Function: \funcname{body}
    Func { name: Vec<MathNode>, body: Vec<MathNode> },
    /// Accent: \hat{body}
    Acc { chr: String, body: Vec<MathNode> },
    /// Equation array: \begin{aligned}...\end{aligned}
    EqArr { rows: Vec<Vec<MathNode>> },
    /// Lower limit: \underset{lim}{body}
    LimLow { body: Vec<MathNode>, lim: Vec<MathNode> },
    /// Upper limit: \overset{lim}{body}
    LimUpp { body: Vec<MathNode>, lim: Vec<MathNode> },
    /// Bar (overline/underline)
    Bar { body: Vec<MathNode>, top: bool },
    /// Border box: \boxed{body}
    BorderBox { body: Vec<MathNode> },
    /// Matrix: \begin{matrix}...\end{matrix}
    Matrix { rows: Vec<Vec<Vec<MathNode>>> },
    /// Grouping container (m:box, m:phant, etc.) — passes through children
    Group { children: Vec<MathNode> },
    /// Pre-sub-superscript: {}_{sub}^{sup}{base}
    SPre {
        base: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
}

// --- Public entry points ---

/// Collect an `m:oMathPara` subtree and convert to LaTeX (display math).
/// The reader should be positioned right after the `<m:oMathPara>` start tag.
pub fn collect_and_convert_omath_para(reader: &mut Reader<&[u8]>) -> String {
    let children = collect_children(reader, b"m:oMathPara");
    // An oMathPara may contain multiple oMath elements; render each.
    let mut parts = Vec::new();
    for child in &children {
        if let MathNode::Group { children: inner } = child {
            // This is the result of an m:oMath inside m:oMathPara
            let rendered = render_nodes(inner);
            if !rendered.is_empty() {
                parts.push(rendered);
            }
        }
    }
    if parts.is_empty() {
        // Fallback: render all children directly
        render_nodes(&children)
    } else {
        parts.join(" \\\\ ")
    }
}

/// Collect an `m:oMath` subtree and convert to LaTeX (inline math).
/// The reader should be positioned right after the `<m:oMath>` start tag.
pub fn collect_and_convert_omath(reader: &mut Reader<&[u8]>) -> String {
    let children = collect_children(reader, b"m:oMath");
    render_nodes(&children)
}

// --- Tree builder ---

/// Recursively collect child nodes until the matching close tag.
fn collect_children(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Vec<MathNode> {
    let mut nodes = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = e.name().as_ref().to_vec();
                match tag.as_slice() {
                    b"m:r" => {
                        nodes.push(collect_run(reader));
                    }
                    b"m:sSup" => {
                        nodes.push(collect_ssup(reader));
                    }
                    b"m:sSub" => {
                        nodes.push(collect_ssub(reader));
                    }
                    b"m:sSubSup" => {
                        nodes.push(collect_ssubsup(reader));
                    }
                    b"m:f" => {
                        nodes.push(collect_frac(reader));
                    }
                    b"m:rad" => {
                        nodes.push(collect_rad(reader));
                    }
                    b"m:nary" => {
                        nodes.push(collect_nary(reader));
                    }
                    b"m:d" => {
                        nodes.push(collect_delim(reader));
                    }
                    b"m:func" => {
                        nodes.push(collect_func(reader));
                    }
                    b"m:acc" => {
                        nodes.push(collect_acc(reader));
                    }
                    b"m:eqArr" => {
                        nodes.push(collect_eqarr(reader));
                    }
                    b"m:limLow" => {
                        nodes.push(collect_limlow(reader));
                    }
                    b"m:limUpp" => {
                        nodes.push(collect_limupp(reader));
                    }
                    b"m:bar" => {
                        nodes.push(collect_bar(reader));
                    }
                    b"m:borderBox" => {
                        nodes.push(collect_borderbox(reader));
                    }
                    b"m:m" => {
                        nodes.push(collect_matrix(reader));
                    }
                    b"m:box" | b"m:phant" => {
                        let children = collect_element_body(reader, &tag);
                        nodes.push(MathNode::Group { children });
                    }
                    b"m:sPre" => {
                        nodes.push(collect_spre(reader));
                    }
                    b"m:oMath" => {
                        // Nested oMath (e.g. inside oMathPara)
                        let inner = collect_children(reader, b"m:oMath");
                        nodes.push(MathNode::Group { children: inner });
                    }
                    _ => {
                        // Unknown element — skip it entirely
                        skip_to_end(reader, &tag);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == end_tag {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    nodes
}

/// Collect text from an m:r element (reads until </m:r>).
fn collect_run(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:t" => in_text = true,
                b"m:rPr" => skip_to_end(reader, b"m:rPr"),
                _ => {}
            },
            Ok(Event::Text(ref e)) => {
                if in_text && let Ok(t) = e.decode() {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"m:t" => in_text = false,
                b"m:r" => break,
                _ => {}
            },
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Run(text)
}

/// Collect an m:sSup (superscript) element.
fn collect_ssup(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut base = Vec::new();
    let mut sup = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => base = collect_children(reader, b"m:e"),
                b"m:sup" => sup = collect_children(reader, b"m:sup"),
                b"m:sSupPr" => skip_to_end(reader, b"m:sSupPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:sSup" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::SSup { base, sup }
}

/// Collect an m:sSub (subscript) element.
fn collect_ssub(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut base = Vec::new();
    let mut sub = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => base = collect_children(reader, b"m:e"),
                b"m:sub" => sub = collect_children(reader, b"m:sub"),
                b"m:sSubPr" => skip_to_end(reader, b"m:sSubPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:sSub" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::SSub { base, sub }
}

/// Collect an m:sSubSup element.
fn collect_ssubsup(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut base = Vec::new();
    let mut sub = Vec::new();
    let mut sup = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => base = collect_children(reader, b"m:e"),
                b"m:sub" => sub = collect_children(reader, b"m:sub"),
                b"m:sup" => sup = collect_children(reader, b"m:sup"),
                b"m:sSubSupPr" => skip_to_end(reader, b"m:sSubSupPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:sSubSup" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::SSubSup { base, sub, sup }
}

/// Collect an m:f (fraction) element.
fn collect_frac(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut num = Vec::new();
    let mut den = Vec::new();
    let mut frac_type = FracType::Bar;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:fPr" => {
                    frac_type = collect_frac_pr(reader);
                }
                b"m:num" => num = collect_children(reader, b"m:num"),
                b"m:den" => den = collect_children(reader, b"m:den"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:f" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Frac { num, den, frac_type }
}

/// Read fraction properties to determine type.
fn collect_frac_pr(reader: &mut Reader<&[u8]>) -> FracType {
    let mut frac_type = FracType::Bar;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == b"m:type"
                    && let Some(val) = get_m_val(e)
                {
                    frac_type = match val.as_str() {
                        "noBar" => FracType::NoBar,
                        "lin" => FracType::Linear,
                        "skw" => FracType::Skewed,
                        _ => FracType::Bar,
                    };
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:fPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    frac_type
}

/// Collect an m:rad (radical/sqrt) element.
fn collect_rad(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut deg = Vec::new();
    let mut body = Vec::new();
    let mut deg_hide = true; // default: no degree shown (plain \sqrt)
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:radPr" => {
                    deg_hide = collect_rad_pr(reader);
                }
                b"m:deg" => deg = collect_children(reader, b"m:deg"),
                b"m:e" => body = collect_children(reader, b"m:e"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:rad" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Rad { deg, body, deg_hide }
}

/// Read radical properties (degHide).
fn collect_rad_pr(reader: &mut Reader<&[u8]>) -> bool {
    let mut deg_hide = true;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == b"m:degHide" {
                    deg_hide = get_m_val(e).as_deref() != Some("0");
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:radPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    deg_hide
}

/// Collect an m:nary (n-ary operator) element.
fn collect_nary(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut chr = "\u{222B}".to_string(); // default: integral
    let mut sub = Vec::new();
    let mut sup = Vec::new();
    let mut body = Vec::new();
    let mut sub_hide = false;
    let mut sup_hide = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:naryPr" => {
                    collect_nary_pr(reader, &mut chr, &mut sub_hide, &mut sup_hide);
                }
                b"m:sub" => sub = collect_children(reader, b"m:sub"),
                b"m:sup" => sup = collect_children(reader, b"m:sup"),
                b"m:e" => body = collect_children(reader, b"m:e"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:nary" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Nary {
        chr,
        sub,
        sup,
        body,
        sub_hide,
        sup_hide,
    }
}

/// Read n-ary properties.
fn collect_nary_pr(reader: &mut Reader<&[u8]>, chr: &mut String, sub_hide: &mut bool, sup_hide: &mut bool) {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                b"m:chr" => {
                    if let Some(val) = get_m_val(e) {
                        *chr = val;
                    }
                }
                b"m:subHide" => {
                    *sub_hide = get_m_val(e).as_deref() != Some("0");
                }
                b"m:supHide" => {
                    *sup_hide = get_m_val(e).as_deref() != Some("0");
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:naryPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Collect an m:d (delimiter) element.
fn collect_delim(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut begin_chr = "(".to_string();
    let mut end_chr = ")".to_string();
    let mut sep_chr = "|".to_string();
    let mut elements: Vec<Vec<MathNode>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:dPr" => {
                    collect_delim_pr(reader, &mut begin_chr, &mut end_chr, &mut sep_chr);
                }
                b"m:e" => {
                    elements.push(collect_children(reader, b"m:e"));
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:d" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Delim {
        begin_chr,
        end_chr,
        sep_chr,
        elements,
    }
}

/// Read delimiter properties.
fn collect_delim_pr(reader: &mut Reader<&[u8]>, begin_chr: &mut String, end_chr: &mut String, sep_chr: &mut String) {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                b"m:begChr" => {
                    if let Some(val) = get_m_val(e) {
                        *begin_chr = val;
                    }
                }
                b"m:endChr" => {
                    if let Some(val) = get_m_val(e) {
                        *end_chr = val;
                    }
                }
                b"m:sepChr" => {
                    if let Some(val) = get_m_val(e) {
                        *sep_chr = val;
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:dPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Collect an m:func element.
fn collect_func(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut name = Vec::new();
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:fName" => name = collect_children(reader, b"m:fName"),
                b"m:e" => body = collect_children(reader, b"m:e"),
                b"m:funcPr" => skip_to_end(reader, b"m:funcPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:func" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Func { name, body }
}

/// Collect an m:acc (accent) element.
fn collect_acc(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut chr = "\u{0302}".to_string(); // default: combining circumflex accent (hat)
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:accPr" => {
                    collect_acc_pr(reader, &mut chr);
                }
                b"m:e" => body = collect_children(reader, b"m:e"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:acc" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Acc { chr, body }
}

/// Read accent properties.
fn collect_acc_pr(reader: &mut Reader<&[u8]>, chr: &mut String) {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == b"m:chr"
                    && let Some(val) = get_m_val(e)
                {
                    *chr = val;
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:accPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Collect an m:eqArr element.
fn collect_eqarr(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut rows = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => rows.push(collect_children(reader, b"m:e")),
                b"m:eqArrPr" => skip_to_end(reader, b"m:eqArrPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:eqArr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::EqArr { rows }
}

/// Collect an m:limLow element.
fn collect_limlow(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut body = Vec::new();
    let mut lim = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => body = collect_children(reader, b"m:e"),
                b"m:lim" => lim = collect_children(reader, b"m:lim"),
                b"m:limLowPr" => skip_to_end(reader, b"m:limLowPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:limLow" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::LimLow { body, lim }
}

/// Collect an m:limUpp element.
fn collect_limupp(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut body = Vec::new();
    let mut lim = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => body = collect_children(reader, b"m:e"),
                b"m:lim" => lim = collect_children(reader, b"m:lim"),
                b"m:limUppPr" => skip_to_end(reader, b"m:limUppPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:limUpp" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::LimUpp { body, lim }
}

/// Collect an m:bar element.
fn collect_bar(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut body = Vec::new();
    let mut top = true; // default: overline
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:barPr" => {
                    top = collect_bar_pr(reader);
                }
                b"m:e" => body = collect_children(reader, b"m:e"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:bar" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Bar { body, top }
}

/// Read bar properties (pos).
fn collect_bar_pr(reader: &mut Reader<&[u8]>) -> bool {
    let mut top = true;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == b"m:pos"
                    && let Some(val) = get_m_val(e)
                {
                    top = val != "bot";
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:barPr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    top
}

/// Collect an m:borderBox element.
fn collect_borderbox(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => body = collect_children(reader, b"m:e"),
                b"m:borderBoxPr" => skip_to_end(reader, b"m:borderBoxPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:borderBox" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::BorderBox { body }
}

/// Collect an m:m (matrix) element.
fn collect_matrix(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut rows: Vec<Vec<Vec<MathNode>>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:mr" => {
                    rows.push(collect_matrix_row(reader));
                }
                b"m:mPr" => skip_to_end(reader, b"m:mPr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:m" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::Matrix { rows }
}

/// Collect a matrix row (m:mr) — returns cells.
fn collect_matrix_row(reader: &mut Reader<&[u8]>) -> Vec<Vec<MathNode>> {
    let mut cells: Vec<Vec<MathNode>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"m:e" {
                    cells.push(collect_children(reader, b"m:e"));
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:mr" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    cells
}

/// Collect an m:sPre (pre-sub-superscript) element.
fn collect_spre(reader: &mut Reader<&[u8]>) -> MathNode {
    let mut base = Vec::new();
    let mut sub = Vec::new();
    let mut sup = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"m:e" => base = collect_children(reader, b"m:e"),
                b"m:sub" => sub = collect_children(reader, b"m:sub"),
                b"m:sup" => sup = collect_children(reader, b"m:sup"),
                b"m:sPrePr" => skip_to_end(reader, b"m:sPrePr"),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"m:sPre" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    MathNode::SPre { base, sub, sup }
}

/// Collect body of a generic element (skip its *Pr, gather m:e children).
fn collect_element_body(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Vec<MathNode> {
    let mut children = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = e.name().as_ref().to_vec();
                if tag.ends_with(b"Pr") {
                    skip_to_end(reader, &tag);
                } else if tag == b"m:e" {
                    children.extend(collect_children(reader, b"m:e"));
                } else {
                    // Try to collect as a math element
                    skip_to_end(reader, &tag);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    children
}

// --- Helpers ---

/// Get the `m:val` attribute value from a start/empty element.
fn get_m_val(e: &quick_xml::events::BytesStart) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        if key == b"m:val" || key == b"val" {
            return std::str::from_utf8(&attr.value).ok().map(|s| s.to_string());
        }
    }
    None
}

/// Skip forward until the matching end tag is consumed.
fn skip_to_end(reader: &mut Reader<&[u8]>, tag: &[u8]) {
    let mut depth = 1u32;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == tag {
                    depth += 1;
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == tag {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

// --- LaTeX renderer ---

/// Render a slice of MathNodes to LaTeX.
fn render_nodes(nodes: &[MathNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        render_node(node, &mut out);
    }
    out
}

/// Render a single MathNode to LaTeX, appending to `out`.
fn render_node(node: &MathNode, out: &mut String) {
    match node {
        MathNode::Run(text) => {
            render_run_text(text, out);
        }
        MathNode::SSup { base, sup } => {
            render_group(base, out);
            out.push_str("^{");
            out.push_str(&render_nodes(sup));
            out.push('}');
        }
        MathNode::SSub { base, sub } => {
            render_group(base, out);
            out.push_str("_{");
            out.push_str(&render_nodes(sub));
            out.push('}');
        }
        MathNode::SSubSup { base, sub, sup } => {
            render_group(base, out);
            out.push_str("_{");
            out.push_str(&render_nodes(sub));
            out.push_str("}^{");
            out.push_str(&render_nodes(sup));
            out.push('}');
        }
        MathNode::Frac { num, den, frac_type } => match frac_type {
            FracType::Bar => {
                out.push_str("\\frac{");
                out.push_str(&render_nodes(num));
                out.push_str("}{");
                out.push_str(&render_nodes(den));
                out.push('}');
            }
            FracType::NoBar => {
                out.push_str("\\binom{");
                out.push_str(&render_nodes(num));
                out.push_str("}{");
                out.push_str(&render_nodes(den));
                out.push('}');
            }
            FracType::Linear | FracType::Skewed => {
                let num_s = render_nodes(num);
                let den_s = render_nodes(den);
                // Wrap in braces if multi-character
                if num_s.len() > 1 {
                    out.push('{');
                    out.push_str(&num_s);
                    out.push('}');
                } else {
                    out.push_str(&num_s);
                }
                out.push('/');
                if den_s.len() > 1 {
                    out.push('{');
                    out.push_str(&den_s);
                    out.push('}');
                } else {
                    out.push_str(&den_s);
                }
            }
        },
        MathNode::Rad { deg, body, deg_hide } => {
            out.push_str("\\sqrt");
            if !*deg_hide && !deg.is_empty() {
                let deg_s = render_nodes(deg);
                if !deg_s.is_empty() {
                    out.push('[');
                    out.push_str(&deg_s);
                    out.push(']');
                }
            }
            out.push('{');
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::Nary {
            chr,
            sub,
            sup,
            body,
            sub_hide,
            sup_hide,
        } => {
            out.push_str(&nary_chr_to_latex(chr));
            if !*sub_hide && !sub.is_empty() {
                out.push_str("_{");
                out.push_str(&render_nodes(sub));
                out.push('}');
            }
            if !*sup_hide && !sup.is_empty() {
                out.push_str("^{");
                out.push_str(&render_nodes(sup));
                out.push('}');
            }
            if !body.is_empty() {
                out.push('{');
                out.push_str(&render_nodes(body));
                out.push('}');
            }
        }
        MathNode::Delim {
            begin_chr,
            end_chr,
            sep_chr,
            elements,
        } => {
            out.push_str("\\left");
            out.push_str(&delim_chr_to_latex(begin_chr));
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(&delim_sep_to_latex(sep_chr));
                }
                out.push_str(&render_nodes(elem));
            }
            out.push_str("\\right");
            out.push_str(&delim_chr_to_latex(end_chr));
        }
        MathNode::Func { name, body } => {
            let func_name = render_nodes(name);
            // Check if it's a known LaTeX function
            let latex_func = match func_name.trim() {
                "sin" => "\\sin",
                "cos" => "\\cos",
                "tan" => "\\tan",
                "cot" => "\\cot",
                "sec" => "\\sec",
                "csc" => "\\csc",
                "log" => "\\log",
                "ln" => "\\ln",
                "exp" => "\\exp",
                "lim" => "\\lim",
                "max" => "\\max",
                "min" => "\\min",
                "sup" => "\\sup",
                "inf" => "\\inf",
                "det" => "\\det",
                "gcd" => "\\gcd",
                "deg" => "\\deg",
                "dim" => "\\dim",
                "hom" => "\\hom",
                "ker" => "\\ker",
                "arg" => "\\arg",
                "sinh" => "\\sinh",
                "cosh" => "\\cosh",
                "tanh" => "\\tanh",
                _ => "",
            };
            if !latex_func.is_empty() {
                out.push_str(latex_func);
            } else {
                out.push_str("\\mathrm{");
                out.push_str(&func_name);
                out.push('}');
            }
            out.push('{');
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::Acc { chr, body } => {
            out.push_str(&accent_chr_to_latex(chr));
            out.push('{');
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::EqArr { rows } => {
            out.push_str("\\begin{aligned}");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    out.push_str(" \\\\ ");
                }
                out.push_str(&render_nodes(row));
            }
            out.push_str("\\end{aligned}");
        }
        MathNode::LimLow { body, lim } => {
            out.push_str("\\underset{");
            out.push_str(&render_nodes(lim));
            out.push_str("}{");
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::LimUpp { body, lim } => {
            out.push_str("\\overset{");
            out.push_str(&render_nodes(lim));
            out.push_str("}{");
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::Bar { body, top } => {
            if *top {
                out.push_str("\\overline{");
            } else {
                out.push_str("\\underline{");
            }
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::BorderBox { body } => {
            out.push_str("\\boxed{");
            out.push_str(&render_nodes(body));
            out.push('}');
        }
        MathNode::Matrix { rows } => {
            out.push_str("\\begin{matrix}");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    out.push_str(" \\\\ ");
                }
                for (j, cell) in row.iter().enumerate() {
                    if j > 0 {
                        out.push_str(" & ");
                    }
                    out.push_str(&render_nodes(cell));
                }
            }
            out.push_str("\\end{matrix}");
        }
        MathNode::Group { children } => {
            out.push_str(&render_nodes(children));
        }
        MathNode::SPre { base, sub, sup } => {
            out.push_str("{}_{");
            out.push_str(&render_nodes(sub));
            out.push_str("}^{");
            out.push_str(&render_nodes(sup));
            out.push('}');
            render_group(base, out);
        }
    }
}

/// Render base nodes, wrapping in braces if needed for subscript/superscript.
fn render_group(nodes: &[MathNode], out: &mut String) {
    let rendered = render_nodes(nodes);
    // Wrap in braces if multi-character or contains special chars
    let needs_braces = rendered.chars().count() > 1 && !rendered.starts_with('\\') && !rendered.starts_with('{');
    if needs_braces {
        out.push('{');
        out.push_str(&rendered);
        out.push('}');
    } else {
        out.push_str(&rendered);
    }
}

/// Render run text, mapping Unicode math symbols to LaTeX commands.
fn render_run_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        if let Some(latex) = unicode_to_latex(ch) {
            out.push_str(latex);
        } else {
            out.push(ch);
        }
    }
}

// --- Character mapping tables ---

/// Map a Unicode character to its LaTeX command (if any).
fn unicode_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        // Greek lowercase
        '\u{03B1}' => Some("\\alpha "),
        '\u{03B2}' => Some("\\beta "),
        '\u{03B3}' => Some("\\gamma "),
        '\u{03B4}' => Some("\\delta "),
        '\u{03B5}' => Some("\\epsilon "),
        '\u{03B6}' => Some("\\zeta "),
        '\u{03B7}' => Some("\\eta "),
        '\u{03B8}' => Some("\\theta "),
        '\u{03B9}' => Some("\\iota "),
        '\u{03BA}' => Some("\\kappa "),
        '\u{03BB}' => Some("\\lambda "),
        '\u{03BC}' => Some("\\mu "),
        '\u{03BD}' => Some("\\nu "),
        '\u{03BE}' => Some("\\xi "),
        '\u{03BF}' => Some("o"), // omicron is just 'o' in LaTeX
        '\u{03C0}' => Some("\\pi "),
        '\u{03C1}' => Some("\\rho "),
        '\u{03C2}' => Some("\\varsigma "),
        '\u{03C3}' => Some("\\sigma "),
        '\u{03C4}' => Some("\\tau "),
        '\u{03C5}' => Some("\\upsilon "),
        '\u{03C6}' => Some("\\phi "),
        '\u{03C7}' => Some("\\chi "),
        '\u{03C8}' => Some("\\psi "),
        '\u{03C9}' => Some("\\omega "),
        // Greek uppercase
        '\u{0393}' => Some("\\Gamma "),
        '\u{0394}' => Some("\\Delta "),
        '\u{0398}' => Some("\\Theta "),
        '\u{039B}' => Some("\\Lambda "),
        '\u{039E}' => Some("\\Xi "),
        '\u{03A0}' => Some("\\Pi "),
        '\u{03A3}' => Some("\\Sigma "),
        '\u{03A5}' => Some("\\Upsilon "),
        '\u{03A6}' => Some("\\Phi "),
        '\u{03A8}' => Some("\\Psi "),
        '\u{03A9}' => Some("\\Omega "),
        // Operators
        '\u{00B1}' => Some("\\pm "),
        '\u{2213}' => Some("\\mp "),
        '\u{00D7}' => Some("\\times "),
        '\u{00F7}' => Some("\\div "),
        '\u{22C5}' => Some("\\cdot "),
        '\u{2217}' => Some("\\ast "),
        '\u{2218}' => Some("\\circ "),
        '\u{2219}' => Some("\\bullet "),
        // Relations
        '\u{2264}' => Some("\\leq "),
        '\u{2265}' => Some("\\geq "),
        '\u{2260}' => Some("\\neq "),
        '\u{2248}' => Some("\\approx "),
        '\u{2261}' => Some("\\equiv "),
        '\u{227A}' => Some("\\prec "),
        '\u{227B}' => Some("\\succ "),
        '\u{2286}' => Some("\\subseteq "),
        '\u{2287}' => Some("\\supseteq "),
        '\u{2282}' => Some("\\subset "),
        '\u{2283}' => Some("\\supset "),
        '\u{2208}' => Some("\\in "),
        '\u{2209}' => Some("\\notin "),
        '\u{220B}' => Some("\\ni "),
        // Arrows
        '\u{2190}' => Some("\\leftarrow "),
        '\u{2192}' => Some("\\rightarrow "),
        '\u{2191}' => Some("\\uparrow "),
        '\u{2193}' => Some("\\downarrow "),
        '\u{2194}' => Some("\\leftrightarrow "),
        '\u{21D0}' => Some("\\Leftarrow "),
        '\u{21D2}' => Some("\\Rightarrow "),
        '\u{21D4}' => Some("\\Leftrightarrow "),
        '\u{21A6}' => Some("\\mapsto "),
        // Special symbols
        '\u{221E}' => Some("\\infty "),
        '\u{2202}' => Some("\\partial "),
        '\u{2207}' => Some("\\nabla "),
        '\u{2200}' => Some("\\forall "),
        '\u{2203}' => Some("\\exists "),
        '\u{2205}' => Some("\\emptyset "),
        '\u{2227}' => Some("\\wedge "),
        '\u{2228}' => Some("\\vee "),
        '\u{00AC}' => Some("\\neg "),
        '\u{2229}' => Some("\\cap "),
        '\u{222A}' => Some("\\cup "),
        '\u{2026}' => Some("\\ldots "),
        '\u{22EF}' => Some("\\cdots "),
        '\u{22EE}' => Some("\\vdots "),
        '\u{22F1}' => Some("\\ddots "),
        '\u{2032}' => Some("'"),
        '\u{2033}' => Some("''"),
        '\u{210F}' => Some("\\hbar "),
        '\u{2113}' => Some("\\ell "),
        '\u{211C}' => Some("\\Re "),
        '\u{2111}' => Some("\\Im "),
        '\u{2118}' => Some("\\wp "),
        '\u{2135}' => Some("\\aleph "),
        // N-ary operators (when used as text)
        '\u{2211}' => Some("\\sum "),
        '\u{220F}' => Some("\\prod "),
        '\u{222B}' => Some("\\int "),
        '\u{222C}' => Some("\\iint "),
        '\u{222D}' => Some("\\iiint "),
        '\u{222E}' => Some("\\oint "),
        '\u{2210}' => Some("\\coprod "),
        '\u{22C0}' => Some("\\bigwedge "),
        '\u{22C1}' => Some("\\bigvee "),
        '\u{22C2}' => Some("\\bigcap "),
        '\u{22C3}' => Some("\\bigcup "),
        _ => None,
    }
}

/// Map n-ary character to LaTeX command.
fn nary_chr_to_latex(chr: &str) -> String {
    if let Some(ch) = chr.chars().next() {
        match ch {
            '\u{2211}' => return "\\sum".to_string(),
            '\u{220F}' => return "\\prod".to_string(),
            '\u{2210}' => return "\\coprod".to_string(),
            '\u{222B}' => return "\\int".to_string(),
            '\u{222C}' => return "\\iint".to_string(),
            '\u{222D}' => return "\\iiint".to_string(),
            '\u{222E}' => return "\\oint".to_string(),
            '\u{22C0}' => return "\\bigwedge".to_string(),
            '\u{22C1}' => return "\\bigvee".to_string(),
            '\u{22C2}' => return "\\bigcap".to_string(),
            '\u{22C3}' => return "\\bigcup".to_string(),
            _ => {}
        }
    }
    // Fallback: use the character directly
    chr.to_string()
}

/// Map delimiter character to LaTeX.
fn delim_chr_to_latex(chr: &str) -> String {
    match chr {
        "(" | ")" | "[" | "]" => chr.to_string(),
        "{" => "\\{".to_string(),
        "}" => "\\}".to_string(),
        "|" => "|".to_string(),
        "\u{2016}" => "\\|".to_string(), // double vertical bar
        "\u{2329}" | "\u{27E8}" => "\\langle".to_string(),
        "\u{232A}" | "\u{27E9}" => "\\rangle".to_string(),
        "\u{230A}" => "\\lfloor".to_string(),
        "\u{230B}" => "\\rfloor".to_string(),
        "\u{2308}" => "\\lceil".to_string(),
        "\u{2309}" => "\\rceil".to_string(),
        "" => ".".to_string(), // empty delimiter
        _ => chr.to_string(),
    }
}

/// Map delimiter separator character to LaTeX.
fn delim_sep_to_latex(sep: &str) -> String {
    match sep {
        "|" => " \\mid ".to_string(),
        _ => sep.to_string(),
    }
}

/// Map accent character to LaTeX command.
fn accent_chr_to_latex(chr: &str) -> String {
    if let Some(ch) = chr.chars().next() {
        match ch {
            '\u{0302}' | '^' => return "\\hat".to_string(),
            '\u{0303}' | '~' => return "\\tilde".to_string(),
            '\u{0304}' | '\u{0305}' => return "\\bar".to_string(),
            '\u{20D7}' | '\u{2192}' => return "\\vec".to_string(),
            '\u{0307}' => return "\\dot".to_string(),
            '\u{0308}' => return "\\ddot".to_string(),
            '\u{030C}' => return "\\check".to_string(),
            '\u{0306}' => return "\\breve".to_string(),
            '\u{0301}' => return "\\acute".to_string(),
            '\u{0300}' => return "\\grave".to_string(),
            _ => {}
        }
    }
    // Fallback
    "\\hat".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse an OMML XML fragment and return rendered LaTeX.
    fn omml_to_latex(xml: &str) -> String {
        let wrapped = format!("<m:oMath>{}</m:oMath>", xml);
        let mut reader = Reader::from_str(&wrapped);
        reader.config_mut().trim_text(false);
        // Skip to the start tag
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"m:oMath" => break,
                Ok(Event::Eof) => return String::new(),
                _ => {}
            }
            buf.clear();
        }
        collect_and_convert_omath(&mut reader)
    }

    #[test]
    fn test_run_plain_text() {
        let latex = omml_to_latex(r#"<m:r><m:t>hello</m:t></m:r>"#);
        assert_eq!(latex, "hello");
    }

    #[test]
    fn test_run_unicode_pi() {
        let latex = omml_to_latex("<m:r><m:t>\u{03C0}</m:t></m:r>");
        assert_eq!(latex, "\\pi ");
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
        assert_eq!(latex, "\\underset{n\\rightarrow \\infty }{lim}");
    }

    #[test]
    fn test_nested_quadratic_formula() {
        // x = \frac{-b \pm \sqrt{b^{2} - 4ac}}{2a}
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
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"m:oMathPara" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        let latex = collect_and_convert_omath_para(&mut reader);
        assert!(latex.contains("E=mc"));
        assert!(latex.contains("^{2}"));
    }

    #[test]
    fn test_run_with_rpr() {
        // m:rPr should be skipped, not treated as text
        let latex = omml_to_latex(r#"<m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>x</m:t></m:r>"#);
        assert_eq!(latex, "x");
    }

    #[test]
    fn test_nary_integral_default() {
        // When no chr is specified, default is integral
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
}
