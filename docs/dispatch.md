# Main thread and dispatch

From the [hreads](https://github.com/VladasZ/hreads) crate. Same model as UIKit: one main thread
owns all UI state, background threads send work to it.

## Main thread

The engine calls `set_current_thread_as_main()` first thing at startup, on every platform.
After that `is_main_thread()` answers in two memory reads (thread-local id + atomic load).

This is strict: if nobody set the main thread, any check panics with
"Main thread is not set". There is no guessing.

All `Own`/`Weak` runtime checks and `MainLock` globals are built on top of this.

`UIEvent::trigger()` also asserts the main thread before invoking subscribers. Its stored `Weak`
pointer keeps dead subscribers from being called; it does not provide thread dispatch. Background
work must use `on_main` or `from_main` before triggering a UI event.

## Sending work to main

- `on_main(action)` — queue a closure. On the main thread it runs immediately, from any other
  thread it runs on the next frame. The engine drains the queue once per frame in
  `AppRunner::update()` via `invoke_dispatched()`.
- `from_main(action)` — same, but blocks the calling thread and returns the result.
  On a multithread tokio worker it uses `block_in_place`, so a blocked worker hands its queued
  tasks to other workers and does not starve the runtime.
- `after(delay, action)` — run a closure on main after a delay.
- `wait_async(future)` — run a future on tokio and block until it finishes.
  Panics when called on the main thread: the future may need `from_main`, which needs the frame
  loop, which the blocked main thread cannot run. That is a guaranteed deadlock.

## Frames on demand

The winit loop draws only when a frame is requested, so a static screen with nothing moving
burns no CPU. `request_frame` in `test-engine/src/window/redraw.rs` sets a redraw flag and is
safe from any thread. Window and input events call it, and so does the dispatch waker, which
`hreads` fires on every background `on_main`/`from_main` enqueue, so a queued closure never
waits on an idle loop.

Continuous work keeps the loop running by itself. While a live animation or a loaded level
exists, `continuous_render_active` is true and `about_to_wait` sets `ControlFlow::Poll`, so the
loop iterates and each requested frame is delivered. Once neither exists it goes back to
`ControlFlow::Wait` and sleeps. The choice keys off the presence of the work, not a per-frame
flag, because under `Poll` `about_to_wait` also runs on the empty iterations between draws and a
flag would read false there.

The two platforms order the loop differently and it matters. On desktop `about_to_wait` runs
after the render, so it sees an animation added mid-frame and switches to `Poll` on its own. On
iOS `about_to_wait` runs before the render, so it misses that animation, and a `request_frame`
made while drawing, like the one from `commit_animations`, comes too late for the current
iteration. So on iOS only, `request_frame` also wakes the loop from the main thread, and the next
iteration re-checks the flag and keeps drawing. Doing that same wake on desktop livelocks the
loop, so it is gated to iOS.

Headless runs render every iteration and ignore the flag. Wasm is browser driven, the loop
polls every iteration and needs no waking. A normal wasm build is single threaded. The
browser test build spawns real workers through `spawn_thread`, and they drive the main
thread like native background threads. Workers may block, the browser main thread must
never block on a contended lock, `Atomics.wait` traps there.

The `Animation drives frames` UI test guards the part of this that can be pinned down. It starts
an animation from code, with nothing injecting input, and checks that the loop reports continuous
work and that the animation finishes on its own. It does not reproduce the iOS stall, which only
showed up part way through a full suite run and never on its own, so the iOS lane is what catches
that one. Note that any `from_main` between starting an animation and waiting for it wakes the
loop and hides exactly this class of bug.

### Known issue: windowed screenshots starve

A screenshot, the path behind `check_colors` in UI tests, waits for one rendered frame driven by
a single `request_frame`. On the desktop windowed loop that frame is starved when the window is
not focused, most likely by macOS App Nap throttling the wake, so each screenshot can take a
second or more and a windowed suite crawls. Headless is unaffected since it renders every
iteration, and the iOS simulator is unaffected since its display link keeps frames flowing. This
arrived with render on demand and is not yet fixed. It costs only the speed of a windowed run,
not correctness.

## Rules

- Never block the main thread waiting for background work.
- `from_main` from a hot background loop is fine occasionally, not thousands of times per frame —
  every call waits up to one frame.
- Queued callbacks run at one defined point of the frame, never in the middle of layout or draw.
