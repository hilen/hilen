/// The first Android version with the Bluetooth permissions of its own,
/// Android 12.
const OWN_PERMISSIONS_API: i32 = 31;

/// The runtime permissions a scan and a connect need on this Android
/// version. Before Android 12 a scan needs the fine location.
pub(super) fn android_permissions(api: i32) -> &'static [&'static str] {
    if api >= OWN_PERMISSIONS_API {
        &[
            "android.permission.BLUETOOTH_SCAN",
            "android.permission.BLUETOOTH_CONNECT",
        ]
    } else {
        &["android.permission.ACCESS_FINE_LOCATION"]
    }
}
