# refs

Smart pointers `Own` and `Weak`, the memory model of the whole engine. They started as the
[refs](https://github.com/VladasZ/refs) crate and now live inside `hilen` as the module
`hilen/src/deps/refs`, re-exported to apps as `hilen::refs`.

## Why

UI is a graph of objects that point at each other: views hold subviews, buttons hold callbacks,
callbacks point back at views. The Rust borrow checker cannot express this graph without heavy
workarounds (`Rc<RefCell<>>`, lifetimes everywhere). This engine uses its own pointers instead, so UI
code looks like Swift or TypeScript:

```rust
self.button.on_tap(move || self.do_thing());
```

No `clone()`, no `borrow_mut()`, no lifetimes.

This is an intentional framework tradeoff. Hilen prioritizes convenient application and UI
code over idiomatic Rust ownership purity. `Own` and `Weak` provide runtime checks for the
single-main-thread UI model instead of trying to express the whole UI graph through lifetimes and
standard smart pointers.

## How it works

- `Own<T>` — the single owner of an object. When `Own` drops, the object is freed.
- `Weak<T>` — a copyable pointer to an object owned by some `Own`. Does not affect lifetime.

Every `Own` registers its address in a global map together with a unique stamp (atomic counter).
A `Weak` remembers the address and the stamp. On every deref it checks: address is still in the map
and the stamp matches. If not — panic with the type name, instead of use-after-free.

The stamp protects from address reuse: when the allocator gives the same address to a new object,
the old `Weak` still reports dead, because the stamp differs.

## Runtime checks

The `refs` crate's default `checks` feature came along as the `hilen` feature of the same
name, on by default. These are regular runtime assertions, so they run in release builds
too:

- Immutable `Weak` dereference verifies that the pointer was initialized, its allocation is still
  alive, and its stamp still matches. Immutable access is allowed from background threads.
- Mutable `Weak` dereference performs the same lifetime checks and asserts that it runs on the main
  thread.
- Mutable `Own` dereference asserts that it runs on the main thread.
- Dropping an `Own` outside the main thread always panics.

The checks catch dangling pointers and background mutation while preserving copyable `Weak`
pointers and direct method receivers. The explicitly unsafe unchecked APIs bypass these checks and
must uphold the same rules themselves.

## In views

The `#[view]` macro rewrites `#[init]` fields to `Weak<Field>`. Real ownership lives in
`ViewBase.subviews: Vec<Own<dyn View>>` — the view tree owns children, like in Swift.
Methods take `self: Weak<Self>`, so closures capture `self` by copy.

This is also why Hilen requires nightly Rust. `Weak<Self>` is a custom smart-pointer method
receiver, enabled by `arbitrary_self_types`. Replacing it with a stable receiver would change the
framework's method syntax and callback ownership model rather than being a mechanical toolchain
change.

## Managed resources

`managed!(T)` gives a type a global storage, a map from name to `Own<T>`. Assets like
`Image` and `Font` live there. A resource loads once and stays until process exit.
`free()` and `free_with_name()` exist in the refs crate, but nothing in the engine calls
them. So a long-lived `Weak` to a managed resource never dangles. Render pipelines rely
on this: `RectPipeline` keys instance batches by `Weak<Image>` and never removes entries.

Storage access is race-safe. Concurrent `download` calls for the same name share one HTTP
request: the first caller fetches, the rest wait and get the same `Weak`. If the fetch
fails, all of them get an error. In the browser a url starting with `/` is resolved
against the page origin before the request, so root relative asset urls work from any host. The sync paths `get`, `load` and `store_with_name`
insert atomically. When two threads race, the losing duplicate is dropped on the main thread.

## Rules

- Objects live and die on the main thread. Dropping `Own` on another thread panics.
- Mutation is main thread only (checked at runtime with the default `checks` feature).
- Reads from background threads are allowed. Reading heap fields (`String`, `Vec`) while the main
  thread rewrites them is a known accepted risk — rare in practice. When in doubt use `from_main`.
- Two `Weak` copies can give two `&mut` to the same object. Not tracked. Accepted by design —
  do not hold a `&mut` while triggering events that may touch the same view.
