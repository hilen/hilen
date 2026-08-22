# Hilen

My attempt to create a cross platform game engine and UI framework from scratch using Rust.
Rendering based on WGPU.

<img width="766" height="647" alt="image" src="https://github.com/user-attachments/assets/8ac5695f-8252-4fe6-b572-a17c56ef6ddb" />


Previously written in `C++`: https://github.com/VladasZ/test_engine_cpp

Inspired by `Cross++`: https://github.com/maxon887/Cross

---

## Rust toolchain

Hilen requires nightly Rust. The repository pins a known working nightly in
`rust-toolchain.toml`.

Two nightly features are fundamental to the view API.

Views are owned by `Own<T>`, while application code uses the copyable, non-owning `Weak<T>`
handle. View methods therefore use `Weak<Self>` as the receiver:

```rust
fn setup(self: Weak<Self>) {
    self.button.on_tap(move || self.do_something());
}
```

Using a custom smart pointer as `self` requires `arbitrary_self_types`. It keeps ordinary method
syntax and allows `self` to be copied into `'static` callbacks without `Rc<RefCell<_>>`, explicit
lifetimes or cloning an owning pointer.

The engine also provides default behavior for every view through blanket implementations, while
individual views override only the methods they need:

```rust
impl<T: View + 'static> Setup for T {
    default fn setup(self: Weak<Self>) {}
}

impl Setup for MainScreen {
    fn setup(self: Weak<Self>) {
        // Configure this view.
    }
}
```

These overlapping implementations require `specialization`. Supporting stable Rust would require
redesigning both the method receiver and the default view behavior, not just removing feature
flags.

---

## Platform support

Hilen currently supports Windows, Linux, macOS, iOS, Android and WebAssembly. The Android
APK builds fully inside docker via `make android`, so no host Android tooling is needed.
Minimum supported Android version is 8.0, API 26. tvOS builds and renders in the Apple TV
simulator, display only with no input path yet, see [docs/tvos.md](docs/tvos.md).

---

Simplest example:

```rust
// main.rs

#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(arbitrary_self_types)]

use hilen::{
    App,
    refs::{Own, Weak},
    ui::{Label, Setup, U8Color, UIManager, View, ViewData, view},
};

#[view]
struct MainScreen {
    #[init]
    hello_label: Label,
}

impl Setup for MainScreen {
    fn setup(self: Weak<Self>) {
        // The screen background. Leave the root view transparent, a colored root
        // view stops at the iOS safe area edge while the clear color fills the screen.
        UIManager::set_clear_color("#4E4D5C");

        self.hello_label
            .set_text("Hello Test Engine!")
            .set_color(U8Color::rgba(156, 149, 220, 255))
            .set_corner_radius(10)
            .set_border_color("#228CDB")
            .set_border_width(5)
            .set_text_size(40);

        self.hello_label.place().center().size(400, 80);
    }
}

#[derive(Default)]
struct ExampleApp;

impl App for ExampleApp {
    fn make_root_view(&self) -> Own<dyn View> {
        MainScreen::new()
    }
}

fn main() {
    ExampleApp::start();
}

```

All fields after `#[init]` must be views. They are created and managed by the engine —
just declare and use them.




Result:

<img width="550" height="305" alt="image" src="https://github.com/user-attachments/assets/ac661f61-8984-4d02-a273-5e24897f8691" />
