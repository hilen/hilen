use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    system::{Links, OpenedLink},
    ui::{Label, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{checkpoint, open_link},
};

/// How many links the listeners showed, read from the test thread.
static SHOWN: AtomicUsize = AtomicUsize::new(0);

/// A view that listens for links the way an app does, in the view that
/// decides which screen a link opens, and shows the last one.
#[view]
struct LinkListener {
    #[init]
    name: Label,
    path: Label,
    url:  Label,
}

impl Setup for LinkListener {
    fn setup(self: Weak<Self>) {
        self.set_color("#D6EAF8");
        self.name.set_text_size(18);
        self.name.place().t(8).lr(12).h(28);
        self.path.set_text("path: none");
        self.path.set_text_size(26);
        self.path.place().t(44).lr(12).h(40);
        self.url.set_text("url: none");
        self.url.set_text_size(16);
        self.url.place().t(92).lr(12).h(28);

        Links::on_open(self, move |link: OpenedLink| {
            self.path.set_text(format!("path: {}", link.path()));
            self.url.set_text(format!("url: {}", link.url));
            SHOWN.fetch_add(1, Ordering::Relaxed);
        });
    }
}

/// A link that opens the app reaches the view that listens, while the app
/// runs and when it comes before anybody listens, like the link of a cold
/// start. The test hands the links in through `open_link`, the same path a
/// platform uses.
#[view]
struct OpenedLinkTest {
    #[init]
    status: Label,
}

impl Setup for OpenedLinkTest {
    fn setup(self: Weak<Self>) {
        SHOWN.store(0, Ordering::Relaxed);
        self.status.set_text_size(20);
        self.status.place().t(48).lr(20).h(40);
    }
}

fn add_listener(view: Weak<OpenedLinkTest>, name: &'static str) -> Weak<LinkListener> {
    from_main(move || {
        let listener = view.add_view::<LinkListener>();
        listener.name.set_text(name);
        listener.place().t(100).lr(20).h(130);
        listener
    })
}

fn set_status(view: Weak<OpenedLinkTest>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

fn path_of(listener: Weak<LinkListener>) -> String {
    from_main(move || listener.path.text().to_string())
}

fn wait_shown(count: usize) -> Result<()> {
    for _ in 0..600 {
        if SHOWN.load(Ordering::Relaxed) >= count {
            return Ok(());
        }
        wait_for_next_frame();
    }
    bail!(
        "{count} links were not shown, only {}",
        SHOWN.load(Ordering::Relaxed)
    )
}

fn wait_frames(count: usize) {
    for _ in 0..count {
        wait_for_next_frame();
    }
}

impl ViewTest for OpenedLinkTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let first = add_listener(view, "listener of the running app");

        set_status(view, "a phone link while the app runs");
        open_link("https://shopping.vladas.xyz/invite/x7k2");
        wait_shown(1)?;
        assert_eq!(path_of(first), "path: invite/x7k2");
        checkpoint("the listener shows the phone link, path invite/x7k2")?;

        set_status(view, "a desktop link of the url scheme");
        open_link("shopping://invite/second");
        wait_shown(2)?;
        assert_eq!(path_of(first), "path: invite/second");
        checkpoint("the scheme link has the same kind of path, invite/second")?;

        let mut gone = first;
        from_main(move || gone.remove_from_superview());
        set_status(view, "nobody listens, the link waits");
        open_link("https://shopping.vladas.xyz/invite/cold");
        wait_frames(10);
        ensure!(
            SHOWN.load(Ordering::Relaxed) == 2,
            "a link with no listener went somewhere"
        );
        checkpoint("no listener, the link of the cold start waits")?;

        let late = add_listener(view, "listener of the first screen");
        set_status(view, "the first listener gets the waiting link");
        wait_shown(3)?;
        assert_eq!(path_of(late), "path: invite/cold");

        // The waiting link is given once.
        wait_frames(10);
        ensure!(
            SHOWN.load(Ordering::Relaxed) == 3,
            "the waiting link was given twice"
        );
        checkpoint("the new listener shows the waiting link, path invite/cold")?;

        Ok(())
    }
}
