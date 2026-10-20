use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{Label, Setup, Tab, TextAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// A color per page, so the open page shows in the pictures.
const PAGE_COLORS: [&str; 3] = ["#F4D6D6", "#D6F4DA", "#D6DEF4"];

/// The first tab is open and marked.
const FIRST_TAB: &str = r"
   4    4 - #f4d6d6
 280    4 - #f4d6d6
 592    4 - #f4d6d6
 340  264 - #000000
 260  272 - #f4d6d6
 300  272 - #000000
 344  272 - #000000
 280  276 - #252121
 312  276 - #625656
 316  276 - #625656
 332  276 - #000000
 260  280 - #f4d6d6
 276  280 - #f4d6d6
 296  280 - #f4d6d6
 308  280 - #000000
 344  280 - #000000
 284  284 - #010101
 300  284 - #000000
 256  288 - #020202
 296  288 - #f4d6d6
 592  300 - #f4d6d6
   4  344 - #f4d6d6
   4  552 - #f9f9f9
 188  552 - #f9f9f9
 384  552 - #f9f9f9
 592  552 - #f9f9f9
 296  564 - #f9f9f9
 492  568 - #f9f9f9
  92  588 - #f9f9f9
 284  588 - #bcbcbf
 484  592 - #f9f9f9
 512  592 - #f9f9f9
";

/// The third tab is open and marked after its tap.
const THIRD_TAB: &str = r"
   4    4 - #d6def4
 272    4 - #d6def4
 592    4 - #d6def4
 592  256 - #d6def4
   4  260 - #d6def4
 340  264 - #010101
 260  272 - #d6def4
 296  272 - #d6def4
 256  276 - #000000
 280  276 - #212225
 312  276 - #565962
 316  276 - #565962
 276  280 - #d6def4
 296  280 - #d6def4
 308  280 - #000000
 264  284 - #000000
 284  284 - #010101
 300  284 - #000000
 336  284 - #060607
 344  284 - #060607
 256  288 - #020202
 296  288 - #d6def4
   4  552 - #f9f9f9
 188  552 - #f9f9f9
 400  552 - #f9f9f9
 592  552 - #f9f9f9
 296  564 - #f9f9f9
 492  568 - #f9f9f9
  92  588 - #f9f9f9
 284  588 - #bcbcbf
 484  592 - #f9f9f9
 512  592 - #f9f9f9
";

/// A detail page in place of the second tab, which stays marked.
const DETAIL_PAGE: &str = r"
   4    4 - #f4eed6
 288    4 - #f4eed6
 592    4 - #f4eed6
 204  264 - #000000
 264  264 - #000000
 300  268 - #000000
 344  272 - #000000
 364  272 - #1e1d1a
 400  272 - #7a776b
 192  276 - #010101
 216  276 - #625f56
 228  276 - #393832
 256  276 - #000000
 328  276 - #010101
 376  276 - #625f56
 400  276 - #7a776b
 228  280 - #393832
 288  280 - #f4eed6
 364  280 - #1e1d1a
 400  280 - #7a776b
 240  284 - #010101
 336  284 - #000000
 364  284 - #1e1d1a
 400  284 - #7a776b
 316  288 - #000000
 592  380 - #f4eed6
   4  388 - #f4eed6
 296  564 - #f9f9f9
 492  568 - #f9f9f9
  92  588 - #f9f9f9
 284  588 - #6ab0fc
 512  592 - #f9f9f9
";

/// A tap on a tab opens its page and marks the tab, the page before is gone,
/// and the shell tells which tab opened. A detail page keeps the mark.
#[view]
struct TabShellTest {
    built:  Vec<usize>,
    opened: Vec<usize>,

    #[init]
    shell: hilen::ui::TabShell,
}

impl Setup for TabShellTest {
    fn setup(self: Weak<Self>) {
        self.shell.place().back();
        self.shell.set_tabs(
            vec![
                Tab::new("Lists", "tab_lists.svg"),
                Tab::new("Friends", "tab_friends.svg"),
                Tab::new("Account", "tab_account.svg"),
            ],
            move |index, content| {
                let mut test = self;
                test.built.push(index);
                let page = content.add_view::<Label>();
                page.set_text(format!("page {index}"))
                    .set_alignment(TextAlignment::Center)
                    .set_color(Color::hex(PAGE_COLORS[index]));
                page
            },
        );
        self.shell.changed.val(move |index| {
            let mut test = self;
            test.opened.push(index);
        });
        self.shell.select(0);
    }
}

impl ViewTest for TabShellTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        open(view, 0)?;
        check_colors(FIRST_TAB)?;

        tap_tab(view, 2);
        open(view, 2)?;
        check_colors(THIRD_TAB)?;

        tap_tab(view, 1);
        open(view, 1)?;

        from_main(move || {
            let page = view.shell.content().add_view::<Label>();
            page.set_text("detail of page 1")
                .set_alignment(TextAlignment::Center)
                .set_color(Color::hex("#F4EED6"));
            view.shell.show_page(page);
        });
        wait_for_next_frame();
        let (selected, pages) =
            from_main(move || (view.shell.selected(), view.shell.content().subviews().len()));
        ensure!(selected == 1, "a detail page moved the mark to tab {selected}");
        ensure!(pages == 1, "{pages} pages are in the shell after the detail page");
        check_colors(DETAIL_PAGE)?;

        let (built, opened) = from_main(move || (view.built.clone(), view.opened.clone()));
        ensure!(built == [0, 2, 1], "the pages built were {built:?}");
        ensure!(opened == [0, 2, 1], "the tabs opened were {opened:?}");
        Ok(())
    }
}

/// The center of a tab, from the frame of the bar, the bar is in points and
/// the canvas in pixels.
fn tap_tab(view: Weak<TabShellTest>, index: u8) {
    let bar = from_main(move || *view.shell.bar().absolute_frame());
    let x = bar.x() + bar.width() * (f32::from(index) + 0.5) / 3.0;
    let y = bar.y() + bar.height() / 2.0;
    inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
}

fn open(view: Weak<TabShellTest>, index: usize) -> Result<()> {
    wait_for_next_frame();
    let (selected, pages) = from_main(move || (view.shell.selected(), view.shell.content().subviews().len()));
    ensure!(selected == index, "tab {selected} is open, not tab {index}");
    ensure!(
        pages == 1,
        "{pages} pages are in the shell, the page before stayed"
    );
    Ok(())
}
