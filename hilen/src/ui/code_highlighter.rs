use std::{borrow::Cow, error::Error, ops::Range, str::FromStr, sync::OnceLock};

use log::debug;
use syntect::{
    parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

use crate::ui::{MarkdownStyle, UIColor};

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
        Some(Self::with(syntax_for_file(file_name)?, &MarkdownStyle::current()))
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
        let style = self.style;
        self.tokens(code)
            .into_iter()
            .map(|(range, token)| (range, style.syntax(token)))
            .collect()
    }

    /// Forgets the code so far, the next call starts at the top of a
    /// file.
    pub fn reset(&mut self) {
        self.state = ParseState::new(self.syntax);
        self.stack = ScopeStack::new();
    }

    fn tokens(&mut self, code: &str) -> Vec<(Range<usize>, Token)> {
        let syntaxes = syntaxes();
        let mut tokens: Vec<(Range<usize>, Token)> = Vec::new();
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
            for (offset, op) in ops {
                let offset = offset.min(line.len());
                push(&mut tokens, line_start + at..line_start + offset, &self.stack);
                at = offset;
                if let Err(err) = self.stack.apply(&op) {
                    self.failed(line_start, &err);
                    return tokens;
                }
            }
            push(&mut tokens, line_start + at..line_start + line.len(), &self.stack);
            line_start += line.len();
        }
        tokens
    }

    fn failed(&mut self, at: usize, err: &dyn Error) {
        debug!("{} code not colored past byte {at}: {err}", self.syntax.name);
        self.reset();
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

    use super::{CodeHighlighter, Token, syntax_for_file, syntaxes};
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
            .tokens(code)
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

        let expected = for_file("a.rs").tokens(code);

        let mut by_line = for_file("a.rs");
        let mut start = 0;
        let mut joined = Vec::new();
        for line in code.lines() {
            for (range, token) in by_line.tokens(line) {
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
