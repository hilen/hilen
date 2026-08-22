#![cfg(all(test, not(target_arch = "wasm32")))]

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{sleep, spawn},
    time::Duration,
};

use serial_test::serial;

use crate::{
    deps::{
        hreads::{invoke_dispatched, set_current_thread_as_main},
        refs::manage::{DataManager, ResourceLoader},
    },
    managed,
};

struct Res {
    data: Vec<u8>,
}

impl ResourceLoader for Res {
    fn load_path(_path: &Path) -> Self {
        Self { data: vec![] }
    }

    fn load_data(data: &[u8], _name: impl ToString) -> Self {
        Self { data: data.to_vec() }
    }
}

managed!(crate::deps::refs, Res);

fn start_server(hits: Arc<AtomicUsize>, respond: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                return;
            };

            hits.fetch_add(1, Ordering::SeqCst);

            if !respond {
                continue;
            }

            let mut buf = [0; 1024];
            let _read = stream.read(&mut buf).unwrap();

            // Delay so concurrent download calls pile up on one in
            // flight request instead of finishing one by one.
            sleep(Duration::from_millis(300));

            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
                .unwrap();
        }
    });

    addr
}

#[test]
#[serial]
fn concurrent_download_fetches_once() {
    set_current_thread_as_main();

    let hits = Arc::new(AtomicUsize::new(0));
    let addr = start_server(hits.clone(), true);
    let url = format!("http://{addr}/image");

    let rt = tokio::runtime::Runtime::new().unwrap();

    let tasks: Vec<_> = (0..16)
        .map(|_| {
            let url = url.clone();
            rt.spawn(async move { Res::download("shared-image", &url).await })
        })
        .collect();

    let mut weaks = vec![];

    for task in tasks {
        weaks.push(rt.block_on(task).unwrap().unwrap());
    }

    assert_eq!(hits.load(Ordering::SeqCst), 1);

    let first = weaks[0].addr();
    assert!(weaks.iter().all(|weak| weak.addr() == first));
    assert_eq!(weaks[0].data, b"hello");
}

#[test]
#[serial]
fn failed_download_fails_waiters_too() {
    set_current_thread_as_main();

    let hits = Arc::new(AtomicUsize::new(0));
    let addr = start_server(hits, false);
    let url = format!("http://{addr}/broken");

    let rt = tokio::runtime::Runtime::new().unwrap();

    let tasks: Vec<_> = (0..16)
        .map(|_| {
            let url = url.clone();
            rt.spawn(async move { Res::download("broken-image", &url).await })
        })
        .collect();

    for task in tasks {
        assert!(rt.block_on(task).unwrap().is_err());
    }

    assert!(Res::get_existing("broken-image").is_none());
    assert!(Res::in_flight_downloads().lock().is_empty());
}

#[test]
#[serial]
fn concurrent_load_drops_losers_on_main() {
    set_current_thread_as_main();

    let threads: Vec<_> = (0..8)
        .map(|_| {
            spawn(|| {
                for i in 0..100 {
                    Res::load(b"data", format!("race-{i}"));
                }
            })
        })
        .collect();

    // Losing duplicates are dispatched here to die. Without pumping
    // they would leak, with the old code they panicked the workers.
    while threads.iter().any(|thread| !thread.is_finished()) {
        invoke_dispatched();
    }

    invoke_dispatched();

    for thread in threads {
        thread.join().unwrap();
    }

    let storage = Res::storage();
    assert_eq!(storage.keys().filter(|key| key.starts_with("race-")).count(), 100);
}
