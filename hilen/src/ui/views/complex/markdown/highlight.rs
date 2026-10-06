use std::{ops::Range, str::FromStr, sync::OnceLock};

use log::debug;
use syntect::{
    parsing::{ParseState, Scope, ScopeStack, SyntaxSet},
    util::LinesWithEndings,
};

/// What a piece of code is, the view gives each kind a color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    Comment,
    String,
    Number,
    Constant,
    Keyword,
    Function,
    Type,
    /// A line a diff adds.
    Inserted,
    /// A line a diff removes.
    Deleted,
}

/// Scope prefixes of the syntax definitions, the first one that fits
/// wins. A scope stack is tried from its inner end.
const RULES: [(&str, Token); 17] = [
    ("comment", Token::Comment),
    ("string", Token::String),
    ("constant.numeric", Token::Number),
    ("constant", Token::Constant),
    ("markup.inserted", Token::Inserted),
    ("markup.deleted", Token::Deleted),
    ("entity.name.function", Token::Function),
    ("support.function", Token::Function),
    ("variable.function", Token::Function),
    ("support.macro", Token::Function),
    ("entity.name", Token::Type),
    ("support.type", Token::Type),
    ("support.class", Token::Type),
    ("keyword", Token::Keyword),
    ("storage", Token::Keyword),
    ("variable.language", Token::Keyword),
    ("entity.other.attribute-name", Token::Constant),
];

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

fn rules() -> &'static [(Scope, Token)] {
    static RULES_PARSED: OnceLock<Vec<(Scope, Token)>> = OnceLock::new();
    RULES_PARSED.get_or_init(|| {
        RULES
            .iter()
            .filter_map(|(prefix, token)| Some((Scope::from_str(prefix).ok()?, *token)))
            .collect()
    })
}

/// The byte ranges of `code` that get a color, in order. A language
/// nobody knows gives none, and a line the definition fails on ends the
/// coloring there, the code is still shown.
pub(crate) fn highlight(language: &str, code: &str) -> Vec<(Range<usize>, Token)> {
    let syntaxes = syntaxes();
    let Some(syntax) = syntaxes.find_syntax_by_token(language) else {
        return Vec::new();
    };
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut tokens: Vec<(Range<usize>, Token)> = Vec::new();
    let mut line_start = 0;
    for line in LinesWithEndings::from(code) {
        let ops = match state.parse_line(line, syntaxes) {
            Ok(ops) => ops,
            Err(err) => {
                debug!("{language} code not colored past byte {line_start}: {err}");
                break;
            }
        };
        let mut at = 0;
        for (offset, op) in ops {
            push(&mut tokens, line_start + at..line_start + offset, &stack);
            at = offset;
            if let Err(err) = stack.apply(&op) {
                debug!("{language} code not colored past byte {line_start}: {err}");
                return tokens;
            }
        }
        push(&mut tokens, line_start + at..line_start + line.len(), &stack);
        line_start += line.len();
    }
    tokens
}

fn push(tokens: &mut Vec<(Range<usize>, Token)>, range: Range<usize>, stack: &ScopeStack) {
    if range.is_empty() {
        return;
    }
    let Some(token) = stack.as_slice().iter().rev().find_map(|scope| {
        rules()
            .iter()
            .find(|(prefix, _)| prefix.is_prefix_of(*scope))
            .map(|(_, token)| *token)
    }) else {
        return;
    };
    match tokens.last_mut() {
        Some((last, kind)) if last.end == range.start && *kind == token => last.end = range.end,
        _ => tokens.push((range, token)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, highlight};

    fn kinds<'a>(language: &str, code: &'a str) -> Vec<(&'a str, Token)> {
        highlight(language, code)
            .into_iter()
            .map(|(range, token)| (&code[range], token))
            .collect()
    }

    #[test]
    fn rust_code_gets_its_kinds() {
        let tokens = kinds("rust", "// hi\nfn main() { let x = \"a\"; }\n");
        assert!(tokens.contains(&("// hi\n", Token::Comment)));
        assert!(tokens.contains(&("fn", Token::Keyword)));
        assert!(tokens.contains(&("main", Token::Function)));
        assert!(tokens.contains(&("let", Token::Keyword)));
        assert!(tokens.iter().any(|(text, token)| text.contains('a') && *token == Token::String));
    }

    #[test]
    fn a_diff_marks_its_lines() {
        let tokens = kinds("diff", "- old\n+ new\n");
        assert!(
            tokens
                .iter()
                .any(|(text, token)| text.contains("old") && *token == Token::Deleted)
        );
        assert!(
            tokens
                .iter()
                .any(|(text, token)| text.contains("new") && *token == Token::Inserted)
        );
    }

    #[test]
    fn an_unknown_language_gets_no_color() {
        assert_eq!(kinds("no-such-language", "x = 1"), []);
    }
}
