// The crate `dispatch` 0.2, which the Objective-C crates of winit use, tells
// the linker to take a library `dispatch` on every system but macOS and iOS.
// tvOS has none, its dispatch functions are in `libSystem` like on iOS. A
// normal tvOS build is a static library and links nothing itself. A hot
// build is a dynamic library, see docs/hot-reload.md, and its link fails
// with `library 'dispatch' not found`. This file is that library, with
// nothing in it.

typedef int hilen_dispatch_stub;
