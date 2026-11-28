//! Whether rebuilt TeX is fit to publish in place of the plain reading of its source spans.

/// Whether rebuilt TeX is well formed enough to publish. Wrong math is worse than the plain prose
/// reading, so a formula the layout pass could not structure is left as its source spans: an
/// empty group that scripts nothing (`{}` outside `{}^{14}C`), a script after an operator
/// (`=_{2}`, a leading `^{`), base-less scripts in a chain (`{}_{1}{}_{.}`), a script that is only
/// punctuation, an operator or a list label (`_{9.}`, `^{(16)}`) or unbalanced braces mark a base
/// or fraction part that landed in the wrong place.
pub(super) fn tex_is_reliable(tex: &str) -> bool {
    if tex
        .match_indices("{}")
        .any(|(at, group)| !tex[at + group.len()..].starts_with(['^', '_']))
        || tex.contains("}{}_")
        || tex.contains("}{}^")
        || script_groups(tex).iter().any(|script| is_stray_script(script))
    {
        return false;
    }
    let mut depth = 0i32;
    let mut previous: Option<char> = None;
    for ch in tex.chars() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            '^' | '_' if previous.is_none_or(|before| matches!(before, '=' | '+' | '-' | ',' | ';' | '(' | '[')) => {
                return false;
            }
            _ => {}
        }
        previous = Some(ch);
    }
    depth == 0
}

/// The content of every `_{...}` and `^{...}` group of `tex`, nested groups included.
fn script_groups(tex: &str) -> Vec<&str> {
    let mut groups = Vec::new();
    for (at, _) in tex.match_indices(['_', '^']) {
        let rest = &tex[at + 1..];
        if !rest.starts_with('{') {
            continue;
        }
        let mut depth = 0_usize;
        for (offset, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        groups.push(&rest[1..offset]);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    groups
}

/// A script that is no exponent or index: an operator, punctuation or a list label such as
/// `9.` or `(16)` printed beside a formula and hung on it by position.
fn is_stray_script(script: &str) -> bool {
    const OPERATOR_COMMANDS: &[&str] = &["\\times", "\\div", "\\pm", "\\mp", "\\cdot", "="];
    let script = script.trim();
    let number = script.trim_start_matches('(').trim_end_matches([')', '.']);
    script.chars().all(|c| matches!(c, '.' | ',' | ';' | ':'))
        || OPERATOR_COMMANDS.contains(&script)
        || (!number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
            && (script.ends_with('.') || script.starts_with('(') || script.ends_with(')')))
}
