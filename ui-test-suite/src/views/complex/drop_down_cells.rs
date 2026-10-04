use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{
        CLEAR, CellRegistry, Color, Container, DropDown, DropDownData, ImageView, Label, Setup,
        TextAlignment, ViewData, ViewTest, ViewTouch, WeakView, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

const ROW: f32 = 44.0;
const PICKED: Color = Color::rgb(0.80, 0.90, 1.0);
const HOVERED: Color = Color::rgb(0.90, 0.95, 1.0);
const BOX: Color = Color::rgb(0.98, 0.98, 1.0);

/// A row with a picture and a name. It draws its own picked and hovered
/// look, the drop down adds nothing around it.
#[view]
struct AnimalCell {
    picked: bool,

    #[init]
    picture: ImageView,
    name:    Label,
}

impl Setup for AnimalCell {
    fn setup(self: Weak<Self>) {
        self.picture.place().l(10).center_y().size(28, 28);

        self.name.set_alignment(TextAlignment::Left).set_text_size(16);
        self.name.place().l(48).r(8).tb(0);

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| self.paint(hovered));
    }
}

impl AnimalCell {
    fn set(mut self: Weak<Self>, name: &str, picture: &str, picked: bool) {
        self.picked = picked;
        self.name.set_text(name);
        self.picture.set_image(picture);
        self.paint(false);
    }

    fn paint(self: Weak<Self>, hovered: bool) {
        self.set_color(if self.picked {
            PICKED
        } else if hovered {
            HOVERED
        } else {
            CLEAR
        });
    }
}

/// A custom drop down is its own view. It holds the engine drop down,
/// owns the data and is the data source.
#[view]
struct AnimalDropDown {
    animals: Vec<(&'static str, &'static str)>,
    picked:  String,

    #[init]
    drop: DropDown,
}

impl Setup for AnimalDropDown {
    fn setup(mut self: Weak<Self>) {
        self.animals = vec![
            ("Cat", "cat.png"),
            ("Ball", "test/ball.png"),
            ("Palm", "test/palm.png"),
        ];

        self.drop
            .set_color(BOX)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(Color::rgb(0.80, 0.84, 0.90));
        self.drop.place().back();
        self.drop.register_cell::<AnimalCell>().set_data_source(self);
    }
}

impl AnimalDropDown {
    fn cell(&self, index: usize, registry: &mut CellRegistry, picked: bool) -> WeakView {
        let (name, picture) = self.animals[index];
        let cell = registry.cell::<AnimalCell>();
        cell.set(name, picture, picked);
        cell
    }
}

impl DropDownData for AnimalDropDown {
    fn number_of_rows(&self) -> usize {
        self.animals.len()
    }

    fn row_height(&self, _: usize) -> f32 {
        ROW
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Option<WeakView> {
        Some(self.cell(index, registry, index == self.drop.selected_index()))
    }

    // The closed box always shows the picked row, so it does not wear the
    // picked color.
    fn setup_box_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Option<WeakView> {
        Some(self.cell(index, registry, false))
    }

    fn row_selected(&mut self, index: usize) {
        self.animals[index].0.clone_into(&mut self.picked);
    }
}

/// A drop down with cells of its own: a picture and a name in every row
/// and in the closed box. Pins that the data source makes the cells, that
/// the box follows the pick, and that a pick reaches the data source.
#[view]
struct DropDownCells {
    #[init]
    card:    Container,
    animals: AnimalDropDown,
}

impl Setup for DropDownCells {
    fn setup(self: Weak<Self>) {
        self.card.set_color(Color::rgb(0.92, 0.94, 0.97)).set_corner_radius(16);
        self.card.place().tl(40).size(320, 300);

        self.animals.place().t(70).l(80).size(220, ROW);
    }
}

impl ViewTest for DropDownCells {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        check_colors(COLORS_1)?;

        // The box, it opens the list under itself.
        inject_touches("190 92 b\n190 92 e");
        check_colors(COLORS_2)?;

        // The panel opens 6 under the box at y 120, its rows start 4
        // lower and are 44 tall, so "Ball" spans 168..212.
        inject_touches("190 190 b\n190 190 e");
        anyhow::ensure!(view.animals.picked == "Ball", "picked {}", view.animals.picked);
        anyhow::ensure!(view.animals.drop.selected_index() == 1);
        check_colors(COLORS_3)?;

        // Open again, "Ball" is the picked row now.
        inject_touches("190 92 b\n190 92 e");
        check_colors(COLORS_4)?;

        Ok(())
    }
}

const COLORS_1: &str = r"
       4    4 - #597c95
      84    4 - #597c95
     436    4 - #597c95
     592    4 - #597c95
     112   40 - #ebf0f7
     152   40 - #ebf0f7
     208   40 - #ebf0f7
     276   40 - #ebf0f7
      48   44 - #ebf0f7
     512   48 - #597c95
     356   52 - #ebf0f7
     312   60 - #ebf0f7
      80   80 - #ccd6e6
      92   80 - #e8c2c7
      96   80 - #e7c0c2
     100   80 - #e3bcbb
     104   80 - #e0b6b7
     108   80 - #dfb4b5
     112   80 - #dcaeae
     200   80 - #fafaff
       4   84 - #597c95
      80   84 - #ccd6e6
      92   84 - #e7bfc0
     100   84 - #d2ad9f
     108   84 - #daaaa9
     112   84 - #d49f9d
      80   88 - #ccd6e6
      92   88 - #e6bcbe
      80   92 - #ccd6e6
      92   92 - #e7b8b9
     100   92 - #d2ae96
     112   92 - #b6937d
     116   92 - #c99697
     148   92 - #fafaff
     164   92 - #b3b3b7
     236   92 - #fafaff
      80   96 - #ccd6e6
      92   96 - #ecc6c0
      96   96 - #e7c9bd
     100   96 - #d1ad98
     104   96 - #caa38d
     108   96 - #c8a791
     344   96 - #ebf0f7
     432   96 - #597c95
     592   96 - #597c95
      80  100 - #ccd6e6
      96  100 - #e5c3b5
     100  100 - #d7b6a1
     104  100 - #cdab95
     300  100 - #ebf0f7
      80  104 - #cfd9e7
      92  104 - #dca2a2
      96  104 - #e6c1b9
     100  104 - #d8b2a3
     104  104 - #d0ac98
      40  108 - #ebf0f7
     200  132 - #ebf0f7
     288  136 - #ebf0f7
     356  136 - #ebf0f7
     508  136 - #597c95
     148  144 - #ebf0f7
     252  148 - #ebf0f7
      40  152 - #ebf0f7
      92  156 - #ebf0f7
     308  168 - #ebf0f7
     592  176 - #597c95
     436  180 - #597c95
      68  188 - #ebf0f7
     200  188 - #ebf0f7
     132  192 - #ebf0f7
     356  200 - #ebf0f7
     288  204 - #ebf0f7
      40  216 - #ebf0f7
     252  216 - #ebf0f7
     524  216 - #597c95
      92  220 - #ebf0f7
     160  232 - #ebf0f7
     308  236 - #ebf0f7
     348  236 - #ebf0f7
     200  256 - #ebf0f7
     456  260 - #597c95
     592  260 - #597c95
     100  268 - #ebf0f7
     252  272 - #ebf0f7
     356  272 - #ebf0f7
      40  276 - #ebf0f7
     152  280 - #ebf0f7
     520  280 - #597c95
     300  296 - #ebf0f7
     232  304 - #ebf0f7
     196  312 - #ebf0f7
      96  320 - #ebf0f7
     420  324 - #597c95
      48  336 - #ebf0f7
     144  336 - #ebf0f7
     252  336 - #ebf0f7
     296  336 - #ebf0f7
     348  336 - #ebf0f7
     544  340 - #597c95
     464  384 - #597c95
       8  392 - #597c95
     324  396 - #597c95
     524  400 - #597c95
     264  408 - #597c95
     384  412 - #597c95
      96  424 - #597c95
     580  424 - #597c95
     196  432 - #597c95
     484  452 - #597c95
       4  460 - #597c95
     316  460 - #597c95
     252  468 - #597c95
     144  484 - #597c95
     372  488 - #597c95
     432  500 - #597c95
     532  504 - #597c95
     220  520 - #597c95
      60  524 - #597c95
     592  528 - #597c95
     324  536 - #597c95
     136  556 - #597c95
     488  588 - #597c95
       4  592 - #597c95
      76  592 - #597c95
     200  592 - #597c95
     272  592 - #597c95
     380  592 - #597c95
     592  592 - #597c95
";

const COLORS_2: &str = r"
     592    4 - #597c95
     200   40 - #ebf0f7
     264   40 - #ebf0f7
     352   44 - #ebf0f7
      80   84 - #00daff
     100   84 - #d2ad9f
     112   84 - #d49f9d
      80   88 - #00daff
      80   92 - #00daff
     112   92 - #b6937d
     116   92 - #c99697
     148   92 - #fafaff
     164   92 - #b3b3b7
      80   96 - #00daff
     100   96 - #d1ad98
     104   96 - #caa38d
     108   96 - #c8a791
      80  100 - #00daff
     100  100 - #d7b6a1
     104  100 - #cdab95
      92  104 - #dca2a2
     100  104 - #d8b2a3
     104  104 - #d0ac98
     356  108 - #ebf0f7
     108  120 - #ccd6e6
     144  120 - #ccd6e6
     168  120 - #ccd6e6
     188  120 - #ccd6e6
     208  120 - #ccd6e6
     228  120 - #ccd6e6
     264  120 - #ccd6e6
     280  120 - #ccd6e6
     296  120 - #cdd1d8
      76  128 - #ced2d9
     120  132 - #d8acac
     300  132 - #b4b8bd
     492  132 - #597c95
     112  136 - #dfb1b2
     120  136 - #d3a1a0
     104  140 - #d7baa9
     108  140 - #cab098
     112  140 - #bea28b
     120  144 - #cb9898
     152  144 - #cce5ff
     156  144 - #cce5ff
     108  148 - #c19680
     116  148 - #b79581
     152  148 - #cce5ff
     300  148 - #b4b8bd
      76  152 - #c9ced3
     104  152 - #d3b39d
     108  152 - #caa990
     112  152 - #cbab96
     104  156 - #d7b2a1
     108  156 - #ceaa95
     300  164 - #b4b8bd
     300  176 - #b4b8bd
      76  180 - #c9ced3
     104  180 - #6b6b6b
     116  184 - #2c2c2c
     116  188 - #242424
     120  188 - #1e1e1e
     168  188 - #000000
     172  188 - #000000
     116  192 - #212121
     120  192 - #2a2a2a
     152  192 - #fafaff
     164  192 - #000000
     168  192 - #000000
     172  192 - #000000
     300  192 - #b4b8bd
     100  196 - #2b2b2b
     108  196 - #242424
     112  196 - #202020
     236  196 - #fafaff
     104  200 - #323232
     112  200 - #363636
     300  204 - #b4b8bd
      76  208 - #c9ced3
     104  220 - #d1baa6
     112  220 - #cdb4a0
     300  220 - #b4b8bd
      96  224 - #b78d6a
     112  224 - #c4bcb8
     104  228 - #e6ca99
     152  228 - #1f1f20
      76  232 - #c9ced3
     152  232 - #fafaff
     168  232 - #000000
     304  232 - #d0d4da
      96  236 - #c69f7a
     116  236 - #d1a16f
     164  236 - #000000
     168  236 - #000000
     172  236 - #000000
     176  236 - #dfdfe3
     104  240 - #fee2b0
     112  240 - #fde1af
     300  240 - #b4b8bd
     300  252 - #b4b8bd
      80  256 - #b1b5ba
     100  260 - #9ea2a6
     120  260 - #9ea2a6
     132  260 - #9ea2a6
     152  260 - #9ea2a6
     196  260 - #9ea2a6
     212  260 - #9ea2a6
     248  260 - #9ea2a6
     272  260 - #9ea2a6
     288  260 - #9ea2a6
     456  260 - #597c95
     592  260 - #597c95
     168  264 - #bbbfc5
     184  264 - #bbbfc5
     228  264 - #bbbfc5
     260  264 - #bbbfc5
     140  268 - #d6dae1
     356  276 - #ebf0f7
      48  336 - #ebf0f7
     148  336 - #ebf0f7
     260  336 - #ebf0f7
     348  336 - #ebf0f7
     580  424 - #597c95
     140  500 - #597c95
     432  500 - #597c95
       4  592 - #597c95
     272  592 - #597c95
     592  592 - #597c95
";

const COLORS_3: &str = r"
       4    4 - #597c95
     140    4 - #597c95
     336    4 - #597c95
     436    4 - #597c95
     592    4 - #597c95
     116   40 - #ebf0f7
     168   40 - #ebf0f7
     216   40 - #ebf0f7
     284   40 - #ebf0f7
      48   44 - #ebf0f7
     512   48 - #597c95
     316   64 - #ebf0f7
     356   64 - #ebf0f7
     204   76 - #fafaff
     272   76 - #fafaff
      80   80 - #ccd6e6
      92   80 - #fafaff
     104   80 - #444444
       4   84 - #597c95
      80   84 - #ccd6e6
     108   84 - #525252
     112   84 - #2d2d2d
      80   88 - #ccd6e6
     112   88 - #272727
     116   88 - #191919
     148   88 - #fafaff
     164   88 - #000000
     168   88 - #000000
      80   92 - #ccd6e6
     112   92 - #222222
     116   92 - #242424
     164   92 - #000000
     168   92 - #000000
     436   92 - #597c95
      80   96 - #ccd6e6
      96   96 - #3a3a3a
     100   96 - #393939
     104   96 - #2d2d2d
     108   96 - #232323
     112   96 - #202020
     116   96 - #2a2a2a
     236   96 - #fafaff
     592   96 - #597c95
      80  100 - #ccd6e6
     100  100 - #262626
     104  100 - #222222
     108  100 - #232323
      80  104 - #cfd9e7
      92  104 - #fafaff
     104  104 - #3a3a3a
     300  104 - #ebf0f7
     340  104 - #ebf0f7
      40  108 - #ebf0f7
     204  136 - #ebf0f7
     508  136 - #597c95
     288  140 - #ebf0f7
     356  140 - #ebf0f7
     156  144 - #ebf0f7
      40  152 - #ebf0f7
     252  152 - #ebf0f7
      92  156 - #ebf0f7
     304  176 - #ebf0f7
     592  176 - #597c95
     436  180 - #597c95
     196  184 - #ebf0f7
      68  188 - #ebf0f7
     132  192 - #ebf0f7
     356  200 - #ebf0f7
     284  208 - #ebf0f7
      40  216 - #ebf0f7
     248  216 - #ebf0f7
     524  216 - #597c95
      92  220 - #ebf0f7
     208  220 - #ebf0f7
     160  232 - #ebf0f7
     308  236 - #ebf0f7
     348  236 - #ebf0f7
     200  256 - #ebf0f7
     456  260 - #597c95
     592  260 - #597c95
     100  268 - #ebf0f7
     252  272 - #ebf0f7
     356  272 - #ebf0f7
      40  276 - #ebf0f7
     152  280 - #ebf0f7
     520  280 - #597c95
     300  296 - #ebf0f7
     232  304 - #ebf0f7
     196  312 - #ebf0f7
      96  320 - #ebf0f7
     420  324 - #597c95
      48  336 - #ebf0f7
     144  336 - #ebf0f7
     252  336 - #ebf0f7
     296  336 - #ebf0f7
     348  336 - #ebf0f7
     544  340 - #597c95
     200  372 - #597c95
     464  384 - #597c95
       8  392 - #597c95
     324  396 - #597c95
     524  400 - #597c95
     264  408 - #597c95
     384  412 - #597c95
      96  424 - #597c95
     580  424 - #597c95
     196  432 - #597c95
     484  452 - #597c95
       4  460 - #597c95
     316  460 - #597c95
     252  468 - #597c95
     144  484 - #597c95
     372  488 - #597c95
     432  500 - #597c95
     532  504 - #597c95
     220  520 - #597c95
      60  524 - #597c95
     592  528 - #597c95
     324  536 - #597c95
     136  556 - #597c95
     432  560 - #597c95
     488  588 - #597c95
       4  592 - #597c95
      76  592 - #597c95
     200  592 - #597c95
     272  592 - #597c95
     380  592 - #597c95
     592  592 - #597c95
";

const COLORS_4: &str = r"
     592    4 - #597c95
     200   40 - #ebf0f7
     260   40 - #ebf0f7
     352   44 - #ebf0f7
      92   80 - #fafaff
     104   80 - #444444
      80   84 - #00daff
     108   84 - #525252
      80   88 - #00daff
     148   88 - #fafaff
     164   88 - #000000
      80   92 - #00daff
     112   92 - #222222
     116   92 - #242424
     168   92 - #000000
      80   96 - #00daff
      96   96 - #3a3a3a
     100   96 - #393939
     108   96 - #232323
      80  100 - #00daff
      92  104 - #fafaff
     104  104 - #3a3a3a
     356  108 - #ebf0f7
     144  120 - #ccd6e6
     188  120 - #ccd6e6
     224  120 - #ccd6e6
     260  120 - #ccd6e6
     296  120 - #cdd1d8
     488  128 - #597c95
      76  132 - #c9ced4
     112  132 - #ddb3b4
     116  132 - #daaeaf
     120  132 - #d8acac
     300  132 - #b4b8bd
     108  136 - #e1b5b6
     112  136 - #dfb1b2
     120  136 - #d3a1a0
     104  140 - #d7baa9
     108  140 - #cab098
     112  140 - #bea28b
     120  144 - #cb9898
     152  144 - #fafaff
     156  144 - #fafaff
     100  148 - #e9cabe
     108  148 - #c19680
     116  148 - #b79581
     152  148 - #fafaff
     300  148 - #b4b8bd
     104  152 - #d3b39d
     108  152 - #caa990
     112  152 - #cbab96
     104  156 - #d7b2a1
     108  156 - #ceaa95
      76  168 - #c9ced3
     300  176 - #b4b8bd
     104  180 - #6b6b6b
     116  184 - #2c2c2c
      80  188 - #ccd6e6
     116  188 - #242424
     120  188 - #1e1e1e
     168  188 - #000000
     172  188 - #000000
     116  192 - #212121
     120  192 - #2a2a2a
     152  192 - #cce5ff
     164  192 - #000000
     168  192 - #000000
     172  192 - #000000
     300  192 - #b4b8bd
     100  196 - #2b2b2b
     108  196 - #242424
     112  196 - #202020
     236  196 - #cce5ff
     104  200 - #323232
     112  200 - #363636
     300  204 - #b4b8bd
      76  208 - #c9ced3
     112  220 - #cdb4a0
     300  220 - #b4b8bd
      96  224 - #b78d6a
     112  224 - #c4bcb8
     104  228 - #e6ca99
     152  228 - #1f1f20
      76  232 - #c9ced3
     152  232 - #fafaff
     168  232 - #000000
     304  232 - #d0d4da
      96  236 - #c69f7a
     116  236 - #d1a16f
     164  236 - #000000
     168  236 - #000000
     172  236 - #000000
     176  236 - #dfdfe3
     104  240 - #fee2b0
     112  240 - #fde1af
     300  244 - #b4b8bd
     592  252 - #597c95
      80  256 - #b1b5ba
     300  256 - #b8bcc1
     112  260 - #9ea2a6
     124  260 - #9ea2a6
     136  260 - #9ea2a6
     156  260 - #9ea2a6
     168  260 - #9ea2a6
     200  260 - #9ea2a6
     220  260 - #9ea2a6
     248  260 - #9ea2a6
     260  260 - #9ea2a6
     280  260 - #9ea2a6
     292  260 - #a1a4a9
     100  264 - #bbbfc5
     184  264 - #bbbfc5
     240  264 - #bbbfc5
     144  268 - #d6dae1
     356  268 - #ebf0f7
     212  324 - #ebf0f7
     356  328 - #ebf0f7
      48  336 - #ebf0f7
     148  336 - #ebf0f7
     272  336 - #ebf0f7
     516  420 - #597c95
       4  460 - #597c95
     332  464 - #597c95
     140  500 - #597c95
     436  560 - #597c95
       4  592 - #597c95
     280  592 - #597c95
     592  592 - #597c95
";
