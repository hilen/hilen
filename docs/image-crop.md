# Image crop, a photo cut for an avatar

`ImageCropView` shows a photo under a mask with a round or square hole, and a player
picks the part that becomes an avatar, like the crop of a profile photo on any big
site. `ImageCropPrompt` is the ready modal around it. The code is
`hilen/src/ui/views/complex/image_crop/`.

## What an app calls

```rust
let picked = Paths::pick_image("Pick a picture").await?;
let request = ImageCropRequest::new(picked.bytes, 256).title("Crop your picture");
ImageCropPrompt::show_modally_with_input(request, move |png| {
    // Some(png), a square of 256 pixels, or None for Cancel and Escape.
});
```

- `ImageCropRequest` has the bytes, the side of the png, the shape, `CropShape::Circle`
  by default or `Square`, the title and the texts of the 2 buttons.
- The prompt is a crop view, a zoom row of `−`, a `Slider` and `+`, then Cancel and
  Save. Its colors come from `DialogStyle`, like `Question`. It is 420 points wide, a
  window too small for that gets a smaller one, `fitted_width`.
- The view alone: `set_photo(bytes)`, then the event `loaded` with true, or false
  when the bytes are no photo. `set_shape`, `zoom`, `set_zoom`, `max_zoom`, `cut` in
  pixels of the upright photo, `cut_png(side)` and the event `on_change`.

## How it moves

- The state is the pixel of the photo in the middle of the hole and the zoom,
  `CropFrame` in `frame.rs`. Zoom 1 is the short side of the photo across the hole,
  the most is 5. A new size of the view keeps the same cut.
- The photo always covers the hole, every pan and zoom is clamped after it.
- A drag moves the photo. The wheel and a pinch zoom at the pointer, the pinch also
  moves with the fingers.
- A press on the view, or Enter on the focus ring, gives it the keys with
  `Focus::hold_keys`: the arrows move the photo by 10 points, `+` and `-` zoom. The
  next Enter gives them back and the ring stands on the view again. While it holds the
  keys the edge of the hole is drawn thicker.
- The mask is 1 `DrawingView`: a dark fill with the hole cut by `FillRule::EvenOdd`
  and a white line around the hole. It is black at 55 percent in both themes, it lies
  over a photo.

## The pixels

`pixels.rs` decodes the photo once, upright and at most 2048 pixels on its long side,
and cuts the png from that copy, not from the picture on screen.

- Native: the `image` crate reads the EXIF orientation with `orientation()` and turns
  the photo with `apply_orientation`. The decode runs in `spawn_blocking`.
- A browser: the wasm build links no jpeg decoder. `decode_upright` in
  `window/image/browser_decode.rs` lets the browser decode, which turns the photo as it
  reads it, and takes the size from that first bitmap, a second one makes it smaller.
  The header gives the size before the turn, so `decode` of downloaded pictures, which
  sizes by the header, would squash a turned phone photo.
- The photo is kept as an image under a name of its own and freed when the view is
  dropped. The drop takes no weak pointer of the view, the view is no longer counted
  while it is freed, and `Focus` drops a key holder that is gone by itself.

## Tests

- `cargo test -p hilen --lib image_crop`: the geometry and the decode, also a jpeg
  marked as taken with the phone upright, which comes out upright.
- `Image crop`: a drag, the stop at the edge of the photo, the wheel, a trackpad pinch,
  the arrows, `+` and `-`, the way out with Enter, the square mask and the png.
- `Image crop prompt test`: the look, `+` with the slider that follows, Save with a
  png of 64 and Escape with none.

The probes keep off the edge of the hole, the lines where the photo changes color and
the round thumb of the slider, they hold with `HILEN_MSAA=1` too.

Not tried: a turned phone photo in a real browser, a pinch of 2 real fingers.
