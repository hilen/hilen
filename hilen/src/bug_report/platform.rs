//! The os and device contexts and the release of a native Sentry event.
//! `sentry-contexts` reads uname everywhere but macOS and Windows, so an
//! iPhone says `Darwin` and an Android phone says `Linux`, and it knows a
//! device model only on macOS. The engine reads each platform itself once
//! at start and puts its contexts over those in `before_send`, which runs
//! after every integration.

use std::{fmt, sync::Arc};

use sentry::{
    ClientOptions,
    protocol::{Context, DeviceContext, Event, OsContext},
};

/// What a Sentry event says about the system and the device it came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Platform {
    /// Like `iOS`, `macOS`, `Android`, `Windows` or the name of a Linux
    /// distribution.
    pub os:           String,
    pub version:      Option<String>,
    /// The build of the system, like `23B85` on Apple systems.
    pub build:        Option<String>,
    pub kernel:       Option<String>,
    /// Like `iPhone17,1`, `Mac14,2` or `Pixel 8`.
    pub model:        Option<String>,
    /// The board of an iPhone, like `D47AP`, or the device name of Android.
    pub model_id:     Option<String>,
    /// Like `iPhone`, `iPad`, `Mac` or `Pixel`.
    pub family:       Option<String>,
    pub manufacturer: Option<String>,
    pub simulator:    bool,
}

impl Platform {
    /// Reads the system this app runs on. It never fails, a value the
    /// system does not give stays empty.
    pub(crate) fn detect() -> Self {
        #[cfg(apple)]
        {
            apple::detect()
        }
        #[cfg(android)]
        {
            android::detect()
        }
        #[cfg(linux)]
        {
            linux::detect()
        }
        #[cfg(win)]
        {
            windows::detect()
        }
    }

    /// Puts the os context in place of the one of `sentry-contexts`, and
    /// fills the device context it made, which keeps its `arch`.
    fn apply(&self, event: &mut Event<'static>) {
        event.contexts.insert(
            "os".to_string(),
            OsContext {
                name: Some(self.os.clone()),
                version: self.version.clone(),
                build: self.build.clone(),
                kernel_version: self.kernel.clone(),
                ..Default::default()
            }
            .into(),
        );

        let mut device = match event.contexts.remove("device") {
            Some(Context::Device(device)) => *device,
            _ => DeviceContext {
                arch: Some(std::env::consts::ARCH.to_string()),
                ..Default::default()
            },
        };
        if self.model.is_some() {
            device.model.clone_from(&self.model);
        }
        if self.model_id.is_some() {
            device.model_id.clone_from(&self.model_id);
        }
        if self.family.is_some() {
            device.family.clone_from(&self.family);
        }
        if let Some(manufacturer) = &self.manufacturer {
            device.other.insert("manufacturer".to_string(), manufacturer.clone().into());
        }
        if self.simulator {
            device.simulator = Some(true);
        }
        event.contexts.insert("device".to_string(), device.into());
    }
}

/// For the log line at start, like `iOS 26.1, iPhone17,1`.
impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.os)?;
        if let Some(version) = &self.version {
            write!(f, " {version}")?;
        }
        if let Some(model) = &self.model {
            write!(f, ", {model}")?;
        }
        if self.simulator {
            write!(f, ", simulator")?;
        }
        Ok(())
    }
}

/// The options of the Sentry client of an app, with the release of the app
/// and the platform on every event.
pub(crate) fn sentry_options(release: String, platform: Platform) -> ClientOptions {
    let mut options = ClientOptions::default();
    options.release = Some(release.into());
    // Apps opt into Sentry by returning a DSN. Include user context, such
    // as IPs and HTTP headers, for richer diagnostics.
    options.send_default_pii = true;
    options.before_send = Some(Arc::new(move |mut event| {
        platform.apply(&mut event);
        Some(event)
    }));
    options
}

/// `26.1` for 26.1.0 and `26.1.2` for 26.1.2, the way the system shows it.
#[cfg_attr(not(apple), allow(dead_code))]
fn version_text(major: isize, minor: isize, patch: isize) -> String {
    if patch == 0 {
        format!("{major}.{minor}")
    } else {
        format!("{major}.{minor}.{patch}")
    }
}

/// The letters a model id starts with, `iPhone` for `iPhone17,1` and
/// `MacBookPro` for `MacBookPro18,3`.
#[cfg_attr(not(apple), allow(dead_code))]
fn apple_family(model: &str) -> Option<String> {
    let family: String = model.chars().take_while(char::is_ascii_alphabetic).collect();
    (!family.is_empty()).then_some(family)
}

/// The first word of an Android model, `Pixel` for `Pixel 8`, the family
/// the Sentry Android SDK reports.
#[cfg_attr(not(android), allow(dead_code))]
fn first_word(model: &str) -> Option<String> {
    model.split_whitespace().next().map(str::to_string)
}

/// `NAME` and `VERSION_ID` of an os-release file, with their quotes
/// taken off.
#[cfg_attr(not(linux), allow(dead_code))]
fn parse_os_release(text: &str) -> (Option<String>, Option<String>) {
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let value = line.trim().strip_prefix(key)?.strip_prefix('=')?;
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            (!value.is_empty()).then(|| value.to_string())
        })
    };
    (value("NAME"), value("VERSION_ID"))
}

/// The firmware of a self built PC keeps a filler text where the model of a
/// computer goes, `None` for those.
#[cfg_attr(not(any(linux, win)), allow(dead_code))]
fn real_model(text: Option<String>) -> Option<String> {
    const FILLERS: [&str; 6] = [
        "System Product Name",
        "System manufacturer",
        "To be filled by O.E.M.",
        "Default string",
        "Not Applicable",
        "None",
    ];
    text.filter(|text| !FILLERS.iter().any(|filler| text.eq_ignore_ascii_case(filler)))
}

/// A text file of the system with its line end taken off, `None` when it
/// is missing or empty.
#[cfg(any(linux, android))]
fn read_line(path: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(apple)]
mod apple {
    use std::{ffi::CString, ptr};

    use log::warn;
    use objc2_foundation::NSProcessInfo;

    use super::{Platform, apple_family, version_text};

    #[cfg(macos)]
    const OS: &str = "macOS";
    #[cfg(tvos)]
    const OS: &str = "tvOS";
    #[cfg(all(ios, not(tvos)))]
    const OS: &str = "iOS";

    pub(super) fn detect() -> Platform {
        // The process info gives the simulated system in a simulator, a
        // sysctl there reads the kernel of the Mac under it.
        let version = NSProcessInfo::processInfo().operatingSystemVersion();
        let version = version_text(version.majorVersion, version.minorVersion, version.patchVersion);

        let simulated = std::env::var("SIMULATOR_MODEL_IDENTIFIER").ok();
        let simulator = simulated.is_some();

        // On a phone `hw.machine` is the model, like iPhone17,1, and
        // `hw.model` the board, like D47AP. On a Mac `hw.model` is the
        // model, like Mac14,2, and `hw.machine` the cpu, like arm64.
        let (model, model_id) = if cfg!(macos) {
            (sysctl("hw.model"), None)
        } else if simulated.is_some() {
            (simulated, None)
        } else {
            (sysctl("hw.machine"), sysctl("hw.model"))
        };

        Platform {
            os: OS.to_string(),
            version: Some(version),
            build: if simulator { None } else { sysctl("kern.osversion") },
            kernel: None,
            family: model.as_deref().and_then(apple_family),
            model,
            model_id,
            manufacturer: Some("Apple".to_string()),
            simulator,
        }
    }

    fn sysctl(name: &str) -> Option<String> {
        let c_name = CString::new(name).ok()?;
        let mut size = 0;
        // SAFETY: a null buffer asks only for the size of the value.
        let result = unsafe {
            libc::sysctlbyname(
                c_name.as_ptr(),
                ptr::null_mut(),
                &raw mut size,
                ptr::null_mut(),
                0,
            )
        };
        if result != 0 || size == 0 {
            warn!(
                "sysctl {name} gave no size, error {}",
                std::io::Error::last_os_error()
            );
            return None;
        }

        let mut buffer = vec![0_u8; size];
        // SAFETY: the buffer holds the `size` bytes the first call asked for.
        let result = unsafe {
            libc::sysctlbyname(
                c_name.as_ptr(),
                buffer.as_mut_ptr().cast(),
                &raw mut size,
                ptr::null_mut(),
                0,
            )
        };
        if result != 0 {
            warn!("sysctl {name} failed, error {}", std::io::Error::last_os_error());
            return None;
        }

        buffer.truncate(size);
        let text: String = String::from_utf8_lossy(&buffer).trim_end_matches('\0').trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}

#[cfg(android)]
mod android {
    use std::ffi::{CStr, CString};

    use super::{Platform, first_word, read_line};

    pub(super) fn detect() -> Platform {
        let model = property("ro.product.model");
        let emulator = property("ro.boot.qemu").or_else(|| property("ro.kernel.qemu"));

        Platform {
            os: "Android".to_string(),
            version: property("ro.build.version.release"),
            build: property("ro.build.id"),
            kernel: read_line("/proc/sys/kernel/osrelease"),
            family: model.as_deref().and_then(first_word),
            model,
            model_id: property("ro.product.device"),
            manufacturer: property("ro.product.manufacturer"),
            simulator: emulator.as_deref() == Some("1"),
        }
    }

    /// `PROP_VALUE_MAX` of `sys/system_properties.h`, the longest value
    /// with its end zero.
    const VALUE_MAX: usize = 92;

    fn property(name: &str) -> Option<String> {
        let c_name = CString::new(name).ok()?;
        let mut value = [0 as libc::c_char; VALUE_MAX];
        // SAFETY: the buffer has the VALUE_MAX bytes the call may write.
        let len = unsafe { libc::__system_property_get(c_name.as_ptr(), value.as_mut_ptr()) };
        if len <= 0 {
            return None;
        }
        // SAFETY: a value that was found ends with a zero inside the buffer.
        let text = unsafe { CStr::from_ptr(value.as_ptr()) }.to_string_lossy().trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}

#[cfg(linux)]
mod linux {
    use log::warn;

    use super::{Platform, parse_os_release, read_line, real_model};

    pub(super) fn detect() -> Platform {
        let kernel = read_line("/proc/sys/kernel/osrelease");

        let release = std::fs::read_to_string("/etc/os-release")
            .or_else(|_| std::fs::read_to_string("/usr/lib/os-release"));
        let (name, version) = match release {
            Ok(text) => parse_os_release(&text),
            Err(err) => {
                warn!("No os-release file, the Sentry os is plain Linux: {err}");
                (None, None)
            }
        };

        Platform {
            os: name.unwrap_or_else(|| "Linux".to_string()),
            version: version.or_else(|| kernel.clone()),
            build: None,
            kernel,
            // A self built PC names its board where a laptop names its model.
            model: real_model(read_line("/sys/class/dmi/id/product_name"))
                .or_else(|| real_model(read_line("/sys/class/dmi/id/board_name"))),
            model_id: None,
            family: None,
            manufacturer: real_model(read_line("/sys/class/dmi/id/sys_vendor"))
                .or_else(|| real_model(read_line("/sys/class/dmi/id/board_vendor"))),
            simulator: false,
        }
    }
}

#[cfg(win)]
mod windows {
    use std::ptr;

    use log::warn;
    use os_info::Version;
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};

    use super::{Platform, real_model};

    pub(super) fn detect() -> Platform {
        let info = os_info::get();
        let version = match info.version() {
            Version::Unknown => None,
            version => Some(version.to_string()),
        };

        Platform {
            os: "Windows".to_string(),
            version,
            // Like `Windows 11 Pro`, the version alone says 10 for both.
            build: info.edition().map(str::to_string),
            kernel: None,
            // A self built PC names its board where a laptop names its model.
            model: real_model(bios("SystemProductName")).or_else(|| real_model(bios("BaseBoardProduct"))),
            model_id: None,
            family: None,
            manufacturer: real_model(bios("SystemManufacturer"))
                .or_else(|| real_model(bios("BaseBoardManufacturer"))),
            simulator: false,
        }
    }

    /// A text value of the BIOS key, where Windows keeps the model and the
    /// maker of the computer.
    fn bios(value: &str) -> Option<String> {
        let key = wide("HARDWARE\\DESCRIPTION\\System\\BIOS");
        let name = wide(value);
        let mut buffer = [0_u16; 256];
        let mut size = u32::try_from(size_of_val(&buffer)).ok()?;

        // SAFETY: both names end with a zero, and `size` is the size of the
        // buffer in bytes.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &raw mut size,
            )
        };
        if status != 0 {
            warn!("No {value} in the BIOS key of the registry, error {status}");
            return None;
        }

        let len = (size as usize / 2).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..len]);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use parking_lot::Mutex;
    use sentry::{
        Client, Envelope, Hub, Scope, Transport,
        integrations::contexts::ContextIntegration,
        protocol::{Context, EnvelopeItem, Event},
    };

    use super::{
        Platform, apple_family, first_word, parse_os_release, real_model, sentry_options, version_text,
    };
    use crate::bug_report::release;

    #[derive(Default)]
    struct Captured(Mutex<Vec<Envelope>>);

    impl Transport for Captured {
        fn send_envelope(&self, envelope: Envelope) {
            self.0.lock().push(envelope);
        }
    }

    fn iphone() -> Platform {
        Platform {
            os:           "iOS".to_string(),
            version:      Some("26.1".to_string()),
            build:        Some("23B85".to_string()),
            kernel:       None,
            model:        Some("iPhone17,1".to_string()),
            model_id:     Some("D47AP".to_string()),
            family:       Some("iPhone".to_string()),
            manufacturer: Some("Apple".to_string()),
            simulator:    false,
        }
    }

    /// The one event a message makes on a client with the options of the
    /// engine and the contexts integration that `sentry::init` adds.
    fn captured_event(platform: Platform) -> Result<Event<'static>> {
        let transport = Arc::new(Captured::default());
        let mut options = sentry_options(release("skaityk", "3.0.6"), platform);
        options.dsn = Some("https://key@sentry.invalid/1".parse()?);
        options.transport = Some(Arc::new(transport.clone()));
        options.integrations.push(Arc::new(ContextIntegration::default()));
        let client = Client::with_options(options);

        Hub::run(
            Arc::new(Hub::new(Some(Arc::new(client)), Arc::new(Scope::default()))),
            || sentry::capture_message("the button does nothing", sentry::Level::Info),
        );

        let mut envelopes = transport.0.lock();
        assert_eq!(envelopes.len(), 1);
        let envelope = envelopes.remove(0);
        let event = envelope.items().find_map(|item| match item {
            EnvelopeItem::Event(event) => Some(event.clone()),
            _ => None,
        });
        Ok(*event.expect("the envelope has no event"))
    }

    /// An event says the platform of the engine and not the uname of
    /// `sentry-contexts`, keeps the arch of that integration, and carries
    /// the release of the app.
    #[test]
    fn event_says_the_platform_and_the_app() -> Result<()> {
        let event = captured_event(iphone())?;

        assert_eq!(event.release.as_deref(), Some("skaityk@3.0.6"));
        // The integration ran, it is the only one that adds a rust context.
        assert!(event.contexts.contains_key("rust"));

        let Some(Context::Os(os)) = event.contexts.get("os") else {
            panic!("the event has no os context: {:?}", event.contexts);
        };
        assert_eq!(os.name.as_deref(), Some("iOS"));
        assert_eq!(os.version.as_deref(), Some("26.1"));
        assert_eq!(os.build.as_deref(), Some("23B85"));
        assert_eq!(os.kernel_version, None);

        let Some(Context::Device(device)) = event.contexts.get("device") else {
            panic!("the event has no device context: {:?}", event.contexts);
        };
        assert_eq!(device.model.as_deref(), Some("iPhone17,1"));
        assert_eq!(device.model_id.as_deref(), Some("D47AP"));
        assert_eq!(device.family.as_deref(), Some("iPhone"));
        assert_eq!(device.arch.as_deref(), Some(std::env::consts::ARCH));
        assert_eq!(device.simulator, None);
        assert_eq!(
            device.other.get("manufacturer").and_then(|value| value.as_str()),
            Some("Apple")
        );

        Ok(())
    }

    /// A value the platform does not give leaves the one of the
    /// integration, and a simulator says so.
    #[test]
    fn event_of_a_platform_with_no_model() -> Result<()> {
        let event = captured_event(Platform {
            os: "Ubuntu".to_string(),
            version: Some("24.04".to_string()),
            kernel: Some("6.8.0-45-generic".to_string()),
            simulator: true,
            ..Default::default()
        })?;

        let Some(Context::Os(os)) = event.contexts.get("os") else {
            panic!("the event has no os context");
        };
        assert_eq!(os.name.as_deref(), Some("Ubuntu"));
        assert_eq!(os.version.as_deref(), Some("24.04"));
        assert_eq!(os.kernel_version.as_deref(), Some("6.8.0-45-generic"));

        let Some(Context::Device(device)) = event.contexts.get("device") else {
            panic!("the event has no device context");
        };
        assert_eq!(device.arch.as_deref(), Some(std::env::consts::ARCH));
        assert_eq!(device.simulator, Some(true));
        assert!(!device.other.contains_key("manufacturer"));

        Ok(())
    }

    #[test]
    fn release_is_the_app_and_its_version() {
        assert_eq!(release("skaityk", "3.0.6"), "skaityk@3.0.6");
    }

    #[test]
    fn version_text_drops_a_zero_patch() {
        assert_eq!(version_text(26, 1, 0), "26.1");
        assert_eq!(version_text(26, 0, 0), "26.0");
        assert_eq!(version_text(18, 6, 2), "18.6.2");
    }

    #[test]
    fn family_of_a_model() {
        assert_eq!(apple_family("iPhone17,1").as_deref(), Some("iPhone"));
        assert_eq!(apple_family("iPad16,3").as_deref(), Some("iPad"));
        assert_eq!(apple_family("Mac14,2").as_deref(), Some("Mac"));
        assert_eq!(apple_family("MacBookPro18,3").as_deref(), Some("MacBookPro"));
        assert_eq!(apple_family("AppleTV14,1").as_deref(), Some("AppleTV"));
        assert_eq!(apple_family("17,1"), None);

        assert_eq!(first_word("Pixel 8").as_deref(), Some("Pixel"));
        assert_eq!(first_word("SM-S918B").as_deref(), Some("SM-S918B"));
        assert_eq!(first_word(" "), None);
    }

    #[test]
    fn os_release_name_and_version() {
        let ubuntu = "PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nNAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nID=ubuntu\n";
        assert_eq!(
            parse_os_release(ubuntu),
            (Some("Ubuntu".to_string()), Some("24.04".to_string()))
        );

        let nixos = "ANSI_COLOR='0;38;2;126;186;228'\nNAME=NixOS\nVERSION_ID='25.05'\n";
        assert_eq!(
            parse_os_release(nixos),
            (Some("NixOS".to_string()), Some("25.05".to_string()))
        );

        // A rolling release has no VERSION_ID.
        let arch = "NAME=\"Arch Linux\"\nBUILD_ID=rolling\n";
        assert_eq!(parse_os_release(arch), (Some("Arch Linux".to_string()), None));
    }

    #[test]
    fn filler_model_is_no_model() {
        let some = |text: &str| Some(text.to_string());
        assert_eq!(real_model(some("System Product Name")), None);
        assert_eq!(real_model(some("To be filled by O.E.M.")), None);
        assert_eq!(real_model(some("default string")), None);
        assert_eq!(real_model(None), None);
        assert_eq!(
            real_model(some("ThinkPad X1 Carbon Gen 11")),
            some("ThinkPad X1 Carbon Gen 11")
        );
    }

    /// The detection on the machine the test runs on.
    #[test]
    fn detect_this_machine() {
        let platform = Platform::detect();
        println!("{platform}, {platform:?}");

        assert_ne!(platform.os, "");
        assert!(!platform.simulator);

        #[cfg(target_os = "macos")]
        {
            assert_eq!(platform.os, "macOS");
            let version = platform.version.expect("a macOS version");
            assert!(
                version.split('.').all(|part| part.parse::<u32>().is_ok()),
                "{version}"
            );
            assert!(platform.build.is_some());
            let model = platform.model.expect("a Mac model");
            assert!(model.contains(','), "{model}");
            assert!(platform.family.expect("a Mac family").starts_with("Mac"));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(platform.kernel.is_some());
            assert!(platform.version.is_some());
            assert_ne!(platform.os, "Linux", "the builder has an os-release file");
        }
    }
}
