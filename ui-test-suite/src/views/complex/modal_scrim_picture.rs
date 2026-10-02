use anyhow::Result;
use hilen::{
    OnceEvent,
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Container, ImageView, Label, ModalView, RED, Setup, Size, UIColor, ViewData, ViewTest, WHITE,
        view,
    },
    ui_test::check_colors,
};

const PAGE: &str = r"
     592    4 - #ff0000
     268   36 - #898989
     120   40 - #9e9e9e
      76   44 - #ffffff
     120   44 - #9e9e9e
     156   44 - #000000
     260   44 - #ffffff
      96   48 - #ffffff
     120   48 - #9e9e9e
     136   48 - #bdbdbd
     192   48 - #2e2e2e
     212   48 - #393939
     220   48 - #474747
     244   48 - #bdbdbd
     248   48 - #bdbdbd
     120   52 - #9e9e9e
     184   52 - #535353
     212   52 - #393939
     220   52 - #474747
     268   52 - #898989
     504  308 - #480000
     400  312 - #000000
     440  312 - #000000
     460  312 - #4f0000
     372  316 - #ff0000
     420  316 - #000000
     500  316 - #ff0000
     480  320 - #ff0000
     380  324 - #010000
       4  592 - #ffffff
     288  592 - #ffffff
     592  592 - #ff0000
";

const DIALOG: &str = r"
     592    4 - #800000
     168   36 - #000000
     268   36 - #454545
      76   44 - #808080
     136   44 - #010101
      96   48 - #808080
     244   48 - #5f5f5f
     192   52 - #171717
     220   52 - #242424
     116   60 - #010101
     244  212 - #dab571
     220  220 - #e8c078
     332  220 - #000000
     352  220 - #825f5f
     376  220 - #af8080
     416  224 - #7a5a5a
     148  228 - #676767
     144  284 - #6b6b6b
     236  300 - #e8c078
     476  312 - #800000
     500  316 - #800000
     460  320 - #260000
     300  352 - #d4ae7c
     168  384 - #525150
     240  384 - #b09564
     280  384 - #b09564
     348  384 - #955656
     408  384 - #955656
     204  388 - #525252
     196  572 - #808080
       4  592 - #808080
     384  592 - #800000
";

// A dialog whose only background is a picture, the modal view itself
// paints nothing. The picture has a solid half, a half clear half, round
// corners and a soft shadow around it.
#[view]
struct PictureModal {
    event: OnceEvent,

    #[init]
    picture: ImageView,
    solid:   Label,
    glass:   Label,
}

impl Setup for PictureModal {
    fn setup(self: Weak<Self>) {
        self.picture.set_image("scrim_panel.png");
        self.picture.place().back();

        self.solid.set_text("solid");
        self.solid.set_text_size(22);
        self.solid.place().t(40).l(45).size(150, 30);

        self.glass.set_text("half clear");
        self.glass.set_text_size(22);
        self.glass.place().t(40).l(195).size(150, 30);
    }
}

impl ModalView for PictureModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (390, 270).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.5).into()
    }
}

// The page is a white and a red strip with a line of text that runs
// under the half clear part of the picture. Everything of the page that
// shows through the picture, its shadow or its round corners must be as
// dim as the page around it.
#[view]
struct ModalScrimPicture {
    #[init]
    white:  Container,
    red:    Container,
    page:   Label,
    behind: Label,
}

impl Setup for ModalScrimPicture {
    fn setup(self: Weak<Self>) {
        self.white.set_color(WHITE);
        self.white.place().tl(0).size(300, 600);

        self.red.set_color(RED);
        self.red.place().t(0).l(300).size(300, 600);

        self.page.set_text("page, dimmed");
        self.page.set_text_size(32);
        self.page.place().t(20).l(20).size(300, 50);

        self.behind.set_text("page text");
        self.behind.set_text_size(36);
        self.behind.place().t(290).l(290).size(300, 50);
    }
}

impl ViewTest for ModalScrimPicture {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // The page before the dialog, at full brightness.
        check_colors(PAGE)?;

        let modal = from_main(PictureModal::prepare_modally);

        // The red strip and the page text under the half clear part are
        // as dim as the page around the dialog, and so is the page under
        // the shadow and the round corners.
        check_colors(DIALOG)?;

        modal.hide_modal(());

        Ok(())
    }
}
