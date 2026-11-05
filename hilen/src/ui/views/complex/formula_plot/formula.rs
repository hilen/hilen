use std::fmt::{self, Display, Formatter};

use exmex::{Express, FlatEx, parse};
use num_complex::Complex64;

use super::complex_math::ComplexExpression;

/// What an expression draws: a curve `y = f(x)`, a surface `z = f(x, y)`
/// or the domain coloring of a complex function `w = f(z)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Curve,
    Surface,
    Complex,
}

/// A function typed by the user, like `x^2`, `y = sin(x)` or `z = x*y`.
#[derive(Debug)]
enum Inner {
    Real(Box<FlatEx<f64>>),
    Complex(Box<ComplexExpression>),
}

#[derive(Debug)]
pub struct Expression {
    inner: Inner,
    kind:  Kind,
}

#[derive(Debug, PartialEq)]
pub enum ExpressionError {
    Empty,
    LeftSide(String),
    Parse(String),
    Variable(String),
    Eval(String),
}

impl Display for ExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Type a function of x, of x and y, or of the complex z"),
            Self::LeftSide(left) => write!(f, "Only y = ..., z = ... and w = ... work, not {left} = ..."),
            Self::Parse(message) | Self::Eval(message) => write!(f, "{message}"),
            Self::Variable(name) => write!(f, "Unknown variable {name}, only x and y work"),
        }
    }
}

impl Expression {
    pub fn parse(text: &str) -> Result<Self, ExpressionError> {
        let (left, body) = split(text)?;
        let body = body.trim();
        if body.is_empty() {
            return Err(ExpressionError::Empty);
        }
        let body = with_implicit_multiplication(body);
        if left == Some(Kind::Complex) || names_complex(&body) {
            let complex = ComplexExpression::parse(&body)?;
            return Ok(Self {
                inner: Inner::Complex(Box::new(complex)),
                kind:  Kind::Complex,
            });
        }
        let flat = parse::<f64>(&body).map_err(|error| ExpressionError::Parse(error.to_string()))?;
        if let Some(name) = flat.var_names().iter().find(|name| !matches!(name.as_str(), "x" | "y")) {
            return Err(ExpressionError::Variable(name.clone()));
        }
        let uses_y = flat.var_names().iter().any(|name| name == "y");
        let kind = match left {
            Some(Kind::Curve) if uses_y => return Err(ExpressionError::Variable("y".to_string())),
            Some(kind) => kind,
            None if uses_y => Kind::Surface,
            None => Kind::Curve,
        };
        Ok(Self {
            inner: Inner::Real(Box::new(flat)),
            kind,
        })
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The value at `x` and `y` of a curve or a surface. A curve does not read
    /// `y`.
    pub fn eval(&self, x: f64, y: f64) -> Result<f64, ExpressionError> {
        let Inner::Real(flat) = &self.inner else {
            return Err(ExpressionError::Eval(
                "a complex function has no real value".to_string(),
            ));
        };
        // exmex takes the values in the order of the sorted variable names.
        let values: Vec<f64> = flat.var_names().iter().map(|name| if name == "x" { x } else { y }).collect();
        flat.eval(&values).map_err(|error| ExpressionError::Eval(error.to_string()))
    }

    /// The value at the complex `z` of a complex function.
    pub fn eval_complex(&self, z: Complex64) -> Result<Complex64, ExpressionError> {
        let Inner::Complex(complex) = &self.inner else {
            return Err(ExpressionError::Eval(
                "a real function takes no complex z".to_string(),
            ));
        };
        complex.eval(z)
    }
}

/// `z` or `i` anywhere makes a complex function, like `z^2 + i`.
fn names_complex(body: &str) -> bool {
    body.split(|char: char| !char.is_alphanumeric())
        .any(|name| name == "z" || name == "i")
}

/// The kind the left side names and the part after `=`. No `=` names no kind.
fn split(text: &str) -> Result<(Option<Kind>, &str), ExpressionError> {
    let Some((left, right)) = text.split_once('=') else {
        return Ok((None, text));
    };
    match left.trim() {
        "y" | "f(x)" => Ok((Some(Kind::Curve), right)),
        "z" | "f(x,y)" | "f(x, y)" => Ok((Some(Kind::Surface), right)),
        "w" | "f(z)" => Ok((Some(Kind::Complex), right)),
        other => Err(ExpressionError::LeftSide(other.to_string())),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Token {
    Number,
    /// `x`, `y`, `z`, or several of them written together like `xy`.
    Variables,
    Name,
    Open,
    Close,
    Other,
}

/// Writes out the multiplication a person leaves out, `5x` as `5*x`,
/// `2(x+1)` as `2*(x+1)` and `x sin(x)` as `x*sin(x)`. A function name
/// like `sin` keeps its `(`.
fn with_implicit_multiplication(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut previous: Option<Token> = None;
    let mut index = 0;
    while index < chars.len() {
        let char = chars[index];
        if char.is_whitespace() {
            out.push(char);
            index += 1;
            continue;
        }
        let start = index;
        let token = if char.is_ascii_digit() || char == '.' {
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            Token::Number
        } else if char.is_alphabetic() || char == '_' {
            while index < chars.len() && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let name: String = chars[start..index].iter().collect();
            if name.chars().all(|letter| matches!(letter, 'x' | 'y' | 'z')) {
                Token::Variables
            } else {
                Token::Name
            }
        } else {
            index += 1;
            match char {
                '(' => Token::Open,
                ')' => Token::Close,
                _ => Token::Other,
            }
        };
        let ends_value = matches!(previous, Some(Token::Number | Token::Variables | Token::Close));
        let starts_value = matches!(
            token,
            Token::Number | Token::Variables | Token::Name | Token::Open
        );
        if ends_value && starts_value {
            out.push('*');
        }
        if token == Token::Variables {
            let letters: Vec<String> = chars[start..index].iter().map(char::to_string).collect();
            out.push_str(&letters.join("*"));
        } else {
            out.extend(&chars[start..index]);
        }
        previous = Some(token);
    }
    out
}

#[cfg(test)]
mod tests {
    use num_complex::Complex64;

    use super::{Expression, ExpressionError, Kind, with_implicit_multiplication};

    fn value(text: &str, x: f64) -> f64 {
        Expression::parse(text).expect("parse").eval(x, 0.0).expect("eval")
    }

    #[test]
    fn plain_function() {
        assert_eq!(value("x^2", 3.0), 9.0);
        assert_eq!(value("2*x+1", -2.0), -3.0);
    }

    #[test]
    fn left_side_is_dropped() {
        assert_eq!(value("y = x^2 - 1", 2.0), 3.0);
        assert_eq!(value("f(x) = 3*x", 2.0), 6.0);
    }

    #[test]
    fn constant_has_no_variable() {
        assert_eq!(value("y = 5", 100.0), 5.0);
    }

    #[test]
    fn functions_work() {
        assert!(value("sin(x)", 0.0).abs() < 1e-12);
        assert!((value("exp(x)", 1.0) - 1.0_f64.exp()).abs() < 1e-12);
    }

    #[test]
    fn errors() {
        assert_eq!(Expression::parse("  ").unwrap_err(), ExpressionError::Empty);
        assert_eq!(Expression::parse("y = ").unwrap_err(), ExpressionError::Empty);
        assert_eq!(
            Expression::parse("x*w").unwrap_err(),
            ExpressionError::Variable("w".to_string())
        );
        assert_eq!(
            Expression::parse("y = x*y").unwrap_err(),
            ExpressionError::Variable("y".to_string())
        );
        assert_eq!(
            Expression::parse("g = x").unwrap_err(),
            ExpressionError::LeftSide("g".to_string())
        );
        assert!(matches!(
            Expression::parse("x +* 2").unwrap_err(),
            ExpressionError::Parse(_)
        ));
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(with_implicit_multiplication("x^20 + 5x"), "x^20 + 5*x");
        assert_eq!(with_implicit_multiplication("2(x+1)"), "2*(x+1)");
        assert_eq!(with_implicit_multiplication("(x+1)(x-1)"), "(x+1)*(x-1)");
        assert_eq!(with_implicit_multiplication("x sin(x)"), "x *sin(x)");
        assert_eq!(
            with_implicit_multiplication("sin(x) + exp(2x)"),
            "sin(x) + exp(2*x)"
        );
        assert_eq!(value("y = x^20 + 5x", 1.0), 6.0);
        assert_eq!(value("3x(x-1)", 2.0), 6.0);
        assert_eq!(with_implicit_multiplication("2xy + yx^2"), "2*x*y + y*x^2");
        assert_eq!(with_implicit_multiplication("exp(x)"), "exp(x)");
    }

    #[test]
    fn surfaces() {
        let saddle = Expression::parse("z = x^2 - y^2").expect("parse");
        assert_eq!(saddle.kind(), Kind::Surface);
        assert_eq!(saddle.eval(3.0, 2.0).expect("eval"), 5.0);
        let plain = Expression::parse("2xy").expect("parse");
        assert_eq!(plain.kind(), Kind::Surface);
        assert_eq!(plain.eval(2.0, 3.0).expect("eval"), 12.0);
        assert_eq!(
            Expression::parse("z = y").expect("parse").eval(5.0, 7.0).expect("eval"),
            7.0
        );
        assert_eq!(Expression::parse("x^2").expect("parse").kind(), Kind::Curve);
    }

    #[test]
    fn complex_functions() {
        let square = Expression::parse("w = z^2").expect("parse");
        assert_eq!(square.kind(), Kind::Complex);
        assert_eq!(
            square.eval_complex(Complex64::new(0.0, 1.0)).expect("eval"),
            Complex64::new(-1.0, 0.0)
        );
        let plain = Expression::parse("2z + i").expect("parse");
        assert_eq!(plain.kind(), Kind::Complex);
        assert_eq!(
            plain.eval_complex(Complex64::new(1.0, 0.0)).expect("eval"),
            Complex64::new(2.0, 1.0)
        );
        assert_eq!(Expression::parse("sin(x)").expect("parse").kind(), Kind::Curve);
    }
}
