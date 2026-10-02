use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Label, Setup, Shadow, Switch, TextAlignment, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

const WOOD: &str = "#d9a55b";
const RUST: &str = "#9a3412";
const SLOT: &str = "#5a3a1e";
const PALE: &str = "#f3dcb0";

const ALL_OFF: &str = r"
      4    4 - #d9a55b
    220   24 - #ffffff
    252   28 - #cccccd
    284   32 - #d0d0d1
    204   36 - #ababab
     84   44 - #000000
    228   44 - #ffffff
     60   48 - #251c0f
    248   56 - #9d9d9e
    228   68 - #8c8c8d
    236   68 - #99999a
    252  104 - #53351c
    288  112 - #5a3a1e
     40  116 - #d9a55b
    124  116 - #d9a55b
    224  136 - #f3dcb0
    228  160 - #2b1908
    244  176 - #af9678
    220  180 - #bd9f7d
    288  180 - #2d1b0a
     72  184 - #a47d45
    212  196 - #a88e70
    224  204 - #b59977
    140  220 - #d9a55b
    220  232 - #85613c
    236  232 - #95744f
    236  248 - #d3b99a
     60  256 - #000000
    284  256 - #6d4726
    232  268 - #dabb97
    196  272 - #bb8f4f
    244  284 - #af8549
";

const ALL_ON: &str = r"
      4    4 - #d9a55b
     84   44 - #000000
    288   44 - #077114
     60   48 - #251c0f
    288   48 - #077014
    204   52 - #09921a
    248   56 - #ffffff
    276   68 - #a67e46
    252  100 - #f3dcb0
    288  112 - #79290e
     40  116 - #d9a55b
    124  116 - #d9a55b
    288  116 - #76280e
    216  124 - #9a3412
    252  136 - #70260d
    256  140 - #aa8147
    228  160 - #2b1909
    272  180 - #d5bb9b
     64  184 - #876739
    272  184 - #ddbe98
    288  184 - #3e3b17
    208  196 - #698639
    268  204 - #bba183
    236  208 - #3f3c18
    252  232 - #7a5936
    244  244 - #9b7f5c
    272  244 - #c5aa89
    288  252 - #b79976
     60  256 - #000000
    236  268 - #57702f
    260  272 - #c9af8f
    276  284 - #9b7641
";

const TWO_OFF: &str = r"
      4    4 - #d9a55b
    236   20 - #09921a
    264   28 - #ffffff
    204   40 - #09921a
     84   44 - #000000
    288   44 - #077114
     60   48 - #251c0f
    288   48 - #077014
    248   56 - #ffffff
    276   68 - #a67e46
    252  108 - #4f331a
     40  116 - #d9a55b
    124  116 - #d9a55b
    288  120 - #5a3a1e
    208  124 - #f3dcb0
    228  160 - #2b1909
    256  180 - #b89c7c
    272  180 - #d5bb9b
     64  184 - #876739
    288  184 - #3e3b17
    208  196 - #698639
    268  204 - #bba183
    132  220 - #d9a55b
    220  232 - #85613c
    236  248 - #d3b99a
     60  256 - #000000
    200  256 - #8a6e4e
    284  256 - #6d4726
    196  272 - #bb8f4f
    224  272 - #ccb293
    216  276 - #b19677
    244  284 - #af8549
";

/// Every way to change the look of a switch, next to a default one: own
/// colors, own pictures with no knob shadow, and pictures with a knob bigger
/// than the track.
#[view]
struct SwitchColors {
    #[init]
    plain_title:    Label,
    plain:          Switch,
    colors_title:   Label,
    colors:         Switch,
    pictures_title: Label,
    pictures:       Switch,
    big_title:      Label,
    big:            Switch,
}

impl Setup for SwitchColors {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WOOD);

        let rows = [
            (self.plain_title, self.plain, "default"),
            (self.colors_title, self.colors, "own colors"),
            (self.pictures_title, self.pictures, "pictures"),
            (self.big_title, self.big, "big knob"),
        ];
        let mut top = 20.0;
        for (title, switch, text) in rows {
            title.set_text(text).set_text_size(20);
            title.set_alignment(TextAlignment::Left);
            title.place().t(top).l(20).size(160, 50);
            switch.place().t(top).r(30).size(88, 50);
            top += 70.0;
        }

        self.colors.set_on_color(RUST).set_off_color(SLOT).set_knob_color(PALE);

        self.pictures
            .set_track_images("switch_track_on.png", "switch_track_off.png")
            .set_knob_images("switch_knob.png", "switch_knob.png")
            .set_knob_shadow(Shadow {
                radius: 0.0,
                ..Shadow::default()
            });

        self.big
            .set_track_images("switch_track_on.png", "switch_track_off.png")
            .set_knob_images("switch_knob.png", "switch_knob.png")
            .set_knob_inset(-5.0);
    }
}

impl ViewTest for SwitchColors {
    fn canvas() -> (u32, u32) {
        (320, 300)
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // All 4 off.
        check_colors(ALL_OFF)?;

        // All 4 on.
        inject_touches(
            r"
            250 45 b
            250 45 e
            250 115 b
            250 115 e
            250 185 b
            250 185 e
            250 255 b
            250 255 e
            ",
        );
        check_colors(ALL_ON)?;

        // The colored one and the big one off again.
        inject_touches(
            r"
            250 115 b
            250 115 e
            250 255 b
            250 255 e
            ",
        );
        check_colors(TWO_OFF)?;

        Ok(())
    }
}
