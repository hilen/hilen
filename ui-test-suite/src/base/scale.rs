use anyhow::Result;
use test_engine::{
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
               4    4 - #597c95
             300    4 - #597c95
             592    4 - #597c95
             168   24 - #ffffff
              80   56 - #ffffff
             100   56 - #ffffff
              80   64 - #ffffff
             100   64 - #ffffff
             532   68 - #ffffff
             384  108 - #597c95
              56  152 - #ffffff
             104  160 - #ffffff
             116  160 - #ffffff
              56  164 - #ffffff
             592  184 - #597c95
             272  240 - #597c95
             132  300 - #597c95
             392  304 - #ffffff
             568  304 - #ffffff
             480  320 - #ffffff
               4  360 - #597c95
             384  392 - #ffffff
             576  392 - #ffffff
             224  404 - #597c95
             480  468 - #ffffff
             480  476 - #ffffff
             572  484 - #ffffff
             384  496 - #ffffff
             172  544 - #597c95
              24  576 - #ffffff
             576  576 - #ffffff
             312  592 - #597c95
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
               4    4 - #597c95
             216    4 - #597c95
             428    4 - #597c95
             592    4 - #597c95
              96   16 - #ffffff
              60   36 - #ffffff
             560   40 - #ffffff
              16   52 - #ffffff
             100   56 - #ffffff
              36   96 - #ffffff
              68   96 - #ffffff
             340  108 - #597c95
             472  152 - #597c95
             200  196 - #597c95
             592  224 - #597c95
             404  288 - #597c95
               4  324 - #597c95
             228  360 - #597c95
             480  424 - #ffffff
             584  424 - #ffffff
             528  432 - #ffffff
             100  452 - #597c95
             560  460 - #ffffff
             324  468 - #597c95
             472  468 - #ffffff
             524  484 - #ffffff
             584  496 - #ffffff
             472  536 - #ffffff
             536  536 - #ffffff
              16  584 - #ffffff
             584  584 - #ffffff
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
             356    4 - #597c95
             568   32 - #ffffff
             196   72 - #000000
             124   84 - #ffffff
             180   84 - #ffffff
             148   92 - #ffffff
             168   92 - #010101
              92  100 - #000000
             116  100 - #010101
             196  100 - #000000
             424  184 - #ffffff
             296  188 - #ffffff
              80  220 - #010101
              92  228 - #ffffff
             156  240 - #ffffff
             100  244 - #000000
             180  244 - #000000
             420  248 - #010101
             568  264 - #ffffff
             416  324 - #ffffff
             428  324 - #010101
             420  348 - #000000
               4  360 - #597c95
             272  380 - #ffffff
             420  408 - #010101
             568  416 - #ffffff
             420  424 - #010101
             132  436 - #597c95
             568  568 - #ffffff
             408  584 - #597c95
               4  592 - #597c95
             248  592 - #597c95
        ",
    )?;

    Ok(())
}
