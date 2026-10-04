//! Subtitle files read as text, for the browser, where no ffmpeg parses
//! them. `SubRip`, `WebVTT` and ASS, the 3 text formats a media server hands
//! out.

/// One line of subtitles and the seconds of the video it shows for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextCue {
    pub start: f64,
    pub end:   f64,
    pub text:  String,
}

/// Every cue of a subtitle file, in time order. The format is told from
/// the text itself, a server does not always send the file name.
pub(crate) fn parse(text: &str) -> Vec<TextCue> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\r', "\n");

    let mut cues = if text.contains("[Events]") || text.lines().any(|line| line.starts_with("Dialogue:")) {
        parse_ass(&text)
    } else {
        parse_blocks(&text)
    };

    cues.retain(|cue| cue.end > cue.start && !cue.text.is_empty());
    cues.sort_by(|a, b| a.start.total_cmp(&b.start));
    cues
}

/// The line on screen at `position`, the last started one when 2 overlap.
pub(crate) fn at(cues: &[TextCue], position: f64) -> Option<&str> {
    cues.iter()
        .rev()
        .find(|cue| cue.start <= position && position < cue.end)
        .map(|cue| cue.text.as_str())
}

/// `SubRip` and `WebVTT` are both blocks split by an empty line, with a
/// `start --> end` line and the text under it.
fn parse_blocks(text: &str) -> Vec<TextCue> {
    let mut cues = Vec::new();

    for block in text.split("\n\n") {
        let mut lines = block.lines().skip_while(|line| !line.contains("-->"));
        let Some(timing) = lines.next() else {
            continue;
        };
        let Some((start, end)) = timing.split_once("-->") else {
            continue;
        };
        // WebVTT puts cue settings after the end time.
        let end = end.split_whitespace().next().unwrap_or_default();
        let (Some(start), Some(end)) = (seconds(start.trim()), seconds(end)) else {
            continue;
        };

        let text: Vec<String> = lines.map(strip_tags).filter(|line| !line.is_empty()).collect();
        cues.push(TextCue {
            start,
            end,
            text: text.join("\n"),
        });
    }

    cues
}

/// `Dialogue: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text`.
fn parse_ass(text: &str) -> Vec<TextCue> {
    let mut cues = Vec::new();

    for line in text.lines() {
        let Some(event) = line.strip_prefix("Dialogue:") else {
            continue;
        };
        let fields: Vec<&str> = event.splitn(10, ',').collect();
        let [_, start, end, .., body] = fields.as_slice() else {
            continue;
        };
        if fields.len() != 10 {
            continue;
        }
        let (Some(start), Some(end)) = (seconds(start.trim()), seconds(end.trim())) else {
            continue;
        };
        cues.push(TextCue {
            start,
            end,
            text: ass_text(body),
        });
    }

    cues
}

/// `01:02:03,456`, `01:02:03.456`, `02:03.456` and the ASS `1:02:03.45`.
fn seconds(time: &str) -> Option<f64> {
    let time = time.replace(',', ".");
    let mut total = 0.0;
    let mut parts = 0;
    for part in time.split(':') {
        total = total * 60.0 + part.trim().parse::<f64>().ok()?;
        parts += 1;
    }
    (2..=3).contains(&parts).then_some(total)
}

/// Drops `<i>` and the like, and the `{\an8}` blocks `SubRip` files carry.
fn strip_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut closer = None;
    for c in line.chars() {
        match (closer, c) {
            (None, '<') => closer = Some('>'),
            (None, '{') => closer = Some('}'),
            (None, _) => out.push(c),
            (Some(close), _) if c == close => closer = None,
            (Some(_), _) => {}
        }
    }
    out.trim().to_string()
}

/// The text of an ASS event with its styling dropped. The override blocks
/// in braces go, `\N` is a line break and `\h` a space.
fn ass_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_override = false;
    while let Some(c) = chars.next() {
        match c {
            '{' => in_override = true,
            '}' => in_override = false,
            _ if in_override => {}
            '\\' => match chars.peek() {
                Some('N' | 'n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('h') => {
                    chars.next();
                    out.push(' ');
                }
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod test {
    use super::{at, parse};

    const SRT: &str = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,500\r\n<i>Hello</i>\r\nthere\r\n\r\n2\r\n00:01:00,000 --> 00:01:01,000\r\n{\\an8}Second\r\n";

    const VTT: &str = "WEBVTT\n\nNOTE a comment\n\nintro\n00:01.000 --> 00:02.000 line:90% align:center\nShort times\n\n01:00:00.000 --> 01:00:01.500\nAn hour in\n";

    const ASS: &str = r"[Script Info]
Title: test

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:03.00,0:00:04.50,Default,,0,0,0,,{\i1}Styled{\i0}\Nline, with a comma
Comment: 0,0:00:05.00,0:00:06.00,Default,,0,0,0,,not shown
";

    #[test]
    fn srt_blocks_are_read_with_tags_dropped() {
        let cues = parse(SRT);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello\nthere");
        assert!((cues[0].start - 1.0).abs() < 1e-9 && (cues[0].end - 2.5).abs() < 1e-9);
        assert_eq!(cues[1].text, "Second");
        assert!((cues[1].start - 60.0).abs() < 1e-9);
    }

    #[test]
    fn vtt_times_without_hours_and_cue_settings_are_read() {
        let cues = parse(VTT);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Short times");
        assert!((cues[0].end - 2.0).abs() < 1e-9);
        assert!((cues[1].start - 3600.0).abs() < 1e-9);
    }

    #[test]
    fn ass_dialogue_keeps_commas_and_drops_styling() {
        let cues = parse(ASS);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Styled\nline, with a comma");
        assert!((cues[0].start - 3.0).abs() < 1e-9 && (cues[0].end - 4.5).abs() < 1e-9);
    }

    #[test]
    fn the_cue_at_a_position_is_found() {
        let cues = parse(SRT);
        assert_eq!(at(&cues, 0.5), None);
        assert_eq!(at(&cues, 1.0), Some("Hello\nthere"));
        assert_eq!(at(&cues, 2.5), None);
        assert_eq!(at(&cues, 60.5), Some("Second"));
    }
}
