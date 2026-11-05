#[cfg(feature = "scene")]
use std::f64::consts::TAU;

use exmex::{BinOp, Express, FlatEx, MakeOperators, Operator, ops_factory};
use num_complex::Complex64;

use super::formula::ExpressionError;
#[cfg(feature = "scene")]
use crate::{gm::LossyConvert, ui::Color};

ops_factory!(
    ComplexOps,
    Complex64,
    Operator::make_bin_unary(
        "-",
        BinOp {
            apply:          |a, b| a - b,
            prio:           0,
            is_commutative: false,
        },
        |a| -a
    ),
    Operator::make_bin(
        "+",
        BinOp {
            apply:          |a, b| a + b,
            prio:           0,
            is_commutative: true,
        }
    ),
    Operator::make_bin(
        "*",
        BinOp {
            apply:          |a, b| a * b,
            prio:           1,
            is_commutative: true,
        }
    ),
    Operator::make_bin(
        "/",
        BinOp {
            apply:          |a, b| a / b,
            prio:           1,
            is_commutative: false,
        }
    ),
    Operator::make_bin(
        "^",
        BinOp {
            apply:          power,
            prio:           2,
            is_commutative: false,
        }
    ),
    Operator::make_unary("sin", Complex64::sin),
    Operator::make_unary("cos", Complex64::cos),
    Operator::make_unary("tan", Complex64::tan),
    Operator::make_unary("sinh", Complex64::sinh),
    Operator::make_unary("cosh", Complex64::cosh),
    Operator::make_unary("exp", Complex64::exp),
    Operator::make_unary("ln", Complex64::ln),
    Operator::make_unary("sqrt", Complex64::sqrt),
    Operator::make_unary("abs", |a| Complex64::new(a.norm(), 0.0)),
    Operator::make_unary("conj", |a| a.conj()),
    Operator::make_unary("re", |a| Complex64::new(a.re, 0.0)),
    Operator::make_unary("im", |a| Complex64::new(a.im, 0.0)),
    Operator::make_constant("i", Complex64::i()),
    Operator::make_constant("PI", Complex64::new(std::f64::consts::PI, 0.0)),
    Operator::make_constant("E", Complex64::new(std::f64::consts::E, 0.0))
);

/// A whole power like `z^3` multiplies, which keeps `(-1)^2` exactly 1. Any
/// other power goes through the logarithm.
fn power(base: Complex64, exponent: Complex64) -> Complex64 {
    if exponent.im == 0.0 && exponent.re.fract() == 0.0 && exponent.re.abs() <= 64.0 {
        let mut result = Complex64::new(1.0, 0.0);
        let mut left = exponent.re.abs();
        while left >= 1.0 {
            result *= base;
            left -= 1.0;
        }
        if exponent.re < 0.0 { result.inv() } else { result }
    } else {
        base.powc(exponent)
    }
}

/// A function of the complex `z`, like `z^2` or `(z - 1)/(z + 1)`.
#[derive(Debug)]
pub struct ComplexExpression {
    flat: FlatEx<Complex64, ComplexOps>,
}

impl ComplexExpression {
    pub fn parse(body: &str) -> Result<Self, ExpressionError> {
        let flat = FlatEx::<Complex64, ComplexOps>::parse(body)
            .map_err(|error| ExpressionError::Parse(error.to_string()))?;
        if let Some(name) = flat.var_names().iter().find(|name| name.as_str() != "z") {
            return Err(ExpressionError::Variable(name.clone()));
        }
        Ok(Self { flat })
    }

    pub fn eval(&self, z: Complex64) -> Result<Complex64, ExpressionError> {
        let values: &[Complex64] = if self.flat.var_names().is_empty() {
            &[]
        } else {
            &[z]
        };
        self.flat.eval(values).map_err(|error| ExpressionError::Eval(error.to_string()))
    }
}

/// The domain coloring of a value: the hue is its angle, red for positive
/// real numbers, and the brightness rises from dark to light between each
/// doubling of its size, so rings mark the size. 0 is black, a value that
/// is not finite is white.
#[cfg(feature = "scene")]
pub fn domain_color(value: Complex64) -> Color {
    if !value.re.is_finite() || !value.im.is_finite() {
        return Color::rgba(1.0, 1.0, 1.0, 1.0);
    }
    let size = value.norm();
    if size == 0.0 {
        return Color::rgba(0.0, 0.0, 0.0, 1.0);
    }
    let hue = value.arg().rem_euclid(TAU) / TAU;
    let ring = size.log2().rem_euclid(1.0);
    hsv(hue, 0.85, 0.55 + 0.4 * ring)
}

#[cfg(feature = "scene")]
fn hsv(hue: f64, saturation: f64, value: f64) -> Color {
    let sector = hue * 6.0;
    let within = sector.fract();
    let low = value * (1.0 - saturation);
    let falling = value * (1.0 - saturation * within);
    let rising = value * (1.0 - saturation * (1.0 - within));
    let (r, g, b) = if sector < 1.0 {
        (value, rising, low)
    } else if sector < 2.0 {
        (falling, value, low)
    } else if sector < 3.0 {
        (low, value, rising)
    } else if sector < 4.0 {
        (low, falling, value)
    } else if sector < 5.0 {
        (rising, low, value)
    } else {
        (value, low, falling)
    };
    Color::rgba(r.lossy_convert(), g.lossy_convert(), b.lossy_convert(), 1.0)
}

#[cfg(test)]
mod tests {
    use num_complex::Complex64;

    use super::ComplexExpression;
    #[cfg(feature = "scene")]
    use super::domain_color;

    fn value(text: &str, z: Complex64) -> Complex64 {
        ComplexExpression::parse(text).expect("parse").eval(z).expect("eval")
    }

    #[test]
    fn arithmetic_with_i() {
        assert_eq!(value("z^2", Complex64::new(0.0, 1.0)), Complex64::new(-1.0, 0.0));
        assert_eq!(value("z*i", Complex64::new(1.0, 0.0)), Complex64::new(0.0, 1.0));
        assert_eq!(
            value("-z + 1", Complex64::new(2.0, 3.0)),
            Complex64::new(-1.0, -3.0)
        );
        assert_eq!(value("1/z", Complex64::new(0.0, 2.0)), Complex64::new(0.0, -0.5));
        assert_eq!(
            value("re(z) - im(z)", Complex64::new(5.0, 2.0)),
            Complex64::new(3.0, 0.0)
        );
        assert!((value("exp(i*PI)", Complex64::default()) + 1.0).norm() < 1e-12);
    }

    #[test]
    fn only_z_is_a_variable() {
        assert!(ComplexExpression::parse("z + x").is_err());
    }

    #[cfg(feature = "scene")]
    #[test]
    fn colors_mark_zero_and_infinity() {
        let black = domain_color(Complex64::new(0.0, 0.0));
        assert_eq!((black.r, black.g, black.b), (0.0, 0.0, 0.0));
        let white = domain_color(Complex64::new(f64::INFINITY, 0.0));
        assert_eq!((white.r, white.g, white.b), (1.0, 1.0, 1.0));
        let positive = domain_color(Complex64::new(1.0, 0.0));
        assert!(positive.r > positive.g && positive.r > positive.b);
    }
}
