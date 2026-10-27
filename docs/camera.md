# Camera

The `camera` feature, off by default, gives `hilen::camera`: the cameras of the
device, a live picture in a view and a photo at full resolution from code. Found by
turnscan, a turntable scanner that takes dozens of photos in a row with nobody at the
screen.

```rust
spawn(async move {
    let devices = Camera::devices().await?;
    let camera = Camera::open(&devices[0]).await?;
    let preview = camera.clone();
    on_main(move || { view.preview.set_camera(Some(preview)); });
    camera.set_locks(CameraLocks::ALL).await?;
    let photo = camera.take_photo(PhotoFormat::Heic).await?;
    anyhow::Ok(())
});
```

- `Camera::devices()` lists the cameras with an `id`, a `name`, a `position`, Back,
  Front, External or Unknown, and a `lens`, Wide, UltraWide, Telephoto or Unknown. A
  phone lists each lens as its own camera, so an app picks the lens by the camera.
- `Camera::open(&device)` opens one and starts its live picture. `devices` and `open`
  ask for the permission themselves, `Camera::request_access()` asks at a moment the
  app picks.
- `take_photo(PhotoFormat::Jpeg | Heic)` gives a `Photo` with the bytes, the format,
  the width and the height. HEIC falls back to JPEG where the camera has none.
- `set_locks(CameraLocks::ALL | NONE)` locks or frees focus, exposure and white
  balance, so a series of photos looks the same. It answers with the locks the
  camera really holds.
- `frame()` is the newest live picture, at most 1280 on its longest side.
  `CameraView` shows it, `set_camera(Some(camera))`.
- A refusal is `CameraError::Refused`, a missing usage text `CameraError::NoUsageText`.
- `Camera` is `Clone` and `Send`, it closes with its last clone. `FakeCamera`, with the
  `ui-tests` feature, is a camera with no hardware for tests.

The usage text is in `hilen.toml`, the build writes it where each system wants it:

```toml
[camera]
usage = "Takes photos of the object on the turntable."
```

## Per platform

- macOS and iOS: AVFoundation through `objc2-av-foundation`, the full size photo on
  macOS 13 and iOS 16 and later, HEVC where the camera has it, all 3 locks. With no
  usage text the engine gives an error, since iOS and a packaged Mac app would stop
  the app. The live picture on an iPhone is turned upright for a phone held up.
  When the camera opens on iOS, the log says whether it straightens its pictures by
  itself, the geometric distortion correction of AVFoundation, which Apple turns on by
  default where a camera has it. Whether the 1x lens of an iPhone has it is not checked
  yet. A hot build waits at its stop until the delegates of the camera are gone, see
  "The stop" in [hot-reload.md](hot-reload.md).
- Android: the NDK camera, JPEG at the largest size. Focus locks with 1 auto focus
  trigger, exposure and white balance have their own lock switches. The permission is
  asked through the activity, the answer is read when the app gets its focus back,
  and no question within 2 seconds counts as a refusal.
- Windows: a webcam through nokhwa, the next picture of the stream at its highest
  resolution. Locks do not work there, nokhwa cannot switch a setting from automatic,
  a warning says so and no lock is held.
- Linux: a webcam through `linuxvideo`, pure Rust. Locks use the UVC auto switches.
- Browser: `getUserMedia` into a hidden video element, the photo through a canvas as
  JPEG. Locks go through the track constraints and are read back from its settings.
- tvOS: no camera, every call answers `NoCamera`.

The build writes `NSCameraUsageDescription` on iOS, the text and the camera
entitlement on a Mac release, and the `CAMERA` permission on Android.

## Proven

Unit tests of the fake camera, the photo, the locks, the refusal, the frame shrink,
the formats and the YUV colors, and the UI test `Camera view test` on Linux, macOS and
the iOS simulator. A real camera on iOS: turnscan on an iPhone 16 Pro Max with iOS
27.0.1 on 2026-10-17 listed the cameras, opened the 1x lens on the back, `Back
Camera`, showed its live picture upright in a `CameraView`, locked exposure and white
balance, and took over 100 JPEG photos of 8064 x 6048 from code, about 1 every 3
seconds with the work of the app between them. A hot loader installed before the
camera came had no `NSCameraUsageDescription`, `make device args=install` fixed it.
On 2026-10-27 turnscan swapped 2 times in a row on that iPhone with the camera open,
each swap about 2.3 seconds.
The hardware code of macOS, Android, Windows, Linux and the browser is not proven
with a real camera. The browser lane leaves the test out: the page stopped
answering in it after the camera opened, about 1 run in 3 on 2026-10-17, also on
`dev` with no other change. The cause is not found.
