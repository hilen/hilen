use std::{
    fmt::{Display, Write},
    sync::OnceLock,
};

use parking_lot::{Mutex, MutexGuard};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{from_str, to_string};

static STATE: OnceLock<State> = OnceLock::new();

#[derive(Default)]
struct State {
    val: Mutex<String>,
}

fn get() -> MutexGuard<'static, String> {
    STATE.get_or_init(Default::default).val.lock()
}

pub fn clear_state() {
    *get() = String::default();
}

pub fn get_state<T: DeserializeOwned + Default>() -> T {
    from_str(&get()).unwrap_or_default()
}

#[allow(dead_code)]
pub fn get_str_state() -> String {
    get_state::<String>()
}

pub fn set_state<T: Serialize>(val: T) {
    *get() = to_string(&val).unwrap();
}

pub fn increment_state() {
    let val: u32 = get_state();
    set_state(val + 1);
}

pub fn append_state(val: impl Display) {
    let mut stored: String = get_state();
    write!(stored, "{val}").unwrap();
    set_state(stored);
}

#[cfg(test)]
mod test {

    use crate::ui::ui_test::{
        clear_state, get_state,
        state::{append_state, increment_state},
    };

    #[test]
    fn test_state() {
        assert_eq!(get_state::<u32>(), 0);
        increment_state();
        increment_state();
        increment_state();
        assert_eq!(get_state::<u32>(), 3);
        clear_state();
        assert_eq!(get_state::<u32>(), 0);
        increment_state();
        assert_eq!(get_state::<u32>(), 1);

        clear_state();
        assert_eq!(get_state::<String>(), "");
        append_state("a");
        assert_eq!(get_state::<String>(), "a");
        append_state("_lalala");
        assert_eq!(get_state::<String>(), "a_lalala");
    }
}
