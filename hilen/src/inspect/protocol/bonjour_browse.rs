//! The search for inspect servers on macOS, through the Bonjour service of
//! the system, see `docs/inspect.md`.
//!
//! The `mdns-sd` crate shares the mDNS port with the system there and often
//! hears no answer at all, while the system has every app. So on a Mac the
//! system is asked: browse for the service, resolve each name to a host and
//! a port, then ask for the address of the host.

use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_void},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ptr::{null, null_mut, with_exposed_provenance_mut},
    slice::from_raw_parts,
};

use anyhow::{Result, bail};
use log::debug;
use parking_lot::Mutex;

use crate::inspect::protocol::SERVICE_TYPE;

/// `_DNSServiceRef_t` and a dispatch queue, only ever behind a pointer here.
#[repr(C)]
struct Service {
    private: [u8; 0],
}

#[repr(C)]
struct Queue {
    private: [u8; 0],
}

/// The start of a `sockaddr_in`, as far as it is read here.
#[repr(C)]
struct SockAddrIn {
    len:    u8,
    family: u8,
    port:   u16,
    addr:   [u8; 4],
}

type BrowseReply =
    extern "C" fn(*mut Service, u32, u32, i32, *const c_char, *const c_char, *const c_char, *mut c_void);
type ResolveReply = extern "C" fn(
    *mut Service,
    u32,
    u32,
    i32,
    *const c_char,
    *const c_char,
    u16,
    u16,
    *const u8,
    *mut c_void,
);
type AddressReply =
    extern "C" fn(*mut Service, u32, u32, i32, *const c_char, *const SockAddrIn, u32, *mut c_void);

unsafe extern "C" {
    fn DNSServiceBrowse(
        service: *mut *mut Service,
        flags: u32,
        interface: u32,
        service_type: *const c_char,
        domain: *const c_char,
        reply: BrowseReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceResolve(
        service: *mut *mut Service,
        flags: u32,
        interface: u32,
        name: *const c_char,
        service_type: *const c_char,
        domain: *const c_char,
        reply: ResolveReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceGetAddrInfo(
        service: *mut *mut Service,
        flags: u32,
        interface: u32,
        protocol: u32,
        host: *const c_char,
        reply: AddressReply,
        context: *mut c_void,
    ) -> i32;
    fn DNSServiceSetDispatchQueue(service: *mut Service, queue: *mut Queue) -> i32;
    fn DNSServiceRefDeallocate(service: *mut Service);
    fn dispatch_queue_create(label: *const c_char, attributes: *const c_void) -> *mut Queue;
    fn dispatch_sync_f(queue: *mut Queue, context: *mut c_void, work: extern "C" fn(*mut c_void));
    fn dispatch_release(queue: *mut Queue);
}

const FLAG_ADD: u32 = 2;
const PROTOCOL_IPV4: u32 = 1;
const AF_INET: u8 = 2;

/// What 1 search has found so far. Every reply runs on the queue of the
/// search, one at a time.
#[derive(Default)]
struct Search {
    queue:    usize,
    /// Every open request, each ends when it is freed.
    requests: Vec<usize>,
    /// The apps on a host whose address is not known yet.
    waiting:  HashMap<String, Vec<(String, u16)>>,
    found:    HashMap<String, SocketAddr>,
}

static SEARCH: Mutex<Option<Search>> = Mutex::new(None);

/// Starts a search. Only 1 runs at a time.
pub(super) fn start() -> Result<()> {
    let mut search = SEARCH.lock();
    if search.is_some() {
        bail!("A search for apps runs already");
    }

    // SAFETY: a label that ends with 0, and no attributes is a serial queue.
    let queue = unsafe { dispatch_queue_create(c"hilen.inspect.browse".as_ptr(), null()) };
    *search = Some(Search {
        queue: queue.expose_provenance(),
        ..Search::default()
    });
    drop(search);

    let service_type = service_type()?;
    let mut service = null_mut();
    // SAFETY: the strings live over the call, the system copies them.
    let error = unsafe {
        DNSServiceBrowse(
            &raw mut service,
            0,
            0,
            service_type.as_ptr(),
            null(),
            browsed,
            null_mut(),
        )
    };
    keep(service, error).inspect_err(|_| {
        finish();
    })
}

/// The apps found so far.
pub(super) fn found() -> HashMap<String, SocketAddr> {
    SEARCH.lock().as_ref().map_or_default(|search| search.found.clone())
}

/// Ends the search and returns what it found.
pub(super) fn finish() -> HashMap<String, SocketAddr> {
    let Some(queue) = SEARCH.lock().as_ref().map(|search| search.queue) else {
        return HashMap::default();
    };
    let queue = with_exposed_provenance_mut::<Queue>(queue);
    // A request is freed on its queue, so no reply of it runs meanwhile.
    // SAFETY: the queue lives until the release below.
    unsafe {
        dispatch_sync_f(queue, null_mut(), free_requests);
        dispatch_release(queue);
    }
    SEARCH.lock().take().map_or_default(|search| search.found)
}

extern "C" fn free_requests(_: *mut c_void) {
    let requests = SEARCH.lock().as_mut().map_or_default(|search| search.requests.split_off(0));
    for request in requests {
        // SAFETY: made by a `DNSService` call and listed once.
        unsafe { DNSServiceRefDeallocate(with_exposed_provenance_mut(request)) };
    }
}

/// The system wants the type alone, the domain is its own choice.
fn service_type() -> Result<CString> {
    Ok(CString::new(SERVICE_TYPE.trim_end_matches(".local."))?)
}

/// Puts a new request on the queue of the search and lists it.
fn keep(service: *mut Service, error: i32) -> Result<()> {
    if error != 0 {
        bail!("Bonjour refused a request, error {error}");
    }
    let mut search = SEARCH.lock();
    let Some(search) = search.as_mut() else {
        // SAFETY: just made, and no search is left to own it.
        unsafe { DNSServiceRefDeallocate(service) };
        return Ok(());
    };
    search.requests.push(service.expose_provenance());
    // SAFETY: a live request and the live queue of the search.
    let error = unsafe { DNSServiceSetDispatchQueue(service, with_exposed_provenance_mut(search.queue)) };
    if error != 0 {
        bail!("Bonjour did not take the queue, error {error}");
    }
    Ok(())
}

fn text(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: not null, and the system ends its strings with 0.
    Some(unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned())
}

extern "C" fn browsed(
    _: *mut Service,
    flags: u32,
    interface: u32,
    error: i32,
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    _: *mut c_void,
) {
    if error != 0 || flags & FLAG_ADD == 0 {
        return;
    }
    let mut service = null_mut();
    // SAFETY: the strings come from the system and live over this reply.
    let error = unsafe {
        DNSServiceResolve(
            &raw mut service,
            0,
            interface,
            name,
            service_type,
            domain,
            resolved,
            null_mut(),
        )
    };
    if let Err(error) = keep(service, error) {
        debug!("{error}");
    }
}

extern "C" fn resolved(
    _: *mut Service,
    _: u32,
    _: u32,
    error: i32,
    _: *const c_char,
    host: *const c_char,
    port: u16,
    text_len: u16,
    text_record: *const u8,
    _: *mut c_void,
) {
    if error != 0 || text_record.is_null() {
        return;
    }
    // SAFETY: the system gives `text_len` bytes that live over this reply.
    let record = unsafe { from_raw_parts(text_record, usize::from(text_len)) };
    let (Some(app_id), Some(host_name)) = (text_value(record, "app_id"), text(host)) else {
        return;
    };

    if let Some(search) = SEARCH.lock().as_mut() {
        let apps = search.waiting.entry(host_name).or_default();
        let app = (app_id, u16::from_be(port));
        if apps.contains(&app) {
            return;
        }
        apps.push(app);
    }

    let mut service = null_mut();
    // Every interface is asked. The one of this reply can be a cable, and
    // the host has its address of the real network on another one.
    // SAFETY: the host comes from the system and lives over this reply.
    let error =
        unsafe { DNSServiceGetAddrInfo(&raw mut service, 0, 0, PROTOCOL_IPV4, host, addressed, null_mut()) };
    if let Err(error) = keep(service, error) {
        debug!("{error}");
    }
}

extern "C" fn addressed(
    _: *mut Service,
    _: u32,
    _: u32,
    error: i32,
    host: *const c_char,
    address: *const SockAddrIn,
    _: u32,
    _: *mut c_void,
) {
    if error != 0 || address.is_null() {
        return;
    }
    // SAFETY: not null, and every socket address starts with these fields.
    let address = unsafe { &*address };
    let Some(host) = text(host) else {
        return;
    };
    if address.family != AF_INET {
        return;
    }
    let ip = Ipv4Addr::from(address.addr);

    if let Some(search) = SEARCH.lock().as_mut() {
        for (app_id, port) in search.waiting.get(&host).cloned().unwrap_or_default() {
            let new = SocketAddr::new(IpAddr::V4(ip), port);
            let known = search.found.get(&app_id).copied();
            if better_address(known, new) {
                search.found.insert(app_id, new);
            }
        }
    }
}

/// A phone on a cable answers with the address of the cable too, which is
/// gone when the cable is. An address of the real network wins over it.
fn better_address(known: Option<SocketAddr>, new: SocketAddr) -> bool {
    let link_local = |address: SocketAddr| matches!(address.ip(), IpAddr::V4(ip) if ip.is_link_local());
    match known {
        None => true,
        Some(known) => link_local(known) && !link_local(new),
    }
}

/// The value of `key` in a DNS text record, a row of entries that each start
/// with a length byte and hold `key=value`.
pub(super) fn text_value(record: &[u8], key: &str) -> Option<String> {
    let mut rest = record;
    while let Some((len, tail)) = rest.split_first() {
        let len = usize::from(*len);
        let entry = tail.get(..len)?;
        rest = &tail[len..];
        if let Some(value) = entry.strip_prefix(key.as_bytes()).and_then(|value| value.strip_prefix(b"=")) {
            return Some(String::from_utf8_lossy(value).into_owned());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;

    use super::{better_address, text_value};

    #[test]
    fn an_address_of_the_network_wins_over_the_one_of_a_cable() {
        let cable: SocketAddr = "169.254.247.85:62489".parse().unwrap();
        let wifi: SocketAddr = "192.168.0.208:62489".parse().unwrap();

        assert!(better_address(None, cable));
        assert!(better_address(Some(cable), wifi));
        assert!(!better_address(Some(wifi), cable));
        assert!(!better_address(Some(wifi), wifi));
    }

    #[test]
    fn a_value_is_read_from_a_text_record() {
        let record = b"\x05a=one\x0dapp_id=ABCDEF";
        assert_eq!(text_value(record, "app_id").as_deref(), Some("ABCDEF"));
        assert_eq!(text_value(record, "a").as_deref(), Some("one"));
        assert_eq!(text_value(record, "missing"), None);
        // An entry that is longer than the record.
        assert_eq!(text_value(b"\x20app_id=A", "app_id"), None);
    }
}
