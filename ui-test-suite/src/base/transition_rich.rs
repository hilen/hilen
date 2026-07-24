use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, ensure};
use test_engine::{
    refs::Weak,
    ui::{
        Button, CLEAR, CheckBox, CircleView, Container, GREEN, ImageView, Label, NumberView, PURPLE,
        ProgressView, RED, Setup, Shadow, Slider, Switch, TURQUOISE, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

static FIRST_SETUPS: AtomicUsize = AtomicUsize::new(0);

#[view]
struct TransitionRich {
    #[init]
    title:    Label,
    subtitle: Label,
    shop:     ImageView,
    check:    CheckBox,
    toggle:   Switch,
    progress: ProgressView,
    number:   NumberView,
    card:     Container,
    circle:   CircleView,
    slider:   Slider,
    hint:     Label,
    to_next:  Button,
    info:     Button,
}

impl Setup for TransitionRich {
    fn setup(mut self: Weak<Self>) {
        FIRST_SETUPS.fetch_add(1, Ordering::Relaxed);

        self.set_color("#C0392B");

        self.title.set_text("Red screen");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.subtitle.set_text("Instant transition test");
        self.subtitle.set_text_size(20);
        self.subtitle.set_text_color(WHITE);
        self.subtitle.set_color(CLEAR);
        self.subtitle.place().below(self.title, 12);

        self.shop.set_image("shop.png");
        self.shop.place().size(150, 150).t(160).l(420);

        self.check.set_on(true);
        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.set_on(true);
        self.toggle.place().size(80, 44).t(224).l(24);

        self.progress.set_progress(0.7);
        self.progress.place().size(150, 24).t(292).l(24);

        self.number.set_value(42);
        self.number.place().size(60, 84).t(340).l(24);

        self.card.set_gradient(TURQUOISE, PURPLE);
        self.card.set_corner_radius(12);
        self.card.place().size(120, 90).t(160).l(240);

        self.circle.set_radius(32);
        self.circle.set_color(PURPLE);
        self.circle.place().t(280).l(268);

        self.slider.set_range(0, 100).set_value(30);
        self.slider.place().size(44, 160).t(330).l(480);

        self.hint.set_text("loop demo");
        self.hint.set_text_size(18);
        self.hint.set_text_color(WHITE);
        self.hint.set_color(CLEAR);
        self.hint.place().r(24).b(24).size(160, 40);

        self.to_next.set_text("Switch");
        self.to_next.set_color("#F1C40F");
        self.to_next.set_shadow(Shadow::default());
        self.to_next.place().center_x().b(80).size(220, 64);
        self.to_next.add_transition::<TransitionRich, SwitchedRich>();

        self.info.set_text("Info");
        self.info.place().l(24).b(24).size(120, 44);
    }
}

#[view]
struct SwitchedRich {
    #[init]
    title:     Label,
    body:      Label,
    crate_box: ImageView,
    check:     CheckBox,
    toggle:    Switch,
    progress:  ProgressView,
    number:    NumberView,
    card:      Container,
    circle:    CircleView,
    slider:    Slider,
    again:     Button,
    footer:    Label,
}

impl Setup for SwitchedRich {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#16A085");

        self.title.set_text("Switched");
        self.title.set_text_size(40);
        self.title.set_text_color(WHITE);
        self.title.set_color(CLEAR);
        self.title.place().t(24).center_x().size(360, 56);

        self.body.set_text("Root view replaced");
        self.body.set_text_size(22);
        self.body.set_text_color(WHITE);
        self.body.set_color(CLEAR);
        self.body.place().below(self.title, 12);

        self.crate_box.set_image("crate_box.png");
        self.crate_box.place().br(24).size(140, 140);

        self.check.place().size(44, 44).t(160).l(24);

        self.toggle.place().size(80, 44).t(224).l(24);

        self.slider.set_range(0, 10).set_value(8);
        self.slider.place().size(44, 160).t(280).l(36);

        self.progress.set_progress(0.25);
        self.progress.place().size(150, 24).t(160).l(110);

        self.circle.set_radius(36);
        self.circle.set_color(RED);
        self.circle.place().t(210).l(140);

        self.number.set_value(7);
        self.number.place().size(60, 84).t(300).l(120);

        self.card.set_gradient(RED, GREEN);
        self.card.set_corner_radius(20);
        self.card.place().size(130, 100).t(160).l(420);

        self.again.set_text("Again");
        self.again.set_color("#9B59B6");
        self.again.set_shadow(Shadow::default());
        self.again.place().center_x().b(80).size(220, 64);
        self.again.add_transition::<SwitchedRich, TransitionRich>();

        self.footer.set_text("root swapped");
        self.footer.set_text_size(18);
        self.footer.set_text_color(WHITE);
        self.footer.set_color(CLEAR);
        self.footer.place().l(24).b(24).size(160, 40);
    }
}

impl ViewTest for TransitionRich {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        // The counter survives suite cycles, so compare against the start value.
        let setups = FIRST_SETUPS.load(Ordering::Relaxed);

        check_colors(FIRST)?;

        // Switch and Again sit at the same spot, each lap swaps there and back.
        for _ in 0..4 {
            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            check_colors(SWITCHED)?;

            inject_touches(
                "
                300 488 b
                300 488 e
            ",
            );

            check_colors(FIRST)?;
        }

        ensure!(
            view.is_null(),
            "the old view must be deallocated after an instant transition"
        );

        ensure!(
            FIRST_SETUPS.load(Ordering::Relaxed) == setups + 4,
            "every lap must create a fresh first view"
        );

        Ok(())
    }
}

const FIRST: &str = r"
      4    4 - 192  57  43
    516    4 - 192  57  43
    268   36 - 207 133 129
    208   40 - 192  57  43
    396   44 - 255 255 255
    268   48 - 207 133 129
    296   48 - 192  57  43
    328   52 - 202 116 111
    268   56 - 207 133 129
    364   56 - 192  57  43
    232   60 - 255 255 255
    268   60 - 207 133 129
    328   60 - 202 116 111
    252  116 - 216 164 161
    320  116 - 239 222 221
    344  116 - 200 104  99
    236  120 - 249 243 242
    280  120 - 230 201 200
    304  120 - 198  95  88
    364  120 - 192  57  43
    372  120 - 226 189 188
    388  120 - 218 169 167
     36  160 -  70  78  97
     56  160 -  70  78  97
    292  160 -  17 254 255
    328  160 -  17 254 255
    244  164 -  63 249 255
    352  164 -  63 249 255
    440  164 -  33  21  10
    260  168 -  87 244 255
    456  168 - 107  33  22
    468  168 - 107  33  22
    484  168 - 134  40  29
    556  168 -  54  40  31
     24  172 -  70  78  97
    280  172 - 104 239 255
    304  172 - 104 239 255
    496  172 -  96  74  64
     40  176 -  88 148 242
     52  176 -  88 148 242
    452  176 -  63  48  42
    528  176 -  63  48  42
     44  180 -  88 148 242
     48  180 -  88 148 242
    320  180 - 131 228 255
    424  180 -  35  23  14
     44  184 -  88 148 242
     48  184 -  88 148 242
     52  184 -  88 148 242
     24  188 -  70  78  97
     40  188 -  88 148 242
    240  188 - 153 216 255
    336  188 - 153 216 255
    276  192 - 162 209 255
    356  192 - 162 209 255
    444  192 -  33  21  10
    508  192 -  33  21  10
    540  192 - 184 172 156
     64  196 -  39  53  73
    528  196 - 184 172 156
     32  200 -  39  53  73
    296  200 - 179 196 255
    536  200 - 184 172 156
    528  204 - 184 172 156
    540  204 - 184 172 156
    316  208 - 194 181 255
    536  208 - 120 112 102
    540  208 - 120 112 102
    240  212 - 201 173 255
    352  212 - 201 173 255
    260  216 - 208 164 255
    432  216 -  84  65  55
    556  216 -  84  65  55
    304  220 - 214 155 255
    456  220 - 207 159  58
    332  224 - 220 145 255
    528  224 - 122  37  26
    288  228 - 226 134 255
    272  232 - 232 122 255
    456  232 - 242 234  60
    460  232 - 247 241  94
    524  232 - 195 192 182
     28  236 -  52 199  89
     60  236 -  49 189  84
    240  236 - 237 108 255
    452  236 - 212 165  57
    560  236 -  55  42  36
    308  240 - 243  91 255
    356  240 - 243  91 255
    432  240 -  84  65  55
    456  240 - 204 156  57
    460  240 - 202 154  57
    252  248 - 253  35 255
    268  248 - 253  35 255
    288  248 - 253  35 255
    328  248 - 253  35 255
    516  248 - 122 120 114
    536  248 - 122 120 114
    424  260 - 178 124  82
    444  260 - 178 124  82
    484  260 - 178 124  82
    548  260 - 178 124  82
     36  264 -  52 199  89
     68  264 -  45 177  78
    504  264 - 102  71  47
    464  268 - 138  96  63
    524  272 - 138  96  63
    564  272 - 138  96  63
    544  276 - 138  96  63
    432  280 - 138  96  63
    452  280 - 138  96  63
    476  280 - 138  96  63
    424  284 - 102  71  47
    492  284 - 138  96  63
    532  284 - 102  71  47
    512  288 - 102  71  47
    116  292 -   0   0 231
    216  292 - 192  57  43
    464  292 - 102  71  47
    568  292 - 101  30  21
    420  296 - 140  41  30
    440  296 -  66  46  31
    480  296 -  66  46  31
    496  300 -  66  46  31
    532  300 -  26  15   8
     24  304 -   0   0 231
    512  308 -  26  15   8
    552  308 -  26  15   8
     64  312 -   0   0 231
     96  312 -   0   0 231
     80  348 -   0 150 230
     48  368 - 255 255 255
     80  384 -   0 150 230
     52  404 - 255 255 255
     24  416 -   0 150 230
     80  420 -   0 150 230
    592  436 - 192  57  43
    192  456 - 241 196  15
    396  456 - 241 196  15
    332  476 -   0   0   0
    264  480 - 241 196  15
    300  484 -   0   0   0
    316  484 -   1   1   0
    288  488 -   0   0   0
    328  488 - 241 196  15
    260  492 - 241 196  15
    308  492 -   0   0   0
    320  492 - 241 196  15
    344  492 -   0   0   0
    204  516 - 241 196  15
    408  516 - 241 196  15
    140  532 - 255 255 255
     60  544 - 116 116 116
     88  544 -   0   0   0
    456  552 - 228 195 194
    100  556 - 255 255 255
    508  556 - 220 173 171
     60  560 - 116 116 116
     80  560 -   0   0   0
     24  572 - 255 255 255
";

const SWITCHED: &str = r"
     64    4 -  22 160 133
    592    4 -  22 160 133
    280   36 - 145 193 178
    320   36 - 131 187 170
    376   36 - 254 254 254
    224   40 - 254 254 254
    280   48 - 145 193 178
    280   56 - 145 193 178
    304   56 -  22 160 133
    372   56 -  22 160 133
    252   60 - 255 255 255
    280   60 - 145 193 178
    320   60 - 131 187 170
    352   60 - 254 254 254
    208  112 - 157 198 186
    344  112 -  94 173 152
    228  116 -  22 160 133
    344  116 -  94 173 152
    248  120 - 101 175 155
    280  120 - 205 224 218
    312  120 - 144 192 178
    324  120 - 205 224 218
    344  120 -  94 173 152
    356  120 -  86 171 149
    388  120 -  22 160 133
    344  124 -  94 173 152
     44  160 -  70  78  97
     56  160 -  70  78  97
    112  160 -   0   0 231
    136  160 -   0   0 231
    472  160 - 254  15   0
    516  160 - 254  15   0
     32  164 -  39  53  73
    432  164 - 250  60   0
    540  164 - 250  60   0
    456  168 - 245  82   0
    492  168 - 245  82   0
     24  176 -  70  78  97
     44  180 -  39  53  73
    120  180 -   0   0 231
    144  180 -   0   0 231
    472  180 - 230 125   0
    504  180 - 230 125   0
    524  180 - 230 125   0
    420  184 - 225 136   0
     64  188 -  39  53  73
    548  188 - 220 145   0
     24  192 -  70  78  97
    444  192 - 214 154   0
     44  200 -  39  53  73
    472  200 - 203 171   0
    512  200 - 203 171   0
    428  204 - 196 178   0
    548  212 - 183 192   0
    448  216 - 176 198   0
    492  216 - 176 198   0
    420  220 - 169 204   0
     48  224 - 221 221 222
    528  224 - 161 210   0
     60  228 - 222 222 223
    548  232 - 143 221   0
    100  236 - 233 233 234
    264  236 -  22 160 133
    444  236 - 133 227   0
    472  236 - 133 227   0
    508  236 - 133 227   0
     68  240 - 219 219 220
    424  240 - 122 232   0
    524  240 - 122 232   0
     28  248 - 255 255 255
    448  252 -  77 246   0
    488  252 -  77 246   0
    540  252 -  77 246   0
     68  256 - 218 218 219
    436  256 -  52 251   0
    460  256 -  52 251   0
    476  256 -  52 251   0
    516  256 -  52 251   0
     96  260 - 233 233 234
     56  264 - 200 200 201
    164  300 -   0 150 230
    120  304 -   0 150 230
    148  320 - 255 255 255
    176  324 -   0 150 230
    320  340 -  22 160 133
    124  344 -   0 150 230
    160  360 - 255 255 255
    124  380 -   0 150 230
      4  436 -  22 160 133
    504  436 -  10  11  13
    464  444 - 176 120  44
    532  444 - 193 118  55
    560  444 - 146  86  43
    440  448 - 198 114  54
    496  448 - 100  67  27
    192  456 - 155  89 182
    236  456 - 155  89 182
    384  460 - 155  89 182
    460  460 -  84  52  29
    488  460 -  33  27  30
    516  460 -  96  65  44
    544  460 -  34  30  22
    500  464 -  80  53  34
    572  464 - 127  68  34
    448  468 - 112  77  47
    476  468 -  69  46  30
    528  472 - 164 119  96
    528  476 - 163 119 102
    288  480 -   0   0   0
    332  480 -   0   0   0
    552  480 -  27  25  26
    308  484 - 155  89 182
    448  484 -  75  41  17
    508  484 - 210 128  72
    264  488 -   0   0   0
    268  488 -   0   0   0
    292  488 - 155  89 182
    320  488 -   6   2   8
    484  488 - 173 109  77
    572  488 - 106  77  57
    288  492 - 155  89 182
    304  492 - 155  89 182
    336  492 -   0   0   1
    532  496 -  67  42  28
    288  500 - 155  89 182
    436  500 -  27  26  34
    516  504 - 180 112  69
    464  508 - 181 100  44
    488  508 - 163  93  49
    508  508 - 175 108  69
    572  508 - 113  62  31
    516  512 - 169 104  61
    552  512 -  32  29  26
    208  516 - 155  89 182
    384  516 - 155  89 182
    472  520 - 183 114  68
    532  520 -  79  47  36
    440  524 - 173 106  54
    484  528 - 131  86  63
    556  528 -  37  22  15
    572  532 - 106  59  32
    460  536 - 198 108  53
    520  536 - 117  69  39
    540  540 - 191 114  60
    504  544 -  27  25  25
    564  544 - 133  74  38
    464  548 - 160  93  48
    156  552 - 213 229 224
     60  556 -  22 160 133
    116  556 - 143 192 177
    136  556 - 252 253 253
    444  556 - 219 225 225
    472  560 - 119  61  18
    524  560 - 161 101  57
    548  560 - 118  66  35
    568  560 -  27  27  27
    488  564 - 183 104  54
    436  568 -  27  26  27
    452  572 - 153  96  58
    504  572 -  95  55  20
";
