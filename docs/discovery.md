# Local network discovery

The `discovery` feature, off by default, gives `hilen::discovery`, a browse for an
mDNS service type on the local network. `inspect` turns it on. Found by turnscan, whose
Wi-Fi boards announce `_turnscan._tcp`.

```rust
let mut browser = ServiceBrowser::start("_turnscan._tcp")?;
spawn(async move {
    while let Ok(event) = browser.next_event().await {
        on_main(move || match event {
            ServiceEvent::Found(board) => self.show(board),
            ServiceEvent::Lost(name) => self.remove(&name),
        });
    }
});
```

- `ServiceBrowser::start(type)` takes a type with or without `.local.`, from any
  thread, and the browse stops when the browser is dropped.
- `next_event().await` gives `ServiceEvent::Found(NetService)` when a device comes or
  changes and `ServiceEvent::Lost(name)` when it goes. An error ends the browse.
- `NetService` has `name`, `host`, `addresses`, never empty, IPv4 of the real network
  first and link local last, `port` and `text`. `socket_address()` is the first
  address with the port.

An app that browses on an iPhone names its types in `hilen.toml`:

```toml
[local_network]
services = ["_turnscan._tcp"]
usage = "Turnscan finds its turntable boards on your Wi-Fi."
```

## Per platform

- macOS, iOS, tvOS: the system Bonjour service, DNSServiceBrowse, then
  DNSServiceResolve and DNSServiceGetAddrInfo on 1 serial queue, so a change is
  reported. On iOS the error -65555 says to add the type to `[local_network]` and
  -65570 that the user did not allow the local network. The build writes
  `NSBonjourServices` and `NSLocalNetworkUsageDescription`, with plutil since a text
  with an apostrophe broke PlistBuddy, and adds `_hilen-inspect._tcp` to a build with
  the inspect server.
- Linux and Windows: `mdns-sd`.
- Android: `mdns-sd` plus a Wi-Fi `MulticastLock` over JNI while a browse runs, and
  the build adds `CHANGE_WIFI_MULTICAST_STATE`.
- Browser: an error, a browser has no mDNS.

`mdns-sd` is on no Apple target. So the inspect server of a Mac announces through the
system Bonjour too, `inspect/bonjour.rs`, as on iOS.

## Proven

On mac and Linux a test announces with `mdns-sd` and the browse finds the service and
sees it go. iOS and wasm build. Windows and Android were not compiled, no real iPhone
ran it. `hilen/native/ios/hot_loader.plist` names only the inspect type, so an app run
through `make device` cannot browse its own types yet.
