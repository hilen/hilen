use image::{GenericImageView, load_from_memory};
use pollster::block_on;
use serial_test::serial;

use crate::camera::{
    Camera, CameraDevice, CameraError, CameraFrame, CameraLens, CameraLocks, CameraPosition, FakeCamera,
    PhotoFormat,
    frame::{FrameSlot, Plane, turn, yuv420_to_rgba, yuyv_to_rgba},
};

#[test]
fn yuyv_becomes_rgba() {
    // 2 gray pixels, then 2 red ones.
    let rgba = yuyv_to_rgba(&[128, 128, 128, 128, 76, 85, 76, 255], 4, 1).unwrap();
    assert_eq!(&rgba[..8], [128, 128, 128, 255].repeat(2));
    assert!(rgba[8] >= 250 && rgba[9] <= 3 && rgba[10] <= 3, "{rgba:?}");
    assert!(yuyv_to_rgba(&[1, 2, 3], 2, 1).is_none());
}

#[test]
fn a_picture_turns_clockwise() {
    // 2 by 1: a red pixel left of a blue one.
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let picture = [red, blue].concat();

    // A quarter turn puts the left pixel on top.
    let (turned, width, height) = turn(&picture, 2, 1, 1);
    assert_eq!((width, height), (1, 2));
    assert_eq!(turned, [red, blue].concat());

    let (turned, width, height) = turn(&picture, 2, 1, 2);
    assert_eq!((width, height), (2, 1));
    assert_eq!(turned, [blue, red].concat());

    // 3 quarters put the left pixel at the bottom.
    let (turned, width, height) = turn(&picture, 2, 1, 3);
    assert_eq!((width, height), (1, 2));
    assert_eq!(turned, [blue, red].concat());
}

/// A picture of 1 color.
fn plain(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    color.repeat(usize::try_from(width * height).unwrap())
}

#[test]
#[serial]
fn the_fake_camera_shows_its_picture_and_takes_a_photo() {
    FakeCamera::install();
    FakeCamera::show(plain(64, 48, [200, 40, 20, 255]), 64, 48);

    let devices = block_on(Camera::devices()).unwrap();
    assert_eq!(devices.len(), 1);
    let camera = block_on(Camera::open(&devices[0])).unwrap();

    let frame = camera.frame().unwrap();
    assert_eq!((frame.width, frame.height, frame.number), (64, 48, 1));
    assert_eq!(&frame.rgba[..4], &[200, 40, 20, 255]);

    FakeCamera::show(plain(64, 48, [10, 20, 30, 255]), 64, 48);
    assert_eq!(camera.frame().unwrap().number, 2);

    let photo = block_on(camera.take_photo(PhotoFormat::Heic)).unwrap();
    assert_eq!(photo.format, PhotoFormat::Jpeg);
    assert_eq!((photo.width, photo.height), (64, 48));
    let decoded = load_from_memory(&photo.bytes).unwrap();
    assert_eq!(decoded.dimensions(), (64, 48));
    let pixel = decoded.get_pixel(32, 24).0;
    assert!(
        pixel[0].abs_diff(10) < 6 && pixel[1].abs_diff(20) < 6 && pixel[2].abs_diff(30) < 6,
        "{pixel:?}"
    );

    let held = block_on(camera.set_locks(CameraLocks::ALL)).unwrap();
    assert_eq!(held, CameraLocks::ALL);
    assert_eq!(FakeCamera::locks(), CameraLocks::ALL);

    drop(camera);
    FakeCamera::remove();
}

#[test]
#[serial]
fn a_refused_camera_is_a_clear_error() {
    FakeCamera::install();
    FakeCamera::refuse_access(true);

    let err = block_on(Camera::devices()).unwrap_err();
    assert_eq!(err.downcast_ref::<CameraError>(), Some(&CameraError::Refused));

    FakeCamera::refuse_access(false);
    let missing = CameraDevice {
        id:       "gone".to_string(),
        name:     "Gone".to_string(),
        position: CameraPosition::Unknown,
        lens:     CameraLens::Unknown,
    };
    let err = block_on(Camera::open(&missing)).err().unwrap();
    assert_eq!(
        err.downcast_ref::<CameraError>(),
        Some(&CameraError::NotFound("Gone".to_string()))
    );
    FakeCamera::remove();
}

#[test]
fn a_big_frame_is_shrunk_for_the_screen() {
    let slot = FrameSlot::default();
    slot.put(plain(2560, 1920, [1, 2, 3, 255]), 2560, 1920);
    let frame = slot.latest().unwrap();
    assert_eq!((frame.width, frame.height), (CameraFrame::MAX_SIDE, 960));
    assert_eq!(frame.rgba.len(), 1280 * 960 * 4);
}

#[test]
fn a_frame_of_a_wrong_length_is_dropped() {
    let slot = FrameSlot::default();
    slot.put(vec![0; 10], 4, 4);
    assert!(slot.latest().is_none());
}

#[test]
fn the_photo_format_is_read_from_the_bytes() {
    assert_eq!(
        PhotoFormat::of(&[0xFF, 0xD8, 0xFF, 0xE0]),
        Some(PhotoFormat::Jpeg)
    );
    assert_eq!(
        PhotoFormat::of(b"\0\0\0\x18ftypheic\0\0\0\0"),
        Some(PhotoFormat::Heic)
    );
    assert_eq!(PhotoFormat::of(b"\x89PNG\r\n\x1a\n"), None);
}

#[test]
fn yuv_becomes_rgba() {
    // 2 by 2 pixels: 1 color sample for all 4. Gray, then pure red in BT.601
    // full range, Y 76, U 85, V 255.
    let gray = yuv420_to_rgba(
        &Plane {
            data:         &[128; 4],
            row_stride:   2,
            pixel_stride: 1,
        },
        &Plane {
            data:         &[128],
            row_stride:   1,
            pixel_stride: 1,
        },
        &Plane {
            data:         &[128],
            row_stride:   1,
            pixel_stride: 1,
        },
        2,
        2,
    )
    .unwrap();
    assert_eq!(gray, [128, 128, 128, 255].repeat(4));

    // Interleaved color planes, the way most phones give them: U and V share
    // 1 buffer with a pixel stride of 2.
    let red = yuv420_to_rgba(
        &Plane {
            data:         &[76; 4],
            row_stride:   2,
            pixel_stride: 1,
        },
        &Plane {
            data:         &[85, 255],
            row_stride:   2,
            pixel_stride: 2,
        },
        &Plane {
            data:         &[255],
            row_stride:   2,
            pixel_stride: 2,
        },
        2,
        2,
    )
    .unwrap();
    let pixel = &red[..4];
    assert!(pixel[0] >= 250 && pixel[1] <= 3 && pixel[2] <= 3, "{pixel:?}");

    let short = yuv420_to_rgba(
        &Plane {
            data:         &[1; 3],
            row_stride:   2,
            pixel_stride: 1,
        },
        &Plane {
            data:         &[1],
            row_stride:   1,
            pixel_stride: 1,
        },
        &Plane {
            data:         &[1],
            row_stride:   1,
            pixel_stride: 1,
        },
        2,
        2,
    );
    assert!(short.is_none());
}
