//! Inline math runs inside PDF text lines.
//!
//! The native PDF route has no geometry for a formula that sits inside a sentence,
//! so a prose paragraph keeps its equation as ordinary characters. This module
//! decides which part of a text line reads as math, using the same character
//! vocabulary as the paragraph-level formula classifier (`super::classify`).
//!
//! A run qualifies only when it carries a relational operator or a superscript and
//! every token in it reads as mathematics. An arithmetic sign alone is not enough:
//! `C++`, `a+b` and `well-known` are ordinary prose.

/// Longest token run accepted as one inline formula.
const MAX_RUN_TOKENS: usize = 12;

/// Multi-letter names that are operands or functions rather than prose.
const MATH_NAMES: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "log", "ln", "lim", "exp", "max", "min", "sup", "inf", "det", "arcsin",
    "arccos", "arctan", "sinh", "cosh", "tanh", "pi", "sqrt", "cbrt", "abs", "gcd", "lcm", "mod", "sgn", "deg", "arg",
    "dim", "ker", "erf",
];

/// What a token contributes to a math run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Carries a value: a number, a variable, a function name or a bracketed term.
    Operand,
    /// Carries only operator or grouping characters.
    Operator,
}

/// One math-candidate token, with its content span inside the paragraph.
#[derive(Debug, Clone, Copy)]
struct Token {
    start: usize,
    end: usize,
    role: Role,
    /// The token holds a relational operator (`=`, `<`, `≤`, `→`, ...).
    relation: bool,
    /// The token holds a superscript marker.
    superscript: bool,
}

/// Byte ranges of the math runs inside `text`, in order and without overlap.
///
/// Ranges exclude surrounding whitespace and sentence punctuation, so a trailing
/// full stop stays prose.
pub(crate) fn inline_math_annotations(text: &str) -> Vec<(u32, u32)> {
    let mut spans = Vec::new();
    let mut run: Vec<Token> = Vec::new();
    for token in tokenize(text) {
        match token {
            Some(token) => run.push(token),
            None => flush_run(text, &mut run, &mut spans),
        }
    }
    flush_run(text, &mut run, &mut spans);
    spans
}

/// Emit one run when it reads as an equation, then clear it.
fn flush_run(text: &str, run: &mut Vec<Token>, spans: &mut Vec<(u32, u32)>) {
    let leading = run.iter().take_while(|token| token.role == Role::Operator).count();
    run.drain(..leading);
    while run.last().is_some_and(|token| token.role == Role::Operator) {
        run.pop();
    }
    let qualifies = match run.len() {
        0 => false,
        1 => run[0].superscript || (run[0].relation && run_reads_as_equation(text, run)),
        count => count <= MAX_RUN_TOKENS && run.iter().any(|token| token.relation) && run_reads_as_equation(text, run),
    };
    if qualifies && let (Some(first), Some(last)) = (run.first(), run.last()) {
        spans.push((first.start as u32, last.end as u32));
    }
    run.clear();
}

/// Whether the source text covered by a run places every relation between two operands.
///
/// A relation token can hold only one of its sides (`x= 5`, `x =5`), so the check spans
/// token boundaries instead of judging each token on its own.
fn run_reads_as_equation(text: &str, run: &[Token]) -> bool {
    match (run.first(), run.last()) {
        (Some(first), Some(last)) => text.get(first.start..last.end).is_some_and(relation_run_reads_as_math),
        _ => false,
    }
}

/// Classify every whitespace-separated token of `text`; `None` is prose or a line break
/// and breaks a run.
fn tokenize(text: &str) -> Vec<Option<Token>> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    let mut line_break = false;
    for (index, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(token_start) = start.take() {
                tokens.push(classify_token(&text[token_start..index], token_start, index));
            }
            line_break |= ch == '\n';
        } else if start.is_none() {
            if line_break {
                tokens.push(None);
                line_break = false;
            }
            start = Some(index);
        }
    }
    if let Some(token_start) = start {
        tokens.push(classify_token(&text[token_start..], token_start, text.len()));
    }
    tokens
}

/// Classify one token, ignoring trailing sentence punctuation.
fn classify_token(raw: &str, start: usize, end: usize) -> Option<Token> {
    let body = raw.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    let end = end - (raw.len() - body.len());
    if body.is_empty() {
        return None;
    }

    let mut letters = 0usize;
    let mut numeric = 0usize;
    let mut relation = false;
    let mut superscript = false;
    for ch in body.chars() {
        if ch.is_alphabetic() {
            letters += 1;
        } else if ch.is_numeric() {
            numeric += 1;
        } else if is_math_relation(ch) {
            relation = true;
        } else if ch == '^' {
            superscript = true;
        } else if ch != '_' && !is_math_punctuation(ch) {
            return None;
        }
    }

    if letters == 0 && numeric == 0 {
        return Some(Token {
            start,
            end,
            role: Role::Operator,
            relation,
            superscript,
        });
    }
    // A relation joins its two sides, so a compact equation (`x=y`) is judged by the
    // operands rather than by the token's own letter count. `revenue=growth` remains a
    // pair of English words because neither side is a single-letter, numeric or named
    // operand.
    if relation {
        return relation_token_candidate(body).then_some(Token {
            start,
            end,
            role: Role::Operand,
            relation,
            superscript,
        });
    }
    // A word of two letters or more is prose unless it names a function or carries a
    // digit or superscript (`mc^2`, `H2O`, `sin`). An underscore alone is an identifier
    // separator in source text (`full_join`), not evidence of math.
    if letters > 1 && numeric == 0 && !superscript && !is_math_name(body) {
        return None;
    }
    Some(Token {
        start,
        end,
        role: Role::Operand,
        relation,
        superscript,
    })
}

pub(super) fn is_math_name(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    MATH_NAMES.contains(&lower.as_str())
}

/// Operand sides of `body`, grouping a run of relation characters (`==`, `<=`, `>=`,
/// `!=`) as one relation rather than as several.
fn relation_sides(body: &str) -> Vec<&str> {
    let mut sides = Vec::new();
    let mut side_start = 0usize;
    let mut in_relation = false;
    for (index, ch) in body.char_indices() {
        if is_math_relation(ch) {
            if !in_relation {
                sides.push(&body[side_start..index]);
                in_relation = true;
            }
        } else if in_relation {
            in_relation = false;
            side_start = index;
        }
    }
    if in_relation {
        // The token ends inside the relation, so its trailing side was never written.
        sides.push("");
    } else {
        sides.push(&body[side_start..]);
    }
    sides
}

/// Number of operand pieces on one side of a relation, or `None` when a piece is prose.
fn relation_side_operands(side: &str) -> Option<usize> {
    let mut operands = 0usize;
    for piece in side.split(|ch: char| is_math_punctuation(ch) || ch == '_' || ch.is_whitespace()) {
        if piece.is_empty() {
            continue;
        }
        if !reads_as_operand(piece) {
            return None;
        }
        operands += 1;
    }
    Some(operands)
}

/// Whether a token holding a relation may join an inline-math run.
///
/// One side may be empty because an adjacent token supplies it (`x= 5`, `x =5`); the run
/// as a whole is validated by [`relation_run_reads_as_math`].
fn relation_token_candidate(body: &str) -> bool {
    let sides = relation_sides(body);
    sides.len() > 1
        && sides
            .into_iter()
            .try_fold(0usize, |operands, side| {
                relation_side_operands(side).map(|count| operands + count)
            })
            .is_some_and(|operands| operands > 0)
}

/// Whether every relation in a candidate run has an operand on both sides.
fn relation_run_reads_as_math(run: &str) -> bool {
    let sides = relation_sides(run);
    sides.len() > 1
        && sides
            .iter()
            .all(|side| relation_side_operands(side).is_some_and(|operands| operands > 0))
}

/// Whether one side of a relation reads as an operand: a variable, a number, a product
/// carrying a digit or superscript, or a named function.
fn reads_as_operand(piece: &str) -> bool {
    let mut letters = 0usize;
    let mut digits = 0usize;
    for ch in piece.chars() {
        if ch.is_alphabetic() {
            letters += 1;
        } else if ch.is_numeric() {
            digits += 1;
        }
    }
    if letters == 0 {
        return digits > 0;
    }
    letters <= 1 || digits > 0 || piece.contains('^') || is_math_name(piece)
}

/// Relational and set operators that mark a run as an equation.
fn is_math_relation(ch: char) -> bool {
    matches!(
        ch,
        '=' | '!'
            | '<'
            | '>'
            | '\u{2264}'
            | '\u{2265}'
            | '\u{2260}'
            | '\u{2248}'
            | '\u{2261}'
            | '\u{2192}'
            | '\u{2190}'
            | '\u{2194}'
            | '\u{21D2}'
            | '\u{21D0}'
            | '\u{2208}'
            | '\u{2209}'
            | '\u{221D}'
            | '\u{223C}'
    )
}

/// Operator and grouping characters a math token may hold. U+2010 and U+2011 read as `-`:
/// assembled text keeps them until `pipeline::finalize_element_hyphens` maps them.
fn is_math_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '+'
            | '-'
            | '\u{2010}'
            | '\u{2011}'
            | '*'
            | '/'
            | '|'
            | '.'
            | ','
            | '\u{2212}'
            | '\u{00B1}'
            | '\u{00D7}'
            | '\u{00F7}'
            | '\u{22C5}'
            | '\u{2211}'
            | '\u{220F}'
            | '\u{222B}'
            | '\u{221A}'
            | '\u{221E}'
    )
}

#[cfg(test)]
mod tests {
    use super::inline_math_annotations;

    fn spans(text: &str) -> Vec<&str> {
        inline_math_annotations(text)
            .into_iter()
            .map(|(start, end)| &text[start as usize..end as usize])
            .collect()
    }

    #[test]
    fn should_wrap_an_equation_inside_a_sentence() {
        assert_eq!(
            spans("The area of a circle is A = pi r^2 for a radius r."),
            ["A = pi r^2"]
        );
    }

    #[test]
    fn unicode_hyphens_read_as_a_minus_like_the_ascii_hyphen() {
        for hyphen in ['-', '\u{2010}', '\u{2011}'] {
            let equation = format!("x{hyphen}1 = y");
            assert_eq!(spans(&format!("so {equation} holds")), [equation.as_str()]);
        }
    }

    #[test]
    fn should_keep_sentence_punctuation_outside_the_run() {
        assert_eq!(spans("It holds that x = 5. Next sentence."), ["x = 5"]);
    }

    #[test]
    fn should_find_two_runs_in_one_line() {
        assert_eq!(spans("Let x = 5 and y = 6 hold."), ["x = 5", "y = 6"]);
    }

    #[test]
    fn should_keep_arithmetic_signs_without_a_relation_as_prose() {
        assert_eq!(spans("We use C++ and a+b in prose."), Vec::<&str>::new());
        assert_eq!(spans("A well-known e-mail address."), Vec::<&str>::new());
    }

    #[test]
    fn should_keep_a_word_between_operands_as_prose() {
        assert_eq!(spans("The revenue = growth is wrong."), Vec::<&str>::new());
    }

    #[test]
    fn should_wrap_a_compact_equation_with_single_letter_operands() {
        assert_eq!(spans("The identity x=y holds here."), ["x=y"]);
        assert_eq!(spans("Both a+b=c and x=y apply."), ["a+b=c", "x=y"]);
    }

    #[test]
    fn should_keep_a_compact_equation_between_words_as_prose() {
        assert_eq!(spans("The revenue=growth claim is wrong."), Vec::<&str>::new());
        assert_eq!(spans("Call full_join=group_by here."), Vec::<&str>::new());
    }

    #[test]
    fn should_wrap_an_equation_split_at_the_relation() {
        assert_eq!(spans("The identity x= 5 holds here."), ["x= 5"]);
        assert_eq!(spans("The identity x =5 holds here."), ["x =5"]);
    }

    #[test]
    fn should_treat_a_compound_relation_as_one_operator() {
        assert_eq!(spans("Both x==y and a<=b hold."), ["x==y", "a<=b"]);
        assert_eq!(spans("Both x!=y and x\u{2264}y hold."), ["x!=y", "x\u{2264}y"]);
    }

    #[test]
    fn should_reject_a_comparison_with_a_missing_operand() {
        assert_eq!(spans("The guards x== and ==y fail."), Vec::<&str>::new());
        assert_eq!(spans("The guard ==5 fails."), Vec::<&str>::new());
    }

    #[test]
    fn should_keep_snake_case_identifiers_as_prose() {
        assert_eq!(spans("Call full_join and group_by(mpg_round)."), Vec::<&str>::new());
    }

    #[test]
    fn should_wrap_a_bare_superscript_term() {
        assert_eq!(spans("The term r^2 appears often."), ["r^2"]);
    }

    #[test]
    fn should_not_wrap_plain_numbers() {
        assert_eq!(spans("Published in 2026 with 3.5 thousand."), Vec::<&str>::new());
    }

    #[test]
    fn should_not_join_a_run_across_a_line_break() {
        assert_eq!(spans("so x = 1\ny = 2 applies"), ["x = 1", "y = 2"]);
    }

    #[test]
    fn should_wrap_greek_identifiers() {
        assert_eq!(spans("Set \u{03B1} = 0.5 here."), ["\u{03B1} = 0.5"]);
    }
}
