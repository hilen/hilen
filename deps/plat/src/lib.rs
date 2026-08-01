pub struct Platform;

impl Platform {
    pub const MAC: bool = cfg!(target_os = "macos");
    pub const LINUX: bool = cfg!(target_os = "linux");
    pub const WINDOWS: bool = cfg!(target_os = "windows");
    // IOS is true on tvOS too, since tvOS runs the same UIKit stack and almost
    // all ios code applies as is. TVOS exists for the few real differences.
    pub const IOS: bool = cfg!(any(target_os = "ios", target_os = "tvos"));
    pub const TVOS: bool = cfg!(target_os = "tvos");
    pub const ANDROID: bool = cfg!(target_os = "android");
    pub const DESKTOP: bool = Self::MAC || Self::LINUX || Self::WINDOWS;
    pub const MOBILE: bool = Self::IOS || Self::ANDROID;
    pub const WASM: bool = cfg!(target_arch = "wasm32");

    pub const APPLE: bool = Self::MAC || Self::IOS;
}

impl Platform {
    pub fn dump() {
        dbg!(Self::MAC);
        dbg!(Self::LINUX);
        dbg!(Self::WINDOWS);
        dbg!(Self::IOS);
        dbg!(Self::TVOS);
        dbg!(Self::ANDROID);
        dbg!(Self::DESKTOP);
        dbg!(Self::MOBILE);
        dbg!(Self::WASM);

        dbg!(Self::APPLE);
    }
}

pub fn platforms() {
    cfg_aliases::cfg_aliases! {
        wasm:     {     target_arch = "wasm32"  },
        not_wasm: { not(target_arch = "wasm32") },

        android:     {     target_os = "android"  },
        not_android: { not(target_os = "android") },

        // ios is also true on tvos, the tvos alias exists for the few real differences
        ios:     {     any(target_os = "ios", target_os = "tvos")  },
        not_ios: { not(any(target_os = "ios", target_os = "tvos")) },

        tvos:    { target_os = "tvos" },

        macos:   { target_os = "macos" },
        linux:   { target_os = "linux" },
        win:     { target_os = "windows" },

        desktop: { any(target_os =   "macos", target_os = "linux", target_os = "windows") },
        mobile:  { any(target_os = "android", target_os =   "ios", target_os =    "tvos") },

        apple:   { any(target_os = "macos", target_os = "ios", target_os = "tvos") }
    }
}

#[cfg(test)]
mod test {
    use crate::Platform;

    #[test]
    fn test() {
        Platform::dump();
    }
}
