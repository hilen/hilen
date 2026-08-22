use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Anchor, BLACK, Setup, ViewData, ViewTest, ViewTouch, WHITE, view},
    ui_test::{helpers::check_colors, inject_touches},
};

#[view]
struct Selectable {}

impl Setup for Selectable {
    fn setup(self: Weak<Self>) {
        self.enable_touch();
        self.set_color(BLACK);
    }

    fn on_selection_changed(self: Weak<Self>, selected: bool) {
        self.set_color(if selected { WHITE } else { BLACK });
    }
}

#[view]
struct Selection {
    #[init]
    a: Selectable,
    b: Selectable,
    c: Selectable,
}

impl Setup for Selection {
    fn setup(self: Weak<Self>) {
        self.a.place().size(100, 100).center();
        self.b.place().same_size(self.a).center_y().anchor(Anchor::Right, self.a, 40);
        self.c.place().same_size(self.a).center_y().anchor(Anchor::Left, self.a, 40);
    }
}

impl ViewTest for Selection {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_none_selected()?;
        check_left_selected()?;
        check_center_selected()?;
        check_right_selected()?;
        check_deselected_outside()?;

        Ok(())
    }
}

fn check_none_selected() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             424   72 - #597c95
             132  104 - #597c95
             592  188 - #597c95
               4  192 - #597c95
             300  248 - #597c95
             180  252 - #000000
             420  252 - #000000
             112  276 - #000000
             348  276 - #000000
             488  276 - #000000
             252  288 - #000000
             392  288 - #000000
             304  304 - #000000
             140  312 - #000000
             184  324 - #000000
             420  324 - #000000
             348  332 - #000000
             112  348 - #000000
             252  348 - #000000
             300  348 - #000000
             488  348 - #000000
               4  444 - #597c95
             592  448 - #597c95
             316  464 - #597c95
             148  508 - #597c95
             440  512 - #597c95
               4  592 - #597c95
             288  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_left_selected() -> Result<()> {
    inject_touches(
        r"
            128  274  b
            128  274  e
    ",
    );

    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             424   72 - #597c95
               4  160 - #597c95
             300  248 - #597c95
             140  252 - #ffffff
             180  252 - #ffffff
             420  252 - #000000
             256  272 - #000000
             112  276 - #ffffff
             348  276 - #000000
             488  276 - #000000
             168  288 - #ffffff
             208  288 - #ffffff
             392  288 - #000000
             444  288 - #000000
             296  308 - #000000
             140  312 - #ffffff
             184  324 - #ffffff
             420  324 - #000000
             348  332 - #000000
             112  348 - #ffffff
             152  348 - #ffffff
             208  348 - #ffffff
             252  348 - #000000
             488  348 - #000000
             444  508 - #597c95
             152  516 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_center_selected() -> Result<()> {
    inject_touches(
        r"
            260  260  b
            260  260  e
    ",
    );

    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             420   72 - #597c95
             128  104 - #597c95
             588  160 - #597c95
             300  248 - #597c95
             180  252 - #000000
             260  252 - #ffffff
             428  252 - #000000
             112  276 - #000000
             284  276 - #ffffff
             316  276 - #ffffff
             348  276 - #ffffff
             488  280 - #000000
             208  288 - #000000
             252  288 - #ffffff
             392  288 - #000000
             304  304 - #ffffff
             140  312 - #000000
             272  320 - #ffffff
             184  324 - #000000
             420  324 - #000000
             348  332 - #ffffff
             112  348 - #000000
             252  348 - #ffffff
             300  348 - #ffffff
             488  348 - #000000
             168  520 - #597c95
               4  592 - #597c95
             332  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_right_selected() -> Result<()> {
    inject_touches(
        r"
            420  260  b
            420  260  e
    ",
    );

    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             424   72 - #597c95
             128  104 - #597c95
             300  248 - #597c95
             180  252 - #000000
             420  252 - #ffffff
             460  252 - #ffffff
             348  264 - #000000
             112  276 - #000000
             440  276 - #ffffff
             488  276 - #ffffff
             208  288 - #000000
             252  288 - #000000
             296  288 - #000000
             392  292 - #ffffff
             440  308 - #ffffff
             140  312 - #000000
             480  312 - #ffffff
             320  320 - #000000
             184  324 - #000000
             392  344 - #ffffff
             112  348 - #000000
             252  348 - #000000
             432  348 - #ffffff
             488  348 - #ffffff
             152  512 - #597c95
             448  512 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_deselected_outside() -> Result<()> {
    inject_touches(
        r"
              5    5  b
    ",
    );

    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             424   72 - #597c95
             132  104 - #597c95
             592  188 - #597c95
               4  192 - #597c95
             300  248 - #597c95
             180  252 - #000000
             420  252 - #000000
             112  276 - #000000
             348  276 - #000000
             488  276 - #000000
             252  288 - #000000
             392  288 - #000000
             304  304 - #000000
             140  312 - #000000
             184  324 - #000000
             420  324 - #000000
             348  332 - #000000
             112  348 - #000000
             252  348 - #000000
             300  348 - #000000
             488  348 - #000000
               4  444 - #597c95
             592  448 - #597c95
             316  464 - #597c95
             148  508 - #597c95
             440  512 - #597c95
               4  592 - #597c95
             288  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
