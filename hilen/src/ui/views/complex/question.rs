use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    sync::mpsc::channel,
};

use ui_proc::view;

use crate::{
    deps::{hreads::from_main, refs::Weak, vents::OnceEvent},
    gm::{
        color::{BLACK, CLEAR},
        flat::Size,
    },
    ui::{
        Anchor::Width,
        Button, Container, Label, ModalView, Setup, UIColor,
        view::ViewData,
        views::complex::alert::{
            ACTION_BLUE, ALERT_WIDTH, BACKGROUND, BUTTON_HEIGHT, MESSAGE_COLOR, MIN_TEXT_HEIGHT, PADDING,
            SEPARATOR, fit_to_text,
        },
    },
};

#[view]
pub struct Question {
    question: String,

    left:  String,
    right: String,

    event:            OnceEvent<bool>,
    #[init]
    label:            Label,
    separator:        Container,
    button_separator: Container,
    ok_button:        Button,
    cancel_button:    Button,
}

impl ModalView<(), bool> for Question {
    fn modal_event(&self) -> &OnceEvent<bool> {
        &self.event
    }

    fn modal_size() -> Size {
        (ALERT_WIDTH, 150.0).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.25).into()
    }
}

impl Question {
    pub fn ask(question: impl Into<String>) -> Self {
        Question {
            question: question.into(),
            left: "No".to_string(),
            right: "Yes".to_string(),
            ..Default::default()
        }
    }

    pub fn options(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.left = left.into();
        self.right = right.into();
        self
    }

    ///bool == true -> right choice
    pub fn callback(self, callback: impl FnOnce(bool) + Send + 'static) {
        self.show().event.val(callback);
    }

    pub fn on_yes(self, callback: impl FnOnce() + Send + 'static) {
        self.callback(|yes| {
            if yes {
                callback();
            }
        });
    }

    fn show(self) -> Weak<Self> {
        let view = Self::show_modally(self);
        fit_to_text(&*view, &view.label);
        view
    }

    fn recv_callback(self) -> bool {
        let (se, rc) = channel::<bool>();

        from_main(move || {
            self.show().event.val(move |answer| {
                se.send(answer).unwrap();
            });
        });

        rc.recv().unwrap()
    }
}

impl IntoFuture for Question {
    type Output = bool;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.recv_callback() })
    }
}

impl Setup for Question {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(14);
        self.set_color(BACKGROUND);

        self.label.set_text_size(17);
        self.label.set_text_color(MESSAGE_COLOR);
        self.label.set_multiline(true);
        self.label.set_text(self.question.clone());
        self.label.place().lrt(PADDING).h(MIN_TEXT_HEIGHT);

        self.separator.set_color(SEPARATOR);
        self.separator.place().lr(0).b(BUTTON_HEIGHT).h(1);

        self.button_separator.set_color(SEPARATOR);
        self.button_separator.place().b(0).w(1).h(BUTTON_HEIGHT).center_x();

        self.cancel_button
            .set_text(self.left.clone())
            .set_text_size(17)
            .set_text_color(ACTION_BLUE);
        self.cancel_button.set_color(CLEAR);
        self.cancel_button.place().h(BUTTON_HEIGHT).bl(0).relative(Width, self, 0.5);
        self.cancel_button.on_tap(move || self.hide_modal(false));

        self.ok_button
            .set_text(self.right.clone())
            .set_text_size(17)
            .set_text_color(ACTION_BLUE);
        self.ok_button.set_color(CLEAR);
        self.ok_button.place().h(BUTTON_HEIGHT).br(0).relative(Width, self, 0.5);
        self.ok_button.on_tap(move || self.hide_modal(true));
    }
}
