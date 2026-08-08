use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    system::Router,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::human_checkpoint,
};

/// The subscription pushes into the view, this only makes the arrival
/// cheap to spin on from the test thread.
static POP_COUNT: AtomicUsize = AtomicUsize::new(0);

/// The page's on screen name, the path the way an app titles a page.
fn page_name(path: &str) -> String {
    format!("/{path}")
}

#[view]
struct RouterTest {
    popped: Vec<String>,

    #[init]
    page: Label,
}

impl Setup for RouterTest {
    fn setup(self: Weak<Self>) {
        POP_COUNT.store(0, Ordering::Relaxed);

        self.page.set_text_size(28);
        self.page.place().center().size(500, 60);

        if let Some(path) = Router::current_path() {
            self.page.set_text(page_name(&path));
        }

        Router::on_pop().val(self, move |path| {
            let mut this = self;
            this.page.set_text(page_name(&path));
            this.popped.push(path);
            POP_COUNT.fetch_add(1, Ordering::Relaxed);
        });
    }
}

fn history() -> web_sys::History {
    web_sys::window()
        .expect("Failed to get browser window")
        .history()
        .expect("Failed to get browser history")
}

fn wait_for_pops(count: usize) {
    while POP_COUNT.load(Ordering::Relaxed) < count {
        std::hint::spin_loop();
    }
}

impl ViewTest for RouterTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let original = from_main(Router::current_path).expect("No path in a browser");

        // A push does not fire `on_pop`, so the pushing side sets the
        // page itself, the way an app opens a page and then records it.
        from_main(move || {
            Router::push("router-test/first");
            view.page.set_text(page_name("router-test/first"));
        });
        assert_eq!(
            from_main(Router::current_path).as_deref(),
            Some("router-test/first")
        );
        assert_eq!(
            from_main(move || view.page.text().to_string()),
            "/router-test/first"
        );
        human_checkpoint("pushed router-test/first");

        from_main(move || {
            Router::push("router-test/second");
            view.page.set_text(page_name("router-test/second"));
        });
        assert_eq!(
            from_main(Router::current_path).as_deref(),
            Some("router-test/second")
        );
        assert_eq!(
            from_main(move || view.page.text().to_string()),
            "/router-test/second"
        );
        human_checkpoint("pushed router-test/second");

        // A push must not fire the pop event, only the browser walking
        // history does.
        assert_eq!(from_main(move || view.popped.clone()), Vec::<String>::new());

        from_main(|| history().back().expect("Failed to go back"));
        wait_for_pops(1);

        assert_eq!(from_main(move || view.popped.clone()), ["router-test/first"]);
        assert_eq!(
            from_main(Router::current_path).as_deref(),
            Some("router-test/first")
        );
        // The pop handler drives the label here, this is the real check
        // that browser history updates the page.
        assert_eq!(
            from_main(move || view.page.text().to_string()),
            "/router-test/first"
        );
        human_checkpoint("back to router-test/first");

        from_main(|| history().forward().expect("Failed to go forward"));
        wait_for_pops(2);

        assert_eq!(
            from_main(move || view.popped.clone()),
            ["router-test/first", "router-test/second"]
        );
        assert_eq!(
            from_main(Router::current_path).as_deref(),
            Some("router-test/second")
        );
        assert_eq!(
            from_main(move || view.page.text().to_string()),
            "/router-test/second"
        );
        human_checkpoint("forward to router-test/second");

        // Leave the URL where the page started so the rest of the suite
        // runs on the address it was served at.
        let restored = original.clone();
        from_main(move || {
            Router::replace(&restored);
            view.page.set_text(page_name(&restored));
        });
        assert_eq!(
            from_main(move || view.page.text().to_string()),
            page_name(&original)
        );
        human_checkpoint("replaced with the original path");

        Ok(())
    }
}
