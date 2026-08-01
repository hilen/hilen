#![cfg(test)]

use std::ops::DerefMut;

use hreads::set_current_thread_as_main;
use serial_test::serial;

use crate::{
    Own,
    editor::{EditedCallback, Editor},
};

#[derive(Default)]
struct Data {
    number: i32,
}

struct DataHolder {
    data:     Data,
    callback: Box<dyn FnMut(i32)>,
}

impl DataHolder {
    fn data(&mut self) -> Editor<'_, DataHolder, Data> {
        Editor::new(self, &self.data)
    }
}

impl EditedCallback for DataHolder {
    fn edited(&mut self) {
        self.callback.deref_mut()(self.data.number);
    }
}

#[test]
#[serial]
fn test_editor() {
    set_current_thread_as_main();

    let test = Own::new(0);
    let mut test = test.weak();

    assert_eq!(*test, 0);

    let mut data = DataHolder {
        data:     Data::default(),
        callback: Box::new(move |a| {
            *test += a;
        }),
    };

    data.data().number = 10;
    assert_eq!(*test, 10);

    data.data().number = 40;
    assert_eq!(*test, 50);
}
