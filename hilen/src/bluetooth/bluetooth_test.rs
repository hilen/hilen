//! These tests take the place of the main thread and of the frame: each one
//! makes its own thread the main one and runs the queue of the main thread by
//! hand. They run one at a time for that.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::spawn,
    time::Duration,
};

use anyhow::Result;
use parking_lot::Mutex;
use pollster::block_on;
use serial_test::serial;

use crate::{
    bluetooth::{
        BACKEND, BleCharacteristic, BleConnection, BleDevice, BleNotification, BleProperty, BleService,
        BleWrite, Bluetooth, Uuid, fake::FakeBluetooth, permissions::android_permissions,
    },
    deps::hreads::{invoke_dispatched, set_current_thread_as_main},
};

const UART: Uuid = Uuid::from_u128(0x6e40_0001_b5a3_f393_e0a9_e50e_24dc_ca9e);
const COMMAND: Uuid = Uuid::from_u128(0x6e40_0002_b5a3_f393_e0a9_e50e_24dc_ca9e);
const STATUS: Uuid = Uuid::from_u128(0x6e40_0003_b5a3_f393_e0a9_e50e_24dc_ca9e);
const HEART_RATE: Uuid = Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);

const SCAN: Duration = Duration::from_secs(1);

fn device(id: &str, name: Option<&str>, services: &[Uuid]) -> BleDevice {
    BleDevice {
        id:       id.to_string(),
        name:     name.map(str::to_string),
        rssi:     Some(-60),
        services: services.to_vec(),
    }
}

/// A fake system with a turntable board and a heart rate belt in range.
fn turntable() -> (FakeBluetooth, BleDevice) {
    set_current_thread_as_main();
    let fake = FakeBluetooth::default();
    *BACKEND.lock() = Some(Arc::new(fake.clone()));
    let board = device("board", Some("Turntable"), &[UART]);
    fake.add(
        board.clone(),
        vec![BleService {
            uuid:            UART,
            characteristics: vec![
                BleCharacteristic {
                    uuid:       COMMAND,
                    service:    UART,
                    properties: vec![BleProperty::Write, BleProperty::WriteWithoutResponse],
                },
                BleCharacteristic {
                    uuid:       STATUS,
                    service:    UART,
                    properties: vec![BleProperty::Read, BleProperty::Notify],
                },
            ],
        }],
    );
    fake.add(device("belt", None, &[HEART_RATE]), Vec::new());
    (fake, board)
}

fn connect(board: &BleDevice) -> BleConnection {
    block_on(Bluetooth::connect(board)).unwrap()
}

/// Every notification a connection hands to the app.
fn heard(link: &BleConnection) -> Arc<Mutex<Vec<BleNotification>>> {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let store = heard.clone();
    link.on_notification(move |notification| store.lock().push(notification));
    heard
}

/// Sends from another thread, the way a system delivers.
fn notify_from_a_device_thread(fake: &FakeBluetooth, board: &BleDevice, values: &[&[u8]]) {
    let (fake, board) = (fake.clone(), board.clone());
    let values: Vec<Vec<u8>> = values.iter().map(|value| value.to_vec()).collect();
    spawn(move || {
        for value in values {
            fake.notify(&board, STATUS, &value);
        }
    })
    .join()
    .unwrap();
}

#[test]
#[serial]
fn a_scan_finds_only_the_devices_with_one_of_the_services() -> Result<()> {
    let (_fake, board) = turntable();

    assert_eq!(block_on(Bluetooth::scan(&[UART], SCAN))?, vec![board.clone()]);
    let both: Vec<_> = block_on(Bluetooth::scan(&[UART, HEART_RATE], SCAN))?
        .into_iter()
        .map(|found| found.id)
        .collect();
    assert_eq!(both, ["board", "belt"]);
    assert_eq!(block_on(Bluetooth::scan(&[], SCAN))?.len(), 2);
    assert_eq!(block_on(Bluetooth::scan(&[COMMAND], SCAN))?, Vec::new());
    Ok(())
}

#[test]
#[serial]
fn a_connection_has_the_services_and_reads_and_writes() -> Result<()> {
    let (fake, board) = turntable();
    fake.set_value(STATUS, b"ready");

    let link = connect(&board);
    assert!(link.is_connected());
    assert_eq!(link.device(), &board);
    assert_eq!(link.services().len(), 1);
    assert_eq!(link.characteristic(STATUS).map(|found| found.service), Some(UART));

    assert_eq!(block_on(link.read(STATUS))?, b"ready");
    block_on(link.write(COMMAND, &[90], BleWrite::WithResponse))?;
    block_on(link.write(COMMAND, &[0, 1], BleWrite::WithoutResponse))?;
    assert_eq!(
        fake.written(),
        [
            (COMMAND, vec![90], BleWrite::WithResponse),
            (COMMAND, vec![0, 1], BleWrite::WithoutResponse),
        ]
    );
    Ok(())
}

#[test]
#[serial]
fn a_call_the_characteristic_does_not_allow_never_reaches_the_device() {
    let (fake, board) = turntable();
    let link = connect(&board);

    let read_only = block_on(link.write(STATUS, &[1], BleWrite::WithResponse)).unwrap_err();
    assert!(
        read_only.to_string().contains("does not allow write"),
        "{read_only}"
    );
    assert!(block_on(link.read(COMMAND)).is_err());
    assert!(block_on(link.subscribe(COMMAND)).is_err());
    let unknown = block_on(link.read(HEART_RATE)).unwrap_err();
    assert!(unknown.to_string().contains("has no characteristic"), "{unknown}");

    assert_eq!(fake.written(), Vec::new());
    assert_eq!(fake.subscribed(), Vec::<Uuid>::new());
}

#[test]
#[serial]
fn notifications_reach_the_main_thread_in_order() -> Result<()> {
    let (fake, board) = turntable();
    let link = connect(&board);
    let heard = heard(&link);
    block_on(link.subscribe(STATUS))?;
    assert_eq!(fake.subscribed(), [STATUS]);

    notify_from_a_device_thread(&fake, &board, &[b"45", b"90", b"135"]);
    assert!(
        heard.lock().is_empty(),
        "nothing before the main thread runs its queue"
    );

    invoke_dispatched();
    let values: Vec<_> = heard.lock().iter().map(|heard| heard.value.clone()).collect();
    assert_eq!(values, [b"45".to_vec(), b"90".to_vec(), b"135".to_vec()]);
    assert!(heard.lock().iter().all(|heard| heard.characteristic == STATUS));

    block_on(link.unsubscribe(STATUS))?;
    notify_from_a_device_thread(&fake, &board, &[b"180"]);
    invoke_dispatched();
    assert_eq!(heard.lock().len(), 3);
    Ok(())
}

#[test]
#[serial]
fn a_lost_connection_calls_on_disconnect_once_and_later_calls_fail() {
    let (fake, board) = turntable();
    let link = connect(&board);
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    link.on_disconnect(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });

    let (lost_fake, lost_board) = (fake.clone(), board.clone());
    spawn(move || lost_fake.lose(&lost_board)).join().unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0, "only on the main thread");
    assert!(!link.is_connected(), "the state changes at once");

    invoke_dispatched();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let failed = block_on(link.read(STATUS)).unwrap_err();
    assert!(failed.to_string().contains("not connected"), "{failed}");
}

#[test]
#[serial]
fn a_dropped_connection_is_closed_and_hears_nothing_more() -> Result<()> {
    let (fake, board) = turntable();
    let link = connect(&board);
    let heard = heard(&link);
    let lost = Arc::new(AtomicUsize::new(0));
    let counter = lost.clone();
    link.on_disconnect(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    block_on(link.subscribe(STATUS))?;

    notify_from_a_device_thread(&fake, &board, &[b"late"]);
    assert_eq!(fake.open_count(), 1);
    drop(link);
    assert_eq!(fake.open_count(), 0);

    invoke_dispatched();
    assert!(
        heard.lock().is_empty(),
        "a value that waited for the main thread is dropped"
    );
    assert_eq!(lost.load(Ordering::Relaxed), 0, "a drop is no loss");
    Ok(())
}

#[test]
#[serial]
fn a_disconnect_closes_without_calling_on_disconnect() -> Result<()> {
    let (fake, board) = turntable();
    let link = connect(&board);
    let lost = Arc::new(AtomicUsize::new(0));
    let counter = lost.clone();
    link.on_disconnect(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });

    block_on(link.disconnect())?;
    invoke_dispatched();
    assert_eq!(fake.open_count(), 0);
    assert_eq!(lost.load(Ordering::Relaxed), 0);
    Ok(())
}

#[test]
#[serial]
fn a_device_out_of_range_does_not_connect() {
    let (_fake, _board) = turntable();
    let gone = device("gone", Some("Old board"), &[UART]);
    let failed = block_on(Bluetooth::connect(&gone)).unwrap_err();
    assert!(failed.to_string().contains("Old board"), "{failed}");
}

#[test]
fn a_device_is_named_by_its_name_or_its_id() {
    assert_eq!(device("board", Some("Turntable"), &[]).label(), "Turntable");
    assert_eq!(device("AA:BB", None, &[]).label(), "AA:BB");
}

#[test]
fn android_12_asks_for_scan_and_connect_and_older_for_the_location() {
    for api in [31, 34] {
        assert_eq!(
            android_permissions(api),
            [
                "android.permission.BLUETOOTH_SCAN",
                "android.permission.BLUETOOTH_CONNECT"
            ]
        );
    }
    for api in [26, 30] {
        assert_eq!(
            android_permissions(api),
            ["android.permission.ACCESS_FINE_LOCATION"]
        );
    }
}
