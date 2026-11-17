use std::{env::temp_dir, fs::write};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

/// Each fixture is 2 frames of 5 flat bars, HEVC main 10 with lossless
/// coding, so the bars decode to the exact 10 bit codes they were made from.
///
/// PQ in BT.2020: black, gray at 100 nits, gray at 203 nits, white at 1000
/// nits, and a BT.709 orange at 100 nits.
const PQ: &[u8] = include_bytes!("pq.mp4");
/// HLG in BT.2020: black, gray at signal 0.5, gray at signal 0.75, white at
/// signal 1, and the same orange.
const HLG: &[u8] = include_bytes!("hlg.mp4");
/// SDR in BT.709: black, middle gray, white, and the red and blue of the 8
/// bit playback fixture.
const SDR: &[u8] = include_bytes!("sdr10.mp4");

const BARS: &str = r"
    592    4 - #597c95
    388   40 - #e7e6e6
    468   40 - #ad7348
    264   76 - #bdbdbd
     84   88 - #0e1317
    112  100 - #597c95
     84  108 - #283743
    504  108 - #ad7348
    324  140 - #bdbdbd
    392  192 - #ffffff
    464  192 - #845838
    260  204 - #000000
    516  240 - #845838
     72  244 - #161e24
    120  256 - #597c95
    372  280 - #e6e6e6
    304  340 - #808080
    440  340 - #e94c3d
    516  356 - #3497db
     72  388 - #11171c
    392  388 - #ca5449
    164  392 - #597c95
     60  400 - #415a6d
    100  400 - #000000
    140  408 - #283743
    456  408 - #688bba
    324  416 - #808080
    516  444 - #3497db
    260  456 - #000000
    388  456 - #ffffff
    456  456 - #688bba
    232  592 - #597c95
";

/// Shows the first frame of 3 ten bit videos. Proves a 10 bit frame reaches
/// the screen with its depth, a PQ and an HLG frame are tone mapped to SDR
/// with the BT.2390 curve and brought from BT.2020 to BT.709, and a 10 bit
/// SDR frame comes out as its signal. The colors each bar must have are
/// computed from the standards' formulas outside the engine: PQ gives
/// `#000000 #bdbdbd #e6e6e6 #ffffff #ad7349`, HLG gives
/// `#000000 #8f8f8f #e6e6e6 #ffffff #845838`, SDR gives
/// `#000000 #808080 #ffffff #e94c3d #3497db`.
#[view]
struct VideoHdr {
    #[init]
    pq_label:  Label,
    pq:        VideoView,
    hlg_label: Label,
    hlg:       VideoView,
    sdr_label: Label,
    sdr:       VideoView,
}

impl Setup for VideoHdr {
    fn setup(self: Weak<Self>) {
        let rows = [
            (self.pq_label, self.pq, "PQ", "hilen-video-hdr-pq.mp4", PQ),
            (self.hlg_label, self.hlg, "HLG", "hilen-video-hdr-hlg.mp4", HLG),
            (
                self.sdr_label,
                self.sdr,
                "10 bit SDR",
                "hilen-video-hdr-sdr.mp4",
                SDR,
            ),
        ];
        let mut top = 40.0;
        for (label, video, name, file, bytes) in rows {
            let path = temp_dir().join(file);
            write(&path, bytes).expect("the fixture video is writable to the temp dir");
            label.set_frame((20, top + 40.0, 160, 40));
            label.set_text(name);
            video.set_frame((200, top, 320, 120));
            video.set_mode(ImageMode::Fill).set_source(path.to_string_lossy());
            top += 150.0;
        }
    }
}

impl ViewTest for VideoHdr {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(Clock::enter_stepped);
        step_frames(2);
        for (name, video) in [("PQ", view.pq), ("HLG", view.hlg), ("SDR", view.sdr)] {
            ensure!(
                from_main(move || video.is_loaded()),
                "the {name} fixture must open"
            );
        }
        check_colors(BARS)?;
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
