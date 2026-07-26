use anyhow::Result;
use log::debug;
use test_engine::{
    RenderPass,
    dispatch::from_main,
    refs::Weak,
    ui::{ViewCallbacks, view},
    ui_test::{UITest, check_colors},
};

use crate::{
    occlusion::render_occlusion,
    path::render_path,
    render::Case::{Occlusion, Path},
};

#[derive(Default)]
enum Case {
    #[default]
    Occlusion,
    Path,
}

#[view]
struct RenderTestView {
    case: Case,
}

impl ViewCallbacks for RenderTestView {
    fn before_render(&self, pass: &mut RenderPass) {
        match self.case {
            Occlusion => {
                render_occlusion(pass);
            }
            Path => {
                render_path(pass);
            }
        }
    }
}

pub async fn test_render() -> Result<()> {
    debug!("Test render");

    let view = UITest::start::<RenderTestView>();

    check_occlusion()?;
    check_path(view)?;

    Ok(())
}

fn check_occlusion() -> Result<()> {
    check_colors(
        r"
              36   36 - #597c95
              47   41 - #597c95
              54   72 - #ff0000
              74   75 - #ff0000
              75   75 - #ff0000
              94   89 - #ff0000
             109  118 - #00ff00
             111  111 - #00ff00
             124  112 - #00ff00
             124  115 - #00ff00
             124  118 - #00ff00
             143  134 - #0000e7
             144  144 - #0000e7
             147  153 - #597c95
             167  305 - #597c95
             152  305 - #597c95
             130  286 - #0000e7
             117  272 - #0000e7
             115  264 - #0000e7
             112  262 - #0000e7
              98  256 - #00ff00
              96  253 - #00ff00
              94  248 - #00ff00
              89  235 - #00ff00
              78  231 - #00ff00
              72  221 - #ff0000
              68  220 - #ff0000
              62  216 - #ff0000
              62  212 - #ff0000
              45  207 - #597c95
              43  189 - #597c95
              43  348 - #597c95
              56  356 - #ff0000
              60  367 - #ff0000
              73  378 - #ff0000
              76  378 - #00ff00
              78  382 - #00ff00
              80  382 - #00ff00
              86  382 - #00ff00
             105  409 - #0000e7
             105  396 - #00ff00
              96  400 - #00ff00
              93  428 - #597c95
             110  418 - #0000e7
             124  424 - #0000e7
             131  425 - #0000e7
             133  426 - #0000e7
             135  431 - #0000e7
             148  455 - #597c95
             191   95 - #597c95
             216   95 - #00ff00
             230   95 - #00ff00
             230   95 - #00ff00
             232   95 - #00ff00
             232   95 - #00ff00
             232   95 - #00ff00
             241   95 - #00ff00
             244   95 - #00ff00
             252   95 - #00ff00
             254   98 - #444444
             256   98 - #444444
             273   98 - #444444
             273   98 - #444444
             280   98 - #00ff00
             284   98 - #00ff00
             292   98 - #00ff00
             315   98 - #597c95
             315   98 - #597c95
             262  145 - #00ff00
             262  143 - #00ff00
             262  136 - #00ff00
             262   99 - #444444
             262   99 - #444444
             262   92 - #444444
             260   83 - #00ff00
             254   75 - #00ff00
             251   65 - #00ff00
             251   64 - #00ff00
             254   31 - #597c95
        ",
    )?;

    debug!("Occlusion: OK");

    Ok(())
}

fn check_path(mut view: Weak<RenderTestView>) -> Result<()> {
    from_main(move || {
        view.case = Path;
    });

    check_colors(
        r"
             167  243 - #597c95
             181  243 - #597c95
             187  243 - #597c95
             220  243 - #0000e7
             239  239 - #597c95
             245  239 - #597c95
             294  243 - #0000e7
             330  247 - #0000e7
             342  247 - #0000e7
             366  254 - #597c95
             366  302 - #597c95
             266  333 - #0000e7
             274  359 - #597c95
             244  376 - #597c95
             219  360 - #0000e7
             206  381 - #597c95
             205  398 - #597c95
             226  399 - #597c95
             249  399 - #597c95
             240  326 - #0000e7
             243  235 - #597c95
             249  210 - #597c95
             195  230 - #597c95
             184  283 - #597c95
             222  229 - #0000e7
             379  233 - #0000e7
             369  192 - #597c95
             369  294 - #597c95
             493  184 - #597c95
             473  213 - #ff0000
             440  234 - #597c95
             439  345 - #597c95
             468  387 - #ff0000
             484  422 - #597c95
             273  307 - #0000e7
             319  378 - #597c95
              69  331 - #597c95
             194  116 - #597c95
        ",
    )?;

    debug!("Path: OK");

    Ok(())
}
