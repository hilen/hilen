use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::Apply,
    level::LevelManager,
    refs::Weak,
    ui::{ImageView, Setup, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use crate::level::SkyboxLevel;

#[view]
struct Transparency {
    #[init]
    background: ImageView,

    view_1: ImageView,
    view_2: ImageView,
    view_3: ImageView,
    view_4: ImageView,
}

impl Setup for Transparency {
    fn setup(self: Weak<Self>) {
        self.background.set_image("gradient.png").place().back();

        self.view_1.set_image("wood-window.png");
        self.view_2.set_image("wood-window.png").place().tl(50);
        self.view_3.set_image("wood-window.png").place().tl(100);
        self.view_4.set_image("wood-window.png").place().tl(150);

        [self.view_1, self.view_2, self.view_3, self.view_4].apply(|v| {
            v.place().size(280, 280);
        });
    }
}

impl ViewTest for Transparency {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        from_main(|| {
            LevelManager::set_level(SkyboxLevel::default());
        });

        from_main(|| {
            LevelManager::stop_level();
        });

        check_colors(
            r"
             100    4 - #60d14a
             308    4 - #367b80
             592    4 - #0305ef
             380    8 - #285d9a
             240   28 - #459765
             512   28 - #1425ca
              44   36 - #597c95
              40   40 - #597c95
             592  100 - #2c2cc9
               4  152 - #67bc37
             560  168 - #4949a7
               4  216 - #6da12f
             592  224 - #616198
              36  268 - #778c2b
             560  272 - #757581
             292  308 - #597c95
             592  332 - #8e8e72
               4  348 - #8e6d22
             560  380 - #a2a25e
              16  432 - #aa4f1f
             592  444 - #bdbd56
             556  548 - #597c95
             104  556 - #d94126
             212  556 - #db652b
             364  556 - #e09f39
             424  560 - #e4b73f
               4  592 - #e63323
             160  592 - #e85428
             316  592 - #ec8d34
             444  592 - #f2bf42
             508  592 - #f5d949
             592  592 - #fbfb53
        ",
        )?;

        Ok(())
    }
}
