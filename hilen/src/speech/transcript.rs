use super::DictationEvent;

/// The text of 1 dictation, a sentence each, as a field shows it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Transcript {
    sentences: Vec<String>,
}

impl Transcript {
    /// Takes the event in, `true` when the text changed. A refined text that
    /// is empty keeps the streaming one, the refiner heard less.
    pub fn apply(&mut self, event: &DictationEvent) -> bool {
        let (sentence, text) = match event {
            DictationEvent::Live { sentence, text } | DictationEvent::Final { sentence, text } => {
                (*sentence, text)
            }
            DictationEvent::Refined { sentence, text } if !text.is_empty() => (*sentence, text),
            _ => return false,
        };
        if self.sentences.len() <= sentence {
            self.sentences.resize(sentence + 1, String::new());
        }
        if self.sentences[sentence] == *text {
            return false;
        }
        self.sentences[sentence].clone_from(text);
        true
    }

    /// The sentences with words, joined by a space.
    pub fn text(&self) -> String {
        self.sentences
            .iter()
            .filter(|sentence| !sentence.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The text of the streaming model, which writes in capitals, the way a
/// sentence is written. A lone `i` is the word I.
pub(super) fn streamed(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let word = word.to_lowercase();
            if word == "i" || word.starts_with("i'") {
                let mut chars = word.chars();
                chars.next();
                format!("I{}", chars.as_str())
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{DictationEvent, Transcript, streamed};

    fn live(sentence: usize, text: &str) -> DictationEvent {
        DictationEvent::Live {
            sentence,
            text: text.to_string(),
        }
    }

    #[test]
    fn a_later_event_replaces_its_sentence() {
        let mut transcript = Transcript::default();
        assert!(transcript.apply(&live(0, "hello")));
        assert!(transcript.apply(&live(0, "hello word")));
        assert!(transcript.apply(&DictationEvent::Final {
            sentence: 0,
            text:     "hello world".to_string(),
        }));
        assert!(transcript.apply(&live(1, "how are")));
        assert_eq!(transcript.text(), "hello world how are");

        assert!(transcript.apply(&DictationEvent::Refined {
            sentence: 0,
            text:     "Hello, world.".to_string(),
        }));
        assert_eq!(transcript.text(), "Hello, world. how are");
    }

    #[test]
    fn an_empty_refined_text_keeps_the_streaming_one() {
        let mut transcript = Transcript::default();
        transcript.apply(&live(0, "yes"));
        assert!(!transcript.apply(&DictationEvent::Refined {
            sentence: 0,
            text:     String::new(),
        }));
        assert!(!transcript.apply(&live(0, "yes")));
        assert!(!transcript.apply(&DictationEvent::Stopped));
        assert_eq!(transcript.text(), "yes");
    }

    #[test]
    fn the_capitals_of_the_stream_go() {
        assert_eq!(streamed(" I THINK I'M   READY "), "I think I'm ready");
        assert_eq!(streamed(""), "");
    }
}
