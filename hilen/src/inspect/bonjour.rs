//! The announce of the inspect server on iOS, see `docs/inspect.md`.
//!
//! On a real iPhone an app may not send multicast packets of its own, that
//! needs an entitlement Apple gives on request. So the `mdns-sd` crate stays
//! silent there with no error. The Bonjour service of the system announces
//! for the app, and for that the `Info.plist` names the service type in
//! `NSBonjourServices` and has a `NSLocalNetworkUsageDescription`.

#[cfg(ios)]
use std::{
    ffi::{CString, c_char, c_void},
    ptr::{null, null_mut},
    sync::atomic::{AtomicPtr, Ordering},
};

#[cfg(ios)]
use anyhow::bail;
use anyhow::{Context, Result};

#[cfg(ios)]
use crate::inspect::protocol::SERVICE_TYPE;

/// `_DNSServiceRef_t`, only ever behind a pointer here.
#[cfg(ios)]
#[repr(C)]
struct Service {
    private: [u8; 0],
}

#[cfg(ios)]
unsafe extern "C" {
    fn DNSServiceRegister(
        service: *mut *mut Service,
        flags: u32,
        interface: u32,
        name: *const c_char,
        service_type: *const c_char,
        domain: *const c_char,
        host: *const c_char,
        port: u16,
        text_len: u16,
        text: *const c_void,
        callback: *const c_void,
        context: *mut c_void,
    ) -> i32;
    #[cfg(hot)]
    fn DNSServiceRefDeallocate(service: *mut Service);
}

/// The announce lives as long as this reference does.
#[cfg(ios)]
static SERVICE: AtomicPtr<Service> = AtomicPtr::new(null_mut());

/// Announces the inspect server of this app on the local network.
#[cfg(ios)]
pub(crate) fn announce(app_id: &str, port: u16) -> Result<()> {
    let name = CString::new(app_id)?;
    // The system wants the type alone, the domain is its own choice.
    let service_type = CString::new(SERVICE_TYPE.trim_end_matches(".local."))?;
    let text = text_record("app_id", app_id)?;

    let mut service = null_mut();
    // SAFETY: every pointer lives over the call, and the system copies what
    // it keeps. With no callback nothing has to read the reference later.
    let error = unsafe {
        DNSServiceRegister(
            &raw mut service,
            0,
            0,
            name.as_ptr(),
            service_type.as_ptr(),
            null(),
            null(),
            port.to_be(),
            u16::try_from(text.len())?,
            text.as_ptr().cast(),
            null(),
            null_mut(),
        )
    };
    if error != 0 {
        bail!("Bonjour did not take the inspect service, error {error}");
    }
    SERVICE.store(service, Ordering::Release);
    Ok(())
}

/// Takes the app off the network.
#[cfg(hot)]
pub(crate) fn stop() {
    let service = SERVICE.swap(null_mut(), Ordering::AcqRel);
    if !service.is_null() {
        // SAFETY: made by `DNSServiceRegister` and taken out of the static.
        unsafe { DNSServiceRefDeallocate(service) };
    }
}

/// 1 entry of a DNS text record: a length byte, then `key=value`.
#[cfg(any(ios, test))]
fn text_record(key: &str, value: &str) -> Result<Vec<u8>> {
    let entry = format!("{key}={value}");
    let len = u8::try_from(entry.len())
        .with_context(|| format!("a text record entry of {} bytes is too long", entry.len()))?;
    let mut record = vec![len];
    record.extend_from_slice(entry.as_bytes());
    Ok(record)
}

#[cfg(test)]
mod test {
    use super::text_record;

    #[test]
    fn a_text_record_entry_starts_with_its_length() {
        assert_eq!(text_record("app_id", "ABCDEF").unwrap(), b"\x0dapp_id=ABCDEF");
        assert!(text_record("app_id", &"A".repeat(300)).is_err());
    }
}
