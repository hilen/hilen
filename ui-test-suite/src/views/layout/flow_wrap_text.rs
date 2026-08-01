use anyhow::Result;
use test_engine::{
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
             196   24 - #00ff00
             240   24 - #00ff00
             284   24 - #00ff00
             328   24 - #00ff00
             408   24 - #00ff00
             116   28 - #00ff00
             148   28 - #00ff00
              48   32 - #000000
             364   36 - #00ff00
              80   40 - #00ff00
             168   56 - #000000
             208   56 - #000000
             264   56 - #000000
             304   56 - #000000
             416   76 - #000000
             104   80 - #00ff00
              64   84 - #00ff00
              24   88 - #00ff00
             140   88 - #000000
             184   88 - #00ff00
             220   88 - #00ff00
             284   88 - #00ff00
             328   88 - #00ff00
             368   88 - #000000
             408  272 - #597c95
             592  304 - #597c95
              60  340 - #597c95
             300  424 - #597c95
               4  592 - #597c95
             200  592 - #597c95
             396  592 - #597c95
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
             128   24 - #00ff00
             240   24 - #00ff00
             300   24 - #00ff00
             408   24 - #00ff00
              48   32 - #000000
             184   40 - #00ff00
             352   48 - #00ff00
             284   72 - #00ff00
              44   76 - #00ff00
             112   76 - #00ff00
             416   80 - #000000
             592   84 - #597c95
             228   96 - #000000
             168  104 - #00ff00
             368  104 - #00ff00
              76  112 - #000000
              28  120 - #00ff00
             128  120 - #00ff00
             292  124 - #00ff00
             104  156 - #000100
              52  164 - #00ff00
             136  168 - #000000
             220  168 - #000000
             344  168 - #000000
             416  168 - #000000
             592  344 - #597c95
               8  364 - #597c95
             296  464 - #597c95
               4  592 - #597c95
             180  592 - #597c95
             416  592 - #597c95
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
