use std::rc::Rc;

use super::formula::{Expression, Kind};

/// What 1 graph draws: the 2D curves of a block together, or 1 surface,
/// or 1 complex function as a surface of its size colored by its angle.
#[derive(Clone, Debug)]
pub struct Graph {
    kind:     Kind,
    /// The formulas as written, 1 for a surface.
    texts:    Vec<String>,
    formulas: Vec<Rc<Expression>>,
    /// The x the graph starts at, from a line like `x = -2..2`. A surface
    /// shows x and y from `-half` to `half`, half the larger end.
    range:    Option<(f64, f64)>,
}

impl Graph {
    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn texts(&self) -> &[String] {
        &self.texts
    }

    pub fn formulas(&self) -> &[Rc<Expression>] {
        &self.formulas
    }

    /// The x range the graph starts at: the one of the block, or -10 to
    /// 10, or -4 to 4 for a complex function.
    pub fn x_range(&self) -> (f64, f64) {
        self.range.unwrap_or(if self.kind == Kind::Complex {
            (-4.0, 4.0)
        } else {
            (-10.0, 10.0)
        })
    }

    /// Half the side of the square a surface stands over.
    pub fn half(&self) -> f64 {
        let (left, right) = self.x_range();
        left.abs().max(right.abs()).max(f64::MIN_POSITIVE)
    }
}

/// The graphs of a block of formulas, 1 per line, in the order of their
/// first line. Every 2D formula goes into 1 graph, each surface and each
/// complex function has its own. A line `x = a..b` sets the range every
/// graph starts at. An empty line counts for nothing. A line that does not
/// read stands as its error, with the line in it.
pub fn graphs(block: &str) -> Vec<Result<Graph, String>> {
    let mut range = None;
    let mut found: Vec<Result<Graph, String>> = Vec::new();
    for line in block.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(read) = read_range(line) {
            match read {
                Ok(read) => range = Some(read),
                Err(error) => found.push(Err(error)),
            }
            continue;
        }
        let formula = match Expression::parse(line) {
            Ok(formula) => Rc::new(formula),
            Err(error) => {
                found.push(Err(format!("{line}: {error}")));
                continue;
            }
        };
        let kind = formula.kind();
        #[cfg(not(feature = "scene"))]
        if kind != Kind::Curve {
            found.push(Err(format!(
                "{line}: a 3D graph needs the scene feature of hilen"
            )));
            continue;
        }
        if kind == Kind::Curve
            && let Some(Ok(curves)) = found
                .iter_mut()
                .find(|graph| matches!(graph, Ok(graph) if graph.kind == Kind::Curve))
        {
            curves.texts.push(line.to_string());
            curves.formulas.push(formula);
            continue;
        }
        found.push(Ok(Graph {
            kind,
            texts: vec![line.to_string()],
            formulas: vec![formula],
            range: None,
        }));
    }
    for graph in found.iter_mut().flatten() {
        graph.range = range;
    }
    found
}

/// A line `x = a..b`, none for a line of another shape.
fn read_range(line: &str) -> Option<Result<(f64, f64), String>> {
    let (left, right) = line.split_once('=')?;
    if left.trim() != "x" {
        return None;
    }
    let (low, high) = right.split_once("..")?;
    let number = |text: &str| text.trim().parse::<f64>().ok().filter(|value| value.is_finite());
    Some(match (number(low), number(high)) {
        (Some(low), Some(high)) if low < high => Ok((low, high)),
        _ => Err(format!(
            "{line}: a range is 2 numbers, the smaller first, like x = -2..2"
        )),
    })
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "scene")]
    use super::super::formula::Kind;
    use super::graphs;

    #[cfg(feature = "scene")]
    fn read(block: &str) -> Vec<(Kind, Vec<String>, (f64, f64))> {
        graphs(block)
            .into_iter()
            .map(|graph| {
                let graph = graph.expect("every line reads");
                (graph.kind(), graph.texts().to_vec(), graph.x_range())
            })
            .collect()
    }

    #[cfg(feature = "scene")]
    #[test]
    fn the_curves_of_a_block_share_1_graph() {
        assert_eq!(
            read("y = sin(x)\nz = x*y\n\ny = cos(x)"),
            vec![
                (
                    Kind::Curve,
                    vec!["y = sin(x)".to_string(), "y = cos(x)".to_string()],
                    (-10.0, 10.0)
                ),
                (Kind::Surface, vec!["z = x*y".to_string()], (-10.0, 10.0)),
            ]
        );
    }

    #[cfg(feature = "scene")]
    #[test]
    fn a_complex_function_has_its_own_graph_and_range() {
        assert_eq!(
            read("w = 1/z"),
            vec![(Kind::Complex, vec!["w = 1/z".to_string()], (-4.0, 4.0))]
        );
    }

    #[cfg(feature = "scene")]
    #[test]
    fn a_range_line_sets_where_every_graph_starts() {
        let found = read("y = x^3 - x\nx = -2..1.5\nz = x^2 - y^2");
        assert_eq!(found[0].2, (-2.0, 1.5));
        assert_eq!(found[1].2, (-2.0, 1.5));
        let surface = graphs("z = x*y\nx = -3..1").remove(0).unwrap();
        assert!((surface.half() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_line_that_does_not_read_stands_as_its_error() {
        let found = graphs("y = sin(x\nx = 2..1\ny = x");
        assert!(found[0].as_ref().is_err_and(|error| error.starts_with("y = sin(x: ")));
        assert!(found[1].as_ref().is_err_and(|error| error.starts_with("x = 2..1: ")));
        assert!(found[2].is_ok());
    }

    #[cfg(not(feature = "scene"))]
    #[test]
    fn a_3d_graph_needs_the_scene_feature() {
        let found = graphs("y = x\nz = x*y");
        assert!(found[0].is_ok());
        assert!(
            found[1]
                .as_ref()
                .is_err_and(|error| error.ends_with("needs the scene feature of hilen"))
        );
    }
}
