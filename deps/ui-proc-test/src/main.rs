#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use hilen::ui::Button;
use ui_proc::view;

#[view]
struct _ProcView {
    _button:      Button,
    #[init]
    _weak_button: Button,
}

// Pins where-clause support: the macro must carry it into generated impls.
#[view]
struct _GenericView<T>
where T: Default + 'static {
    _value: T,
}

fn main() {}
