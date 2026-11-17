//! Where the running build was installed from, so an app can tell a
//! `TestFlight` tester from an App Store user.

#[cfg(device_ios)]
use std::sync::OnceLock;

/// Where the running build was installed from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InstallSource {
    AppStore,
    /// `TestFlight`. A build in App Review looks the same.
    TestFlight,
    /// Everything else: desktop, Android, the browser, the simulator, and a
    /// build put on a phone by Xcode or as ad hoc.
    Other,
}

/// Where the running build was installed from. Only iOS and tvOS on a real
/// device can answer `AppStore` or `TestFlight`.
pub fn install_source() -> InstallSource {
    #[cfg(device_ios)]
    {
        static SOURCE: OnceLock<InstallSource> = OnceLock::new();
        *SOURCE.get_or_init(read_bundle)
    }
    #[cfg(not(device_ios))]
    {
        InstallSource::Other
    }
}

// StoreKit 2 `AppTransaction` has only a Swift API, so the receipt file name
// is the check Rust can reach. Apple deprecated `appStoreReceiptURL` in iOS 18
// but still fills it.
#[cfg(device_ios)]
fn read_bundle() -> InstallSource {
    use log::info;
    use objc2_foundation::{NSBundle, NSString};

    let bundle = NSBundle::mainBundle();
    let receipt = bundle
        .appStoreReceiptURL()
        .and_then(|url| url.lastPathComponent())
        .map(|name| name.to_string());
    let has_profile = bundle
        .pathForResource_ofType(
            Some(&NSString::from_str("embedded")),
            Some(&NSString::from_str("mobileprovision")),
        )
        .is_some();

    let source = source_of(receipt.as_deref(), has_profile);
    info!("install source {source:?}, receipt {receipt:?}, provisioning profile {has_profile}");
    source
}

// A build from Xcode or an ad hoc build also has a sandbox receipt. Only the
// provisioning profile in its bundle tells it from TestFlight, the App Store
// and TestFlight strip that file.
#[cfg(any(test, device_ios))]
fn source_of(receipt: Option<&str>, has_profile: bool) -> InstallSource {
    if has_profile {
        return InstallSource::Other;
    }

    match receipt {
        Some("receipt") => InstallSource::AppStore,
        Some("sandboxReceipt") => InstallSource::TestFlight,
        _ => InstallSource::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{InstallSource, install_source, source_of};

    #[test]
    fn source_from_receipt_and_profile() {
        assert_eq!(source_of(Some("receipt"), false), InstallSource::AppStore);
        assert_eq!(
            source_of(Some("sandboxReceipt"), false),
            InstallSource::TestFlight
        );
        assert_eq!(source_of(Some("sandboxReceipt"), true), InstallSource::Other);
        assert_eq!(source_of(Some("receipt"), true), InstallSource::Other);
        assert_eq!(source_of(None, false), InstallSource::Other);
        assert_eq!(source_of(Some("other"), false), InstallSource::Other);
    }

    #[test]
    fn desktop_is_other() {
        assert_eq!(install_source(), InstallSource::Other);
    }
}
