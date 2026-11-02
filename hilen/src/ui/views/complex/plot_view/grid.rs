const MAX_DECIMALS: usize = 12;

/// The step between 2 grid lines: 1, 2 or 5 times a power of 10, the
/// nearest one to `raw`, like a ruler or a graphing calculator.
pub(super) fn nice_step(raw: f64) -> f64 {
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let nice = if fraction < 1.5 {
        1.0
    } else if fraction < 3.5 {
        2.0
    } else if fraction < 7.5 {
        5.0
    } else {
        10.0
    };
    nice * power
}

/// The values of the grid lines from `min` to `max`, both included.
pub(super) fn lines(min: f64, max: f64, step: f64) -> Vec<f64> {
    let first = (min / step).ceil();
    let mut values = Vec::new();
    let mut index = 0.0;
    loop {
        let value = (first + index) * step;
        if value > max {
            return values;
        }
        values.push(value);
        index += 1.0;
    }
}

/// A grid value as text with as many decimals as the step needs, so a
/// step of 0.5 shows 1.5 and a step of 2 shows 4, never 4.0000001.
pub(super) fn label(value: f64, step: f64) -> String {
    let mut decimals = 0;
    let mut scaled = step;
    while scaled < 0.999_999 && decimals < MAX_DECIMALS {
        scaled *= 10.0;
        decimals += 1;
    }
    let text = format!("{value:.decimals$}");
    if text.trim_start_matches('-').chars().all(|char| char == '0' || char == '.') {
        return "0".to_string();
    }
    text
}

#[cfg(test)]
mod test {
    use super::{label, lines, nice_step};

    #[test]
    fn steps_are_1_2_5() {
        assert_eq!(nice_step(0.9), 1.0);
        assert_eq!(nice_step(1.7), 2.0);
        assert_eq!(nice_step(4.0), 5.0);
        assert_eq!(nice_step(8.0), 10.0);
        assert_eq!(nice_step(23.0), 20.0);
        assert!((nice_step(0.0042) - 0.005).abs() < 1e-12);
    }

    #[test]
    fn lines_cover_the_range() {
        assert_eq!(lines(-2.5, 2.5, 1.0), [-2.0, -1.0, 0.0, 1.0, 2.0]);
        assert_eq!(lines(0.0, 10.0, 5.0), [0.0, 5.0, 10.0]);
        assert_eq!(lines(0.1, 0.9, 1.0), [] as [f64; 0]);
    }

    #[test]
    fn labels_follow_the_step() {
        assert_eq!(label(4.0, 2.0), "4");
        assert_eq!(label(1.5, 0.5), "1.5");
        assert_eq!(label(0.300_000_000_000_000_04, 0.1), "0.3");
        assert_eq!(label(-0.0, 1.0), "0");
        assert_eq!(label(0.000_000_1, 0.5), "0");
        assert_eq!(label(-20.0, 5.0), "-20");
    }
}
