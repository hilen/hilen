use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::{Own, Weak},
    ui::{
        Button, Container, DynamicColor, Label, NavigationView, Setup, TextAlignment, View, ViewController,
        ViewData, ViewSubviews, ViewTest, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#F7F7FA"), Color::hex("#1C1C1E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const CONTENT: DynamicColor = DynamicColor::new(Color::hex("#D7F0E3"), Color::hex("#1D4D38"));
const BACK: DynamicColor = DynamicColor::new(Color::hex("#4361EE"), Color::hex("#5B78F2"));
const WHITE: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#FFFFFF"));

/// The first page of the stack. A press on Back of the pushed page comes
/// back here.
#[view]
struct NavigationBackOverContentTest {
    #[init]
    title: Label,
}

impl Setup for NavigationBackOverContentTest {
    fn setup(self: Weak<Self>) {
        self.set_color(PAGE);
        self.title
            .set_text("first page")
            .set_text_size(24)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Center);
        self.title.place().center().size(400, 40);
    }
}

/// A pushed page that asks for its Back button first and builds its content
/// after it, like a page that fills itself again when its data changes. The
/// content is 1 button over the whole page, so it lies under Back and was
/// added after it.
#[view]
struct ContentAfterBack {
    taps: usize,

    #[init]
    content: Container,
    status:  Label,
}

impl Setup for ContentAfterBack {
    fn setup(self: Weak<Self>) {
        self.set_color(PAGE);
        self.content.place().back();

        let back = self.back_button();
        back.set_color(BACK).set_corner_radius(8);
        back.place().t(60).l(20).size(180, 44);
        let name = back.add_view::<Label>();
        name.set_text("Back button")
            .set_text_size(16)
            .set_text_color(WHITE)
            .set_alignment(TextAlignment::Center);
        name.place().back();

        let button = self.content.add_view::<Button>();
        button.set_text("content button, added after Back").set_color(CONTENT);
        button.set_text_color(TEXT);
        button.place().back();
        button.on_tap(move || {
            let mut page = self;
            page.taps += 1;
            page.status.set_text(format!("the content button took {} press", page.taps));
        });

        self.status
            .set_text("pushed page, Back lies over the content button")
            .set_text_size(16)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Center);
        self.status.place().b(40).lr(12).h(30);
    }
}

impl ViewTest for NavigationBackOverContentTest {
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let page = from_main(move || {
            let page = ContentAfterBack::new();
            let weak = page.weak();
            view.navigation().push(page);
            weak
        });
        settle();
        checkpoint("pushed page, the blue Back button lies over the green content button")?;

        inject_touches("300 400 b\n300 400 e");
        settle();
        from_main(move || {
            ensure!(page.is_ok(), "a press on the content left the page open");
            ensure!(
                page.taps == 1,
                "the content button took the press next to Back, it counts {}",
                page.taps
            );
            Ok(())
        })?;
        checkpoint("the content button took a press next to Back")?;

        inject_touches("110 82 b\n110 82 e");
        settle();
        from_main(move || {
            ensure!(!page.is_ok(), "a press on Back closed the pushed page");
            ensure!(!view.is_hidden(), "the first page shows again");
            Ok(())
        })?;
        checkpoint("a press on Back went back to the first page")?;

        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}
