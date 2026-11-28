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

/// A script group of `tex`: its content, and whether it hangs on an empty base `{}` or on
/// nothing, so that no glyph of the formula carries it.
struct Script<'a> {
    content: &'a str,
    base_less: bool,
}

/// Every `_{...}` and `^{...}` group of `tex`, nested groups included.
fn script_groups(tex: &str) -> Vec<Script<'_>> {
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
                        groups.push(Script {
                            content: &rest[1..offset],
                            base_less: at == 0 || tex[..at].ends_with("{}"),
                        });
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    groups
}

/// A script that is no exponent or index. Punctuation alone never is one. An operator
/// (`\\times`, `\\pm`) or a list label such as `9.` or `(16)` is a stray only on no base: a
/// printed problem number or sign hung on the formula by position, where `K^{\\times}`,
/// `W^{\\pm}` and `f^{(3)}` are scripts of the glyph before them.
fn is_stray_script(script: &Script<'_>) -> bool {
    const OPERATOR_COMMANDS: &[&str] = &["\\times", "\\div", "\\pm", "\\mp", "\\cdot", "="];
    let content = script.content.trim();
    if content.chars().all(|c| matches!(c, '.' | ',' | ';' | ':')) {
        return true;
    }
    let number = content.trim_start_matches('(').trim_end_matches([')', '.']);
    script.base_less
        && (OPERATOR_COMMANDS.contains(&content)
            || (!number.is_empty()
                && number.bytes().all(|b| b.is_ascii_digit())
                && (content.ends_with('.') || content.starts_with('(') || content.ends_with(')'))))
}
