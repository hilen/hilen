//! Web Bluetooth. The JS objects of a connection stay on the main thread in
//! `OPEN`, a `WebLink` only holds its key, so a `BleConnection` can go to
//! `on_main` like on every other platform.

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use anyhow::{Result, anyhow, bail};
use log::{debug, warn};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    EventTarget,
    js_sys::{Array, DataView, Function, Object, Promise, Reflect, Uint8Array},
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

use crate::bluetooth::{
    BleCharacteristic, BleDevice, BleProperty, BleService, BleWrite, Uuid,
    backend::{Backend, Connected, Link, Pending},
    connection::Events,
};

type Listener = Closure<dyn FnMut(JsValue)>;

thread_local! {
    /// The devices the user picked in the chooser, by id.
    static PICKED: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
    static OPEN: RefCell<HashMap<u32, Open>> = RefCell::new(HashMap::new());
}

static NEXT_KEY: AtomicU32 = AtomicU32::new(0);

pub(super) struct Web;

impl Backend for Web {
    fn scan(&self, services: Vec<Uuid>, _time: Duration) -> Pending<'_, Vec<BleDevice>> {
        Box::pin(async move {
            let navigator = web_sys::window().ok_or_else(|| anyhow!("no window"))?.navigator();
            let bluetooth = get(&navigator, "bluetooth")?;
            if bluetooth.is_undefined() {
                bail!("This browser has no Web Bluetooth, Chrome and Edge have it");
            }
            let options = request_options(&services)?;
            let device = match wait(call(&bluetooth, "requestDevice", &[options.into()])?).await {
                Ok(device) => device,
                // The user closed the chooser.
                Err(err) if err.to_string().starts_with("NotFoundError") => {
                    debug!("BLE chooser closed with no device: {err}");
                    return Ok(Vec::new());
                }
                Err(err) => return Err(err),
            };
            let id = text(&device, "id")?.ok_or_else(|| anyhow!("the picked device has no id"))?;
            let picked = BleDevice {
                id:       id.clone(),
                name:     text(&device, "name")?,
                rssi:     None,
                services: services.clone(),
            };
            PICKED.with_borrow_mut(|picked| picked.insert(id, device));
            Ok(vec![picked])
        })
    }

    fn connect(&self, device: BleDevice, events: Events) -> Pending<'_, Connected> {
        Box::pin(async move {
            let js_device = PICKED
                .with_borrow(|picked| picked.get(&device.id).cloned())
                .ok_or_else(|| anyhow!("BLE {} was not picked in the chooser", device.label()))?;
            let gatt = get(&js_device, "gatt")?;
            let server = wait(call(&gatt, "connect", &[])?).await?;

            let mut services = Vec::new();
            let mut characteristics = Vec::new();
            let js_services: Array = wait(call(&server, "getPrimaryServices", &[])?).await?.unchecked_into();
            for js_service in js_services.iter() {
                let service = uuid(&js_service)?;
                let js_characteristics: Array =
                    wait(call(&js_service, "getCharacteristics", &[])?).await?.unchecked_into();
                let mut found = Vec::new();
                for js_characteristic in js_characteristics.iter() {
                    let characteristic = BleCharacteristic {
                        uuid: uuid(&js_characteristic)?,
                        service,
                        properties: properties(&js_characteristic)?,
                    };
                    found.push(characteristic.clone());
                    characteristics.push((characteristic, js_characteristic));
                }
                services.push(BleService {
                    uuid:            service,
                    characteristics: found,
                });
            }

            let lost = events.clone();
            let on_lost: Listener = Closure::new(move |_| lost.lost());
            listen(&js_device, "gattserverdisconnected", &on_lost)?;

            let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
            let open = Open {
                device: js_device,
                gatt,
                characteristics,
                on_lost,
                events,
                listeners: Vec::new(),
            };
            OPEN.with_borrow_mut(|opened| opened.insert(key, open));
            Ok(Connected {
                link: Box::new(WebLink {
                    key,
                    label: device.label().to_string(),
                }),
                services,
            })
        })
    }
}

/// One filter per service, a device with any of them shows in the
/// chooser. Only these services can be used after the connect.
fn request_options(services: &[Uuid]) -> Result<Object> {
    let options = Object::new();
    if services.is_empty() {
        set(&options, "acceptAllDevices", &JsValue::TRUE)?;
        return Ok(options);
    }
    let names: Array = services.iter().map(|uuid| JsValue::from_str(&uuid.to_string())).collect();
    let filters: Array = names
        .iter()
        .map(|name| {
            let filter = Object::new();
            set(&filter, "services", &Array::of1(&name))?;
            Ok(JsValue::from(filter))
        })
        .collect::<Result<_>>()?;
    set(&options, "filters", &filters)?;
    set(&options, "optionalServices", &names)?;
    Ok(options)
}

/// The JS objects of 1 open connection.
struct Open {
    device:          JsValue,
    gatt:            JsValue,
    characteristics: Vec<(BleCharacteristic, JsValue)>,
    on_lost:         Listener,
    events:          Events,
    /// The value listeners of the subscribed characteristics.
    listeners:       Vec<(Uuid, JsValue, Listener)>,
}

impl Open {
    fn remove_listeners(&mut self) {
        for (uuid, characteristic, listener) in self.listeners.drain(..) {
            if let Err(err) = unlisten(&characteristic, "characteristicvaluechanged", &listener) {
                warn!("BLE value listener of {uuid} stays: {err}");
            }
        }
        if let Err(err) = unlisten(&self.device, "gattserverdisconnected", &self.on_lost) {
            warn!("BLE disconnect listener stays: {err}");
        }
    }
}

struct WebLink {
    key:   u32,
    label: String,
}

impl WebLink {
    fn find(&self, wanted: &BleCharacteristic) -> Result<JsValue> {
        OPEN.with_borrow(|open| {
            let open = open.get(&self.key).ok_or_else(|| anyhow!("BLE {} is closed", self.label))?;
            open.characteristics
                .iter()
                .find(|(characteristic, _)| characteristic == wanted)
                .map(|(_, js)| js.clone())
                .ok_or_else(|| anyhow!("BLE {} has no characteristic {}", self.label, wanted.uuid))
        })
    }
}

impl Link for WebLink {
    fn read(&self, characteristic: BleCharacteristic) -> Pending<'_, Vec<u8>> {
        Box::pin(async move {
            let js = self.find(&characteristic)?;
            let view: DataView = wait(call(&js, "readValue", &[])?).await?.unchecked_into();
            bytes(&view)
        })
    }

    fn write(&self, characteristic: BleCharacteristic, value: Vec<u8>, kind: BleWrite) -> Pending<'_, ()> {
        Box::pin(async move {
            let js = self.find(&characteristic)?;
            let method = match kind {
                BleWrite::WithResponse => "writeValueWithResponse",
                BleWrite::WithoutResponse => "writeValueWithoutResponse",
            };
            wait(call(&js, method, &[Uint8Array::from(value.as_slice()).into()])?).await?;
            Ok(())
        })
    }

    fn subscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        Box::pin(async move {
            let js = self.find(&characteristic)?;
            let events = OPEN
                .with_borrow(|open| open.get(&self.key).map(|open| open.events.clone()))
                .ok_or_else(|| anyhow!("BLE {} is closed", self.label))?;
            let uuid = characteristic.uuid;
            let listener: Listener = Closure::new(move |event: JsValue| match changed_value(&event) {
                Ok(value) => events.notify(uuid, value),
                Err(err) => warn!("BLE notification of {uuid} has no value: {err}"),
            });
            listen(&js, "characteristicvaluechanged", &listener)?;
            OPEN.with_borrow_mut(|open| {
                if let Some(open) = open.get_mut(&self.key) {
                    open.listeners.push((uuid, js.clone(), listener));
                }
            });
            wait(call(&js, "startNotifications", &[])?).await?;
            Ok(())
        })
    }

    fn unsubscribe(&self, characteristic: BleCharacteristic) -> Pending<'_, ()> {
        Box::pin(async move {
            let js = self.find(&characteristic)?;
            wait(call(&js, "stopNotifications", &[])?).await?;
            let removed = OPEN.with_borrow_mut(|open| {
                let Some(open) = open.get_mut(&self.key) else {
                    return Vec::new();
                };
                let (removed, kept): (Vec<_>, Vec<_>) =
                    open.listeners.drain(..).partition(|(uuid, ..)| *uuid == characteristic.uuid);
                open.listeners = kept;
                removed
            });
            for (_, js, listener) in removed {
                unlisten(&js, "characteristicvaluechanged", &listener)?;
            }
            Ok(())
        })
    }

    fn disconnect(&self) -> Pending<'_, ()> {
        Box::pin(async move {
            self.close();
            Ok(())
        })
    }

    fn close(&self) {
        let Some(mut open) = OPEN.with_borrow_mut(|open| open.remove(&self.key)) else {
            return;
        };
        open.remove_listeners();
        if let Err(err) = call(&open.gatt, "disconnect", &[]) {
            warn!("BLE disconnect of {} failed: {err}", self.label);
        }
    }
}

fn changed_value(event: &JsValue) -> Result<Vec<u8>> {
    let target = get(event, "target")?;
    let view: DataView = get(&target, "value")?.unchecked_into();
    bytes(&view)
}

fn bytes(view: &DataView) -> Result<Vec<u8>> {
    let offset = u32::try_from(view.byte_offset())?;
    let length = u32::try_from(view.byte_length())?;
    Ok(Uint8Array::new_with_byte_offset_and_length(&view.buffer(), offset, length).to_vec())
}

fn uuid(object: &JsValue) -> Result<Uuid> {
    let text = text(object, "uuid")?.ok_or_else(|| anyhow!("a Web Bluetooth object has no uuid"))?;
    Ok(Uuid::parse_str(&text)?)
}

fn properties(characteristic: &JsValue) -> Result<Vec<BleProperty>> {
    let flags = get(characteristic, "properties")?;
    let mut properties = Vec::new();
    for (name, property) in [
        ("read", BleProperty::Read),
        ("write", BleProperty::Write),
        ("writeWithoutResponse", BleProperty::WriteWithoutResponse),
        ("notify", BleProperty::Notify),
        ("indicate", BleProperty::Indicate),
    ] {
        if get(&flags, name)?.as_bool() == Some(true) {
            properties.push(property);
        }
    }
    Ok(properties)
}

fn listen(target: &JsValue, event: &str, listener: &Listener) -> Result<()> {
    target
        .unchecked_ref::<EventTarget>()
        .add_event_listener_with_callback(event, listener.as_ref().unchecked_ref())
        .map_err(js_error)
}

fn unlisten(target: &JsValue, event: &str, listener: &Listener) -> Result<()> {
    target
        .unchecked_ref::<EventTarget>()
        .remove_event_listener_with_callback(event, listener.as_ref().unchecked_ref())
        .map_err(js_error)
}

fn get(target: &JsValue, key: &str) -> Result<JsValue> {
    Reflect::get(target, &JsValue::from_str(key)).map_err(js_error)
}

fn set(target: &JsValue, key: &str, value: &JsValue) -> Result<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(js_error)?;
    Ok(())
}

fn text(target: &JsValue, key: &str) -> Result<Option<String>> {
    Ok(get(target, key)?.as_string())
}

fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue> {
    let function: Function = get(target, method)?
        .dyn_into()
        .map_err(|_| anyhow!("{method} is not a function"))?;
    let args: Array = args.iter().collect();
    function.apply(target, &args).map_err(js_error)
}

async fn wait(promise: JsValue) -> Result<JsValue> {
    JsFuture::from(Promise::from(promise)).await.map_err(js_error)
}

/// `NotFoundError: User cancelled the requestDevice() chooser.` for an error
/// with a name and a message.
fn js_error(value: JsValue) -> anyhow::Error {
    let name = get(&value, "name").ok().and_then(|name| name.as_string());
    let message = get(&value, "message").ok().and_then(|message| message.as_string());
    match (name, message) {
        (Some(name), Some(message)) => anyhow!("{name}: {message}"),
        _ => anyhow!("{value:?}"),
    }
}
