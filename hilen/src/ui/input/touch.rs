use std::{fmt::Display, str::FromStr};

use crate::{
    gm::{LossyConvert, flat::Point},
    ui::{TouchLock, input::TouchEvent},
    window::MouseButton,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Touch {
    pub id:       usize,
    pub position: Point,
    pub event:    TouchEvent,
    pub button:   MouseButton,
}

impl Touch {
    pub(crate) fn is_began(&self) -> bool {
        self.event == TouchEvent::Began
    }

    pub(crate) fn is_moved(&self) -> bool {
        self.event == TouchEvent::Moved
    }

    pub(crate) fn is_ended(&self) -> bool {
        self.event == TouchEvent::Ended
    }

    pub fn lock() -> TouchLock {
        TouchLock::new()
    }
}

impl Touch {
    pub(crate) fn vec_from_str(s: &str) -> Vec<Self> {
        s.split('\n')
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.parse().unwrap())
            .collect()
    }

    pub(crate) fn str_from_vec(v: Vec<Touch>) -> String {
        v.into_iter()
            .map(|t| "            ".to_string() + &t.to_string() + "\n")
            .collect()
    }
}

impl Display for Touch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: isize = self.position.x.lossy_convert();
        let y: isize = self.position.y.lossy_convert();

        write!(f, "{:<4} {:<4} {}", x, y, self.event)?;

        // Only multitouch prints the id, so recorded single finger tests keep
        // their exact three column form.
        if self.id != 1 {
            write!(f, " {}", self.id)?;
        }

        Ok(())
    }
}

impl From<&str> for Touch {
    fn from(value: &str) -> Self {
        Touch::from_str(value).unwrap()
    }
}

impl FromStr for Touch {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vals: Vec<_> = s.split_whitespace().collect();

        // A fourth token is the finger id for multitouch. Without it the
        // touch is the lone finger 1, so every single finger test is unchanged.
        let id = match vals.get(3) {
            Some(id) => id.parse()?,
            None => 1,
        };

        let touch = Touch {
            id,
            position: Point {
                x: vals[0].parse()?,
                y: vals[1].parse()?,
            },
            event: vals[2].parse()?,
            button: MouseButton::Left,
        };

        Ok(touch)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        ui::{Touch, input::TouchEvent},
        window::MouseButton,
    };

    #[test]
    fn touch_to_string() {
        let touches = [
            Touch {
                id:       1,
                position: (0, 0).into(),
                event:    TouchEvent::Began,
                button:   MouseButton::Left,
            },
            Touch {
                id:       1,
                position: (2000, 10).into(),
                event:    TouchEvent::Ended,
                button:   MouseButton::Left,
            },
            Touch {
                id:       1,
                position: (100, 4000).into(),
                event:    TouchEvent::Ended,
                button:   MouseButton::Left,
            },
            Touch {
                id:       1,
                position: (1, 4000).into(),
                event:    TouchEvent::Moved,
                button:   MouseButton::Left,
            },
            Touch {
                id:       1,
                position: (4000, 1).into(),
                event:    TouchEvent::Moved,
                button:   MouseButton::Left,
            },
        ];

        let result: String = touches.into_iter().map(|t| t.to_string() + "\n").collect();

        println!("{result}");

        assert_eq!(
            result,
            r"0    0    b
2000 10   e
100  4000 e
1    4000 m
4000 1    m
"
        );

        assert_eq!(touches.as_slice(), &Touch::vec_from_str(&result));

        assert_eq!(
            touches.as_slice(),
            &Touch::vec_from_str(
                r"
                                       0             0 b
                                    2000            10 e
                                     100          4000 e
                                       1          4000 m
                                    4000             1 m
                "
            )
        );

        assert_eq!(
            vec![Touch {
                id:       1,
                position: (10, 20).into(),
                event:    TouchEvent::Began,
                button:   MouseButton::Left,
            }],
            Touch::vec_from_str("10 20 b")
        );
    }
}
