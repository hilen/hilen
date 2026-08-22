use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::{Weak, weak_from_ref},
    ui::{
        Button, CellRegistry, Label, Setup, TableData, TableView, UIManager, View, ViewData, ViewSubviews,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_touches},
};

#[view]
struct ScaleView {
    data: Vec<String>,

    #[init]
    label:  Label,
    button: Button,
    table:  TableView,

    tr_button: Button,
    bl_button: Button,
    br_button: Button,
}

impl Setup for ScaleView {
    fn setup(mut self: Weak<Self>) {
        self.label.set_text("Label").set_color(WHITE);
        self.label.place().tl(20).size(150, 80);

        self.button.set_text("Button");
        self.button.place().below(self.label, 20);

        self.table.place().size(200, 280).br(20);
        self.table.set_data_source(self).register_cell::<Label>();

        self.tr_button.place().tr(20).size(50, 50);
        self.bl_button.place().bl(20).size(50, 50);
        self.br_button.place().br(20).size(50, 50);

        let mut this = self;
        self.apply_to::<Button>(move |b| {
            let b = weak_from_ref(b);
            b.on_tap(move || {
                this.data.push(b.label().to_string());
            });
        });
    }
}

impl TableData for ScaleView {
    fn number_of_cells(&self) -> usize {
        4
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(index);
        cell.set_color(WHITE);
        cell
    }
}

impl ViewTest for ScaleView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_default_scale(view)?;
        check_downscaled(view)?;
        check_upscaled(view)?;

        from_main(move || {
            UIManager::override_scale(1);
        });

        Ok(())
    }
}

fn check_default_scale(view: Weak<ScaleView>) -> Result<()> {
    inject_touches(
        "
            39   40   b
            39   40   e
            61   541  b
            61   541  e
            538  539  b
            538  539  e
            537  60   b
            537  60   e
            551  83   b
            551  83   e
            517  46   b
            518  46   e
            182  167  b
            182  167  e
            78   171  b
            78   171  e
            82   210  b
            82   210  e
            51   518  b
            51   518  e
            88   560  b
            88   560  e
            516  557  b
            516  557  e

        ",
    );

    let data = from_main(move || view.data.clone());

    assert_eq!(
        data,
        [
            "ScaleView.bl_button: Button",
            "ScaleView.br_button: Button",
            "ScaleView.tr_button: Button",
            "ScaleView.button: Button",
        ]
        .map(ToOwned::to_owned)
        .into_iter()
        .collect::<Vec<_>>()
    );

    check_colors(
        r"
             576   20 - #ffffff
             128   48 - #565656
              60   60 - #000000
              88   60 - #626262
             128   60 - #565656
              96   64 - #f2f2f2
              64   68 - #7c7c7c
              68   68 - #7c7c7c
             128   68 - #565656
             360  112 - #597c95
              52  148 - #303030
              56  148 - #303030
              88  152 - #c0c0c0
              72  156 - #181818
             100  156 - #000000
              56  160 - #686868
              72  160 - #181818
              80  160 - #6c6c6c
             112  160 - #ffffff
              88  164 - #c0c0c0
              56  168 - #7c7c7c
             128  168 - #020202
             476  320 - #ffffff
             472  324 - #cecece
             484  324 - #464646
             232  364 - #597c95
             480  380 - #000000
             484  420 - #010101
             480  484 - #000000
              20  532 - #ffffff
             576  576 - #ffffff
             140  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_downscaled(mut view: Weak<ScaleView>) -> Result<()> {
    from_main(move || {
        UIManager::override_scale(0.6);
        view.data.clear();
    });

    inject_touches(
        "
            53   958  b
            53   958  e
            53   958  b
            53   958  e
            53   956  b
            53   956  e
            948  942  b
            948  942  e
            955  39   b
            955  39   e
            960  95   b
            960  95   e
            899  52   b
            899  52   e
            128  185  b
            128  185  e

        ",
    );

    let data = from_main(move || view.data.clone());

    assert_eq!(
        data,
        [
            "ScaleView.bl_button: Button",
            "ScaleView.bl_button: Button",
            "ScaleView.bl_button: Button",
            "ScaleView.br_button: Button",
            "ScaleView.tr_button: Button",
            "ScaleView.button: Button",
        ]
        .map(ToOwned::to_owned)
        .into_iter()
        .collect::<Vec<_>>()
    );

    check_colors(
        r"
             224    4 - #597c95
             584   12 - #ffffff
              36   32 - #000000
              60   36 - #ffffff
              36   40 - #000000
              52   40 - #000000
              32   88 - #e9e9e9
              32   92 - #d8d8d8
              60   92 - #000000
              48   96 - #414141
              56   96 - #ffffff
              68   96 - #ffffff
              76  100 - #010101
             336  124 - #597c95
             592  228 - #597c95
             388  296 - #597c95
               4  316 - #597c95
             220  368 - #597c95
             584  420 - #ffffff
             524  432 - #000000
             528  432 - #ffffff
             528  436 - #ffffff
             468  456 - #ffffff
             528  464 - #010101
             584  480 - #ffffff
             528  500 - #242424
             528  524 - #000000
             580  532 - #ffffff
             468  536 - #ffffff
             584  584 - #ffffff
               4  592 - #597c95
             248  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_upscaled(mut view: Weak<ScaleView>) -> Result<()> {
    from_main(move || {
        UIManager::override_scale(1.5);
        view.data.clear();
    });

    inject_touches(
        "
            40   389  b
            40   389  e
            44   363  b
            44   363  e
            40   318  b
            40   318  e
            307  356  b
            308  356  e
            347  356  b
            347  356  e
            390  355  b
            390  355  e
            348  86   b
            348  86   e
            352  45   b
            352  45   e
            349  10   b
            349  10   e
            75   112  b
            75   112  e
            74   135  b
            74   135  e
            63   185  b
            63   185  e
            59   215  b
            59   215  e

        ",
    );

    let data = from_main(move || view.data.clone());

    assert_eq!(
        data,
        [
            "ScaleView.bl_button: Button",
            "ScaleView.br_button: Button",
            "ScaleView.tr_button: Button",
            "ScaleView.button: Button",
            "ScaleView.button: Button"
        ]
        .map(ToOwned::to_owned)
        .into_iter()
        .collect::<Vec<_>>()
    );

    check_colors(
        r"
             592    4 - #597c95
             192   72 - #818181
              88   76 - #717171
             196   88 - #232323
             160   92 - #010101
             128   96 - #979797
              88  104 - #717171
             196  104 - #232323
             428  176 - #000000
              80  228 - #9c9c9c
             148  228 - #d4d4d4
             120  236 - #a2a2a2
             148  236 - #d4d4d4
              84  240 - #1c1c1c
             148  240 - #d4d4d4
             176  240 - #ffffff
             120  244 - #a2a2a2
             148  244 - #d4d4d4
             208  244 - #010101
             148  248 - #d4d4d4
             420  248 - #000000
              84  252 - #bababa
             192  256 - #989898
             568  256 - #ffffff
             420  320 - #000000
             412  352 - #000000
             416  400 - #ffffff
             424  420 - #ffffff
             272  440 - #ffffff
              32  568 - #ffffff
             264  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
