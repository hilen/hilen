use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BLACK, Container, GREEN, Label, Rect, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct FlowWrapText {
    words: Vec<Weak<Label>>,

    #[init]
    sentence: Container,
}

impl Setup for FlowWrapText {
    fn setup(self: Weak<Self>) {
        self.sentence.set_color(BLACK);
        self.sentence.place().tl(20).w(400).all(8).all_wrap();

        for word in "Grumpy wizards make toxic brew for the jovial queen".split(' ') {
            self.add_word(word);
        }
    }
}

impl FlowWrapText {
    fn add_word(mut self: Weak<Self>, text: &str) {
        let word = self.sentence.add_view::<Label>();
        word.set_color(GREEN);
        word.set_text(text).set_text_size(32);
        word.place().fit_text();
        self.words.push(word);
    }
}

fn check_flow(words: &[Rect], sentence: Rect, margin: f32) {
    assert!(
        words[0].x().abs() < 0.1 && words[0].y().abs() < 0.1,
        "first word is not at the origin: {:?}",
        words[0]
    );

    let mut rows = 0;
    let mut bottom: f32 = 0.0;

    for (i, word) in words.iter().enumerate() {
        assert!(
            word.max_x() <= sentence.width() + 0.5,
            "word {i} sticks out of the container: {word:?}"
        );

        bottom = bottom.max(word.max_y());

        if word.x().abs() < 0.1 {
            rows += 1;
        } else {
            let previous = &words[i - 1];
            assert!(
                (word.y() - previous.y()).abs() < 0.1,
                "word {i} is not on the row of its predecessor: {word:?} vs {previous:?}"
            );
            assert!(
                (word.x() - previous.max_x() - margin).abs() < 0.1,
                "word {i} does not follow its predecessor with the margin: {word:?} vs {previous:?}"
            );
        }
    }

    assert!(rows > 1, "the sentence did not wrap");
    assert!(
        (sentence.height() - bottom).abs() < 0.5,
        "container height {} does not match content bottom {bottom}",
        sentence.height()
    );
}

impl ViewTest for FlowWrapText {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             136   20 - #000000
             416   20 - #000000
             240   28 - #00ff00
             348   28 - #00ac00
              72   36 - #007400
             100   36 - #008800
              24   40 - #000100
             168   40 - #005400
             300   40 - #006000
              36   44 - #007400
             208   44 - #000100
             280   44 - #007c00
             100   52 - #008800
             392   60 - #000000
             116   68 - #00ff00
             200   80 - #009400
             300   80 - #004000
             244   84 - #003000
             148   88 - #00fc00
             200   88 - #009400
             300   88 - #004000
              68   92 - #00ff00
             356   92 - #00ff00
              36  100 - #00ff00
             100  100 - #00ff00
             200  100 - #000000
             288  100 - #00ff00
             416  100 - #000000
             588  316 - #597c95
             300  432 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        let (words, sentence) = from_main(move || {
            let words: Vec<Rect> = view.words.iter().map(|w| *w.frame()).collect();
            (words, *view.sentence.frame())
        });

        check_flow(&words, sentence, 8.0);

        from_main(move || {
            for word in ["and", "jack", "quickly", "vexed", "the", "sphinx"] {
                view.add_word(word);
            }
        });

        wait_for_next_frame();

        check_colors(
            r"
             224   20 - #00ff00
             288   20 - #00ff00
             416   20 - #000000
              28   28 - #000000
             348   28 - #00ac00
              72   40 - #007400
             100   44 - #008800
             168   44 - #005400
             300   44 - #006000
              20   80 - #00ff00
             300   80 - #004000
             240   84 - #00ff00
             364   84 - #000000
              72   92 - #000000
             148   92 - #00ff00
             200  100 - #000000
             416  112 - #000000
              28  128 - #00ff00
             136  140 - #005000
             304  140 - #00ff00
              68  148 - #004000
             196  148 - #004000
             260  148 - #004000
             368  148 - #004000
              92  184 - #007c00
              28  192 - #00ff00
             252  192 - #000000
             416  192 - #000000
             592  352 - #597c95
             296  480 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        let (grown_words, grown_sentence) = from_main(move || {
            let words: Vec<Rect> = view.words.iter().map(|w| *w.frame()).collect();
            (words, *view.sentence.frame())
        });

        check_flow(&grown_words, grown_sentence, 8.0);

        assert!(
            grown_sentence.height() > sentence.height(),
            "container did not grow with new words: {} vs {}",
            grown_sentence.height(),
            sentence.height()
        );

        Ok(())
    }
}
