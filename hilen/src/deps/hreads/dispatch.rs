use std::{
    mem::take,
    sync::mpsc::{Receiver, channel},
};

use anyhow::Result;
// The main thread drains the queue every frame while workers push into
// it. In the browser the main thread must never park, a contended
// parking lock there raises an Atomics.wait error, so wasm spins. The
// critical sections are single vec operations.
#[cfg(not_wasm)]
use parking_lot::Mutex;
#[cfg(wasm)]
use spin::Mutex;
#[cfg(not_wasm)]
use tokio::{
    runtime::{Handle, RuntimeFlavor},
    task::block_in_place,
};

use crate::deps::hreads::main_thread::is_main_thread;

type Callback = Box<dyn FnOnce() + Send>;
type Callbacks = Mutex<Vec<Callback>>;

static CALLBACKS: Callbacks = Callbacks::new(vec![]);

type Waker = Box<dyn Fn() + Send + Sync>;
static WAKER: Mutex<Option<Waker>> = Mutex::new(None);

/// Register a callback that fires whenever work is queued for the main thread
/// from another thread. An event driven main loop uses this to wake from sleep
/// and drain the queue, instead of polling every frame just in case.
pub fn set_dispatch_waker(waker: impl Fn() + Send + Sync + 'static) {
    *WAKER.lock() = Some(Box::new(waker));
}

fn wake_main() {
    if let Some(waker) = WAKER.lock().as_ref() {
        waker();
    }
}

pub fn from_main<T, A>(action: A) -> T
where
    A: FnOnce() -> T + Send + 'static,
    T: Send + 'static, {
    if is_main_thread() {
        return action();
    }

    let (se, re) = channel::<T>();

    on_main(move || {
        se.send(action()).expect("Failed to send result of from_main");
    });

    recv_dispatched(&re).expect("Failed to receive result in from_main")
}

/// Receive without starving tokio. A blocked multithread worker hands its
/// queued tasks to other workers before parking.
fn recv_dispatched<T>(re: &Receiver<T>) -> Option<T> {
    #[cfg(not_wasm)]
    {
        let on_multithread_worker =
            Handle::try_current().is_ok_and(|handle| handle.runtime_flavor() == RuntimeFlavor::MultiThread);

        if on_multithread_worker {
            return block_in_place(|| re.recv()).ok();
        }
    }

    re.recv().ok()
}

pub fn wait_async<T, A>(action: A) -> T
where
    A: Future<Output = T> + Send + 'static,
    T: Send + 'static, {
    assert!(!is_main_thread(), "wait_async on the main thread can deadlock");

    let (se, re) = channel::<T>();

    crate::deps::hreads::spawn(async move {
        se.send(action.await).expect("Failed to send result of wait_async");
    });

    re.recv().expect("Failed to receive result in wait_async")
}

pub fn wait_for_next_frame() {
    assert!(
        !is_main_thread(),
        "Waiting for next frame on main thread does nothing"
    );
    from_main(|| {});
}

pub fn on_main(action: impl FnOnce() + Send + 'static) {
    if is_main_thread() {
        action();
    } else {
        CALLBACKS.lock().push(Box::new(action));
        wake_main();
    }
}

pub fn ok_main(action: impl FnOnce() + Send + 'static) -> Result<()> {
    on_main(action);
    Ok(())
}

pub fn after(delay: f32, action: impl FnOnce() + Send + 'static) {
    crate::deps::hreads::spawn(async move {
        crate::deps::hreads::sleep(delay).await;
        CALLBACKS.lock().push(Box::new(action));
        wake_main();
    });
}

pub fn invoke_dispatched() {
    let actions = take(&mut *CALLBACKS.lock());

    for action in actions {
        action();
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use serial_test::serial;
    use tokio::runtime::Runtime;

    use crate::deps::hreads::{
        from_main, invoke_dispatched, on_main, set_current_thread_as_main, set_dispatch_waker,
    };

    #[test]
    #[serial]
    fn from_main_on_tokio_worker() {
        set_current_thread_as_main();

        let rt = Runtime::new().unwrap();
        let handle = rt.spawn(async { from_main(|| 5) });

        while !handle.is_finished() {
            invoke_dispatched();
        }

        assert_eq!(rt.block_on(handle).unwrap(), 5);
    }

    #[test]
    #[serial]
    fn waker_fires_on_background_enqueue() {
        set_current_thread_as_main();

        let woken = Arc::new(AtomicUsize::new(0));
        let woken_in_waker = woken.clone();
        set_dispatch_waker(move || {
            woken_in_waker.fetch_add(1, Ordering::Relaxed);
        });

        // From the main thread the action runs inline, so nothing to wake.
        on_main(|| {});
        assert_eq!(woken.load(Ordering::Relaxed), 0);

        // From another thread it is queued, and the waker fires once.
        let handle = std::thread::spawn(|| {
            on_main(|| {});
        });
        handle.join().unwrap();
        assert_eq!(woken.load(Ordering::Relaxed), 1);

        invoke_dispatched();
        set_dispatch_waker(|| {});
    }
}
