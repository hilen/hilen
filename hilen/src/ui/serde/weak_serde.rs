use std::{collections::HashSet, mem::transmute, sync::OnceLock};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{deps::refs::RawPointer, ui::WeakView};

#[derive(Serialize, Deserialize)]
pub(super) struct WeakRepr {
    addr:      usize,
    stamp:     u64,
    type_name: String,
}

impl From<WeakView> for WeakRepr {
    fn from(weak: WeakView) -> Self {
        let raw = weak.raw();

        Self {
            addr:      raw.addr(),
            stamp:     raw.stamp(),
            type_name: raw.type_name().to_string(),
        }
    }
}

impl From<WeakRepr> for WeakView {
    fn from(repr: WeakRepr) -> Self {
        unsafe {
            WeakView::from_raw(RawPointer::new(
                repr.addr,
                repr.stamp,
                string_to_static(repr.type_name),
            ))
        }
    }
}

fn string_to_static(string: String) -> &'static str {
    static STR_STORAGE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

    let mut storage = STR_STORAGE.get_or_init(|| Mutex::new(HashSet::new())).lock();

    storage.insert(string.clone());

    let result = storage.get(&string).unwrap().as_str();

    unsafe { transmute(result) }
}
