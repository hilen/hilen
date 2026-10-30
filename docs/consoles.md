# Xbox, PS5 and Switch

What it would take to run the engine on a game console. Nothing here is built. The
research is from 2026-10-30, with no console and no console SDK at hand. What a console
SDK really allows is secret, so the parts marked as not checked may change the plan. The
month figures are guesses, for 1 person with a dev kit and with no certification, and no
run is behind them.

## The engine today

- `docs` and `hilen/src` have no gamepad code. The key focus in [focus.md](focus.md)
  already moves on arrow keys and Enter, so a gamepad can drive the UI through it.
- `winit` is in 30 engine files. The smallest change is a private `winit` backend that
  keeps its API. The Android backend of the fork is 1,749 lines, the iOS one 3,649.
- `wgpu` is in 61 engine files, which need no change when a backend exists. The surface
  is made in `window/window.rs` and `window/surface.rs`.
- The 27 WGSL shaders go through `naga`, which writes SPIR-V, HLSL, MSL and GLSL.
- `deps/plat/src/lib.rs` needs a `console` alias. Each of the about 161 `desktop` gates
  and 271 `not_wasm` gates then needs a review.
- Sound needs a new output, `kira` runs on `cpal`. Saves and the text keyboard need
  console versions too. `rfd`, the updater, Sentry and the inspector are probably off on
  a console, the rules for that are not public.

## A native game build

A console SDK comes with a secrecy contract, an NDA. `hilen/hilen` is public and the
`wgpu` and `winit` forks are published, so console code has to live in a private layer
on top of them.

- Xbox, 1 to 3 months. [ID@Xbox](https://developer.microsoft.com/games/publish/id/welcome)
  is free and takes at least 2 weeks, dev hardware comes after a game concept is
  approved. `wgpu` has DX12, but a console uses D3D12.x, `GameInput`, `XGameSave` and
  `MSIXVC` packaging, see the [porting guides](https://devdocs.xbox.com/home/build-first-title/porting-guides)
  and the [GDK readme](https://www.github.com/microsoft/GDK). Whether the normal Windows
  Rust target works is not checked.
- Switch, 3 to 6 months. The public [Rust target](https://doc.rust-lang.org/nightly/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
  has no `std` and is for homebrew, so it needs an own target and a `std` port. Vulkan on
  Switch stopped at an NDA in [wgpu issue 3032](https://github.com/gfx-rs/wgpu/issues/3032),
  with no code merged. Official Vulkan on Switch 2 and the access terms of Nintendo are
  not checked.
- PS5, 6 to 12 months. [PlayStation Partners](https://sonyinteractive.com/en/?p=3002341)
  lends 2 kits after registration. `rustc` lists no target and no public `wgpu` backend
  was found, so it needs a new `wgpu-hal` backend, and the Metal and Vulkan ones are 8,432
  and 14,154 lines. `naga` has no PSSL output. The names AGC and PSSL come from unofficial
  sources.

The order is gamepad input first, since it needs no contract and helps desktop and
tvOS, then a PC build on the public GDK with `GameInput`, then Xbox, Switch and PS5.

## An app on Xbox, like flixen

A non-game app needs no contract and no concept approval, only a developer account and a
Store submission, says the [concept approval page](https://learn.microsoft.com/en-us/windows/uwp/gaming/concept-approval)
from 2017. No public way was found for a non-game app on PS5 or Switch.

UWP, the Universal Windows Platform, is the older app format of Microsoft that runs on
Windows and Xbox in a sandbox. Microsoft no longer develops it actively.

[Jellyfin for Xbox](https://github.com/jellyfin/jellyfin-xbox) is the example, a C# UWP
shell of 47 files, GPL-2.0, that opens the web player in WebView2, the embedded Chromium
of Microsoft. An engine app can do the same with its wasm build, like it does on an LG
TV, see [webos.md](webos.md). That needs no new `winit` or `wgpu` backend.

- [A 2023 report](https://github.com/MicrosoftEdge/WebView2Feedback/discussions/4138) says
  WebView2 on Xbox has WebGL 2 and no working WebGPU. The engine falls back to WebGL by
  itself.
- Video is the biggest gap. Users report that WebView2 opens no mkv in
  [Jellyfin issue 153](https://github.com/jellyfin/jellyfin-xbox/issues/153) and plays no
  HEVC in [issue 148](https://github.com/jellyfin/jellyfin-xbox/issues/148). Those are
  user reports, not statements of Microsoft. A server that only sends the file as it is
  would have to remux or transcode, or the app would need a native player.
- The shell needs Visual Studio on Windows, and new `make` steps. Whether the D-pad
  reaches the page as arrow keys is not checked. The shell and the input are a guess of 1
  to 2 weeks.
- The [Xbox page of Partner Center](https://learn.microsoft.com/en-us/windows/uwp/apps-for-xbox/partner-center-xbox)
  asks for a business account, while [free registration](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/)
  for a person does not mention Xbox. So publishing may need a company.
- [Dev Mode](https://learn.microsoft.com/en-us/windows/uwp/xbox-apps/devkit-activation)
  installs on an own Xbox with that account, and retail apps do not run while the console
  is in it. The fee is unclear, reports say $19, $20 or $99.
- Not checked, all need a real Xbox: which files play, whether HDR works, and whether
  Microsoft still takes a new UWP app. The Store page of Jellyfin returned only its title.

The first step is to host the wasm build at a test address and open it in Edge on an
Xbox, with an mkv, an mp4 with H.264 and an mp4 with HEVC. A user in issue 148 says Edge
on Xbox plays the same formats as the webview, which is not proven.
