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
development headers of OpenSSL, X11 and ALSA, the `libxkbcommon-x11` runtime
library, the Mesa Vulkan drivers and `zenity`. The Mesa package includes the
driver that maps Vulkan onto the Windows GPU under WSLg. `zenity` draws the
file and folder pickers. On a desktop Linux those go through the XDG portal
service, WSL has none, so `rfd` falls back to `zenity`. Rust comes from
[rustup](https://rustup.rs) when `cargo` is missing.

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
  `rdpMonitor[0]: desktopScaleFactor:150`, and the engine uses the first
  monitor's, so the window has the same size as a native Windows build. A
  `WINIT_X11_SCALE_FACTOR` set by the user wins. One scale applies to every
  monitor, a window moved to a monitor with a different scale keeps it.

## If no window appears

Check that WSLg shows Linux windows at all with `xmessage hi` inside WSL.
A small box should appear on the Windows desktop.

X11 needs the `libxkbcommon-x11` runtime library. Without it the app panics
with "Library libxkbcommon-x11.so could not be loaded". `make setup`
installs it.

WSLg presents the GPU through a Vulkan translation driver. If wgpu fails to
find it the app may crash or show a blank window. Check that `/dev/dxg`
exists in WSL and that the Mesa Vulkan drivers are installed.
