//! The browse through the Bonjour service of the system, on macOS, iOS and
//! tvOS: browse for the names, resolve each name to a host, a port and a
//! text record, then ask for the addresses of the host. Each request stays
//! open while its service is on the network, so a change comes as a new
//! `Found`.
//!
//! Every reply runs on 1 serial dispatch queue of the browse, and every
//! request is freed on it, so no reply runs while a request goes away.

use std::{
    collections::{BTreeMap, HashMap},
    ffi::{CStr, CString, c_char, c_void},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr::{NonNull, null, null_mut},
    slice::from_raw_parts,
};

use anyhow::{Result, anyhow, bail};
use log::{debug, warn};
use parking_lot::Mutex;

use super::{
    NetService, Sender, ServiceEvent,
    service::{parse_text_record, sort_addresses},
};

/// `_DNSServiceRef_t` and a dispatch queue, only ever behind a pointer here.
#[repr(C)]
struct Service {
    private: [u8; 0],
}

#[repr(C)]
struct Queue {
    private: [u8; 0],
}

/// The start every socket address has.
#[repr(C)]
struct SockAddr {
    len:    u8,
    family: u8,
}

#[repr(C)]
struct SockAddrIn {
    len:    u8,
    family: u8,
    port:   u16,
    addr:   [u8; 4],
}

#[repr(C)]
struct SockAddrIn6 {
    len:      u8,
    family:   u8,
    port:     u16,
    flowinfo: u32,
    addr:     [u8; 16],
    scope_id: u32,
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
    extern "C" fn(*mut Service, u32, u32, i32, *const c_char, *const c_void, u32, *mut c_void);

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

const FLAG_MORE_COMING: u32 = 0x1;
const FLAG_ADD: u32 = 0x2;
const PROTOCOL_IPV4: u32 = 0x1;
const PROTOCOL_IPV6: u32 = 0x2;
const AF_INET: u8 = 2;
const AF_INET6: u8 = 30;
/// The service type is not in `NSBonjourServices` of the `Info.plist`.
const ERROR_NO_AUTH: i32 = -65555;
/// The user did not allow the local network.
const ERROR_POLICY_DENIED: i32 = -65570;

/// 1 open request. Freed on the queue of its browse.
struct Request(NonNull<Service>);

impl Request {
    /// Puts a new request on the queue.
    fn keep(service: *mut Service, error: i32, queue: NonNull<Queue>) -> Result<Self> {
        if error != 0 {
            bail!("error {error}");
        }
        let request = Self(NonNull::new(service).ok_or_else(|| anyhow!("no request"))?);
        // SAFETY: a live request and the live queue of the browse.
        let error = unsafe { DNSServiceSetDispatchQueue(request.0.as_ptr(), queue.as_ptr()) };
        if error != 0 {
            bail!("the queue was refused, error {error}");
        }
        Ok(request)
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        // SAFETY: made by a `DNSService` call, freed once, on its queue.
        unsafe { DNSServiceRefDeallocate(self.0.as_ptr()) };
    }
}

/// The context of the requests of 1 service.
struct Context {
    state: NonNull<State>,
    name:  String,
}

/// 1 name the browse found. The requests are freed before the context.
struct Found {
    resolve:    Option<Request>,
    lookup:     Option<Request>,
    /// The interfaces the name was seen on, it is lost when none is left.
    interfaces: u32,
    host:       String,
    port:       u16,
    text:       BTreeMap<String, String>,
    addresses:  Vec<IpAddr>,
    sent:       Option<NetService>,
    context:    Box<Context>,
}

impl Found {
    /// Sends the service when it is complete and differs from the last one
    /// sent.
    fn send(&mut self, sender: Option<&Sender>) {
        if self.addresses.is_empty() {
            return;
        }
        let mut addresses = self.addresses.clone();
        sort_addresses(&mut addresses);
        let service = NetService {
            name: self.context.name.clone(),
            host: self.host.clone(),
            addresses,
            port: self.port,
            text: self.text.clone(),
        };
        if self.sent.as_ref() == Some(&service) {
            return;
        }
        self.sent = Some(service.clone());
        if let Some(sender) = sender
            && sender.send(Ok(ServiceEvent::Found(service))).is_err()
        {
            debug!("A found service has no browser left to take it");
        }
    }
}

struct Inner {
    browse:   Option<Request>,
    services: HashMap<String, Found>,
    /// Taken when the browse fails, which ends the events.
    sender:   Option<Sender>,
}

struct State {
    service_type: String,
    queue:        NonNull<Queue>,
    inner:        Mutex<Inner>,
}

impl State {
    fn fail(&self, error: i32) {
        let message = match error {
            ERROR_NO_AUTH => format!(
                "The system does not let this app browse for {}. Add it to `services` of the [local_network] table in hilen.toml, it goes into NSBonjourServices of the Info.plist",
                self.service_type
            ),
            ERROR_POLICY_DENIED => format!(
                "The user did not allow this app to find devices on the local network, so {} is not found. Settings, Privacy, Local Network turns it on",
                self.service_type
            ),
            error => format!(
                "Bonjour stopped the browse for {}, error {error}",
                self.service_type
            ),
        };
        warn!("{message}");
        if let Some(sender) = self.inner.lock().sender.take()
            && sender.send(Err(anyhow!(message))).is_err()
        {
            debug!("The failed browse has no browser left");
        }
    }
}

pub(super) struct Browse {
    state: NonNull<State>,
}

// SAFETY: the state is used on the queue of the browse and through its
// mutex, and freed once, in `stop`, after the queue let go of it.
unsafe impl Send for Browse {}

impl Browse {
    pub(super) fn start(service_type: &str, sender: Sender) -> Result<Self> {
        let type_text = CString::new(service_type)?;
        // SAFETY: a label that ends with 0, and no attributes is a serial
        // queue.
        let queue = unsafe { dispatch_queue_create(c"hilen.discovery".as_ptr(), null()) };
        let queue = NonNull::new(queue).ok_or_else(|| anyhow!("No dispatch queue for the browse"))?;
        let state = NonNull::from(Box::leak(Box::new(State {
            service_type: service_type.to_string(),
            queue,
            inner: Mutex::new(Inner {
                browse:   None,
                services: HashMap::new(),
                sender:   Some(sender),
            }),
        })));
        let mut browse = Self { state };

        let mut service = null_mut();
        // SAFETY: the type lives over the call, the system copies it. The
        // state lives until `stop` freed every request.
        let error = unsafe {
            DNSServiceBrowse(
                &raw mut service,
                0,
                0,
                type_text.as_ptr(),
                null(),
                browsed,
                state.as_ptr().cast(),
            )
        };
        match Request::keep(service, error, queue) {
            Ok(request) => {
                // SAFETY: the state lives until `stop`.
                unsafe { state.as_ref() }.inner.lock().browse = Some(request);
                Ok(browse)
            }
            Err(error) => {
                browse.stop();
                Err(error.context(format!("Bonjour did not start the browse for {service_type}")))
            }
        }
    }

    /// Frees every request on the queue, then the state.
    pub(super) fn stop(&mut self) {
        // SAFETY: the state lives until the end of this call.
        let queue = unsafe { self.state.as_ref() }.queue;
        // SAFETY: the queue and the state live over the call.
        unsafe {
            dispatch_sync_f(queue.as_ptr(), self.state.as_ptr().cast(), free_requests);
            dispatch_release(queue.as_ptr());
            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}

extern "C" fn free_requests(state: *mut c_void) {
    // SAFETY: the state of the browse, alive over this call.
    let state = unsafe { &*state.cast::<State>() };
    let mut inner = state.inner.lock();
    inner.browse = None;
    inner.services.clear();
    inner.sender = None;
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
    _: u32,
    error: i32,
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    // SAFETY: the state of the browse, alive while its requests are.
    let state = unsafe { &*context.cast::<State>() };
    if error != 0 {
        state.fail(error);
        return;
    }
    let Some(name_text) = text(name) else {
        return;
    };
    let mut inner = state.inner.lock();
    let inner = &mut *inner;

    if flags & FLAG_ADD == 0 {
        let Some(found) = inner.services.get_mut(&name_text) else {
            return;
        };
        found.interfaces = found.interfaces.saturating_sub(1);
        if found.interfaces > 0 {
            return;
        }
        let Some(found) = inner.services.remove(&name_text) else {
            return;
        };
        if found.sent.is_some()
            && let Some(sender) = &inner.sender
            && sender.send(Ok(ServiceEvent::Lost(name_text))).is_err()
        {
            debug!("A lost service has no browser left to take it");
        }
        return;
    }

    let found = inner.services.entry(name_text.clone()).or_insert_with(|| Found {
        resolve:    None,
        lookup:     None,
        interfaces: 0,
        host:       String::new(),
        port:       0,
        text:       BTreeMap::new(),
        addresses:  Vec::new(),
        sent:       None,
        context:    Box::new(Context {
            state: NonNull::from(state),
            name:  name_text,
        }),
    });
    found.interfaces += 1;
    if found.resolve.is_some() {
        return;
    }

    let mut service = null_mut();
    let context: *const Context = &raw const *found.context;
    // Any interface, a name seen on several is resolved once.
    // SAFETY: the strings come from the system and live over this reply.
    // The context lives as long as the request, both go with `found`.
    let error = unsafe {
        DNSServiceResolve(
            &raw mut service,
            0,
            0,
            name,
            service_type,
            domain,
            resolved,
            context.cast_mut().cast(),
        )
    };
    match Request::keep(service, error, state.queue) {
        Ok(request) => found.resolve = Some(request),
        Err(error) => warn!("Bonjour did not resolve {}: {error}", found.context.name),
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
    context: *mut c_void,
) {
    // SAFETY: the context of a service, alive while its requests are.
    let service_context = unsafe { &*context.cast::<Context>() };
    // SAFETY: the state outlives every request.
    let state = unsafe { service_context.state.as_ref() };
    if error != 0 {
        debug!("Bonjour did not resolve {}, error {error}", service_context.name);
        return;
    }
    let Some(host_name) = text(host) else {
        return;
    };
    let record = if text_record.is_null() {
        &[][..]
    } else {
        // SAFETY: the system gives `text_len` bytes that live over this reply.
        unsafe { from_raw_parts(text_record, usize::from(text_len)) }
    };

    let mut inner = state.inner.lock();
    let inner = &mut *inner;
    let Some(found) = inner.services.get_mut(&service_context.name) else {
        return;
    };
    found.port = u16::from_be(port);
    found.text = parse_text_record(record);
    if found.host == host_name && found.lookup.is_some() {
        found.send(inner.sender.as_ref());
        return;
    }

    found.host = host_name;
    found.addresses.clear();
    found.lookup = None;
    let mut service = null_mut();
    // Every interface is asked. The one of this reply can be a cable, and
    // the host has its address of the real network on another one.
    // SAFETY: the host comes from the system and lives over this reply, the
    // context lives as long as the request.
    let error = unsafe {
        DNSServiceGetAddrInfo(
            &raw mut service,
            0,
            0,
            PROTOCOL_IPV4 | PROTOCOL_IPV6,
            host,
            addressed,
            context,
        )
    };
    match Request::keep(service, error, state.queue) {
        Ok(request) => found.lookup = Some(request),
        Err(error) => warn!("Bonjour gave no address of {}: {error}", found.host),
    }
}

extern "C" fn addressed(
    _: *mut Service,
    flags: u32,
    _: u32,
    error: i32,
    _: *const c_char,
    address: *const c_void,
    _: u32,
    context: *mut c_void,
) {
    // SAFETY: the context of a service, alive while its requests are.
    let context = unsafe { &*context.cast::<Context>() };
    // SAFETY: the state outlives every request.
    let state = unsafe { context.state.as_ref() };
    let mut inner = state.inner.lock();
    let inner = &mut *inner;
    let Some(found) = inner.services.get_mut(&context.name) else {
        return;
    };

    // A host with no IPv6 address answers that with an error, that is no
    // failure of the lookup.
    if error == 0
        && let Some(ip) = ip_address(address)
    {
        if flags & FLAG_ADD == 0 {
            found.addresses.retain(|known| *known != ip);
        } else if !found.addresses.contains(&ip) {
            found.addresses.push(ip);
        }
    }
    if flags & FLAG_MORE_COMING == 0 {
        found.send(inner.sender.as_ref());
    }
}

/// The system gives a socket address aligned for its family.
fn ip_address(address: *const c_void) -> Option<IpAddr> {
    if address.is_null() {
        return None;
    }
    // SAFETY: not null, and every socket address starts with these fields.
    let family = unsafe { (*address.cast::<SockAddr>()).family };
    match family {
        AF_INET => {
            // SAFETY: the family says it is a `sockaddr_in`.
            let address = unsafe { &*address.cast::<SockAddrIn>() };
            Some(IpAddr::V4(Ipv4Addr::from(address.addr)))
        }
        AF_INET6 => {
            // SAFETY: the family says it is a `sockaddr_in6`.
            let address = unsafe { &*address.cast::<SockAddrIn6>() };
            Some(IpAddr::V6(Ipv6Addr::from(address.addr)))
        }
        _ => None,
    }
}
