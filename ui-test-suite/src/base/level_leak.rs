use anyhow::Result;
use hilen::{
    dispatch::from_main,
    level::LevelManager,
    refs::Weak,
    ui::{Container, Setup, ViewTest, view},
    ui_test::{UITest, check_colors, get_test_name},
};

use crate::level::SkyboxLevel;

// A level is not in the view tree. A test that started one and did not
// stop it once kept it drawing under every test after it, 35 failures
// from one leak. Installing the next test view has to stop the level.
#[view]
struct LevelLeak {}

impl Setup for LevelLeak {}

impl ViewTest for LevelLeak {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        from_main(|| {
            LevelManager::set_level(SkyboxLevel::default());
        });

        check_colors(
            r"
               4    4 - #67b75b
             184    4 - #52b155
             300    4 - #52b155
             448    4 - #52b155
             592   32 - #369f5c
              92   40 - #52b155
             372   72 - #52b155
             240   84 - #3aa25b
             152  116 - #4fa760
             528  144 - #54aa5f
               8  152 - #369f5c
             400  168 - #41a35e
             268  176 - #369f5c
              96  200 - #369f5c
             188  216 - #369f5c
             544  272 - #39a05d
               4  300 - #5cb2ac
              80  300 - #5cb2ac
             152  300 - #5cb2ac
             228  300 - #51afab
             360  300 - #51afab
             424  300 - #5cb2ac
             488  300 - #5cb2ac
             592  300 - #5cb2ac
             300  304 - #65bae3
             340  428 - #65bae3
               4  456 - #65bae3
             484  460 - #65bae3
             192  464 - #65bae3
             336  560 - #65bae3
              80  592 - #65bae3
             592  592 - #65bae3
            ",
        )?;

        // The name stays so the run reports this test, not `Container`.
        UITest::set(Container::new(), 600, 600, false, get_test_name::<Self>());

        check_colors(
            r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
