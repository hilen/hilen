# iOS

The oldest supported device is an iPhone 5S on iOS 12.5, an A7 GPU. Everything here exists
to keep that device working. Nothing is cosmetic, and each knob was found by a crash.

## Logs

iOS drops a launched app's stdout and stderr, so `println!`, the fern chain and the default
panic hook all go nowhere. `ios_log.rs` routes three things through `NSLog`, which reaches
the device system log:

- the fern chain, so `debug!` and friends show up,
- a panic hook, or a panic leaves only `abort()` in the crash report with no message,
- an `objc_setExceptionPreprocessor`, which logs an Objective-C exception at the throw.

The last one matters more than it looks. An ObjC exception that unwinds through an
`extern "C"` frame, a UIKit callback block for instance, makes Rust abort with `panic in a
function that cannot unwind`, reporting neither the reason nor the throw site. A
`panic in a function that cannot unwind` with no `panicked at` line before it is always a
foreign exception, never a Rust panic.

Read the log with `idevicesyslog -u <udid>`, unfiltered. Its `-m` filter silently drops
matching lines.

## The deployment target

`IPHONEOS_DEPLOYMENT_TARGET` is a **compile time** value. `objc2::available!` reads it
through `option_env!` and folds to a constant `true` whenever the target already meets
the version being checked, which deletes the runtime guard. Setting it to 13.0 made
wgpu's `available!(ios = 13.0)` vanish and sent `supportsFamily:` to the A7, which has
no such selector. `make build-ios` pins 12.0. Never raise it to or above a version the
code guards against.

There used to be a look-alike second setting, a `-platform_version,ios,13.0` linker flag
in `.cargo/config.toml`. It existed only because the foundational crates, back when they
were separate crates, built intermediate dylibs whose link needed `__chkstk_darwin`, an
iOS 13 `aws-lc-sys` symbol. That code is plain modules inside `hilen` now, nothing links a
dylib during the lib build, and the flag is gone, verified with `make build-ios`.

`cargo build --lib` links nothing, so it cannot reproduce a link error that
`make build-ios` hits. Check with the Makefile target.

## Weak linked CoreGraphics

`OTHER_LDFLAGS` in the Xcode project carries `-Wl,-weak_framework,CoreGraphics`. wgpu names
`kCGColorSpaceExtendedDisplayP3` and the two `kCGColorSpaceITUR_2100` constants, all iOS
14, and a Rust `extern` static has no availability metadata, so rustc always emits a strong
reference and dyld aborts before `main` on an older device. Weak linking makes the missing
constants NULL, which is safe because the surface only ever asks for sRGB. The deployment
target cannot fix this.

## wgpu fork

`wgpu` comes from the fork published on crates.io as `hilen-wgpu`, declared with
`package = "hilen-wgpu"` in the root `Cargo.toml` and built from the `ios-edr-guard`
branch at github.com/VladasZ/wgpu. The fork guards `wantsExtendedDynamicRangeContent`, an
iOS 16 property that upstream messages with no availability check, which breaks every iOS
below 16. `wgpu_text` is the fork at github.com/VladasZ/wgpu-text, pinned by git rev, and
it builds against that same `hilen-wgpu`, because a stock `wgpu` next to it would give two
`wgpu` crates whose types do not interchange. Do not swap either back to upstream without
checking on an old device first. The EDR guard went upstream as
[gfx-rs/wgpu#10257](https://github.com/gfx-rs/wgpu/pull/10257), the rest of the fork's
state against upstream is in [forks.md](forks.md).

## A7 limits

The A7 reports missing downlevel flags: indirect execution, base vertex and cube array
texturing. Rendering works, but a UI test that leans on those paths will fail there and
pass everywhere else.

## An A7 draws nothing from a fat shader

**An A7 GPU draws nothing at all from a shader carrying more than eight float components
from the vertex stage to the fragment stage.** It builds, wgpu validates it, Metal accepts
the pipeline, and no pixels come out. No error, no warning, nothing in the log. Twelve
components in total counting `@builtin(position)`.

Measured, not guessed. Eight components draw, nine do not. Locations are irrelevant, eight
components spread over eight locations draw fine while nine over nine do not.
`@interpolate(flat)` does not help.

Nothing models this limit. wgpu counts locations against `max_inter_stage_shader_variables`,
15 on this device, and Apple's tables allow 60 components, so both call these shaders legal.
The hardware is not the problem either, OpenGL ES 2.0 guaranteed 32 components in 2007 and
an A7 is a full ES 3.0 part. naga's MSL output is correct, a nine component shader differs
from an eight component one by exactly one valid `[[user(locN), center_perspective]]` line.
It is an Apple driver bug on a deprecated chip and there is nothing to fix in the fork.

The UI shaders stay under the limit by keeping per-instance constants out of the
interpolator. Almost nothing a rect shader carries actually varies across the shape, only
`uv` does, so the rest is read from a storage buffer indexed by a flat `@builtin(instance_index)`.
`ui_rect` went from 17 components to 3 that way, `ui_image` 15 to 5, `ui_gradient` 17 to 3,
`ui_shadow` 13 to 3 and `ui_backdrop` 17 to 3, with no visual change at all and every
`check_colors` block still passing untouched.

**If a shader renders on a Mac and draws nothing on the phone, count its varyings first.**
That is the single most likely cause and it costs nothing to check.

An instance struct that a fragment stage reads through a storage buffer follows `std430`: a
`vec4` must start at a multiple of 16 and the struct size rounds up to a multiple of 16.
Rust packs a `repr(C)` struct at 4 byte alignment, so the fields are ordered vec4s first
with explicit tail padding. Reordering one silently feeds every shader that shares it
whatever landed at the offset it expected — `ui_backdrop.wgsl` shares `UIRectInstance` with
`ui_rect.wgsl` and broke exactly that way. The layout tests next to each instance struct
exist to catch it.

## The system text field

While a `TextField` is edited, a real `UITextField` sits exactly over it and takes the
keys, a `UITextView` for a multiline field. So the keyboard, autocorrect, text composing
and the selection handles are the ones of iOS. The engine draws no text and no caret for
that time, its label shows only the placeholder of an empty field. The Rust half is
`hilen/src/ui/views/basic/text_field/system_field.rs`, the native half is
`hilen/native/ios/hilen_text.m`.

- The native file is compiled into the engine library by `hilen/build.rs` with `cc`, so
  the 2 halves are always 1 version and the Xcode project of an app carries nothing for
  it. `hilen.h` of the mobile template still has the 3 old `hilen_ios_*keyboard`
  functions, only for apps on an engine from before this.
- The build script takes the compiler from `xcrun --find clang`. The first `clang` on
  the PATH can be the one of the Command Line Tools, older than the iOS SDK, and it
  fails on the modules of that SDK.
- On open the system field gets the text, the font file with its axis values, the text
  size, the color, the alignment, the insets and the secure flag, so the switch is not
  visible. `Screen keyboard look` measures that on a capture of the screen. A field with
  a number constraint opens the numbers page of the keyboard.
- Every change of the text goes to the engine at once, so `changed` and the constraints
  work per key. Return in a single line field arrives as Enter. A change of the UI scale
  opens the system field again with the new sizes.
- Closing has to take the keys from the engine view too. The winit view can be a first
  responder, it takes the keys the moment its subview lets them go, and the keyboard
  stayed up.
- The system field follows the engine field every frame, `ViewCallbacks::update`, so
  it stays over a field that scrolls.
- Apple TV has none of this, the code is behind `all(ios, not(tvos))`.

Open: password autofill and a field under the keyboard, see [roadmap.md](roadmap.md).
