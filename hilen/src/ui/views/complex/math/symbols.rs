//! The commands of LaTeX that stand for 1 char, and the letters of each
//! math alphabet in Unicode.

use super::parse::Class;

/// The char and the class of a command like `\alpha` or `\le`.
pub(crate) fn symbol(name: &str) -> Option<(char, Class)> {
    greek(name)
        .map(|ch| (ch, Class::Ord))
        .or_else(|| ordinary(name).map(|ch| (ch, Class::Ord)))
        .or_else(|| binary(name).map(|ch| (ch, Class::Bin)))
        .or_else(|| relation(name).map(|ch| (ch, Class::Rel)))
        .or_else(|| bracket(name))
}

/// Greek, lowercase italic like TeX, uppercase upright.
fn greek(name: &str) -> Option<char> {
    let found = match name {
        "alpha" => '\u{1D6FC}',
        "beta" => '\u{1D6FD}',
        "gamma" => '\u{1D6FE}',
        "delta" => '\u{1D6FF}',
        "epsilon" => '\u{1D716}',
        "varepsilon" => '\u{1D700}',
        "zeta" => '\u{1D701}',
        "eta" => '\u{1D702}',
        "theta" => '\u{1D703}',
        "vartheta" => '\u{1D717}',
        "iota" => '\u{1D704}',
        "kappa" => '\u{1D705}',
        "lambda" => '\u{1D706}',
        "mu" => '\u{1D707}',
        "nu" => '\u{1D708}',
        "xi" => '\u{1D709}',
        "omicron" => '\u{1D70A}',
        "pi" => '\u{1D70B}',
        "varpi" => '\u{1D71B}',
        "rho" => '\u{1D70C}',
        "varrho" => '\u{1D71A}',
        "sigma" => '\u{1D70E}',
        "varsigma" => '\u{1D70D}',
        "tau" => '\u{1D70F}',
        "upsilon" => '\u{1D710}',
        "phi" => '\u{1D719}',
        "varphi" => '\u{1D711}',
        "chi" => '\u{1D712}',
        "psi" => '\u{1D713}',
        "omega" => '\u{1D714}',
        "Gamma" => 'Γ',
        "Delta" => 'Δ',
        "Theta" => 'Θ',
        "Lambda" => 'Λ',
        "Xi" => 'Ξ',
        "Pi" => 'Π',
        "Sigma" => 'Σ',
        "Upsilon" => 'Υ',
        "Phi" => 'Φ',
        "Psi" => 'Ψ',
        "Omega" => 'Ω',
        _ => return None,
    };
    Some(found)
}

fn ordinary(name: &str) -> Option<char> {
    let found = match name {
        "infty" => '∞',
        "partial" => '\u{1D715}',
        "nabla" => '∇',
        "forall" => '∀',
        "exists" => '∃',
        "nexists" => '∄',
        "emptyset" | "varnothing" => '∅',
        "hbar" | "hslash" => 'ℏ',
        "ell" => 'ℓ',
        "Re" => 'ℜ',
        "Im" => 'ℑ',
        "aleph" => 'ℵ',
        "wp" => '℘',
        "prime" => '′',
        "angle" => '∠',
        "triangle" => '△',
        "neg" | "lnot" => '¬',
        "top" => '⊤',
        "bot" => '⊥',
        "degree" => '°',
        "ldots" | "dots" | "dotsc" | "dotso" => '…',
        "cdots" | "dotsb" | "dotsm" | "dotsi" => '⋯',
        "vdots" => '⋮',
        "ddots" => '⋱',
        "imath" => '\u{1D6A4}',
        "jmath" => '\u{1D6A5}',
        "#" => '#',
        "%" => '%',
        "&" => '&',
        "_" => '_',
        "$" => '$',
        "|" | "Vert" => '‖',
        "vert" => '|',
        "backslash" => '\\',
        "clubsuit" => '♣',
        "diamondsuit" => '♢',
        "heartsuit" => '♡',
        "spadesuit" => '♠',
        _ => return None,
    };
    Some(found)
}

fn binary(name: &str) -> Option<char> {
    let found = match name {
        "pm" => '±',
        "mp" => '∓',
        "times" => '×',
        "div" => '÷',
        "cdot" => '⋅',
        "ast" => '∗',
        "star" => '⋆',
        "circ" => '∘',
        "bullet" => '∙',
        "cup" => '∪',
        "cap" => '∩',
        "wedge" | "land" => '∧',
        "vee" | "lor" => '∨',
        "setminus" => '∖',
        "oplus" => '⊕',
        "ominus" => '⊖',
        "otimes" => '⊗',
        "oslash" => '⊘',
        "odot" => '⊙',
        "dagger" => '†',
        "ddagger" => '‡',
        "amalg" => '⨿',
        "wr" => '≀',
        _ => return None,
    };
    Some(found)
}

fn relation(name: &str) -> Option<char> {
    let found = match name {
        "le" | "leq" => '≤',
        "ge" | "geq" => '≥',
        "leqslant" => '⩽',
        "geqslant" => '⩾',
        "ne" | "neq" => '≠',
        "approx" => '≈',
        "equiv" => '≡',
        "sim" => '∼',
        "simeq" => '≃',
        "cong" => '≅',
        "propto" => '∝',
        "ll" => '≪',
        "gg" => '≫',
        "in" => '∈',
        "notin" => '∉',
        "ni" | "owns" => '∋',
        "subset" => '⊂',
        "supset" => '⊃',
        "subseteq" => '⊆',
        "supseteq" => '⊇',
        "subsetneq" => '⊊',
        "supsetneq" => '⊋',
        "perp" => '⊥',
        "parallel" => '∥',
        "mid" => '∣',
        "nmid" => '∤',
        "models" => '⊨',
        "vdash" => '⊢',
        "dashv" => '⊣',
        "asymp" => '≍',
        "doteq" => '≐',
        "prec" => '≺',
        "succ" => '≻',
        "preceq" => '⪯',
        "succeq" => '⪰',
        "to" | "rightarrow" => '→',
        "gets" | "leftarrow" => '←',
        "leftrightarrow" => '↔',
        "Rightarrow" | "implies" => '⇒',
        "Leftarrow" | "impliedby" => '⇐',
        "Leftrightarrow" | "iff" => '⇔',
        "longrightarrow" => '⟶',
        "longleftarrow" => '⟵',
        "Longrightarrow" => '⟹',
        "Longleftarrow" => '⟸',
        "longleftrightarrow" => '⟷',
        "Longleftrightarrow" => '⟺',
        "mapsto" => '↦',
        "longmapsto" => '⟼',
        "uparrow" => '↑',
        "downarrow" => '↓',
        "updownarrow" => '↕',
        "Uparrow" => '⇑',
        "Downarrow" => '⇓',
        "nearrow" => '↗',
        "searrow" => '↘',
        "hookrightarrow" => '↪',
        "hookleftarrow" => '↩',
        "rightleftharpoons" => '⇌',
        "coloneqq" | "coloneq" => '≔',
        "triangleq" => '≜',
        "therefore" => '∴',
        "because" => '∵',
        _ => return None,
    };
    Some(found)
}

fn bracket(name: &str) -> Option<(char, Class)> {
    use Class::{Close, Open, Punct};
    let found = match name {
        "langle" => ('⟨', Open),
        "rangle" => ('⟩', Close),
        "lfloor" => ('⌊', Open),
        "rfloor" => ('⌋', Close),
        "lceil" => ('⌈', Open),
        "rceil" => ('⌉', Close),
        "lvert" => ('|', Open),
        "rvert" => ('|', Close),
        "lVert" => ('‖', Open),
        "rVert" => ('‖', Close),
        "{" | "lbrace" => ('{', Open),
        "}" | "rbrace" => ('}', Close),
        "lbrack" => ('[', Open),
        "rbrack" => (']', Close),
        "colon" => (':', Punct),
        _ => return None,
    };
    Some(found)
}

/// The large operators, `\sum` and its kin. The flag says whether its
/// limits go above and below in a display formula, an integral keeps
/// them at its side.
pub(crate) fn large_operator(name: &str) -> Option<(char, bool)> {
    let found = match name {
        "sum" => ('∑', true),
        "prod" => ('∏', true),
        "coprod" => ('∐', true),
        "bigcup" => ('⋃', true),
        "bigcap" => ('⋂', true),
        "bigvee" => ('⋁', true),
        "bigwedge" => ('⋀', true),
        "bigoplus" => ('⨁', true),
        "bigotimes" => ('⨂', true),
        "bigodot" => ('⨀', true),
        "bigsqcup" => ('⨆', true),
        "int" => ('∫', false),
        "iint" => ('∬', false),
        "iiint" => ('∭', false),
        "oint" => ('∮', false),
        "oiint" => ('∯', false),
        _ => return None,
    };
    Some(found)
}

/// The names that are set upright like `\sin`. The flag says whether its
/// limits go below in a display formula, like the one of `\lim`.
pub(crate) fn function(name: &str) -> Option<bool> {
    let limits = match name {
        "arccos" | "arcsin" | "arctan" | "arg" | "cos" | "cosh" | "cot" | "coth" | "csc" | "deg" | "dim"
        | "exp" | "hom" | "ker" | "lg" | "ln" | "log" | "sec" | "sin" | "sinh" | "tan" | "tanh" => false,
        "det" | "gcd" | "inf" | "lim" | "liminf" | "limsup" | "max" | "min" | "Pr" | "sup" => true,
        _ => return None,
    };
    Some(limits)
}

/// How the name of a function is written, `\liminf` is 2 words.
pub(crate) fn function_text(name: &str) -> &str {
    match name {
        "liminf" => "lim inf",
        "limsup" => "lim sup",
        other => other,
    }
}

/// The accent a command puts over its argument and whether it stretches
/// to the width of it.
pub(crate) fn accent(name: &str) -> Option<(char, bool)> {
    let found = match name {
        "hat" => ('\u{0302}', false),
        "widehat" => ('\u{0302}', true),
        "check" => ('\u{030C}', false),
        "widecheck" => ('\u{030C}', true),
        "tilde" => ('\u{0303}', false),
        "widetilde" => ('\u{0303}', true),
        "bar" => ('\u{0304}', false),
        "vec" => ('\u{20D7}', false),
        "overrightarrow" => ('\u{20D7}', true),
        "overleftarrow" => ('\u{20D6}', true),
        "dot" => ('\u{0307}', false),
        "ddot" => ('\u{0308}', false),
        "dddot" => ('\u{20DB}', false),
        "acute" => ('\u{0301}', false),
        "grave" => ('\u{0300}', false),
        "breve" => ('\u{0306}', false),
        "mathring" => ('\u{030A}', false),
        _ => return None,
    };
    Some(found)
}

/// The space a command puts in, in ems.
pub(crate) fn space(name: &str) -> Option<f32> {
    let found = match name {
        "," | "thinspace" => 3.0 / 18.0,
        ":" | ">" | "medspace" => 4.0 / 18.0,
        ";" | "thickspace" => 5.0 / 18.0,
        "!" | "negthinspace" => -3.0 / 18.0,
        " " => 0.25,
        "enspace" => 0.5,
        "quad" => 1.0,
        "qquad" => 2.0,
        _ => return None,
    };
    Some(found)
}

/// The negated form of a relation, for `\not`.
pub(crate) fn negated(ch: char) -> Option<char> {
    let found = match ch {
        '=' => '≠',
        '<' => '≮',
        '>' => '≯',
        '≤' => '≰',
        '≥' => '≱',
        '∈' => '∉',
        '≡' => '≢',
        '∼' => '≁',
        '≈' => '≉',
        '≃' => '≄',
        '≅' => '≇',
        '⊂' => '⊄',
        '⊃' => '⊅',
        '⊆' => '⊈',
        '⊇' => '⊉',
        '∃' => '∄',
        '∣' => '∤',
        '∥' => '∦',
        '→' => '↛',
        '←' => '↚',
        '⇒' => '⇏',
        '⇔' => '⇎',
        _ => return None,
    };
    Some(found)
}

/// A set of letters, what `\mathbf` and its kin pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Alphabet {
    /// Letters italic, digits upright, the default of math.
    Math,
    Roman,
    Bold,
    BoldItalic,
    Italic,
    DoubleStruck,
    Script,
    Fraktur,
    SansSerif,
    Mono,
}

impl Alphabet {
    pub(crate) fn of_command(name: &str) -> Option<Self> {
        let found = match name {
            "mathrm" | "rm" | "mathup" => Self::Roman,
            "mathbf" | "bf" => Self::Bold,
            "boldsymbol" | "bm" => Self::BoldItalic,
            "mathit" | "it" => Self::Italic,
            "mathbb" | "Bbb" => Self::DoubleStruck,
            "mathcal" | "mathscr" | "cal" => Self::Script,
            "mathfrak" | "frak" => Self::Fraktur,
            "mathsf" | "sf" => Self::SansSerif,
            "mathtt" | "tt" => Self::Mono,
            _ => return None,
        };
        Some(found)
    }

    /// The char a Latin letter or a digit takes in this alphabet. Any
    /// other char stays as it is.
    pub(crate) fn map(self, ch: char) -> char {
        let (upper, lower, digit) = match self {
            Self::Roman => return ch,
            Self::Math | Self::Italic => {
                if ch == 'h' {
                    return 'ℎ';
                }
                (0x1D434, 0x1D44E, None)
            }
            Self::Bold => (0x1D400, 0x1D41A, Some(0x1D7CE)),
            Self::BoldItalic => (0x1D468, 0x1D482, Some(0x1D7CE)),
            Self::DoubleStruck => {
                let gap = match ch {
                    'C' => Some('ℂ'),
                    'H' => Some('ℍ'),
                    'N' => Some('ℕ'),
                    'P' => Some('ℙ'),
                    'Q' => Some('ℚ'),
                    'R' => Some('ℝ'),
                    'Z' => Some('ℤ'),
                    _ => None,
                };
                if let Some(gap) = gap {
                    return gap;
                }
                (0x1D538, 0x1D552, Some(0x1D7D8))
            }
            Self::Script => {
                let gap = match ch {
                    'B' => Some('ℬ'),
                    'E' => Some('ℰ'),
                    'F' => Some('ℱ'),
                    'H' => Some('ℋ'),
                    'I' => Some('ℐ'),
                    'L' => Some('ℒ'),
                    'M' => Some('ℳ'),
                    'R' => Some('ℛ'),
                    'e' => Some('ℯ'),
                    'g' => Some('ℊ'),
                    'o' => Some('ℴ'),
                    _ => None,
                };
                if let Some(gap) = gap {
                    return gap;
                }
                (0x1D49C, 0x1D4B6, None)
            }
            Self::Fraktur => {
                let gap = match ch {
                    'C' => Some('ℭ'),
                    'H' => Some('ℌ'),
                    'I' => Some('ℑ'),
                    'R' => Some('ℜ'),
                    'Z' => Some('ℨ'),
                    _ => None,
                };
                if let Some(gap) = gap {
                    return gap;
                }
                (0x1D504, 0x1D51E, None)
            }
            Self::SansSerif => (0x1D5A0, 0x1D5BA, Some(0x1D7E2)),
            Self::Mono => (0x1D670, 0x1D68A, Some(0x1D7F6)),
        };
        let code = match ch {
            'A'..='Z' => upper + (ch as u32 - 'A' as u32),
            'a'..='z' => lower + (ch as u32 - 'a' as u32),
            '0'..='9' => match digit {
                Some(digit) => digit + (ch as u32 - '0' as u32),
                None => return ch,
            },
            _ => return ch,
        };
        char::from_u32(code).unwrap_or(ch)
    }
}
