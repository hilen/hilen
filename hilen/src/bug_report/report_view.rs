use std::{
    mem::take,
    sync::atomic::{AtomicUsize, Ordering},
};

use log::error;
use ui_proc::view;

use crate::{
    bug_report::{BugReport, input_ring::KeyPress},
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        color::{CLEAR, Color, WHITE},
        flat::Size,
    },
    ui::{
        Anchor::{Height, Left, Right, Top, Y},
        AnimatedImage, Button, CheckBox, Container, DynamicColor, ImageMode, ImageView, Label, ModalView,
        ScrollView, Setup, TextAlignment, TextField, UIManager, ViewData, ViewFrame, ViewSubviews,
    },
    window::image::Image,
};

const PAGE: DynamicColor = DynamicColor::new(Color::hex("#f7f8fa"), Color::hex("#1e2126"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#17191d"), Color::hex("#f2f4f7"));
const MUTED: DynamicColor = DynamicColor::new(Color::hex("#5d6570"), Color::hex("#a8b0bc"));
const PANEL: DynamicColor = DynamicColor::new(Color::hex("#ececef"), Color::hex("#2a2e35"));
const LINE: DynamicColor = DynamicColor::new(Color::hex("#e0e2e6"), Color::hex("#343941"));
const FIELD_SELECTED: DynamicColor = DynamicColor::new(Color::hex("#e6edfa"), Color::hex("#3a4250"));
const ACCENT: Color = Color::hex("#3c78f0");

/// The description must carry enough to act on, matching the karkas
/// dialog's rule.
const MIN_DESCRIPTION_CHARS: usize = 20;

/// Every open uploads the screenshot as a fresh managed texture. Managed
/// resources are never freed, the pipelines rely on that, so the name
/// carries a counter instead of being reused.
static SHOT_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// What the dialog receives when it opens. The screenshot is taken
/// before the dialog shows, so the dialog itself is never in the shot,
/// and the key ring is frozen at the same moment, presses inside the
/// dialog do not mutate it.
pub(crate) struct BugReportInput {
    pub screenshot_png:  Vec<u8>,
    pub screenshot_rgba: Vec<u8>,
    pub screenshot_size: Size<u32>,
    pub log_bytes:       usize,
    pub keys:            Vec<KeyPress>,
}

/// What the reporter agreed to send. Keys are present only when the opt
/// in checkbox was on at submit time.
pub(crate) struct BugReportData {
    pub email:          String,
    pub description:    String,
    pub screenshot_png: Vec<u8>,
    pub keys:           Option<Vec<KeyPress>>,
}

/// The scrollable middle of the page. Its own view because #[init]
/// cannot aim children into the scroll container.
#[view]
pub struct BugReportForm {
    pub(crate) keys_preview: Weak<Label>,

    #[init]
    email_label:            Label,
    pub(crate) email:       TextField,
    desc_label:             Label,
    pub(crate) description: TextField,
    pub(crate) counter:     Label,
    pub(crate) animation:   AnimatedImage,
    shot_caption:           Label,
    screenshot:             ImageView,
    log_caption:            Label,
    pub(crate) attach_keys: CheckBox,
    keys_caption:           Label,
    keys_note:              Label,
    keys_panel:             ScrollView,
    sysinfo:                Label,
}

impl BugReportForm {
    fn caption(label: Weak<Label>, text: &str) {
        label.set_text(text).set_text_size(12).set_text_color(MUTED);
        label.set_alignment(TextAlignment::Left);
    }

    fn field(mut field: Weak<TextField>) {
        field.set_color(PANEL).set_corner_radius(8);
        field.set_text_color(TEXT).set_selected_color(FIELD_SELECTED);
        field.set_text_size(15);
        field.set_alignment(TextAlignment::Left);
    }
}

impl Setup for BugReportForm {
    fn setup(self: Weak<Self>) {
        Self::caption(self.email_label, "Your email (required)");
        self.email_label.place().t(16).l(24).size(300, 16);

        Self::field(self.email);
        self.email.set_placeholder("you@example.com");
        self.email.place().anchor(Top, self.email_label, 6).lr(24).h(36);

        Self::caption(self.desc_label, "Describe the bug (required, min 20 chars)");
        self.desc_label.place().anchor(Top, self.email, 14).l(24).size(300, 16);

        Self::field(self.description);
        self.description.set_placeholder("What did you expect? What happened?");
        self.description.set_multiline(true);
        self.description.place().anchor(Top, self.desc_label, 6).lr(24).h(100);

        Self::caption(self.counter, "0 / 20");
        self.counter.place().anchor(Top, self.description, 6).l(24).size(300, 14);

        Self::caption(self.shot_caption, "Screenshot (will be sent)");

        // The karkas dialogs play a looping animation between the description
        // and the screenshot. Only when the app registered one.
        if let Some(gif) = BugReport::animation() {
            match self.animation.set_gif_keyed(gif, "bug-report") {
                Ok(anim) => {
                    anim.set_mode(ImageMode::AspectFit);
                }
                Err(err) => error!("Bug report animation failed to decode: {err}"),
            }
            self.animation.set_corner_radius(8);
            self.animation.place().anchor(Top, self.counter, 14).l(24).size(322, 200);
            self.shot_caption.place().anchor(Top, self.animation, 14).l(24).size(300, 16);
        } else {
            self.animation.set_hidden(true);
            self.shot_caption.place().anchor(Top, self.counter, 14).l(24).size(300, 16);
        }

        self.screenshot.set_corner_radius(8).set_border_color(LINE);
        self.screenshot.place().anchor(Top, self.shot_caption, 6).l(24).size(214, 120);

        Self::caption(self.log_caption, "Log (will be sent)");
        self.log_caption.place().anchor(Top, self.screenshot, 14).l(24).size(300, 16);

        self.attach_keys.place().anchor(Top, self.log_caption, 14).l(24).size(20, 20);

        self.keys_caption.set_text("Attach recent key presses").set_text_size(14);
        self.keys_caption.set_text_color(TEXT).set_alignment(TextAlignment::Left);
        self.keys_caption
            .place()
            .anchor(Left, self.attach_keys, 10)
            .same([Y, Height], self.attach_keys)
            .w(300);

        self.keys_note.set_multiline(true).set_text_size(11);
        self.keys_note.set_text_color(MUTED).set_alignment(TextAlignment::Left);
        self.keys_note
            .set_text("Only modifier combos and navigation keys are recorded, never typed text.");
        self.keys_note.place().anchor(Top, self.attach_keys, 6).lr(24).h(28);

        self.keys_panel.set_color(PANEL).set_corner_radius(8);
        self.keys_panel.place().anchor(Top, self.keys_note, 6).lr(24).h(110);

        let preview = self.keys_panel.add_view::<Label>();
        preview.set_multiline(true).set_text_size(12);
        preview.set_color(CLEAR).set_text_color(MUTED);
        preview.set_alignment(TextAlignment::Left);
        preview.place().t(8).lr(10).fit_text_height();

        let mut this = self;
        this.keys_preview = preview;

        Self::caption(
            self.sysinfo,
            &format!("os: {} ({})", std::env::consts::OS, std::env::consts::ARCH),
        );
        self.sysinfo.place().anchor(Top, self.keys_panel, 14).l(24).size(300, 14);
    }
}

#[view]
pub struct BugReportView {
    event:          OnceEvent<Option<BugReportData>>,
    screenshot_png: Vec<u8>,
    keys:           Vec<KeyPress>,

    pub(crate) form: Weak<BugReportForm>,

    #[init]
    title:           Label,
    hint:            Label,
    close:           Button,
    top_line:        Container,
    scroll:          ScrollView,
    bottom_line:     Container,
    cancel:          Button,
    pub(crate) send: Button,
}

impl BugReportView {
    fn valid(self: Weak<Self>) -> bool {
        let email = self.form.email;
        let description = self.form.description;

        if email.is_placeholding() || description.is_placeholding() {
            return false;
        }

        let email = email.text().trim();
        let at = email.find('@');
        let email_ok = at.is_some_and(|at| at > 0 && email[at + 1..].contains('.'));

        email_ok && description.text().trim().chars().count() >= MIN_DESCRIPTION_CHARS
    }

    fn update_state(self: Weak<Self>) {
        let count = if self.form.description.is_placeholding() {
            0
        } else {
            self.form.description.text().trim().chars().count()
        };

        let counter = if count < MIN_DESCRIPTION_CHARS {
            format!(
                "{count} / {MIN_DESCRIPTION_CHARS} ({} more)",
                MIN_DESCRIPTION_CHARS - count
            )
        } else {
            format!("{count} / {MIN_DESCRIPTION_CHARS}")
        };

        self.form.counter.set_text(counter);

        let valid = self.valid();
        let mut this = self;
        this.send.set_enabled(valid);
    }
}

impl Setup for BugReportView {
    fn setup(self: Weak<Self>) {
        self.set_color(PAGE);

        self.title.set_text("Report a bug").set_text_size(16).set_text_color(TEXT);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(14).l(24).size(140, 20);

        self.hint
            .set_text("Cmd/Ctrl + Shift + R")
            .set_text_size(12)
            .set_text_color(MUTED);
        self.hint.set_alignment(TextAlignment::Left);
        self.hint
            .place()
            .anchor(Left, self.title, 10)
            .same([Y, Height], self.title)
            .w(160);

        self.close.set_text("X").set_text_size(16);
        self.close.set_color(CLEAR).set_text_color(MUTED);
        self.close.place().t(10).r(16).size(28, 28);
        self.close.on_tap(move || self.hide_modal(None));

        self.top_line.set_color(LINE);
        self.top_line.place().t(48).lr(0).h(1);

        self.scroll.place().t(49).b(57).lr(0);

        let mut this = self;
        this.form = self.scroll.add_view::<BugReportForm>();
        // The animation slot adds its height plus one anchor gap.
        let form_height = if BugReport::animation().is_some() {
            834
        } else {
            620
        };
        self.form.place().t(0).l(0).r(0).h(form_height);

        self.bottom_line.set_color(LINE);
        self.bottom_line.place().b(56).lr(0).h(1);

        self.send.set_text("Send").set_text_size(15);
        self.send.set_color(ACCENT).set_text_color(WHITE).set_corner_radius(8);
        self.send.place().size(120, 36).br(10);
        self.send.on_tap(move || {
            if !self.valid() {
                return;
            }

            let mut this = self;
            let form = self.form;

            let data = BugReportData {
                email:          form.email.text().trim().to_string(),
                description:    form.description.text().trim().to_string(),
                screenshot_png: take(&mut this.screenshot_png),
                keys:           form.attach_keys.on().then(|| take(&mut this.keys)),
            };

            self.hide_modal(Some(data));
        });

        self.cancel.set_text("Cancel").set_text_size(15);
        self.cancel.set_color(CLEAR).set_text_color(MUTED);
        self.cancel
            .place()
            .anchor(Right, self.send, 12)
            .same([Y, Height], self.send)
            .w(90);
        self.cancel.on_tap(move || self.hide_modal(None));

        self.form.email.changed.val(move |_| self.update_state());
        self.form.description.changed.val(move |_| self.update_state());

        self.update_state();
    }
}

impl ModalView<BugReportInput, Option<BugReportData>> for BugReportView {
    fn modal_event(&self) -> &OnceEvent<Option<BugReportData>> {
        &self.event
    }

    fn modal_size() -> Size {
        UIManager::root_view().size()
    }

    fn setup_input(self: Weak<Self>, input: BugReportInput) {
        let mut this = self;

        this.screenshot_png = input.screenshot_png;
        this.keys = input.keys;

        if input.screenshot_rgba.is_empty() {
            self.form.shot_caption.set_text("No screenshot, capture failed");
        } else {
            let name = format!("bug-report-shot-{}", SHOT_COUNTER.fetch_add(1, Ordering::Relaxed));
            let image = Image::from_raw_data(input.screenshot_rgba, name, input.screenshot_size, 4);
            let mut shot = self.form.screenshot;
            shot.set_image(image);
            shot.mode = ImageMode::AspectFit;
        }

        self.form
            .log_caption
            .set_text(format!("Log (will be sent) - {} bytes", input.log_bytes));

        let preview = if this.keys.is_empty() {
            "No recent key presses".to_string()
        } else {
            let first_ms = this.keys[0].at_ms;
            let lines: Vec<String> = this.keys.iter().map(|key| key.display(first_ms)).collect();
            lines.join("\n")
        };

        self.form.keys_preview.set_text(preview);
    }
}
