use anyhow::Result;
use hilen::{
    refs::{Weak, manage::DataManager},
    ui::{Image, Label, Setup, StickView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// A brass ring with a dark glass inside, 4 arrows and a dial of 24 marks.
const RING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
<defs>
<linearGradient id="brass" x1="0" y1="0" x2="1" y2="1">
<stop offset="0" stop-color="#fff2b0"/>
<stop offset="0.35" stop-color="#d9a021"/>
<stop offset="0.65" stop-color="#7a4a08"/>
<stop offset="1" stop-color="#f0c040"/>
</linearGradient>
<radialGradient id="glass" cx="0.5" cy="0.5" r="0.5">
<stop offset="0" stop-color="#1c4a66" stop-opacity="0.85"/>
<stop offset="0.8" stop-color="#0a1c2c" stop-opacity="0.9"/>
<stop offset="1" stop-color="#02060a" stop-opacity="0.95"/>
</radialGradient>
</defs>
<circle cx="100" cy="100" r="88" fill="url(#glass)"/>
<circle cx="100" cy="100" r="74" fill="none" stroke="#5fd0ff" stroke-opacity="0.5" stroke-width="5"
 stroke-dasharray="3 16.37"/>
<circle cx="100" cy="100" r="46" fill="none" stroke="#5fd0ff" stroke-opacity="0.25" stroke-width="1.5"/>
<g fill="#5fd0ff" fill-opacity="0.8">
<path d="M100 22 L112 40 L88 40 Z"/>
<path d="M100 178 L112 160 L88 160 Z"/>
<path d="M22 100 L40 88 L40 112 Z"/>
<path d="M178 100 L160 88 L160 112 Z"/>
</g>
<circle cx="100" cy="100" r="92" fill="none" stroke="url(#brass)" stroke-width="12"/>
<circle cx="100" cy="100" r="98.5" fill="none" stroke="#3a2204" stroke-width="2"/>
<circle cx="100" cy="100" r="86" fill="none" stroke="#3a2204" stroke-width="1.5"/>
</svg>"##;

/// A red glass ball with a light spot and a dark rim.
const KNOB: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
<defs>
<radialGradient id="ball" cx="0.38" cy="0.32" r="0.75">
<stop offset="0" stop-color="#ff9a80"/>
<stop offset="0.35" stop-color="#e02818"/>
<stop offset="0.8" stop-color="#7a0a08"/>
<stop offset="1" stop-color="#2a0202"/>
</radialGradient>
<radialGradient id="spot" cx="0.5" cy="0.5" r="0.5">
<stop offset="0" stop-color="#ffffff" stop-opacity="0.95"/>
<stop offset="1" stop-color="#ffffff" stop-opacity="0"/>
</radialGradient>
</defs>
<circle cx="50" cy="50" r="46" fill="url(#ball)"/>
<circle cx="50" cy="50" r="46" fill="none" stroke="#1a0000" stroke-width="2"/>
<ellipse cx="36" cy="30" rx="18" ry="12" fill="url(#spot)" transform="rotate(-30 36 30)"/>
<path d="M22 66 A34 34 0 0 0 78 66" fill="none" stroke="#ffb090" stroke-opacity="0.45" stroke-width="3"
 stroke-linecap="round"/>
</svg>"##;

const CHECK_1: &str = r"
     236   48 - #354958
     200   52 - #000000
      80   60 - #597c95
     364   60 - #1a242c
     532   60 - #3e5769
     140   64 - #000001
     480   64 - #597c95
     344   68 - #000000
     408   68 - #2c3d4a
     444  104 - #e4b749
     436  136 - #51aed8
     212  140 - #707070
     492  160 - #13283a
     428  176 - #fcd7d0
     464  180 - #ba1d12
     532  180 - #8a5a10
     124  184 - #c3c3c3
     416  184 - #f59e91
     440  184 - #f0634e
     420  188 - #f69f91
     436  196 - #ec533f
     508  200 - #50aed8
     352  212 - #dfad37
     480  212 - #630806
     412  216 - #d75f4b
     464  220 - #bd5747
     532  224 - #b38323
     436  236 - #7f0c09
     376  268 - #a57113
     440  268 - #50aed8
     144  280 - #707070
     592  592 - #597c95
";

const CHECK_2: &str = r"
     156   48 - #030506
     236   48 - #354958
      80   60 - #597c95
     364   60 - #1a242c
     416   60 - #000001
     532   60 - #3e5769
     480   64 - #597c95
     236   68 - #354958
     344   68 - #000000
     356   68 - #121a1f
     440  104 - #e5b94d
     504  132 - #ac7715
     436  136 - #51aed8
     212  140 - #707070
     488  176 - #fcd7d0
     376  180 - #13283a
     500  180 - #f06b58
     124  184 - #c3c3c3
     480  188 - #f69f91
     488  188 - #faa291
     536  188 - #92110c
     456  196 - #bc1e12
     492  200 - #e94835
     372  204 - #50aed8
     348  216 - #dfad37
     528  232 - #4d0505
     496  240 - #6c0907
     364  256 - #ba8419
     440  268 - #50aed8
     144  280 - #707070
     400  284 - #83520a
     592  592 - #597c95
";

/// 2 sticks side by side, each named by the label over it. The left one
/// keeps the gray pictures of the engine. The right one got a brass ring
/// and a red glass ball from `set_images`, both svg pictures the engine
/// draws at the size of the view. Proves both pictures are replaced and the
/// knob picture still follows a drag.
#[view]
struct StickImages {
    #[init]
    plain_title: Label,
    plain:       StickView,
    own_title:   Label,
    own:         StickView,
}

impl Setup for StickImages {
    fn setup(self: Weak<Self>) {
        self.plain_title.set_frame((20, 40, 280, 40));
        self.plain_title.set_text("default stick");
        self.plain.set_frame((60, 100, 200, 200));

        self.own_title.set_frame((300, 40, 280, 40));
        self.own_title.set_text("own ring and knob");
        self.own.set_frame((340, 100, 200, 200));
        self.own.set_images(
            Image::load(RING.as_bytes(), "stick images ring"),
            Image::load(KNOB.as_bytes(), "stick images knob"),
        );
    }
}

impl ViewTest for StickImages {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(CHECK_1)?;

        inject_touches(
            "
            440 200 b
            500 200 m
        ",
        );
        check_colors(CHECK_2)?;

        inject_touches("500 200 e");
        Ok(())
    }
}
