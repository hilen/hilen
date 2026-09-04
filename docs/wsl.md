# Running on Windows through WSL

Any hilen app runs on Windows without a Windows build. Build the normal
Linux binary inside WSL2 and the window shows up on the Windows desktop
through WSLg. WSLg is the GUI layer built into WSL2 on Windows 11 and updated
Windows 10. It forwards Linux windows to Windows, with a taskbar entry,
alt-tab and clipboard sharing. Nothing to install or enable.

## Dependencies

`make setup` from the app or engine root installs the system packages and
Rust. It covers apt on Debian and Ubuntu, dnf on Fedora and pacman on Arch,
probes for what is already there and installs only the rest, so it asks for
the sudo password once. On another distro it prints the list and stops.

The packages are a C and C++ toolchain, `git`, `cmake`, `pkg-config`, the
development headers of OpenSSL, X11, xcb and ALSA, the `libxkbcommon-x11`
runtime library, the Mesa Vulkan drivers, the tools that build the GPU
driver below and `zenity`. `zenity` draws the file and folder pickers. On a
desktop Linux those go through the XDG portal service, WSL has none, so
`rfd` falls back to `zenity`. Rust comes from [rustup](https://rustup.rs)
when `cargo` is missing.

## The GPU driver

The distro Mesa ships no Vulkan driver for the Windows GPU. Its only Vulkan
device in WSL is lavapipe, the CPU rasterizer, so an app renders every frame
in software: sixteen `llvmpipe-N` threads at forty percent each, about seven
cores, on a 2800 row git client doing nothing special.

Mesa's Direct3D 12 driver, `dzn`, reaches the GPU through `/dev/dxg` and the
`libd3d12.so` that WSL mounts under `/usr/lib/wsl/lib`. Ubuntu builds Mesa
without it, so `make setup` builds it from the Mesa source on WSL,
`build/dzn.sh` in the build repo. It installs into `~/.local/lib/hilen/dzn`
and drops the driver manifest into `~/.local/share/vulkan/icd.d`, where the
Vulkan loader picks it up next to the system drivers. No sudo beyond the
packages, nothing under `/usr` changes.

Two things make the engine use it:

- `dzn` reports Vulkan conformance version 0 and wgpu hides such adapters.
  `Window::instance` sets `ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER` when
  `WSL_DISTRO_NAME` is set. With both drivers installed wgpu then ranks the
  integrated GPU above the CPU device and picks `dzn` on its own.
- Rendering through `dzn` cut the main thread of the same session from 28 s
  to 2 s per 100 s and left the `llvmpipe` threads idle.

The log line is `Backend: vulkan` either way. To tell them apart look at the
threads of the running process, `top -H`: busy `llvmpipe-N` threads mean
software. `vulkaninfo --summary` lists `Microsoft Direct3D12 (<GPU name>)`
first when the driver works.

GL is no way out. WSLg's X server offers EGL no DRI3, so GL through X11 is
software too, and the UI pipelines fail validation on the GL backend anyway.

## What the engine does on WSL

At startup, when `WSL_DISTRO_NAME` is set, `window/wsl.rs` adjusts two
environment variables before winit starts:

- It drops `WAYLAND_DISPLAY`, so winit uses X11 through Xwayland. The
  Wayland path renders, but WSLg's shell places a new Wayland window at a
  random spot on the monitor it takes for primary, offers no way for a client
  to position it, and with mixed DPI monitors the mapping to Windows
  coordinates can put winit's own title bar above the screen edge. It also
  reports scale 1 to Wayland clients with no override. X11 honors the saved
  window position and takes the scale below. The cost is the small title bar
  Weston's X window manager draws.
- It sets `WINIT_X11_SCALE_FACTOR` from `/mnt/wslg/weston.log`. WSLg reports
  scale 1 to Linux clients for every monitor, whatever Windows is set to, so
  a window would come out at one physical pixel per logical pixel. The log
  carries the value the Windows side reports for each monitor, for example
  `rdpMonitor[1]: desktopScaleFactor:150`, and the engine uses the monitor
  whose geometry line says `is_primary:1`, so the window has the same size
  as a native Windows build. The primary is not always monitor 0, Windows
  numbers them by position. Weston prints the layout it computed in the
  same shape right after, with `desktopScaleFactor:0` on every monitor,
  and winit refuses a zero scale, so a zero is skipped. A
  `WINIT_X11_SCALE_FACTOR` set by the user wins. One scale applies to every
  monitor, a window moved to a monitor with a different scale keeps it.

## Where a fresh window opens

WSLg sets no X11 primary monitor, so winit reports a zero sized nameless
dummy as the primary. Left to itself the engine would center a new window
on that dummy and it would land off screen, often on a secondary monitor
with the title bar above the top edge, so it cannot be dragged. When the
reported primary has no size, `window/placement.rs` picks the largest
attached monitor instead. It also shrinks a new window to fit that
monitor and centers it, so a window taller than the display never opens
with its title bar off screen. A saved placement still comes back as is.

## Resizing the window

On X11 winit answers `inner_size` with a live round trip to the X server,
so two size queries in one frame can disagree while the window is being
dragged. WSLg's remote display makes that gap wide. The engine records
the size from each `Resized` event and every query in a frame reads that
one value, see `Screen::Windowed` in `window/screen.rs`, so the surface,
the render attachments and the scissor rects always agree. Without it a
drag crashes with a wgpu validation error about attachments of differing
sizes.

## If no window appears

Check that WSLg shows Linux windows at all with `xmessage hi` inside WSL.
A small box should appear on the Windows desktop.

X11 needs the `libxkbcommon-x11` runtime library. Without it the app panics
with "Library libxkbcommon-x11.so could not be loaded". `make setup`
installs it.

A blank window or a crash right after `Backend: vulkan` points at the GPU
driver. Check that `/dev/dxg` exists in WSL, that the Mesa Vulkan drivers
are installed, and that `~/.local/share/vulkan/icd.d/dzn_icd.x86_64.json`
is there, `make setup` builds it. Without that file the app still runs, on
lavapipe, slowly.
