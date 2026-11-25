use std::{borrow::Cow, error::Error, ops::Range, str::FromStr, sync::OnceLock};

use log::debug;
use syntect::{
    parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

use crate::ui::{CodeScope, MarkdownStyle, UIColor};

/// What a piece of code is, the style has a color per kind.
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

/// What `CodeHighlighter::runs` gives for some code.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CodeRuns {
    /// The byte ranges that get a color, in order, like `color_runs`.
    pub colors: Vec<(Range<usize>, UIColor)>,
    /// The byte ranges inside a string or a comment, in order. A range
    /// that goes on over the break at the end of a line holds that break,
    /// also the break the last line would have, 1 byte past the code. So
    /// a place at the end of a line comment or of an open string is
    /// inside, and the place after a closed string is not.
    pub scopes: Vec<(Range<usize>, CodeScope)>,
}

/// The tokens and the scopes of some code, see `CodeRuns`.
struct Parsed {
    tokens: Vec<(Range<usize>, Token)>,
    scopes: Vec<(Range<usize>, CodeScope)>,
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

fn scope_rules() -> &'static [(Scope, CodeScope)] {
    static SCOPES_PARSED: OnceLock<Vec<(Scope, CodeScope)>> = OnceLock::new();
    SCOPES_PARSED.get_or_init(|| {
        [("comment", CodeScope::Comment), ("string", CodeScope::String)]
            .iter()
            .filter_map(|(prefix, scope)| Some((Scope::from_str(prefix).ok()?, *scope)))
            .collect()
    })
}

/// The scope of a comment that ends with its line.
fn line_comment() -> Option<Scope> {
    static LINE_COMMENT: OnceLock<Option<Scope>> = OnceLock::new();
    *LINE_COMMENT.get_or_init(|| Scope::from_str("comment.line").ok())
}

/// `op` takes a line comment off `stack`.
fn pops_line_comment(stack: &ScopeStack, op: &ScopeStackOp) -> bool {
    let (ScopeStackOp::Pop(count), Some(line_comment)) = (op, line_comment()) else {
        return false;
    };
    stack
        .as_slice()
        .iter()
        .rev()
        .take(*count)
        .any(|scope| line_comment.is_prefix_of(*scope))
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

/// The language of a file by its name: the whole name first, for a
/// `Makefile`, then what follows the last dot.
fn syntax_for_file(file_name: &str) -> Option<&'static SyntaxReference> {
    let name = file_name.rsplit(['/', '\\']).next()?;
    let syntaxes = syntaxes();
    syntaxes.find_syntax_by_extension(name).or_else(|| {
        let (_, extension) = name.rsplit_once('.')?;
        syntaxes.find_syntax_by_extension(extension)
    })
}

/// Colors code by its language, for a `Label` that shows code. It gives
/// byte ranges with colors, the shape `Label::set_color_runs` takes, in
/// the colors a code block of `MarkdownView` has: the syntax colors of
/// the `MarkdownStyle` that is global when the highlighter is made. Each
/// color is a light and dark pair, so a label follows a theme switch
/// with no new call.
///
/// A highlighter keeps where the code stands after each call, inside a
/// block comment or a long string. So the lines of a file can go in 1 by
/// 1, in order, each into its own label:
///
/// ```ignore
/// let mut code = CodeHighlighter::for_file("src/main.rs")?;
/// for (label, line) in labels.iter().zip(lines) {
///     label.set_text(line);
///     label.set_color_runs(code.color_runs(line));
/// }
/// ```
///
/// `reset` starts clean again, for a line colored alone or a jump to
/// another place of the file. A clone goes on from the same place, the
/// way to keep the state of a line and come back to it.
///
/// Make it on the main thread, it reads the global style. After that it
/// colors on any thread.
#[derive(Clone)]
pub struct CodeHighlighter {
    syntax: &'static SyntaxReference,
    state:  ParseState,
    stack:  ScopeStack,
    style:  MarkdownStyle,
}

impl CodeHighlighter {
    /// For a file by its name or its path, like `a.rs`, `src/x.tsx` or
    /// `Makefile`. None for a file of a language nobody knows.
    pub fn for_file(file_name: &str) -> Option<Self> {
        Self::for_file_styled(file_name, &MarkdownStyle::current())
    }

    /// `for_file` in the colors of `style`, on any thread.
    pub(crate) fn for_file_styled(file_name: &str, style: &MarkdownStyle) -> Option<Self> {
        Some(Self::with(syntax_for_file(file_name)?, style))
    }

    /// For a language by the word a markdown code block names it with,
    /// like `rust`, `rs` or `diff`. None for a language nobody knows.
    pub fn for_language(language: &str) -> Option<Self> {
        Some(Self::with(
            syntaxes().find_syntax_by_token(language)?,
            &MarkdownStyle::current(),
        ))
    }

    fn with(syntax: &'static SyntaxReference, style: &MarkdownStyle) -> Self {
        Self {
            syntax,
            state: ParseState::new(syntax),
            stack: ScopeStack::new(),
            style: *style,
        }
    }

    /// The byte ranges of `code` that get a color, in order. `code` is
    /// whole lines, 1 or many, and goes on from where the call before
    /// ended. The break at the end of the last line may be left out. A
    /// line the language definition fails on ends the coloring of this
    /// call there, and the next call starts clean.
    pub fn color_runs(&mut self, code: &str) -> Vec<(Range<usize>, UIColor)> {
        self.runs(code).colors
    }

    /// The colors of `code` like `color_runs`, and where its strings and
    /// comments are, from 1 pass. A code editor skips some pairs there,
    /// see `CodeEditor::set_scope_runs`.
    pub fn runs(&mut self, code: &str) -> CodeRuns {
        let style = self.style;
        let Parsed { tokens, scopes } = self.parse(code);
        CodeRuns {
            colors: tokens.into_iter().map(|(range, token)| (range, style.syntax(token))).collect(),
            scopes,
        }
    }

    /// Forgets the code so far, the next call starts at the top of a
    /// file.
    pub fn reset(&mut self) {
        self.state = ParseState::new(self.syntax);
        self.stack = ScopeStack::new();
    }

    fn parse(&mut self, code: &str) -> Parsed {
        let syntaxes = syntaxes();
        let mut tokens: Vec<(Range<usize>, Token)> = Vec::new();
        let mut scopes: Vec<(Range<usize>, CodeScope)> = Vec::new();
        let mut line_start = 0;
        for line in LinesWithEndings::from(code) {
            // The definitions end a line comment at the line break, a
            // line without one would leave the next line inside it.
            let ended = if line.ends_with('\n') {
                Cow::Borrowed(line)
            } else {
                Cow::Owned(format!("{line}\n"))
            };
            let ops = match self.state.parse_line(&ended, syntaxes) {
                Ok(ops) => ops,
                Err(err) => {
                    self.failed(line_start, &err);
                    break;
                }
            };
            let mut at = 0;
            // The scopes count the break too, also the one put in above.
            let mut scope_at = 0;
            let break_at = ended.len() - 1;
            let mut comment_ended = false;
            for (offset, op) in ops {
                push_scope(
                    &mut scopes,
                    line_start + scope_at..line_start + offset,
                    &self.stack,
                );
                scope_at = offset;
                comment_ended |= offset == break_at && pops_line_comment(&self.stack, &op);
                let offset = offset.min(line.len());
                push(&mut tokens, line_start + at..line_start + offset, &self.stack);
                at = offset;
                if let Err(err) = self.stack.apply(&op) {
                    self.failed(line_start, &err);
                    return Parsed { tokens, scopes };
                }
            }
            push(&mut tokens, line_start + at..line_start + line.len(), &self.stack);
            push_scope(
                &mut scopes,
                line_start + scope_at..line_start + ended.len(),
                &self.stack,
            );
            // Some definitions end a line comment before the break, like
            // the `#` of TOML. It holds the break all the same, see
            // `CodeRuns`.
            if comment_ended
                && let Some((range, CodeScope::Comment)) = scopes.last_mut()
                && range.end == line_start + break_at
            {
                range.end = line_start + ended.len();
            }
            line_start += line.len();
        }
        Parsed { tokens, scopes }
    }

    fn failed(&mut self, at: usize, err: &dyn Error) {
        debug!("{} code not colored past byte {at}: {err}", self.syntax.name);
        self.reset();
    }
}

/// The innermost string or comment of the stack wins, so an escape in a
/// string is still in the string.
fn push_scope(scopes: &mut Vec<(Range<usize>, CodeScope)>, range: Range<usize>, stack: &ScopeStack) {
    if range.is_empty() {
        return;
    }
    let Some(scope) = stack.as_slice().iter().rev().find_map(|scope| {
        scope_rules()
            .iter()
            .find(|(prefix, _)| prefix.is_prefix_of(*scope))
            .map(|(_, kind)| *kind)
    }) else {
        return;
    };
    match scopes.last_mut() {
        Some((last, kind)) if last.end == range.start && *kind == scope => last.end = range.end,
        _ => scopes.push((range, scope)),
    }
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
    use std::ops::Range;

    use super::{CodeHighlighter, CodeScope, Token, syntax_for_file, syntaxes};
    use crate::ui::MarkdownStyle;

    fn for_language(language: &str) -> Option<CodeHighlighter> {
        let syntax = syntaxes().find_syntax_by_token(language)?;
        Some(CodeHighlighter::with(syntax, &MarkdownStyle::DEFAULT))
    }

    fn for_file(file_name: &str) -> CodeHighlighter {
        let syntax = syntax_for_file(file_name).unwrap_or_else(|| panic!("no language for {file_name}"));
        CodeHighlighter::with(syntax, &MarkdownStyle::DEFAULT)
    }

    fn kinds<'a>(highlighter: &mut CodeHighlighter, code: &'a str) -> Vec<(&'a str, Token)> {
        highlighter
            .parse(code)
            .tokens
            .into_iter()
            .map(|(range, token)| (&code[range], token))
            .collect()
    }

    #[test]
    fn rust_code_gets_its_kinds() {
        let mut rust = for_language("rust").unwrap();
        let tokens = kinds(&mut rust, "// hi\nfn main() { let x = \"a\"; }\n");
        assert!(tokens.contains(&("// hi\n", Token::Comment)));
        assert!(tokens.contains(&("fn", Token::Keyword)));
        assert!(tokens.contains(&("main", Token::Function)));
        assert!(tokens.contains(&("let", Token::Keyword)));
        assert!(tokens.iter().any(|(text, token)| text.contains('a') && *token == Token::String));
    }

    #[test]
    fn a_diff_marks_its_lines() {
        let mut diff = for_language("diff").unwrap();
        let tokens = kinds(&mut diff, "- old\n+ new\n");
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
    fn an_unknown_language_has_no_highlighter() {
        assert!(for_language("no-such-language").is_none());
        assert!(syntax_for_file("notes.no-such-extension").is_none());
        assert!(syntax_for_file("no-such-name").is_none());
    }

    #[test]
    fn a_file_name_gives_its_language() {
        for (file, language) in [
            ("a.rs", "Rust"),
            ("src/ui/main.rs", "Rust"),
            (r"C:\work\main.RS", "Rust"),
            ("Makefile", "Makefile"),
            ("build/Makefile", "Makefile"),
            ("x.tsx", "TypeScriptReact"),
            ("x.ts", "TypeScript"),
            ("index.d.ts", "TypeScript"),
            ("Cargo.toml", "TOML"),
            ("page.html", "HTML"),
            ("run.py", "Python"),
            ("notes.md", "Markdown"),
        ] {
            let syntax = syntax_for_file(file).unwrap_or_else(|| panic!("no language for {file}"));
            assert_eq!(syntax.name, language, "{file}");
        }
    }

    #[test]
    fn a_line_without_its_break_is_colored_like_with_it() {
        let mut rust = for_file("a.rs");
        let with_break = kinds(&mut rust, "let x = 1; // one\n");
        rust.reset();
        let without = kinds(&mut rust, "let x = 1; // one");

        assert_eq!(without.last(), Some(&("// one", Token::Comment)));
        assert_eq!(with_break.last(), Some(&("// one\n", Token::Comment)));
        assert_eq!(with_break.len(), without.len());

        // The comment ended with its line, though the line had no break.
        assert!(kinds(&mut rust, "let y = 2;").contains(&("let", Token::Keyword)));
    }

    /// Every byte with a color that is not a line break. The whole text
    /// also colors the break of a line, a single line has none.
    fn visible(code: &str, tokens: Vec<(Range<usize>, Token)>) -> Vec<(usize, Token)> {
        tokens
            .into_iter()
            .flat_map(|(range, token)| range.map(move |byte| (byte, token)))
            .filter(|(byte, _)| code.as_bytes()[*byte] != b'\n')
            .collect()
    }

    #[test]
    fn lines_one_by_one_get_the_colors_of_the_whole_text() {
        let code = r#"/* a comment
   let inside = 1;
   the end */ let after = "text
   still text";
fn done() {}
"#;

        let expected = for_file("a.rs").parse(code).tokens;

        let mut by_line = for_file("a.rs");
        let mut start = 0;
        let mut joined = Vec::new();
        for line in code.lines() {
            for (range, token) in by_line.parse(line).tokens {
                joined.push((range.start + start..range.end + start, token));
            }
            start += line.len() + 1;
        }

        assert_ne!(expected, []);
        assert_eq!(visible(code, joined), visible(code, expected));
    }

    #[test]
    fn the_state_goes_from_one_line_to_the_next() {
        let mut rust = for_file("a.rs");
        assert_eq!(kinds(&mut rust, "/* open"), [("/* open", Token::Comment)]);
        assert_eq!(kinds(&mut rust, "let x = 1;"), [("let x = 1;", Token::Comment)]);

        // A clone goes on from the same place and leaves the first alone.
        let mut copy = rust.clone();
        assert!(kinds(&mut copy, "*/ let y = 2;").contains(&("let", Token::Keyword)));
        assert_eq!(kinds(&mut rust, "let z = 3;"), [("let z = 3;", Token::Comment)]);

        rust.reset();
        assert!(kinds(&mut rust, "let x = 1;").contains(&("let", Token::Keyword)));
    }

    #[test]
    fn strings_and_comments_have_their_scopes() {
        let mut rust = for_file("a.rs");
        // The comment holds the break the line would have, the closed
        // string ends at its quote.
        let code = r#"let s = "ab"; // c"#;
        assert_eq!(
            rust.parse(code).scopes,
            [(8..12, CodeScope::String), (14..19, CodeScope::Comment)]
        );
        // An open string goes on over the break and on the next line.
        rust.reset();
        assert_eq!(rust.parse(r#"let s = "ab"#).scopes, [(8..12, CodeScope::String)]);
        assert_eq!(rust.parse(r#"cd";"#).scopes, [(0..3, CodeScope::String)]);
        // A placeholder in a string is still in the string.
        rust.reset();
        assert_eq!(rust.parse(r#"f("{x}");"#).scopes, [(2..7, CodeScope::String)]);
        // Whole lines count their breaks.
        rust.reset();
        assert_eq!(rust.parse("// a\nlet b;\n").scopes, [(0..5, CodeScope::Comment)]);
    }

    #[test]
    fn a_line_comment_holds_its_break_a_closed_block_comment_not() {
        let mut toml = for_file("notes.toml");
        // The place at the end of the comment is inside it, so a typed
        // quote there gets no closer.
        assert_eq!(toml.parse("# say ").scopes, [(0..7, CodeScope::Comment)]);
        toml.reset();
        assert_eq!(
            toml.parse("a = \"x\" # y\n").scopes,
            [(4..7, CodeScope::String), (8..12, CodeScope::Comment)]
        );
        // A block comment closed at the end of its line ends there, like
        // a closed string.
        assert_eq!(
            for_file("a.rs").parse("x /* a */").scopes,
            [(2..9, CodeScope::Comment)]
        );
    }

    #[test]
    fn the_runs_have_the_colors_of_color_runs() {
        let code = "fn main() { let s = \"a\"; } // go";
        let colors = for_file("a.rs").color_runs(code);
        assert_eq!(for_file("a.rs").runs(code).colors, colors);
    }

    #[test]
    fn empty_code_gets_no_color() {
        let mut rust = for_file("a.rs");
        assert_eq!(kinds(&mut rust, ""), []);
        assert_eq!(kinds(&mut rust, "\n"), []);
    }

    #[test]
    fn the_colors_are_the_ones_of_the_style() {
        let style = MarkdownStyle::DEFAULT;
        let mut rust = for_file("a.rs");
        let code = "fn main() {} // go";
        let runs = rust.color_runs(code);

        let color_of = |text: &str| {
            let at = code.find(text).unwrap();
            runs.iter().find(|(range, _)| range.start == at).map(|(_, color)| *color)
        };
        assert_eq!(color_of("fn"), Some(style.keyword));
        assert_eq!(color_of("main"), Some(style.function));
        assert_eq!(color_of("// go"), Some(style.comment));
    }
}
