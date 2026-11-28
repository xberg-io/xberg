//! Whether rebuilt TeX is fit to publish in place of the plain reading of its source spans.

/// Whether rebuilt TeX is well formed enough to publish. Wrong math is worse than the plain prose
/// reading, so a formula the layout pass could not structure is left as its source spans: an
/// empty group that scripts nothing (`{}` outside `{}^{14}C`), a script after an operator
/// (`=_{2}`, a leading `^{`) or unbalanced braces mark a base or fraction part that landed in the
/// wrong place.
pub(super) fn tex_is_reliable(tex: &str) -> bool {
    if tex
        .match_indices("{}")
        .any(|(at, group)| !tex[at + group.len()..].starts_with(['^', '_']))
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
