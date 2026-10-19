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

## App activity and screen timeout

`system::AppActivity::changed()` notifies views on the main thread when the app
loses or regains activity. Focus loss, occlusion and suspension make it inactive;
all three must recover before it becomes active again. On iOS suspension comes
from UIKit's resign-active notification, so a phone lock is reported before the
app enters the background. Subscribers can stop foreground-only work immediately.

`system::ScreenAwake::acquire()` returns a guard that prevents automatic screen
sleep on iOS and Android while the app is active. Hold it only while needed;
dropping the last guard restores normal timeout. It uses the UIKit idle timer on
iOS and Android's `KEEP_SCREEN_ON` window flag through the activity's UI-thread
queue. Inactivity releases the OS request even if a guard remains alive. Manual
locking still works, and this grants no background execution. Other platforms
keep their normal display policy. Acquire on main; dropping on another thread
queues the release on main.

## The privacy cover

`UIManager::set_privacy_cover(true)` turns on a plain cover in the clear color for the
rest of the run. While a phone app is not active it is drawn in place of every view,
modals and their scrims included. It is off by default and on desktop and in a
browser it never shows, a window that only lost the focus stays as it is. The code is
`hilen/src/ui/privacy_cover.rs`.

- iOS keeps a picture of the app for the app switcher. It takes it right after
  `applicationDidEnterBackground` returns. While the app is only inactive the card in
  the switcher is live. An app may not send work to the GPU in the background, so
  the cover cannot be drawn at that late point. It is drawn at once inside
  `applicationWillResignActive`, which winit sends as `Suspended`, see `suspended`
  in `window/app_handler.rs`. That is the first event of an app switch and of a
  lock, so the frame on the screen is the cover before the switcher shows it.
- Android goes first through the loss of the focus, while the surface is still
  there, and the cover is drawn there. `Suspended` on Android comes when the
  surface goes away.
- The cover goes away with the next frame once the app is active again. That frame
  is drawn whole, the frames under the cover drew no view.
- What sits over the frame is hidden with it: the system text field of an edited
  field on iPhone and the Core Animation layers of the loops.
- A system prompt over the app, like Face ID, makes it resign active too, so the
  cover shows behind it.
- A check on a real iPhone that the app switcher shows the cover is still open.

The test is `Privacy cover`. It forces the rule of a desktop and of a phone with
`set_test_phone`, so it checks both on every platform.

## The end of the app

`App::will_quit` runs once on the main thread right before the app ends, from
`AppHandler::exiting`: a closed window, Cmd and Q, `AppRunner::stop`, the end of a
headless run, and `applicationWillTerminate` on iOS. A hot build calls it at the
stop of a generation too.

- iOS sends `applicationWillTerminate` only to an app that runs at that time. A
  suspended app closed from the app switcher or killed by the system gets no call.
- Android reports no end, winit does not pass on `onDestroy`, so the hook never runs
  there.
- A killed process runs nothing.

Work that must be done before a phone app goes away also listens to
`AppActivity::changed`.

## Frames on demand

The winit loop draws only when a frame is requested, so a static screen with nothing moving
burns no CPU. `request_frame` in `hilen/src/window/redraw.rs` sets a redraw flag and is
safe from any thread. Window and input events call it, and so does the dispatch waker, which
`hreads` fires on every background `on_main`/`from_main` enqueue, so a queued closure never
waits on an idle loop.

Continuous work keeps the loop running by itself. While a live animation or a loaded level
exists, `continuous_render_active` is true and `about_to_wait` asks for a frame on every
iteration. Once neither exists the loop goes back to `ControlFlow::Wait` and sleeps. The choice
keys off the presence of the work, not a per-frame flag, because `about_to_wait` also runs on
iterations that draw nothing and a flag would read false there.

What brings the loop to its next iteration is the `Pacer` of the platform, in `redraw.rs`:

- `Pacer::Poll`, on desktop and Android. `about_to_wait` sets `ControlFlow::Poll`. Every
  iteration draws a frame and the draw waits for the screen, so the loop does not spin.
- `Pacer::FrameWake`, on iOS and tvOS. `about_to_wait` sets `ControlFlow::Wait` and the loop
  sleeps. After every drawn frame `frame_drawn` wakes the loop once while continuous work
  lives, the next iteration asks for the next frame, and UIKit draws it on the next screen
  refresh.

iOS must not use `Poll`. winit runs it there as a run loop timer with a period of 0.1
microseconds, `EventLoopWaker` in `src/platform_impl/ios/app_state.rs` of the fork. A redraw
on iOS is `setNeedsDisplay`, and UIKit draws once per screen refresh however often it is
asked. So a polling loop went around many times between 2 frames with nothing to do. In banda
on an iPhone 16 Pro Max the main thread took 96 percent of a core while a small mark pulsed,
and the drawing was 3.5 percent of it. With `FrameWake` the same build on the same phone,
measured on 2026-10-11 with `xcrun xctrace record` and the templates Activity Monitor and
Time Profiler: the process went from 89 to 33 percent CPU at the same 60 frames a second,
the drawing is 86 percent of the main thread, and with nothing that moves the process sits
at 0 to 1 percent. tvOS was not measured. The unit tests next to `frame_pacing` hold both pacers.
A desktop UI test cannot catch this: a headless run has no winit loop, and a desktop window
runs another winit backend, where `Poll` is right.

A covered window holds every frame. winit's occluded event fires when the window is
minimized, fully covered or on another desktop, the app handler stores it, and
`frame_pacing` in `redraw.rs` answers `Wait` with no redraw while it holds, continuous work
included. Animations keep their clock, so a long hold lands them at the end state on the
first frame back, and the event that uncovers the window requests that frame. Two things
keep working while covered: a pending screenshot still gets its one frame, drawn offscreen,
so `check_colors` and `hilen-inspect screenshot` never hang on a minimized window, and the
wake event drains the dispatch queue itself, since a frame is where queued callbacks normally
run and a background thread in `from_main` must not wait until the window shows. That
drain frees the removed views first, the way a frame does, and so does the drain of a
paused app. Else a view removed behind another window stays alive until the window
shows, and a timer of a removed page still finds its page. The UI test `Removed while
paused` covers it. The
pacing table is a pure function with unit tests next to it. A browser throttles its own
frame callbacks for a hidden tab, so wasm is not part of this.

The two platforms order the loop differently and it matters. On desktop `about_to_wait` runs
after the render, so it sees an animation added mid-frame and switches to `Poll` on its own. On
iOS and tvOS `about_to_wait` runs before the render, so it misses that animation, and a
`request_frame` made while drawing, like the one from `commit_animations`, comes too late for
the current iteration. So there `request_frame` also wakes the loop from the main thread, and
the next iteration re-checks the flag. The wake of `frame_drawn` covers an animation that
starts while a frame draws and asks for no frame itself: it reads `continuous_render_active`
after the frame. Doing a wake from the main thread on desktop livelocks the loop, so both are
gated to iOS and tvOS.

A resize draws inside its own event rather than waiting for the next frame. Reconfiguring the
surface resizes the backing buffer and clears it, and a window drag fires resizes far faster
than frames arrive, so waiting presents an empty buffer for the whole drag. In a browser that
reads as the page going black while it is dragged, since the cleared canvas shows the page
background behind it.

Headless runs render every iteration and ignore the flag. Wasm uses `ControlFlow::Wait`, and a
page asks the browser for a frame only when it needs one, so a still page draws nothing.
`about_to_wait` calls `request_redraw` only while the flag is set or continuous work runs, and
a redraw that nobody asked for returns at once. A JS callback runs outside the loop, so
`request_frame` on the main thread of a page calls `request_redraw` itself, and the dispatch
waker is `request_frame` there too. `spawn` and `block_on` wrap every task so that each poll of it wakes
the loop, since an `on_main` from a task of a page runs at once and wakes nothing. The listeners
of a socket, popstate, pointer lock and the TV back key ask for a frame the same way. A worker
of the atomics build cannot ask the browser, so that build asks on every refresh and the frame
skips itself when the flag is clear. The UI test `Idle draws no frame` checks that a still
screen draws at most 2 frames in a second, and that a timer and a task each get their frame.
It must not use `Poll`: winit implements web `Poll` with a
continuous `scheduler.postTask` chain, and Firefox starves requestAnimationFrame under that
flood, which freezes the app. A normal wasm build is single threaded. The browser test build
spawns real workers through `spawn_thread`, and they drive the main thread like native
background threads. Workers may block, the browser main thread must never block on a contended
lock, `Atomics.wait` traps there. That rule is why the hottest shared locks spin on wasm
instead of parking: the refs pointer stamp map, the managed asset storages and the hreads
dispatch queue. A full suite run used to die at Navigation rich exactly this way, a main
thread read of the stamp map parked while a worker held it for a write.

The `Animation drives frames` UI test guards the part of this that can be pinned down. It starts
an animation from code, with nothing injecting input, and checks that the loop reports continuous
work and that the animation finishes on its own. It does not reproduce the iOS stall, which only
showed up part way through a full suite run and never on its own, so the iOS lane is what catches
that one. Note that any `from_main` between starting an animation and waiting for it wakes the
loop and hides exactly this class of bug.

### A view that needs every frame

A view that moves a thing by a clock in `update` needs a frame on every screen refresh,
and nothing asks for one. `keep_frames_while(condition)` on a view does: while the
condition returns true the loop draws on every refresh, and it sleeps again when the
condition ends or the view is gone. A view has 1 condition, a new call replaces it. It
is an empty `UIAnimation` with a finish condition, so it works wherever animations do.

The condition is dropped the first time it returns false, and nothing asks it again.
So make the call at the moment the condition becomes true, like the start of a player,
and make it again at every such start. A call in `setup` with a condition that is
still false ends in the first frame and keeps no frame. Blast had that bug, its play
line was drawn about 8 times a second.
`VideoView` and a gif with a loop count use it. A covered window still draws
nothing. The UI test is `Keep frames while`.

It is for a thing that ends. A condition that holds for 10 seconds writes a warning
once per view, a thing that repeats is a declared loop, see [loops.md](loops.md),
which the system can show with no frame of the app. `Spinner`, `RingSpinner` and
a gif that loops forever are declared loops.

```rust
fn toggle_play(mut self: Weak<Self>) {
    self.player = match self.player.take() {
        Some(_) => None,
        None => Some(Player::start()),
    };
    self.keep_frames_while(move || self.player.is_some());
}
```

### The time of a frame

`time::frame_seconds()` is the time of the frame that is drawn, in the seconds of
`monotonic_seconds`. `update` runs a few ms early or late in each frame, so a thing
placed by the moment the code runs moves in uneven steps. The frame time goes on by
the refresh period of the screen and takes a tenth of each new difference to the real
time. Every read in 1 frame gives the same value. `Animation` runs on it.

- The period is measured from the frames, never read from the system: the slope of a
  line through the last 16 frame times. A browser gives the frame time itself, from
  `document.timeline.currentTime`.
- A dropped frame moves the time by the whole 2 or 3 periods.
- A gap of 0.1 seconds or more is a hold, like a covered window or a loop that slept.
  The time is then the real time at once.
- It is never more than 8 ms from the real time the frame began at, and it never goes
  back.
- Frames with no rhythm get the real time.
- In frame stepped time it moves by exactly 1 step per stepped frame.
- It is the time the frame began, not the time the screen shows it.

The code is `hilen/src/frame_time/`, the follower is `follower.rs` with unit tests on
made up frame times. The UI test `Even frame time` proves that 2 views read the same
value in a frame and that a stepped frame moves it by 1 step. A headless run has no
screen refresh, so even steps on a real display are not proven by a test, see
[roadmap.md](roadmap.md).

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
