//! Font families that mark a math run, and the LaTeX spelling of each math glyph.

/// What a math font's letter slots draw, which decides how an ASCII letter is spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MathFont {
    /// Italic letters, digits and operators (CMMI, the OpenType `*Math` fonts, ...).
    Italic,
    /// Symbols whose uppercase slots are calligraphic capitals (CMSY).
    Symbol,
    /// Blackboard-bold capitals (AMS MSBM).
    Blackboard,
    /// Fraktur letters (AMS EUFM).
    Fraktur,
    /// Script capitals (RSFS).
    Script,
}

/// TeX math font families, matched as a prefix of the subset-stripped, upper-cased base name.
const ITALIC_FAMILIES: &[&str] = &[
    "CMMI", "CMEX", "MSAM", "EUEX", "EURM", "EURB", "ESINT", "STMARY", "TXMI", "TXEX", "PXMI", "PXEX", "NTXMI",
    "NTXEX", "NEWTXMI", "NEWPXMI", "ZXMI", "ZPXMI", "MTMI", "MTEX", "LMMATH", "LMMI", "LMEX", "RTXMI",
];
const SYMBOL_FAMILIES: &[&str] = &["CMSY", "CMBSY", "TXSY", "PXSY", "NTXSY", "LMSY", "MTSY"];
const BLACKBOARD_FAMILIES: &[&str] = &["MSBM"];
const FRAKTUR_FAMILIES: &[&str] = &["EUFM", "EUFB"];
const SCRIPT_FAMILIES: &[&str] = &["RSFS", "EUSM", "EUSB"];

/// Length of a PDF subset tag (`ABCDEF+`), ISO 32000-1 §9.6.4.
const SUBSET_TAG_LEN: usize = 6;

/// Classify a PDF font name as a math font, or `None` for a text font.
///
/// Text fonts of the same families (CMR, CMTI, CMBX, ...) are not math fonts: their glyphs
/// are prose unless geometry puts them inside a math run.
pub(crate) fn math_font(font_name: &str) -> Option<MathFont> {
    let name = match font_name.split_once('+') {
        Some((tag, rest)) if tag.len() == SUBSET_TAG_LEN && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => font_name,
    };
    let family: String = name
        .split(['-', ','])
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let has = |families: &[&str]| families.iter().any(|prefix| family.starts_with(prefix));
    if has(BLACKBOARD_FAMILIES) {
        Some(MathFont::Blackboard)
    } else if has(FRAKTUR_FAMILIES) {
        Some(MathFont::Fraktur)
    } else if has(SCRIPT_FAMILIES) {
        Some(MathFont::Script)
    } else if has(SYMBOL_FAMILIES) {
        Some(MathFont::Symbol)
    } else if has(ITALIC_FAMILIES) || family.ends_with("MATH") {
        Some(MathFont::Italic)
    } else {
        None
    }
}

/// The LaTeX command name of a Greek letter.
pub(crate) fn greek_command(c: char) -> Option<&'static str> {
    Some(match c {
        'α' => "alpha",
        'β' => "beta",
        'γ' => "gamma",
        'δ' => "delta",
        'ε' => "varepsilon",
        'ϵ' => "epsilon",
        'ζ' => "zeta",
        'η' => "eta",
        'θ' => "theta",
        'ϑ' => "vartheta",
        'ι' => "iota",
        'κ' => "kappa",
        'λ' => "lambda",
        'μ' | 'µ' => "mu",
        'ν' => "nu",
        'ξ' => "xi",
        'π' => "pi",
        'ϖ' => "varpi",
        'ρ' => "rho",
        'ϱ' => "varrho",
        'σ' => "sigma",
        'ς' => "varsigma",
        'τ' => "tau",
        'υ' => "upsilon",
        'φ' => "varphi",
        'ϕ' => "phi",
        'χ' => "chi",
        'ψ' => "psi",
        'ω' => "omega",
        'Γ' => "Gamma",
        'Δ' => "Delta",
        'Θ' => "Theta",
        'Λ' => "Lambda",
        'Ξ' => "Xi",
        'Π' => "Pi",
        'Σ' => "Sigma",
        'Υ' => "Upsilon",
        'Φ' => "Phi",
        'Ψ' => "Psi",
        'Ω' => "Omega",
        _ => return None,
    })
}

/// Relations, operators and other math symbols with their LaTeX command names.
const SYMBOL_COMMANDS: &[(char, &str)] = &[
    ('≤', "leq"),
    ('≥', "geq"),
    ('≠', "neq"),
    ('≡', "equiv"),
    ('≈', "approx"),
    ('∼', "sim"),
    ('≃', "simeq"),
    ('≅', "cong"),
    ('∝', "propto"),
    ('≪', "ll"),
    ('≫', "gg"),
    ('≺', "prec"),
    ('≻', "succ"),
    ('⪯', "preceq"),
    ('⪰', "succeq"),
    ('⊂', "subset"),
    ('⊃', "supset"),
    ('⊆', "subseteq"),
    ('⊇', "supseteq"),
    ('⊊', "subsetneq"),
    ('⊋', "supsetneq"),
    ('∈', "in"),
    ('∉', "notin"),
    ('∋', "ni"),
    ('⊥', "perp"),
    ('∥', "parallel"),
    ('∣', "mid"),
    ('→', "rightarrow"),
    ('←', "leftarrow"),
    ('↔', "leftrightarrow"),
    ('⇒', "Rightarrow"),
    ('⇐', "Leftarrow"),
    ('⇔', "Leftrightarrow"),
    ('↦', "mapsto"),
    ('↑', "uparrow"),
    ('↓', "downarrow"),
    ('↪', "hookrightarrow"),
    ('⟶', "longrightarrow"),
    ('⟹', "Longrightarrow"),
    ('⟺', "Longleftrightarrow"),
    ('⊢', "vdash"),
    ('⊨', "models"),
    ('≲', "lesssim"),
    ('≳', "gtrsim"),
    ('⩽', "leqslant"),
    ('⩾', "geqslant"),
    ('≜', "triangleq"),
    ('±', "pm"),
    ('∓', "mp"),
    ('×', "times"),
    ('÷', "div"),
    ('·', "cdot"),
    ('⋅', "cdot"),
    ('∘', "circ"),
    ('◦', "circ"),
    ('⋆', "star"),
    ('⊕', "oplus"),
    ('⊗', "otimes"),
    ('⊖', "ominus"),
    ('⊙', "odot"),
    ('∪', "cup"),
    ('∩', "cap"),
    ('∧', "wedge"),
    ('∨', "vee"),
    ('∖', "setminus"),
    ('⊎', "uplus"),
    ('⊓', "sqcap"),
    ('⊔', "sqcup"),
    ('†', "dagger"),
    ('‡', "ddagger"),
    ('∑', "sum"),
    ('∏', "prod"),
    ('∐', "coprod"),
    ('∫', "int"),
    ('⋃', "bigcup"),
    ('⋂', "bigcap"),
    ('∞', "infty"),
    ('∂', "partial"),
    ('∇', "nabla"),
    ('∀', "forall"),
    ('∃', "exists"),
    ('∄', "nexists"),
    ('∅', "emptyset"),
    ('¬', "neg"),
    ('ℓ', "ell"),
    ('ℏ', "hbar"),
    ('ℜ', "Re"),
    ('ℑ', "Im"),
    ('ℵ', "aleph"),
    ('℘', "wp"),
    ('⟨', "langle"),
    ('⟩', "rangle"),
    ('⌊', "lfloor"),
    ('⌋', "rfloor"),
    ('⌈', "lceil"),
    ('⌉', "rceil"),
    ('…', "ldots"),
    ('⋯', "cdots"),
    ('⋮', "vdots"),
    ('⋱', "ddots"),
    ('√', "sqrt"),
    ('♯', "sharp"),
    ('♭', "flat"),
    ('△', "triangle"),
    ('□', "square"),
    ('•', "bullet"),
    ('⋄', "diamond"),
    ('∠', "angle"),
];

/// The LaTeX command name of a relation, operator or other math symbol.
pub(crate) fn symbol_command(c: char) -> Option<&'static str> {
    SYMBOL_COMMANDS
        .iter()
        .find(|(symbol, _)| *symbol == c)
        .map(|(_, name)| *name)
}

/// A character spelled the same in LaTeX math after an ASCII substitution or escape.
fn plain_spelling(c: char) -> Option<&'static str> {
    Some(match c {
        '−' | '–' => "-",
        '′' => "'",
        '″' => "''",
        '∗' => "*",
        '∶' => ":",
        '⁄' => "/",
        '{' => "\\{",
        '}' => "\\}",
        '%' => "\\%",
        '#' => "\\#",
        '&' => "\\&",
        '$' => "\\$",
        '_' => "\\_",
        '^' => "\\hat{}",
        '~' => "\\sim",
        '\\' => "\\backslash",
        _ => return None,
    })
}

/// The letter a double-struck Letterlike Symbols character draws.
fn double_struck(c: char) -> Option<char> {
    Some(match c {
        'ℝ' => 'R',
        'ℕ' => 'N',
        'ℤ' => 'Z',
        'ℚ' => 'Q',
        'ℂ' => 'C',
        'ℙ' => 'P',
        'ℍ' => 'H',
        _ => return None,
    })
}

/// The accent command a spacing accent glyph draws over its base.
pub(crate) fn accent_command(c: char) -> Option<&'static str> {
    Some(match c {
        'ˆ' | '^' => "hat",
        '˜' | '~' => "tilde",
        '¯' | '‾' => "bar",
        '˙' => "dot",
        '¨' => "ddot",
        '⃗' => "vec",
        'ˇ' => "check",
        '˘' => "breve",
        '´' => "acute",
        '`' => "grave",
        _ => return None,
    })
}

/// Whether a glyph carries no recoverable character: an unmapped glyph (U+FFFD) or a
/// Private Use Area code a font without a ToUnicode map emits for its extension pieces.
pub(crate) fn is_unmapped(c: char) -> bool {
    c == '\u{FFFD}' || ('\u{E000}'..='\u{F8FF}').contains(&c)
}

/// Footnote and list marks a math font also supplies; on their own they are not math.
pub(crate) fn is_mark(c: char) -> bool {
    matches!(c, '†' | '‡' | '§' | '¶' | '✓' | '✗' | '★' | '∗' | '*' | '•' | '◦' | '⋆')
}

/// Relations, the glyphs a display equation is built around.
pub(crate) fn is_relation(c: char) -> bool {
    matches!(
        c,
        '=' | '<'
            | '>'
            | '≤'
            | '≥'
            | '≠'
            | '≡'
            | '≈'
            | '∼'
            | '≃'
            | '≅'
            | '∝'
            | '≪'
            | '≫'
            | '⊂'
            | '⊃'
            | '⊆'
            | '⊇'
            | '∈'
            | '∉'
            | '→'
            | '←'
            | '↔'
            | '⇒'
            | '⇐'
            | '⇔'
            | '↦'
            | '≺'
            | '≻'
            | '⪯'
            | '⪰'
            | '⊢'
            | '⊨'
            | '≲'
            | '≳'
            | '⩽'
            | '⩾'
            | '≜'
    )
}

/// Binary operators.
pub(crate) fn is_operator(c: char) -> bool {
    matches!(
        c,
        '+' | '−'
            | '-'
            | '±'
            | '∓'
            | '×'
            | '÷'
            | '·'
            | '⋅'
            | '∘'
            | '⊕'
            | '⊗'
            | '∪'
            | '∩'
            | '∧'
            | '∨'
            | '/'
    )
}

/// Upright operator names TeX sets in roman type (`\sin`, `\max`, ...).
pub(crate) const OPERATOR_NAMES: &[&str] = &[
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim", "exp", "gcd", "hom",
    "inf", "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min", "Pr", "sec", "sin", "sinh", "sup", "tan",
    "tanh",
];

/// Upright words a math run may hold besides [`OPERATOR_NAMES`], spelled `\mathrm{...}`.
pub(crate) const ROMAN_MATH_WORDS: &[&str] = &[
    "mod", "tr", "Tr", "diag", "rank", "sgn", "sign", "var", "Var", "Cov", "argmin", "argmax", "erf",
];

/// One glyph's LaTeX spelling and whether it ends in a command word, after which an ASCII
/// letter needs a separating space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spelling {
    pub(crate) tex: String,
    pub(crate) command: bool,
}

impl Spelling {
    fn command(name: &str) -> Self {
        Self {
            tex: format!("\\{name}"),
            command: true,
        }
    }

    fn literal(tex: impl Into<String>) -> Self {
        Self {
            tex: tex.into(),
            command: false,
        }
    }
}

/// Spell one glyph of a span set in `font` (`None` for a text font inside a math run).
///
/// Unmapped glyphs and whitespace spell as nothing; an unknown character is kept as is.
pub(crate) fn spell_glyph(c: char, font: Option<MathFont>) -> Spelling {
    if is_unmapped(c) || c.is_whitespace() {
        return Spelling::literal("");
    }
    if c.is_ascii_alphabetic() {
        let alphabet = match font {
            Some(MathFont::Blackboard) => Some("mathbb"),
            Some(MathFont::Fraktur) => Some("mathfrak"),
            Some(MathFont::Script) if c.is_ascii_uppercase() => Some("mathscr"),
            Some(MathFont::Symbol) if c.is_ascii_uppercase() => Some("mathcal"),
            _ => None,
        };
        return Spelling::literal(match alphabet {
            Some(alphabet) => format!("\\{alphabet}{{{c}}}"),
            None => c.to_string(),
        });
    }
    if let Some(letter) = double_struck(c) {
        return Spelling::literal(format!("\\mathbb{{{letter}}}"));
    }
    if let Some(name) = greek_command(c).or_else(|| symbol_command(c)) {
        return Spelling::command(name);
    }
    if let Some(spelling) = plain_spelling(c) {
        return Spelling::literal(spelling);
    }
    Spelling::literal(c.to_string())
}
