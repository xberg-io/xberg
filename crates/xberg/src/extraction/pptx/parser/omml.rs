//! OMML (`m:oMath`/`m:oMathPara`) bridge between the `roxmltree` slide DOM and the shared
//! `quick_xml` streaming math converter.

use roxmltree::Node;

use super::{MATH_NAMESPACE, Run};
use crate::extraction::pptx::elements::Formatting;

/// Prefix the converter matches OMML element and attribute names against.
const MATH_PREFIX: &str = "m";

/// Prefix for elements outside OMML; the converter skips them whatever they are called.
const FOREIGN_PREFIX: &str = "x";

/// Re-serialize `node` with OMML names rewritten to the `m:` prefix the converter matches
/// literally. Slicing the source text instead would keep the producer's prefix and lose
/// namespace declarations made on an ancestor.
fn canonical_omml(node: Node) -> String {
    enum Step<'a, 'input> {
        Open(Node<'a, 'input>),
        Close(Node<'a, 'input>),
    }

    // An explicit stack: element nesting is document-controlled and roxmltree does not bound it.
    let mut out = String::new();
    let mut stack = vec![Step::Open(node)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Open(n) if n.is_text() => {
                if let Some(text) = n.text() {
                    out.push_str(&quick_xml::escape::escape(text));
                }
            }
            Step::Open(n) if n.is_element() => {
                out.push('<');
                push_element_name(&mut out, n);
                for attr in n.attributes() {
                    out.push(' ');
                    if attr.namespace() == Some(MATH_NAMESPACE) {
                        out.push_str(MATH_PREFIX);
                        out.push(':');
                    }
                    out.push_str(attr.name());
                    out.push_str("=\"");
                    out.push_str(&quick_xml::escape::escape(attr.value()));
                    out.push('"');
                }
                out.push('>');
                stack.push(Step::Close(n));
                let children_start = stack.len();
                stack.extend(n.children().map(Step::Open));
                stack[children_start..].reverse();
            }
            Step::Close(n) => {
                out.push_str("</");
                push_element_name(&mut out, n);
                out.push('>');
            }
            Step::Open(_) => {}
        }
    }
    out
}

fn push_element_name(out: &mut String, node: Node) {
    let prefix = if node.tag_name().namespace() == Some(MATH_NAMESPACE) {
        MATH_PREFIX
    } else {
        FOREIGN_PREFIX
    };
    out.push_str(prefix);
    out.push(':');
    out.push_str(node.tag_name().name());
}

/// Find the first OMML `oMathPara`/`oMath` node under `container` and convert it to LaTeX via
/// the shared OMML converter (`extraction::docx::math`), producing a single math `Run`.
/// Returns `None` if no math node is present or conversion fails.
pub(super) fn omml_node_to_run(container: &Node) -> Option<Run> {
    let find = |name: &str| {
        container
            .descendants()
            .find(|d| d.tag_name().namespace() == Some(MATH_NAMESPACE) && d.tag_name().name() == name)
    };
    let (is_display, math_node) = match find("oMathPara") {
        Some(n) => (true, n),
        None => (false, find("oMath")?),
    };

    let canonical = canonical_omml(math_node);
    let expected_tag = if is_display { "m:oMathPara" } else { "m:oMath" };

    let mut reader = quick_xml::Reader::from_str(&canonical);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) if e.name().as_ref() == expected_tag => break,
            Ok(quick_xml::events::Event::Eof) | Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }

    let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
    let latex = if is_display {
        crate::extraction::docx::math::collect_and_convert_omath_para(&mut reader, &mut budget).ok()?
    } else {
        crate::extraction::docx::math::collect_and_convert_omath(&mut reader, &mut budget).ok()?
    };

    // The rendered slide text is trimmed, so a converter result that keeps the
    // source indentation would no longer equal the LaTeX kept for the formula
    // element and the equation would stay in the text only.
    let latex = latex.trim().to_string();
    if latex.is_empty() {
        return None;
    }

    Some(Run {
        text: String::new(),
        formatting: Formatting::default(),
        hyperlink_id: None,
        math_latex: Some((latex, is_display)),
    })
}
