use std::fmt::Display;

use parking_lot::Mutex;

#[derive(Debug)]
pub struct TestState {
    string: Mutex<String>,
}

impl TestState {
    pub const fn new() -> Self {
        Self {
            string: Mutex::new(String::new()),
        }
    }

    pub fn add(&'static self, val: impl Display) {
        *self.string.lock() += &format!("|{val}|");
    }

    pub fn get(&'static self) -> String {
        self.string.lock().clone()
    }

    pub fn clear(&'static self) {
        self.string.lock().clear();
    }
}

impl PartialEq<&str> for TestState {
    fn eq(&self, other: &&str) -> bool {
        let mut lock = self.string.lock();
        let eq = lock.as_str() == *other;
        if eq {
            lock.clear();
        }
        eq
    }
}

#[cfg(test)]
mod test {
    use crate::gm::test_state::TestState;

    static STATE: TestState = TestState::new();

    #[test]
    fn test_state() {
        STATE.add(5);
        STATE.add(5.0);
        STATE.add("aaa");
        assert_eq!(STATE.get(), "|5||5||aaa|");
    }
}
