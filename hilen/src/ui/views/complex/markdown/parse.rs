use std::mem::take;

use linkify::{LinkFinder, LinkKind};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, TextMergeStream};

use super::model::{Block, Face, Item, Link, Marker, Span, Style, Styled};

/// Stands right after every link when the view shows copy buttons, the
/// button of the link is drawn over it. A copy of the text leaves it out.
pub(crate) const COPY_ROOM: char = '\u{2003}';

/// GitHub flavored markdown as blocks, with tables, task lists,
/// strikethrough and bare urls as links.
#[cfg(test)]
pub(crate) fn parse(text: &str) -> Vec<Block> {
    parse_lines(text, false, false)
}

/// Like `parse`. With `keeps_line_breaks` a line break inside a paragraph
/// stays one, markdown itself reads it as a space. With `copy_rooms` a
/// `COPY_ROOM` follows every link.
pub(crate) fn parse_lines(text: &str, keeps_line_breaks: bool, copy_rooms: bool) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut builder = Builder {
        copy_rooms,
        soft_break: if keeps_line_breaks { "\n" } else { " " },
        ..Builder::default()
    };
    // The parser can hand 1 text over in several pieces, a url among them,
    // merged it is found whole.
    for event in TextMergeStream::new(Parser::new_ext(text, options)) {
        builder.event(event);
    }
    builder.finish()
}

/// What blocks are collected into right now.
enum Frame {
    Quote(Vec<Block>),
    /// `next` is the number of the next item of a numbered list.
    List {
        next:  Option<u64>,
        items: Vec<Item>,
    },
    Item {
        marker: Marker,
        blocks: Vec<Block>,
    },
}

#[derive(Default)]
struct Table {
    head: Vec<Styled>,
    rows: Vec<Vec<Styled>>,
    row:  Vec<Styled>,
}

/// How deep the text is inside each kind of mark.
#[derive(Default)]
struct Depth {
    bold:   u32,
    italic: u32,
    strike: u32,
    link:   u32,
}

#[derive(Default)]
struct Builder {
    root:       Vec<Block>,
    frames:     Vec<Frame>,
    /// The text being collected. A tight list item has no paragraph
    /// around its text, so a text can open one by itself.
    inline:     Option<Styled>,
    depth:      Depth,
    /// Where the link being read starts in the text, and its address.
    link:       Option<(usize, String)>,
    heading:    Option<u8>,
    code:       Option<(String, String)>,
    table:      Option<Table>,
    /// What a line break inside a paragraph becomes.
    soft_break: &'static str,
    copy_rooms: bool,
}

impl Builder {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => match &mut self.code {
                Some((_, code)) => code.push_str(&text),
                None => self.text(&text, false),
            },
            Event::Code(text) => self.text(&text, true),
            Event::Html(text) | Event::InlineHtml(text) => self.text(&text, false),
            Event::SoftBreak => self.text(self.soft_break, false),
            Event::HardBreak => self.text("\n", false),
            Event::Rule => {
                self.flush();
                self.push(Block::Rule);
            }
            Event::TaskListMarker(done) => {
                if let Some(Frame::Item { marker, .. }) = self.frames.last_mut() {
                    *marker = Marker::Task(done);
                }
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush();
                self.inline = Some(Styled::default());
            }
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
                self.inline = Some(Styled::default());
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.frames.push(Frame::Quote(Vec::new()));
            }
            Tag::List(next) => {
                self.flush();
                self.frames.push(Frame::List {
                    next,
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                let marker = match self.frames.last_mut() {
                    Some(Frame::List { next: Some(next), .. }) => {
                        let number = *next;
                        *next += 1;
                        Marker::Number(number)
                    }
                    _ => Marker::Bullet,
                };
                self.frames.push(Frame::Item {
                    marker,
                    blocks: Vec::new(),
                });
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or_default().to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((language, String::new()));
            }
            Tag::Table(_) => {
                self.flush();
                self.table = Some(Table::default());
            }
            Tag::TableCell => self.inline = Some(Styled::default()),
            Tag::Strong => self.depth.bold += 1,
            Tag::Emphasis => self.depth.italic += 1,
            Tag::Strikethrough => self.depth.strike += 1,
            Tag::Link { dest_url, .. } => {
                self.depth.link += 1;
                let start = self.inline.as_ref().map_or(0, |text| text.text.len());
                self.link = Some((start, dest_url.to_string()));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                let text = self.inline.take().unwrap_or_default();
                let level = self.heading.take().unwrap_or(1);
                self.push(Block::Heading { level, text });
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                if let Some(Frame::Quote(blocks)) = self.frames.pop() {
                    self.push(Block::Quote(blocks));
                }
            }
            TagEnd::List(_) => {
                if let Some(Frame::List { items, .. }) = self.frames.pop() {
                    self.push(Block::List(items));
                }
            }
            TagEnd::Item => {
                self.flush();
                if let Some(Frame::Item { marker, blocks }) = self.frames.pop()
                    && let Some(Frame::List { items, .. }) = self.frames.last_mut()
                {
                    items.push(Item { marker, blocks });
                }
            }
            TagEnd::CodeBlock => {
                if let Some((language, text)) = self.code.take() {
                    let text = text.trim_end_matches('\n').to_string();
                    self.push(Block::Code { language, text });
                }
            }
            TagEnd::TableCell => {
                let cell = self.inline.take().unwrap_or_default();
                if let Some(table) = &mut self.table {
                    table.row.push(cell);
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = &mut self.table {
                    table.head = take(&mut table.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    let row = take(&mut table.row);
                    table.rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.push(Block::Table {
                        head: table.head,
                        rows: table.rows,
                    });
                }
            }
            TagEnd::Strong => self.depth.bold = self.depth.bold.saturating_sub(1),
            TagEnd::Emphasis => self.depth.italic = self.depth.italic.saturating_sub(1),
            TagEnd::Strikethrough => self.depth.strike = self.depth.strike.saturating_sub(1),
            TagEnd::Link => {
                self.depth.link = self.depth.link.saturating_sub(1);
                if let Some((start, url)) = self.link.take()
                    && let Some(text) = &mut self.inline
                    && start < text.text.len()
                {
                    text.links.push(Link {
                        range: start..text.text.len(),
                        url,
                    });
                    self.copy_room();
                }
            }
            _ => {}
        }
    }

    /// Adds a text, a bare url in it becomes a link, also one in inline
    /// code. A trailing dot or other mark after the url stays out of it,
    /// `linkify` decides that.
    fn text(&mut self, text: &str, code: bool) {
        if self.depth.link > 0 {
            self.push_text(text, code, false);
            return;
        }
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);
        for piece in finder.spans(text) {
            let url = piece.kind().is_some();
            let start = self.inline.as_ref().map_or(0, |styled| styled.text.len());
            self.push_text(piece.as_str(), code, url);
            if url && let Some(styled) = &mut self.inline {
                styled.links.push(Link {
                    range: start..styled.text.len(),
                    url:   piece.as_str().to_string(),
                });
                self.copy_room();
            }
        }
    }

    fn copy_room(&mut self) {
        if self.copy_rooms {
            self.push_text(COPY_ROOM.encode_utf8(&mut [0; 4]), false, false);
        }
    }

    fn push_text(&mut self, text: &str, code: bool, url: bool) {
        let style = Style {
            face: Face::new(self.depth.bold > 0, self.depth.italic > 0),
            code,
            strike: self.depth.strike > 0,
            link: url || self.depth.link > 0,
        };
        let styled = self.inline.get_or_insert_with(Styled::default);
        let start = styled.text.len();
        styled.text.push_str(text);
        let end = styled.text.len();
        if style == Style::default() {
            return;
        }
        match styled.spans.last_mut() {
            Some(last) if last.range.end == start && last.style == style => last.range.end = end,
            _ => styled.spans.push(Span {
                range: start..end,
                style,
            }),
        }
    }

    /// Ends the text being collected as a paragraph.
    fn flush(&mut self) {
        if let Some(text) = self.inline.take()
            && !text.text.trim().is_empty()
        {
            self.push(Block::Paragraph(text));
        }
    }

    fn push(&mut self, block: Block) {
        match self.frames.last_mut() {
            Some(Frame::Quote(blocks) | Frame::Item { blocks, .. }) => blocks.push(block),
            // The parser puts nothing between a list and its items.
            Some(Frame::List { .. }) => {}
            None => self.root.push(block),
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush();
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::{COPY_ROOM, parse, parse_lines};
    use crate::ui::views::complex::markdown::model::{Block, Face, Marker, Style};

    #[test]
    fn inline_marks_become_spans_and_leave_the_text() {
        let blocks = parse("a **bold** and `code` and ~~gone~~ and [link](https://x.y)");
        let [Block::Paragraph(text)] = &blocks[..] else {
            panic!("one paragraph expected, got {blocks:?}");
        };
        assert_eq!(text.text, "a bold and code and gone and link");
        let styles: Vec<(&str, Style)> = text
            .spans
            .iter()
            .map(|span| (&text.text[span.range.clone()], span.style))
            .collect();
        let plain = Style::default();
        assert_eq!(
            styles,
            [
                (
                    "bold",
                    Style {
                        face: Face::Bold,
                        ..plain
                    }
                ),
                ("code", Style { code: true, ..plain }),
                (
                    "gone",
                    Style {
                        strike: true,
                        ..plain
                    }
                ),
                ("link", Style { link: true, ..plain }),
            ]
        );
        assert_eq!(text.links.len(), 1);
        assert_eq!(&text.text[text.links[0].range.clone()], "link");
        assert_eq!(text.links[0].url, "https://x.y");
    }

    /// The urls of a paragraph and their texts.
    fn urls(text: &str) -> Vec<(String, String)> {
        let blocks = parse(text);
        let [Block::Paragraph(text)] = &blocks[..] else {
            panic!("one paragraph expected, got {blocks:?}");
        };
        text.links
            .iter()
            .map(|link| (text.text[link.range.clone()].to_string(), link.url.clone()))
            .collect()
    }

    #[test]
    fn a_bare_url_is_a_link_without_the_dot_after_it() {
        let url = "https://webostv.developer.lge.com";
        assert_eq!(
            urls(&format!("The site is {url}. Click SIGN IN.")),
            [(url.to_string(), url.to_string())]
        );
        let blocks = parse(&format!("Here it is:\n\n{url}\n\nClick SIGN IN."));
        let [_, Block::Paragraph(alone), _] = &blocks[..] else {
            panic!("3 paragraphs expected, got {blocks:?}");
        };
        assert_eq!(alone.links.len(), 1);
        assert_eq!(alone.links[0].range, 0..url.len());
        assert_eq!(alone.links[0].url, url);
        assert!(alone.spans.iter().all(|span| span.style.link));
    }

    #[test]
    fn marks_after_a_bare_url_stay_out_of_it() {
        let found = urls("(see https://x.com/a_b), or https://y.com/q?a=1! and \"https://z.com\"; end");
        let found: Vec<&str> = found.iter().map(|(_, url)| url.as_str()).collect();
        assert_eq!(
            found,
            ["https://x.com/a_b", "https://y.com/q?a=1", "https://z.com"]
        );
    }

    #[test]
    fn a_url_in_code_is_a_link_and_one_in_a_link_is_not_found_again() {
        assert_eq!(
            urls("`https://x.com` and www.y.com"),
            [("https://x.com".to_string(), "https://x.com".to_string())]
        );
        assert_eq!(
            urls("[https://x.com](https://x.com/long)"),
            [("https://x.com".to_string(), "https://x.com/long".to_string())]
        );
    }

    #[test]
    fn a_copy_room_follows_every_link_only_when_asked() {
        let text = "see https://x.com. and [a](https://y.com) or `https://z.com`";
        let [Block::Paragraph(plain)] = &parse_lines(text, false, false)[..] else {
            panic!("one paragraph expected");
        };
        assert!(!plain.text.contains(COPY_ROOM));
        let [Block::Paragraph(roomy)] = &parse_lines(text, false, true)[..] else {
            panic!("one paragraph expected");
        };
        assert_eq!(
            roomy.text,
            "see https://x.com\u{2003}. and a\u{2003} or https://z.com\u{2003}"
        );
        for link in &roomy.links {
            assert!(roomy.text[link.range.end..].starts_with(COPY_ROOM));
            assert!(!roomy.text[link.range.clone()].contains(COPY_ROOM));
        }
        // The room is plain text, no underline and no code color.
        let room = roomy.links[0].range.end;
        assert!(roomy.spans.iter().all(|span| !span.range.contains(&room)));
    }

    #[test]
    fn an_item_of_only_bold_text_keeps_it_over_its_nested_list() {
        let blocks = parse("- **Exception.** text\n- **Docs.**\n  - deep one\n  - deep two\n");
        let [Block::List(items)] = &blocks[..] else {
            panic!("one list expected, got {blocks:#?}");
        };
        let [Block::Paragraph(text), Block::List(deep)] = &items[1].blocks[..] else {
            panic!("a text and a list expected, got {:#?}", items[1].blocks);
        };
        assert_eq!(text.text, "Docs.");
        assert_eq!(deep.len(), 2);
    }

    #[test]
    fn a_line_break_in_a_paragraph_is_a_space_or_stays() {
        let text = "one\ntwo\n\nthree";
        let [Block::Paragraph(joined), Block::Paragraph(_)] = &parse(text)[..] else {
            panic!("2 paragraphs expected");
        };
        assert_eq!(joined.text, "one two");
        let [Block::Paragraph(kept), Block::Paragraph(_)] = &parse_lines(text, true, false)[..] else {
            panic!("2 paragraphs expected");
        };
        assert_eq!(kept.text, "one\ntwo");
    }

    #[test]
    fn the_blocks_of_a_reply_come_out() {
        let text = "# Title\n\ntext\n\n- one\n- two\n  - deep\n\n1. first\n2. second\n\n- [ ] open\n- [x] done\n\n> quoted\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n---\n";
        let blocks = parse(text);
        let [
            Block::Heading {
                level: 1,
                text: title,
            },
            Block::Paragraph(_),
            Block::List(bullets),
            Block::List(numbers),
            Block::List(tasks),
            Block::Quote(quote),
            Block::Code { language, text: code },
            Block::Table { head, rows },
            Block::Rule,
        ] = &blocks[..]
        else {
            panic!("other blocks than expected: {blocks:#?}");
        };
        assert_eq!(title.text, "Title");
        assert_eq!(bullets.len(), 2);
        assert_eq!(bullets[0].marker, Marker::Bullet);
        // The nested list sits inside its item, after the text of it.
        assert!(
            matches!(&bullets[1].blocks[..], [Block::Paragraph(_), Block::List(deep)] if deep.len() == 1)
        );
        assert_eq!(numbers[1].marker, Marker::Number(2));
        assert_eq!(tasks[0].marker, Marker::Task(false));
        assert_eq!(tasks[1].marker, Marker::Task(true));
        assert!(matches!(&quote[..], [Block::Paragraph(text)] if text.text == "quoted"));
        assert_eq!(language, "rust");
        assert_eq!(code, "fn main() {}");
        assert_eq!(head.len(), 2);
        assert_eq!(rows[0][1].text, "2");
    }
}
