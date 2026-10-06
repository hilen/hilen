use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, ModifiersState, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{checkpoint, inject_keys, inject_modifiers, inject_named_key, inject_touches},
    window::NamedKey,
};

/// A multiline field with `set_submit_on_enter` is a chat box. A plain
/// Enter fires `submitted` and the field stays in edit, Shift and Enter
/// is the new line. The field below it has no such option, its Enter is
/// a new line as before.
#[view]
struct MultilineSubmit {
    sent: Vec<String>,

    #[init]
    chat_title:  Label,
    chat:        TextField,
    log:         Label,
    plain_title: Label,
    plain:       TextField,
}

impl Setup for MultilineSubmit {
    fn setup(mut self: Weak<Self>) {
        self.chat_title
            .set_text("chat box, Enter sends, Shift and Enter is a new line")
            .set_text_size(18);
        self.chat_title.set_alignment(TextAlignment::Left);
        self.chat_title.place().t(20).lr(20).h(26);

        self.chat.set_multiline(true).set_submit_on_enter(true).set_text_size(24);
        self.chat.set_alignment(TextAlignment::Left);
        self.chat.set_placeholder("message");
        self.chat.place().t(50).lr(20).h(110);
        // What a chat does with a sent message: keep it and empty the box.
        self.chat.submitted.val(move |text| self.send(text));

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(170).lr(20).h(120);

        self.plain_title
            .set_text("plain text area, Enter is a new line")
            .set_text_size(18);
        self.plain_title.set_alignment(TextAlignment::Left);
        self.plain_title.place().t(310).lr(20).h(26);

        self.plain.set_multiline(true).set_text_size(24);
        self.plain.set_alignment(TextAlignment::Left);
        self.plain.set_placeholder("notes");
        self.plain.place().t(340).lr(20).h(110);
        self.plain.submitted.val(move |text| self.send(format!("plain area: {text}")));

        self.show_log();
    }
}

impl MultilineSubmit {
    fn send(mut self: Weak<Self>, text: String) {
        self.sent.push(text);
        self.chat.clear();
        self.show_log();
    }

    fn show_log(self: Weak<Self>) {
        let mut lines = vec![format!("sent {} messages", self.sent.len())];
        lines.extend(
            self.sent
                .iter()
                .enumerate()
                .map(|(index, text)| format!("{}: {}", index + 1, text.replace('\n', " / "))),
        );
        self.log.set_text(lines.join("\n"));
    }
}

impl ViewTest for MultilineSubmit {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("200 100 b\n200 100 e");
        inject_keys("hi");
        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::Enter);
        inject_modifiers(ModifiersState::empty());
        inject_keys("there");
        checkpoint("the chat box holds hi and there on 2 lines, the log says sent 0 messages")?;
        from_main(move || {
            ensure!(
                view.chat.text() == "hi\nthere",
                "chat holds {:?}",
                view.chat.text()
            );
            ensure!(view.sent.is_empty(), "sent before Enter: {:?}", view.sent);
            Ok(())
        })?;

        inject_named_key(NamedKey::Enter);
        checkpoint("the log says 1: hi / there, the chat box is empty and still has the caret")?;
        from_main(move || {
            ensure!(view.sent == ["hi\nthere"], "sent {:?}", view.sent);
            ensure!(view.chat.is_editing(), "Enter ended the editing of the chat box");
            ensure!(view.chat.text().is_empty(), "chat holds {:?}", view.chat.text());
            Ok(())
        })?;

        // No tap in between, the caret never left the box.
        inject_keys("next");
        inject_named_key(NamedKey::Enter);
        checkpoint("the log says 2: next, typed with no tap after the first message")?;
        from_main(move || {
            ensure!(view.sent == ["hi\nthere", "next"], "sent {:?}", view.sent);
            Ok(())
        })?;

        inject_touches("200 390 b\n200 390 e");
        inject_keys("a");
        inject_named_key(NamedKey::Enter);
        inject_keys("b");
        checkpoint("the plain text area holds a and b on 2 lines, the log still has 2 messages")?;
        from_main(move || {
            ensure!(view.plain.text() == "a\nb", "plain holds {:?}", view.plain.text());
            ensure!(view.sent.len() == 2, "the plain area sent: {:?}", view.sent);
            Ok(())
        })
    }
}
