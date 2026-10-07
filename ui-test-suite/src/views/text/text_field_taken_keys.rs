use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Label, ModifiersState, NamedKey, Setup, TextAlignment, TextField, UIManager, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key, set_record_probe_count},
};

const ROWS: [&str; 3] = ["/help", "/clear", "/model"];
const LIST_KEYS: [NamedKey; 5] = [
    NamedKey::ArrowUp,
    NamedKey::ArrowDown,
    NamedKey::Tab,
    NamedKey::Enter,
    NamedKey::Escape,
];

/// A list of commands over a text field, the way a chat shows it. While the
/// list is open the owner takes the arrows, Tab, Enter and Escape with
/// `take_keys`: they move the mark, pick a row and close the list, and the
/// field does nothing with them. While the list is closed the same keys
/// work as in every field.
#[view]
struct TextFieldTakenKeys {
    /// The field the open list belongs to.
    owner:          Weak<TextField>,
    marked:         usize,
    taken:          usize,
    picked:         Vec<&'static str>,
    submits:        Vec<String>,
    ended:          usize,
    screen_escapes: usize,

    #[init]
    list_title: Label,
    row_0:      Label,
    row_1:      Label,
    row_2:      Label,
    chat_title: Label,
    chat:       TextField,
    name_title: Label,
    name:       TextField,
    next_title: Label,
    next:       TextField,
    log:        Label,
}

impl Setup for TextFieldTakenKeys {
    fn setup(mut self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(24);
        };

        title(self.list_title, "", 8.0);
        for (index, row) in self.rows().into_iter().enumerate() {
            row.set_text(ROWS[index]).set_text_size(18);
            row.set_alignment(TextAlignment::Left);
            let y = 36 + index * 30;
            row.place().t(y).lr(20).h(28);
        }

        title(self.chat_title, "chat box, Enter sends", 132.0);
        self.chat.set_multiline(true).set_submit_on_enter(true).set_text_size(20);
        self.chat.set_alignment(TextAlignment::Left);
        self.chat.place().t(158).lr(20).h(70);

        title(self.name_title, "single line field, Enter submits", 236.0);
        self.name.set_text_size(20);
        self.name.set_alignment(TextAlignment::Left);
        self.name.place().t(262).lr(20).h(40);

        title(self.next_title, "next field, Tab lands here", 310.0);
        self.next.set_text_size(20);
        self.next.set_alignment(TextAlignment::Left);
        self.next.place().t(336).lr(20).h(40);

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(388).lr(20).h(200);

        for field in [self.chat, self.name] {
            field.key_taken.val(move |key| self.list_key(key));
            field.submitted.val(move |text| self.note_submit(text));
            field.editing_ended.val(move |_| self.note_ended());
        }

        // What a screen binds to leave itself. It must stay silent for an
        // Escape the open list takes.
        UIManager::keymap().add(self, NamedKey::Escape, move || self.note_screen_escape());

        self.show();
    }
}

impl TextFieldTakenKeys {
    fn rows(self: Weak<Self>) -> [Weak<Label>; 3] {
        [self.row_0, self.row_1, self.row_2]
    }

    fn open_list(mut self: Weak<Self>, field: Weak<TextField>) {
        self.owner = field;
        self.marked = 0;
        field.take_keys(LIST_KEYS);
        self.show();
    }

    fn close_list(mut self: Weak<Self>) {
        self.owner.release_keys();
        self.owner = Weak::default();
        self.show();
    }

    fn list_key(mut self: Weak<Self>, key: NamedKey) {
        self.taken += 1;
        match key {
            NamedKey::ArrowUp => self.marked = (self.marked + ROWS.len() - 1) % ROWS.len(),
            NamedKey::ArrowDown => self.marked = (self.marked + 1) % ROWS.len(),
            NamedKey::Tab | NamedKey::Enter => {
                let row = ROWS[self.marked];
                self.picked.push(row);
                self.owner.set_text(row);
                self.close_list();
            }
            _ => self.close_list(),
        }
        self.show();
    }

    fn note_submit(mut self: Weak<Self>, text: String) {
        self.submits.push(text);
        self.show();
    }

    fn note_ended(mut self: Weak<Self>) {
        self.ended += 1;
        self.show();
    }

    fn note_screen_escape(mut self: Weak<Self>) {
        self.screen_escapes += 1;
        self.show();
    }

    fn show(self: Weak<Self>) {
        let open = self.owner.is_ok();
        self.list_title.set_text(if open {
            "list is open, it takes the keys"
        } else {
            "list is closed, the field has the keys"
        });
        for (index, row) in self.rows().into_iter().enumerate() {
            row.set_hidden(!open);
            row.set_color(if index == self.marked {
                "#228CDB"
            } else {
                "#DDE3E8"
            });
        }
        self.log.set_text(format!(
            "keys taken: {}\npicked: {}\nsubmitted: {}\ntimes editing ended: {}\nscreen Escape ran: {}",
            self.taken,
            self.picked.join(", "),
            self.submits.join(", "),
            self.ended,
            self.screen_escapes,
        ));
    }
}

fn press(keys: &[NamedKey]) {
    for key in keys {
        inject_named_key(*key);
    }
    wait_for_next_frame();
}

/// The keys of a chat box before the list opens, while it is open, and a
/// pick with Tab.
fn chat_box_list(view: Weak<TextFieldTakenKeys>) -> Result<()> {
    from_main(move || view.chat.focus());
    inject_keys("one");
    inject_modifiers(ModifiersState::SHIFT);
    inject_named_key(NamedKey::Enter);
    inject_modifiers(ModifiersState::empty());
    inject_keys("two");
    press(&[NamedKey::ArrowUp]);
    inject_keys("X");
    wait_for_next_frame();
    from_main(move || {
        ensure!(
            view.chat.text() == "oneX\ntwo",
            "the up arrow of a closed list: {:?}",
            view.chat.text()
        );
        ensure!(view.taken == 0, "a key was taken with no list open");
        Ok(())
    })?;
    // list closed: the up arrow moved the caret, X ends line one
    check_colors(CLOSED)?;

    from_main(move || view.open_list(view.chat));
    press(&[NamedKey::ArrowDown, NamedKey::ArrowDown, NamedKey::ArrowUp]);
    inject_keys("Y");
    wait_for_next_frame();
    from_main(move || {
        ensure!(view.marked == 1, "the mark is on row {}", view.marked);
        ensure!(view.taken == 3, "{} keys taken, not 3", view.taken);
        ensure!(
            view.chat.text() == "oneXY\ntwo",
            "a taken arrow moved the caret: {:?}",
            view.chat.text()
        );
        Ok(())
    })?;
    // list open, /clear is marked, Y landed right after X
    check_colors(OPEN_MARKED)?;

    press(&[NamedKey::Tab]);
    from_main(move || {
        ensure!(view.picked == ["/clear"], "Tab picked {:?}", view.picked);
        ensure!(view.chat.text() == "/clear", "chat holds {:?}", view.chat.text());
        ensure!(view.chat.is_editing(), "a taken Tab ended the editing");
        ensure!(
            !view.name.is_editing() && !view.next.is_editing(),
            "a taken Tab selected another field"
        );
        ensure!(view.owner.is_null(), "the list stayed open after a pick");
        Ok(())
    })?;
    // Tab picked /clear into the chat box, the list is closed, the chat box
    // is still edited. The block has no probe on the caret, it is 1 pixel
    // wide on a part of a column and differs with 1 sample.
    check_colors(TAB_PICKED)
}

/// A pick with Enter and a close with Escape in the chat box, then the
/// same 2 keys with the list closed.
fn chat_box_pick_and_close(view: Weak<TextFieldTakenKeys>) -> Result<()> {
    from_main(move || view.open_list(view.chat));
    press(&[NamedKey::Enter]);
    from_main(move || {
        ensure!(
            view.picked == ["/clear", "/help"],
            "Enter picked {:?}",
            view.picked
        );
        ensure!(view.submits.is_empty(), "a taken Enter sent {:?}", view.submits);
        ensure!(view.chat.text() == "/help", "chat holds {:?}", view.chat.text());
        ensure!(view.chat.is_editing(), "a taken Enter ended the editing");
        Ok(())
    })?;

    from_main(move || view.open_list(view.chat));
    press(&[NamedKey::Escape]);
    from_main(move || {
        ensure!(view.owner.is_null(), "Escape left the list open");
        ensure!(view.chat.is_editing(), "a taken Escape ended the editing");
        ensure!(view.ended == 0, "editing ended {} times", view.ended);
        ensure!(
            view.screen_escapes == 0,
            "a taken Escape also ran the Escape of the screen"
        );
        ensure!(view.taken == 6, "{} keys taken, not 6", view.taken);
        Ok(())
    })?;
    // Enter picked /help and sent nothing, Escape closed the list, the chat
    // box is still edited and holds /help
    check_colors(ESCAPE_CLOSED)?;

    press(&[NamedKey::Enter]);
    from_main(move || {
        ensure!(
            view.submits == ["/help"],
            "Enter of a closed list sent {:?}",
            view.submits
        );
        Ok(())
    })?;
    press(&[NamedKey::Escape]);
    from_main(move || {
        ensure!(
            !view.chat.is_editing(),
            "Escape of a closed list kept the editing"
        );
        ensure!(view.ended == 1, "editing ended {} times", view.ended);
        ensure!(
            view.screen_escapes == 1,
            "the Escape of the screen ran {} times for a key nobody took",
            view.screen_escapes
        );
        ensure!(view.taken == 6, "a key was taken with no list open");
        Ok(())
    })?;
    // list closed: Enter sent /help, Escape ended the chat box editing
    check_colors(RELEASED)
}

/// The same list over a single line field.
fn single_line_list(view: Weak<TextFieldTakenKeys>) -> Result<()> {
    from_main(move || {
        view.name.focus();
        view.open_list(view.name);
    });
    inject_keys("ab");
    // The list never took this arrow, so it still moves the caret.
    press(&[NamedKey::ArrowLeft]);
    inject_keys("c");
    press(&[NamedKey::ArrowDown, NamedKey::ArrowDown]);
    from_main(move || {
        ensure!(view.name.text() == "acb", "name holds {:?}", view.name.text());
        ensure!(view.marked == 2, "the mark is on row {}", view.marked);
        Ok(())
    })?;
    // the single line field holds acb and is edited, the list is open with
    // /model marked
    check_colors(SINGLE_OPEN)?;

    press(&[NamedKey::Enter]);
    from_main(move || {
        ensure!(view.name.text() == "/model", "name holds {:?}", view.name.text());
        ensure!(
            view.name.is_editing(),
            "a taken Enter ended the editing of a single line field"
        );
        ensure!(view.submits == ["/help"], "a taken Enter submitted");
        Ok(())
    })?;

    from_main(move || view.open_list(view.name));
    press(&[NamedKey::Tab]);
    from_main(move || {
        ensure!(
            view.name.is_editing() && !view.next.is_editing(),
            "a taken Tab left the single line field"
        );
        Ok(())
    })?;

    from_main(move || view.open_list(view.name));
    press(&[NamedKey::Escape]);
    from_main(move || {
        ensure!(view.name.is_editing(), "a taken Escape ended the editing");
        ensure!(view.ended == 1, "editing ended {} times", view.ended);
        ensure!(view.screen_escapes == 1, "the screen Escape ran for a taken key");
        Ok(())
    })?;
    // Enter picked /model, Tab picked /help, Escape closed the list, the
    // single line field is still edited
    check_colors(SINGLE_PICKED)
}

/// Tab and Enter of the single line field after the list closed.
fn single_line_released(view: Weak<TextFieldTakenKeys>) -> Result<()> {
    press(&[NamedKey::Tab]);
    from_main(move || {
        ensure!(
            view.next.is_editing() && !view.name.is_editing(),
            "Tab of a closed list did not select the next field"
        );
        Ok(())
    })?;
    // list closed: Tab left the single line field, the next field is edited
    check_colors(NEXT_FIELD)?;

    from_main(move || view.name.focus());
    press(&[NamedKey::Enter]);
    from_main(move || {
        ensure!(
            view.submits == ["/help", "/help"],
            "Enter of a closed list submitted {:?}",
            view.submits
        );
        ensure!(!view.name.is_editing(), "Enter kept the single line field edited");
        Ok(())
    })?;
    // list closed: Tab went to the next field, Enter submitted /help from
    // the single line field and ended its editing
    check_colors(SINGLE_RELEASED)
}

impl ViewTest for TextFieldTakenKeys {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(44);

        chat_box_list(view)?;
        chat_box_pick_and_close(view)?;
        single_line_list(view)?;
        single_line_released(view)
    }
}

const CLOSED: &str = r"
     592    4 - #597c95
     440    8 - #597c95
      52   20 - #597c95
     120   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
     180   24 - #1d2931
      76  140 - #415b6d
     196  140 - #394f5f
     136  144 - #435e71
      48  148 - #425b6e
     308  160 - #bcbcbc
     416  160 - #bcbcbc
     532  160 - #bcbcbc
      48  176 - #939393
      80  176 - #bcbcbc
      48  180 - #939393
      64  180 - #bcbcbc
      40  200 - #bcbcbc
      64  204 - #bcbcbc
     472  224 - #bcbcbc
      72  244 - #1b252c
     108  248 - #000000
     168  248 - #40596b
     260  248 - #1d2830
      56  252 - #466175
     248  252 - #597c95
     364  264 - #ffffff
     576  264 - #ffffff
     124  316 - #131a20
      44  324 - #19232a
     220  324 - #030405
     168  328 - #597c95
     408  372 - #ffffff
     528  372 - #ffffff
     136  444 - #597c95
      52  488 - #202d36
     172  504 - #273641
     208  508 - #597c95
     128  512 - #344857
      56  532 - #000001
      96  532 - #000000
     392  588 - #597c95
     592  592 - #597c95
";

const OPEN_MARKED: &str = r"
     236   20 - #334756
     164   24 - #597c95
     384   36 - #dde3e8
     576   36 - #dde3e8
      44   44 - #8e9295
      64   44 - #3d3e40
      40   56 - #dde3e8
     260   68 - #228cdb
     480   72 - #228cdb
      68   76 - #228cdb
      48   80 - #228cdb
      40   84 - #228cdb
     188   84 - #228cdb
      76  104 - #a3a7ab
      44  108 - #8a8e91
     360  108 - #dde3e8
      60  116 - #dde3e8
      88  116 - #dde3e8
     136  144 - #435e71
     196  148 - #394f5f
     416  160 - #bcbcbc
     532  160 - #bcbcbc
     308  164 - #bcbcbc
      48  176 - #939393
      80  176 - #bcbcbc
      40  200 - #bcbcbc
      64  204 - #bcbcbc
     128  248 - #000000
     184  248 - #49667a
      72  252 - #1b252c
     248  252 - #597c95
     576  264 - #ffffff
     384  300 - #ffffff
      44  324 - #19232a
     216  324 - #597c95
     156  328 - #597c95
     504  372 - #ffffff
     112  448 - #040506
     396  448 - #597c95
      52  488 - #202d36
     160  512 - #466175
      56  532 - #000001
     308  592 - #597c95
     592  592 - #597c95
";

const TAB_PICKED: &str = r"
     592    4 - #597c95
     440    8 - #597c95
      56   16 - #000000
     120   20 - #334756
     188   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
     196  140 - #394f5f
     136  144 - #435e71
      48  148 - #425b6e
     308  160 - #bcbcbc
     416  160 - #bcbcbc
     532  160 - #bcbcbc
      48  180 - #bcbcbc
      52  180 - #bcbcbc
      64  180 - #bcbcbc
      68  180 - #bcbcbc
      84  180 - #878787
     472  224 - #bcbcbc
      72  244 - #1b252c
     108  248 - #000000
     168  248 - #40596b
      56  252 - #466175
     248  252 - #597c95
     364  264 - #ffffff
     576  264 - #ffffff
     124  316 - #131a20
      44  324 - #19232a
     204  328 - #597c95
     408  372 - #ffffff
     528  372 - #ffffff
     124  468 - #12191e
      52  488 - #202d36
     100  508 - #597c95
     208  508 - #597c95
      56  532 - #000001
     136  532 - #597c95
     180  532 - #000000
     392  588 - #597c95
     592  592 - #597c95
";

const ESCAPE_CLOSED: &str = r"
     432    4 - #597c95
     592    4 - #597c95
      56   16 - #000000
     120   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
     180   24 - #1d2931
      76  140 - #415b6d
     196  140 - #394f5f
     136  144 - #435e71
      48  148 - #425b6e
     308  160 - #bcbcbc
     416  160 - #bcbcbc
     532  160 - #bcbcbc
      60  176 - #bcbcbc
      76  176 - #bcbcbc
      60  180 - #bcbcbc
      64  180 - #bcbcbc
      76  180 - #bcbcbc
     472  224 - #bcbcbc
      48  248 - #3e5768
     112  248 - #334756
     168  248 - #40596b
      72  252 - #1b252c
     248  252 - #597c95
     364  264 - #ffffff
     576  264 - #ffffff
     124  316 - #131a20
      44  324 - #19232a
     204  328 - #597c95
     408  372 - #ffffff
     528  372 - #ffffff
     140  464 - #0b0f12
     180  468 - #11181d
      52  488 - #202d36
      96  488 - #597c95
     492  500 - #597c95
     208  508 - #597c95
     160  512 - #466175
      96  528 - #000000
      56  532 - #000001
     136  532 - #597c95
     392  588 - #597c95
     592  592 - #597c95
";

const RELEASED: &str = r"
     488    4 - #597c95
      56   16 - #000000
     120   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
     180   24 - #1d2931
      76  140 - #415b6d
     196  140 - #394f5f
     136  144 - #435e71
      48  148 - #425b6e
     400  160 - #ffffff
     576  160 - #ffffff
      76  176 - #ffffff
      52  180 - #2c2c2c
      60  180 - #ffffff
      64  180 - #ffffff
      76  180 - #ffffff
     168  248 - #40596b
      68  252 - #2f414e
     124  252 - #476276
     200  252 - #000001
     248  252 - #597c95
     400  280 - #ffffff
     576  300 - #ffffff
     124  316 - #131a20
      44  324 - #19232a
      76  324 - #344857
     204  328 - #597c95
     328  372 - #ffffff
     460  380 - #597c95
     592  444 - #597c95
      64  448 - #597c95
     112  448 - #040506
     124  468 - #12191e
     188  468 - #597c95
      52  488 - #202d36
     148  488 - #334756
     100  508 - #597c95
     208  512 - #11181d
      56  532 - #000001
     136  532 - #597c95
     180  532 - #000000
     372  592 - #597c95
     592  592 - #597c95
";

const SINGLE_OPEN: &str = r"
     592    4 - #597c95
     132   20 - #000000
     204   24 - #597c95
     352   36 - #dde3e8
      44   44 - #8e9295
      64   44 - #3d3e40
      44   52 - #8e9295
      72   52 - #dde3e8
      52   76 - #dde3e8
      40   84 - #dde3e8
     276   92 - #dde3e8
     576  100 - #228cdb
      76  104 - #1967a1
      88  104 - #228cdb
      44  112 - #155789
      92  112 - #1b6fae
      68  116 - #228cdb
      88  116 - #228cdb
     484  120 - #228cdb
     136  144 - #435e71
     196  148 - #394f5f
      76  176 - #ffffff
      60  180 - #ffffff
      72  244 - #1b252c
     124  248 - #476276
     184  248 - #49667a
     248  252 - #597c95
     544  264 - #bcbcbc
     428  268 - #bcbcbc
      52  280 - #bcbcbc
      64  284 - #bcbcbc
     124  316 - #131a20
      44  324 - #19232a
     176  324 - #000000
     328  372 - #ffffff
     576  372 - #ffffff
     180  464 - #11181d
     124  468 - #12191e
      52  488 - #202d36
     208  504 - #11181d
      56  532 - #000001
     148  532 - #597c95
     100  536 - #3e5769
     448  592 - #597c95
";

const SINGLE_PICKED: &str = r"
     592    4 - #597c95
      52   20 - #597c95
     120   20 - #334756
     188   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
     420   60 - #597c95
     136  144 - #435e71
     192  144 - #597c95
     340  160 - #ffffff
     452  164 - #ffffff
     560  168 - #ffffff
      76  176 - #ffffff
      52  180 - #2c2c2c
      60  180 - #ffffff
      64  180 - #ffffff
      76  180 - #ffffff
     124  248 - #476276
      56  252 - #466175
     180  252 - #10171b
     248  252 - #597c95
     420  264 - #bcbcbc
     516  264 - #bcbcbc
     576  272 - #bcbcbc
      60  284 - #b8b8b8
      76  284 - #bcbcbc
      76  292 - #bcbcbc
      80  292 - #bcbcbc
     332  300 - #bcbcbc
     124  316 - #131a20
      44  324 - #19232a
     204  328 - #597c95
     576  336 - #ffffff
     452  444 - #597c95
      64  448 - #597c95
     112  448 - #040506
     160  464 - #000000
     220  468 - #415a6d
     288  468 - #12191e
      52  488 - #202d36
     172  504 - #273641
     112  508 - #0e1317
      56  532 - #000001
     520  592 - #597c95
";

const NEXT_FIELD: &str = r"
     404    4 - #597c95
     592    4 - #597c95
      56   16 - #000000
     120   20 - #334756
     188   20 - #334756
     248   20 - #0e1317
      76  140 - #415b6d
     136  144 - #435e71
     192  144 - #597c95
     312  160 - #ffffff
     424  160 - #ffffff
     576  160 - #ffffff
      76  176 - #ffffff
      52  180 - #2c2c2c
      64  180 - #ffffff
     112  248 - #334756
     168  248 - #40596b
      56  252 - #466175
     248  252 - #597c95
      52  284 - #2c2c2c
      60  284 - #f9f9f9
      76  284 - #ffffff
     544  300 - #ffffff
     124  316 - #131a20
     200  316 - #000000
     304  336 - #bcbcbc
     388  340 - #bcbcbc
      36  348 - #000000
      36  352 - #000000
      36  360 - #000000
      36  364 - #000000
     228  368 - #bcbcbc
     480  372 - #bcbcbc
     116  448 - #597c95
     180  464 - #11181d
     248  468 - #12191e
     288  468 - #12191e
      52  488 - #202d36
      96  488 - #597c95
     172  504 - #273641
      56  532 - #000001
     120  532 - #597c95
     416  592 - #597c95
     592  592 - #597c95
";

const SINGLE_RELEASED: &str = r"
     408    4 - #597c95
     592    4 - #597c95
      52   20 - #597c95
     120   20 - #334756
     188   20 - #334756
     248   20 - #0e1317
     288   20 - #334756
      76  140 - #415b6d
     136  144 - #435e71
     192  144 - #597c95
     324  160 - #ffffff
     444  160 - #ffffff
      76  176 - #ffffff
      52  180 - #2c2c2c
      64  180 - #ffffff
      76  180 - #ffffff
     576  196 - #ffffff
     112  248 - #334756
     168  248 - #40596b
      56  252 - #466175
     248  252 - #597c95
     492  264 - #ffffff
      52  284 - #2c2c2c
      60  284 - #f9f9f9
      76  284 - #ffffff
     124  316 - #131a20
      44  324 - #19232a
     204  328 - #597c95
     404  328 - #597c95
     576  336 - #ffffff
      64  448 - #597c95
     112  448 - #040506
     180  464 - #11181d
     220  468 - #415a6d
     288  468 - #12191e
      52  488 - #202d36
     140  492 - #19232b
     172  504 - #273641
     112  508 - #0e1317
      56  532 - #000001
     148  532 - #597c95
     196  532 - #070a0c
     372  592 - #597c95
     520  592 - #597c95
";
