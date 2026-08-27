//! Window sizing over the Accessibility API, so `run` can put both
//! apps at the same window size without the user dragging corners.
//! Needs the one time Accessibility permission for the terminal.

#[cfg(not(target_os = "macos"))]
use anyhow::Result;
#[cfg(target_os = "macos")]
pub use mac::AxWindow;

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::c_void;

    use anyhow::{Result, bail, ensure};
    use objc2_core_foundation::{CFArray, CFString, CGPoint, CGSize};

    type AxElement = *const c_void;

    const AX_VALUE_TYPE_CGPOINT: u32 = 1;
    const AX_VALUE_TYPE_CGSIZE: u32 = 2;
    const AX_SUCCESS: i32 = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateApplication(pid: i32) -> AxElement;
        fn AXUIElementCopyAttributeValue(
            element: AxElement,
            attribute: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
        fn AXUIElementSetAttributeValue(
            element: AxElement,
            attribute: *const c_void,
            value: *const c_void,
        ) -> i32;
        fn AXValueCreate(kind: u32, value: *const c_void) -> *const c_void;
        fn AXValueGetValue(value: *const c_void, kind: u32, out: *mut c_void) -> bool;
        // Private but long stable, the only way to match an AX window to
        // its CGWindowID. Window managers like yabai rely on it too.
        fn _AXUIElementGetWindow(element: AxElement, out: *mut u32) -> i32;
    }

    unsafe extern "C" {
        fn CFRetain(cf: *const c_void) -> *const c_void;
        fn CFRelease(cf: *const c_void);
    }

    fn attribute(name: &str) -> objc2_core_foundation::CFRetained<CFString> {
        CFString::from_str(name)
    }

    fn attribute_ptr(name: &objc2_core_foundation::CFRetained<CFString>) -> *const c_void {
        core::ptr::from_ref::<CFString>(name).cast()
    }

    /// One resizable window of a running app.
    pub struct AxWindow {
        element: AxElement,
    }

    impl Drop for AxWindow {
        fn drop(&mut self) {
            unsafe { CFRelease(self.element) };
        }
    }

    impl AxWindow {
        /// Find the AX window of `pid` whose window server id is `id`.
        /// Falls back to the app's only window when the private id
        /// bridge is unavailable.
        pub fn find(pid: i32, id: u32) -> Result<Self> {
            ensure!(
                unsafe { AXIsProcessTrusted() },
                "the Accessibility permission is missing, grant it to this terminal in \
                 System Settings, Privacy & Security, Accessibility, then rerun",
            );
            let app = unsafe { AXUIElementCreateApplication(pid) };
            ensure!(!app.is_null(), "no accessibility element for pid {pid}");

            let windows_key = attribute("AXWindows");
            let mut value: *const c_void = core::ptr::null();
            let status =
                unsafe { AXUIElementCopyAttributeValue(app, attribute_ptr(&windows_key), &raw mut value) };
            unsafe { CFRelease(app) };
            ensure!(
                status == AX_SUCCESS && !value.is_null(),
                "reading the window list of pid {pid} failed with AXError {status}",
            );

            let list = unsafe { &*value.cast::<CFArray>() };
            let mut fallback: Option<AxElement> = None;
            for index in 0..list.count() {
                let element = unsafe { list.value_at_index(index) };
                if fallback.is_none() {
                    fallback = Some(element);
                }
                let mut window_id = 0_u32;
                let bridged = unsafe { _AXUIElementGetWindow(element, &raw mut window_id) };
                if bridged == AX_SUCCESS && window_id == id {
                    let element = unsafe { CFRetain(element) };
                    unsafe { CFRelease(value) };
                    return Ok(Self { element });
                }
            }
            if list.count() == 1
                && let Some(element) = fallback
            {
                let element = unsafe { CFRetain(element) };
                unsafe { CFRelease(value) };
                return Ok(Self { element });
            }
            unsafe { CFRelease(value) };
            bail!("window {id} of pid {pid} has no accessibility element");
        }

        /// The window frame size in points, title bar included.
        pub fn size(&self) -> Result<(f64, f64)> {
            let size_key = attribute("AXSize");
            let mut value: *const c_void = core::ptr::null();
            let status = unsafe {
                AXUIElementCopyAttributeValue(self.element, attribute_ptr(&size_key), &raw mut value)
            };
            ensure!(
                status == AX_SUCCESS && !value.is_null(),
                "reading the window size failed with AXError {status}",
            );
            let mut size = CGSize::new(0.0, 0.0);
            let ok =
                unsafe { AXValueGetValue(value, AX_VALUE_TYPE_CGSIZE, (&raw mut size).cast::<c_void>()) };
            unsafe { CFRelease(value) };
            ensure!(ok, "the window size attribute is not a CGSize");
            Ok((size.width, size.height))
        }

        /// Resize the window frame to `width` x `height` points. macOS
        /// clamps the frame to the screen, so a resize that would push
        /// past an edge needs the window moved first, see `set_position`.
        pub fn set_size(&self, width: f64, height: f64) -> Result<()> {
            let size = CGSize::new(width, height);
            let value = unsafe { AXValueCreate(AX_VALUE_TYPE_CGSIZE, (&raw const size).cast::<c_void>()) };
            ensure!(!value.is_null(), "creating the AXValue failed");
            let size_key = attribute("AXSize");
            let status =
                unsafe { AXUIElementSetAttributeValue(self.element, attribute_ptr(&size_key), value) };
            unsafe { CFRelease(value) };
            ensure!(
                status == AX_SUCCESS,
                "resizing failed with AXError {status}, the app may refuse this size",
            );
            Ok(())
        }

        /// The window origin in screen points.
        pub fn position(&self) -> Result<(f64, f64)> {
            let position_key = attribute("AXPosition");
            let mut value: *const c_void = core::ptr::null();
            let status = unsafe {
                AXUIElementCopyAttributeValue(self.element, attribute_ptr(&position_key), &raw mut value)
            };
            ensure!(
                status == AX_SUCCESS && !value.is_null(),
                "reading the window position failed with AXError {status}",
            );
            let mut point = CGPoint::new(0.0, 0.0);
            let ok =
                unsafe { AXValueGetValue(value, AX_VALUE_TYPE_CGPOINT, (&raw mut point).cast::<c_void>()) };
            unsafe { CFRelease(value) };
            ensure!(ok, "the window position attribute is not a CGPoint");
            Ok((point.x, point.y))
        }

        /// Move the window origin to `x`, `y` in screen points.
        pub fn set_position(&self, x: f64, y: f64) -> Result<()> {
            let point = CGPoint::new(x, y);
            let value = unsafe { AXValueCreate(AX_VALUE_TYPE_CGPOINT, (&raw const point).cast::<c_void>()) };
            ensure!(!value.is_null(), "creating the AXValue failed");
            let position_key = attribute("AXPosition");
            let status =
                unsafe { AXUIElementSetAttributeValue(self.element, attribute_ptr(&position_key), value) };
            unsafe { CFRelease(value) };
            ensure!(status == AX_SUCCESS, "moving failed with AXError {status}");
            Ok(())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct AxWindow;

#[cfg(not(target_os = "macos"))]
impl AxWindow {
    pub fn find(pid: i32, id: u32) -> Result<Self> {
        let _unused = (pid, id);
        anyhow::bail!("window resizing is only implemented on macos");
    }

    pub fn size(&self) -> Result<(f64, f64)> {
        anyhow::bail!("window resizing is only implemented on macos");
    }

    pub fn set_size(&self, width: f64, height: f64) -> Result<()> {
        let _unused = (width, height);
        anyhow::bail!("window resizing is only implemented on macos");
    }

    pub fn position(&self) -> Result<(f64, f64)> {
        anyhow::bail!("window resizing is only implemented on macos");
    }

    pub fn set_position(&self, x: f64, y: f64) -> Result<()> {
        let _unused = (x, y);
        anyhow::bail!("window resizing is only implemented on macos");
    }
}
