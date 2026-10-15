# Bluetooth LE

The `bluetooth` feature, off by default, gives `hilen::bluetooth`, the central role
of Bluetooth LE through [btleplug](https://github.com/deviceplug/btleplug) and through
Web Bluetooth in a browser. Found by turnscan, which drives the servos of its turntable
through a Seeed XIAO nRF52840.

```rust
async fn open_board() -> anyhow::Result<Option<BleConnection>> {
    let devices = Bluetooth::scan(&[SERVICE], Duration::from_secs(5)).await?;
    let Some(board) = devices.first() else { return Ok(None) };
    let link = Bluetooth::connect(board).await?;
    link.subscribe(STATUS).await?;
    link.write(COMMAND, &[90], BleWrite::WithResponse).await?;
    Ok(Some(link))
}
```

- `Bluetooth::scan(&[Uuid], Duration)` gives the devices that advertise any of the
  services, an empty list means every device.
- `Bluetooth::connect(&device)` connects and finds the services.
- A `BleConnection` has `services`, `characteristic(uuid)`, `read`, `write` with or
  without response, `subscribe`, `unsubscribe` and `disconnect`. A call the
  characteristic does not allow fails with a clear error and never reaches the device.
- `on_notification` and `on_disconnect` run on the main thread through `on_main`. A
  drop disconnects, and neither a drop nor `disconnect` fires `on_disconnect`.

The usage text is in `hilen.toml`:

```toml
[bluetooth]
usage = "The app drives its turntable over Bluetooth."
```

## Per platform

- macOS and iOS: btleplug over CoreBluetooth. A scan waits up to 3 s for the adapter
  to be on and fails when Bluetooth is off. A Mac bundle gets
  `NSBluetoothAlwaysUsageDescription` from the release scripts of `build`, iOS gets it
  and `NSBluetoothPeripheralUsageDescription` from hilen-mobile.
- Windows: btleplug over WinRT. Linux: btleplug over BlueZ through dbus.
- Android: btleplug needs its Java classes, the engine carries them as
  `hilen/android/btleplug.dex`, made by `make android-btleplug-dex`, and loads them
  with `InMemoryDexClassLoader` as the context loader while btleplug starts. The
  engine asks for `BLUETOOTH_SCAN` and `BLUETOOTH_CONNECT` on API 31 and up, and for
  `ACCESS_FINE_LOCATION` before. hilen-mobile writes the manifest lines.
- Browser: Web Bluetooth. A scan opens the chooser of the browser and gives the 1
  device the user picked, or none, and only the services named in the scan can be
  used. The JS objects stay on the main thread, so a `BleConnection` is still `Send`.

## Proven

10 tests against a fake backend on mac and Linux: the scan filter, read and write,
refused calls, notifications from another thread in order on the main thread, a lost
connection, a drop and `disconnect`. No real device was used on any platform, no
builder has Bluetooth. Android never ran, and the hilen code was not compiled for
Windows, only btleplug alone. Phones get the permission lines only from a hilen-mobile
release with them.
