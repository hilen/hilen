//! LaTeX math as a tree of nodes. It reads the part of LaTeX that people
//! write in a chat and in notes: scripts, fractions, roots, big operators,
//! brackets that grow, accents, fonts, spaces and the matrix and cases
//! environments. Anything else is an error, and the caller shows the
//! source as it is.

use anyhow::{Result, bail};

use super::symbols::{Alphabet, accent, function, function_text, large_operator, negated, space, symbol};

/// How an atom spaces itself against its neighbours, the classes of TeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

/// The size a part of a formula is set in, the 4 styles of TeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Display,
    Text,
    Script,
    ScriptScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GridKind {
    /// A matrix, with the brackets around it.
    Matrix(Option<char>, Option<char>),
    /// Rows with a brace on the left, every column starts at its left.
    Cases,
    /// Columns that take turns to align right and left, like `aligned`.
    Aligned,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Char {
        ch:    char,
        class: Class,
    },
    /// Upright text, from `\text` and the names of functions.
    Text(String),
    Group(Vec<Node>),
    Scripts {
        base: Box<Node>,
        sup:  Option<Vec<Node>>,
        sub:  Option<Vec<Node>>,
    },
    Frac {
        num:    Vec<Node>,
        den:    Vec<Node>,
        rule:   bool,
        level:  Option<Level>,
        /// The brackets around it, for `\binom`.
        delims: Option<(char, char)>,
    },
    Sqrt {
        index: Option<Vec<Node>>,
        body:  Vec<Node>,
    },
    /// A large operator like `\sum`, or a function with limits like
    /// `\lim`. `limits` is whether its scripts go above and below.
    Op {
        symbol:         Option<char>,
        name:           Option<String>,
        limits:         Option<bool>,
        /// The limits go above and below in a display formula.
        display_limits: bool,
    },
    /// `\left ... \right`, brackets that grow to the height of the body.
    Fenced {
        left:  Option<char>,
        right: Option<char>,
        body:  Vec<Node>,
    },
    /// A bracket of a fixed bigger size, `\big(` and its kin, 1 to 4.
    Big {
        ch:    char,
        size:  u8,
        class: Class,
    },
    Accent {
        ch:   char,
        wide: bool,
        body: Vec<Node>,
    },
    Overline(Vec<Node>),
    Underline(Vec<Node>),
    /// A brace over or under its body, `\overbrace` and `\underbrace`.
    Brace {
        over: bool,
        body: Vec<Node>,
    },
    /// A body with a small formula over or under it, `\overset`.
    Stack {
        base:  Vec<Node>,
        over:  Option<Vec<Node>>,
        under: Option<Vec<Node>>,
    },
    /// Space in ems.
    Space(f32),
    Grid {
        kind: GridKind,
        rows: Vec<Vec<Vec<Node>>>,
    },
    /// `\displaystyle` and its kin, for the rest of the list.
    Style(Level),
}

pub(crate) fn parse(source: &str) -> Result<Vec<Node>> {
    let mut parser = Parser {
        chars:    source.chars().collect(),
        at:       0,
        alphabet: Alphabet::Math,
    };
    let list = parser.list(Stop::End)?;
    if parser.at < parser.chars.len() {
        bail!("an extra {:?} at {}", parser.chars[parser.at], parser.at);
    }
    Ok(list)
}

/// What ends a list.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    End,
    Brace,
    /// `]` ends the index of a root.
    Bracket,
    /// `\right` ends the body of `\left`.
    Right,
    /// A cell ends at `&`, `\\` or `\end`.
    Cell,
}

/// How a list ended, for a cell of a grid.
#[derive(PartialEq, Eq)]
enum Ended {
    Plain,
    Column,
    Row,
    End,
}

struct Parser {
    chars:    Vec<char>,
    at:       usize,
    alphabet: Alphabet,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => self.at += 1,
                Some('%') => {
                    while let Some(ch) = self.peek() {
                        self.at += 1;
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    /// The name after a backslash: letters, or 1 char that is not one.
    fn command_name(&mut self) -> Result<String> {
        let start = self.at;
        while self.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.at += 1;
        }
        if self.at == start {
            let Some(ch) = self.peek() else {
                bail!("a backslash at the end");
            };
            self.at += 1;
            return Ok(ch.to_string());
        }
        let name: String = self.chars[start..self.at].iter().collect();
        // A star belongs to the name, `\operatorname*`.
        if self.peek() == Some('*') && matches!(name.as_str(), "operatorname" | "boxed") {
            self.at += 1;
            return Ok(format!("{name}*"));
        }
        Ok(name)
    }

    fn list(&mut self, stop: Stop) -> Result<Vec<Node>> {
        Ok(self.list_ended(stop)?.0)
    }

    fn list_ended(&mut self, stop: Stop) -> Result<(Vec<Node>, Ended)> {
        let mut list: Vec<Node> = Vec::new();
        loop {
            self.skip_space();
            let Some(ch) = self.peek() else {
                if stop == Stop::End {
                    return Ok((list, Ended::Plain));
                }
                bail!("the formula ends inside a group");
            };
            match ch {
                '}' => {
                    if stop == Stop::Brace {
                        self.at += 1;
                        return Ok((list, Ended::Plain));
                    }
                    bail!("a }} with no {{");
                }
                ']' if stop == Stop::Bracket => {
                    self.at += 1;
                    return Ok((list, Ended::Plain));
                }
                '&' if stop == Stop::Cell => {
                    self.at += 1;
                    return Ok((list, Ended::Column));
                }
                '^' | '_' => {
                    self.at += 1;
                    let script = self.argument()?;
                    attach(&mut list, ch == '^', script)?;
                }
                '\'' => {
                    let mut primes = 0;
                    while self.peek() == Some('\'') {
                        self.at += 1;
                        primes += 1;
                    }
                    let prime = match primes {
                        1 => '′',
                        2 => '″',
                        _ => '‴',
                    };
                    attach(
                        &mut list,
                        true,
                        vec![Node::Char {
                            ch:    prime,
                            class: Class::Ord,
                        }],
                    )?;
                }
                '\\' => {
                    let save = self.at;
                    self.at += 1;
                    let name = self.command_name()?;
                    match name.as_str() {
                        "right" if stop == Stop::Right => return Ok((list, Ended::Plain)),
                        "\\" | "cr" if stop == Stop::Cell => {
                            // An optional space after it, `\\[4pt]`.
                            self.skip_space();
                            if self.peek() == Some('[') {
                                self.group_text('[', ']')?;
                            }
                            return Ok((list, Ended::Row));
                        }
                        "end" if stop == Stop::Cell => {
                            self.at = save;
                            return Ok((list, Ended::End));
                        }
                        "limits" | "nolimits" => {
                            let wanted = name == "limits";
                            match list.last_mut() {
                                Some(Node::Op { limits, .. }) => *limits = Some(wanted),
                                Some(Node::Scripts { base, .. }) => {
                                    if let Node::Op { limits, .. } = base.as_mut() {
                                        *limits = Some(wanted);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {
                            let nodes = self.command(&name)?;
                            list.extend(nodes);
                        }
                    }
                }
                '{' => {
                    self.at += 1;
                    list.push(Node::Group(self.list(Stop::Brace)?));
                }
                _ => {
                    self.at += 1;
                    list.push(self.plain_char(ch));
                }
            }
        }
    }

    fn plain_char(&self, ch: char) -> Node {
        let (ch, class) = match ch {
            '-' => ('−', Class::Bin),
            '*' => ('∗', Class::Bin),
            '+' => ('+', Class::Bin),
            '=' | '<' | '>' | ':' => (ch, Class::Rel),
            ',' | ';' => (ch, Class::Punct),
            '(' | '[' => (ch, Class::Open),
            ')' | ']' | '!' | '?' => (ch, Class::Close),
            '~' => return Node::Space(0.25),
            _ => (self.alphabet.map(ch), Class::Ord),
        };
        Node::Char { ch, class }
    }

    /// The argument of a command or a script: a group in braces, or 1
    /// token.
    fn argument(&mut self) -> Result<Vec<Node>> {
        self.skip_space();
        match self.peek() {
            None => bail!("an argument is missing"),
            Some('{') => {
                self.at += 1;
                self.list(Stop::Brace)
            }
            Some('\\') => {
                self.at += 1;
                let name = self.command_name()?;
                self.command(&name)
            }
            Some(ch) if matches!(ch, '}' | '&' | '^' | '_') => bail!("an argument is missing before {ch:?}"),
            Some(ch) => {
                self.at += 1;
                Ok(vec![self.plain_char(ch)])
            }
        }
    }

    /// The raw text between 2 chars, for `\text` and environment names.
    fn group_text(&mut self, open: char, close: char) -> Result<String> {
        self.skip_space();
        if self.peek() != Some(open) {
            // 1 char with no braces, `\text x`.
            let Some(ch) = self.peek() else {
                bail!("an argument is missing");
            };
            self.at += 1;
            return Ok(ch.to_string());
        }
        self.at += 1;
        let mut depth = 0;
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            self.at += 1;
            if ch == '\\' {
                if let Some(next) = self.peek() {
                    self.at += 1;
                    if next.is_ascii_alphabetic() {
                        // A command inside text is not read, only its
                        // name is left out.
                        while self.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) {
                            self.at += 1;
                        }
                    } else {
                        text.push(next);
                    }
                }
                continue;
            }
            if ch == open && open == '{' {
                depth += 1;
            } else if ch == close {
                if depth == 0 {
                    return Ok(text);
                }
                depth -= 1;
            }
            if ch != '{' && ch != '}' {
                text.push(ch);
            }
        }
        bail!("a {open} with no {close}")
    }

    /// The char of a bracket after `\left`, `\right` or `\big`. A dot is
    /// no bracket.
    fn delimiter(&mut self) -> Result<Option<char>> {
        self.skip_space();
        let Some(ch) = self.peek() else {
            bail!("a bracket is missing");
        };
        self.at += 1;
        if ch == '.' {
            return Ok(None);
        }
        if ch != '\\' {
            return Ok(Some(ch));
        }
        let name = self.command_name()?;
        match symbol(&name) {
            Some((ch, _)) => Ok(Some(ch)),
            None => match name.as_str() {
                "uparrow" | "downarrow" => Ok(Some(if name == "uparrow" { '↑' } else { '↓' })),
                _ => bail!("\\{name} is no bracket"),
            },
        }
    }

    fn with_alphabet(&mut self, alphabet: Alphabet) -> Result<Vec<Node>> {
        let old = self.alphabet;
        self.alphabet = alphabet;
        let body = self.argument();
        self.alphabet = old;
        body
    }

    fn command(&mut self, name: &str) -> Result<Vec<Node>> {
        if let Some((ch, class)) = symbol(name) {
            return Ok(vec![Node::Char { ch, class }]);
        }
        if let Some((ch, display_limits)) = large_operator(name) {
            return Ok(vec![Node::Op {
                symbol: Some(ch),
                name: None,
                limits: None,
                display_limits,
            }]);
        }
        if let Some(display_limits) = function(name) {
            return Ok(vec![Node::Op {
                symbol: None,
                name: Some(function_text(name).to_string()),
                limits: None,
                display_limits,
            }]);
        }
        if let Some((ch, wide)) = accent(name) {
            let body = self.argument()?;
            return Ok(vec![Node::Accent { ch, wide, body }]);
        }
        if let Some(em) = space(name) {
            return Ok(vec![Node::Space(em)]);
        }
        if let Some(alphabet) = Alphabet::of_command(name) {
            return Ok(vec![Node::Group(self.with_alphabet(alphabet)?)]);
        }
        if let Some(nodes) = self.structure(name)? {
            return Ok(nodes);
        }
        if let Some(nodes) = self.decoration(name)? {
            return Ok(nodes);
        }
        match self.other(name)? {
            Some(nodes) => Ok(nodes),
            None => bail!("\\{name} is not known"),
        }
    }

    /// Fractions, roots and brackets.
    fn structure(&mut self, name: &str) -> Result<Option<Vec<Node>>> {
        let nodes = match name {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.argument()?;
                let den = self.argument()?;
                let level = match name {
                    "dfrac" | "cfrac" => Some(Level::Display),
                    "tfrac" => Some(Level::Text),
                    _ => None,
                };
                vec![Node::Frac {
                    num,
                    den,
                    rule: true,
                    level,
                    delims: None,
                }]
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.argument()?;
                let den = self.argument()?;
                let level = match name {
                    "dbinom" => Some(Level::Display),
                    "tbinom" => Some(Level::Text),
                    _ => None,
                };
                vec![Node::Frac {
                    num,
                    den,
                    rule: false,
                    level,
                    delims: Some(('(', ')')),
                }]
            }
            "sqrt" => {
                self.skip_space();
                let index = if self.peek() == Some('[') {
                    self.at += 1;
                    Some(self.list(Stop::Bracket)?)
                } else {
                    None
                };
                let body = self.argument()?;
                vec![Node::Sqrt { index, body }]
            }
            "left" => {
                let left = self.delimiter()?;
                let body = self.list(Stop::Right)?;
                let right = self.delimiter()?;
                vec![Node::Fenced { left, right, body }]
            }
            "right" => bail!("a \\right with no \\left"),
            "middle" => match self.delimiter()? {
                Some(ch) => vec![Node::Char {
                    ch,
                    class: Class::Rel,
                }],
                None => Vec::new(),
            },
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr" | "Bigr"
            | "biggr" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                let size = match name.trim_end_matches(['l', 'r', 'm']) {
                    "big" => 1,
                    "Big" => 2,
                    "bigg" => 3,
                    _ => 4,
                };
                let class = if name.ends_with('l') {
                    Class::Open
                } else if name.ends_with('r') {
                    Class::Close
                } else if name.ends_with('m') {
                    Class::Rel
                } else {
                    Class::Ord
                };
                match self.delimiter()? {
                    Some(ch) => vec![Node::Big { ch, size, class }],
                    None => Vec::new(),
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(nodes))
    }

    /// Text, names and the marks over and under a formula.
    fn decoration(&mut self, name: &str) -> Result<Option<Vec<Node>>> {
        let nodes = match name {
            "text" | "textrm" | "textnormal" | "mbox" | "textup" | "textbf" | "textit" | "textsf"
            | "texttt" | "hbox" => {
                let text = self.group_text('{', '}')?;
                vec![Node::Text(text)]
            }
            "operatorname" | "operatorname*" => {
                let text = self.group_text('{', '}')?;
                vec![Node::Op {
                    symbol:         None,
                    name:           Some(text),
                    limits:         None,
                    display_limits: name.ends_with('*'),
                }]
            }
            "overline" => vec![Node::Overline(self.argument()?)],
            "underline" => vec![Node::Underline(self.argument()?)],
            "overbrace" => vec![Node::Brace {
                over: true,
                body: self.argument()?,
            }],
            "underbrace" => vec![Node::Brace {
                over: false,
                body: self.argument()?,
            }],
            "overset" | "stackrel" => {
                let over = self.argument()?;
                let base = self.argument()?;
                vec![Node::Stack {
                    base,
                    over: Some(over),
                    under: None,
                }]
            }
            "underset" => {
                let under = self.argument()?;
                let base = self.argument()?;
                vec![Node::Stack {
                    base,
                    over: None,
                    under: Some(under),
                }]
            }
            "not" => {
                let next = self.argument()?;
                match next.as_slice() {
                    [Node::Char { ch, class }] => match negated(*ch) {
                        Some(ch) => vec![Node::Char { ch, class: *class }],
                        None => next,
                    },
                    _ => next,
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(nodes))
    }

    /// Styles, classes, spaces and the commands that draw nothing.
    fn other(&mut self, name: &str) -> Result<Option<Vec<Node>>> {
        let nodes = match name {
            "displaystyle" => vec![Node::Style(Level::Display)],
            "textstyle" => vec![Node::Style(Level::Text)],
            "scriptstyle" => vec![Node::Style(Level::Script)],
            "scriptscriptstyle" => vec![Node::Style(Level::ScriptScript)],
            "color" => {
                self.group_text('{', '}')?;
                Vec::new()
            }
            "textcolor" | "colorbox" => {
                self.group_text('{', '}')?;
                vec![Node::Group(self.argument()?)]
            }
            "boxed" | "boxed*" | "mathop" | "mathord" | "mathinner" => vec![Node::Group(self.argument()?)],
            "mathbin" => self.classed(Class::Bin)?,
            "mathrel" => self.classed(Class::Rel)?,
            "mathopen" => self.classed(Class::Open)?,
            "mathclose" => self.classed(Class::Close)?,
            "mathpunct" => self.classed(Class::Punct)?,
            "phantom" | "hphantom" | "vphantom" => {
                self.argument()?;
                Vec::new()
            }
            "hspace" | "kern" | "mkern" | "mspace" | "label" | "tag" | "nonumber" | "notag" => {
                if !matches!(name, "nonumber" | "notag") {
                    self.group_text('{', '}')?;
                }
                Vec::new()
            }
            "bmod" => vec![Node::Op {
                symbol:         None,
                name:           Some("mod".to_string()),
                limits:         Some(false),
                display_limits: false,
            }],
            "mod" => vec![
                Node::Space(1.0),
                Node::Text("mod".to_string()),
                Node::Space(6.0 / 18.0),
            ],
            "pmod" => {
                let body = self.argument()?;
                vec![
                    Node::Space(1.0),
                    Node::Char {
                        ch:    '(',
                        class: Class::Open,
                    },
                    Node::Text("mod".to_string()),
                    Node::Space(6.0 / 18.0),
                    Node::Group(body),
                    Node::Char {
                        ch:    ')',
                        class: Class::Close,
                    },
                ]
            }
            "begin" => self.environment()?,
            _ => return Ok(None),
        };
        Ok(Some(nodes))
    }

    fn classed(&mut self, class: Class) -> Result<Vec<Node>> {
        let body = self.argument()?;
        Ok(match body.as_slice() {
            [Node::Char { ch, .. }] => vec![Node::Char { ch: *ch, class }],
            _ => vec![Node::Group(body)],
        })
    }

    fn environment(&mut self) -> Result<Vec<Node>> {
        let name = self.group_text('{', '}')?;
        let kind = match name.trim_end_matches('*') {
            "matrix" | "smallmatrix" => GridKind::Matrix(None, None),
            "pmatrix" => GridKind::Matrix(Some('('), Some(')')),
            "bmatrix" => GridKind::Matrix(Some('['), Some(']')),
            "Bmatrix" => GridKind::Matrix(Some('{'), Some('}')),
            "vmatrix" => GridKind::Matrix(Some('|'), Some('|')),
            "Vmatrix" => GridKind::Matrix(Some('‖'), Some('‖')),
            "array" => {
                // The column spec is not read, every column is centered.
                self.group_text('{', '}')?;
                GridKind::Matrix(None, None)
            }
            "cases" | "dcases" => GridKind::Cases,
            "aligned" | "align" | "alignat" | "alignedat" | "split" | "gathered" | "gather" | "eqnarray"
            | "equation" | "multline" => {
                if name.starts_with("alignat") {
                    self.group_text('{', '}')?;
                }
                GridKind::Aligned
            }
            other => bail!("the environment {other} is not known"),
        };
        let mut rows: Vec<Vec<Vec<Node>>> = Vec::new();
        let mut row: Vec<Vec<Node>> = Vec::new();
        loop {
            let (cell, ended) = self.list_ended(Stop::Cell)?;
            row.push(cell);
            match ended {
                Ended::Column => {}
                Ended::Row => rows.push(std::mem::take(&mut row)),
                Ended::End => {
                    self.at += 1;
                    let end = self.command_name()?;
                    if end != "end" {
                        bail!("\\end expected");
                    }
                    let closing = self.group_text('{', '}')?;
                    if closing != name {
                        bail!("\\begin{{{name}}} ends with \\end{{{closing}}}");
                    }
                    break;
                }
                Ended::Plain => bail!("\\begin{{{name}}} has no \\end"),
            }
        }
        // A last `\\` leaves an empty row behind.
        if !(row.len() == 1 && row[0].is_empty()) {
            rows.push(row);
        }
        Ok(vec![Node::Grid { kind, rows }])
    }
}

/// Puts a script on the last node of the list, an empty base when there
/// is none.
fn attach(list: &mut Vec<Node>, is_power: bool, script: Vec<Node>) -> Result<()> {
    let previous = list.pop().unwrap_or(Node::Group(Vec::new()));
    let node = match previous {
        Node::Scripts {
            base,
            sup: power,
            sub: index,
        } => {
            let (sup, sub) = if is_power {
                (Some(join(power, script)?), index)
            } else {
                if index.is_some() {
                    bail!("a double subscript");
                }
                (power, Some(script))
            };
            Node::Scripts { base, sup, sub }
        }
        other => Node::Scripts {
            base: Box::new(other),
            sup:  is_power.then(|| script.clone()),
            sub:  (!is_power).then_some(script),
        },
    };
    list.push(node);
    Ok(())
}

/// A prime and then a superscript, `f'^2`, go into 1 superscript. Two
/// superscripts in any other order are an error, as in TeX.
fn join(old: Option<Vec<Node>>, new: Vec<Node>) -> Result<Vec<Node>> {
    let Some(mut old) = old else {
        return Ok(new);
    };
    let primes = old.iter().all(|node| {
        matches!(
            node,
            Node::Char {
                ch: '′' | '″' | '‴',
                ..
            }
        )
    });
    if !primes {
        bail!("a double superscript");
    }
    old.extend(new);
    Ok(old)
}

#[cfg(test)]
mod tests {
    use super::{Class, Node, parse};

    #[test]
    fn letters_are_italic_and_digits_and_operators_are_not() {
        let nodes = parse("x+1").unwrap();
        assert_eq!(
            nodes,
            [
                Node::Char {
                    ch:    '\u{1D465}',
                    class: Class::Ord,
                },
                Node::Char {
                    ch:    '+',
                    class: Class::Bin,
                },
                Node::Char {
                    ch:    '1',
                    class: Class::Ord,
                },
            ]
        );
    }

    #[test]
    fn a_power_and_an_index_go_on_their_base() {
        let nodes = parse("x_i^{2}").unwrap();
        let [Node::Scripts { base, sup, sub }] = &nodes[..] else {
            panic!("1 node with scripts expected, got {nodes:?}");
        };
        assert!(matches!(base.as_ref(), Node::Char { ch: '\u{1D465}', .. }));
        assert_eq!(sup.as_ref().map(Vec::len), Some(1));
        assert_eq!(sub.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn a_prime_and_a_power_share_the_superscript() {
        let nodes = parse("f'^2").unwrap();
        let [Node::Scripts { sup: Some(sup), .. }] = &nodes[..] else {
            panic!("1 node with a superscript expected, got {nodes:?}");
        };
        assert_eq!(sup.len(), 2);
    }

    #[test]
    fn commands_of_a_real_formula_are_read() {
        for source in [
            r"e^{i\pi} + 1 = 0",
            r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
            r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}",
            r"\int_0^1 x^{-1/2}\,dx",
            r"\left( \frac{a}{b} \right)^{n}",
            r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
            r"f(x) = \begin{cases} 1 & x > 0 \\ 0 & \text{otherwise} \end{cases}",
            r"\mathbb{R}^n \to \mathcal{L}",
            r"\hat{x} \vec{v} \overline{AB}",
            r"(\cos a + i\sin a)^n = \cos(na) + i\sin(na)",
            r"x^{1/n} = \sqrt[n]{x}",
            r"\binom{n}{k} \not= 0",
        ] {
            assert!(
                parse(source).is_ok(),
                "{source} did not parse: {:?}",
                parse(source)
            );
        }
    }

    #[test]
    fn a_broken_formula_is_an_error() {
        for source in [
            r"\frac{a}",
            r"x^",
            r"{a",
            r"a}",
            r"\unknowncommand",
            r"\left( a",
            r"x_1_2",
        ] {
            assert!(parse(source).is_err(), "{source} parsed");
        }
    }
}
